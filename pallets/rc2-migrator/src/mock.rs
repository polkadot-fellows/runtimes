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

//! Test runtime for `pallet-rc2-migrator`.

use crate as pallet_rc2_migrator;
use codec::Decode;
use frame_support::{
	derive_impl, ord_parameter_types, parameter_types,
	traits::{fungible::Mutate, ConstU128, OnInitialize, ReservableCurrency, Time},
	PalletId,
};
use frame_system::{EnsureRoot, EnsureSignedBy};
use polkadot_parachain_primitives::primitives::Id as ParaId;
use polkadot_runtime_common::paras_registrar;
use runtime_parachains::{configuration, origin as parachains_origin, paras, shared};
use sp_runtime::{
	traits::{AccountIdConversion, IdentityLookup},
	transaction_validity::TransactionPriority,
	AccountId32, BuildStorage,
};
use xcm::prelude::*;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = AccountId32;

// The pallets that take deposits are the real ones, so every expected reserve in the tests is
// placed the way the live chain placed it.
frame_support::construct_runtime! {
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		Configuration: configuration,
		ParasShared: shared,
		Parachains: paras,
		ParachainsOrigin: parachains_origin,
		Registrar: paras_registrar,
		Rc2Migrator: pallet_rc2_migrator,
	}
}

impl<C> frame_system::offchain::CreateTransactionBase<C> for Test
where
	RuntimeCall: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type RuntimeCall = RuntimeCall;
}

impl<C> frame_system::offchain::CreateBare<C> for Test
where
	RuntimeCall: From<C>,
{
	fn create_bare(call: Self::RuntimeCall) -> Self::Extrinsic {
		UncheckedExtrinsic::new_bare(call)
	}
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<AccountId>;
	type AccountData = pallet_balances::AccountData<u128>;
}

/// The Relay Chain's existential deposit.
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

// `paras_registrar` is what the tests need; it requires `paras`, which requires the rest.
impl shared::Config for Test {
	type DisabledValidators = ();
}

impl parachains_origin::Config for Test {}

impl configuration::Config for Test {
	type WeightInfo = configuration::TestWeightInfo;
}

parameter_types! {
	pub const ParasUnsignedPriority: TransactionPriority = TransactionPriority::MAX;
}

impl paras::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = paras::TestWeightInfo;
	type UnsignedPriority = ParasUnsignedPriority;
	type QueueFootprinter = ();
	type NextSessionRotation = ();
	type OnNewHead = ();
	type AssignCoretime = ();
	type Fungible = Balances;
	type CooldownRemovalMultiplier = ConstU128<1>;
	type AuthorizeCurrentCodeOrigin = EnsureRoot<AccountId>;
}

/// The registrar deposit, reserved on a para's manager.
pub const PARA_DEPOSIT: u128 = 300;

parameter_types! {
	pub const ParaDeposit: u128 = PARA_DEPOSIT;
	pub const DataDepositPerByte: u128 = 1;
}

impl paras_registrar::Config for Test {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type OnSwap = ();
	type ParaDeposit = ParaDeposit;
	type DataDepositPerByte = DataDepositPerByte;
	type WeightInfo = paras_registrar::TestWeightInfo;
}

/// The account the mock treats as the Coretime chain's dispatch origin.
pub const CORETIME: AccountId = AccountId32::new([5; 32]);

/// Somebody
pub const ALICE: AccountId = AccountId32::new([1; 32]);

/// Somebody else
pub const BOB: AccountId = AccountId32::new([3; 32]);
pub const CHARLIE: AccountId = AccountId32::new([4; 32]);
pub const DAVE: AccountId = AccountId32::new([6; 32]);
pub const EVE: AccountId = AccountId32::new([7; 32]);
pub const FRANK: AccountId = AccountId32::new([8; 32]);

/// The account behind the admin origin.
pub const ADMIN: AccountId = AccountId32::new([2; 32]);

pub const CT_PARA_ID: u32 = 1005;
pub const AH_PARA_ID: u32 = 1000;
/// Free balance that follows a deposit to the Coretime chain.
pub const CT_FREE_BUFFER: u128 = 100;
pub const WARM_UP: u64 = 4;
pub const COOL_OFF: u64 = 10;
/// Relay-chain block time
pub const BLOCK_TIME_MS: u64 = 6_000;

parameter_types! {
	pub const CtParaId: u32 = CT_PARA_ID;
	pub const AhParaId: u32 = AH_PARA_ID;
	pub const CtFreeBuffer: u128 = CT_FREE_BUFFER;

	/// Every message the pallet successfully sent, in order.
	pub static SentXcm: Vec<(Location, Xcm<()>)> = vec![];
	/// Makes the router reject everything, to exercise the retry-next-block path.
	pub static SendFails: bool = false;

	/// The mock wall clock, in milliseconds. Advanced by [`run_blocks`].
	pub static MockNow: u64 = BLOCK_TIME_MS;
}

/// Stands in for `pallet_timestamp`.
pub struct MockTime;

impl Time for MockTime {
	type Moment = u64;

	fn now() -> Self::Moment {
		MockNow::get()
	}
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

ord_parameter_types! {
	pub const CoretimeAccount: AccountId = CORETIME;
	pub const AdminAccount: AccountId = ADMIN;
}

impl pallet_rc2_migrator::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type SendXcm = RecordingRouter;
	type CtParaId = CtParaId;
	type AhParaId = AhParaId;
	type CtFreeBuffer = CtFreeBuffer;
	type TimeProvider = MockTime;
	type CtOrigin = EnsureSignedBy<CoretimeAccount, AccountId>;
	type AdminOrigin = EnsureSignedBy<AdminAccount, AccountId>;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	SentXcm::set(vec![]);
	SendFails::set(false);
	MockNow::set(BLOCK_TIME_MS);

	let storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	let mut ext = sp_io::TestExternalities::new(storage);
	// Block 0 does not record events.
	ext.execute_with(|| System::set_block_number(1));
	ext
}

/// Run the next `n` blocks, advancing the clock like a real chain does.
///
/// The clock moves *after* the block's hooks, mirroring the timestamp inherent, which is an
/// extrinsic and so runs after `on_initialize`.
pub fn run_blocks(n: u64) {
	for _ in 0..n {
		let now = System::block_number() + 1;
		System::set_block_number(now);
		<Rc2Migrator as OnInitialize<u64>>::on_initialize(now);
		MockNow::set(now.saturating_mul(BLOCK_TIME_MS));
	}
}

/// The mock clock's current value.
pub fn now_ms() -> u64 {
	MockTime::now()
}

/// The messages sent so far, destination and all.
pub fn sent() -> Vec<(Location, Xcm<()>)> {
	SentXcm::get()
}

/// Decode the `Transact` payload of the `n`th sent message as a Coretime runtime call.
pub fn sent_call(n: usize) -> crate::CtRuntimeCall {
	let (_, Xcm(instructions)) = sent().get(n).expect("message was sent").clone();
	for instruction in instructions {
		if let Instruction::Transact { call, .. } = instruction {
			return crate::CtRuntimeCall::decode(&mut &call.into_encoded()[..])
				.expect("payload decodes as a Coretime call");
		}
	}
	panic!("message {n} carried no Transact");
}

/// Give `who` exactly `amount` of free balance.
pub fn fund(who: &AccountId, amount: u128) {
	<Balances as Mutate<AccountId>>::set_balance(who, amount);
}

/// Reference `who` from another pallet, as session keys do for a validator.
pub fn add_consumer(who: &AccountId) {
	frame_system::Pallet::<Test>::inc_consumers(who).unwrap();
}

/// A para's sovereign account on the Relay Chain, `para` + id.
pub fn child_sov(para: u32) -> AccountId {
	ParaId::from(para).into_account_truncating()
}

/// A pallet (module) account, `modl` + pallet id.
pub fn pallet_account() -> AccountId {
	PalletId(*b"mock/pot").into_account_truncating()
}

/// Replicates an account below the existential deposit: the balances pallet refuses to create
/// one, but they exist on chain, kept alive by another pallet's provider reference.
pub fn force_below_ed(who: &AccountId, free: u128) {
	assert!(free < ED);
	frame_system::Pallet::<Test>::inc_providers(who);
	frame_system::Account::<Test>::mutate(who, |a| a.data.free = free);
	pallet_balances::TotalIssuance::<Test>::mutate(|ti| *ti += free);
}

/// Register a para through the actual registrar path, which records `PARA_DEPOSIT` against the
/// manager and reserves it.
pub fn register_para(id: u32, manager: &AccountId) {
	paras_registrar::NextFreeParaId::<Test>::put(ParaId::from(id));
	Registrar::reserve(RuntimeOrigin::signed(manager.clone())).expect("manager can reserve");
}

/// Reserve part of `who`'s balance the way the registrar, HRMP, proxy and multisig pallets take
/// their deposits on the Relay Chain: an unnamed reserve.
pub fn reserve(who: &AccountId, amount: u128) {
	<Balances as ReservableCurrency<AccountId>>::reserve(who, amount).unwrap();
}
