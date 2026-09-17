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

//! Mock relay-chain runtime for `pallet-rc2-migrator` unit tests.

use crate as pallet_rc2_migrator;
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::{
	derive_impl, ord_parameter_types, parameter_types,
	traits::{Currency, InstanceFilter, OnFinalize, OnInitialize, ReservableCurrency, Time},
	weights::WeightMeter,
};
use frame_system::{EnsureRoot, EnsureSignedBy};
use pallet_message_queue::ForceSetHead;
use polkadot_parachain_primitives::primitives::{HrmpChannelId, Id as ParaId};
use polkadot_runtime_common::paras_registrar;
use runtime_parachains::{
	configuration, dmp, hrmp as parachains_hrmp, inclusion::AggregateMessageOrigin,
	origin as parachains_origin, paras, shared,
};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
	transaction_validity::TransactionPriority,
	AccountId32, BuildStorage,
};
use xcm::prelude::*;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlockU32<Test>;

frame_support::construct_runtime!(
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
		Multisig: pallet_multisig,
		Proxy: pallet_proxy,
		Preimage: pallet_preimage,
		Rc2Migrator: pallet_rc2_migrator,
	}
);

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
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<AccountId32>;
	type Block = Block;
	type AccountData = pallet_balances::AccountData<u128>;
}

/// The relay chain's existential deposit.
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
	type CooldownRemovalMultiplier = sp_core::ConstUint<1>;
	type AuthorizeCurrentCodeOrigin = EnsureRoot<AccountId32>;
}

impl dmp::Config for Test {}

parameter_types! {
	pub const DefaultChannelSizeAndCapacityWithSystem: (u32, u32) = (4096, 4);
}

impl parachains_hrmp::Config for Test {
	type ParaSelfOrigin = runtime_parachains::origin::EnsureParachain;
	type ParaRequests = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type ChannelManager = EnsureRoot<AccountId32>;
	type Currency = Balances;
	type DefaultChannelSizeAndCapacityWithSystem = DefaultChannelSizeAndCapacityWithSystem;
	type VersionWrapper = ();
	type WeightInfo = parachains_hrmp::TestWeightInfo;
}

parameter_types! {
	pub const ParaDeposit: u128 = 300;
	pub const DataDepositPerByte: u128 = 1;
}

impl paras_registrar::Config for Test {
	type ParaSelfOrigin = runtime_parachains::origin::EnsureParachain;
	type ParaRequests = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type OnSwap = ();
	type ParaDeposit = ParaDeposit;
	type DataDepositPerByte = DataDepositPerByte;
	type WeightInfo = paras_registrar::TestWeightInfo;
}

parameter_types! {
	pub const MultisigDepositBase: u128 = 30;
	pub const MultisigDepositFactor: u128 = 5;
}

impl pallet_multisig::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = MultisigDepositBase;
	type DepositFactor = MultisigDepositFactor;
	type MaxSignatories = frame_support::traits::ConstU32<5>;
	type WeightInfo = ();
	type BlockNumberProvider = System;
}

/// Relay-side proxy permissions: two portable ones and one (`Staking`) that the destination does
/// not represent, mirroring the production `TryFrom` split.
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

impl TryFrom<ProxyType> for migrator_types::PortableProxyType {
	type Error = ();

	fn try_from(t: ProxyType) -> Result<Self, ()> {
		use migrator_types::PortableProxyType as P;
		match t {
			ProxyType::Any => Ok(P::Any),
			ProxyType::NonTransfer => Ok(P::NonTransfer),
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

parameter_types! {
	pub const ProxyDepositBase: u128 = 40;
	pub const ProxyDepositFactor: u128 = 4;
	pub const AnnouncementDepositBase: u128 = 25;
	pub const AnnouncementDepositFactor: u128 = 6;
	pub const MaxProxies: u16 = 4;
	pub const MaxPending: u16 = 4;
}

parameter_types! {
	pub const PreimageBaseDeposit: u128 = 1;
	pub const PreimageByteDeposit: u128 = 1;
	pub const PreimageHoldReason: RuntimeHoldReason =
		RuntimeHoldReason::Preimage(pallet_preimage::HoldReason::Preimage);
}

impl pallet_preimage::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Currency = Balances;
	// Root, matching both live relay runtimes: the migration unnotes every preimage as Root
	// during `AccountsInit`.
	type ManagerOrigin = EnsureRoot<AccountId32>;
	type Consideration = frame_support::traits::fungible::HoldConsideration<
		AccountId32,
		Balances,
		PreimageHoldReason,
		frame_support::traits::LinearStoragePrice<PreimageBaseDeposit, PreimageByteDeposit, u128>,
	>;
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

// ---------------------------------------------------------------------------
// XCM capture
// ---------------------------------------------------------------------------

/// Storage key under which the router records delivered messages. Storage (not a thread-local)
/// on purpose: the production DMP router enqueues into storage, so a rolled-back block must also
/// roll its sends back here — tests of the retry path rely on that.
const SENT_XCM_KEY: &[u8] = b":test:sent_xcm";

parameter_types! {
	/// Make every send fail, to exercise the rollback/retry paths.
	pub static FailSends: bool = false;
}

pub struct TestXcmRouter;
impl SendXcm for TestXcmRouter {
	type Ticket = (Location, Xcm<()>);

	fn validate(
		dest: &mut Option<Location>,
		msg: &mut Option<Xcm<()>>,
	) -> SendResult<Self::Ticket> {
		if FailSends::get() {
			return Err(SendError::Transport("test-induced failure"));
		}
		Ok(((dest.take().unwrap(), msg.take().unwrap()), Assets::new()))
	}

	fn deliver(ticket: Self::Ticket) -> Result<XcmHash, SendError> {
		let mut sent = sent_xcm();
		sent.push(ticket);
		frame_support::storage::unhashed::put(SENT_XCM_KEY, &sent);
		Ok(XcmHash::default())
	}
}

/// All messages sent so far, in order, as `(destination, message)`.
pub fn sent_xcm() -> Vec<(Location, Xcm<()>)> {
	frame_support::storage::unhashed::get(SENT_XCM_KEY).unwrap_or_default()
}

pub fn take_sent_xcm() -> Vec<(Location, Xcm<()>)> {
	let sent = sent_xcm();
	frame_support::storage::unhashed::kill(SENT_XCM_KEY);
	sent
}

/// Decode every `Transact` sent to the Coretime chain into the migrator's call type.
pub fn decode_ct_calls(msgs: &[(Location, Xcm<()>)]) -> Vec<crate::CtMigratorCall> {
	let ct: Location = Location::new(0, [Parachain(CT_PARA_ID)]);
	msgs.iter()
		.filter(|(dest, _)| *dest == ct)
		.flat_map(|(_, xcm)| xcm.0.iter())
		.filter_map(|instruction| match instruction {
			Transact { call, .. } => {
				let call = crate::CtRuntimeCall::decode(&mut &call.clone().into_encoded()[..])
					.expect("sent Transacts carry a decodable CtRuntimeCall");
				let crate::CtRuntimeCall::CtMigrator(call) = call;
				Some(call)
			},
			_ => None,
		})
		.collect()
}

/// The teleport messages sent to Asset Hub, reduced to their beneficiary payouts.
pub fn decode_teleports(msgs: &[(Location, Xcm<()>)]) -> Vec<Vec<(AccountId32, u128)>> {
	let ah: Location = Location::new(0, [Parachain(AH_PARA_ID)]);
	msgs.iter()
		.filter(|(dest, _)| *dest == ah)
		.map(|(_, xcm)| {
			xcm.0
				.iter()
				.filter_map(|instruction| match instruction {
					DepositAsset { assets, beneficiary } => {
						let AssetFilter::Definite(assets) = assets else { panic!("definite") };
						let Fungibility::Fungible(amount) = assets.get(0).unwrap().fun else {
							panic!("fungible")
						};
						let Some(Junction::AccountId32 { id, .. }) = beneficiary.interior().first()
						else {
							panic!("account beneficiary")
						};
						Some((AccountId32::new(*id), amount))
					},
					_ => None,
				})
				.collect()
		})
		.collect()
}

// ---------------------------------------------------------------------------
// Migrator config
// ---------------------------------------------------------------------------

pub const CT_PARA_ID: u32 = 1005;
pub const AH_PARA_ID: u32 = 1000;

parameter_types! {
	pub const CtParaId: u32 = CT_PARA_ID;
	/// Members of the manager multisig; set per test.
	pub static MultisigMembers: Vec<AccountId32> = vec![];
	pub const MultisigThreshold: u32 = 2;
	pub const MultisigMaxVotesPerRound: u32 = 3;
	pub const MultisigStartRound: u32 = 7;
	/// Every queue the pallet forced to the head, in order.
	pub static ForcedHeads: Vec<AggregateMessageOrigin> = vec![];
	/// Three blocks of priority, one of round robin.
	pub const CtUmpQueuePriorityPattern: (u32, u32) = (3, 1);
	pub const AhParaId: u32 = AH_PARA_ID;
	/// Working buffer that follows deposits to the Coretime chain.
	pub const CtFreeBuffer: u128 = 100;
	/// Asset Hub's ED: half the relay's in this mock, so the dust-follows-deposit rule has a
	/// window (a teleport of 1..=4 is valid nowhere).
	pub const AhExistentialDeposit: u128 = 5;
	pub SweepAccounts: Vec<AccountId32> = vec![pot()];
	pub SweepBeneficiary: AccountId32 = acc(200);
	/// Audited phantom issuance; set per test.
	pub static TiCorrection: u128 = 0;
	/// The mock's wall clock, advanced by `run_blocks`.
	pub static MockNow: u64 = 0;
}

ord_parameter_types! {
	/// The account the mock treats as the Coretime chain's dispatch origin.
	pub const CoretimeAccount: AccountId32 = AccountId32::new([205u8; 32]);
}

/// How long the migration holds in each of its two lockdown windows, in this mock.
pub const WARM_UP: u32 = 4;
pub const COOL_OFF: u32 = 10;

/// Milliseconds per block, so the wall clock and the block number stay in step.
pub const BLOCK_TIME_MS: u64 = 6_000;

/// Wall clock the schedule is compared against.
pub struct MockTime;
impl Time for MockTime {
	type Moment = u64;
	fn now() -> Self::Moment {
		MockNow::get()
	}
}

/// The mock clock's current value.
pub fn now_ms() -> u64 {
	MockTime::now()
}

/// Records which queue was put first instead of touching a real message queue.
pub struct RecordingHead;
impl ForceSetHead<AggregateMessageOrigin> for RecordingHead {
	fn force_set_head(_: &mut WeightMeter, origin: &AggregateMessageOrigin) -> Result<bool, ()> {
		ForcedHeads::mutate(|forced| forced.push(origin.clone()));
		Ok(true)
	}
}

impl pallet_rc2_migrator::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type SendXcm = TestXcmRouter;
	type CtParaId = CtParaId;
	type AhParaId = AhParaId;
	type CtFreeBuffer = CtFreeBuffer;
	type AhExistentialDeposit = AhExistentialDeposit;
	type SweepAccounts = SweepAccounts;
	type SweepBeneficiary = SweepBeneficiary;
	type TiCorrection = TiCorrection;
	type TimeProvider = MockTime;
	type CtOrigin = EnsureSignedBy<CoretimeAccount, AccountId32>;
	type AdminOrigin = EnsureRoot<AccountId32>;
	type RuntimeCall = RuntimeCall;
	type MultisigMembers = MultisigMembers;
	type MultisigThreshold = MultisigThreshold;
	type MultisigMaxVotesPerRound = MultisigMaxVotesPerRound;
	type MultisigStartRound = MultisigStartRound;
	type MessageQueue = RecordingHead;
	type CtUmpQueuePriorityPattern = CtUmpQueuePriorityPattern;
	type XcmResponseTimeout = XcmResponseTimeout;
	type UnprocessedMsgBuffer = UnprocessedMsgBufferSize;
	type NotifyQueryHandler = CountingQueries;
	// The response arrives as a plain signed call from the Coretime account in these tests; the
	// real runtime distinguishes a query response from a Coretime-chain call, which is a
	// distinction `pallet-xcm` draws and this mock has no executor to reproduce.
	type ResponseOrigin = EnsureSignedBy<CoretimeAccount, AccountId32>;
}

parameter_types! {
	pub const XcmResponseTimeout: u32 = 10;
	/// One in flight at a time, so a test that sends two batches exercises the gate.
	pub const UnprocessedMsgBufferSize: u32 = 1;
}

/// Hands out sequential query ids, so a test can name the query a given batch registered.
pub struct CountingQueries;
impl pallet_rc2_migrator::NotifyQueryHandler<Test> for CountingQueries {
	fn new_notify_query(_responder: Location, _timeout: u32) -> u64 {
		let next = NextQueryId::get();
		NextQueryId::set(next + 1);
		next
	}
}

parameter_types! {
	pub static NextQueryId: u64 = 0;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	// A consistent host configuration; the zeroed default fails the genesis consistency check.
	configuration::GenesisConfig::<Test> {
		config: configuration::HostConfiguration {
			max_code_size: 3 * 1024 * 1024,
			max_head_data_size: 1024 * 1024,
			..Default::default()
		},
	}
	.assimilate_storage(&mut t)
	.unwrap();

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

/// The Coretime chain, as an origin this mock can dispatch from.
pub fn coretime() -> AccountId32 {
	CoretimeAccount::get()
}

/// Run `n` blocks of the stage machine, with a Coretime chain that accepts every batch.
///
/// Each block's outgoing batches are confirmed before the next one, which is what the stage
/// machine waits for. Tests that care about a batch being rejected or ignored drive the blocks
/// themselves; see [`run_blocks_without_ct`].
///
/// The clock moves *after* the block's hooks, mirroring the timestamp inherent, which is an
/// extrinsic and so runs after `on_initialize`.
pub fn run_blocks(n: u32) {
	for _ in 0..n {
		run_blocks_without_ct(1);
		confirm_pending_batches();
	}
}

/// Run `n` blocks with no reply from the Coretime chain, leaving every batch unconfirmed.
pub fn run_blocks_without_ct(n: u32) {
	for _ in 0..n {
		let now = System::block_number() + 1;
		System::set_block_number(now);
		Rc2Migrator::on_initialize(now);
		Rc2Migrator::on_finalize(now);
		MockNow::set(u64::from(now).saturating_mul(BLOCK_TIME_MS));
	}
}

/// Answer every outstanding batch with a success, as a healthy Coretime chain would.
pub fn confirm_pending_batches() {
	for query_id in crate::UnconfirmedBatches::<Test>::iter_keys().collect::<Vec<_>>() {
		Rc2Migrator::receive_query_response(
			RuntimeOrigin::signed(coretime()),
			query_id,
			Response::DispatchResult(MaybeErrorCode::Success),
		)
		.expect("the mock Coretime chain accepts every batch");
	}
}

/// A pallet (module) account: the kind the migration leaves for the sweep stage.
pub fn pot() -> AccountId32 {
	let mut bytes = [0u8; 32];
	bytes[..12].copy_from_slice(b"modlpy/trsry");
	AccountId32::new(bytes)
}

/// The child sovereign account of a para on the relay chain (`para…`).
pub fn child_sov(para: u32) -> AccountId32 {
	ParaId::from(para).into_account_truncating()
}

pub fn fund(who: &AccountId32, amount: u128) {
	let _ = <Balances as Currency<AccountId32>>::make_free_balance_be(who, amount);
}

pub fn free(who: &AccountId32) -> u128 {
	pallet_balances::Pallet::<Test>::free_balance(who)
}

pub fn reserved(who: &AccountId32) -> u128 {
	pallet_balances::Pallet::<Test>::reserved_balance(who)
}

pub fn total_issuance() -> u128 {
	pallet_balances::TotalIssuance::<Test>::get()
}

/// Register a para through the real registrar path (`reserve`): records `ParaDeposit` (= 300)
/// against the manager and reserves it, exactly like mainnet state.
pub fn register_para(id: u32, manager: &AccountId32) {
	paras_registrar::NextFreeParaId::<Test>::put(ParaId::from(id));
	Registrar::reserve(RuntimeOrigin::signed(manager.clone())).expect("manager can reserve");
}

/// Insert an HRMP channel with its deposits reserved on the child sovereigns — the state the
/// real channel-open handshake leaves behind (the handshake itself needs live paras + sessions,
/// far beyond unit scope).
pub fn open_channel(sender: u32, recipient: u32, sender_deposit: u128, recipient_deposit: u128) {
	for (para, deposit) in [(sender, sender_deposit), (recipient, recipient_deposit)] {
		let sov = child_sov(para);
		fund(&sov, free(&sov) + deposit + ED);
		<Balances as ReservableCurrency<AccountId32>>::reserve(&sov, deposit).unwrap();
	}
	let id = HrmpChannelId { sender: sender.into(), recipient: recipient.into() };
	parachains_hrmp::HrmpChannels::<Test>::insert(
		&id,
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

/// Insert a pending open-channel request with the sender deposit reserved, mirroring
/// `hrmp_init_open_channel`'s end state.
pub fn open_request(sender: u32, recipient: u32, deposit: u128) {
	let sov = child_sov(sender);
	fund(&sov, free(&sov) + deposit + ED);
	<Balances as ReservableCurrency<AccountId32>>::reserve(&sov, deposit).unwrap();
	let id = HrmpChannelId { sender: sender.into(), recipient: recipient.into() };
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

/// Grant a proxy through the real pallet path; reserves the deposit at this chain's rates.
/// Calling the dispatchable directly does not bump the delegator's nonce, so a never-signed
/// delegator stays at nonce 0 — exactly how pures and multisigs look on chain.
pub fn add_proxy(delegator: &AccountId32, delegate: &AccountId32, proxy_type: ProxyType) {
	Proxy::add_proxy(RuntimeOrigin::signed(delegator.clone()), delegate.clone(), proxy_type, 0)
		.expect("can add proxy");
}

/// All `pallet-rc2-migrator` events since the last call to this function.
pub fn migrator_events() -> Vec<crate::Event<Test>> {
	let events = System::events()
		.into_iter()
		.filter_map(|r| match r.event {
			RuntimeEvent::Rc2Migrator(e) => Some(e),
			_ => None,
		})
		.collect();
	System::reset_events();
	events
}

/// Create the below-ED / broken-refcount account shapes the sweep stage exists to clean up.
/// These cannot be produced through the balances API (it refuses sub-ED accounts), which is the
/// point: they are pre-existing on-chain anomalies the migrator must handle.
pub fn force_anomalous_account(who: &AccountId32, free: u128, reserved: u128, consumers: u32) {
	let _ = frame_system::Pallet::<Test>::inc_providers(who);
	frame_system::Account::<Test>::mutate(who, |a| {
		a.data.free = free;
		a.data.reserved = reserved;
	});
	for _ in 0..consumers {
		frame_system::Pallet::<Test>::inc_consumers(who).unwrap();
	}
	pallet_balances::TotalIssuance::<Test>::mutate(|ti| *ti += free + reserved);
}
