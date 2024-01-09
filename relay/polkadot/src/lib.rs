// Copyright (C) Parity Technologies (UK) Ltd.
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
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! The Polkadot runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "512"]

use pallet_transaction_payment::CurrencyAdapter;
use runtime_common::{
	auctions, claims, crowdloan, impl_runtime_weights,
	impls::{
		DealWithFees, LocatableAssetConverter, VersionedLocatableAsset,
		VersionedMultiLocationConverter,
	},
	paras_registrar, prod_or_fast, slots, BlockHashCount, BlockLength, CurrencyToVote,
	SlowAdjustingFeeUpdate,
};

use runtime_parachains::{
	assigner_parachains as parachains_assigner_parachains,
	configuration as parachains_configuration, disputes as parachains_disputes,
	disputes::slashing as parachains_slashing,
	dmp as parachains_dmp, hrmp as parachains_hrmp, inclusion as parachains_inclusion,
	inclusion::{AggregateMessageOrigin, UmpQueueId},
	initializer as parachains_initializer, origin as parachains_origin, paras as parachains_paras,
	paras_inherent as parachains_paras_inherent, reward_points as parachains_reward_points,
	runtime_api_impl::v7 as parachains_runtime_api_impl,
	scheduler as parachains_scheduler, session_info as parachains_session_info,
	shared as parachains_shared,
};

use authority_discovery_primitives::AuthorityId as AuthorityDiscoveryId;
use beefy_primitives::{
	ecdsa_crypto::{AuthorityId as BeefyId, Signature as BeefySignature},
	mmr::{BeefyDataProvider, MmrLeafVersion},
};
use frame_election_provider_support::{
	bounds::ElectionBoundsBuilder, generate_solution_type, onchain, SequentialPhragmen,
};
use frame_support::{
	construct_runtime,
	genesis_builder_helper::{build_config, create_default_config},
	parameter_types,
	traits::{
		fungible::HoldConsideration, ConstU32, EitherOf, EitherOfDiverse, Get, InstanceFilter,
		KeyOwnerProofSystem, LinearStoragePrice, PrivilegeCmp, ProcessMessage, ProcessMessageError,
		WithdrawReasons,
	},
	weights::{ConstantMultiplier, WeightMeter},
	PalletId,
};
use frame_system::EnsureRoot;
use pallet_grandpa::{fg_primitives, AuthorityId as GrandpaId};
use pallet_identity::simple::IdentityInfo;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pallet_session::historical as session_historical;
use pallet_transaction_payment::{FeeDetails, RuntimeDispatchInfo};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use primitives::{
	slashing, AccountId, AccountIndex, Balance, BlockNumber, CandidateEvent, CandidateHash,
	CommittedCandidateReceipt, CoreState, DisputeState, ExecutorParams, GroupRotationInfo, Hash,
	Id as ParaId, InboundDownwardMessage, InboundHrmpMessage, Moment, Nonce,
	OccupiedCoreAssumption, PersistedValidationData, ScrapedOnChainVotes, SessionInfo, Signature,
	ValidationCode, ValidationCodeHash, ValidatorId, ValidatorIndex, LOWEST_PUBLIC_ID,
	PARACHAIN_KEY_TYPE_ID,
};
use sp_core::{OpaqueMetadata, H256};
use sp_runtime::{
	create_runtime_str,
	curve::PiecewiseLinear,
	generic, impl_opaque_keys,
	traits::{
		AccountIdLookup, BlakeTwo256, Block as BlockT, ConvertInto, Extrinsic as ExtrinsicT,
		IdentityLookup, Keccak256, OpaqueKeys, SaturatedConversion, Verify,
	},
	transaction_validity::{TransactionPriority, TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, FixedU128, KeyTypeId, Perbill, Percent, Permill, RuntimeDebug,
};
use sp_staking::SessionIndex;
use sp_std::{cmp::Ordering, collections::btree_map::BTreeMap, prelude::*};
#[cfg(any(feature = "std", test))]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
use xcm::{
	latest::{InteriorMultiLocation, Junction, Junction::PalletInstance},
	VersionedMultiLocation,
};
use xcm_builder::PayOverXcm;

pub use frame_system::Call as SystemCall;
pub use pallet_balances::Call as BalancesCall;
pub use pallet_election_provider_multi_phase::{Call as EPMCall, GeometricDepositBase};
#[cfg(feature = "std")]
pub use pallet_staking::StakerStatus;
use pallet_staking::UseValidatorsMap;
pub use pallet_timestamp::Call as TimestampCall;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

/// Constant values used within the runtime.
use polkadot_runtime_constants::{currency::*, fee::*, time::*, TREASURY_PALLET_ID};

// Weights used in the runtime.
mod weights;

mod bag_thresholds;

// Governance configurations.
pub mod governance;
use governance::{
	pallet_custom_origins, AuctionAdmin, FellowshipAdmin, GeneralAdmin, LeaseAdmin, StakingAdmin,
	Treasurer, TreasurySpender,
};

pub mod xcm_config;

pub const LOG_TARGET: &'static str = "runtime::polkadot";

impl_runtime_weights!(polkadot_runtime_constants);

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

// Polkadot version identifier;
/// Runtime version (Polkadot).
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("polkadot"),
	impl_name: create_runtime_str!("parity-polkadot"),
	authoring_version: 0,
	spec_version: 1_001_000,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 24,
	state_version: 0,
};

/// The BABE epoch configuration at genesis.
pub const BABE_GENESIS_EPOCH_CONFIG: babe_primitives::BabeEpochConfiguration =
	babe_primitives::BabeEpochConfiguration {
		c: PRIMARY_PROBABILITY,
		allowed_slots: babe_primitives::AllowedSlots::PrimaryAndSecondaryVRFSlots,
	};

/// Native version.
#[cfg(any(feature = "std", test))]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
	pub const SS58Prefix: u8 = 0;
}

impl frame_system::Config for Runtime {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = BlockWeights;
	type BlockLength = BlockLength;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = Nonce;
	type Hash = Hash;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = AccountIdLookup<AccountId, ()>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type DbWeight = RocksDbWeight;
	type Version = Version;
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = weights::frame_system::WeightInfo<Runtime>;
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
		BlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
	pub const NoPreimagePostponement: Option<u32> = Some(10);
}

/// Used the compare the privilege of an origin inside the scheduler.
pub struct OriginPrivilegeCmp;

impl PrivilegeCmp<OriginCaller> for OriginPrivilegeCmp {
	fn cmp_privilege(left: &OriginCaller, right: &OriginCaller) -> Option<Ordering> {
		if left == right {
			return Some(Ordering::Equal)
		}

		match (left, right) {
			// Root is greater than anything.
			(OriginCaller::system(frame_system::RawOrigin::Root), _) => Some(Ordering::Greater),
			// For every other origin we don't care, as they are not used for `ScheduleOrigin`.
			_ => None,
		}
	}
}

impl pallet_scheduler::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	// The goal of having ScheduleOrigin include AuctionAdmin is to allow the auctions track of
	// OpenGov to schedule periodic auctions.
	// Also allow Treasurer to schedule recurring payments.
	type ScheduleOrigin = EitherOf<EitherOf<EnsureRoot<AccountId>, AuctionAdmin>, Treasurer>;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type WeightInfo = weights::pallet_scheduler::WeightInfo<Runtime>;
	type OriginPrivilegeCmp = OriginPrivilegeCmp;
	type Preimages = Preimage;
}

parameter_types! {
	pub const PreimageBaseDeposit: Balance = deposit(2, 64);
	pub const PreimageByteDeposit: Balance = deposit(0, 1);
	pub const PreimageHoldReason: RuntimeHoldReason =
		RuntimeHoldReason::Preimage(pallet_preimage::HoldReason::Preimage);
}

impl pallet_preimage::Config for Runtime {
	type WeightInfo = weights::pallet_preimage::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type Consideration = HoldConsideration<
		AccountId,
		Balances,
		PreimageHoldReason,
		LinearStoragePrice<PreimageBaseDeposit, PreimageByteDeposit, Balance>,
	>;
}

parameter_types! {
	pub EpochDuration: u64 = prod_or_fast!(
		EPOCH_DURATION_IN_SLOTS as u64,
		2 * MINUTES as u64,
		"DOT_EPOCH_DURATION"
	);
	pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
	pub ReportLongevity: u64 =
		BondingDuration::get() as u64 * SessionsPerEra::get() as u64 * EpochDuration::get();
}

impl pallet_babe::Config for Runtime {
	type EpochDuration = EpochDuration;
	type ExpectedBlockTime = ExpectedBlockTime;

	// session module is the trigger
	type EpochChangeTrigger = pallet_babe::ExternalTrigger;

	type DisabledValidators = Session;

	type WeightInfo = ();

	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = MaxNominatorRewardedPerValidator;

	type KeyOwnerProof =
		<Historical as KeyOwnerProofSystem<(KeyTypeId, pallet_babe::AuthorityId)>>::Proof;

	type EquivocationReportSystem =
		pallet_babe::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

parameter_types! {
	pub const IndexDeposit: Balance = 10 * DOLLARS;
}

impl pallet_indices::Config for Runtime {
	type AccountIndex = AccountIndex;
	type Currency = Balances;
	type Deposit = IndexDeposit;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_indices::WeightInfo<Runtime>;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = weights::pallet_balances::WeightInfo<Runtime>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxHolds = ConstU32<1>;
	type MaxFreezes = ConstU32<8>;
}

parameter_types! {
	pub BeefySetIdSessionEntries: u32 = BondingDuration::get() * SessionsPerEra::get();
}

impl pallet_beefy::Config for Runtime {
	type BeefyId = BeefyId;
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = MaxNominatorRewardedPerValidator;
	type MaxSetIdSessionEntries = BeefySetIdSessionEntries;
	type OnNewValidatorSet = BeefyMmrLeaf;
	type WeightInfo = ();
	type KeyOwnerProof = <Historical as KeyOwnerProofSystem<(KeyTypeId, BeefyId)>>::Proof;
	type EquivocationReportSystem =
		pallet_beefy::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

impl pallet_mmr::Config for Runtime {
	const INDEXING_PREFIX: &'static [u8] = mmr::INDEXING_PREFIX;
	type Hashing = Keccak256;
	type OnNewRoot = pallet_beefy_mmr::DepositBeefyDigest<Runtime>;
	type WeightInfo = ();
	type LeafData = pallet_beefy_mmr::Pallet<Runtime>;
}

/// MMR helper types.
mod mmr {
	use super::Runtime;
	pub use pallet_mmr::primitives::*;

	pub type Leaf = <<Runtime as pallet_mmr::Config>::LeafData as LeafDataProvider>::LeafData;
	pub type Hashing = <Runtime as pallet_mmr::Config>::Hashing;
	pub type Hash = <Hashing as sp_runtime::traits::Hash>::Output;
}

parameter_types! {
	/// Version of the produced MMR leaf.
	///
	/// The version consists of two parts;
	/// - `major` (3 bits)
	/// - `minor` (5 bits)
	///
	/// `major` should be updated only if decoding the previous MMR Leaf format from the payload
	/// is not possible (i.e. backward incompatible change).
	/// `minor` should be updated if fields are added to the previous MMR Leaf, which given SCALE
	/// encoding does not prevent old leafs from being decoded.
	///
	/// Hence we expect `major` to be changed really rarely (think never).
	/// See [`MmrLeafVersion`] type documentation for more details.
	pub LeafVersion: MmrLeafVersion = MmrLeafVersion::new(0, 0);
}

/// A BEEFY data provider that merkelizes all the parachain heads at the current block
/// (sorted by their parachain id).
pub struct ParaHeadsRootProvider;
impl BeefyDataProvider<H256> for ParaHeadsRootProvider {
	fn extra_data() -> H256 {
		let mut para_heads: Vec<(u32, Vec<u8>)> = Paras::parachains()
			.into_iter()
			.filter_map(|id| Paras::para_head(&id).map(|head| (id.into(), head.0)))
			.collect();
		para_heads.sort_by_key(|k| k.0);
		binary_merkle_tree::merkle_root::<mmr::Hashing, _>(
			para_heads.into_iter().map(|pair| pair.encode()),
		)
		.into()
	}
}

impl pallet_beefy_mmr::Config for Runtime {
	type LeafVersion = LeafVersion;
	type BeefyAuthorityToMerkleLeaf = pallet_beefy_mmr::BeefyEcdsaToEthereum;
	type LeafExtra = H256;
	type BeefyDataProvider = ParaHeadsRootProvider;
}

parameter_types! {
	pub const TransactionByteFee: Balance = 10 * MILLICENTS;
	/// This value increases the priority of `Operational` transactions by adding
	/// a "virtual tip" that's equal to the `OperationalFeeMultiplier * final_fee`.
	pub const OperationalFeeMultiplier: u8 = 5;
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = CurrencyAdapter<Balances, DealWithFees<Runtime>>;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type WeightToFee = WeightToFee;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
}

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}
impl pallet_timestamp::Config for Runtime {
	type Moment = u64;
	type OnTimestampSet = Babe;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = weights::pallet_timestamp::WeightInfo<Runtime>;
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
	type EventHandler = (Staking, ImOnline);
}

impl_opaque_keys! {
	pub struct OldSessionKeys {
		pub grandpa: Grandpa,
		pub babe: Babe,
		pub im_online: ImOnline,
		pub para_validator: Initializer,
		pub para_assignment: ParaSessionInfo,
		pub authority_discovery: AuthorityDiscovery,
	}
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub grandpa: Grandpa,
		pub babe: Babe,
		pub im_online: ImOnline,
		pub para_validator: Initializer,
		pub para_assignment: ParaSessionInfo,
		pub authority_discovery: AuthorityDiscovery,
		pub beefy: Beefy,
	}
}

// remove this when removing `OldSessionKeys`
fn transform_session_keys(v: AccountId, old: OldSessionKeys) -> SessionKeys {
	SessionKeys {
		grandpa: old.grandpa,
		babe: old.babe,
		im_online: old.im_online,
		para_validator: old.para_validator,
		para_assignment: old.para_assignment,
		authority_discovery: old.authority_discovery,
		beefy: {
			// From Session::upgrade_keys():
			//
			// Care should be taken that the raw versions of the
			// added keys are unique for every `ValidatorId, KeyTypeId` combination.
			// This is an invariant that the session pallet typically maintains internally.
			//
			// So, produce a dummy value that's unique for the `ValidatorId, KeyTypeId` combination.
			let mut id: BeefyId = sp_application_crypto::ecdsa::Public::from_raw([0u8; 33]).into();
			let id_raw: &mut [u8] = id.as_mut();
			id_raw[1..33].copy_from_slice(v.as_ref());
			id_raw[0..4].copy_from_slice(b"beef");
			id
		},
	}
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = AccountId;
	type ValidatorIdOf = pallet_staking::StashOf<Self>;
	type ShouldEndSession = Babe;
	type NextSessionRotation = Babe;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type WeightInfo = weights::pallet_session::WeightInfo<Runtime>;
}

impl pallet_session::historical::Config for Runtime {
	type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
	type FullIdentificationOf = pallet_staking::ExposureOf<Runtime>;
}

parameter_types! {
	// phase durations. 1/4 of the last session for each.
	// in testing: 1min or half of the session for each
	pub SignedPhase: u32 = prod_or_fast!(
		EPOCH_DURATION_IN_SLOTS / 4,
		(1 * MINUTES).min(EpochDuration::get().saturated_into::<u32>() / 2),
		"DOT_SIGNED_PHASE"
	);
	pub UnsignedPhase: u32 = prod_or_fast!(
		EPOCH_DURATION_IN_SLOTS / 4,
		(1 * MINUTES).min(EpochDuration::get().saturated_into::<u32>() / 2),
		"DOT_UNSIGNED_PHASE"
	);

	// signed config
	pub const SignedMaxSubmissions: u32 = 16;
	pub const SignedMaxRefunds: u32 = 16 / 4;
	pub const SignedFixedDeposit: Balance = deposit(2, 0);
	pub const SignedDepositIncreaseFactor: Percent = Percent::from_percent(10);
	// 0.01 DOT per KB of solution data.
	pub const SignedDepositByte: Balance = deposit(0, 10) / 1024;
	// Each good submission will get 1 DOT as reward
	pub SignedRewardBase: Balance = 1 * UNITS;
	pub BetterUnsignedThreshold: Perbill = Perbill::from_rational(5u32, 10_000);

	// 4 hour session, 1 hour unsigned phase, 32 offchain executions.
	pub OffchainRepeat: BlockNumber = UnsignedPhase::get() / 32;

	pub const MaxElectingVoters: u32 = 22_500;
	/// We take the top 22500 nominators as electing voters and all of the validators as electable
	/// targets. Whilst this is the case, we cannot and shall not increase the size of the
	/// validator intentions.
	pub ElectionBounds: frame_election_provider_support::bounds::ElectionBounds =
		ElectionBoundsBuilder::default().voters_count(MaxElectingVoters::get().into()).build();
	/// Setup election pallet to support maximum winners upto 1200. This will mean Staking Pallet
	/// cannot have active validators higher than this count.
	pub const MaxActiveValidators: u32 = 1200;
}

generate_solution_type!(
	#[compact]
	pub struct NposCompactSolution16::<
		VoterIndex = u32,
		TargetIndex = u16,
		Accuracy = sp_runtime::PerU16,
		MaxVoters = MaxElectingVoters,
	>(16)
);

pub struct OnChainSeqPhragmen;
impl onchain::Config for OnChainSeqPhragmen {
	type System = Runtime;
	type Solver = SequentialPhragmen<AccountId, runtime_common::elections::OnChainAccuracy>;
	type DataProvider = Staking;
	type WeightInfo = weights::frame_election_provider_support::WeightInfo<Runtime>;
	type MaxWinners = MaxActiveValidators;
	type Bounds = ElectionBounds;
}

impl pallet_election_provider_multi_phase::MinerConfig for Runtime {
	type AccountId = AccountId;
	type MaxLength = OffchainSolutionLengthLimit;
	type MaxWeight = OffchainSolutionWeightLimit;
	type Solution = NposCompactSolution16;
	type MaxVotesPerVoter = <
		<Self as pallet_election_provider_multi_phase::Config>::DataProvider
		as
		frame_election_provider_support::ElectionDataProvider
	>::MaxVotesPerVoter;
	type MaxWinners = MaxActiveValidators;

	// The unsigned submissions have to respect the weight of the submit_unsigned call, thus their
	// weight estimate function is wired to this call's weight.
	fn solution_weight(v: u32, t: u32, a: u32, d: u32) -> Weight {
		<
			<Self as pallet_election_provider_multi_phase::Config>::WeightInfo
			as
			pallet_election_provider_multi_phase::WeightInfo
		>::submit_unsigned(v, t, a, d)
	}
}

impl pallet_election_provider_multi_phase::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type EstimateCallFee = TransactionPayment;
	type SignedPhase = SignedPhase;
	type UnsignedPhase = UnsignedPhase;
	type SignedMaxSubmissions = SignedMaxSubmissions;
	type SignedMaxRefunds = SignedMaxRefunds;
	type SignedRewardBase = SignedRewardBase;
	type SignedDepositBase =
		GeometricDepositBase<Balance, SignedFixedDeposit, SignedDepositIncreaseFactor>;
	type SignedDepositByte = SignedDepositByte;
	type SignedDepositWeight = ();
	type SignedMaxWeight =
		<Self::MinerConfig as pallet_election_provider_multi_phase::MinerConfig>::MaxWeight;
	type MinerConfig = Self;
	type SlashHandler = (); // burn slashes
	type RewardHandler = (); // nothing to do upon rewards
	type BetterUnsignedThreshold = BetterUnsignedThreshold;
	type BetterSignedThreshold = ();
	type OffchainRepeat = OffchainRepeat;
	type MinerTxPriority = NposSolutionPriority;
	type DataProvider = Staking;
	#[cfg(any(feature = "fast-runtime", feature = "runtime-benchmarks"))]
	type Fallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
	#[cfg(not(any(feature = "fast-runtime", feature = "runtime-benchmarks")))]
	type Fallback = frame_election_provider_support::NoElection<(
		AccountId,
		BlockNumber,
		Staking,
		MaxActiveValidators,
	)>;
	type GovernanceFallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type Solver = SequentialPhragmen<
		AccountId,
		pallet_election_provider_multi_phase::SolutionAccuracyOf<Self>,
		(),
	>;
	type BenchmarkingConfig = runtime_common::elections::BenchmarkConfig;
	type ForceOrigin = EitherOf<EnsureRoot<Self::AccountId>, StakingAdmin>;
	type WeightInfo = weights::pallet_election_provider_multi_phase::WeightInfo<Self>;
	type MaxWinners = MaxActiveValidators;
	type ElectionBounds = ElectionBounds;
}

parameter_types! {
	pub const BagThresholds: &'static [u64] = &bag_thresholds::THRESHOLDS;
}

type VoterBagsListInstance = pallet_bags_list::Instance1;
impl pallet_bags_list::Config<VoterBagsListInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ScoreProvider = Staking;
	type WeightInfo = weights::pallet_bags_list::WeightInfo<Runtime>;
	type BagThresholds = BagThresholds;
	type Score = sp_npos_elections::VoteWeight;
}

// TODO #6469: This shouldn't be static, but a lazily cached value, not built unless needed, and
// re-built in case input parameters have changed. The `ideal_stake` should be determined by the
// amount of parachain slots being bid on: this should be around `(75 - 25.min(slots / 4))%`.
pallet_staking_reward_curve::build! {
	const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
		min_inflation: 0_025_000,
		max_inflation: 0_100_000,
		// 3:2:1 staked : parachains : float.
		// while there's no parachains, then this is 75% staked : 25% float.
		ideal_stake: 0_750_000,
		falloff: 0_050_000,
		max_piece_count: 40,
		test_precision: 0_005_000,
	);
}

parameter_types! {
	// Six sessions in an era (24 hours).
	pub const SessionsPerEra: SessionIndex = prod_or_fast!(6, 1);

	// 28 eras for unbonding (28 days).
	pub BondingDuration: sp_staking::EraIndex = prod_or_fast!(
		28,
		28,
		"DOT_BONDING_DURATION"
	);
	pub SlashDeferDuration: sp_staking::EraIndex = prod_or_fast!(
		27,
		27,
		"DOT_SLASH_DEFER_DURATION"
	);
	pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
	pub const MaxNominatorRewardedPerValidator: u32 = 512;
	pub const OffendingValidatorsThreshold: Perbill = Perbill::from_percent(17);
	// 16
	pub const MaxNominations: u32 = <NposCompactSolution16 as frame_election_provider_support::NposSolution>::LIMIT as u32;
}

/// Custom version of `runtime_commong::era_payout` somewhat tailored for Polkadot's crowdloan
/// unlock history. The only tweak should be
///
/// ```diff
/// - let auction_proportion = Perquintill::from_rational(auctioned_slots.min(60), 200u64);
/// + let auction_proportion = Perquintill::from_rational(auctioned_slots.min(60), 300u64);
/// ```
///
/// See <https://forum.polkadot.network/t/adjusting-polkadots-ideal-staking-rate-calculation/3897>.
fn polkadot_era_payout(
	total_staked: Balance,
	total_stakable: Balance,
	max_annual_inflation: Perquintill,
	period_fraction: Perquintill,
	auctioned_slots: u64,
) -> (Balance, Balance) {
	use pallet_staking_reward_fn::compute_inflation;
	use sp_runtime::traits::Saturating;

	let min_annual_inflation = Perquintill::from_rational(25u64, 1000u64);
	let delta_annual_inflation = max_annual_inflation.saturating_sub(min_annual_inflation);

	// 20% reserved for up to 60 slots.
	let auction_proportion = Perquintill::from_rational(auctioned_slots.min(60), 300u64);

	// Therefore the ideal amount at stake (as a percentage of total issuance) is 75% less the
	// amount that we expect to be taken up with auctions.
	let ideal_stake = Perquintill::from_percent(75).saturating_sub(auction_proportion);

	let stake = Perquintill::from_rational(total_staked, total_stakable);
	let falloff = Perquintill::from_percent(5);
	let adjustment = compute_inflation(stake, ideal_stake, falloff);
	let staking_inflation =
		min_annual_inflation.saturating_add(delta_annual_inflation * adjustment);

	let max_payout = period_fraction * max_annual_inflation * total_stakable;
	let staking_payout = (period_fraction * staking_inflation) * total_stakable;
	let rest = max_payout.saturating_sub(staking_payout);

	let other_issuance = total_stakable.saturating_sub(total_staked);
	if total_staked > other_issuance {
		let _cap_rest = Perquintill::from_rational(other_issuance, total_staked) * staking_payout;
		// We don't do anything with this, but if we wanted to, we could introduce a cap on the
		// treasury amount with: `rest = rest.min(cap_rest);`
	}
	(staking_payout, rest)
}

pub struct EraPayout;
impl pallet_staking::EraPayout<Balance> for EraPayout {
	fn era_payout(
		total_staked: Balance,
		total_issuance: Balance,
		era_duration_millis: u64,
	) -> (Balance, Balance) {
		// all para-ids that are not active.
		let auctioned_slots = Paras::parachains()
			.into_iter()
			// all active para-ids that do not belong to a system chain is the number
			// of parachains that we should take into account for inflation.
			.filter(|i| *i >= LOWEST_PUBLIC_ID)
			.count() as u64;

		const MAX_ANNUAL_INFLATION: Perquintill = Perquintill::from_percent(10);
		const MILLISECONDS_PER_YEAR: u64 = 1000 * 3600 * 24 * 36525 / 100;

		polkadot_era_payout(
			total_staked,
			total_issuance,
			MAX_ANNUAL_INFLATION,
			Perquintill::from_rational(era_duration_millis, MILLISECONDS_PER_YEAR),
			auctioned_slots,
		)
	}
}

impl pallet_staking::Config for Runtime {
	type Currency = Balances;
	type CurrencyBalance = Balance;
	type UnixTime = Timestamp;
	type CurrencyToVote = CurrencyToVote;
	type RewardRemainder = Treasury;
	type RuntimeEvent = RuntimeEvent;
	type Slash = Treasury;
	type Reward = ();
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	type AdminOrigin = EitherOf<EnsureRoot<Self::AccountId>, StakingAdmin>;
	type SessionInterface = Self;
	type EraPayout = EraPayout;
	type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
	type OffendingValidatorsThreshold = OffendingValidatorsThreshold;
	type NextNewSession = Session;
	type ElectionProvider = ElectionProviderMultiPhase;
	type GenesisElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type VoterList = VoterList;
	type TargetList = UseValidatorsMap<Self>;
	type NominationsQuota = pallet_staking::FixedNominationsQuota<{ MaxNominations::get() }>;
	type MaxUnlockingChunks = frame_support::traits::ConstU32<32>;
	type HistoryDepth = frame_support::traits::ConstU32<84>;
	type BenchmarkingConfig = runtime_common::StakingBenchmarkingConfig;
	type EventListeners = NominationPools;
	type WeightInfo = weights::pallet_staking::WeightInfo<Runtime>;
}

impl pallet_fast_unstake::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BatchSize = frame_support::traits::ConstU32<16>;
	type Deposit = frame_support::traits::ConstU128<{ UNITS }>;
	type ControlOrigin = EnsureRoot<AccountId>;
	type Staking = Staking;
	type MaxErasToCheckPerBlock = ConstU32<1>;
	#[cfg(feature = "runtime-benchmarks")]
	type MaxBackersPerValidator = MaxNominatorRewardedPerValidator;
	type WeightInfo = weights::pallet_fast_unstake::WeightInfo<Runtime>;
}

parameter_types! {
	// Minimum 4 CENTS/byte
	pub const BasicDeposit: Balance = deposit(1, 258);
	pub const FieldDeposit: Balance = deposit(0, 66);
	pub const SubAccountDeposit: Balance = deposit(1, 53);
	pub const MaxSubAccounts: u32 = 100;
	pub const MaxAdditionalFields: u32 = 100;
	pub const MaxRegistrars: u32 = 20;
}

impl pallet_identity::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BasicDeposit = BasicDeposit;
	type FieldDeposit = FieldDeposit;
	type SubAccountDeposit = SubAccountDeposit;
	type MaxSubAccounts = MaxSubAccounts;
	type MaxAdditionalFields = MaxAdditionalFields;
	type IdentityInformation = IdentityInfo<MaxAdditionalFields>;
	type MaxRegistrars = MaxRegistrars;
	type Slashed = Treasury;
	type ForceOrigin = EitherOf<EnsureRoot<Self::AccountId>, GeneralAdmin>;
	type RegistrarOrigin = EitherOf<EnsureRoot<Self::AccountId>, GeneralAdmin>;
	type WeightInfo = weights::pallet_identity::WeightInfo<Runtime>;
}

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = 100 * DOLLARS;
	pub const ProposalBondMaximum: Balance = 500 * DOLLARS;
	pub const SpendPeriod: BlockNumber = 24 * DAYS;
	pub const Burn: Permill = Permill::from_percent(1);
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	pub const PayoutSpendPeriod: BlockNumber = 30 * DAYS;
	// The asset's interior location for the paying account. This is the Treasury
	// pallet instance (which sits at index 19).
	pub TreasuryInteriorLocation: InteriorMultiLocation = PalletInstance(TREASURY_PALLET_ID).into();

	pub const TipCountdown: BlockNumber = 1 * DAYS;
	pub const TipFindersFee: Percent = Percent::from_percent(20);
	pub const TipReportDepositBase: Balance = 1 * DOLLARS;
	pub const DataDepositPerByte: Balance = 1 * CENTS;
	pub const MaxApprovals: u32 = 100;
	pub const MaxAuthorities: u32 = 100_000;
	pub const MaxKeys: u32 = 10_000;
	pub const MaxPeerInHeartbeats: u32 = 10_000;
	pub const RootSpendOriginMaxAmount: Balance = Balance::MAX;
	pub const CouncilSpendOriginMaxAmount: Balance = Balance::MAX;
}

impl pallet_treasury::Config for Runtime {
	type PalletId = TreasuryPalletId;
	type Currency = Balances;
	type ApproveOrigin = EitherOfDiverse<EnsureRoot<AccountId>, Treasurer>;
	type RejectOrigin = EitherOfDiverse<EnsureRoot<AccountId>, Treasurer>;
	type RuntimeEvent = RuntimeEvent;
	type OnSlash = Treasury;
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type ProposalBondMaximum = ProposalBondMaximum;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type BurnDestination = ();
	type SpendFunds = Bounties;
	type MaxApprovals = MaxApprovals;
	type WeightInfo = weights::pallet_treasury::WeightInfo<Runtime>;
	type SpendOrigin = TreasurySpender;
	type AssetKind = VersionedLocatableAsset;
	type Beneficiary = VersionedMultiLocation;
	type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
	type Paymaster = PayOverXcm<
		TreasuryInteriorLocation,
		crate::xcm_config::XcmRouter,
		crate::XcmPallet,
		ConstU32<{ 6 * HOURS }>,
		Self::Beneficiary,
		Self::AssetKind,
		LocatableAssetConverter,
		VersionedMultiLocationConverter,
	>;
	type BalanceConverter = AssetRate;
	type PayoutPeriod = PayoutSpendPeriod;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = runtime_common::impls::benchmarks::TreasuryArguments;
}

parameter_types! {
	pub const BountyDepositBase: Balance = 1 * DOLLARS;
	pub const BountyDepositPayoutDelay: BlockNumber = 8 * DAYS;
	pub const BountyUpdatePeriod: BlockNumber = 90 * DAYS;
	pub const MaximumReasonLength: u32 = 16384;
	pub const CuratorDepositMultiplier: Permill = Permill::from_percent(50);
	pub const CuratorDepositMin: Balance = 10 * DOLLARS;
	pub const CuratorDepositMax: Balance = 200 * DOLLARS;
	pub const BountyValueMinimum: Balance = 10 * DOLLARS;
}

impl pallet_bounties::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type BountyDepositBase = BountyDepositBase;
	type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
	type BountyUpdatePeriod = BountyUpdatePeriod;
	type CuratorDepositMultiplier = CuratorDepositMultiplier;
	type CuratorDepositMin = CuratorDepositMin;
	type CuratorDepositMax = CuratorDepositMax;
	type BountyValueMinimum = BountyValueMinimum;
	type ChildBountyManager = ChildBounties;
	type DataDepositPerByte = DataDepositPerByte;
	type MaximumReasonLength = MaximumReasonLength;
	type WeightInfo = weights::pallet_bounties::WeightInfo<Runtime>;
}

parameter_types! {
	pub const MaxActiveChildBountyCount: u32 = 100;
	pub const ChildBountyValueMinimum: Balance = BountyValueMinimum::get() / 10;
}

impl pallet_child_bounties::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MaxActiveChildBountyCount = MaxActiveChildBountyCount;
	type ChildBountyValueMinimum = ChildBountyValueMinimum;
	type WeightInfo = weights::pallet_child_bounties::WeightInfo<Runtime>;
}

impl pallet_offences::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
	type OnOffenceHandler = Staking;
}

impl pallet_authority_discovery::Config for Runtime {
	type MaxAuthorities = MaxAuthorities;
}

parameter_types! {
	pub NposSolutionPriority: TransactionPriority =
		Perbill::from_percent(90) * TransactionPriority::max_value();
	pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
}

impl pallet_im_online::Config for Runtime {
	type AuthorityId = ImOnlineId;
	type RuntimeEvent = RuntimeEvent;
	type ValidatorSet = Historical;
	type NextSessionRotation = Babe;
	type ReportUnresponsiveness = Offences;
	type UnsignedPriority = ImOnlineUnsignedPriority;
	type WeightInfo = weights::pallet_im_online::WeightInfo<Runtime>;
	type MaxKeys = MaxKeys;
	type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
}

parameter_types! {
	pub MaxSetIdSessionEntries: u32 = BondingDuration::get() * SessionsPerEra::get();
}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;

	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = MaxNominatorRewardedPerValidator;
	type MaxSetIdSessionEntries = MaxSetIdSessionEntries;

	type KeyOwnerProof = <Historical as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;

	type EquivocationReportSystem =
		pallet_grandpa::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

/// Submits a transaction with the node's public and signature type. Adheres to the signed extension
/// format of the chain.
impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: RuntimeCall,
		public: <Signature as Verify>::Signer,
		account: AccountId,
		nonce: <Runtime as frame_system::Config>::Nonce,
	) -> Option<(RuntimeCall, <UncheckedExtrinsic as ExtrinsicT>::SignaturePayload)> {
		use sp_runtime::traits::StaticLookup;
		// take the biggest period possible.
		let period =
			BlockHashCount::get().checked_next_power_of_two().map(|c| c / 2).unwrap_or(2) as u64;

		let current_block = System::block_number()
			.saturated_into::<u64>()
			// The `System::block_number` is initialized with `n+1`,
			// so the actual block number is `n`.
			.saturating_sub(1);
		let tip = 0;
		let extra: SignedExtra = (
			frame_system::CheckNonZeroSender::<Runtime>::new(),
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckMortality::<Runtime>::from(generic::Era::mortal(
				period,
				current_block,
			)),
			frame_system::CheckNonce::<Runtime>::from(nonce),
			frame_system::CheckWeight::<Runtime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
			claims::PrevalidateAttests::<Runtime>::new(),
		);
		let raw_payload = SignedPayload::new(call, extra)
			.map_err(|e| {
				log::warn!(target: LOG_TARGET, "Unable to create signed payload: {:?}", e);
			})
			.ok()?;
		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
		let (call, extra, _) = raw_payload.deconstruct();
		let address = <Runtime as frame_system::Config>::Lookup::unlookup(account);
		Some((call, (address, signature, extra)))
	}
}

impl frame_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
	RuntimeCall: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type OverarchingCall = RuntimeCall;
}

parameter_types! {
	// Deposit for a parathread (on-demand parachain)
	pub const ParathreadDeposit: Balance = 500 * DOLLARS;
	pub const MaxRetries: u32 = 3;
}

parameter_types! {
	pub Prefix: &'static [u8] = b"Pay DOTs to the Polkadot account:";
}

impl claims::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type VestingSchedule = Vesting;
	type Prefix = Prefix;
	/// Only Root can move a claim.
	type MoveClaimOrigin = EnsureRoot<AccountId>;
	type WeightInfo = weights::runtime_common_claims::WeightInfo<Runtime>;
}

parameter_types! {
	pub const MinVestedTransfer: Balance = 1 * DOLLARS;
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

impl pallet_vesting::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BlockNumberToBalance = ConvertInto;
	type MinVestedTransfer = MinVestedTransfer;
	type WeightInfo = weights::pallet_vesting::WeightInfo<Runtime>;
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	const MAX_VESTING_SCHEDULES: u32 = 28;
}

impl pallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = weights::pallet_utility::WeightInfo<Runtime>;
}

parameter_types! {
	// One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
	pub const DepositBase: Balance = deposit(1, 88);
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = deposit(0, 32);
	pub const MaxSignatories: u32 = 100;
}

impl pallet_multisig::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = MaxSignatories;
	type WeightInfo = weights::pallet_multisig::WeightInfo<Runtime>;
}

parameter_types! {
	// One storage item; key size 32, value size 8; .
	pub const ProxyDepositBase: Balance = deposit(1, 8);
	// Additional storage item size of 33 bytes.
	pub const ProxyDepositFactor: Balance = deposit(0, 33);
	pub const MaxProxies: u16 = 32;
	pub const AnnouncementDepositBase: Balance = deposit(1, 8);
	pub const AnnouncementDepositFactor: Balance = deposit(0, 66);
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
	MaxEncodedLen,
	scale_info::TypeInfo,
)]
pub enum ProxyType {
	Any = 0,
	NonTransfer = 1,
	Governance = 2,
	Staking = 3,
	// Skip 4 as it is now removed (was SudoBalances)
	IdentityJudgement = 5,
	CancelProxy = 6,
	Auction = 7,
	NominationPools = 8,
}

#[cfg(test)]
mod proxy_type_tests {
	use super::*;

	#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug)]
	pub enum OldProxyType {
		Any,
		NonTransfer,
		Governance,
		Staking,
		SudoBalances,
		IdentityJudgement,
	}

	#[test]
	fn proxy_type_decodes_correctly() {
		for (i, j) in vec![
			(OldProxyType::Any, ProxyType::Any),
			(OldProxyType::NonTransfer, ProxyType::NonTransfer),
			(OldProxyType::Governance, ProxyType::Governance),
			(OldProxyType::Staking, ProxyType::Staking),
			(OldProxyType::IdentityJudgement, ProxyType::IdentityJudgement),
		]
		.into_iter()
		{
			assert_eq!(i.encode(), j.encode());
		}
		assert!(ProxyType::decode(&mut &OldProxyType::SudoBalances.encode()[..]).is_err());
	}
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
				RuntimeCall::System(..) |
				RuntimeCall::Scheduler(..) |
				RuntimeCall::Babe(..) |
				RuntimeCall::Timestamp(..) |
				RuntimeCall::Indices(pallet_indices::Call::claim{..}) |
				RuntimeCall::Indices(pallet_indices::Call::free{..}) |
				RuntimeCall::Indices(pallet_indices::Call::freeze{..}) |
				// Specifically omitting Indices `transfer`, `force_transfer`
				// Specifically omitting the entire Balances pallet
				RuntimeCall::Staking(..) |
				RuntimeCall::Session(..) |
				RuntimeCall::Grandpa(..) |
				RuntimeCall::ImOnline(..) |
				RuntimeCall::Treasury(..) |
				RuntimeCall::Bounties(..) |
				RuntimeCall::ChildBounties(..) |
				RuntimeCall::ConvictionVoting(..) |
				RuntimeCall::Referenda(..) |
				RuntimeCall::Whitelist(..) |
				RuntimeCall::Claims(..) |
				RuntimeCall::Vesting(pallet_vesting::Call::vest{..}) |
				RuntimeCall::Vesting(pallet_vesting::Call::vest_other{..}) |
				// Specifically omitting Vesting `vested_transfer`, and `force_vested_transfer`
				RuntimeCall::Utility(..) |
				RuntimeCall::Identity(..) |
				RuntimeCall::Proxy(..) |
				RuntimeCall::Multisig(..) |
				RuntimeCall::Registrar(paras_registrar::Call::register {..}) |
				RuntimeCall::Registrar(paras_registrar::Call::deregister {..}) |
				// Specifically omitting Registrar `swap`
				RuntimeCall::Registrar(paras_registrar::Call::reserve {..}) |
				RuntimeCall::Crowdloan(..) |
				RuntimeCall::Slots(..) |
				RuntimeCall::Auctions(..) | // Specifically omitting the entire XCM Pallet
				RuntimeCall::VoterList(..) |
				RuntimeCall::NominationPools(..) |
				RuntimeCall::FastUnstake(..)
			),
			ProxyType::Governance => matches!(
				c,
				RuntimeCall::Treasury(..) |
					RuntimeCall::Bounties(..) |
					RuntimeCall::Utility(..) |
					RuntimeCall::ChildBounties(..) |
					RuntimeCall::ConvictionVoting(..) |
					RuntimeCall::Referenda(..) |
					RuntimeCall::Whitelist(..)
			),
			ProxyType::Staking => {
				matches!(
					c,
					RuntimeCall::Staking(..) |
						RuntimeCall::Session(..) | RuntimeCall::Utility(..) |
						RuntimeCall::FastUnstake(..) |
						RuntimeCall::VoterList(..) |
						RuntimeCall::NominationPools(..)
				)
			},
			ProxyType::NominationPools => {
				matches!(c, RuntimeCall::NominationPools(..) | RuntimeCall::Utility(..))
			},
			ProxyType::IdentityJudgement => matches!(
				c,
				RuntimeCall::Identity(pallet_identity::Call::provide_judgement { .. }) |
					RuntimeCall::Utility(..)
			),
			ProxyType::CancelProxy => {
				matches!(c, RuntimeCall::Proxy(pallet_proxy::Call::reject_announcement { .. }))
			},
			ProxyType::Auction => matches!(
				c,
				RuntimeCall::Auctions(..) |
					RuntimeCall::Crowdloan(..) |
					RuntimeCall::Registrar(..) |
					RuntimeCall::Slots(..)
			),
		}
	}
	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			(ProxyType::NonTransfer, _) => true,
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

impl parachains_origin::Config for Runtime {}

impl parachains_configuration::Config for Runtime {
	type WeightInfo = weights::runtime_parachains_configuration::WeightInfo<Runtime>;
}

impl parachains_shared::Config for Runtime {}

impl parachains_session_info::Config for Runtime {
	type ValidatorSet = Historical;
}

impl parachains_inclusion::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type DisputesHandler = ParasDisputes;
	type RewardValidators = parachains_reward_points::RewardValidatorsWithEraPoints<Runtime>;
	type MessageQueue = MessageQueue;
	type WeightInfo = weights::runtime_parachains_inclusion::WeightInfo<Runtime>;
}

parameter_types! {
	pub const ParasUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
}

impl parachains_paras::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::runtime_parachains_paras::WeightInfo<Runtime>;
	type UnsignedPriority = ParasUnsignedPriority;
	type QueueFootprinter = ParaInclusion;
	type NextSessionRotation = Babe;
	type OnNewHead = Registrar;
}

parameter_types! {
	/// Amount of weight that can be spent per block to service messages.
	///
	/// # WARNING
	///
	/// This is not a good value for para-chains since the `Scheduler` already uses up to 80% block weight.
	pub MessageQueueServiceWeight: Weight = Perbill::from_percent(20) * BlockWeights::get().max_block;
	pub const MessageQueueHeapSize: u32 = 65_536;
	pub const MessageQueueMaxStale: u32 = 8;
}

/// Message processor to handle any messages that were enqueued into the `MessageQueue` pallet.
pub struct MessageProcessor;
impl ProcessMessage for MessageProcessor {
	type Origin = AggregateMessageOrigin;

	fn process_message(
		message: &[u8],
		origin: Self::Origin,
		meter: &mut WeightMeter,
		id: &mut [u8; 32],
	) -> Result<bool, ProcessMessageError> {
		let para = match origin {
			AggregateMessageOrigin::Ump(UmpQueueId::Para(para)) => para,
		};
		xcm_builder::ProcessXcmMessage::<
			Junction,
			xcm_executor::XcmExecutor<xcm_config::XcmConfig>,
			RuntimeCall,
		>::process_message(message, Junction::Parachain(para.into()), meter, id)
	}
}

impl pallet_message_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Size = u32;
	type HeapSize = MessageQueueHeapSize;
	type MaxStale = MessageQueueMaxStale;
	type ServiceWeight = MessageQueueServiceWeight;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type MessageProcessor = MessageProcessor;
	#[cfg(feature = "runtime-benchmarks")]
	type MessageProcessor =
		pallet_message_queue::mock_helpers::NoopMessageProcessor<AggregateMessageOrigin>;
	type QueueChangeHandler = ParaInclusion;
	type QueuePausedQuery = ();
	type WeightInfo = weights::pallet_message_queue::WeightInfo<Runtime>;
}

impl parachains_dmp::Config for Runtime {}

impl parachains_hrmp::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type ChannelManager = EitherOf<EnsureRoot<Self::AccountId>, GeneralAdmin>;
	type Currency = Balances;
	type WeightInfo = weights::runtime_parachains_hrmp::WeightInfo<Self>;
}

impl parachains_paras_inherent::Config for Runtime {
	type WeightInfo = weights::runtime_parachains_paras_inherent::WeightInfo<Runtime>;
}

impl parachains_scheduler::Config for Runtime {
	type AssignmentProvider = ParaAssignmentProvider;
}

impl parachains_assigner_parachains::Config for Runtime {}

impl parachains_initializer::Config for Runtime {
	type Randomness = pallet_babe::RandomnessFromOneEpochAgo<Runtime>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type WeightInfo = weights::runtime_parachains_initializer::WeightInfo<Runtime>;
}

impl parachains_disputes::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RewardValidators = parachains_reward_points::RewardValidatorsWithEraPoints<Runtime>;
	type SlashingHandler = parachains_slashing::SlashValidatorsForDisputes<ParasSlashing>;
	type WeightInfo = weights::runtime_parachains_disputes::WeightInfo<Runtime>;
}

impl parachains_slashing::Config for Runtime {
	type KeyOwnerProofSystem = Historical;
	type KeyOwnerProof =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, ValidatorId)>>::Proof;
	type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
		KeyTypeId,
		ValidatorId,
	)>>::IdentificationTuple;
	type HandleReports = parachains_slashing::SlashingReportHandler<
		Self::KeyOwnerIdentification,
		Offences,
		ReportLongevity,
	>;
	type WeightInfo = weights::runtime_parachains_disputes_slashing::WeightInfo<Runtime>;
	type BenchmarkingConfig = parachains_slashing::BenchConfig<1000>;
}

parameter_types! {
	// Mostly arbitrary deposit price, but should provide an adequate incentive not to spam reserve
	// `ParaId`s.
	pub const ParaDeposit: Balance = 100 * DOLLARS;
	pub const ParaDataByteDeposit: Balance = deposit(0, 1);
}

impl paras_registrar::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type OnSwap = (Crowdloan, Slots);
	type ParaDeposit = ParaDeposit;
	type DataDepositPerByte = ParaDataByteDeposit;
	type WeightInfo = weights::runtime_common_paras_registrar::WeightInfo<Runtime>;
}

parameter_types! {
	// 12 weeks = 3 months per lease period -> 8 lease periods ~ 2 years
	pub LeasePeriod: BlockNumber = prod_or_fast!(12 * WEEKS, 12 * WEEKS, "DOT_LEASE_PERIOD");
	// Polkadot Genesis was on May 26, 2020.
	// Target Parachain Onboarding Date: Dec 15, 2021.
	// Difference is 568 days.
	// We want a lease period to start on the target onboarding date.
	// 568 % (12 * 7) = 64 day offset
	pub LeaseOffset: BlockNumber = prod_or_fast!(64 * DAYS, 0, "DOT_LEASE_OFFSET");
}

impl slots::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type Registrar = Registrar;
	type LeasePeriod = LeasePeriod;
	type LeaseOffset = LeaseOffset;
	type ForceOrigin = EitherOf<EnsureRoot<Self::AccountId>, LeaseAdmin>;
	type WeightInfo = weights::runtime_common_slots::WeightInfo<Runtime>;
}

parameter_types! {
	pub const CrowdloanId: PalletId = PalletId(*b"py/cfund");
	// Accounts for 10_000 contributions, each using 48 bytes (16 bytes for balance, and 32 bytes
	// for a memo).
	pub const SubmissionDeposit: Balance = deposit(1, 480_000);
	// The minimum crowdloan contribution.
	pub const MinContribution: Balance = 5 * DOLLARS;
	pub const RemoveKeysLimit: u32 = 1000;
	// Allow 32 bytes for an additional memo to a crowdloan.
	pub const MaxMemoLength: u8 = 32;
}

impl crowdloan::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = CrowdloanId;
	type SubmissionDeposit = SubmissionDeposit;
	type MinContribution = MinContribution;
	type RemoveKeysLimit = RemoveKeysLimit;
	type Registrar = Registrar;
	type Auctioneer = Auctions;
	type MaxMemoLength = MaxMemoLength;
	type WeightInfo = weights::runtime_common_crowdloan::WeightInfo<Runtime>;
}

parameter_types! {
	// The average auction is 7 days long, so this will be 70% for ending period.
	// 5 Days = 72000 Blocks @ 6 sec per block
	pub const EndingPeriod: BlockNumber = 5 * DAYS;
	// ~ 1000 samples per day -> ~ 20 blocks per sample -> 2 minute samples
	pub const SampleLength: BlockNumber = 2 * MINUTES;
}

impl auctions::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Leaser = Slots;
	type Registrar = Registrar;
	type EndingPeriod = EndingPeriod;
	type SampleLength = SampleLength;
	type Randomness = pallet_babe::RandomnessFromOneEpochAgo<Runtime>;
	type InitiateOrigin = EitherOf<EnsureRoot<Self::AccountId>, AuctionAdmin>;
	type WeightInfo = weights::runtime_common_auctions::WeightInfo<Runtime>;
}

parameter_types! {
	pub const PoolsPalletId: PalletId = PalletId(*b"py/nopls");
	// Allow pools that got slashed up to 90% to remain operational.
	pub const MaxPointsToBalance: u8 = 10;
}

impl pallet_nomination_pools::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type RewardCounter = FixedU128;
	type BalanceToU256 = runtime_common::BalanceToU256;
	type U256ToBalance = runtime_common::U256ToBalance;
	type Staking = Staking;
	type PostUnbondingPoolsWindow = frame_support::traits::ConstU32<4>;
	type MaxMetadataLen = frame_support::traits::ConstU32<256>;
	// we use the same number of allowed unlocking chunks as with staking.
	type MaxUnbonding = <Self as pallet_staking::Config>::MaxUnlockingChunks;
	type PalletId = PoolsPalletId;
	type MaxPointsToBalance = MaxPointsToBalance;
	type WeightInfo = weights::pallet_nomination_pools::WeightInfo<Self>;
}

pub struct InitiateNominationPools;
impl frame_support::traits::OnRuntimeUpgrade for InitiateNominationPools {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		// we use one as an indicator if this has already been set.
		if pallet_nomination_pools::MaxPools::<Runtime>::get().is_none() {
			// 5 DOT to join a pool.
			pallet_nomination_pools::MinJoinBond::<Runtime>::put(5 * UNITS);
			// 100 DOT to create a pool.
			pallet_nomination_pools::MinCreateBond::<Runtime>::put(100 * UNITS);

			// Initialize with limits for now.
			pallet_nomination_pools::MaxPools::<Runtime>::put(0);
			pallet_nomination_pools::MaxPoolMembersPerPool::<Runtime>::put(0);
			pallet_nomination_pools::MaxPoolMembers::<Runtime>::put(0);

			log::info!(target: LOG_TARGET, "pools config initiated 🎉");
			<Runtime as frame_system::Config>::DbWeight::get().reads_writes(1, 5)
		} else {
			log::info!(target: LOG_TARGET, "pools config already initiated 😏");
			<Runtime as frame_system::Config>::DbWeight::get().reads(1)
		}
	}
}

impl pallet_asset_rate::Config for Runtime {
	type WeightInfo = weights::pallet_asset_rate::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type CreateOrigin = EitherOfDiverse<EnsureRoot<AccountId>, Treasurer>;
	type RemoveOrigin = EitherOfDiverse<EnsureRoot<AccountId>, Treasurer>;
	type UpdateOrigin = EitherOfDiverse<EnsureRoot<AccountId>, Treasurer>;
	type Currency = Balances;
	type AssetKind = <Runtime as pallet_treasury::Config>::AssetKind;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = runtime_common::impls::benchmarks::AssetRateArguments;
}

construct_runtime! {
	pub enum Runtime
	{
		// Basic stuff; balances is uncallable initially.
		System: frame_system::{Pallet, Call, Storage, Config<T>, Event<T>} = 0,
		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>} = 1,
		Preimage: pallet_preimage::{Pallet, Call, Storage, Event<T>, HoldReason} = 10,

		// Babe must be before session.
		Babe: pallet_babe::{Pallet, Call, Storage, Config<T>, ValidateUnsigned} = 2,

		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 3,
		Indices: pallet_indices::{Pallet, Call, Storage, Config<T>, Event<T>} = 4,
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 5,
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage, Event<T>} = 32,

		// Consensus support.
		// Authorship must be before session in order to note author in the correct session and era
		// for im-online and staking.
		Authorship: pallet_authorship::{Pallet, Storage} = 6,
		Staking: pallet_staking::{Pallet, Call, Storage, Config<T>, Event<T>} = 7,
		Offences: pallet_offences::{Pallet, Storage, Event} = 8,
		Historical: session_historical::{Pallet} = 33,

		// BEEFY Bridges support.
		Beefy: pallet_beefy::{Pallet, Call, Storage, Config<T>, ValidateUnsigned} = 200,
		// MMR leaf construction must be before session in order to have leaf contents
		// refer to block<N-1> consistently. see substrate issue #11797 for details.
		Mmr: pallet_mmr::{Pallet, Storage} = 201,
		BeefyMmrLeaf: pallet_beefy_mmr::{Pallet, Storage} = 202,

		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>} = 9,
		Grandpa: pallet_grandpa::{Pallet, Call, Storage, Config<T>, Event, ValidateUnsigned} = 11,
		ImOnline: pallet_im_online::{Pallet, Call, Storage, Event<T>, ValidateUnsigned, Config<T>} = 12,
		AuthorityDiscovery: pallet_authority_discovery::{Pallet, Config<T>} = 13,

		// OpenGov stuff.
		Treasury: pallet_treasury::{Pallet, Call, Storage, Config<T>, Event<T>} = 19,
		ConvictionVoting: pallet_conviction_voting::{Pallet, Call, Storage, Event<T>} = 20,
		Referenda: pallet_referenda::{Pallet, Call, Storage, Event<T>} = 21,
		Origins: pallet_custom_origins::{Origin} = 22,
		Whitelist: pallet_whitelist::{Pallet, Call, Storage, Event<T>} = 23,

		// Claims. Usable initially.
		Claims: claims::{Pallet, Call, Storage, Event<T>, Config<T>, ValidateUnsigned} = 24,
		// Vesting. Usable initially, but removed once all vesting is finished.
		Vesting: pallet_vesting::{Pallet, Call, Storage, Event<T>, Config<T>} = 25,
		// Cunning utilities. Usable initially.
		Utility: pallet_utility::{Pallet, Call, Event} = 26,

		// Identity. Late addition.
		Identity: pallet_identity::{Pallet, Call, Storage, Event<T>} = 28,

		// Proxy module. Late addition.
		Proxy: pallet_proxy::{Pallet, Call, Storage, Event<T>} = 29,

		// Multisig dispatch. Late addition.
		Multisig: pallet_multisig::{Pallet, Call, Storage, Event<T>} = 30,

		// Bounties modules.
		Bounties: pallet_bounties::{Pallet, Call, Storage, Event<T>} = 34,
		ChildBounties: pallet_child_bounties = 38,

		// Election pallet. Only works with staking, but placed here to maintain indices.
		ElectionProviderMultiPhase: pallet_election_provider_multi_phase::{Pallet, Call, Storage, Event<T>, ValidateUnsigned} = 36,

		// Provides a semi-sorted list of nominators for staking.
		VoterList: pallet_bags_list::<Instance1>::{Pallet, Call, Storage, Event<T>} = 37,

		// Nomination pools: extension to staking.
		NominationPools: pallet_nomination_pools::{Pallet, Call, Storage, Event<T>, Config<T>, FreezeReason} = 39,

		// Fast unstake pallet: extension to staking.
		FastUnstake: pallet_fast_unstake = 40,

		// Parachains pallets. Start indices at 50 to leave room.
		ParachainsOrigin: parachains_origin::{Pallet, Origin} = 50,
		Configuration: parachains_configuration::{Pallet, Call, Storage, Config<T>} = 51,
		ParasShared: parachains_shared::{Pallet, Call, Storage} = 52,
		ParaInclusion: parachains_inclusion::{Pallet, Call, Storage, Event<T>} = 53,
		ParaInherent: parachains_paras_inherent::{Pallet, Call, Storage, Inherent} = 54,
		ParaScheduler: parachains_scheduler::{Pallet, Storage} = 55,
		Paras: parachains_paras::{Pallet, Call, Storage, Event, Config<T>, ValidateUnsigned} = 56,
		Initializer: parachains_initializer::{Pallet, Call, Storage} = 57,
		Dmp: parachains_dmp::{Pallet, Storage} = 58,
		// Ump 59
		Hrmp: parachains_hrmp::{Pallet, Call, Storage, Event<T>, Config<T>} = 60,
		ParaSessionInfo: parachains_session_info::{Pallet, Storage} = 61,
		ParasDisputes: parachains_disputes::{Pallet, Call, Storage, Event<T>} = 62,
		ParasSlashing: parachains_slashing::{Pallet, Call, Storage, ValidateUnsigned} = 63,
		ParaAssignmentProvider: parachains_assigner_parachains::{Pallet} = 64,

		// Parachain Onboarding Pallets. Start indices at 70 to leave room.
		Registrar: paras_registrar::{Pallet, Call, Storage, Event<T>} = 70,
		Slots: slots::{Pallet, Call, Storage, Event<T>} = 71,
		Auctions: auctions::{Pallet, Call, Storage, Event<T>} = 72,
		Crowdloan: crowdloan::{Pallet, Call, Storage, Event<T>} = 73,

		// Pallet for sending XCM.
		XcmPallet: pallet_xcm::{Pallet, Call, Storage, Event<T>, Origin, Config<T>} = 99,

		// Generalized message queue
		MessageQueue: pallet_message_queue::{Pallet, Call, Storage, Event<T>} = 100,

		// Asset rate.
		AssetRate: pallet_asset_rate::{Pallet, Call, Storage, Event<T>} = 101,
	}
}

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// `BlockId` type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The `SignedExtension` to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckMortality<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
	claims::PrevalidateAttests<Runtime>,
);

pub struct NominationPoolsMigrationV4OldPallet;
impl Get<Perbill> for NominationPoolsMigrationV4OldPallet {
	fn get() -> Perbill {
		Perbill::zero()
	}
}

/// All migrations that will run on the next runtime upgrade.
///
/// This contains the combined migrations of the last 10 releases. It allows to skip runtime
/// upgrades in case governance decides to do so. THE ORDER IS IMPORTANT.
pub type Migrations = migrations::Unreleased;

/// The runtime migrations per release.
#[allow(deprecated, missing_docs)]
pub mod migrations {
	use super::*;

	/// Upgrade Session keys to include BEEFY key.
	/// When this is removed, should also remove `OldSessionKeys`.
	pub struct UpgradeSessionKeys;
	impl frame_support::traits::OnRuntimeUpgrade for UpgradeSessionKeys {
		fn on_runtime_upgrade() -> Weight {
			Session::upgrade_keys::<OldSessionKeys, _>(transform_session_keys);
			Perbill::from_percent(50) * BlockWeights::get().max_block
		}
	}

	/// Unreleased migrations. Add new ones here:
	pub type Unreleased = (
		// Upgrade SessionKeys to include BEEFY key
		UpgradeSessionKeys,
		pallet_nomination_pools::migration::versioned_migrations::V5toV6<Runtime>,
		pallet_nomination_pools::migration::versioned_migrations::V6ToV7<Runtime>,
	);
}

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
	Migrations,
>;

/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	frame_benchmarking::define_benchmarks!(
		// Polkadot
		// NOTE: Make sure to prefix these with `runtime_common::` so
		// the that path resolves correctly in the generated file.
		[runtime_common::auctions, Auctions]
		[runtime_common::claims, Claims]
		[runtime_common::crowdloan, Crowdloan]
		[runtime_common::slots, Slots]
		[runtime_common::paras_registrar, Registrar]
		[runtime_parachains::configuration, Configuration]
		[runtime_parachains::disputes, ParasDisputes]
		[runtime_parachains::disputes::slashing, ParasSlashing]
		[runtime_parachains::hrmp, Hrmp]
		[runtime_parachains::inclusion, ParaInclusion]
		[runtime_parachains::initializer, Initializer]
		[runtime_parachains::paras, Paras]
		[runtime_parachains::paras_inherent, ParaInherent]
		// Substrate
		[pallet_bags_list, VoterList]
		[pallet_balances, Balances]
		[frame_benchmarking::baseline, Baseline::<Runtime>]
		[pallet_bounties, Bounties]
		[pallet_child_bounties, ChildBounties]
		[pallet_election_provider_multi_phase, ElectionProviderMultiPhase]
		[frame_election_provider_support, ElectionProviderBench::<Runtime>]
		[pallet_fast_unstake, FastUnstake]
		[pallet_identity, Identity]
		[pallet_im_online, ImOnline]
		[pallet_indices, Indices]
		[pallet_message_queue, MessageQueue]
		[pallet_multisig, Multisig]
		[pallet_nomination_pools, NominationPoolsBench::<Runtime>]
		[pallet_offences, OffencesBench::<Runtime>]
		[pallet_preimage, Preimage]
		[pallet_proxy, Proxy]
		[pallet_scheduler, Scheduler]
		[pallet_session, SessionBench::<Runtime>]
		[pallet_staking, Staking]
		[frame_system, SystemBench::<Runtime>]
		[pallet_timestamp, Timestamp]
		[pallet_treasury, Treasury]
		[pallet_utility, Utility]
		[pallet_vesting, Vesting]
		[pallet_conviction_voting, ConvictionVoting]
		[pallet_referenda, Referenda]
		[pallet_whitelist, Whitelist]
		[pallet_asset_rate, AssetRate]
		// XCM
		[pallet_xcm, XcmPallet]
		[pallet_xcm_benchmarks::fungible, pallet_xcm_benchmarks::fungible::Pallet::<Runtime>]
		[pallet_xcm_benchmarks::generic, pallet_xcm_benchmarks::generic::Pallet::<Runtime>]
	);
}

sp_api::impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
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

	impl block_builder_api::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: inherents::InherentData,
		) -> inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl pallet_nomination_pools_runtime_api::NominationPoolsApi<
		Block,
		AccountId,
		Balance,
	> for Runtime {
		fn pending_rewards(member: AccountId) -> Balance {
			NominationPools::api_pending_rewards(member).unwrap_or_default()
		}

		fn points_to_balance(pool_id: pallet_nomination_pools::PoolId, points: Balance) -> Balance {
			NominationPools::api_points_to_balance(pool_id, points)
		}

		fn balance_to_points(pool_id: pallet_nomination_pools::PoolId, new_funds: Balance) -> Balance {
			NominationPools::api_balance_to_points(pool_id, new_funds)
		}
	}

	impl pallet_staking_runtime_api::StakingApi<Block, Balance> for Runtime {
		fn nominations_quota(balance: Balance) -> u32 {
			Staking::api_nominations_quota(balance)
		}
	}

	impl tx_pool_api::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl offchain_primitives::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl primitives::runtime_api::ParachainHost<Block, Hash, BlockNumber> for Runtime {
		fn validators() -> Vec<ValidatorId> {
			parachains_runtime_api_impl::validators::<Runtime>()
		}

		fn validator_groups() -> (Vec<Vec<ValidatorIndex>>, GroupRotationInfo<BlockNumber>) {
			parachains_runtime_api_impl::validator_groups::<Runtime>()
		}

		fn availability_cores() -> Vec<CoreState<Hash, BlockNumber>> {
			parachains_runtime_api_impl::availability_cores::<Runtime>()
		}

		fn persisted_validation_data(para_id: ParaId, assumption: OccupiedCoreAssumption)
			-> Option<PersistedValidationData<Hash, BlockNumber>> {
			parachains_runtime_api_impl::persisted_validation_data::<Runtime>(para_id, assumption)
		}

		fn assumed_validation_data(
			para_id: ParaId,
			expected_persisted_validation_data_hash: Hash,
		) -> Option<(PersistedValidationData<Hash, BlockNumber>, ValidationCodeHash)> {
			parachains_runtime_api_impl::assumed_validation_data::<Runtime>(
				para_id,
				expected_persisted_validation_data_hash,
			)
		}

		fn check_validation_outputs(
			para_id: ParaId,
			outputs: primitives::CandidateCommitments,
		) -> bool {
			parachains_runtime_api_impl::check_validation_outputs::<Runtime>(para_id, outputs)
		}

		fn session_index_for_child() -> SessionIndex {
			parachains_runtime_api_impl::session_index_for_child::<Runtime>()
		}

		fn validation_code(para_id: ParaId, assumption: OccupiedCoreAssumption)
			-> Option<ValidationCode> {
			parachains_runtime_api_impl::validation_code::<Runtime>(para_id, assumption)
		}

		fn candidate_pending_availability(para_id: ParaId) -> Option<CommittedCandidateReceipt<Hash>> {
			parachains_runtime_api_impl::candidate_pending_availability::<Runtime>(para_id)
		}

		fn candidate_events() -> Vec<CandidateEvent<Hash>> {
			parachains_runtime_api_impl::candidate_events::<Runtime, _>(|ev| {
				match ev {
					RuntimeEvent::ParaInclusion(ev) => {
						Some(ev)
					}
					_ => None,
				}
			})
		}

		fn session_info(index: SessionIndex) -> Option<SessionInfo> {
			parachains_runtime_api_impl::session_info::<Runtime>(index)
		}

		fn session_executor_params(session_index: SessionIndex) -> Option<ExecutorParams> {
			parachains_runtime_api_impl::session_executor_params::<Runtime>(session_index)
		}

		fn dmq_contents(recipient: ParaId) -> Vec<InboundDownwardMessage<BlockNumber>> {
			parachains_runtime_api_impl::dmq_contents::<Runtime>(recipient)
		}

		fn inbound_hrmp_channels_contents(
			recipient: ParaId
		) -> BTreeMap<ParaId, Vec<InboundHrmpMessage<BlockNumber>>> {
			parachains_runtime_api_impl::inbound_hrmp_channels_contents::<Runtime>(recipient)
		}

		fn validation_code_by_hash(hash: ValidationCodeHash) -> Option<ValidationCode> {
			parachains_runtime_api_impl::validation_code_by_hash::<Runtime>(hash)
		}

		fn on_chain_votes() -> Option<ScrapedOnChainVotes<Hash>> {
			parachains_runtime_api_impl::on_chain_votes::<Runtime>()
		}

		fn submit_pvf_check_statement(
			stmt: primitives::PvfCheckStatement,
			signature: primitives::ValidatorSignature,
		) {
			parachains_runtime_api_impl::submit_pvf_check_statement::<Runtime>(stmt, signature)
		}

		fn pvfs_require_precheck() -> Vec<ValidationCodeHash> {
			parachains_runtime_api_impl::pvfs_require_precheck::<Runtime>()
		}

		fn validation_code_hash(para_id: ParaId, assumption: OccupiedCoreAssumption)
			-> Option<ValidationCodeHash>
		{
			parachains_runtime_api_impl::validation_code_hash::<Runtime>(para_id, assumption)
		}

		fn disputes() -> Vec<(SessionIndex, CandidateHash, DisputeState<BlockNumber>)> {
			parachains_runtime_api_impl::get_session_disputes::<Runtime>()
		}

		fn unapplied_slashes(
		) -> Vec<(SessionIndex, CandidateHash, slashing::PendingSlashes)> {
			parachains_runtime_api_impl::unapplied_slashes::<Runtime>()
		}

		fn key_ownership_proof(
			validator_id: ValidatorId,
		) -> Option<slashing::OpaqueKeyOwnershipProof> {
			use parity_scale_codec::Encode;

			Historical::prove((PARACHAIN_KEY_TYPE_ID, validator_id))
				.map(|p| p.encode())
				.map(slashing::OpaqueKeyOwnershipProof::new)
		}

		fn submit_report_dispute_lost(
			dispute_proof: slashing::DisputeProof,
			key_ownership_proof: slashing::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			parachains_runtime_api_impl::submit_unsigned_slashing_report::<Runtime>(
				dispute_proof,
				key_ownership_proof,
			)
		}
	}

	impl beefy_primitives::BeefyApi<Block, BeefyId> for Runtime {
		fn beefy_genesis() -> Option<BlockNumber> {
			Beefy::genesis_block()
		}

		fn validator_set() -> Option<beefy_primitives::ValidatorSet<BeefyId>> {
			Beefy::validator_set()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: beefy_primitives::EquivocationProof<
				BlockNumber,
				BeefyId,
				BeefySignature,
			>,
			key_owner_proof: beefy_primitives::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Beefy::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}

		fn generate_key_ownership_proof(
			_set_id: beefy_primitives::ValidatorSetId,
			authority_id: BeefyId,
		) -> Option<beefy_primitives::OpaqueKeyOwnershipProof> {
			use parity_scale_codec::Encode;

			Historical::prove((beefy_primitives::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(beefy_primitives::OpaqueKeyOwnershipProof::new)
		}
	}

	impl mmr::MmrApi<Block, Hash, BlockNumber> for Runtime {
		fn mmr_root() -> Result<mmr::Hash, mmr::Error> {
			Ok(Mmr::mmr_root())
		}

		fn mmr_leaf_count() -> Result<mmr::LeafIndex, mmr::Error> {
			Ok(Mmr::mmr_leaves())
		}

		fn generate_proof(
			block_numbers: Vec<BlockNumber>,
			best_known_block_number: Option<BlockNumber>,
		) -> Result<(Vec<mmr::EncodableOpaqueLeaf>, mmr::Proof<mmr::Hash>), mmr::Error> {
			Mmr::generate_proof(block_numbers, best_known_block_number).map(
				|(leaves, proof)| {
					(
						leaves
							.into_iter()
							.map(|leaf| mmr::EncodableOpaqueLeaf::from_leaf(&leaf))
							.collect(),
						proof,
					)
				},
			)
		}

		fn verify_proof(leaves: Vec<mmr::EncodableOpaqueLeaf>, proof: mmr::Proof<mmr::Hash>)
			-> Result<(), mmr::Error>
		{
			let leaves = leaves.into_iter().map(|leaf|
				leaf.into_opaque_leaf()
				.try_decode()
				.ok_or(mmr::Error::Verify)).collect::<Result<Vec<mmr::Leaf>, mmr::Error>>()?;
			Mmr::verify_leaves(leaves, proof)
		}

		fn verify_proof_stateless(
			root: mmr::Hash,
			leaves: Vec<mmr::EncodableOpaqueLeaf>,
			proof: mmr::Proof<mmr::Hash>
		) -> Result<(), mmr::Error> {
			let nodes = leaves.into_iter().map(|leaf|mmr::DataOrHash::Data(leaf.into_opaque_leaf())).collect();
			pallet_mmr::verify_leaves_proof::<mmr::Hashing, _>(root, nodes, proof)
		}
	}

	impl pallet_beefy_mmr::BeefyMmrApi<Block, Hash> for RuntimeApi {
		fn authority_set_proof() -> beefy_primitives::mmr::BeefyAuthoritySet<Hash> {
			BeefyMmrLeaf::authority_set_proof()
		}

		fn next_authority_set_proof() -> beefy_primitives::mmr::BeefyNextAuthoritySet<Hash> {
			BeefyMmrLeaf::next_authority_set_proof()
		}
	}

	impl fg_primitives::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> Vec<(GrandpaId, u64)> {
			Grandpa::grandpa_authorities()
		}

		fn current_set_id() -> fg_primitives::SetId {
			Grandpa::current_set_id()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: fg_primitives::EquivocationProof<
				<Block as BlockT>::Hash,
				sp_runtime::traits::NumberFor<Block>,
			>,
			key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Grandpa::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}

		fn generate_key_ownership_proof(
			_set_id: fg_primitives::SetId,
			authority_id: fg_primitives::AuthorityId,
		) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
			use parity_scale_codec::Encode;

			Historical::prove((fg_primitives::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(fg_primitives::OpaqueKeyOwnershipProof::new)
		}
	}

	impl babe_primitives::BabeApi<Block> for Runtime {
		fn configuration() -> babe_primitives::BabeConfiguration {
			let epoch_config = Babe::epoch_config().unwrap_or(BABE_GENESIS_EPOCH_CONFIG);
			babe_primitives::BabeConfiguration {
				slot_duration: Babe::slot_duration(),
				epoch_length: EpochDuration::get(),
				c: epoch_config.c,
				authorities: Babe::authorities().to_vec(),
				randomness: Babe::randomness(),
				allowed_slots: epoch_config.allowed_slots,
			}
		}

		fn current_epoch_start() -> babe_primitives::Slot {
			Babe::current_epoch_start()
		}

		fn current_epoch() -> babe_primitives::Epoch {
			Babe::current_epoch()
		}

		fn next_epoch() -> babe_primitives::Epoch {
			Babe::next_epoch()
		}

		fn generate_key_ownership_proof(
			_slot: babe_primitives::Slot,
			authority_id: babe_primitives::AuthorityId,
		) -> Option<babe_primitives::OpaqueKeyOwnershipProof> {
			use parity_scale_codec::Encode;

			Historical::prove((babe_primitives::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(babe_primitives::OpaqueKeyOwnershipProof::new)
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: babe_primitives::EquivocationProof<<Block as BlockT>::Header>,
			key_owner_proof: babe_primitives::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Babe::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}
	}

	impl authority_discovery_primitives::AuthorityDiscoveryApi<Block> for Runtime {
		fn authorities() -> Vec<AuthorityDiscoveryId> {
			parachains_runtime_api_impl::relevant_authority_ids::<Runtime>()
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, sp_core::crypto::KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
		Block,
		Balance,
	> for Runtime {
		fn query_info(uxt: <Block as BlockT>::Extrinsic, len: u32) -> RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(uxt: <Block as BlockT>::Extrinsic, len: u32) -> FeeDetails<Balance> {
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
		fn query_call_info(call: RuntimeCall, len: u32) -> RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(call: RuntimeCall, len: u32) -> FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
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
			log::info!(target: LOG_TARGET, "try-runtime::on_runtime_upgrade polkadot.");
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, BlockWeights::get().max_block)
		}

		fn execute_block(
			block: Block,
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
			use frame_benchmarking::{Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;

			use pallet_session_benchmarking::Pallet as SessionBench;
			use pallet_offences_benchmarking::Pallet as OffencesBench;
			use pallet_election_provider_support_benchmarking::Pallet as ElectionProviderBench;
			use pallet_nomination_pools_benchmarking::Pallet as NominationPoolsBench;
			use frame_system_benchmarking::Pallet as SystemBench;
			use frame_benchmarking::baseline::Pallet as Baseline;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();
			return (list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<
			Vec<frame_benchmarking::BenchmarkBatch>,
			sp_runtime::RuntimeString,
		> {
			use frame_support::traits::WhitelistedStorageKeys;
			use frame_benchmarking::{Benchmarking, BenchmarkBatch, BenchmarkError};
			use sp_storage::TrackedStorageKey;
			// Trying to add benchmarks directly to some pallets caused cyclic dependency issues.
			// To get around that, we separated the benchmarks into its own crate.
			use pallet_session_benchmarking::Pallet as SessionBench;
			use pallet_offences_benchmarking::Pallet as OffencesBench;
			use pallet_election_provider_support_benchmarking::Pallet as ElectionProviderBench;
			use pallet_nomination_pools_benchmarking::Pallet as NominationPoolsBench;
			use frame_system_benchmarking::Pallet as SystemBench;
			use frame_benchmarking::baseline::Pallet as Baseline;
			use xcm::latest::prelude::*;
			use xcm_config::{XcmConfig, AssetHubLocation, TokenLocation, LocalCheckAccount, SovereignAccountOf};

			impl pallet_session_benchmarking::Config for Runtime {}
			impl pallet_offences_benchmarking::Config for Runtime {}
			impl pallet_election_provider_support_benchmarking::Config for Runtime {}
			impl frame_system_benchmarking::Config for Runtime {}
			impl frame_benchmarking::baseline::Config for Runtime {}
			impl pallet_nomination_pools_benchmarking::Config for Runtime {}
			impl runtime_parachains::disputes::slashing::benchmarking::Config for Runtime {}

			let mut whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();
			let treasury_key = frame_system::Account::<Runtime>::hashed_key_for(Treasury::account_id());
			whitelist.push(treasury_key.to_vec().into());

			parameter_types! {
				pub ExistentialDepositMultiAsset: Option<MultiAsset> = Some((
					TokenLocation::get(),
					ExistentialDeposit::get()
				).into());
				pub ToParachain: ParaId = polkadot_runtime_constants::system_parachain::ASSET_HUB_ID.into();
			}

			impl pallet_xcm_benchmarks::Config for Runtime {
				type XcmConfig = XcmConfig;
				type AccountIdConverter = SovereignAccountOf;
				type DeliveryHelper = runtime_common::xcm_sender::ToParachainDeliveryHelper<
					XcmConfig,
					ExistentialDepositMultiAsset,
					xcm_config::PriceForChildParachainDelivery,
					ToParachain,
					(),
				>;
				fn valid_destination() -> Result<MultiLocation, BenchmarkError> {
					Ok(AssetHubLocation::get())
				}
				fn worst_case_holding(_depositable_count: u32) -> MultiAssets {
					// Polkadot only knows about DOT
					vec![MultiAsset { id: Concrete(TokenLocation::get()), fun: Fungible(1_000_000 * UNITS) }].into()
				}
			}

			parameter_types! {
				pub const TrustedTeleporter: Option<(MultiLocation, MultiAsset)> = Some((
					AssetHubLocation::get(),
					MultiAsset { id: Concrete(TokenLocation::get()), fun: Fungible(1 * UNITS) }
				));
				pub const TrustedReserve: Option<(MultiLocation, MultiAsset)> = None;
			}

			impl pallet_xcm_benchmarks::fungible::Config for Runtime {
				type TransactAsset = Balances;

				type CheckedAccount = LocalCheckAccount;
				type TrustedTeleporter = TrustedTeleporter;
				type TrustedReserve = TrustedReserve;

				fn get_multi_asset() -> MultiAsset {
					MultiAsset {
						id: Concrete(TokenLocation::get()),
						fun: Fungible(1 * UNITS)
					}
				}
			}

			impl pallet_xcm_benchmarks::generic::Config for Runtime {
				type TransactAsset = Balances;
				type RuntimeCall = RuntimeCall;

				fn worst_case_response() -> (u64, Response) {
					(0u64, Response::Version(Default::default()))
				}

				fn worst_case_asset_exchange() -> Result<(MultiAssets, MultiAssets), BenchmarkError> {
					// Polkadot doesn't support asset exchanges
					Err(BenchmarkError::Skip)
				}

				fn universal_alias() -> Result<(MultiLocation, Junction), BenchmarkError> {
					// The XCM executor of Polkadot doesn't have a configured `UniversalAliases`
					Err(BenchmarkError::Skip)
				}

				fn transact_origin_and_runtime_call() -> Result<(MultiLocation, RuntimeCall), BenchmarkError> {
					Ok((AssetHubLocation::get(), frame_system::Call::remark_with_event { remark: vec![] }.into()))
				}

				fn subscribe_origin() -> Result<MultiLocation, BenchmarkError> {
					Ok(AssetHubLocation::get())
				}

				fn claimable_asset() -> Result<(MultiLocation, MultiLocation, MultiAssets), BenchmarkError> {
					let origin = AssetHubLocation::get();
					let assets: MultiAssets = (Concrete(TokenLocation::get()), 1_000 * UNITS).into();
					let ticket = MultiLocation { parents: 0, interior: Here };
					Ok((origin, ticket, assets))
				}

				fn unlockable_asset() -> Result<(MultiLocation, MultiLocation, MultiAsset), BenchmarkError> {
					// Polkadot doesn't support asset locking
					Err(BenchmarkError::Skip)
				}

				fn export_message_origin_and_destination(
				) -> Result<(MultiLocation, NetworkId, InteriorMultiLocation), BenchmarkError> {
					// Polkadot doesn't support exporting messages
					Err(BenchmarkError::Skip)
				}

				fn alias_origin() -> Result<(MultiLocation, MultiLocation), BenchmarkError> {
					// The XCM executor of Polkadot doesn't have a configured `Aliasers`
					Err(BenchmarkError::Skip)
				}
			}

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);

			add_benchmarks!(params, batches);

			Ok(batches)
		}
	}
}

#[cfg(test)]
mod test_fees {
	use super::*;
	use frame_support::{dispatch::GetDispatchInfo, weights::WeightToFee as WeightToFeeT};
	use keyring::Sr25519Keyring::{Alice, Charlie};
	use pallet_transaction_payment::Multiplier;
	use runtime_common::MinimumMultiplier;
	use separator::Separatable;
	use sp_runtime::{assert_eq_error_rate, FixedPointNumber, MultiAddress, MultiSignature};

	#[test]
	fn payout_weight_portion() {
		use pallet_staking::WeightInfo;
		let payout_weight =
			<Runtime as pallet_staking::Config>::WeightInfo::payout_stakers_alive_staked(
				MaxNominatorRewardedPerValidator::get(),
			)
			.ref_time() as f64;
		let block_weight = BlockWeights::get().max_block.ref_time() as f64;

		println!(
			"a full payout takes {:.2} of the block weight [{} / {}]",
			payout_weight / block_weight,
			payout_weight,
			block_weight
		);
		assert!(payout_weight * 2f64 < block_weight);
	}

	#[test]
	fn block_cost() {
		let max_block_weight = BlockWeights::get().max_block;
		let raw_fee = WeightToFee::weight_to_fee(&max_block_weight);

		let fee_with_multiplier = |m: Multiplier| {
			println!(
				"Full Block weight == {} // multiplier: {:?} // WeightToFee(full_block) == {} plank",
				max_block_weight,
				m,
				m.saturating_mul_int(raw_fee).separated_string(),
			);
		};
		fee_with_multiplier(MinimumMultiplier::get());
		fee_with_multiplier(Multiplier::from_rational(1, 2));
		fee_with_multiplier(Multiplier::from_u32(1));
		fee_with_multiplier(Multiplier::from_u32(2));
	}

	#[test]
	fn transfer_cost_min_multiplier() {
		let min_multiplier = MinimumMultiplier::get();
		let call = pallet_balances::Call::<Runtime>::transfer_keep_alive {
			dest: Charlie.to_account_id().into(),
			value: Default::default(),
		};
		let info = call.get_dispatch_info();
		println!("call = {:?} / info = {:?}", call, info);
		// convert to runtime call.
		let call = RuntimeCall::Balances(call);
		let extra: SignedExtra = (
			frame_system::CheckNonZeroSender::<Runtime>::new(),
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckMortality::<Runtime>::from(generic::Era::immortal()),
			frame_system::CheckNonce::<Runtime>::from(1),
			frame_system::CheckWeight::<Runtime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(0),
			claims::PrevalidateAttests::<Runtime>::new(),
		);
		let uxt = UncheckedExtrinsic {
			function: call,
			signature: Some((
				MultiAddress::Id(Alice.to_account_id()),
				MultiSignature::Sr25519(Alice.sign(b"foo")),
				extra,
			)),
		};
		let len = uxt.encoded_size();

		let mut ext = sp_io::TestExternalities::new_empty();
		let mut test_with_multiplier = |m: Multiplier| {
			ext.execute_with(|| {
				pallet_transaction_payment::NextFeeMultiplier::<Runtime>::put(m);
				let fee = TransactionPayment::query_fee_details(uxt.clone(), len as u32);
				println!(
					"multiplier = {:?} // fee details = {:?} // final fee = {:?}",
					pallet_transaction_payment::NextFeeMultiplier::<Runtime>::get(),
					fee,
					fee.final_fee().separated_string(),
				);
			});
		};

		test_with_multiplier(min_multiplier);
		test_with_multiplier(Multiplier::saturating_from_rational(1u128, 1u128));
		test_with_multiplier(Multiplier::saturating_from_rational(1u128, 1_0u128));
		test_with_multiplier(Multiplier::saturating_from_rational(1u128, 1_00u128));
		test_with_multiplier(Multiplier::saturating_from_rational(1u128, 1_000u128));
		test_with_multiplier(Multiplier::saturating_from_rational(1u128, 1_000_000u128));
		test_with_multiplier(Multiplier::saturating_from_rational(1u128, 1_000_000_000u128));
	}

	#[test]
	fn nominator_limit() {
		use pallet_election_provider_multi_phase::WeightInfo;
		// starting point of the nominators.
		let target_voters: u32 = 50_000;

		// assuming we want around 5k candidates and 1k active validators. (March 31, 2021)
		let all_targets: u32 = 5_000;
		let desired: u32 = 1_000;
		let weight_with = |active| {
			<Runtime as pallet_election_provider_multi_phase::Config>::WeightInfo::submit_unsigned(
				active,
				all_targets,
				active,
				desired,
			)
		};

		let mut active = target_voters;
		while weight_with(active).all_lte(OffchainSolutionWeightLimit::get()) ||
			active == target_voters
		{
			active += 1;
		}

		println!("can support {} nominators to yield a weight of {}", active, weight_with(active));
		assert!(active > target_voters, "we need to reevaluate the weight of the election system");
	}

	#[test]
	fn signed_deposit_is_sensible() {
		// ensure this number does not change, or that it is checked after each change.
		// a 1 MB solution should take (40 + 10) DOTs of deposit.
		let deposit = SignedFixedDeposit::get() + (SignedDepositByte::get() * 1024 * 1024);
		assert_eq_error_rate!(deposit, 50 * DOLLARS, DOLLARS);
	}
}

#[cfg(test)]
mod test {
	use std::collections::HashSet;

	use super::*;
	use frame_support::traits::WhitelistedStorageKeys;
	use scale_info::TypeInfo;
	use sp_core::hexdisplay::HexDisplay;

	#[test]
	fn call_size() {
		RuntimeCall::assert_size_under(256);
	}

	#[test]
	fn check_whitelist() {
		let whitelist: HashSet<String> = AllPalletsWithSystem::whitelisted_storage_keys()
			.iter()
			.map(|e| HexDisplay::from(&e.key).to_string())
			.collect();

		// Block number
		assert!(
			whitelist.contains("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac")
		);
		// Total issuance
		assert!(
			whitelist.contains("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80")
		);
		// Execution phase
		assert!(
			whitelist.contains("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a")
		);
		// Event count
		assert!(
			whitelist.contains("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850")
		);
		// System events
		assert!(
			whitelist.contains("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7")
		);
		// XcmPallet VersionDiscoveryQueue
		assert!(
			whitelist.contains("1405f2411d0af5a7ff397e7c9dc68d194a222ba0333561192e474c59ed8e30e1")
		);
		// XcmPallet SafeXcmVersion
		assert!(
			whitelist.contains("1405f2411d0af5a7ff397e7c9dc68d196323ae84c43568be0d1394d5d0d522c4")
		);
	}

	#[test]
	fn check_treasury_pallet_id() {
		assert_eq!(
			<Treasury as frame_support::traits::PalletInfoAccess>::index() as u8,
			polkadot_runtime_constants::TREASURY_PALLET_ID
		);
	}

	#[test]
	fn ensure_xcm_metadata_is_correct() {
		let path = xcm::VersionedXcm::<()>::type_info().path;
		// Ensure that the name doesn't include `staging` (from the pallet name)
		assert_eq!(vec!["xcm", "VersionedXcm"], path.segments);
	}
}

#[cfg(test)]
mod multiplier_tests {
	use super::*;
	use frame_support::{
		dispatch::DispatchInfo,
		traits::{OnFinalize, PalletInfoAccess},
	};
	use runtime_common::{MinimumMultiplier, TargetBlockFullness};
	use separator::Separatable;
	use sp_runtime::traits::Convert;

	fn run_with_system_weight<F>(w: Weight, mut assertions: F)
	where
		F: FnMut() -> (),
	{
		let mut t: sp_io::TestExternalities = frame_system::GenesisConfig::<Runtime>::default()
			.build_storage()
			.unwrap()
			.into();
		t.execute_with(|| {
			System::set_block_consumed_resources(w, 0);
			assertions()
		});
	}

	#[test]
	fn multiplier_can_grow_from_zero() {
		let minimum_multiplier = MinimumMultiplier::get();
		let target = TargetBlockFullness::get() *
			BlockWeights::get().get(DispatchClass::Normal).max_total.unwrap();
		// if the min is too small, then this will not change, and we are doomed forever.
		// the weight is 1/100th bigger than target.
		run_with_system_weight(target.saturating_mul(101) / 100, || {
			let next = SlowAdjustingFeeUpdate::<Runtime>::convert(minimum_multiplier);
			assert!(next > minimum_multiplier, "{:?} !>= {:?}", next, minimum_multiplier);
		})
	}

	#[test]
	fn fast_unstake_estimate() {
		use pallet_fast_unstake::WeightInfo;
		let block_time = BlockWeights::get().max_block.ref_time() as f32;
		let on_idle = weights::pallet_fast_unstake::WeightInfo::<Runtime>::on_idle_check(
			300,
			<Runtime as pallet_fast_unstake::Config>::BatchSize::get(),
		)
		.ref_time() as f32;
		println!("ratio of block weight for full batch fast-unstake {}", on_idle / block_time);
		assert!(on_idle / block_time <= 0.5f32)
	}

	#[test]
	fn treasury_pallet_index_is_correct() {
		assert_eq!(TREASURY_PALLET_ID, <Treasury as PalletInfoAccess>::index() as u8);
	}

	#[test]
	#[ignore]
	fn multiplier_growth_simulator() {
		// assume the multiplier is initially set to its minimum. We update it with values twice the
		//target (target is 25%, thus 50%) and we see at which point it reaches 1.
		let mut multiplier = MinimumMultiplier::get();
		let block_weight = BlockWeights::get().get(DispatchClass::Normal).max_total.unwrap();
		let mut blocks = 0;
		let mut fees_paid = 0;

		frame_system::Pallet::<Runtime>::set_block_consumed_resources(Weight::MAX, 0);
		let info = DispatchInfo { weight: Weight::MAX, ..Default::default() };

		let mut t: sp_io::TestExternalities = frame_system::GenesisConfig::<Runtime>::default()
			.build_storage()
			.unwrap()
			.into();
		// set the minimum
		t.execute_with(|| {
			pallet_transaction_payment::NextFeeMultiplier::<Runtime>::set(MinimumMultiplier::get());
		});

		while multiplier <= Multiplier::from_u32(1) {
			t.execute_with(|| {
				// imagine this tx was called.
				let fee = TransactionPayment::compute_fee(0, &info, 0);
				fees_paid += fee;

				// this will update the multiplier.
				System::set_block_consumed_resources(block_weight, 0);
				TransactionPayment::on_finalize(1);
				let next = TransactionPayment::next_fee_multiplier();

				assert!(next > multiplier, "{:?} !>= {:?}", next, multiplier);
				multiplier = next;

				println!(
					"block = {} / multiplier {:?} / fee = {:?} / fess so far {:?}",
					blocks,
					multiplier,
					fee.separated_string(),
					fees_paid.separated_string()
				);
			});
			blocks += 1;
		}
	}

	#[test]
	#[ignore]
	fn multiplier_cool_down_simulator() {
		// assume the multiplier is initially set to its minimum. We update it with values twice the
		//target (target is 25%, thus 50%) and we see at which point it reaches 1.
		let mut multiplier = Multiplier::from_u32(2);
		let mut blocks = 0;

		let mut t: sp_io::TestExternalities = frame_system::GenesisConfig::<Runtime>::default()
			.build_storage()
			.unwrap()
			.into();
		// set the minimum
		t.execute_with(|| {
			pallet_transaction_payment::NextFeeMultiplier::<Runtime>::set(multiplier);
		});

		while multiplier > Multiplier::from_u32(0) {
			t.execute_with(|| {
				// this will update the multiplier.
				TransactionPayment::on_finalize(1);
				let next = TransactionPayment::next_fee_multiplier();

				assert!(next < multiplier, "{:?} !>= {:?}", next, multiplier);
				multiplier = next;

				println!("block = {} / multiplier {:?}", blocks, multiplier);
			});
			blocks += 1;
		}
	}
}

#[cfg(all(test, feature = "try-runtime"))]
mod remote_tests {
	use super::*;
	use frame_try_runtime::{runtime_decl_for_try_runtime::TryRuntime, UpgradeCheckSelect};
	use remote_externalities::{
		Builder, Mode, OfflineConfig, OnlineConfig, RemoteExternalities, SnapshotConfig, Transport,
	};
	use std::env::var;

	async fn remote_ext_test_setup() -> RemoteExternalities<Block> {
		let transport: Transport =
			var("WS").unwrap_or("wss://rpc.polkadot.io:443".to_string()).into();
		let maybe_state_snapshot: Option<SnapshotConfig> = var("SNAP").map(|s| s.into()).ok();
		Builder::<Block>::default()
			.mode(if let Some(state_snapshot) = maybe_state_snapshot {
				Mode::OfflineOrElseOnline(
					OfflineConfig { state_snapshot: state_snapshot.clone() },
					OnlineConfig {
						transport,
						state_snapshot: Some(state_snapshot),
						..Default::default()
					},
				)
			} else {
				Mode::Online(OnlineConfig { transport, ..Default::default() })
			})
			.build()
			.await
			.unwrap()
	}

	#[tokio::test]
	async fn dispatch_all_proposals() {
		if var("RUN_OPENGOV_TEST").is_err() {
			return
		}

		sp_tracing::try_init_simple();
		let mut ext = remote_ext_test_setup().await;
		ext.execute_with(|| {
			type Ref = pallet_referenda::ReferendumInfoOf<Runtime, ()>;
			type RefStatus = pallet_referenda::ReferendumStatusOf<Runtime, ()>;
			use sp_runtime::traits::Dispatchable;
			let all_refs: Vec<(u32, RefStatus)> =
				pallet_referenda::ReferendumInfoFor::<Runtime>::iter()
					.filter_map(|(idx, reff): (_, Ref)| {
						if let Ref::Ongoing(ref_status) = reff {
							Some((idx, ref_status))
						} else {
							None
						}
					})
					.collect::<Vec<_>>();

			for (ref_index, referenda) in all_refs {
				log::info!(target: LOG_TARGET, "🚀 executing referenda #{}", ref_index);
				let RefStatus { origin, proposal, .. } = referenda;
				// we do more or less what the scheduler will do under the hood, as best as we can
				// imitate:
				let (call, _len) = match <
					<Runtime as pallet_scheduler::Config>::Preimages
					as
					frame_support::traits::QueryPreimage
				>::peek(&proposal) {
					Ok(x) => x,
					Err(e) => {
						log::error!(target: LOG_TARGET, "failed to get preimage: {:?}", e);
						continue;
					}
				};

				let dispatch_result = call.dispatch(origin.clone().into());
				log::info!(target: LOG_TARGET, "outcome of dispatch with origin {:?}: {:?}", origin, dispatch_result);
			}
		});
	}

	#[tokio::test]
	async fn run_migrations() {
		if var("RUN_MIGRATION_TESTS").is_err() {
			return
		}

		sp_tracing::try_init_simple();
		let mut ext = remote_ext_test_setup().await;
		ext.execute_with(|| Runtime::on_runtime_upgrade(UpgradeCheckSelect::PreAndPost));
	}

	#[tokio::test]
	#[ignore = "this test is meant to be executed manually"]
	async fn try_fast_unstake_all() {
		sp_tracing::try_init_simple();
		let transport: Transport =
			var("WS").unwrap_or("wss://rpc.polkadot.io:443".to_string()).into();
		let maybe_state_snapshot: Option<SnapshotConfig> = var("SNAP").map(|s| s.into()).ok();
		let mut ext = Builder::<Block>::default()
			.mode(if let Some(state_snapshot) = maybe_state_snapshot {
				Mode::OfflineOrElseOnline(
					OfflineConfig { state_snapshot: state_snapshot.clone() },
					OnlineConfig {
						transport,
						state_snapshot: Some(state_snapshot),
						..Default::default()
					},
				)
			} else {
				Mode::Online(OnlineConfig { transport, ..Default::default() })
			})
			.build()
			.await
			.unwrap();
		ext.execute_with(|| {
			pallet_fast_unstake::ErasToCheckPerBlock::<Runtime>::put(1);
			runtime_common::try_runtime::migrate_all_inactive_nominators::<Runtime>()
		});
	}
}
