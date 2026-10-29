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
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::{
	derive_impl, ord_parameter_types, parameter_types,
	traits::{
		fungible::{HoldConsideration, InspectHold, Mutate, MutateHold},
		ConstU128, ConstU32, Contains, Equals, InstanceFilter, LinearStoragePrice,
		LockableCurrency, OnInitialize, ReservableCurrency, Time, WithdrawReasons,
	},
	weights::Weight,
	PalletId,
};
use frame_system::{EnsureRoot, EnsureSignedBy};
use migrator_types::PortableProxyType;
use polkadot_parachain_primitives::primitives::{HrmpChannelId, Id as ParaId};
use polkadot_runtime_common::paras_registrar;
use runtime_parachains::{
	configuration, dmp, hrmp as parachains_hrmp, origin as parachains_origin, paras, shared,
};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AccountIdConversion, BlakeTwo256, Hash, IdentityLookup},
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
		Dmp: dmp,
		Hrmp: parachains_hrmp,
		ParachainsOrigin: parachains_origin,
		Registrar: paras_registrar,
		Proxy: pallet_proxy,
		Multisig: pallet_multisig,
		Preimage: pallet_preimage,
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
	type BaseCallFilter = Rc2Migrator;
	type Block = Block;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<AccountId>;
	type AccountData = pallet_balances::AccountData<u128>;
}

/// The Relay Chain's existential deposit.
pub const ED: u128 = 10;

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type Balance = u128;
	type AccountStore = System;
	type ExistentialDeposit = ConstU128<ED>;
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

impl dmp::Config for Test {}

parameter_types! {
	pub const DefaultChannelSizeAndCapacityWithSystem: (u32, u32) = (4096, 4);
}

impl parachains_hrmp::Config for Test {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type ChannelManager = EnsureRoot<AccountId>;
	type Currency = Balances;
	type DefaultChannelSizeAndCapacityWithSystem = DefaultChannelSizeAndCapacityWithSystem;
	type VersionWrapper = ();
	type WeightInfo = parachains_hrmp::TestWeightInfo;
}

/// Relay-side proxy permissions, one of each kind: two with a meaning on the Coretime chain and
/// one without. The full set and its mapping are the runtime's, tested in its `ahm_v2` module.
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
	Staking,
}

impl TryFrom<ProxyType> for PortableProxyType {
	type Error = ();

	fn try_from(t: ProxyType) -> Result<Self, ()> {
		match t {
			ProxyType::Any => Ok(PortableProxyType::Any),
			ProxyType::NonTransfer => Ok(PortableProxyType::NonTransfer),
			ProxyType::Staking => Err(()),
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

/// What one proxy definition costs: `PROXY_DEPOSIT_BASE + PROXY_DEPOSIT_FACTOR` for the first.
pub const PROXY_DEPOSIT_BASE: u128 = 40;
pub const PROXY_DEPOSIT_FACTOR: u128 = 4;

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
	type AnnouncementDepositBase = ConstU128<ANNOUNCEMENT_DEPOSIT_BASE>;
	type AnnouncementDepositFactor = ConstU128<ANNOUNCEMENT_DEPOSIT_FACTOR>;
	type BlockNumberProvider = System;
}

/// What an announcement costs: `ANNOUNCEMENT_DEPOSIT_BASE + ANNOUNCEMENT_DEPOSIT_FACTOR` for the
/// first.
pub const ANNOUNCEMENT_DEPOSIT_BASE: u128 = 25;
pub const ANNOUNCEMENT_DEPOSIT_FACTOR: u128 = 6;

/// What a multisig operation costs: `MULTISIG_DEPOSIT_BASE + threshold * MULTISIG_DEPOSIT_FACTOR`.
pub const MULTISIG_DEPOSIT_BASE: u128 = 30;
pub const MULTISIG_DEPOSIT_FACTOR: u128 = 5;

impl pallet_multisig::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = ConstU128<MULTISIG_DEPOSIT_BASE>;
	type DepositFactor = ConstU128<MULTISIG_DEPOSIT_FACTOR>;
	type MaxSignatories = ConstU32<5>;
	type WeightInfo = ();
	type BlockNumberProvider = System;
}

parameter_types! {
	pub const PreimageHoldReason: RuntimeHoldReason =
		RuntimeHoldReason::Preimage(pallet_preimage::HoldReason::Preimage);
}

/// A preimage deposit is `1 + len` here.
impl pallet_preimage::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type Consideration = HoldConsideration<
		AccountId,
		Balances,
		PreimageHoldReason,
		LinearStoragePrice<ConstU128<1>, ConstU128<1>, u128>,
	>;
}

/// What the active configuration makes a recipient reserve when it accepts a channel request.
pub const HRMP_RECIPIENT_DEPOSIT: u128 = 20;

/// The registrar deposit, reserved on a para's manager.
pub const PARA_DEPOSIT: u128 = 300;

impl paras_registrar::Config for Test {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type OnSwap = ();
	type ParaDeposit = ConstU128<PARA_DEPOSIT>;
	type DataDepositPerByte = ConstU128<1>;
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
pub const GRACE: AccountId = AccountId32::new([9; 32]);
pub const HEIDI: AccountId = AccountId32::new([10; 32]);
pub const IVAN: AccountId = AccountId32::new([11; 32]);
pub const JUDY: AccountId = AccountId32::new([12; 32]);
pub const LEO: AccountId = AccountId32::new([13; 32]);
pub const MALLORY: AccountId = AccountId32::new([14; 32]);
pub const NIAJ: AccountId = AccountId32::new([15; 32]);

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

parameter_types! {
	pub MockPot: AccountId = pallet_account();
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
	type PreMigrationCalls = TaggedRemark<PRE_MIGRATION>;
	type IntraMigrationCalls = TaggedRemark<INTRA_MIGRATION>;
	type PostMigrationCalls = TaggedRemark<POST_MIGRATION>;
	type SweptAccounts = Equals<MockPot>;
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
	MockNow::set(BLOCK_TIME_MS);

	let storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	let mut ext = sp_io::TestExternalities::new(storage);
	ext.execute_with(|| {
		// Block 0 does not record events.
		System::set_block_number(1);
		configuration::ActiveConfig::<Test>::mutate(|c| {
			c.hrmp_recipient_deposit = HRMP_RECIPIENT_DEPOSIT
		});
	});
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

pub type Stage = crate::MigrationStageOf<Test>;

pub fn assert_stage(expected: Stage) {
	assert_eq!(crate::RcMigrationStage::<Test>::get(), expected);
}

pub fn ah_dest() -> Location {
	Location::new(0, [Parachain(AH_PARA_ID)])
}

pub fn ct_dest() -> Location {
	Location::new(0, [Parachain(CT_PARA_ID)])
}

pub fn total_issuance() -> u128 {
	pallet_balances::TotalIssuance::<Test>::get()
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

/// Reserve a deposit on a para's sovereign, funded first so that its existential deposit stays
/// free.
fn reserve_on_sov(para: u32, deposit: u128) {
	let sov = child_sov(para);
	fund(&sov, free(&sov) + deposit + ED);
	reserve(&sov, deposit);
}

fn channel_id(sender: u32, recipient: u32) -> HrmpChannelId {
	HrmpChannelId { sender: sender.into(), recipient: recipient.into() }
}

/// An open HRMP channel with its deposits reserved on the sovereigns, the state the channel-open
/// handshake leaves behind. The handshake itself needs live paras and sessions.
pub fn open_channel(sender: u32, recipient: u32, sender_deposit: u128, recipient_deposit: u128) {
	reserve_on_sov(sender, sender_deposit);
	reserve_on_sov(recipient, recipient_deposit);
	parachains_hrmp::HrmpChannels::<Test>::insert(
		channel_id(sender, recipient),
		parachains_hrmp::HrmpChannel {
			max_capacity: 8,
			max_total_size: 4096,
			max_message_size: 1024,
			msg_count: 0,
			total_size: 0,
			mqc_head: None,
			sender_deposit,
			recipient_deposit,
		},
	);
}

/// A pending request to close a channel, as `hrmp_close_channel` leaves it until the next
/// session.
pub fn request_close(sender: u32, recipient: u32) {
	let id = channel_id(sender, recipient);
	parachains_hrmp::HrmpCloseChannelRequests::<Test>::insert(&id, ());
	parachains_hrmp::HrmpCloseChannelRequestsList::<Test>::mutate(|list| list.push(id));
}

/// A pending open-channel request with the sender's deposit reserved, as `hrmp_init_open_channel`
/// leaves it.
pub fn open_request(sender: u32, recipient: u32, deposit: u128) {
	reserve_on_sov(sender, deposit);
	let id = channel_id(sender, recipient);
	parachains_hrmp::HrmpOpenChannelRequests::<Test>::insert(
		&id,
		parachains_hrmp::HrmpOpenChannelRequest {
			confirmed: false,
			_age: 0,
			sender_deposit: deposit,
			max_message_size: 1024,
			max_capacity: 8,
			max_total_size: 4096,
		},
	);
	parachains_hrmp::HrmpOpenChannelRequestsList::<Test>::mutate(|list| list.push(id));
	parachains_hrmp::HrmpOpenChannelRequestCount::<Test>::mutate(ParaId::from(sender), |c| *c += 1);
}

/// Accept a pending request as `hrmp_accept_open_channel` does: the recipient's deposit is
/// reserved at the active configuration's value and recorded nowhere until the next session.
pub fn accept_request(sender: u32, recipient: u32) {
	reserve_on_sov(recipient, HRMP_RECIPIENT_DEPOSIT);
	parachains_hrmp::HrmpOpenChannelRequests::<Test>::mutate(
		channel_id(sender, recipient),
		|request| {
			request.as_mut().expect("request is pending").confirmed = true;
		},
	);
}

/// Grant a proxy through the real pallet, which reserves the deposit on the delegator, and bump
/// the nonce as a signed call would.
pub fn add_proxy(delegator: &AccountId, delegate: &AccountId, proxy_type: ProxyType) {
	Proxy::add_proxy(RuntimeOrigin::signed(delegator.clone()), delegate.clone(), proxy_type, 0)
		.expect("can add proxy");
	// a signed call bumps the nonce, which tells a delegator from a pure proxy
	System::inc_account_nonce(delegator);
}

/// Create a pure proxy through the real pallet: the deposit is reserved on the creator and the
/// definition recorded under the pure. Returns the pure's account.
pub fn create_pure(creator: &AccountId) -> AccountId {
	Proxy::create_pure(RuntimeOrigin::signed(creator.clone()), ProxyType::Any, 0, 0)
		.expect("can create a pure proxy");
	Proxy::pure_account(creator, &ProxyType::Any, 0, None)
}

/// Announce a proxied call through the real pallet, which reserves the deposit on the announcer.
/// `announcer` must already be a proxy of `real`.
pub fn announce(announcer: &AccountId, real: &AccountId) {
	Proxy::announce(RuntimeOrigin::signed(announcer.clone()), real.clone(), Default::default())
		.expect("can announce");
}

/// Start a 2-of-2 multisig operation through the real pallet, which reserves the deposit on the
/// depositor.
pub fn start_multisig(depositor: &AccountId, other: &AccountId) {
	Multisig::approve_as_multi(
		RuntimeOrigin::signed(depositor.clone()),
		2,
		vec![other.clone()],
		None,
		[1; 32],
		Weight::zero(),
	)
	.expect("can start a multisig operation");
}

/// Note a preimage through the real pallet, which takes the deposit as a named hold. Returns the
/// hash.
pub fn note_preimage(who: &AccountId, bytes: Vec<u8>) -> <Test as frame_system::Config>::Hash {
	Preimage::note_preimage(RuntimeOrigin::signed(who.clone()), bytes.clone())
		.expect("can note a preimage");
	<Test as frame_system::Config>::Hashing::hash(&bytes)
}

/// A named hold no pallet accounts for any more, as a pallet that left the Relay Chain in AHM v1
/// left behind.
pub fn stray_hold(who: &AccountId, amount: u128) {
	<Balances as MutateHold<AccountId>>::hold(&PreimageHoldReason::get(), who, amount)
		.expect("can hold");
}

/// What `who` has on hold under the preimage reason.
pub fn held(who: &AccountId) -> u128 {
	<Balances as InspectHold<AccountId>>::balance_on_hold(&PreimageHoldReason::get(), who)
}

pub fn free(who: &AccountId) -> u128 {
	pallet_balances::Pallet::<Test>::free_balance(who)
}

/// Lock part of `who`'s balance, as staking and vesting did before they left the Relay Chain.
pub fn lock(who: &AccountId, amount: u128) {
	<Balances as LockableCurrency<AccountId>>::set_lock(
		*b"testlock",
		who,
		amount,
		WithdrawReasons::all(),
	);
}

/// Reserve part of `who`'s balance the way the registrar, HRMP, proxy and multisig pallets take
/// their deposits on the Relay Chain: an unnamed reserve.
pub fn reserve(who: &AccountId, amount: u128) {
	<Balances as ReservableCurrency<AccountId>>::reserve(who, amount).unwrap();
}
