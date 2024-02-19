// Copyright (c) 2023 Encointer Association
// This file is part of Encointer
//
// Encointer is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Encointer is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Encointer.  If not, see <http://www.gnu.org/licenses/>.

//! # Encointer Parachain Runtime
//!
//! Encointer runtime containing all the specific logic to:
//!  * perform ceremonies and receive a community income
//!  * pay fees in the respective community currency
//!
//! The configuration (especially XCM) is almost identical to `asset-hub`. Therefore, upstream
//! updates should always check the diff to see if there are some configuration updates.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

mod deal_with_fees;
mod migrations_fix;
mod weights;
pub mod xcm_config;
use codec::{Decode, Encode, MaxEncodedLen};
use cumulus_pallet_parachain_system::RelayNumberMonotonicallyIncreases;
use deal_with_fees::FeesToTreasury;
use encointer_balances_tx_payment::{AssetBalanceOf, AssetIdOf, BalanceToCommunityBalance};
pub use encointer_primitives::{
	balances::{BalanceEntry, BalanceType, Demurrage},
	bazaar::{BusinessData, BusinessIdentifier, OfferingData},
	ceremonies::{AggregatedAccountData, CeremonyIndexType, CeremonyInfo, CommunityReputation},
	common::PalletString,
	communities::{CommunityIdentifier, Location},
	scheduler::CeremonyPhaseType,
};
use frame_support::{
	construct_runtime,
	dispatch::DispatchClass,
	genesis_builder_helper::{build_config, create_default_config},
	parameter_types,
	traits::{
		tokens::{pay::PayFromAccount, ConversionFromAssetBalance, ConversionToAssetBalance},
		ConstBool, Contains, EitherOfDiverse, EqualPrivilegeOnly, InstanceFilter,
	},
	weights::{ConstantMultiplier, Weight},
	PalletId,
};
use frame_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot,
};
pub use pallet_encointer_balances::Call as EncointerBalancesCall;
pub use pallet_encointer_bazaar::Call as EncointerBazaarCall;
pub use pallet_encointer_ceremonies::Call as EncointerCeremoniesCall;
pub use pallet_encointer_communities::Call as EncointerCommunitiesCall;
pub use pallet_encointer_faucet::Call as EncointerFaucetCall;
pub use pallet_encointer_reputation_commitments::Call as EncointerReputationCommitmentsCall;
pub use pallet_encointer_scheduler::Call as EncointerSchedulerCall;
use pallet_xcm::{EnsureXcm, IsMajorityOfBody};
pub use parachains_common::{
	impls::DealWithFees, AccountId, AssetIdForTrustBackedAssets, AuraId, Balance, BlockNumber,
	Hash, Header, Nonce, Signature,
};
use polkadot_runtime_common::{BlockHashCount, SlowAdjustingFeeUpdate};
use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, ConstU32, OpaqueMetadata};
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{AccountIdLookup, BlakeTwo256, Block as BlockT, IdentityLookup, Verify},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, Perbill, Permill, RuntimeDebug,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
use system_parachains_constants::{
	kusama::{consensus::*, currency::*, fee::WeightToFee},
	AVERAGE_ON_INITIALIZE_RATIO, DAYS, HOURS, MAXIMUM_BLOCK_WEIGHT, NORMAL_DISPATCH_RATIO,
	SLOT_DURATION,
};
use weights::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight};
use xcm::{
	latest::{BodyId, InteriorMultiLocation, Junction::PalletInstance},
	v3::AssetId as XcmAssetId,
};

use xcm_config::{KsmLocation, XcmConfig, XcmOriginToTransactDispatchOrigin};
use xcm_executor::XcmExecutor;

/// A type to hold UTC unix epoch [ms]
pub type Moment = u64;

pub type AssetId = AssetIdOf<Runtime>;
pub type AssetBalance = AssetBalanceOf<Runtime>;

impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: Aura,
	}
}

/// This runtime version.
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("encointer-parachain"),
	impl_name: create_runtime_str!("encointer-parachain"),
	authoring_version: 1,
	spec_version: 1_001_000,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 3,
	state_version: 0,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

parameter_types! {
	// One storage item; key size 32, value size 8; .
	pub const ProxyDepositBase: Balance = system_para_deposit(1, 40);
	// Additional storage item size of 33 bytes.
	pub const ProxyDepositFactor: Balance = system_para_deposit(0, 33);
	pub const MaxProxies: u16 = 32;
	// One storage item; key size 32, value size 16
	pub const AnnouncementDepositBase: Balance = system_para_deposit(1, 48);
	pub const AnnouncementDepositFactor: Balance = system_para_deposit(0, 66);
	pub const MaxPending: u16 = 32;
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
	RuntimeDebug,
	scale_info::TypeInfo,
	MaxEncodedLen,
)]
pub enum ProxyType {
	Any,
	NonTransfer,
	BazaarEdit,
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
			ProxyType::NonTransfer =>
				!matches!(c, RuntimeCall::Balances { .. } | RuntimeCall::EncointerBalances { .. }),
			ProxyType::BazaarEdit => matches!(
				c,
				RuntimeCall::EncointerBazaar(EncointerBazaarCall::create_offering { .. }) |
					RuntimeCall::EncointerBazaar(EncointerBazaarCall::update_offering { .. }) |
					RuntimeCall::EncointerBazaar(EncointerBazaarCall::delete_offering { .. })
			),
		}
	}

	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			(ProxyType::NonTransfer, ProxyType::BazaarEdit) => true,
			_ => false,
		}
	}
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
	pub const SS58Prefix: u8 = 2;
}

pub struct BaseFilter;
impl Contains<RuntimeCall> for BaseFilter {
	fn contains(_c: &RuntimeCall) -> bool {
		true
	}
}

// Configure FRAME pallets to include in runtime.
impl frame_system::Config for Runtime {
	type BaseCallFilter = BaseFilter;
	// The block type.
	type Block = generic::Block<Header, UncheckedExtrinsic>;
	type BlockWeights = RuntimeBlockWeights;
	type BlockLength = RuntimeBlockLength;
	type AccountId = AccountId;
	type RuntimeCall = RuntimeCall;
	type Lookup = AccountIdLookup<AccountId, ()>;
	type Nonce = Nonce;
	type Hash = Hash;
	type Hashing = BlakeTwo256;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type BlockHashCount = BlockHashCount;
	type DbWeight = RocksDbWeight;
	type Version = Version;
	type PalletInfo = PalletInfo;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type AccountData = pallet_balances::AccountData<Balance>;
	type SystemWeightInfo = weights::frame_system::WeightInfo<Runtime>;
	type SS58Prefix = SS58Prefix;
	type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = Moment;
	type OnTimestampSet = EncointerScheduler;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = weights::pallet_timestamp::WeightInfo<Runtime>;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = SYSTEM_PARA_EXISTENTIAL_DEPOSIT;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = MaxLocks;
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = weights::pallet_balances::WeightInfo<Runtime>;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = ();
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = ();
	type MaxHolds = ConstU32<0>;
	type MaxFreezes = ConstU32<0>;
}

parameter_types! {
	/// Relay Chain `TransactionByteFee` / 10, same as statemine
	pub const TransactionByteFee: Balance = MILLICENTS;
	pub const OperationalFeeMultiplier: u8 = 5;
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	// `FeesToTreasury is an encointer adaptation.
	type OnChargeTransaction =
		pallet_transaction_payment::CurrencyAdapter<Balances, FeesToTreasury<Runtime>>;
	type WeightToFee = WeightToFee;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
}

pub const ENCOINTER_TREASURY_PALLET_ID: u8 = 43;

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = 100 * MILLICENTS;
	pub const ProposalBondMaximum: Balance = 500 * CENTS;
	pub const SpendPeriod: BlockNumber = 6 * DAYS;
	pub const Burn: Permill = Permill::from_percent(1);
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	pub const PayoutSpendPeriod: BlockNumber = 30 * DAYS;
	// The asset's interior location for the paying account. This is the Treasury
	// pallet instance (which sits at index 18).
	pub TreasuryInteriorMultiLocation: InteriorMultiLocation = PalletInstance(ENCOINTER_TREASURY_PALLET_ID).into();
	pub const MaxApprovals: u32 = 10;
	pub TreasuryAccount: AccountId = Treasury::account_id();
}

pub struct NoConversion;
impl ConversionFromAssetBalance<u128, (), u128> for NoConversion {
	type Error = ();
	fn from_asset_balance(balance: Balance, _asset_id: ()) -> Result<Balance, Self::Error> {
		return Ok(balance)
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn ensure_successful(_: ()) {}
}

impl pallet_treasury::Config for Runtime {
	type PalletId = TreasuryPalletId;
	type Currency = pallet_balances::Pallet<Runtime>;
	type ApproveOrigin = MoreThanHalfCouncil;
	type RejectOrigin = MoreThanHalfCouncil;
	type RuntimeEvent = RuntimeEvent;
	type OnSlash = (); //No proposal
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type ProposalBondMaximum = ProposalBondMaximum;
	type SpendPeriod = SpendPeriod; //Cannot be 0: Error: Thread 'tokio-runtime-worker' panicked at 'attempt to calculate the
								// remainder with a divisor of zero
	type Burn = (); //No burn
	type BurnDestination = (); //No burn
	type SpendFunds = (); //No spend, no bounty
	type MaxApprovals = MaxApprovals;
	type WeightInfo = weights::pallet_treasury::WeightInfo<Runtime>;
	type SpendOrigin = frame_support::traits::NeverEnsureOrigin<Balance>; //No spend, no bounty
	type AssetKind = ();
	type Beneficiary = AccountId;
	type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
	type Paymaster = PayFromAccount<Balances, TreasuryAccount>;
	type BalanceConverter = NoConversion;
	type PayoutPeriod = PayoutSpendPeriod;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

impl pallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = weights::pallet_utility::WeightInfo<Runtime>;
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
		RuntimeBlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
}

impl pallet_scheduler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = MoreThanHalfCouncil;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type Preimages = ();
}

parameter_types! {
	pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
}

impl cumulus_pallet_parachain_system::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnSystemEvent = ();
	type SelfParaId = parachain_info::Pallet<Runtime>;
	type DmpMessageHandler = DmpQueue;
	type ReservedDmpWeight = ReservedDmpWeight;
	type OutboundXcmpMessageSource = XcmpQueue;
	type XcmpMessageHandler = XcmpQueue;
	type ReservedXcmpWeight = ReservedXcmpWeight;
	type CheckAssociatedRelayNumber = RelayNumberMonotonicallyIncreases;
	type ConsensusHook = cumulus_pallet_aura_ext::FixedVelocityConsensusHook<
		Runtime,
		RELAY_CHAIN_SLOT_DURATION_MILLIS,
		BLOCK_PROCESSING_VELOCITY,
		UNINCLUDED_SEGMENT_CAPACITY,
	>;
}

impl pallet_insecure_randomness_collective_flip::Config for Runtime {}

impl parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

parameter_types! {
	pub const ExecutiveBody: BodyId = BodyId::Executive;
	/// The asset ID for the asset that we use to pay for message delivery fees.
	pub FeeAssetId: XcmAssetId = XcmAssetId::Concrete(xcm_config::KsmLocation::get());
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

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ChannelInfo = ParachainSystem;
	type VersionWrapper = PolkadotXcm;
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
	type ControllerOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		EnsureXcm<IsMajorityOfBody<KsmLocation, ExecutiveBody>>,
	>;
	type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
	type WeightInfo = weights::cumulus_pallet_xcmp_queue::WeightInfo<Runtime>;
	type PriceForSiblingDelivery = PriceForSiblingParachainDelivery;
}

// TODO: remove dmp with 1.3.0 (https://github.com/polkadot-fellows/runtimes/issues/186)
impl cumulus_pallet_dmp_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
}

parameter_types! {
	pub const Period: u32 = 6 * HOURS;
	pub const Offset: u32 = 0;
	pub const MaxAuthorities: u32 = 100_000;
}

parameter_types! {
	pub const MomentsPerDay: Moment = 86_400_000; // [ms/d]
	pub const DefaultDemurrage: Demurrage = Demurrage::from_bits(0x0000000000000000000001E3F0A8A973_i128);
	pub const EncointerExistentialDeposit: BalanceType = BalanceType::from_bits(0x0000000000000000000053e2d6238da4_u128);
	pub const MeetupSizeTarget: u64 = 10;
	pub const MeetupMinSize: u64 = 3;
	pub const MeetupNewbieLimitDivider: u64 = 2; // 2 means 1/3 of participants may be newbies
	pub const FaucetPalletId: PalletId = PalletId(*b"ectrfct0");
}

impl pallet_encointer_scheduler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnCeremonyPhaseChange = EncointerCeremonies;
	type MomentsPerDay = MomentsPerDay;
	type CeremonyMaster = MoreThanHalfCouncil;
	type WeightInfo = weights::pallet_encointer_scheduler::WeightInfo<Runtime>;
}

impl pallet_encointer_ceremonies::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
	type RandomnessSource = RandomnessCollectiveFlip;
	type MeetupSizeTarget = MeetupSizeTarget;
	type MeetupMinSize = MeetupMinSize;
	type MeetupNewbieLimitDivider = MeetupNewbieLimitDivider;
	type CeremonyMaster = MoreThanHalfCouncil;
	type WeightInfo = weights::pallet_encointer_ceremonies::WeightInfo<Runtime>;
	type MaxAttestations = ConstU32<100>;
}

impl pallet_encointer_communities::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type CommunityMaster = MoreThanHalfCouncil;
	type TrustableForNonDestructiveAction = MoreThanHalfCouncil;
	type WeightInfo = weights::pallet_encointer_communities::WeightInfo<Runtime>;
	type MaxCommunityIdentifiers = ConstU32<10000>;
	type MaxBootstrappers = ConstU32<10000>;
	type MaxLocationsPerGeohash = ConstU32<10000>;
	type MaxCommunityIdentifiersPerGeohash = ConstU32<10000>;
}

impl pallet_encointer_balances::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type DefaultDemurrage = DefaultDemurrage;
	type ExistentialDeposit = EncointerExistentialDeposit;
	type CeremonyMaster = MoreThanHalfCouncil;
	type WeightInfo = weights::pallet_encointer_balances::WeightInfo<Runtime>;
}

impl pallet_encointer_bazaar::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_encointer_bazaar::WeightInfo<Runtime>;
}

impl pallet_encointer_reputation_commitments::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_encointer_reputation_commitments::WeightInfo<Runtime>;
}

impl pallet_encointer_faucet::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ControllerOrigin = EnsureRoot<AccountId>;
	type Currency = Balances;
	type PalletId = FaucetPalletId;
	type WeightInfo = weights::pallet_encointer_faucet::WeightInfo<Runtime>;
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = MaxAuthorities;
	type AllowMultipleBlocksPerSlot = ConstBool<false>;
	#[cfg(feature = "experimental")]
	type SlotDuration = ConstU64<SLOT_DURATION>;
}

parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 7 * DAYS;
	pub const CouncilMaxProposals: u32 = 100;
	pub const CouncilMaxMembers: u32 = 100;
	pub MaxProposalWeight: Weight = sp_runtime::Perbill::from_percent(50) * RuntimeBlockWeights::get().max_block;
}

type MoreThanHalfCouncil = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
>;

pub type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Config<CouncilCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = CouncilMotionDuration;
	type MaxProposals = CouncilMaxProposals;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type MaxMembers = CouncilMaxMembers;
	type WeightInfo = weights::pallet_collective::WeightInfo<Runtime>;
	type SetMembersOrigin = MoreThanHalfCouncil;
	type MaxProposalWeight = MaxProposalWeight;
}

// support for collective pallet
impl pallet_membership::Config<pallet_membership::Instance1> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AddOrigin = MoreThanHalfCouncil;
	type RemoveOrigin = MoreThanHalfCouncil;
	type SwapOrigin = MoreThanHalfCouncil;
	type ResetOrigin = MoreThanHalfCouncil;
	type PrimeOrigin = MoreThanHalfCouncil;
	type MembershipInitialized = Collective;
	type MembershipChanged = Collective;
	type MaxMembers = CouncilMaxMembers;
	type WeightInfo = weights::pallet_membership::WeightInfo<Runtime>;
}

// Allow fee payment in community currency
impl pallet_asset_tx_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Fungibles = pallet_encointer_balances::Pallet<Runtime>;
	type OnChargeAssetTransaction = pallet_asset_tx_payment::FungiblesAdapter<
		encointer_balances_tx_payment::BalanceToCommunityBalance<Runtime>,
		encointer_balances_tx_payment::BurnCredit,
	>;
}

construct_runtime! {
	pub enum Runtime {
		// System support stuff.
		System: frame_system = 0,
		ParachainSystem: cumulus_pallet_parachain_system = 1,
		RandomnessCollectiveFlip: pallet_insecure_randomness_collective_flip = 2,
		Timestamp: pallet_timestamp = 3,
		ParachainInfo: parachain_info = 4,

		// Monetary stuff.
		Balances: pallet_balances = 10,
		TransactionPayment: pallet_transaction_payment = 11,
		AssetTxPayment: pallet_asset_tx_payment = 12,

		Aura: pallet_aura = 23,
		AuraExt: cumulus_pallet_aura_ext = 24,

		// XCM helpers.
		XcmpQueue: cumulus_pallet_xcmp_queue = 30,
		PolkadotXcm: pallet_xcm = 31,
		CumulusXcm: cumulus_pallet_xcm = 32,
		// TODO: remove dmp with 1.3.0 (https://github.com/polkadot-fellows/runtimes/issues/186)
		DmpQueue: cumulus_pallet_dmp_queue = 33,

		// Handy utilities.
		Utility: pallet_utility = 40,
		Treasury: pallet_treasury = 43,
		Proxy: pallet_proxy = 44,
		Scheduler: pallet_scheduler = 48,

		// Encointer council.
		Collective: pallet_collective::<Instance1> = 50,
		Membership: pallet_membership::<Instance1> = 51,

		EncointerScheduler: pallet_encointer_scheduler = 60,
		EncointerCeremonies: pallet_encointer_ceremonies = 61,
		EncointerCommunities: pallet_encointer_communities = 62,
		EncointerBalances: pallet_encointer_balances = 63,
		EncointerBazaar: pallet_encointer_bazaar = 64,
		EncointerReputationCommitments: pallet_encointer_reputation_commitments = 65,
		EncointerFaucet: pallet_encointer_faucet = 66,
	}
}

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_asset_tx_payment::ChargeAssetTxPayment<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, RuntimeCall, SignedExtra>;

/// Migrations to apply on runtime upgrade.
pub type Migrations = (
	// fixing the scheduler with a local migration is necessary because we have missed intermediate
	// migrations. the safest migration is, therefore, to clear all storage and bump StorageVersion
	migrations_fix::scheduler::v4::MigrateToV4<Runtime>,
	// also here we're actually too late with applying the migration. however, the migration does
	// work as-is.
	pallet_xcm::migration::v1::VersionUncheckedMigrateToV1<Runtime>,
	// balances are more tricky. We missed to do the migration to V1 and now we have inconsistent
	// state which can't be decoded to V0, yet the StorageVersion is V0.
	// the strategy is to: just pretend we're on V1
	migrations_fix::balances::v1::BruteForceToV1<Runtime>,
	// then reset to V0
	pallet_balances::migration::ResetInactive<Runtime>,
	//then apply the proper migration as we should have done earlier
	pallet_balances::migration::MigrateToTrackInactive<Runtime, xcm_config::CheckingAccount>,
);

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
	Migrations,
>;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	frame_benchmarking::define_benchmarks!(
		[frame_system, SystemBench::<Runtime>]
		[pallet_balances, Balances]
		[pallet_collective, Collective]
		[pallet_membership, Membership]
		[pallet_timestamp, Timestamp]
		// todo: treasury will be removed in separate PR, so no need to fix broken benchmarks: https://github.com/polkadot-fellows/runtimes/issues/176
		//[pallet_treasury, Treasury]
		[pallet_utility, Utility]
		[pallet_proxy, Proxy]
		[pallet_encointer_balances, EncointerBalances]
		[pallet_encointer_bazaar, EncointerBazaar]
		[pallet_encointer_ceremonies, EncointerCeremonies]
		[pallet_encointer_communities, EncointerCommunities]
		[pallet_encointer_faucet, EncointerFaucet]
		[pallet_encointer_reputation_commitments, EncointerReputationCommitments]
		[pallet_encointer_scheduler, EncointerScheduler]
		[cumulus_pallet_xcmp_queue, XcmpQueue]
	);
}

impl_runtime_apis! {
	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(SLOT_DURATION)
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities().into_inner()
		}
	}

	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
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

		fn metadata_versions() -> sp_std::vec::Vec<u32> {
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
			block: Block,
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

	impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
		fn collect_collation_info(header: &<Block as BlockT>::Header) -> cumulus_primitives_core::CollationInfo {
			ParachainSystem::collect_collation_info(header)
		}
	}

	impl pallet_encointer_ceremonies_rpc_runtime_api::CeremoniesApi<Block, AccountId, Moment> for Runtime {
		fn get_reputations(account: &AccountId) -> Vec<(CeremonyIndexType, CommunityReputation)> {
			EncointerCeremonies::get_reputations(account)
		}
		fn get_aggregated_account_data(cid:CommunityIdentifier, account: &AccountId) -> AggregatedAccountData<AccountId, Moment> {
			EncointerCeremonies::get_aggregated_account_data(cid, account)
		}
		fn get_ceremony_info() -> CeremonyInfo {
			EncointerCeremonies::get_ceremony_info()
		}
	}

	impl pallet_encointer_communities_rpc_runtime_api::CommunitiesApi<Block, AccountId, BlockNumber> for Runtime {
		fn get_all_balances(account: &AccountId) -> Vec<(CommunityIdentifier, BalanceEntry<BlockNumber>)> {
			EncointerCommunities::get_all_balances(account)
		}

		fn get_cids() -> Vec<CommunityIdentifier> {
			EncointerCommunities::get_cids()
		}

		fn get_name(cid: &CommunityIdentifier) -> Option<PalletString> {
			EncointerCommunities::get_name(cid)
		}

		fn get_locations(cid: &CommunityIdentifier) -> Vec<Location> {
			EncointerCommunities::get_locations(cid)
		}
	}

	impl pallet_encointer_bazaar_rpc_runtime_api::BazaarApi<Block, AccountId> for Runtime {
		fn get_offerings(business: &BusinessIdentifier<AccountId>) -> Vec<OfferingData>{
			EncointerBazaar::get_offerings(business)
		}

		fn get_businesses(community: &CommunityIdentifier) -> Vec<(AccountId, BusinessData)>{
			EncointerBazaar::get_businesses(community)
		}
	}

	impl encointer_balances_tx_payment_rpc_runtime_api::BalancesTxPaymentApi<Block, Balance, AssetId, AssetBalance> for Runtime {
		fn balance_to_asset_balance(amount: Balance, asset_id: AssetId) -> Result<AssetBalance, encointer_balances_tx_payment_rpc_runtime_api::Error> {
			BalanceToCommunityBalance::<Runtime>::to_asset_balance(amount, asset_id).map_err(|_e|
				encointer_balances_tx_payment_rpc_runtime_api::Error::RuntimeError
			)
		}
	}

	impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
		fn create_default_config() -> Vec<u8> {
			create_default_config::<RuntimeGenesisConfig>()
		}

		fn build_config(config: Vec<u8>) -> sp_genesis_builder::Result {
			build_config::<RuntimeGenesisConfig>(config)
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here. If any of the pre/post migration checks fail, we shall stop
			// right here and right now.
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, RuntimeBlockWeights::get().max_block)
		}

		fn execute_block(
			block: Block,
			state_root_check: bool,
			signature_check: bool,
			select: frame_try_runtime::TryStateSelect
		) -> Weight {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here.
			Executive::try_execute_block(block, state_root_check, signature_check, select).expect("execute-block failed")
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;
			use frame_system_benchmarking::Pallet as SystemBench;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();
			return (list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{Benchmarking, BenchmarkBatch, BenchmarkError};
			use frame_support::traits::TrackedStorageKey;

			use frame_system_benchmarking::Pallet as SystemBench;
			impl frame_system_benchmarking::Config for Runtime {
				fn setup_set_code_requirements(code: &sp_std::vec::Vec<u8>) -> Result<(), BenchmarkError> {
					ParachainSystem::initialize_for_set_code_benchmark(code.len() as u32);
					Ok(())
				}

				fn verify_set_code() {
					System::assert_last_event(cumulus_pallet_parachain_system::Event::<Runtime>::ValidationFunctionStored.into());
				}
			}

			let whitelist: Vec<TrackedStorageKey> = vec![
				// Block Number
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac").to_vec().into(),
				// Total Issuance
				hex_literal::hex!("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80").to_vec().into(),
				// Execution Phase
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a").to_vec().into(),
				// Event Count
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850").to_vec().into(),
				// System Events
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7").to_vec().into(),
				// Treasury Account
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da95ecffd7b6c0f78751baa9d281e0bfa3a6d6f646c70792f74727372790000000000000000000000000000000000000000").to_vec().into(),
			];

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);

			if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
			Ok(batches)
		}
	}
}

cumulus_pallet_parachain_system::register_validate_block! {
	Runtime = Runtime,
	BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
}

#[cfg(test)]
mod multiplier_tests {
	use super::*;
	use frame_support::pallet_prelude::PalletInfoAccess;

	#[test]
	fn treasury_pallet_index_is_correct() {
		assert_eq!(ENCOINTER_TREASURY_PALLET_ID, <Treasury as PalletInfoAccess>::index() as u8);
	}
}

#[test]
fn test_ed_is_one_tenth_of_relay() {
	let relay_ed = kusama_runtime_constants::currency::EXISTENTIAL_DEPOSIT;
	let encointer_ed = ExistentialDeposit::get();
	assert_eq!(relay_ed / 10, encointer_ed);
}

#[test]
fn test_constants_compatiblity() {
	assert_eq!(
		::kusama_runtime_constants::currency::EXISTENTIAL_DEPOSIT,
		system_parachains_constants::kusama_runtime_constants::currency::EXISTENTIAL_DEPOSIT
	);
	assert_eq!(
		::kusama_runtime_constants::currency::deposit(5, 3),
		system_parachains_constants::kusama_runtime_constants::currency::deposit(5, 3)
	);
	assert_eq!(
		::system_parachains_constants::AVERAGE_ON_INITIALIZE_RATIO * 1u32,
		system_parachains_constants::AVERAGE_ON_INITIALIZE_RATIO * 1u32
	);
	assert_eq!(
		::system_parachains_constants::NORMAL_DISPATCH_RATIO * 1u32,
		system_parachains_constants::NORMAL_DISPATCH_RATIO * 1u32
	);
	assert_eq!(
		::system_parachains_constants::MAXIMUM_BLOCK_WEIGHT.encode(),
		system_parachains_constants::MAXIMUM_BLOCK_WEIGHT.encode()
	);
	assert_eq!(::system_parachains_constants::MINUTES, system_parachains_constants::MINUTES);
	assert_eq!(::system_parachains_constants::HOURS, system_parachains_constants::HOURS);
	assert_eq!(::system_parachains_constants::DAYS, system_parachains_constants::DAYS);
	assert_eq!(
		::system_parachains_constants::kusama::currency::SYSTEM_PARA_EXISTENTIAL_DEPOSIT,
		system_parachains_constants::kusama::currency::SYSTEM_PARA_EXISTENTIAL_DEPOSIT
	);
	assert_eq!(
		::system_parachains_constants::kusama::currency::UNITS,
		system_parachains_constants::kusama::currency::UNITS
	);
	assert_eq!(
		::system_parachains_constants::kusama::currency::QUID,
		system_parachains_constants::kusama::currency::QUID
	);
	assert_eq!(
		::system_parachains_constants::kusama::currency::CENTS,
		system_parachains_constants::kusama::currency::CENTS
	);
	assert_eq!(
		::system_parachains_constants::kusama::currency::MILLICENTS,
		system_parachains_constants::kusama::currency::MILLICENTS
	);
	assert_eq!(
		::system_parachains_constants::kusama::currency::system_para_deposit(5, 3),
		system_parachains_constants::kusama::currency::system_para_deposit(5, 3)
	);
	assert_eq!(
		::system_parachains_constants::kusama::fee::calculate_weight_to_fee(
			&::system_parachains_constants::MAXIMUM_BLOCK_WEIGHT
		),
		system_parachains_constants::kusama::fee::calculate_weight_to_fee(
			&system_parachains_constants::MAXIMUM_BLOCK_WEIGHT
		)
	);
}

// TODO: Encointer pallets does not have compatible `polkadot-sdk` versions,
// so we cannot easily reuse `system-parachains-constants` module.
mod system_parachains_constants {
	use super::*;
	use frame_support::weights::constants::WEIGHT_REF_TIME_PER_SECOND;

	/// This determines the average expected block time that we are targeting. Blocks will be
	/// produced at a minimum duration defined by `SLOT_DURATION`. `SLOT_DURATION` is picked up by
	/// `pallet_timestamp` which is in turn picked up by `pallet_aura` to implement `fn
	/// slot_duration()`.
	///
	/// Change this to adjust the block time.
	pub const MILLISECS_PER_BLOCK: u64 = 12000;
	pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

	// Time is measured by number of blocks.
	pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
	pub const HOURS: BlockNumber = MINUTES * 60;
	pub const DAYS: BlockNumber = HOURS * 24;

	/// We assume that ~5% of the block weight is consumed by `on_initialize` handlers. This is
	/// used to limit the maximal weight of a single extrinsic.
	pub const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(5);
	/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used by
	/// Operational  extrinsics.
	pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

	/// We allow for 0.5 seconds of compute with a 6 second average block time.
	pub const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
		WEIGHT_REF_TIME_PER_SECOND.saturating_div(2),
		polkadot_primitives::MAX_POV_SIZE as u64,
	);

	pub(crate) mod kusama {
		/// Consensus-related.
		pub mod consensus {
			/// Maximum number of blocks simultaneously accepted by the Runtime, not yet included
			/// into the relay chain.
			pub const UNINCLUDED_SEGMENT_CAPACITY: u32 = 1;
			/// How many parachain blocks are processed by the relay chain per parent. Limits the
			/// number of blocks authored per slot.
			pub const BLOCK_PROCESSING_VELOCITY: u32 = 1;
			/// Relay chain slot duration, in milliseconds.
			pub const RELAY_CHAIN_SLOT_DURATION_MILLIS: u32 = 6000;
		}

		/// Constants relating to KSM.
		pub mod currency {
			use super::super::kusama_runtime_constants;
			use polkadot_core_primitives::Balance;

			/// The default existential deposit for system chains. 1/10th of the Relay Chain's
			/// existential deposit. Individual system parachains may modify this in special cases.
			pub const SYSTEM_PARA_EXISTENTIAL_DEPOSIT: Balance =
				kusama_runtime_constants::currency::EXISTENTIAL_DEPOSIT / 10;

			/// One "KSM" that a UI would show a user.
			pub const UNITS: Balance = 1_000_000_000_000;
			pub const QUID: Balance = UNITS / 30;
			pub const CENTS: Balance = QUID / 100;
			pub const MILLICENTS: Balance = CENTS / 1_000;

			/// Deposit rate for stored data. 1/100th of the Relay Chain's deposit rate. `items` is
			/// the number of keys in storage and `bytes` is the size of the value.
			pub const fn system_para_deposit(items: u32, bytes: u32) -> Balance {
				kusama_runtime_constants::currency::deposit(items, bytes) / 100
			}
		}

		/// Constants related to Kusama fee payment.
		pub mod fee {
			use frame_support::{
				pallet_prelude::Weight,
				weights::{
					constants::ExtrinsicBaseWeight, FeePolynomial, WeightToFeeCoefficient,
					WeightToFeeCoefficients, WeightToFeePolynomial,
				},
			};
			use polkadot_core_primitives::Balance;
			use smallvec::smallvec;
			pub use sp_runtime::Perbill;

			/// Handles converting a weight scalar to a fee value, based on the scale and
			/// granularity of the node's balance type.
			///
			/// This should typically create a mapping between the following ranges:
			///   - [0, MAXIMUM_BLOCK_WEIGHT]
			///   - [Balance::min, Balance::max]
			///
			/// Yet, it can be used for any other sort of change to weight-fee. Some examples being:
			///   - Setting it to `0` will essentially disable the weight fee.
			///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
			pub struct WeightToFee;

			impl frame_support::weights::WeightToFee for WeightToFee {
				type Balance = Balance;

				fn weight_to_fee(weight: &Weight) -> Self::Balance {
					let time_poly: FeePolynomial<Balance> = RefTimeToFee::polynomial().into();
					let proof_poly: FeePolynomial<Balance> = ProofSizeToFee::polynomial().into();

					// Take the maximum instead of the sum to charge by the more scarce resource.
					time_poly.eval(weight.ref_time()).max(proof_poly.eval(weight.proof_size()))
				}
			}

			/// Maps the reference time component of `Weight` to a fee.
			pub struct RefTimeToFee;

			impl WeightToFeePolynomial for RefTimeToFee {
				type Balance = Balance;
				fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
					// In Kusama, extrinsic base weight (smallest non-zero weight) is mapped to 1/10
					// CENT: The standard system parachain configuration is 1/10 of that, as in
					// 1/100 CENT.
					let p = super::currency::CENTS;
					let q = 100 * Balance::from(ExtrinsicBaseWeight::get().ref_time());

					smallvec![WeightToFeeCoefficient {
						degree: 1,
						negative: false,
						coeff_frac: Perbill::from_rational(p % q, q),
						coeff_integer: p / q,
					}]
				}
			}

			/// Maps the proof size component of `Weight` to a fee.
			pub struct ProofSizeToFee;

			impl WeightToFeePolynomial for ProofSizeToFee {
				type Balance = Balance;
				fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
					// Map 10kb proof to 1 CENT.
					let p = super::currency::CENTS;
					let q = 10_000;

					smallvec![WeightToFeeCoefficient {
						degree: 1,
						negative: false,
						coeff_frac: Perbill::from_rational(p % q, q),
						coeff_integer: p / q,
					}]
				}
			}

			#[cfg(test)]
			pub fn calculate_weight_to_fee(weight: &Weight) -> Balance {
				<WeightToFee as frame_support::weights::WeightToFee>::weight_to_fee(weight)
			}
		}
	}

	// TODO: Encointer pallets does not have compatible `polkadot-sdk` versions,
	// so we cannot easily reuse `kusama-runtime-constants` module.
	pub(crate) mod kusama_runtime_constants {
		/// Money matters.
		pub mod currency {
			use polkadot_primitives::Balance;

			/// The existential deposit.
			pub const EXISTENTIAL_DEPOSIT: Balance = 1 * CENTS;

			pub const UNITS: Balance = 1_000_000_000_000;
			pub const QUID: Balance = UNITS / 30;
			pub const CENTS: Balance = QUID / 100;
			pub const MILLICENTS: Balance = CENTS / 1_000;

			pub const fn deposit(items: u32, bytes: u32) -> Balance {
				items as Balance * 2_000 * CENTS + (bytes as Balance) * 100 * MILLICENTS
			}
		}
	}
}
