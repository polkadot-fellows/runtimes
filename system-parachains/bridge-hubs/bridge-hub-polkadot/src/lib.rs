// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Cumulus.

// Cumulus is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Cumulus is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Cumulus.  If not, see <http://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

extern crate alloc;

pub mod bridge_common_config;
pub mod bridge_to_ethereum_config;
pub mod bridge_to_kusama_config;
// Genesis preset configurations.
pub mod genesis_config_presets;
mod weights;
pub mod xcm_config;

use alloc::{borrow::Cow, vec, vec::Vec};
use bridge_hub_common::message_queue::{
	AggregateMessageOrigin, NarrowOriginToSibling, ParaIdToSibling,
};
use bridge_to_kusama_config::bp_kusama;
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use cumulus_pallet_parachain_system::RelayNumberMonotonicallyIncreases;
use cumulus_primitives_core::ParaId;
use snowbridge_core::{AgentId, PricingParameters};
use snowbridge_outbound_queue_primitives::v1::{Command, Fee};

use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::{
	generic, impl_opaque_keys,
	traits::{AccountIdLookup, BlakeTwo256, Block as BlockT, Get},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult,
};

#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

use frame_support::{
	construct_runtime,
	dispatch::DispatchClass,
	genesis_builder_helper::{build_state, get_preset},
	parameter_types,
	traits::{
		tokens::imbalance::ResolveTo, ConstBool, ConstU32, ConstU64, ConstU8, EitherOf,
		EitherOfDiverse, Everything, InstanceFilter, TransformOrigin,
	},
	weights::{ConstantMultiplier, Weight},
	PalletId,
};
use frame_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot,
};
use pallet_xcm::{EnsureXcm, IsVoiceOfBody};
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;
pub use sp_runtime::{MultiAddress, Perbill, Permill};
use xcm_config::{
	AssetHubLocation, FellowshipLocation, RelayChainLocation, StakingPot,
	XcmOriginToTransactDispatchOrigin,
};

#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

// Polkadot imports
use polkadot_runtime_common::{BlockHashCount, SlowAdjustingFeeUpdate};

use weights::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight};

use parachains_common::{AccountId, Balance, BlockNumber, Hash, Header, Nonce, Signature};
pub use system_parachains_constants::SLOT_DURATION;

use system_parachains_constants::{
	polkadot::{consensus::*, currency::*, fee::WeightToFee},
	AVERAGE_ON_INITIALIZE_RATIO, HOURS, MAXIMUM_BLOCK_WEIGHT, NORMAL_DISPATCH_RATIO,
};

// XCM Imports
use xcm::prelude::*;
use xcm_runtime_apis::{
	dry_run::{CallDryRunEffects, Error as XcmDryRunApiError, XcmDryRunEffects},
	fees::Error as XcmPaymentApiError,
};

/// The address format for describing accounts.
pub type Address = MultiAddress<AccountId, ()>;

/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;

/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;

/// The `TransactionExtension` to the basic transaction logic.
pub type TxExtension = cumulus_pallet_weight_reclaim::StorageWeightReclaim<
	Runtime,
	(
		frame_system::CheckNonZeroSender<Runtime>,
		frame_system::CheckSpecVersion<Runtime>,
		frame_system::CheckTxVersion<Runtime>,
		frame_system::CheckGenesis<Runtime>,
		frame_system::CheckEra<Runtime>,
		frame_system::CheckNonce<Runtime>,
		frame_system::CheckWeight<Runtime>,
		pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
		BridgeRejectObsoleteHeadersAndMessages,
		bridge_to_kusama_config::OnBridgeHubPolkadotRefundBridgeHubKusamaMessages,
		frame_metadata_hash_extension::CheckMetadataHash<Runtime>,
	),
>;

bridge_runtime_common::generate_bridge_reject_obsolete_headers_and_messages! {
	RuntimeCall, AccountId,
	// Grandpa
	BridgeKusamaGrandpa,
	// Parachains
	BridgeKusamaParachains,
	// Messages
	BridgeKusamaMessages
}

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, TxExtension>;

parameter_types! {
	pub const EthereumSystemPalletName: &'static str = "EthereumSystem";
	pub const NativeToForeignIdKey: &'static str = "NativeToForeignId";
}

/// The runtime migrations per release.
#[allow(deprecated, missing_docs)]
pub mod migrations {
	use super::*;

	/// Unreleased migrations. Add new ones here:
	pub type Unreleased = ();

	/// Migrations/checks that do not need to be versioned and can run on every update.
	pub type Permanent = pallet_xcm::migration::MigrateToLatestXcmVersion<Runtime>;

	/// All migrations that will run on the next runtime upgrade.
	pub type SingleBlockMigrations = (Unreleased, Permanent);

	/// MBM migrations to apply on runtime upgrade.
	pub type MbmMigrations = ();
}

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: Aura,
	}
}

#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: Cow::Borrowed("bridge-hub-polkadot"),
	impl_name: Cow::Borrowed("bridge-hub-polkadot"),
	authoring_version: 1,
	spec_version: 2_001_000,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 4,
	system_version: 1,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
	pub const SS58Prefix: u8 = 0;
}

// Configure FRAME pallets to include in runtime.

impl frame_system::Config for Runtime {
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = AccountIdLookup<AccountId, ()>;
	/// The index type for storing how many extrinsics an account has signed.
	type Nonce = Nonce;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The block type.
	type Block = Block;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type RuntimeTask = RuntimeTask;
	/// The ubiquitous origin type.
	type RuntimeOrigin = RuntimeOrigin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// Runtime version.
	type Version = Version;
	/// Converts a module to an index of this module in the runtime.
	type PalletInfo = PalletInfo;
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = Everything;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = weights::frame_system::WeightInfo<Runtime>;
	/// Weight information for the extensions from this pallet.
	type ExtensionsWeightInfo = weights::frame_system_extensions::WeightInfo<Runtime>;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = RuntimeBlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = RuntimeBlockLength;
	type SS58Prefix = SS58Prefix;
	/// The action to take on a Runtime Upgrade
	type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
	type MaxConsumers = ConstU32<16>;
	type SingleBlockMigrations = migrations::SingleBlockMigrations;
	type MultiBlockMigrator = ();
	type PreInherents = ();
	type PostInherents = ();
	type PostTransactions = ();
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
	type WeightInfo = weights::pallet_timestamp::WeightInfo<Runtime>;
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
	type EventHandler = (CollatorSelection,);
}

parameter_types! {
	pub const ExistentialDeposit: Balance = SYSTEM_PARA_EXISTENTIAL_DEPOSIT;
}

impl pallet_balances::Config for Runtime {
	/// The type for recording an account's balance.
	type Balance = Balance;
	type DustRemoval = ();
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = weights::pallet_balances::WeightInfo<Runtime>;
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = ();
	type MaxFreezes = ConstU32<0>;
	type DoneSlashHandler = ();
}

parameter_types! {
	/// Relay Chain `TransactionByteFee` / 10
	pub const TransactionByteFee: Balance = system_parachains_constants::polkadot::fee::TRANSACTION_BYTE_FEE;
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction =
		pallet_transaction_payment::FungibleAdapter<Balances, ResolveTo<StakingPot, Balances>>;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightToFee = WeightToFee<Self>;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
	type WeightInfo = weights::pallet_transaction_payment::WeightInfo<Self>;
}

parameter_types! {
	pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
	pub const RelayOrigin: AggregateMessageOrigin = AggregateMessageOrigin::Parent;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnSystemEvent = ();
	type SelfParaId = parachain_info::Pallet<Runtime>;
	type OutboundXcmpMessageSource = XcmpQueue;
	type DmpQueue = frame_support::traits::EnqueueWithOrigin<MessageQueue, RelayOrigin>;
	type ReservedDmpWeight = ReservedDmpWeight;
	type XcmpMessageHandler = XcmpQueue;
	type ReservedXcmpWeight = ReservedXcmpWeight;
	type CheckAssociatedRelayNumber = RelayNumberMonotonicallyIncreases;
	type ConsensusHook = ConsensusHook;
	type WeightInfo = weights::cumulus_pallet_parachain_system::WeightInfo<Runtime>;
	type RelayParentOffset = ConstU32<0>;
}

type ConsensusHook = cumulus_pallet_aura_ext::FixedVelocityConsensusHook<
	Runtime,
	RELAY_CHAIN_SLOT_DURATION_MILLIS,
	BLOCK_PROCESSING_VELOCITY,
	UNINCLUDED_SEGMENT_CAPACITY,
>;

impl parachain_info::Config for Runtime {}

parameter_types! {
	/// Amount of weight that can be spent per block to service messages. This was increased
	/// from 35% to 60% of the max block weight to accommodate the Ethereum beacon light client
	/// extrinsics. The `force_checkpoint` and `submit` extrinsics (for submit, optionally) includes
	/// the sync committee's pubkeys (512 x 48 bytes).
	/// Furthermore, Bridge Hub is a specialized chain for moving messages between sibling
	/// parachains and external ecosystems. As such, most of the block weight is expected to be
	/// consumed by the Message Queue.
	pub MessageQueueServiceWeight: Weight = Perbill::from_percent(60) * RuntimeBlockWeights::get().max_block;
	pub MessageQueueIdleServiceWeight: Weight = Perbill::from_percent(20) * RuntimeBlockWeights::get().max_block;
}

impl pallet_message_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_message_queue::WeightInfo<Runtime>;
	// Use the NoopMessageProcessor exclusively for benchmarks, not for tests with the
	// runtime-benchmarks feature as tests require the BridgeHubMessageRouter to process messages.
	// The "test" feature flag doesn't work, hence the reliance on the "std" feature, which is
	// enabled during tests.
	#[cfg(all(not(feature = "std"), feature = "runtime-benchmarks"))]
	type MessageProcessor =
		pallet_message_queue::mock_helpers::NoopMessageProcessor<AggregateMessageOrigin>;
	#[cfg(not(all(not(feature = "std"), feature = "runtime-benchmarks")))]
	type MessageProcessor = bridge_hub_common::BridgeHubDualMessageRouter<
		xcm_builder::ProcessXcmMessage<
			AggregateMessageOrigin,
			xcm_executor::XcmExecutor<xcm_config::XcmConfig>,
			RuntimeCall,
		>,
		EthereumOutboundQueue,
		EthereumOutboundQueueV2,
	>;
	type Size = u32;
	// The XCMP queue pallet is only ever able to handle the `Sibling(ParaId)` origin:
	type QueueChangeHandler = NarrowOriginToSibling<XcmpQueue>;
	type QueuePausedQuery = NarrowOriginToSibling<XcmpQueue>;
	type HeapSize = ConstU32<{ 64 * 1024 }>;
	type MaxStale = ConstU32<8>;
	type ServiceWeight = MessageQueueServiceWeight;
	type IdleMaxServiceWeight = MessageQueueIdleServiceWeight;
}

impl cumulus_pallet_aura_ext::Config for Runtime {}

parameter_types! {
	// Fellows pluralistic body.
	pub const FellowsBodyId: BodyId = BodyId::Technical;
	/// The asset ID for the asset that we use to pay for message delivery fees.
	pub FeeAssetId: AssetId = AssetId(xcm_config::DotRelayLocation::get());
	/// The base fee for the message delivery fees.
	pub const ToSiblingBaseDeliveryFee: u128 = CENTS.saturating_mul(3);
	pub const ToParentBaseDeliveryFee: u128 = CENTS.saturating_mul(3);
}

/// Privileged origin that represents Root or Fellows.
pub type RootOrFellows = EitherOfDiverse<
	EnsureRoot<AccountId>,
	EnsureXcm<IsVoiceOfBody<FellowshipLocation, FellowsBodyId>>,
>;

pub type PriceForSiblingParachainDelivery = polkadot_runtime_common::xcm_sender::ExponentialPrice<
	FeeAssetId,
	ToSiblingBaseDeliveryFee,
	TransactionByteFee,
	XcmpQueue,
>;
pub type PriceForParentDelivery = polkadot_runtime_common::xcm_sender::ExponentialPrice<
	FeeAssetId,
	ToParentBaseDeliveryFee,
	TransactionByteFee,
	ParachainSystem,
>;

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ChannelInfo = ParachainSystem;
	type VersionWrapper = PolkadotXcm;
	// Enqueue XCMP messages from siblings for later processing.
	type XcmpQueue = TransformOrigin<MessageQueue, AggregateMessageOrigin, ParaId, ParaIdToSibling>;
	type MaxActiveOutboundChannels = ConstU32<128>;
	// Most on-chain HRMP channels are configured to use 102400 bytes of max message size, so we
	// need to set the page size larger than that until we reduce the channel size on-chain.
	type MaxPageSize = ConstU32<{ 103 * 1024 }>;
	type MaxInboundSuspended = ConstU32<1_000>;
	type ControllerOrigin = RootOrFellows;
	type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
	type WeightInfo = weights::cumulus_pallet_xcmp_queue::WeightInfo<Runtime>;
	type PriceForSiblingDelivery = PriceForSiblingParachainDelivery;
}

impl cumulus_pallet_xcmp_queue::migration::v5::V5Config for Runtime {
	// This must be the same as the `ChannelInfo` from the `Config`:
	type ChannelList = ParachainSystem;
}

pub const PERIOD: u32 = 6 * HOURS;
pub const OFFSET: u32 = 0;

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	// we don't have stash and controller, thus we don't need the convert as well.
	type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
	type ShouldEndSession = pallet_session::PeriodicSessions<ConstU32<PERIOD>, ConstU32<OFFSET>>;
	type NextSessionRotation = pallet_session::PeriodicSessions<ConstU32<PERIOD>, ConstU32<OFFSET>>;
	type SessionManager = CollatorSelection;
	// Essentially just Aura, but let's be pedantic.
	type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type WeightInfo = weights::pallet_session::WeightInfo<Runtime>;
	type DisablingStrategy = ();
	type Currency = Balances;
	type KeyDeposit = ();
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = ConstU32<100_000>;
	type AllowMultipleBlocksPerSlot = ConstBool<false>;
	type SlotDuration = ConstU64<SLOT_DURATION>;
}

parameter_types! {
	pub const PotId: PalletId = PalletId(*b"PotStake");
	pub const SessionLength: BlockNumber = 6 * HOURS;
	// StakingAdmin pluralistic body.
	pub const StakingAdminBodyId: BodyId = BodyId::Defense;
}

/// We allow root, the StakingAdmin to execute privileged collator selection operations.
pub type CollatorSelectionUpdateOrigin = EitherOfDiverse<
	EnsureRoot<AccountId>,
	EitherOf<
		EnsureXcm<IsVoiceOfBody<RelayChainLocation, StakingAdminBodyId>>,
		EnsureXcm<IsVoiceOfBody<AssetHubLocation, StakingAdminBodyId>>,
	>,
>;

impl pallet_collator_selection::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type UpdateOrigin = CollatorSelectionUpdateOrigin;
	type PotId = PotId;
	type MaxCandidates = ConstU32<100>;
	type MinEligibleCollators = ConstU32<4>;
	type MaxInvulnerables = ConstU32<20>;
	// should be a multiple of session or things will get inconsistent
	type KickThreshold = ConstU32<PERIOD>;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
	type ValidatorRegistration = Session;
	type WeightInfo = weights::pallet_collator_selection::WeightInfo<Runtime>;
}

parameter_types! {
	// One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
	pub const DepositBase: Balance = system_para_deposit(1, 88);
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = system_para_deposit(0, 32);
}

impl pallet_multisig::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = ConstU32<100>;
	type WeightInfo = weights::pallet_multisig::WeightInfo<Runtime>;
	type BlockNumberProvider = System;
}

impl pallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = weights::pallet_utility::WeightInfo<Runtime>;
}

/// The type used to represent the kinds of proxying allowed.
#[derive(
	Copy,
	Clone,
	Debug,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Encode,
	Decode,
	DecodeWithMemTracking,
	MaxEncodedLen,
	scale_info::TypeInfo,
)]
pub enum ProxyType {
	/// Fully permissioned proxy. Can execute any call on behalf of _proxied_.
	Any,
	/// Can execute any call that does not transfer funds or assets.
	NonTransfer,
	/// Proxy with the ability to reject time-delay proxy announcements.
	CancelProxy,
	/// Collator selection proxy. Can execute calls related to collator selection mechanism.
	Collator,
}

impl Default for ProxyType {
	fn default() -> Self {
		Self::Any
	}
}

impl InstanceFilter<RuntimeCall> for ProxyType {
	fn filter(&self, c: &RuntimeCall) -> bool {
		match self {
			ProxyType::Any => true,
			ProxyType::NonTransfer => matches!(
				c,
				RuntimeCall::System(_) |
					RuntimeCall::ParachainSystem(_) |
					RuntimeCall::Timestamp(_) |
					RuntimeCall::CollatorSelection(_) |
					RuntimeCall::Session(_) |
					RuntimeCall::Utility(_) |
					RuntimeCall::Multisig(_) |
					RuntimeCall::Proxy(_) |
					RuntimeCall::BridgeRelayers(pallet_bridge_relayers::Call::register { .. }) |
					RuntimeCall::BridgeRelayers(pallet_bridge_relayers::Call::deregister { .. }) |
					RuntimeCall::BridgeRelayers(
						pallet_bridge_relayers::Call::claim_rewards { .. }
					)
			),
			ProxyType::CancelProxy => matches!(
				c,
				RuntimeCall::Proxy(pallet_proxy::Call::reject_announcement { .. }) |
					RuntimeCall::Utility { .. } |
					RuntimeCall::Multisig { .. }
			),
			ProxyType::Collator => matches!(
				c,
				RuntimeCall::CollatorSelection { .. } |
					RuntimeCall::Utility { .. } |
					RuntimeCall::Multisig { .. }
			),
		}
	}

	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			(ProxyType::NonTransfer, ProxyType::Collator) => true,
			(ProxyType::NonTransfer, ProxyType::CancelProxy) => true,
			_ => false,
		}
	}
}

parameter_types! {
	// One storage item; key size 32, value size 8.
	pub const ProxyDepositBase: Balance = system_para_deposit(1, 40);
	// Additional storage item size of 33 bytes.
	pub const ProxyDepositFactor: Balance = system_para_deposit(0, 33);
	pub const MaxProxies: u16 = 32;
	// One storage item; key size 32, value size 16.
	pub const AnnouncementDepositBase: Balance = system_para_deposit(1, 48);
	pub const AnnouncementDepositFactor: Balance = system_para_deposit(0, 66);
	pub const MaxPending: u16 = 32;
}

impl pallet_proxy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type MaxProxies = MaxProxies;
	type WeightInfo = weights::pallet_proxy::WeightInfo<Runtime>;
	type MaxPending = MaxPending;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
	type BlockNumberProvider = System;
}

impl cumulus_pallet_weight_reclaim::Config for Runtime {
	type WeightInfo = weights::cumulus_pallet_weight_reclaim::WeightInfo<Runtime>;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub enum Runtime
	{
		// System support stuff.
		System: frame_system = 0,
		ParachainSystem: cumulus_pallet_parachain_system = 1,
		Timestamp: pallet_timestamp = 2,
		ParachainInfo: parachain_info = 3,
		WeightReclaim: cumulus_pallet_weight_reclaim = 4,

		// Monetary stuff.
		Balances: pallet_balances = 10,
		TransactionPayment: pallet_transaction_payment = 11,

		// Collator support. The order of these 4 are important and shall not change.
		Authorship: pallet_authorship = 20,
		CollatorSelection: pallet_collator_selection = 21,
		Session: pallet_session = 22,
		Aura: pallet_aura = 23,
		AuraExt: cumulus_pallet_aura_ext = 24,

		// XCM helpers.
		XcmpQueue: cumulus_pallet_xcmp_queue = 30,
		PolkadotXcm: pallet_xcm = 31,
		CumulusXcm: cumulus_pallet_xcm = 32,
		// DmpQueue: cumulus_pallet_dmp_queue = 33, removed

		// Handy utilities.
		Utility: pallet_utility = 40,
		Multisig: pallet_multisig = 41,
		Proxy: pallet_proxy = 42,

		// Pallets that may be used by all bridges.
		BridgeRelayers: pallet_bridge_relayers = 50,

		// Kusama bridge pallets.
		BridgeKusamaGrandpa: pallet_bridge_grandpa::<Instance1> = 51,
		BridgeKusamaParachains: pallet_bridge_parachains::<Instance1> = 52,
		BridgeKusamaMessages: pallet_bridge_messages::<Instance1> = 53,
		XcmOverBridgeHubKusama: pallet_xcm_bridge_hub::<Instance1> = 54,

		// Ethereum bridge pallets.
		EthereumInboundQueue: snowbridge_pallet_inbound_queue = 80,
		EthereumOutboundQueue: snowbridge_pallet_outbound_queue = 81,
		EthereumBeaconClient: snowbridge_pallet_ethereum_client = 82,
		EthereumSystem: snowbridge_pallet_system = 83,

		// Ethereum bridge pallets V2.
		EthereumSystemV2: snowbridge_pallet_system_v2 = 90,
		EthereumInboundQueueV2: snowbridge_pallet_inbound_queue_v2 = 91,
		EthereumOutboundQueueV2: snowbridge_pallet_outbound_queue_v2 = 92,

		// Message Queue. Importantly, it is registered after Snowbridge pallets
		// so that messages are processed after the `on_initialize` hooks of bridging pallets.
		MessageQueue: pallet_message_queue = 175,
	}
);

#[cfg(feature = "runtime-benchmarks")]
use pallet_bridge_messages::LaneIdOf;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	use super::*;
	use alloc::boxed::Box;
	use polkadot_runtime_constants::system_parachain::AssetHubParaId;
	use system_parachains_constants::polkadot::locations::AssetHubLocation;

	frame_benchmarking::define_benchmarks!(
		[frame_system, SystemBench::<Runtime>]
		[frame_system_extensions, SystemExtensionsBench::<Runtime>]
		[pallet_balances, Balances]
		[pallet_message_queue, MessageQueue]
		[pallet_multisig, Multisig]
		[pallet_proxy, Proxy]
		[pallet_session, SessionBench::<Runtime>]
		[pallet_utility, Utility]
		[pallet_timestamp, Timestamp]
		[pallet_transaction_payment, TransactionPayment]
		[pallet_collator_selection, CollatorSelection]
		[cumulus_pallet_parachain_system, ParachainSystem]
		[cumulus_pallet_weight_reclaim, WeightReclaim]
		[cumulus_pallet_xcmp_queue, XcmpQueue]
		// XCM
		[pallet_xcm, PalletXcmExtrinsicsBenchmark::<Runtime>]
		// NOTE: Make sure you point to the individual modules below.
		[pallet_xcm_benchmarks::fungible, XcmBalances]
		[pallet_xcm_benchmarks::generic, XcmGeneric]
		// Shared bridge pallets
		[pallet_bridge_relayers, BridgeRelayersBench::<Runtime>]
		// Polkadot bridge pallets.
		[pallet_bridge_grandpa, KusamaFinality]
		[pallet_bridge_parachains, KusamaParachains]
		[pallet_bridge_messages, KusamaMessages]
		// Ethereum Bridge
		[snowbridge_pallet_inbound_queue, EthereumInboundQueue]
		[snowbridge_pallet_outbound_queue, EthereumOutboundQueue]
		[snowbridge_pallet_system, EthereumSystem]
		[snowbridge_pallet_ethereum_client, EthereumBeaconClient]
		[snowbridge_pallet_inbound_queue_v2, EthereumInboundQueueV2]
		[snowbridge_pallet_outbound_queue_v2, EthereumOutboundQueueV2]
		[snowbridge_pallet_system_v2, EthereumSystemV2]
	);

	impl frame_system_benchmarking::Config for Runtime {
		fn setup_set_code_requirements(code: &Vec<u8>) -> Result<(), BenchmarkError> {
			ParachainSystem::initialize_for_set_code_benchmark(code.len() as u32);
			Ok(())
		}

		fn verify_set_code() {
			System::assert_last_event(
				cumulus_pallet_parachain_system::Event::<Runtime>::ValidationFunctionStored.into(),
			);
		}
	}

	impl cumulus_pallet_session_benchmarking::Config for Runtime {}

	use xcm_config::DotRelayLocation;

	parameter_types! {
		pub ExistentialDepositAsset: Option<Asset> = Some((
			DotRelayLocation::get(),
			ExistentialDeposit::get()
		).into());
	}

	impl pallet_xcm::benchmarking::Config for Runtime {
		type DeliveryHelper = polkadot_runtime_common::xcm_sender::ToParachainDeliveryHelper<
			xcm_config::XcmConfig,
			ExistentialDepositAsset,
			PriceForSiblingParachainDelivery,
			AssetHubParaId,
			ParachainSystem,
		>;

		fn reachable_dest() -> Option<Location> {
			Some(AssetHubLocation::get())
		}

		fn teleportable_asset_and_dest() -> Option<(Asset, Location)> {
			// Relay/native token can be teleported between BH and Asset Hub.
			Some((
				Asset { fun: Fungible(ExistentialDeposit::get()), id: AssetId(Parent.into()) },
				AssetHubLocation::get(),
			))
		}

		fn reserve_transferable_asset_and_dest() -> Option<(Asset, Location)> {
			// Reserve transfers are disabled on BH.
			None
		}

		fn set_up_complex_asset_transfer() -> Option<(Assets, u32, Location, Box<dyn FnOnce()>)> {
			// Only supports native token teleports to system parachain
			let native_location = Parent.into();
			let dest = AssetHubLocation::get();

			pallet_xcm::benchmarking::helpers::native_teleport_as_asset_transfer::<Runtime>(
				native_location,
				dest,
			)
		}

		fn get_asset() -> Asset {
			Asset { id: AssetId(Location::parent()), fun: Fungible(ExistentialDeposit::get()) }
		}
	}

	impl pallet_xcm_benchmarks::Config for Runtime {
		type XcmConfig = xcm_config::XcmConfig;
		type AccountIdConverter = xcm_config::LocationToAccountId;
		type DeliveryHelper = polkadot_runtime_common::xcm_sender::ToParachainDeliveryHelper<
			xcm_config::XcmConfig,
			ExistentialDepositAsset,
			PriceForSiblingParachainDelivery,
			AssetHubParaId,
			ParachainSystem,
		>;
		fn valid_destination() -> Result<Location, BenchmarkError> {
			Ok(AssetHubLocation::get())
		}
		fn worst_case_holding(_depositable_count: u32) -> Assets {
			// just concrete assets according to relay chain.
			let assets: Vec<Asset> = vec![Asset {
				id: AssetId(DotRelayLocation::get()),
				fun: Fungible(1_000_000 * UNITS),
			}];
			assets.into()
		}
	}

	parameter_types! {
		pub TrustedTeleporter: Option<(Location, Asset)> = Some((
			AssetHubLocation::get(),
			Asset { fun: Fungible(UNITS), id: AssetId(DotRelayLocation::get()) },
		));
		pub const CheckedAccount: Option<(AccountId, xcm_builder::MintLocation)> = None;
		pub const TrustedReserve: Option<(Location, Asset)> = None;
	}

	impl pallet_xcm_benchmarks::fungible::Config for Runtime {
		type TransactAsset = Balances;

		type CheckedAccount = CheckedAccount;
		type TrustedTeleporter = TrustedTeleporter;
		type TrustedReserve = TrustedReserve;

		fn get_asset() -> Asset {
			Asset { id: AssetId(DotRelayLocation::get()), fun: Fungible(UNITS) }
		}
	}

	impl pallet_xcm_benchmarks::generic::Config for Runtime {
		type TransactAsset = Balances;
		type RuntimeCall = RuntimeCall;

		fn worst_case_response() -> (u64, Response) {
			(0u64, Response::Version(Default::default()))
		}

		fn worst_case_asset_exchange() -> Result<(Assets, Assets), BenchmarkError> {
			Err(BenchmarkError::Skip)
		}

		fn universal_alias() -> Result<(Location, Junction), BenchmarkError> {
			Err(BenchmarkError::Skip)
		}

		fn transact_origin_and_runtime_call() -> Result<(Location, RuntimeCall), BenchmarkError> {
			Ok((
				AssetHubLocation::get(),
				frame_system::Call::remark_with_event { remark: vec![] }.into(),
			))
		}

		fn subscribe_origin() -> Result<Location, BenchmarkError> {
			Ok(AssetHubLocation::get())
		}

		fn claimable_asset() -> Result<(Location, Location, Assets), BenchmarkError> {
			let origin = AssetHubLocation::get();
			let assets: Assets = (AssetId(DotRelayLocation::get()), 1_000 * UNITS).into();
			let ticket = Location { parents: 0, interior: Here };
			Ok((origin, ticket, assets))
		}

		fn worst_case_for_trader() -> Result<(Asset, WeightLimit), BenchmarkError> {
			Ok((
				Asset { id: AssetId(DotRelayLocation::get()), fun: Fungible(1_000_000 * UNITS) },
				Limited(Weight::from_parts(5000, 5000)),
			))
		}

		fn unlockable_asset() -> Result<(Location, Location, Asset), BenchmarkError> {
			Err(BenchmarkError::Skip)
		}

		fn export_message_origin_and_destination(
		) -> Result<(Location, NetworkId, InteriorLocation), BenchmarkError> {
			// save XCM version for remote bridge hub
			PolkadotXcm::force_xcm_version(
				RuntimeOrigin::root(),
				Box::new(bridge_to_kusama_config::BridgeHubKusamaLocation::get()),
				XCM_VERSION,
			)
			.map_err(|e| {
				log::error!(
					"Failed to dispatch `force_xcm_version({:?}, {:?}, {:?})`, error: {:?}",
					RuntimeOrigin::root(),
					bridge_to_kusama_config::BridgeHubKusamaLocation::get(),
					XCM_VERSION,
					e
				);
				BenchmarkError::Stop("XcmVersion was not stored!")
			})?;

			let remote_parachain_id = Parachain(8765);
			let sibling_parachain_location = AssetHubLocation::get();

			// open bridge
			let bridge_destination_universal_location: InteriorLocation =
				[GlobalConsensus(NetworkId::Kusama), remote_parachain_id].into();
			let locations = XcmOverBridgeHubKusama::bridge_locations(
				sibling_parachain_location.clone(),
				bridge_destination_universal_location.clone(),
			)?;
			XcmOverBridgeHubKusama::do_open_bridge(
				locations,
				bp_messages::LegacyLaneId([1, 2, 3, 4]),
				true,
			)
			.map_err(|e| {
				log::error!(
					"Failed to `XcmOverBridgeHubRococo::open_bridge`({sibling_parachain_location:?}, {bridge_destination_universal_location:?})`, error: {e:?}"
				);
				BenchmarkError::Stop("Bridge was not opened!")
			})?;

			Ok((sibling_parachain_location, NetworkId::Kusama, [remote_parachain_id].into()))
		}

		fn alias_origin() -> Result<(Location, Location), BenchmarkError> {
			Ok((
				Location::new(1, [Parachain(1000)]),
				Location::new(1, [Parachain(1000), AccountId32 { id: [111u8; 32], network: None }]),
			))
		}
	}

	use pallet_bridge_relayers::benchmarking::Config as BridgeRelayersConfig;

	impl BridgeRelayersConfig for Runtime {
		fn bench_reward() -> Self::Reward {
			bp_relayers::RewardsAccountParams::new(
				bp_messages::LegacyLaneId::default(),
				*b"test",
				bp_relayers::RewardsAccountOwner::ThisChain,
			)
			.into()
		}

		fn prepare_rewards_account(
			reward_kind: Self::Reward,
			reward: Balance,
		) -> Option<
			pallet_bridge_relayers::BeneficiaryOf<
				Runtime,
				bridge_common_config::BridgeRelayersInstance,
			>,
		> {
			let bridge_common_config::BridgeReward::PolkadotKusamaBridge(reward_kind) = reward_kind
			else {
				panic!(
					"Unexpected reward_kind: {reward_kind:?} - not compatible with `bench_reward`!"
				);
			};
			let rewards_account = bp_relayers::PayRewardFromAccount::<
				Balances,
				AccountId,
				bp_messages::LegacyLaneId,
				Balance,
			>::rewards_account(reward_kind);
			Self::deposit_account(rewards_account.clone(), reward);

			Some(bridge_common_config::BridgeRewardBeneficiaries::LocalAccount(rewards_account))
		}

		fn deposit_account(account: AccountId, balance: Balance) {
			use frame_support::traits::fungible::Mutate;
			Balances::mint_into(&account, balance.saturating_add(ExistentialDeposit::get()))
				.unwrap();
		}
	}

	use bridge_runtime_common::parachains_benchmarking::prepare_parachain_heads_proof;
	use pallet_bridge_parachains::benchmarking::Config as BridgeParachainsConfig;

	impl BridgeParachainsConfig<bridge_to_kusama_config::BridgeParachainKusamaInstance> for Runtime {
		fn parachains() -> Vec<bp_polkadot_core::parachains::ParaId> {
			use bp_runtime::Parachain;
			vec![bp_polkadot_core::parachains::ParaId(
				bp_bridge_hub_kusama::BridgeHubKusama::PARACHAIN_ID,
			)]
		}

		fn prepare_parachain_heads_proof(
			parachains: &[bp_polkadot_core::parachains::ParaId],
			parachain_head_size: u32,
			proof_params: bp_runtime::UnverifiedStorageProofParams,
		) -> (
			bp_parachains::RelayBlockNumber,
			bp_parachains::RelayBlockHash,
			bp_polkadot_core::parachains::ParaHeadsProof,
			Vec<(bp_polkadot_core::parachains::ParaId, bp_polkadot_core::parachains::ParaHash)>,
		) {
			prepare_parachain_heads_proof::<
				Runtime,
				bridge_to_kusama_config::BridgeParachainKusamaInstance,
			>(parachains, parachain_head_size, proof_params)
		}
	}

	use bridge_runtime_common::messages_benchmarking::{
		generate_xcm_builder_bridge_message_sample, prepare_message_delivery_proof_from_parachain,
		prepare_message_proof_from_parachain,
	};
	use pallet_bridge_messages::benchmarking::{
		Config as BridgeMessagesConfig, MessageDeliveryProofParams, MessageProofParams,
	};

	impl BridgeMessagesConfig<bridge_to_kusama_config::WithBridgeHubKusamaMessagesInstance>
		for Runtime
	{
		fn is_relayer_rewarded(relayer: &Self::AccountId) -> bool {
			let bench_lane_id = <Self as BridgeMessagesConfig<
				bridge_to_kusama_config::WithBridgeHubKusamaMessagesInstance,
			>>::bench_lane_id();
			use bp_runtime::Chain;
			let bridged_chain_id = <Self as pallet_bridge_messages::Config<
				bridge_to_kusama_config::WithBridgeHubKusamaMessagesInstance,
			>>::BridgedChain::ID;
			pallet_bridge_relayers::Pallet::<
				Runtime,
				bridge_common_config::BridgeRelayersInstance,
			>::relayer_reward(
				relayer,
				bridge_common_config::BridgeReward::PolkadotKusamaBridge(
					bp_relayers::RewardsAccountParams::new(
						bench_lane_id,
						bridged_chain_id,
						bp_relayers::RewardsAccountOwner::BridgedChain,
					)
				)
			)
			.is_some()
		}

		fn prepare_message_proof(
			params: MessageProofParams<
				LaneIdOf<Runtime, bridge_to_kusama_config::WithBridgeHubKusamaMessagesInstance>,
			>,
		) -> (
			bridge_to_kusama_config::FromKusamaBridgeHubMessagesProof<
				bridge_to_kusama_config::WithBridgeHubKusamaMessagesInstance,
			>,
			Weight,
		) {
			use cumulus_primitives_core::XcmpMessageSource;
			assert!(XcmpQueue::take_outbound_messages(usize::MAX).is_empty());
			ParachainSystem::open_outbound_hrmp_channel_for_benchmarks_or_tests(42.into());
			PolkadotXcm::force_xcm_version(
				RuntimeOrigin::root(),
				Box::new(Location::new(1, Parachain(42))),
				XCM_VERSION,
			)
			.map_err(|e| {
				log::error!(
					"Failed to dispatch `force_xcm_version({:?}, {:?}, {:?})`, error: {:?}",
					RuntimeOrigin::root(),
					Location::new(1, Parachain(42)),
					XCM_VERSION,
					e
				);
			})
			.expect("XcmVersion stored!");
			let universal_source = bridge_to_kusama_config::open_bridge_for_benchmarks::<
				Runtime,
				bridge_to_kusama_config::XcmOverBridgeHubKusamaInstance,
				xcm_config::LocationToAccountId,
			>(params.lane, 42);
			prepare_message_proof_from_parachain::<
				Runtime,
				bridge_to_kusama_config::BridgeGrandpaKusamaInstance,
				bridge_to_kusama_config::WithBridgeHubKusamaMessagesInstance,
			>(params, generate_xcm_builder_bridge_message_sample(universal_source))
		}

		fn prepare_message_delivery_proof(
			params: MessageDeliveryProofParams<
				AccountId,
				LaneIdOf<Runtime, bridge_to_kusama_config::WithBridgeHubKusamaMessagesInstance>,
			>,
		) -> bridge_to_kusama_config::ToKusamaBridgeHubMessagesDeliveryProof<
			bridge_to_kusama_config::WithBridgeHubKusamaMessagesInstance,
		> {
			let _ = bridge_to_kusama_config::open_bridge_for_benchmarks::<
				Runtime,
				bridge_to_kusama_config::XcmOverBridgeHubKusamaInstance,
				xcm_config::LocationToAccountId,
			>(params.lane, 42);
			prepare_message_delivery_proof_from_parachain::<
				Runtime,
				bridge_to_kusama_config::BridgeGrandpaKusamaInstance,
				bridge_to_kusama_config::WithBridgeHubKusamaMessagesInstance,
			>(params)
		}

		fn is_message_successfully_dispatched(_nonce: bp_messages::MessageNonce) -> bool {
			use cumulus_primitives_core::XcmpMessageSource;
			!XcmpQueue::take_outbound_messages(usize::MAX).is_empty()
		}
	}

	pub use cumulus_pallet_session_benchmarking::Pallet as SessionBench;
	pub use frame_benchmarking::{BenchmarkBatch, BenchmarkError, BenchmarkList};
	pub use frame_support::traits::StorageInfoTrait;
	pub use frame_system_benchmarking::{
		extensions::Pallet as SystemExtensionsBench, Pallet as SystemBench,
	};
	pub use pallet_xcm::benchmarking::Pallet as PalletXcmExtrinsicsBenchmark;
	pub type XcmBalances = pallet_xcm_benchmarks::fungible::Pallet<Runtime>;
	pub type XcmGeneric = pallet_xcm_benchmarks::generic::Pallet<Runtime>;
	pub use pallet_bridge_relayers::benchmarking::Pallet as BridgeRelayersBench;
	pub type KusamaFinality = BridgeKusamaGrandpa;
	pub use frame_support::traits::WhitelistedStorageKeys;
	pub use sp_storage::TrackedStorageKey;
	pub type KusamaParachains = pallet_bridge_parachains::benchmarking::Pallet<
		Runtime,
		bridge_to_kusama_config::BridgeParachainKusamaInstance,
	>;
	pub type KusamaMessages = pallet_bridge_messages::benchmarking::Pallet<
		Runtime,
		bridge_to_kusama_config::WithBridgeHubKusamaMessagesInstance,
	>;
}

#[cfg(feature = "runtime-benchmarks")]
use benches::*;

impl_runtime_apis! {
	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(SLOT_DURATION)
		}

		fn authorities() -> Vec<AuraId> {
			pallet_aura::Authorities::<Runtime>::get().into_inner()
		}
	}

	impl cumulus_primitives_core::RelayParentOffsetApi<Block> for Runtime {
		fn relay_parent_offset() -> u32 {
			0
		}
	}

	impl cumulus_primitives_aura::AuraUnincludedSegmentApi<Block> for Runtime {
		fn can_build_upon(
			included_hash: <Block as BlockT>::Hash,
			slot: cumulus_primitives_aura::Slot,
		) -> bool {
			ConsensusHook::can_build_upon(included_hash, slot)
		}
	}

	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: <Block as BlockT>::LazyBlock) {
			Executive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: <Block as BlockT>::LazyBlock,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl frame_support::view_functions::runtime_api::RuntimeViewFunction<Block> for Runtime {
		fn execute_view_function(
			id: frame_support::view_functions::ViewFunctionId,
			input: Vec<u8>
		) -> Result<Vec<u8>, frame_support::view_functions::ViewFunctionDispatchError> {
			Runtime::execute_view_function(id, input)
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
		for Runtime
	{
		fn query_call_info(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl xcm_runtime_apis::fees::XcmPaymentApi<Block> for Runtime {
		fn query_acceptable_payment_assets(xcm_version: xcm::Version) -> Result<Vec<VersionedAssetId>, XcmPaymentApiError> {
			let acceptable_assets = vec![AssetId(xcm_config::DotRelayLocation::get())];
			PolkadotXcm::query_acceptable_payment_assets(xcm_version, acceptable_assets)
		}

		fn query_weight_to_asset_fee(weight: Weight, asset: VersionedAssetId) -> Result<u128, XcmPaymentApiError> {
			use crate::xcm_config::XcmConfig;
			type Trader = <XcmConfig as xcm_executor::Config>::Trader;
			PolkadotXcm::query_weight_to_asset_fee::<Trader>(weight, asset)
		}

		fn query_xcm_weight(message: VersionedXcm<()>) -> Result<Weight, XcmPaymentApiError> {
			PolkadotXcm::query_xcm_weight(message)
		}

		fn query_delivery_fees(destination: VersionedLocation, message: VersionedXcm<()>, asset_id: VersionedAssetId) -> Result<VersionedAssets, XcmPaymentApiError> {
			type AssetExchanger = <xcm_config::XcmConfig as xcm_executor::Config>::AssetExchanger;
			PolkadotXcm::query_delivery_fees::<AssetExchanger>(destination, message, asset_id)
		}
	}

	impl xcm_runtime_apis::dry_run::DryRunApi<Block, RuntimeCall, RuntimeEvent, OriginCaller> for Runtime {
		fn dry_run_call(origin: OriginCaller, call: RuntimeCall, result_xcms_version: XcmVersion) -> Result<CallDryRunEffects<RuntimeEvent>, XcmDryRunApiError> {
			PolkadotXcm::dry_run_call::<Runtime, xcm_config::XcmRouter, OriginCaller, RuntimeCall>(origin, call, result_xcms_version)
		}

		fn dry_run_xcm(origin_location: VersionedLocation, xcm: VersionedXcm<RuntimeCall>) -> Result<XcmDryRunEffects<RuntimeEvent>, XcmDryRunApiError> {
			PolkadotXcm::dry_run_xcm::<xcm_config::XcmRouter>(origin_location, xcm)
		}
	}

	impl xcm_runtime_apis::conversions::LocationToAccountApi<Block, AccountId> for Runtime {
		fn convert_location(location: VersionedLocation) -> Result<
			AccountId,
			xcm_runtime_apis::conversions::Error
		> {
			xcm_runtime_apis::conversions::LocationToAccountHelper::<
				AccountId,
				xcm_config::LocationToAccountId,
			>::convert_location(location)
		}
	}

	impl xcm_runtime_apis::trusted_query::TrustedQueryApi<Block> for Runtime {
		fn is_trusted_reserve(asset: VersionedAsset, location: VersionedLocation) -> xcm_runtime_apis::trusted_query::XcmTrustedQueryResult {
			PolkadotXcm::is_trusted_reserve(asset, location)
		}
		fn is_trusted_teleporter(asset: VersionedAsset, location: VersionedLocation) -> xcm_runtime_apis::trusted_query::XcmTrustedQueryResult {
			PolkadotXcm::is_trusted_teleporter(asset, location)
		}
	}

	impl xcm_runtime_apis::authorized_aliases::AuthorizedAliasersApi<Block> for Runtime {
		fn authorized_aliasers(target: VersionedLocation) -> Result<
			Vec<xcm_runtime_apis::authorized_aliases::OriginAliaser>,
			xcm_runtime_apis::authorized_aliases::Error
		> {
			PolkadotXcm::authorized_aliasers(target)
		}
		fn is_authorized_alias(origin: VersionedLocation, target: VersionedLocation) -> Result<
			bool,
			xcm_runtime_apis::authorized_aliases::Error
		> {
			PolkadotXcm::is_authorized_alias(origin, target)
		}
	}

	impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
		fn collect_collation_info(header: &<Block as BlockT>::Header) -> cumulus_primitives_core::CollationInfo {
			ParachainSystem::collect_collation_info(header)
		}
	}

	impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
		fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
			build_state::<RuntimeGenesisConfig>(config)
		}

		fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
			get_preset::<RuntimeGenesisConfig>(id, &genesis_config_presets::get_preset)
		}

		fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
			genesis_config_presets::preset_names()
		}
	}

	impl bp_kusama::KusamaFinalityApi<Block> for Runtime {
		fn best_finalized() -> Option<bp_runtime::HeaderId<bp_kusama::Hash, bp_kusama::BlockNumber>> {
			BridgeKusamaGrandpa::best_finalized()
		}

		fn free_headers_interval() -> Option<bp_kusama::BlockNumber> {
			<Runtime as pallet_bridge_grandpa::Config<
				bridge_to_kusama_config::BridgeGrandpaKusamaInstance
			>>::FreeHeadersInterval::get()
		}

		fn synced_headers_grandpa_info(
		) -> Vec<bp_header_chain::StoredHeaderGrandpaInfo<bp_kusama::Header>> {
			BridgeKusamaGrandpa::synced_headers_grandpa_info()
		}
	}

	impl bp_bridge_hub_kusama::BridgeHubKusamaFinalityApi<Block> for Runtime {
		fn best_finalized() -> Option<bp_runtime::HeaderId<bp_bridge_hub_kusama::Hash, bp_bridge_hub_kusama::BlockNumber>> {
			BridgeKusamaParachains::best_parachain_head_id::<
				bp_bridge_hub_kusama::BridgeHubKusama
			>().unwrap_or(None)
		}

		fn free_headers_interval() -> Option<bp_bridge_hub_kusama::BlockNumber> {
			// "free interval" is not currently used for parachains
			None
		}
	}

	impl bp_bridge_hub_kusama::FromBridgeHubKusamaInboundLaneApi<Block> for Runtime {
		fn message_details(
			lane: bp_messages::LegacyLaneId,
			messages: Vec<(bp_messages::MessagePayload, bp_messages::OutboundMessageDetails)>,
		) -> Vec<bp_messages::InboundMessageDetails> {
			bridge_runtime_common::messages_api::inbound_message_details::<
				Runtime,
				bridge_to_kusama_config::WithBridgeHubKusamaMessagesInstance,
			>(lane, messages)
		}
	}

	impl bp_bridge_hub_kusama::ToBridgeHubKusamaOutboundLaneApi<Block> for Runtime {
		fn message_details(
			lane: bp_messages::LegacyLaneId,
			begin: bp_messages::MessageNonce,
			end: bp_messages::MessageNonce,
		) -> Vec<bp_messages::OutboundMessageDetails> {
			bridge_runtime_common::messages_api::outbound_message_details::<
				Runtime,
				bridge_to_kusama_config::WithBridgeHubKusamaMessagesInstance,
			>(lane, begin, end)
		}
	}

	impl snowbridge_outbound_queue_runtime_api::OutboundQueueApi<Block, Balance> for Runtime {
		fn prove_message(leaf_index: u64) -> Option<snowbridge_merkle_tree::MerkleProof> {
			snowbridge_pallet_outbound_queue::api::prove_message::<Runtime>(leaf_index)
		}

		fn calculate_fee(command: Command, parameters: Option<PricingParameters<Balance>>) -> Fee<Balance> {
			snowbridge_pallet_outbound_queue::api::calculate_fee::<Runtime>(command, parameters)
		}
	}

	impl snowbridge_outbound_queue_v2_runtime_api::OutboundQueueV2Api<Block, Balance> for Runtime {
		fn prove_message(leaf_index: u64) -> Option<snowbridge_merkle_tree::MerkleProof> {
			snowbridge_pallet_outbound_queue_v2::api::prove_message::<Runtime>(leaf_index)
		}
	}

	impl snowbridge_system_runtime_api::ControlApi<Block> for Runtime {
		fn agent_id(location: VersionedLocation) -> Option<AgentId> {
			snowbridge_pallet_system::api::agent_id::<Runtime>(location)
		}
	}

	impl snowbridge_system_v2_runtime_api::ControlV2Api<Block> for Runtime {
		fn agent_id(location: VersionedLocation) -> Option<AgentId> {
			snowbridge_pallet_system_v2::api::agent_id::<Runtime>(location)
		}
	}

	impl cumulus_primitives_core::GetParachainInfo<Block> for Runtime {
		fn parachain_id() -> ParaId {
			ParachainInfo::parachain_id()
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, RuntimeBlockWeights::get().max_block)
		}

		fn execute_block(
			block: <Block as BlockT>::LazyBlock,
			state_root_check: bool,
			signature_check: bool,
			select: frame_try_runtime::TryStateSelect,
		) -> Weight {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here.
			Executive::try_execute_block(block, state_root_check, signature_check, select).unwrap()
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();
			(list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, alloc::string::String> {
			let whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();
			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);

			Ok(batches)
		}
	}
}

cumulus_pallet_parachain_system::register_validate_block! {
	Runtime = Runtime,
	BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_transasction_byte_fee_is_one_twentieth_of_relay() {
		let relay_tbf = polkadot_runtime_constants::fee::TRANSACTION_BYTE_FEE;
		let parachain_tbf = TransactionByteFee::get();
		assert_eq!(relay_tbf / 20, parachain_tbf);
	}
}
