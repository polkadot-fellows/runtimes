// Copyright (C) Parity Technologies and the various Polkadot contributors, see Contributions.md
// for a list of specific contributors.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

extern crate alloc;

mod coretime;
// Genesis preset configurations.
pub mod genesis_config_presets;
#[cfg(test)]
mod tests;
mod weights;
pub mod xcm_config;

use alloc::{borrow::Cow, vec, vec::Vec};
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use cumulus_pallet_parachain_system::RelayNumberMonotonicallyIncreases;
use cumulus_primitives_core::{AggregateMessageOrigin, ParaId};
use frame_support::{
	construct_runtime, derive_impl,
	dispatch::DispatchClass,
	genesis_builder_helper::{build_state, get_preset},
	parameter_types,
	traits::{
		tokens::imbalance::ResolveTo, ConstBool, ConstU32, ConstU64, ConstU8, Contains, EitherOf,
		EitherOfDiverse, EverythingBut, InstanceFilter, TransformOrigin,
	},
	weights::{ConstantMultiplier, Weight},
	PalletId,
};
use frame_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot,
};
use pallet_xcm::{EnsureXcm, IsVoiceOfBody};
use parachains_common::{
	message_queue::{NarrowOriginToSibling, ParaIdToSibling},
	AccountId, AuraId, Balance, BlockNumber, Hash, Header, Nonce, Signature,
};
use polkadot_runtime_common::{BlockHashCount, SlowAdjustingFeeUpdate};
use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
use sp_runtime::{
	generic, impl_opaque_keys,
	traits::{BlakeTwo256, Block as BlockT, BlockNumberProvider},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, MultiAddress, Perbill, RuntimeDebug,
};
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
use system_parachains_constants::{
	polkadot::{consensus::*, currency::*, fee::WeightToFee},
	AVERAGE_ON_INITIALIZE_RATIO, HOURS, MAXIMUM_BLOCK_WEIGHT, NORMAL_DISPATCH_RATIO, SLOT_DURATION,
};
use weights::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight};
use xcm::prelude::*;
use xcm_config::{
	AssetHubLocation, DotRelayLocation, FellowshipLocation, RelayChainLocation, StakingPot,
	XcmOriginToTransactDispatchOrigin,
};
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

/// The TransactionExtension to the basic transaction logic.
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
		frame_metadata_hash_extension::CheckMetadataHash<Runtime>,
	),
>;

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, TxExtension>;

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
	spec_name: Cow::Borrowed("coretime-polkadot"),
	impl_name: Cow::Borrowed("coretime-polkadot"),
	authoring_version: 1,
	spec_version: 2_001_000,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 0,
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

/// Filter:
/// - Credit purchase calls until the credit system is implemented. Otherwise, users may have chance
///   of locking their funds forever on purchased credits they cannot use.
/// - The interlace call until the relay can support this fully
pub struct IsFilteredBrokerCall;
impl Contains<RuntimeCall> for IsFilteredBrokerCall {
	fn contains(c: &RuntimeCall) -> bool {
		matches!(
			c,
			RuntimeCall::Broker(pallet_broker::Call::purchase_credit { .. }) |
				RuntimeCall::Broker(pallet_broker::Call::interlace { .. })
		)
	}
}

/// Implements [`pallet_broker::BlockToRelayHeightConversion`] for the migration to relay chain
/// block numbers for the broker pallet.
pub struct BrokerMigrationV4BlockConversion;

impl pallet_broker::migration::v4::BlockToRelayHeightConversion<Runtime>
	for BrokerMigrationV4BlockConversion
{
	fn convert_block_number_to_relay_height(input_block_number: u32) -> u32 {
		let relay_height = pallet_broker::RCBlockNumberProviderOf::<
			<Runtime as pallet_broker::Config>::Coretime,
		>::current_block_number();
		let parachain_block_number = frame_system::Pallet::<Runtime>::block_number();
		let offset = relay_height - parachain_block_number * 2;
		offset + input_block_number * 2
	}

	fn convert_block_length_to_relay_length(input_block_length: u32) -> u32 {
		input_block_length * 2
	}
}

// Configure FRAME pallets to include in runtime.
#[derive_impl(frame_system::config_preludes::ParaChainDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Runtime {
	type BaseCallFilter = EverythingBut<IsFilteredBrokerCall>;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The nonce type for storing how many extrinsics an account has signed.
	type Nonce = Nonce;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The block type.
	type Block = Block;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// Runtime version.
	type Version = Version;
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = weights::frame_system::WeightInfo<Runtime>;
	/// Weight information for the extensions in this pallet.
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
	type Balance = Balance;
	type DustRemoval = ();
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
	pub const TransactionByteFee: Balance = MILLICENTS;
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
	type WeightInfo = weights::cumulus_pallet_parachain_system::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type OnSystemEvent = ();
	type SelfParaId = parachain_info::Pallet<Runtime>;
	type DmpQueue = frame_support::traits::EnqueueWithOrigin<MessageQueue, RelayOrigin>;
	type OutboundXcmpMessageSource = XcmpQueue;
	type ReservedDmpWeight = ReservedDmpWeight;
	type XcmpMessageHandler = XcmpQueue;
	type ReservedXcmpWeight = ReservedXcmpWeight;
	type CheckAssociatedRelayNumber = RelayNumberMonotonicallyIncreases;
	type ConsensusHook = ConsensusHook;
	type RelayParentOffset = ConstU32<0>;
}

type ConsensusHook = cumulus_pallet_aura_ext::FixedVelocityConsensusHook<
	Runtime,
	RELAY_CHAIN_SLOT_DURATION_MILLIS,
	BLOCK_PROCESSING_VELOCITY,
	UNINCLUDED_SEGMENT_CAPACITY,
>;

parameter_types! {
	pub MessageQueueServiceWeight: Weight = Perbill::from_percent(35) * RuntimeBlockWeights::get().max_block;
	pub MessageQueueIdleServiceWeight: Weight = Perbill::from_percent(20) * RuntimeBlockWeights::get().max_block;
}

impl pallet_message_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_message_queue::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type MessageProcessor = pallet_message_queue::mock_helpers::NoopMessageProcessor<
		cumulus_primitives_core::AggregateMessageOrigin,
	>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type MessageProcessor = xcm_builder::ProcessXcmMessage<
		AggregateMessageOrigin,
		xcm_executor::XcmExecutor<xcm_config::XcmConfig>,
		RuntimeCall,
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

impl parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

/// Privileged origin that represents Root or Fellows pluralistic body.
pub type RootOrFellows = EitherOfDiverse<
	EnsureRoot<AccountId>,
	EnsureXcm<IsVoiceOfBody<FellowshipLocation, FellowsBodyId>>,
>;

parameter_types! {
	// Fellows pluralistic body.
	pub const FellowsBodyId: BodyId = BodyId::Technical;
	/// The asset ID for the asset that we use to pay for message delivery fees.
	pub FeeAssetId: AssetId = AssetId(DotRelayLocation::get());
	/// The base fee for the message delivery fees.
	pub const ToSiblingBaseDeliveryFee: u128 = CENTS.saturating_mul(3);
	pub const ToParentBaseDeliveryFee: u128 = CENTS.saturating_mul(3);
}

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
	// This must be the same as the `ChannelInfo` from the `Config`.
	type ChannelList = ParachainSystem;
}

pub const SESSION_LENGTH: u32 = 6 * HOURS;
pub const OFFSET: u32 = 0;

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	// We don't have stash and controller, thus we don't need the convert as well.
	type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
	type ShouldEndSession =
		pallet_session::PeriodicSessions<ConstU32<SESSION_LENGTH>, ConstU32<OFFSET>>;
	type NextSessionRotation =
		pallet_session::PeriodicSessions<ConstU32<SESSION_LENGTH>, ConstU32<OFFSET>>;
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
	/// StakingAdmin pluralistic body.
	pub const StakingAdminBodyId: BodyId = BodyId::Defense;
}

/// We allow Root and the `StakingAdmin` to execute privileged collator selection operations.
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
	// Should be a multiple of session or things will get inconsistent.
	type KickThreshold = ConstU32<SESSION_LENGTH>;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
	type ValidatorRegistration = Session;
	type WeightInfo = weights::pallet_collator_selection::WeightInfo<Runtime>;
}

parameter_types! {
	/// One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
	pub const DepositBase: Balance = system_para_deposit(1, 88);
	/// Additional storage item size of 32 bytes.
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

/// The type used to represent the kinds of proxying allowed.
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
	RuntimeDebug,
	MaxEncodedLen,
	scale_info::TypeInfo,
	Default,
)]
pub enum ProxyType {
	/// Fully permissioned proxy. Can execute any call on behalf of _proxied_.
	#[default]
	Any,
	/// Can execute any call that does not transfer funds or assets.
	NonTransfer,
	/// Proxy with the ability to reject time-delay proxy announcements.
	CancelProxy,
	/// Proxy for all Broker pallet calls.
	Broker,
	/// Proxy for renewing coretime.
	CoretimeRenewer,
	/// Proxy able to purchase on-demand coretime credits.
	OnDemandPurchaser,
	/// Collator selection proxy. Can execute calls related to collator selection mechanism.
	Collator,
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
					// We don't allow `purchase`, `renew`, `transfer`, `purchase_credit`,
					// `pool` doesn't transfer, but it defines the account to be paid for contributions,
					// `assign` is essentially a transfer of a region NFT.
					RuntimeCall::Broker(pallet_broker::Call::claim_revenue { .. }) |
					RuntimeCall::Broker(pallet_broker::Call::drop_region { .. }) |
					RuntimeCall::Broker(pallet_broker::Call::drop_contribution { .. }) |
					RuntimeCall::Broker(pallet_broker::Call::drop_history { .. }) |
					RuntimeCall::Broker(pallet_broker::Call::drop_renewal { .. })
			),
			ProxyType::CancelProxy => matches!(
				c,
				RuntimeCall::Proxy(pallet_proxy::Call::reject_announcement { .. }) |
					RuntimeCall::Utility { .. } |
					RuntimeCall::Multisig { .. }
			),
			ProxyType::Broker => {
				matches!(
					c,
					RuntimeCall::Broker { .. } |
						RuntimeCall::Utility { .. } |
						RuntimeCall::Multisig { .. }
				)
			},
			ProxyType::CoretimeRenewer => {
				matches!(
					c,
					RuntimeCall::Broker(pallet_broker::Call::renew { .. }) |
						RuntimeCall::Utility { .. } |
						RuntimeCall::Multisig { .. }
				)
			},
			ProxyType::OnDemandPurchaser => {
				matches!(
					c,
					RuntimeCall::Broker(pallet_broker::Call::purchase_credit { .. }) |
						RuntimeCall::Utility { .. } |
						RuntimeCall::Multisig { .. }
				)
			},
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
			(ProxyType::Broker, ProxyType::CoretimeRenewer) => true,
			(ProxyType::Broker, ProxyType::OnDemandPurchaser) => true,
			(ProxyType::NonTransfer, ProxyType::Collator) => true,
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

impl pallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = weights::pallet_utility::WeightInfo<Runtime>;
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
		Timestamp: pallet_timestamp = 3,
		ParachainInfo: parachain_info = 4,
		WeightReclaim: cumulus_pallet_weight_reclaim = 5,

		// Monetary stuff.
		Balances: pallet_balances = 10,
		TransactionPayment: pallet_transaction_payment = 11,

		// Collator support. The order of these 5 are important and shall not change.
		Authorship: pallet_authorship = 20,
		CollatorSelection: pallet_collator_selection = 21,
		Session: pallet_session = 22,
		Aura: pallet_aura = 23,
		AuraExt: cumulus_pallet_aura_ext = 24,

		// XCM & related
		XcmpQueue: cumulus_pallet_xcmp_queue = 30,
		PolkadotXcm: pallet_xcm = 31,
		CumulusXcm: cumulus_pallet_xcm = 32,
		MessageQueue: pallet_message_queue = 34,

		// Handy utilities.
		Utility: pallet_utility = 40,
		Multisig: pallet_multisig = 41,
		Proxy: pallet_proxy = 42,

		// The main stage.
		Broker: pallet_broker = 50,
	}
);

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	use super::*;
	use alloc::boxed::Box;
	use polkadot_runtime_constants::system_parachain::AssetHubParaId;
	use system_parachains_constants::polkadot::locations::AssetHubLocation;

	frame_benchmarking::define_benchmarks!(
		[frame_system, SystemBench::<Runtime>]
		[frame_system_extensions, SystemExtensionsBench::<Runtime>]
		[cumulus_pallet_parachain_system, ParachainSystem]
		[cumulus_pallet_weight_reclaim, WeightReclaim]
		[pallet_timestamp, Timestamp]
		[pallet_balances, Balances]
		[pallet_broker, Broker]
		[pallet_collator_selection, CollatorSelection]
		[pallet_session, SessionBench::<Runtime>]
		[cumulus_pallet_xcmp_queue, XcmpQueue]
		[pallet_xcm, PalletXcmExtrinsicsBenchmark::<Runtime>]
		[pallet_message_queue, MessageQueue]
		[pallet_multisig, Multisig]
		[pallet_proxy, Proxy]
		[pallet_transaction_payment, TransactionPayment]
		[pallet_utility, Utility]
		// NOTE: Make sure you point to the individual modules below.
		[pallet_xcm_benchmarks::fungible, XcmBalances]
		[pallet_xcm_benchmarks::generic, XcmGeneric]
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
		pub const RandomParaId: ParaId = ParaId::new(43211234);
	}

	impl pallet_xcm::benchmarking::Config for Runtime {
		type DeliveryHelper = (
			polkadot_runtime_common::xcm_sender::ToParachainDeliveryHelper<
				xcm_config::XcmConfig,
				ExistentialDepositAsset,
				PriceForSiblingParachainDelivery,
				RandomParaId,
				ParachainSystem,
			>,
			polkadot_runtime_common::xcm_sender::ToParachainDeliveryHelper<
				xcm_config::XcmConfig,
				ExistentialDepositAsset,
				PriceForSiblingParachainDelivery,
				AssetHubParaId,
				ParachainSystem,
			>,
		);

		fn reachable_dest() -> Option<Location> {
			Some(AssetHubLocation::get())
		}

		fn teleportable_asset_and_dest() -> Option<(Asset, Location)> {
			// Relay/native token can be teleported between Coretime and Relay.
			Some((
				Asset { fun: Fungible(ExistentialDeposit::get()), id: AssetId(Parent.into()) },
				AssetHubLocation::get(),
			))
		}

		fn reserve_transferable_asset_and_dest() -> Option<(Asset, Location)> {
			// Coretime chain can reserve transfer regions to some random parachain.

			// Properties of a mock region:
			let core = 0;
			let begin = 0;
			let end = 42;

			let region_id = pallet_broker::Pallet::<Runtime>::issue(
				core,
				begin,
				CoreMask::complete(),
				end,
				None,
				None,
			);
			Some((
				Asset {
					fun: NonFungible(Index(region_id.into())),
					id: AssetId(xcm_config::BrokerPalletLocation::get()),
				},
				ParentThen(Parachain(RandomParaId::get().into()).into()).into(),
			))
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
		type DeliveryHelper = polkadot_runtime_common::xcm_sender::ToParachainDeliveryHelper<
			xcm_config::XcmConfig,
			ExistentialDepositAsset,
			PriceForSiblingParachainDelivery,
			AssetHubParaId,
			ParachainSystem,
		>;
		type AccountIdConverter = xcm_config::LocationToAccountId;
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
		type RuntimeCall = RuntimeCall;
		type TransactAsset = Balances;

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
			Err(BenchmarkError::Skip)
		}

		fn alias_origin() -> Result<(Location, Location), BenchmarkError> {
			Ok((
				Location::new(1, [Parachain(1000)]),
				Location::new(1, [Parachain(1000), AccountId32 { id: [111u8; 32], network: None }]),
			))
		}
	}

	pub use cumulus_pallet_session_benchmarking::Pallet as SessionBench;
	pub use frame_benchmarking::{BenchmarkBatch, BenchmarkError, BenchmarkList};
	pub use frame_support::traits::{StorageInfoTrait, WhitelistedStorageKeys};
	pub use frame_system_benchmarking::{
		extensions::Pallet as SystemExtensionsBench, Pallet as SystemBench,
	};
	pub use pallet_broker::CoreMask;
	pub use pallet_xcm::benchmarking::Pallet as PalletXcmExtrinsicsBenchmark;
	pub use sp_storage::TrackedStorageKey;
	pub type XcmBalances = pallet_xcm_benchmarks::fungible::Pallet<Runtime>;
	pub type XcmGeneric = pallet_xcm_benchmarks::generic::Pallet<Runtime>;
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
}

cumulus_pallet_parachain_system::register_validate_block! {
	Runtime = Runtime,
	BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
}
