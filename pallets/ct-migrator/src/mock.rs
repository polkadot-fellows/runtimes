// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Mock runtime for `pallet-ct-migrator` unit tests.

use crate as pallet_ct_migrator;
use crate::*;
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use cumulus_primitives_core::AggregateMessageOrigin;
use frame_support::{
	derive_impl, parameter_types,
	traits::{fungible::Mutate, ConstU32, InstanceFilter, OnFinalize},
	weights::WeightMeter,
};
use frame_system::EnsureRoot;
use pallet_message_queue::ForceSetHead;
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	AccountId32, BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		Proxy: pallet_proxy,
		CtMigrator: pallet_ct_migrator,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<AccountId32>;
	type Block = Block;
	type AccountData = pallet_balances::AccountData<u128>;
}

/// Existential deposit of the receiving chain.
pub const ED: u128 = 10;

parameter_types! {
	pub const ExistentialDeposit: u128 = ED;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type Balance = u128;
	type AccountStore = System;
	type ExistentialDeposit = ExistentialDeposit;
}

/// Local proxy permissions. Mirrors the shape of the Coretime runtime's `ProxyType`: a total
/// `From<PortableProxyType>` because the wire only carries permissions this chain represents.
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
	CancelProxy,
	ParaRegistration,
}

impl From<PortableProxyType> for ProxyType {
	fn from(portable: PortableProxyType) -> Self {
		match portable {
			PortableProxyType::Any => ProxyType::Any,
			PortableProxyType::NonTransfer => ProxyType::NonTransfer,
			PortableProxyType::CancelProxy => ProxyType::CancelProxy,
			PortableProxyType::ParaRegistration => ProxyType::ParaRegistration,
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

parameter_types! {
	/// This chain's proxy deposit rates; deliberately different from any "relay" rate a test
	/// simulates so resize assertions can't pass by accident.
	pub const ProxyDepositBase: u128 = 100;
	pub const ProxyDepositFactor: u128 = 20;
	/// Small on purpose: lets tests exercise the merged-set overflow path cheaply.
	pub const MaxProxies: u16 = 4;
	pub const MaxPending: u16 = 4;
	pub const AnnouncementDepositBase: u128 = 50;
	pub const AnnouncementDepositFactor: u128 = 10;
}

impl pallet_proxy::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type MaxProxies = MaxProxies;
	type WeightInfo = ();
	type MaxPending = MaxPending;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
	type BlockNumberProvider = System;
}

frame_support::parameter_types! {
	/// Registrations handed to the registrar pallet, oldest first.
	pub static ReceivedParas: Vec<registrar_primitives::MigratedPara<sp_runtime::AccountId32>> = Vec::new();
	/// The id counter this chain adopted, if one arrived.
	pub static ReceivedNextFreeParaId: Option<u32> = None;
	/// Channels handed to the HRMP pallet, oldest first.
	pub static ReceivedChannels: Vec<hrmp_primitives::MigratedChannel> = Vec::new();
	/// When set, the next hand-over is refused, so the parking path can be exercised.
	pub static ReceiveFails: bool = false;
}

/// Stands in for `pallet-registrar-para`.
///
/// A recorder rather than the real pallet: these tests are about what the migrator hands over and
/// what it does when a hand-over is refused. That the receiving pallet then holds the right
/// deposits in the right state is its own business, and its own tests.
pub struct RecordingRegistrar;

impl registrar_primitives::ReceiveMigratedParas for RecordingRegistrar {
	type AccountId = sp_runtime::AccountId32;

	fn receive_para(
		para: registrar_primitives::MigratedPara<sp_runtime::AccountId32>,
	) -> sp_runtime::DispatchResult {
		if ReceiveFails::get() {
			return Err(sp_runtime::DispatchError::Other("receiver refused"));
		}
		ReceivedParas::mutate(|v| v.push(para));
		Ok(())
	}

	fn receive_next_free_para_id(para_id: u32) {
		ReceivedNextFreeParaId::set(Some(para_id));
	}
}

/// Stands in for `pallet-hrmp-para`. See [`RecordingRegistrar`].
pub struct RecordingHrmp;

impl hrmp_primitives::ReceiveMigratedChannels for RecordingHrmp {
	fn receive_channel(channel: hrmp_primitives::MigratedChannel) -> sp_runtime::DispatchResult {
		if ReceiveFails::get() {
			return Err(sp_runtime::DispatchError::Other("receiver refused"));
		}
		ReceivedChannels::mutate(|v| v.push(channel));
		Ok(())
	}
}

/// Records what this chain sends upwards, so the handshake answer can be asserted on.
pub struct RecordingRouter;
impl SendXcm for RecordingRouter {
	type Ticket = (Location, Xcm<()>);

	fn validate(
		dest: &mut Option<Location>,
		msg: &mut Option<Xcm<()>>,
	) -> SendResult<Self::Ticket> {
		if SendFails::get() {
			return Err(SendError::Transport("test-induced failure"));
		}
		Ok(((dest.take().unwrap(), msg.take().unwrap()), Assets::new()))
	}

	fn deliver(ticket: Self::Ticket) -> Result<XcmHash, SendError> {
		SentXcm::mutate(|sent| sent.push(ticket));
		Ok(XcmHash::default())
	}
}

frame_support::parameter_types! {
	pub static SendFails: bool = false;
	pub static SentXcm: Vec<(Location, Xcm<()>)> = Vec::new();
	/// Every queue the pallet forced to the head, in order.
	pub static ForcedHeads: Vec<AggregateMessageOrigin> = Vec::new();
	/// Three blocks of priority, one of round robin.
	pub const DmpQueuePriorityPattern: (u64, u64) = (3, 1);
}

/// Records which queue was put first instead of touching a real message queue.
pub struct RecordingHead;
impl ForceSetHead<AggregateMessageOrigin> for RecordingHead {
	fn force_set_head(_: &mut WeightMeter, origin: &AggregateMessageOrigin) -> Result<bool, ()> {
		ForcedHeads::mutate(|forced| forced.push(origin.clone()));
		Ok(true)
	}
}

/// Run the pallet's hooks for the next `n` blocks.
pub fn run_blocks(n: u64) {
	for _ in 0..n {
		let now = System::block_number() + 1;
		System::set_block_number(now);
		<CtMigrator as OnFinalize<u64>>::on_finalize(now);
	}
}

/// The messages sent upwards so far, destination and all.
pub fn sent() -> Vec<(Location, Xcm<()>)> {
	SentXcm::get()
}

impl pallet_ct_migrator::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	// Sender block time 6s, this chain 12s: migrated delays halve.
	type RcBlockTimeRatio = ConstU32<2>;
	type RegistrarReceiver = RecordingRegistrar;
	type HrmpReceiver = RecordingHrmp;
	type SendXcm = RecordingRouter;
	type AdminOrigin = EnsureRoot<AccountId32>;
	type MessageQueue = RecordingHead;
	type DmpQueuePriorityPattern = DmpQueuePriorityPattern;
}

/// What each migrated hold becomes locally; mirrors the Coretime runtime's mapping.
impl From<PortableHoldReason> for RuntimeHoldReason {
	fn from(reason: PortableHoldReason) -> Self {
		match reason {
			PortableHoldReason::UnnamedReserve =>
				RuntimeHoldReason::CtMigrator(HoldReason::RcMigratedReserve),
			PortableHoldReason::ProxyDeposit =>
				RuntimeHoldReason::CtMigrator(HoldReason::ProxyDeposit),
			PortableHoldReason::UnattributedReserve =>
				RuntimeHoldReason::CtMigrator(HoldReason::UnattributedReserve),
		}
	}
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	// The recorders are thread locals and outlive a single test, so clear them here.
	ReceivedParas::set(Vec::new());
	ReceivedNextFreeParaId::set(None);
	ReceivedChannels::set(Vec::new());
	ReceiveFails::set(false);

	let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	// Block 1 so deposited events are recorded.
	ext.execute_with(|| System::set_block_number(1));
	ext
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

pub fn acc(n: u8) -> AccountId32 {
	AccountId32::new([n; 32])
}

pub fn free(who: &AccountId32) -> u128 {
	pallet_balances::Pallet::<Test>::free_balance(who)
}

pub fn held(reason: HoldReason, who: &AccountId32) -> u128 {
	use frame_support::traits::fungible::InspectHold;
	<Balances as InspectHold<AccountId32>>::balance_on_hold(
		&RuntimeHoldReason::CtMigrator(reason),
		who,
	)
}

pub fn total_issuance() -> u128 {
	pallet_balances::TotalIssuance::<Test>::get()
}

/// Fund `who` and place `amount` under the given migrated-hold reason — the state the accounts
/// stage leaves behind, which the record stages then re-attribute.
pub fn give_hold(who: &AccountId32, reason: HoldReason, amount: u128) {
	use frame_support::traits::fungible::MutateHold;
	<Balances as Mutate<AccountId32>>::mint_into(who, amount + ED).unwrap();
	<Balances as MutateHold<AccountId32>>::hold(
		&RuntimeHoldReason::CtMigrator(reason),
		who,
		amount,
	)
	.unwrap();
}

/// All `pallet-ct-migrator` events since the last call to this function.
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
	who: &AccountId32,
	free: u128,
	holds: Vec<(PortableHoldReason, u128)>,
) -> PortableAccountOf<Test> {
	let holds: Vec<_> = holds
		.into_iter()
		.map(|(reason, amount)| PortableHold { reason, amount })
		.collect();
	PortableAccount { who: who.clone(), free, holds: holds.try_into().unwrap() }
}
