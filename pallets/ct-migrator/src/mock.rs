// Copyright (C) Polkadot Fellows.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot. If not, see <http://www.gnu.org/licenses/>.

//! Test runtime for `pallet-ct-migrator`.

use crate::{
	self as pallet_ct_migrator, accounts::PortableAccountOf, proxy::PortableProxyOf, HoldReason,
};
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::{
	derive_impl, ord_parameter_types, parameter_types,
	traits::{fungible::InspectHold, ConstU128, ConstU32, Contains, InstanceFilter},
};
use frame_system::EnsureSignedBy;
use migrator_types::{
	PortableAccount, PortableHold, PortableHoldReason, PortableProxy, PortableProxyDelegate,
	PortableProxyType,
};
use scale_info::TypeInfo;
use sp_runtime::{traits::BlakeTwo256, BuildStorage};
use xcm::prelude::*;

type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = u64;

frame_support::construct_runtime! {
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		Proxy: pallet_proxy,
		CtMigrator: pallet_ct_migrator,
	}
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type BaseCallFilter = CtMigrator;
	type Block = Block;
	type AccountData = pallet_balances::AccountData<u128>;
}

/// The existential deposit of the receiving chain
pub const ED: u128 = 1;

parameter_types! {
	pub const ExistentialDeposit: u128 = ED;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type Balance = u128;
	type AccountStore = System;
	type ExistentialDeposit = ExistentialDeposit;
	type RuntimeHoldReason = RuntimeHoldReason;
}

// Somebody
pub const ALICE: AccountId = 1;
// The account behind the admin origin.
pub const ADMIN: AccountId = 2;
pub const BOB: AccountId = 3;
pub const CHARLIE: AccountId = 4;
pub const DAVE: AccountId = 5;

#[derive(
	Copy,
	Clone,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Encode,
	Decode,
	DecodeWithMemTracking,
	Debug,
	MaxEncodedLen,
	TypeInfo,
	Default,
)]
pub enum ProxyType {
	#[default]
	Any,
	NonTransfer,
}

impl From<PortableProxyType> for ProxyType {
	fn from(t: PortableProxyType) -> Self {
		match t {
			PortableProxyType::Any | PortableProxyType::ParaRegistration => ProxyType::Any,
			PortableProxyType::NonTransfer | PortableProxyType::CancelProxy =>
				ProxyType::NonTransfer,
		}
	}
}

impl InstanceFilter<RuntimeCall> for ProxyType {
	fn filter(&self, _c: &RuntimeCall) -> bool {
		matches!(self, ProxyType::Any)
	}
	fn is_superset(&self, o: &Self) -> bool {
		self == o || matches!(self, ProxyType::Any)
	}
}

/// What one proxy definition costs here: `PROXY_DEPOSIT_BASE + PROXY_DEPOSIT_FACTOR` for the first.
pub const PROXY_DEPOSIT_BASE: u128 = 100;
pub const PROXY_DEPOSIT_FACTOR: u128 = 20;

impl pallet_proxy::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = ConstU128<PROXY_DEPOSIT_BASE>;
	type ProxyDepositFactor = ConstU128<PROXY_DEPOSIT_FACTOR>;
	type MaxProxies = ConstU32<4>;
	type WeightInfo = ();
	type MaxPending = ConstU32<4>;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = ConstU128<0>;
	type AnnouncementDepositFactor = ConstU128<0>;
	type BlockNumberProvider = System;
}

ord_parameter_types! {
	pub const AdminAccount: AccountId = ADMIN;
}

parameter_types! {
	/// Every message the pallet successfully sent, in order.
	pub static SentXcm: Vec<(Location, Xcm<()>)> = vec![];
	/// Makes the router reject everything.
	pub static SendFails: bool = false;
}

/// Records what the pallet sends instead of delivering it.
pub struct RecordingRouter;

impl SendXcm for RecordingRouter {
	type Ticket = (Location, Xcm<()>);

	fn validate(
		dest: &mut Option<Location>,
		msg: &mut Option<Xcm<()>>,
	) -> SendResult<Self::Ticket> {
		if SendFails::get() {
			return Err(SendError::Transport("send forced to fail"));
		}
		let ticket =
			(dest.take().expect("destination is set"), msg.take().expect("message is set"));
		Ok((ticket, Assets::new()))
	}

	fn deliver(ticket: Self::Ticket) -> Result<XcmHash, SendError> {
		let mut sent = SentXcm::get();
		sent.push(ticket);
		SentXcm::set(sent);
		Ok([0; 32])
	}
}

impl pallet_ct_migrator::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type SendXcm = RecordingRouter;
	type AdminOrigin = EnsureSignedBy<AdminAccount, AccountId>;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	// relay 6s blocks, this chain 12s
	type RcBlockTimeRatio = ConstU32<2>;
	type PreMigrationCalls = TaggedRemark<PRE_MIGRATION>;
	type IntraMigrationCalls = TaggedRemark<INTRA_MIGRATION>;
	type PostMigrationCalls = TaggedRemark<POST_MIGRATION>;
}

/// Tags for [`TaggedRemark`], one per call set.
pub const PRE_MIGRATION: u8 = 0;
pub const INTRA_MIGRATION: u8 = 1;
pub const POST_MIGRATION: u8 = 2;

/// Allows only a remark of the single byte `TAG`, so a test can tell which call set was consulted.
pub struct TaggedRemark<const TAG: u8>;

impl<const TAG: u8> Contains<RuntimeCall> for TaggedRemark<TAG> {
	fn contains(call: &RuntimeCall) -> bool {
		*call == tagged_remark(TAG)
	}
}

/// The remark that only the call set tagged `tag` allows.
pub fn tagged_remark(tag: u8) -> RuntimeCall {
	RuntimeCall::System(frame_system::Call::remark { remark: vec![tag] })
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	SentXcm::set(vec![]);
	SendFails::set(false);

	let storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	let mut ext = sp_io::TestExternalities::new(storage);
	// Block 0 does not record events.
	ext.execute_with(|| System::set_block_number(1));
	ext
}

/// The messages sent so far, destination and all.
pub fn sent() -> Vec<(Location, Xcm<()>)> {
	SentXcm::get()
}

/// Decode the `Transact` payload of the `n`th sent message as a relay-chain runtime call.
pub fn sent_call(n: usize) -> crate::Rc2RuntimeCall {
	let (_, Xcm(instructions)) = sent().get(n).expect("message was sent").clone();
	for instruction in instructions {
		if let Instruction::Transact { call, .. } = instruction {
			return crate::Rc2RuntimeCall::decode(&mut &call.into_encoded()[..])
				.expect("payload decodes as a relay-chain call");
		}
	}
	panic!("message {n} carried no Transact");
}

pub fn fund(who: &AccountId, amount: u128) {
	frame_support::assert_ok!(
		<Balances as frame_support::traits::fungible::Mutate<AccountId>>::mint_into(who, amount)
	);
}

pub fn free(who: &AccountId) -> u128 {
	pallet_balances::Pallet::<Test>::free_balance(who)
}

pub fn reserved(who: &AccountId) -> u128 {
	pallet_balances::Pallet::<Test>::reserved_balance(who)
}

/// What the proxy pallet reserves for `n` definitions here.
pub fn proxy_deposit(n: u128) -> u128 {
	PROXY_DEPOSIT_BASE + PROXY_DEPOSIT_FACTOR * n
}

pub fn held(reason: HoldReason, who: &AccountId) -> u128 {
	<Balances as InspectHold<AccountId>>::balance_on_hold(
		&RuntimeHoldReason::CtMigrator(reason),
		who,
	)
}

pub fn total_issuance() -> u128 {
	pallet_balances::TotalIssuance::<Test>::get()
}

/// Every `pallet-ct-migrator` event since the last call.
pub fn migrator_events() -> Vec<crate::Event<Test>> {
	let events = System::events()
		.into_iter()
		.filter_map(|r| match r.event {
			RuntimeEvent::CtMigrator(e) => Some(e),
			_ => None,
		})
		.collect();
	System::reset_events();
	events
}

pub fn portable_account(
	who: &AccountId,
	free: u128,
	holds: Vec<(PortableHoldReason, u128)>,
) -> PortableAccountOf<Test> {
	let holds: Vec<_> = holds
		.into_iter()
		.map(|(reason, amount)| PortableHold { reason, amount })
		.collect();
	PortableAccount { who: *who, free, holds: holds.try_into().unwrap() }
}

pub fn portable_proxy(
	delegator: &AccountId,
	delegates: Vec<(AccountId, PortableProxyType, u32)>,
) -> PortableProxyOf<Test> {
	let delegates: Vec<_> = delegates
		.into_iter()
		.map(|(delegate, proxy_type, delay)| PortableProxyDelegate { delegate, proxy_type, delay })
		.collect();
	PortableProxy { delegator: *delegator, delegates: delegates.try_into().unwrap() }
}
