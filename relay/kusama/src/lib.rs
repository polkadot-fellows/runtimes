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
// along with Polkadot. If not, see <http://www.gnu.org/licenses/>.

//! The Kusama runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit.
#![recursion_limit = "512"]

extern crate alloc;

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
	dynamic_params::{dynamic_pallet_params, dynamic_params},
	traits::{EnsureOrigin, EnsureOriginWithArg},
	weights::constants::{WEIGHT_PROOF_SIZE_PER_KB, WEIGHT_REF_TIME_PER_MICROS},
};
use kusama_runtime_constants::system_parachain::coretime::TIMESLICE_PERIOD;
use pallet_nis::WithMaximumOf;
use polkadot_primitives::{
	slashing, AccountId, AccountIndex, ApprovalVotingParams, Balance, BlockNumber, CandidateEvent,
	CandidateHash, CommittedCandidateReceipt, CoreIndex, CoreState, DisputeState, ExecutorParams,
	GroupRotationInfo, Hash, Id as ParaId, InboundDownwardMessage, InboundHrmpMessage, Moment,
	NodeFeatures, Nonce, OccupiedCoreAssumption, PersistedValidationData, ScrapedOnChainVotes,
	SessionInfo, Signature, ValidationCode, ValidationCodeHash, ValidatorId, ValidatorIndex,
	LOWEST_PUBLIC_ID, PARACHAIN_KEY_TYPE_ID,
};
use polkadot_runtime_common::{
	auctions, claims, crowdloan, impl_runtime_weights,
	impls::{
		ContainsParts as ContainsLocationParts, DealWithFees, LocatableAssetConverter,
		VersionedLocatableAsset, VersionedLocationConverter,
	},
	paras_registrar, prod_or_fast, slots, BalanceToU256, BlockHashCount, BlockLength,
	CurrencyToVote, SlowAdjustingFeeUpdate, U256ToBalance,
};
use relay_common::apis::*;
use scale_info::TypeInfo;
use sp_runtime::traits::Saturating;
use sp_std::{
	cmp::Ordering,
	collections::{btree_map::BTreeMap, vec_deque::VecDeque},
	prelude::*,
};

use runtime_parachains::{
	assigner_coretime as parachains_assigner_coretime, configuration as parachains_configuration,
	configuration::ActiveConfigHrmpChannelSizeAndCapacityRatio,
	coretime, disputes as parachains_disputes,
	disputes::slashing as parachains_slashing,
	dmp as parachains_dmp, hrmp as parachains_hrmp, inclusion as parachains_inclusion,
	inclusion::{AggregateMessageOrigin, UmpQueueId},
	initializer as parachains_initializer, on_demand as parachains_on_demand,
	origin as parachains_origin, paras as parachains_paras,
	paras_inherent as parachains_paras_inherent, reward_points as parachains_reward_points,
	runtime_api_impl::{
		v10 as parachains_runtime_api_impl, vstaging as parachains_vstaging_api_impl,
	},
	scheduler as parachains_scheduler, session_info as parachains_session_info,
	shared as parachains_shared,
};

use authority_discovery_primitives::AuthorityId as AuthorityDiscoveryId;
use beefy_primitives::{
	ecdsa_crypto::{AuthorityId as BeefyId, Signature as BeefySignature},
	mmr::{BeefyDataProvider, MmrLeafVersion},
	OpaqueKeyOwnershipProof,
};
use frame_election_provider_support::{
	bounds::ElectionBoundsBuilder, generate_solution_type, onchain, NposSolution,
	SequentialPhragmen,
};
use frame_support::{
	construct_runtime,
	genesis_builder_helper::{build_state, get_preset},
	parameter_types,
	traits::{
		fungible::HoldConsideration,
		tokens::{imbalance::ResolveTo, UnityOrOuterConversion},
		ConstU32, ConstU8, EitherOf, EitherOfDiverse, Everything, FromContains, InstanceFilter,
		KeyOwnerProofSystem, LinearStoragePrice, PrivilegeCmp, ProcessMessage, ProcessMessageError,
		StorageMapShim, WithdrawReasons,
	},
	weights::{ConstantMultiplier, WeightMeter, WeightToFee as _},
	PalletId,
};
use frame_system::EnsureRoot;
use pallet_grandpa::{fg_primitives, AuthorityId as GrandpaId};
use pallet_session::historical as session_historical;
use pallet_transaction_payment::{FeeDetails, FungibleAdapter, RuntimeDispatchInfo};
use sp_core::{ConstU128, OpaqueMetadata, H256};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{
		AccountIdConversion, AccountIdLookup, BlakeTwo256, Block as BlockT, ConvertInto,
		Extrinsic as ExtrinsicT, IdentityLookup, Keccak256, OpaqueKeys, SaturatedConversion,
		Verify,
	},
	transaction_validity::{TransactionPriority, TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, FixedU128, KeyTypeId, OpaqueValue, Perbill, Percent, Permill,
	RuntimeDebug,
};
use sp_staking::SessionIndex;
#[cfg(any(feature = "std", test))]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
use xcm::prelude::*;
use xcm_builder::PayOverXcm;
use xcm_runtime_apis::{
	dry_run::{CallDryRunEffects, Error as XcmDryRunApiError, XcmDryRunEffects},
	fees::Error as XcmPaymentApiError,
};

pub use frame_system::Call as SystemCall;
pub use pallet_balances::Call as BalancesCall;
pub use pallet_election_provider_multi_phase::{Call as EPMCall, GeometricDepositBase};
use pallet_staking::UseValidatorsMap;
use pallet_treasury::TreasuryAccountId;
use sp_runtime::traits::Get;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

/// Constant values used within the runtime.
use kusama_runtime_constants::{
	currency::*, fee::*, system_parachain, time::*, TREASURY_PALLET_ID,
};

/// Default logging target.
pub const LOG_TARGET: &str = "runtime::kusama";

// Genesis preset configurations.
pub mod genesis_config_presets;

// Weights used in the runtime.
mod weights;

// Voter bag threshold definitions.
mod bag_thresholds;

// Historical information of society finances.
mod past_payouts;

// XCM configurations.
pub mod xcm_config;

// Governance configurations.
pub mod governance;
use governance::{
	pallet_custom_origins, AuctionAdmin, Fellows, GeneralAdmin, LeaseAdmin, StakingAdmin,
	Treasurer, TreasurySpender,
};

#[cfg(test)]
mod tests;

impl_runtime_weights!(kusama_runtime_constants);

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// Runtime version (Kusama).
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("kusama"),
	impl_name: create_runtime_str!("parity-kusama"),
	authoring_version: 2,
	spec_version: 1_003_003,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 26,
	state_version: 1,
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
	pub const SS58Prefix: u8 = 2;
}

impl frame_system::Config for Runtime {
	type BaseCallFilter = Everything;
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
	type RuntimeTask = RuntimeTask;
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
	type SingleBlockMigrations = ();
	type MultiBlockMigrator = ();
	type PreInherents = ();
	type PostInherents = ();
	type PostTransactions = ();
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * BlockWeights::get().max_block;
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
		"KSM_EPOCH_DURATION"
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

	type KeyOwnerProof =
		<Historical as KeyOwnerProofSystem<(KeyTypeId, pallet_babe::AuthorityId)>>::Proof;

	type EquivocationReportSystem =
		pallet_babe::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;

	type WeightInfo = ();

	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = MaxNominators;
}

parameter_types! {
	pub const IndexDeposit: Balance = 100 * CENTS;
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
	type WeightInfo = weights::pallet_balances_native::WeightInfo<Runtime>;
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxFreezes = ConstU32<8>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
}

parameter_types! {
	pub BeefySetIdSessionEntries: u32 = BondingDuration::get() * SessionsPerEra::get();
}

impl pallet_beefy::Config for Runtime {
	type BeefyId = BeefyId;
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = MaxNominators;
	type MaxSetIdSessionEntries = BeefySetIdSessionEntries;
	type OnNewValidatorSet = BeefyMmrLeaf;
	type AncestryHelper = BeefyMmrLeaf;
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
	type BlockHashProvider = pallet_mmr::DefaultBlockHashProvider<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = parachains_paras::benchmarking::mmr_setup::MmrSetup<Runtime>;
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
		let mut para_heads: Vec<(u32, Vec<u8>)> = parachains_paras::Parachains::<Runtime>::get()
			.into_iter()
			.filter_map(|id| {
				parachains_paras::Heads::<Runtime>::get(id).map(|head| (id.into(), head.0))
			})
			.collect();
		para_heads.sort_by_key(|k| k.0);
		binary_merkle_tree::merkle_root::<mmr::Hashing, _>(
			para_heads.into_iter().map(|pair| pair.encode()),
		)
	}
}

impl pallet_beefy_mmr::Config for Runtime {
	type LeafVersion = LeafVersion;
	type BeefyAuthorityToMerkleLeaf = pallet_beefy_mmr::BeefyEcdsaToEthereum;
	type LeafExtra = H256;
	type BeefyDataProvider = ParaHeadsRootProvider;
	type WeightInfo = weights::pallet_beefy_mmr::WeightInfo<Runtime>;
}

parameter_types! {
	pub const TransactionByteFee: Balance = kusama_runtime_constants::fee::TRANSACTION_BYTE_FEE;
	/// This value increases the priority of `Operational` transactions by adding
	/// a "virtual tip" that's equal to the `OperationalFeeMultiplier * final_fee`.
	pub const OperationalFeeMultiplier: u8 = 5;
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = FungibleAdapter<Balances, DealWithFees<Self>>;
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
	type EventHandler = Staking;
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub grandpa: Grandpa,
		pub babe: Babe,
		pub para_validator: Initializer,
		pub para_assignment: ParaSessionInfo,
		pub authority_discovery: AuthorityDiscovery,
		pub beefy: Beefy,
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
		MINUTES.min(EpochDuration::get().saturated_into::<u32>() / 2),
		"KSM_SIGNED_PHASE"
	);
	pub UnsignedPhase: u32 = prod_or_fast!(
		EPOCH_DURATION_IN_SLOTS / 4,
		MINUTES.min(EpochDuration::get().saturated_into::<u32>() / 2),
		"KSM_UNSIGNED_PHASE"
	);

	// signed config
	pub const SignedMaxSubmissions: u32 = 16;
	pub const SignedMaxRefunds: u32 = 16 / 4;
	pub const SignedFixedDeposit: Balance = deposit(2, 0);
	pub const SignedDepositIncreaseFactor: Percent = Percent::from_percent(10);
	pub const SignedDepositByte: Balance = deposit(0, 10) / 1024;
	// Each good submission will get 1/10 KSM as reward
	pub SignedRewardBase: Balance =  UNITS / 10;

	// 1 hour session, 15 minutes unsigned phase, 8 offchain executions.
	pub OffchainRepeat: BlockNumber = UnsignedPhase::get() / 8;

	pub const MaxElectingVoters: u32 = 12_500;
	/// We take the top 12500 nominators as electing voters and all of the validators as electable
	/// targets. Whilst this is the case, we cannot and shall not increase the size of the
	/// validator intentions.
	pub ElectionBounds: frame_election_provider_support::bounds::ElectionBounds =
		ElectionBoundsBuilder::default().voters_count(MaxElectingVoters::get().into()).build();
	pub NposSolutionPriority: TransactionPriority =
		Perbill::from_percent(90) * TransactionPriority::MAX;
	/// Setup election pallet to support maximum winners upto 2000. This will mean Staking Pallet
	/// cannot have active validators higher than this count.
	pub const MaxActiveValidators: u32 = 2000;
}

generate_solution_type!(
	#[compact]
	pub struct NposCompactSolution24::<
		VoterIndex = u32,
		TargetIndex = u16,
		Accuracy = sp_runtime::PerU16,
		MaxVoters = MaxElectingVoters,
	>(24)
);

pub struct OnChainSeqPhragmen;
impl onchain::Config for OnChainSeqPhragmen {
	type System = Runtime;
	type Solver =
		SequentialPhragmen<AccountId, polkadot_runtime_common::elections::OnChainAccuracy>;
	type DataProvider = Staking;
	type WeightInfo = weights::frame_election_provider_support::WeightInfo<Runtime>;
	type MaxWinners = MaxActiveValidators;
	type Bounds = ElectionBounds;
}

impl pallet_election_provider_multi_phase::MinerConfig for Runtime {
	type AccountId = AccountId;
	type MaxLength = OffchainSolutionLengthLimit;
	type MaxWeight = OffchainSolutionWeightLimit;
	type Solution = NposCompactSolution24;
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
	type SignedPhase = SignedPhase;
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
	type BenchmarkingConfig = polkadot_runtime_common::elections::BenchmarkConfig;
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

#[derive(Default, MaxEncodedLen, Encode, Decode, TypeInfo, Clone, Eq, PartialEq, Debug)]
pub struct BurnDestinationAccount(pub Option<AccountId>);

impl BurnDestinationAccount {
	pub fn is_set(&self) -> bool {
		self.0.is_some()
	}
}

/// Dynamic params that can be adjusted at runtime.
#[dynamic_params(RuntimeParameters, pallet_parameters::Parameters::<Runtime>)]
pub mod dynamic_params {
	use super::*;

	/// Parameters used to calculate era payouts, see
	/// [`polkadot_runtime_common::impls::EraPayoutParams`].
	#[dynamic_pallet_params]
	#[codec(index = 0)]
	pub mod inflation {
		/// Minimum inflation rate used to calculate era payouts.
		#[codec(index = 0)]
		pub static MinInflation: Perquintill = Perquintill::from_rational(25u64, 1000);

		/// Maximum inflation rate used to calculate era payouts.
		#[codec(index = 1)]
		pub static MaxInflation: Perquintill = Perquintill::from_percent(10);

		/// Ideal stake ratio used to calculate era payouts.
		#[codec(index = 2)]
		pub static IdealStake: Perquintill = Perquintill::from_percent(75);

		/// Falloff used to calculate era payouts.
		#[codec(index = 3)]
		pub static Falloff: Perquintill = Perquintill::from_percent(5);

		/// Whether to use auction slots or not in the calculation of era payouts. If true, then we
		/// subtract `num_auctioned_slots.min(60) / 200` from `ideal_stake`.
		///
		/// That is, we assume up to 60 parachains that are leased can reduce the ideal stake by a
		/// maximum of 30%.
		///
		/// With the move to Agile Coretime, this parameter does not make much sense and should
		/// generally be set to false.
		#[codec(index = 4)]
		pub static UseAuctionSlots: bool = true;
	}

	/// Parameters used by `pallet-treasury` to handle the burn process.
	#[dynamic_pallet_params]
	#[codec(index = 1)]
	pub mod treasury {
		#[codec(index = 0)]
		pub static BurnPortion: Permill = Permill::from_percent(0);

		#[codec(index = 1)]
		pub static BurnDestination: BurnDestinationAccount = Default::default();
	}
}

#[cfg(feature = "runtime-benchmarks")]
impl Default for RuntimeParameters {
	fn default() -> Self {
		RuntimeParameters::Inflation(dynamic_params::inflation::Parameters::MinInflation(
			dynamic_params::inflation::MinInflation,
			Some(Perquintill::from_rational(25u64, 1000u64)),
		))
	}
}

/// Defines what origin can modify which dynamic parameters.
pub struct DynamicParameterOrigin;
impl EnsureOriginWithArg<RuntimeOrigin, RuntimeParametersKey> for DynamicParameterOrigin {
	type Success = ();

	fn try_origin(
		origin: RuntimeOrigin,
		key: &RuntimeParametersKey,
	) -> Result<Self::Success, RuntimeOrigin> {
		use crate::RuntimeParametersKey::*;

		match key {
			Inflation(_) => frame_system::ensure_root(origin.clone()),
			Treasury(_) =>
				EitherOf::<EnsureRoot<AccountId>, GeneralAdmin>::ensure_origin(origin.clone()),
		}
		.map_err(|_| origin)
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin(_key: &RuntimeParametersKey) -> Result<RuntimeOrigin, ()> {
		// Provide the origin for the parameter returned by `Default`:
		Ok(RuntimeOrigin::root())
	}
}

impl pallet_parameters::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeParameters = RuntimeParameters;
	type AdminOrigin = DynamicParameterOrigin;
	type WeightInfo = weights::pallet_parameters::WeightInfo<Runtime>;
}

pub struct EraPayout;
impl pallet_staking::EraPayout<Balance> for EraPayout {
	fn era_payout(
		total_staked: Balance,
		_total_issuance: Balance,
		era_duration_millis: u64,
	) -> (Balance, Balance) {
		const MILLISECONDS_PER_YEAR: u64 = 1000 * 3600 * 24 * 36525 / 100;

		let params = relay_common::EraPayoutParams {
			total_staked,
			total_stakable: Nis::issuance().other,
			ideal_stake: dynamic_params::inflation::IdealStake::get(),
			max_annual_inflation: dynamic_params::inflation::MaxInflation::get(),
			min_annual_inflation: dynamic_params::inflation::MinInflation::get(),
			falloff: dynamic_params::inflation::Falloff::get(),
			period_fraction: Perquintill::from_rational(era_duration_millis, MILLISECONDS_PER_YEAR),
			legacy_auction_proportion: if dynamic_params::inflation::UseAuctionSlots::get() {
				let auctioned_slots = parachains_paras::Parachains::<Runtime>::get()
					.into_iter()
					// All active para-ids that do not belong to a system chain is the number of
					// parachains that we should take into account for inflation.
					.filter(|i| *i >= LOWEST_PUBLIC_ID)
					.count() as u64;
				Some(Perquintill::from_rational(auctioned_slots.min(60), 200u64))
			} else {
				None
			},
		};
		log::debug!(target: "runtime::kusama", "params: {:?}", params);
		relay_common::relay_era_payout(params)
	}
}

parameter_types! {
	// Six sessions in an era (6 hours).
	pub const SessionsPerEra: SessionIndex = prod_or_fast!(6, 1);

	// 28 eras for unbonding (7 days).
	pub BondingDuration: sp_staking::EraIndex = prod_or_fast!(
		28,
		28,
		"DOT_BONDING_DURATION"
	);
	// 27 eras in which slashes can be cancelled (slightly less than 7 days).
	pub SlashDeferDuration: sp_staking::EraIndex = prod_or_fast!(
		27,
		27,
		"DOT_SLASH_DEFER_DURATION"
	);
	pub const MaxExposurePageSize: u32 = 512;
	// Note: this is not really correct as Max Nominators is (MaxExposurePageSize * page_count) but
	// this is an unbounded number. We just set it to a reasonably high value, 1 full page
	// of nominators.
	pub const MaxNominators: u32 = 512;
	pub const OffendingValidatorsThreshold: Perbill = Perbill::from_percent(17);
	// 24
	pub const MaxNominations: u32 = <NposCompactSolution24 as NposSolution>::LIMIT as u32;
}

impl pallet_staking::Config for Runtime {
	type Currency = Balances;
	type CurrencyBalance = Balance;
	type UnixTime = Timestamp;
	type CurrencyToVote = CurrencyToVote;
	type ElectionProvider = ElectionProviderMultiPhase;
	type GenesisElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
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
	type NextNewSession = Session;
	type MaxExposurePageSize = MaxExposurePageSize;
	type VoterList = VoterList;
	type TargetList = UseValidatorsMap<Self>;
	type NominationsQuota = pallet_staking::FixedNominationsQuota<{ MaxNominations::get() }>;
	type MaxUnlockingChunks = frame_support::traits::ConstU32<32>;
	type HistoryDepth = frame_support::traits::ConstU32<84>;
	type MaxControllersInDeprecationBatch = ConstU32<5169>;
	type BenchmarkingConfig = polkadot_runtime_common::StakingBenchmarkingConfig;
	type EventListeners = (NominationPools, DelegatedStaking);
	type DisablingStrategy = pallet_staking::UpToLimitDisablingStrategy;
	type WeightInfo = weights::pallet_staking::WeightInfo<Runtime>;
}

impl pallet_fast_unstake::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BatchSize = frame_support::traits::ConstU32<64>;
	type Deposit = frame_support::traits::ConstU128<{ CENTS * 100 }>;
	type ControlOrigin = EnsureRoot<AccountId>;
	type Staking = Staking;
	type MaxErasToCheckPerBlock = ConstU32<1>;
	type WeightInfo = weights::pallet_fast_unstake::WeightInfo<Runtime>;
}

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = 2000 * CENTS;
	pub const ProposalBondMaximum: Balance = GRAND;
	pub const SpendPeriod: BlockNumber = 6 * DAYS;
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	pub const PayoutSpendPeriod: BlockNumber = 30 * DAYS;
	// The asset's interior location for the paying account. This is the Treasury
	// pallet instance (which sits at index 18).
	pub TreasuryInteriorLocation: InteriorLocation = PalletInstance(TREASURY_PALLET_ID).into();

	pub const TipCountdown: BlockNumber = DAYS;
	pub const TipFindersFee: Percent = Percent::from_percent(20);
	pub const TipReportDepositBase: Balance = 100 * CENTS;
	pub const DataDepositPerByte: Balance = CENTS / 10;
	pub const MaxApprovals: u32 = 100;
	pub const MaxAuthorities: u32 = 100_000;
	pub const MaxKeys: u32 = 10_000;
	pub const MaxPeerInHeartbeats: u32 = 10_000;
}

use frame_support::traits::{Currency, OnUnbalanced};

pub type BalancesNegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;
pub struct TreasuryBurnHandler;

impl OnUnbalanced<BalancesNegativeImbalance> for TreasuryBurnHandler {
	fn on_nonzero_unbalanced(amount: BalancesNegativeImbalance) {
		let destination = dynamic_params::treasury::BurnDestination::get();

		if let BurnDestinationAccount(Some(account)) = destination {
			// Must resolve into existing but better to be safe.
			Balances::resolve_creating(&account, amount);
		} else {
			// If no account to destinate the funds to, just drop the
			// imbalance.
			<() as OnUnbalanced<_>>::on_nonzero_unbalanced(amount)
		}
	}
}

impl Get<Permill> for TreasuryBurnHandler {
	fn get() -> Permill {
		let destination = dynamic_params::treasury::BurnDestination::get();

		if destination.is_set() {
			dynamic_params::treasury::BurnPortion::get()
		} else {
			Permill::zero()
		}
	}
}

impl pallet_treasury::Config for Runtime {
	type PalletId = TreasuryPalletId;
	type Currency = Balances;
	type RejectOrigin = EitherOfDiverse<EnsureRoot<AccountId>, Treasurer>;
	type RuntimeEvent = RuntimeEvent;
	type SpendPeriod = SpendPeriod;
	type Burn = TreasuryBurnHandler;
	type BurnDestination = TreasuryBurnHandler;
	type MaxApprovals = MaxApprovals;
	type WeightInfo = weights::pallet_treasury::WeightInfo<Runtime>;
	type SpendFunds = Bounties;
	type SpendOrigin = TreasurySpender;
	type AssetKind = VersionedLocatableAsset;
	type Beneficiary = VersionedLocation;
	type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
	type Paymaster = PayOverXcm<
		TreasuryInteriorLocation,
		crate::xcm_config::XcmRouter,
		crate::XcmPallet,
		ConstU32<{ 6 * HOURS }>,
		Self::Beneficiary,
		Self::AssetKind,
		LocatableAssetConverter,
		VersionedLocationConverter,
	>;
	type BalanceConverter = AssetRateWithNative;
	type PayoutPeriod = PayoutSpendPeriod;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = polkadot_runtime_common::impls::benchmarks::TreasuryArguments;
}

parameter_types! {
	pub const BountyDepositBase: Balance = 100 * CENTS;
	pub const BountyDepositPayoutDelay: BlockNumber = 0;
	pub const BountyUpdatePeriod: BlockNumber = 90 * DAYS;
	pub const MaximumReasonLength: u32 = 16384;
	pub const CuratorDepositMultiplier: Permill = Permill::from_percent(50);
	pub const CuratorDepositMin: Balance = 10 * CENTS;
	pub const CuratorDepositMax: Balance = 500 * CENTS;
	pub const BountyValueMinimum: Balance = 200 * CENTS;
}

impl pallet_bounties::Config for Runtime {
	type BountyDepositBase = BountyDepositBase;
	type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
	type BountyUpdatePeriod = BountyUpdatePeriod;
	type CuratorDepositMultiplier = CuratorDepositMultiplier;
	type CuratorDepositMin = CuratorDepositMin;
	type CuratorDepositMax = CuratorDepositMax;
	type BountyValueMinimum = BountyValueMinimum;
	type ChildBountyManager = ChildBounties;
	type DataDepositPerByte = DataDepositPerByte;
	type RuntimeEvent = RuntimeEvent;
	type MaximumReasonLength = MaximumReasonLength;
	type OnSlash = Treasury;
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
	pub MaxSetIdSessionEntries: u32 = BondingDuration::get() * SessionsPerEra::get();
}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;

	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = MaxNominators;
	type MaxSetIdSessionEntries = MaxSetIdSessionEntries;

	type KeyOwnerProof = <Historical as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;

	type EquivocationReportSystem =
		pallet_grandpa::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

/// Submits transaction with the node's public and signature type. Adheres to the signed extension
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
			frame_metadata_hash_extension::CheckMetadataHash::new(false),
		);
		let raw_payload = SignedPayload::new(call, extra)
			.map_err(|e| {
				log::warn!("Unable to create signed payload: {:?}", e);
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
	pub Prefix: &'static [u8] = b"Pay KSMs to the Kusama account:";
}

impl claims::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type VestingSchedule = Vesting;
	type Prefix = Prefix;
	type MoveClaimOrigin = EnsureRoot<AccountId>;
	type WeightInfo = weights::polkadot_runtime_common_claims::WeightInfo<Runtime>;
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
	pub const ConfigDepositBase: Balance = 500 * CENTS;
	pub const FriendDepositFactor: Balance = 50 * CENTS;
	pub const MaxFriends: u16 = 9;
	pub const RecoveryDeposit: Balance = 500 * CENTS;
}

impl pallet_recovery::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ConfigDepositBase = ConfigDepositBase;
	type FriendDepositFactor = FriendDepositFactor;
	type MaxFriends = MaxFriends;
	type RecoveryDeposit = RecoveryDeposit;
}

parameter_types! {
	pub const SocietyPalletId: PalletId = PalletId(*b"py/socie");
}

impl pallet_society::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type Randomness = pallet_babe::RandomnessFromOneEpochAgo<Runtime>;
	type GraceStrikes = ConstU32<10>;
	type PeriodSpend = ConstU128<{ 500 * QUID }>;
	type VotingPeriod = ConstU32<{ 5 * DAYS }>;
	type ClaimPeriod = ConstU32<{ 2 * DAYS }>;
	type MaxLockDuration = ConstU32<{ 36 * 30 * DAYS }>;
	type FounderSetOrigin = EnsureRoot<AccountId>;
	type ChallengePeriod = ConstU32<{ 7 * DAYS }>;
	type MaxPayouts = ConstU32<8>;
	type MaxBids = ConstU32<512>;
	type PalletId = SocietyPalletId;
	type WeightInfo = weights::pallet_society::WeightInfo<Runtime>;
}

parameter_types! {
	pub const MinVestedTransfer: Balance = 100 * CENTS;
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
	type BlockNumberProvider = System;
	const MAX_VESTING_SCHEDULES: u32 = 28;
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
	TypeInfo,
)]
pub enum ProxyType {
	#[codec(index = 0)]
	Any,
	#[codec(index = 1)]
	NonTransfer,
	#[codec(index = 2)]
	Governance,
	#[codec(index = 3)]
	Staking,
	// Index 4 skipped. Formerly `IdentityJudgement`.
	#[codec(index = 5)]
	CancelProxy,
	#[codec(index = 6)]
	Auction,
	#[codec(index = 7)]
	Society,
	#[codec(index = 8)]
	NominationPools,
	#[codec(index = 9)]
	Spokesperson,
	#[codec(index = 10)]
	ParaRegistration,
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
				RuntimeCall::Babe(..) |
				RuntimeCall::Timestamp(..) |
				RuntimeCall::Indices(pallet_indices::Call::claim {..}) |
				RuntimeCall::Indices(pallet_indices::Call::free {..}) |
				RuntimeCall::Indices(pallet_indices::Call::freeze {..}) |
				// Specifically omitting Indices `transfer`, `force_transfer`
				// Specifically omitting the entire Balances pallet
				RuntimeCall::Staking(..) |
				RuntimeCall::Session(..) |
				RuntimeCall::Grandpa(..) |
				RuntimeCall::Treasury(..) |
				RuntimeCall::Bounties(..) |
				RuntimeCall::ChildBounties(..) |
				RuntimeCall::ConvictionVoting(..) |
				RuntimeCall::Referenda(..) |
				RuntimeCall::FellowshipCollective(..) |
				RuntimeCall::FellowshipReferenda(..) |
				RuntimeCall::Whitelist(..) |
				RuntimeCall::Claims(..) |
				RuntimeCall::Utility(..) |
				RuntimeCall::Society(..) |
				RuntimeCall::Recovery(pallet_recovery::Call::as_recovered {..}) |
				RuntimeCall::Recovery(pallet_recovery::Call::vouch_recovery {..}) |
				RuntimeCall::Recovery(pallet_recovery::Call::claim_recovery {..}) |
				RuntimeCall::Recovery(pallet_recovery::Call::close_recovery {..}) |
				RuntimeCall::Recovery(pallet_recovery::Call::remove_recovery {..}) |
				RuntimeCall::Recovery(pallet_recovery::Call::cancel_recovered {..}) |
				// Specifically omitting Recovery `create_recovery`, `initiate_recovery`
				RuntimeCall::Vesting(pallet_vesting::Call::vest {..}) |
				RuntimeCall::Vesting(pallet_vesting::Call::vest_other {..}) |
				// Specifically omitting Vesting `vested_transfer`, and `force_vested_transfer`
				RuntimeCall::Scheduler(..) |
				RuntimeCall::Proxy(..) |
				RuntimeCall::Multisig(..) |
				RuntimeCall::Nis(..) |
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
					// OpenGov calls
					RuntimeCall::ConvictionVoting(..) |
					RuntimeCall::Referenda(..) |
					RuntimeCall::FellowshipCollective(..) |
					RuntimeCall::FellowshipReferenda(..) |
					RuntimeCall::Whitelist(..)
			),
			ProxyType::Staking => {
				matches!(
					c,
					RuntimeCall::Staking(..) |
						RuntimeCall::Session(..) |
						RuntimeCall::Utility(..) |
						RuntimeCall::FastUnstake(..) |
						RuntimeCall::VoterList(..) |
						RuntimeCall::NominationPools(..)
				)
			},
			ProxyType::NominationPools => {
				matches!(c, RuntimeCall::NominationPools(..) | RuntimeCall::Utility(..))
			},
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
			ProxyType::Society => matches!(c, RuntimeCall::Society(..)),
			ProxyType::Spokesperson => matches!(
				c,
				RuntimeCall::System(frame_system::Call::remark { .. }) |
					RuntimeCall::System(frame_system::Call::remark_with_event { .. })
			),
			ProxyType::ParaRegistration => matches!(
				c,
				RuntimeCall::Registrar(paras_registrar::Call::reserve { .. }) |
					RuntimeCall::Registrar(paras_registrar::Call::register { .. }) |
					RuntimeCall::Utility(pallet_utility::Call::batch { .. }) |
					RuntimeCall::Utility(pallet_utility::Call::batch_all { .. }) |
					RuntimeCall::Utility(pallet_utility::Call::force_batch { .. }) |
					RuntimeCall::Proxy(pallet_proxy::Call::remove_proxy { .. })
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

impl parachains_shared::Config for Runtime {
	type DisabledValidators = Session;
}

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
	pub const ParasUnsignedPriority: TransactionPriority = TransactionPriority::MAX;
}

impl parachains_paras::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::runtime_parachains_paras::WeightInfo<Runtime>;
	type UnsignedPriority = ParasUnsignedPriority;
	type QueueFootprinter = ParaInclusion;
	type NextSessionRotation = Babe;
	type OnNewHead = Registrar;
	type AssignCoretime = CoretimeAssignmentProvider;
}

parameter_types! {
	/// Amount of weight that can be spent per block to service messages.
	///
	/// # WARNING
	///
	/// This is not a good value for para-chains since the `Scheduler` already uses up to 80% block weight.
	pub MessageQueueServiceWeight: Weight = Perbill::from_percent(20) * BlockWeights::get().max_block;
	pub MessageQueueIdleServiceWeight: Weight = Perbill::from_percent(20) * BlockWeights::get().max_block;
	pub const MessageQueueHeapSize: u32 = 65_536;
	pub const MessageQueueMaxStale: u32 = 16;
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
	type IdleMaxServiceWeight = MessageQueueIdleServiceWeight;
}

impl parachains_dmp::Config for Runtime {}

parameter_types! {
	pub const HrmpChannelSizeAndCapacityWithSystemRatio: Percent = Percent::from_percent(100);
}

impl parachains_hrmp::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type ChannelManager = EitherOf<EnsureRoot<Self::AccountId>, GeneralAdmin>;
	type Currency = Balances;
	// Use the `HrmpChannelSizeAndCapacityWithSystemRatio` ratio from the actual active
	// `HostConfiguration` configuration for `hrmp_channel_max_message_size` and
	// `hrmp_channel_max_capacity`.
	type DefaultChannelSizeAndCapacityWithSystem = ActiveConfigHrmpChannelSizeAndCapacityRatio<
		Runtime,
		HrmpChannelSizeAndCapacityWithSystemRatio,
	>;
	type WeightInfo = weights::runtime_parachains_hrmp::WeightInfo<Runtime>;
	type VersionWrapper = XcmPallet;
}

impl parachains_paras_inherent::Config for Runtime {
	type WeightInfo = weights::runtime_parachains_paras_inherent::WeightInfo<Runtime>;
}

impl parachains_scheduler::Config for Runtime {
	// If you change this, make sure the `Assignment` type of the new provider is binary compatible,
	// otherwise provide a migration.
	type AssignmentProvider = CoretimeAssignmentProvider;
}

parameter_types! {
	pub const BrokerId: u32 = system_parachain::BROKER_ID;
	pub const BrokerPalletId: PalletId = PalletId(*b"py/broke");
	pub MaxXcmTransactWeight: Weight = Weight::from_parts(
		250 * WEIGHT_REF_TIME_PER_MICROS,
		20 * WEIGHT_PROOF_SIZE_PER_KB
	);
}

pub struct BrokerPot;
impl Get<InteriorLocation> for BrokerPot {
	fn get() -> InteriorLocation {
		Junction::AccountId32 { network: None, id: BrokerPalletId::get().into_account_truncating() }
			.into()
	}
}

impl coretime::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BrokerId = BrokerId;
	type WeightInfo = weights::runtime_parachains_coretime::WeightInfo<Runtime>;
	type SendXcm = crate::xcm_config::XcmRouter;
	type MaxXcmTransactWeight = MaxXcmTransactWeight;
	type BrokerPotLocation = BrokerPot;
	type AssetTransactor = crate::xcm_config::LocalAssetTransactor;
	type AccountToLocation = xcm_builder::AliasesIntoAccountId32<
		xcm_config::ThisNetwork,
		<Runtime as frame_system::Config>::AccountId,
	>;
}

parameter_types! {
	pub const OnDemandTrafficDefaultValue: FixedU128 = FixedU128::from_u32(1);
	pub const MaxHistoricalRevenue: BlockNumber = 2 * TIMESLICE_PERIOD;
	pub const OnDemandPalletId: PalletId = PalletId(*b"py/ondmd");
}

impl parachains_on_demand::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type TrafficDefaultValue = OnDemandTrafficDefaultValue;
	type WeightInfo = weights::runtime_parachains_on_demand::WeightInfo<Runtime>;
	type MaxHistoricalRevenue = MaxHistoricalRevenue;
	type PalletId = OnDemandPalletId;
}

impl parachains_assigner_coretime::Config for Runtime {}

impl parachains_initializer::Config for Runtime {
	type Randomness = pallet_babe::RandomnessFromOneEpochAgo<Runtime>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type WeightInfo = weights::runtime_parachains_initializer::WeightInfo<Runtime>;
	type CoretimeOnNewSession = Coretime;
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
	pub const ParaDeposit: Balance = 4 * UNITS;
}

impl paras_registrar::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type OnSwap = (Crowdloan, Slots);
	type ParaDeposit = ParaDeposit;
	type DataDepositPerByte = DataDepositPerByte;
	type WeightInfo = weights::polkadot_runtime_common_paras_registrar::WeightInfo<Runtime>;
}

parameter_types! {
	// 6 weeks
	pub LeasePeriod: BlockNumber = prod_or_fast!(6 * WEEKS, 6 * WEEKS, "KSM_LEASE_PERIOD");
}

impl slots::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type Registrar = Registrar;
	type LeasePeriod = LeasePeriod;
	type LeaseOffset = ();
	type ForceOrigin = EitherOf<EnsureRoot<Self::AccountId>, LeaseAdmin>;
	type WeightInfo = weights::polkadot_runtime_common_slots::WeightInfo<Runtime>;
}

parameter_types! {
	pub const CrowdloanId: PalletId = PalletId(*b"py/cfund");
	pub const OldSubmissionDeposit: Balance = 3 * GRAND; // ~ 10 KSM
	pub const MinContribution: Balance = 3_000 * CENTS; // ~ .1 KSM
	pub const RemoveKeysLimit: u32 = 1000;
	// Allow 32 bytes for an additional memo to a crowdloan.
	pub const MaxMemoLength: u8 = 32;
}

impl crowdloan::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = CrowdloanId;
	type SubmissionDeposit = OldSubmissionDeposit;
	type MinContribution = MinContribution;
	type RemoveKeysLimit = RemoveKeysLimit;
	type Registrar = Registrar;
	type Auctioneer = Auctions;
	type MaxMemoLength = MaxMemoLength;
	type WeightInfo = weights::polkadot_runtime_common_crowdloan::WeightInfo<Runtime>;
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
	type WeightInfo = weights::polkadot_runtime_common_auctions::WeightInfo<Runtime>;
}

type NisCounterpartInstance = pallet_balances::Instance2;
impl pallet_balances::Config<NisCounterpartInstance> for Runtime {
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ConstU128<10_000_000_000>; // One KTC cent
	type AccountStore = StorageMapShim<
		pallet_balances::Account<Runtime, NisCounterpartInstance>,
		AccountId,
		pallet_balances::AccountData<u128>,
	>;
	type MaxLocks = ConstU32<4>;
	type MaxReserves = ConstU32<4>;
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = weights::pallet_balances_nis_counterpart::WeightInfo<Runtime>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = ();
	type MaxFreezes = ConstU32<1>;
}

parameter_types! {
	pub const NisBasePeriod: BlockNumber = 7 * DAYS;
	pub const MinBid: Balance = 100 * QUID;
	pub MinReceipt: Perquintill = Perquintill::from_rational(1u64, 10_000_000u64);
	pub const IntakePeriod: BlockNumber = 5 * MINUTES;
	pub MaxIntakeWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 10;
	pub const ThawThrottle: (Perquintill, BlockNumber) = (Perquintill::from_percent(25), 5);
	pub storage NisTarget: Perquintill = Perquintill::zero();
	pub const NisPalletId: PalletId = PalletId(*b"py/nis  ");
}

impl pallet_nis::Config for Runtime {
	type WeightInfo = weights::pallet_nis::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type CurrencyBalance = Balance;
	type FundOrigin = frame_system::EnsureSigned<AccountId>;
	type Counterpart = NisCounterpartBalances;
	type CounterpartAmount = WithMaximumOf<ConstU128<21_000_000_000_000_000_000u128>>;
	type Deficit = (); // Mint
	type IgnoredIssuance = ();
	type Target = NisTarget;
	type PalletId = NisPalletId;
	type QueueCount = ConstU32<500>;
	type MaxQueueLen = ConstU32<1000>;
	type FifoQueueLen = ConstU32<250>;
	type BasePeriod = NisBasePeriod;
	type MinBid = MinBid;
	type MinReceipt = MinReceipt;
	type IntakePeriod = IntakePeriod;
	type MaxIntakeWeight = MaxIntakeWeight;
	type ThawThrottle = ThawThrottle;
	type RuntimeHoldReason = RuntimeHoldReason;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkSetup = ();
}

parameter_types! {
	pub const PoolsPalletId: PalletId = PalletId(*b"py/nopls");
	pub const MaxPointsToBalance: u8 = 10;
}

impl pallet_nomination_pools::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_nomination_pools::WeightInfo<Self>;
	type Currency = Balances;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type RewardCounter = FixedU128;
	type BalanceToU256 = BalanceToU256;
	type U256ToBalance = U256ToBalance;
	type StakeAdapter =
		pallet_nomination_pools::adapter::DelegateStake<Self, Staking, DelegatedStaking>;
	type PostUnbondingPoolsWindow = ConstU32<4>;
	type MaxMetadataLen = ConstU32<256>;
	// we use the same number of allowed unlocking chunks as with staking.
	type MaxUnbonding = <Self as pallet_staking::Config>::MaxUnlockingChunks;
	type PalletId = PoolsPalletId;
	type MaxPointsToBalance = MaxPointsToBalance;
	type AdminOrigin = EitherOf<EnsureRoot<AccountId>, StakingAdmin>;
}

parameter_types! {
	pub const DelegatedStakingPalletId: PalletId = PalletId(*b"py/dlstk");
	pub const SlashRewardFraction: Perbill = Perbill::from_percent(1);
}

impl pallet_delegated_staking::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = DelegatedStakingPalletId;
	type Currency = Balances;
	// slashes are sent to the treasury.
	type OnSlash = ResolveTo<TreasuryAccountId<Self>, Balances>;
	type SlashRewardFraction = SlashRewardFraction;
	type RuntimeHoldReason = RuntimeHoldReason;
	type CoreStaking = Staking;
}

/// The [frame_support::traits::tokens::ConversionFromAssetBalance] implementation provided by the
/// `AssetRate` pallet instance.
///
/// With additional decoration to identify different IDs/locations of native asset and provide a
/// one-to-one balance conversion for them.
pub type AssetRateWithNative = UnityOrOuterConversion<
	ContainsLocationParts<
		FromContains<
			xcm_builder::IsChildSystemParachain<ParaId>,
			xcm_builder::IsParentsOnly<ConstU8<1>>,
		>,
	>,
	AssetRate,
>;

impl pallet_asset_rate::Config for Runtime {
	type WeightInfo = weights::pallet_asset_rate::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type CreateOrigin = EitherOfDiverse<EnsureRoot<AccountId>, Treasurer>;
	type RemoveOrigin = EitherOfDiverse<EnsureRoot<AccountId>, Treasurer>;
	type UpdateOrigin = EitherOfDiverse<EnsureRoot<AccountId>, Treasurer>;
	type Currency = Balances;
	type AssetKind = <Runtime as pallet_treasury::Config>::AssetKind;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = polkadot_runtime_common::impls::benchmarks::AssetRateArguments;
}

construct_runtime! {
	pub enum Runtime
	{
		// Basic stuff.
		System: frame_system = 0,

		// Babe must be before session.
		Babe: pallet_babe = 1,

		Timestamp: pallet_timestamp = 2,
		Indices: pallet_indices = 3,
		Balances: pallet_balances = 4,
		TransactionPayment: pallet_transaction_payment = 33,

		// Consensus support.
		// Authorship must be before session in order to note author in the correct session and era
		// for staking.
		Authorship: pallet_authorship = 5,
		Staking: pallet_staking = 6,
		Offences: pallet_offences = 7,
		Historical: session_historical = 34,

		Session: pallet_session = 8,
		Grandpa: pallet_grandpa = 10,
		AuthorityDiscovery: pallet_authority_discovery = 12,

		// Governance stuff.
		Treasury: pallet_treasury = 18,
		ConvictionVoting: pallet_conviction_voting = 20,
		Referenda: pallet_referenda = 21,
//		pub type FellowshipCollectiveInstance = pallet_ranked_collective::Instance1;
		FellowshipCollective: pallet_ranked_collective::<Instance1> = 22,
//		pub type FellowshipReferendaInstance = pallet_referenda::Instance2;
		FellowshipReferenda: pallet_referenda::<Instance2> = 23,
		Origins: pallet_custom_origins = 43,
		Whitelist: pallet_whitelist = 44,
		Parameters: pallet_parameters = 46,

		// Claims. Usable initially.
		Claims: claims = 19,

		// Utility module.
		Utility: pallet_utility = 24,

		// pallet_identity = 25 (removed post 1.2.4)

		// Society module.
		Society: pallet_society = 26,

		// Social recovery module.
		Recovery: pallet_recovery = 27,

		// Vesting. Usable initially, but removed once all vesting is finished.
		Vesting: pallet_vesting = 28,

		// System scheduler.
		Scheduler: pallet_scheduler = 29,

		// Proxy module. Late addition.
		Proxy: pallet_proxy = 30,

		// Multisig module. Late addition.
		Multisig: pallet_multisig = 31,

		// Preimage registrar.
		Preimage: pallet_preimage = 32,

		// Bounties modules.
		Bounties: pallet_bounties = 35,
		ChildBounties: pallet_child_bounties = 40,

		// Election pallet. Only works with staking, but placed here to maintain indices.
		ElectionProviderMultiPhase: pallet_election_provider_multi_phase = 37,

		// NIS pallet.
		Nis: pallet_nis = 38,
		NisCounterpartBalances: pallet_balances::<Instance2> = 45,

		// Provides a semi-sorted list of nominators for staking.
		VoterList: pallet_bags_list::<Instance1> = 39,

		// nomination pools: extension to staking.
		NominationPools: pallet_nomination_pools = 41,

		// Fast unstake pallet: extension to staking.
		FastUnstake: pallet_fast_unstake = 42,

		// Staking extension for delegation
		DelegatedStaking: pallet_delegated_staking = 47,

		// Parachains pallets. Start indices at 50 to leave room.
		ParachainsOrigin: parachains_origin = 50,
		Configuration: parachains_configuration = 51,
		ParasShared: parachains_shared = 52,
		ParaInclusion: parachains_inclusion = 53,
		ParaInherent: parachains_paras_inherent = 54,
		ParaScheduler: parachains_scheduler = 55,
		Paras: parachains_paras = 56,
		Initializer: parachains_initializer = 57,
		Dmp: parachains_dmp = 58,
		Hrmp: parachains_hrmp = 60,
		ParaSessionInfo: parachains_session_info = 61,
		ParasDisputes: parachains_disputes = 62,
		ParasSlashing: parachains_slashing = 63,
		OnDemandAssignmentProvider: parachains_on_demand = 64,
		CoretimeAssignmentProvider: parachains_assigner_coretime = 65,

		// Parachain Onboarding Pallets. Start indices at 70 to leave room.
		Registrar: paras_registrar = 70,
		Slots: slots = 71,
		Auctions: auctions = 72,
		Crowdloan: crowdloan = 73,
		Coretime: coretime = 74,

		// Pallet for sending XCM.
		XcmPallet: pallet_xcm = 99,

		// Generalized message queue
		MessageQueue: pallet_message_queue = 100,

		// Asset rate.
		AssetRate: pallet_asset_rate = 101,

		// BEEFY Bridges support.
		Beefy: pallet_beefy = 200,
		// MMR leaf construction must be after session in order to have a leaf's next_auth_set
		// refer to block<N>. See issue #160 for details.
		Mmr: pallet_mmr = 201,
		BeefyMmrLeaf: pallet_beefy_mmr = 202,
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
	frame_metadata_hash_extension::CheckMetadataHash<Runtime>,
);

pub struct NominationPoolsMigrationV4OldPallet;
impl Get<Perbill> for NominationPoolsMigrationV4OldPallet {
	fn get() -> Perbill {
		Perbill::from_percent(10)
	}
}

parameter_types! {
	// This is used to limit max pools that migrates in the runtime upgrade. This is set to
	// ~existing_pool_count * 2 to also account for any new pools getting created before the
	// migration is actually executed.
	pub const MaxPoolsToMigrate: u32 = 500;
}

/// All migrations that will run on the next runtime upgrade.
///
/// This contains the combined migrations of the last 10 releases. It allows to skip runtime
/// upgrades in case governance decides to do so. THE ORDER IS IMPORTANT.
pub type Migrations = (migrations::Unreleased, migrations::Permanent);

/// The runtime migrations per release.
#[allow(deprecated, missing_docs)]
pub mod migrations {
	use super::*;

	/// Unreleased migrations. Add new ones here:
	pub type Unreleased = (
		parachains_configuration::migration::v12::MigrateToV12<Runtime>,
		pallet_staking::migrations::v15::MigrateV14ToV15<Runtime>,
		parachains_inclusion::migration::MigrateToV1<Runtime>,
		parachains_on_demand::migration::MigrateV0ToV1<Runtime>,
		restore_corrupted_ledgers::Migrate<Runtime>,
		// Migrate NominationPools to `DelegateStake` adapter. This is an unversioned upgrade.
		pallet_nomination_pools::migration::unversioned::DelegationStakeMigration<
			Runtime,
			MaxPoolsToMigrate,
		>,
	);

	/// Migrations/checks that do not need to be versioned and can run on every update.
	pub type Permanent = (pallet_xcm::migration::MigrateToLatestXcmVersion<Runtime>,);
}

/// Migration to fix current corrupted staking ledgers in Kusama.
///
/// It consists of:
/// * Call into `pallet_staking::Pallet::<T>::restore_ledger` with:
///   * Root origin;
///   * Default `None` paramters.
/// * Forces unstake of recovered ledger if the final restored ledger has higher stake than the
///   stash's free balance.
///
/// The stashes associated with corrupted ledgers that will be "migrated" are set in
/// [`CorruptedStashes`].
///
/// For more details about the corrupt ledgers issue, recovery and which stashes to migrate, check
/// <https://hackmd.io/m_h9DRutSZaUqCwM9tqZ3g?view>.
pub(crate) mod restore_corrupted_ledgers {
	use super::*;

	use frame_support::traits::{Currency, OnRuntimeUpgrade};
	use frame_system::RawOrigin;

	use pallet_staking::WeightInfo;
	use sp_staking::StakingAccount;

	parameter_types! {
		pub CorruptedStashes: Vec<AccountId> = vec![
			// stash account ESGsxFePah1qb96ooTU4QJMxMKUG7NZvgTig3eJxP9f3wLa
			hex_literal::hex!["52559f2c7324385aade778eca4d7837c7492d92ee79b66d6b416373066869d2e"].into(),
			// stash account DggTJdwWEbPS4gERc3SRQL4heQufMeayrZGDpjHNC1iEiui
			hex_literal::hex!["31162f413661f3f5351169299728ab6139725696ac6f98db9665e8b76d73d300"].into(),
			// stash account Du2LiHk1D1kAoaQ8wsx5jiNEG5CNRQEg6xME5iYtGkeQAJP
			hex_literal::hex!["3a8012a52ec2715e711b1811f87684fe6646fc97a276043da7e75cd6a6516d29"].into(),
		];
	}

	pub struct Migrate<T>(sp_std::marker::PhantomData<T>);
	impl<T: pallet_staking::Config> OnRuntimeUpgrade for Migrate<T> {
		fn on_runtime_upgrade() -> Weight {
			let mut total_weight: Weight = Default::default();
			let mut ok_migration = 0;
			let mut err_migration = 0;

			for stash in CorruptedStashes::get().into_iter() {
				let stash_account: T::AccountId = if let Ok(stash_account) =
					T::AccountId::decode(&mut &Into::<[u8; 32]>::into(stash.clone())[..])
				{
					stash_account
				} else {
					log::error!(
						target: LOG_TARGET,
						"migrations::corrupted_ledgers: error converting account {:?}. skipping.",
						stash.clone(),
					);
					err_migration += 1;
					continue
				};

				// restore currupted ledger.
				match pallet_staking::Pallet::<T>::restore_ledger(
					RawOrigin::Root.into(),
					stash_account.clone(),
					None,
					None,
					None,
				) {
					Ok(_) => (), // proceed.
					Err(err) => {
						// note: after first migration run, restoring ledger will fail with
						// `staking::pallet::Error::<T>CannotRestoreLedger`.
						log::error!(
							target: LOG_TARGET,
							"migrations::corrupted_ledgers: error restoring ledger {:?}, unexpected (unless running try-state idempotency round).",
							err
						);
						continue
					},
				};

				// check if restored ledger total is higher than the stash's free balance. If
				// that's the case, force unstake the ledger.
				let weight = if let Ok(ledger) = pallet_staking::Pallet::<T>::ledger(
					StakingAccount::Stash(stash_account.clone()),
				) {
					// force unstake the ledger.
					if ledger.total > T::Currency::free_balance(&stash_account) {
						let slashing_spans = 10; // default slashing spans for migration.
						let _ = pallet_staking::Pallet::<T>::force_unstake(
							RawOrigin::Root.into(),
							stash_account.clone(),
							slashing_spans,
						)
						.inspect_err(|err| {
							log::error!(
								target: LOG_TARGET,
								"migrations::corrupted_ledgers: error force unstaking ledger, unexpected. {:?}",
								err
							);
							err_migration += 1;
						});

						log::info!(
							target: LOG_TARGET,
							"migrations::corrupted_ledgers: ledger of {:?} restored (with force unstake).",
							stash_account,
						);
						ok_migration += 1;

						<T::WeightInfo>::restore_ledger()
							.saturating_add(<T::WeightInfo>::force_unstake(slashing_spans))
					} else {
						log::info!(
							target: LOG_TARGET,
							"migrations::corrupted_ledgers: ledger of {:?} restored.",
							stash,
						);
						ok_migration += 1;

						<T::WeightInfo>::restore_ledger()
					}
				} else {
					log::error!(
						target: LOG_TARGET,
						"migrations::corrupted_ledgers: ledger does not exist, unexpected."
					);
					err_migration += 1;
					<T::WeightInfo>::restore_ledger()
				};

				total_weight.saturating_accrue(weight);
			}

			log::info!(
				target: LOG_TARGET,
				"migrations::corrupted_ledgers: done. success: {}, error: {}",
				ok_migration, err_migration
			);

			total_weight
		}
	}
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
/// The payload being signed in the transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	frame_benchmarking::define_benchmarks!(
		// Polkadot
		[polkadot_runtime_common::auctions, Auctions]
		[polkadot_runtime_common::crowdloan, Crowdloan]
		[polkadot_runtime_common::claims, Claims]
		[polkadot_runtime_common::slots, Slots]
		[polkadot_runtime_common::paras_registrar, Registrar]
		[runtime_parachains::configuration, Configuration]
		[runtime_parachains::hrmp, Hrmp]
		[runtime_parachains::disputes, ParasDisputes]
		[runtime_parachains::disputes::slashing, ParasSlashing]
		[runtime_parachains::inclusion, ParaInclusion]
		[runtime_parachains::initializer, Initializer]
		[runtime_parachains::paras_inherent, ParaInherent]
		[runtime_parachains::paras, Paras]
		[runtime_parachains::on_demand, OnDemandAssignmentProvider]
		[runtime_parachains::coretime, Coretime]
		// Substrate
		[pallet_balances, Native]
		[pallet_balances, NisCounterpart]
		[pallet_bags_list, VoterList]
		[pallet_beefy_mmr, BeefyMmrLeaf]
		[frame_benchmarking::baseline, Baseline::<Runtime>]
		[pallet_bounties, Bounties]
		[pallet_child_bounties, ChildBounties]
		[pallet_conviction_voting, ConvictionVoting]
		[pallet_election_provider_multi_phase, ElectionProviderMultiPhase]
		[frame_election_provider_support, ElectionProviderBench::<Runtime>]
		[pallet_fast_unstake, FastUnstake]
		[pallet_nis, Nis]
		[pallet_indices, Indices]
		[pallet_message_queue, MessageQueue]
		[pallet_multisig, Multisig]
		[pallet_nomination_pools, NominationPoolsBench::<Runtime>]
		[pallet_offences, OffencesBench::<Runtime>]
		[pallet_preimage, Preimage]
		[pallet_proxy, Proxy]
		[pallet_ranked_collective, FellowshipCollective]
		[pallet_recovery, Recovery]
		[pallet_referenda, Referenda]
		[pallet_referenda, FellowshipReferenda]
		[pallet_scheduler, Scheduler]
		[pallet_session, SessionBench::<Runtime>]
		[pallet_society, Society]
		[pallet_staking, Staking]
		[frame_system, SystemBench::<Runtime>]
		[pallet_timestamp, Timestamp]
		[pallet_treasury, Treasury]
		[pallet_utility, Utility]
		[pallet_vesting, Vesting]
		[pallet_whitelist, Whitelist]
		[pallet_asset_rate, AssetRate]
		[pallet_parameters, Parameters]
		// XCM
		[pallet_xcm, PalletXcmExtrinsiscsBenchmark::<Runtime>]
		[pallet_xcm_benchmarks::fungible, pallet_xcm_benchmarks::fungible::Pallet::<Runtime>]
		[pallet_xcm_benchmarks::generic, pallet_xcm_benchmarks::generic::Pallet::<Runtime>]
	);
}

impl Runtime {
	fn impl_experimental_inflation_info() -> InflationInfo {
		use pallet_staking::{ActiveEra, EraPayout, ErasTotalStake};
		let (staked, _start) = ActiveEra::<Runtime>::get()
			.map(|ae| (ErasTotalStake::<Runtime>::get(ae.index), ae.start.unwrap_or(0)))
			.unwrap_or((0, 0));
		let stake_able_issuance = Nis::issuance().other;

		let ideal_staking_rate = dynamic_params::inflation::IdealStake::get();
		let inflation = if dynamic_params::inflation::UseAuctionSlots::get() {
			let auctioned_slots = parachains_paras::Parachains::<Runtime>::get()
				.into_iter()
				// All active para-ids that do not belong to a system chain is the number of
				// parachains that we should take into account for inflation.
				.filter(|i| *i >= LOWEST_PUBLIC_ID)
				.count() as u64;
			ideal_staking_rate
				.saturating_sub(Perquintill::from_rational(auctioned_slots.min(60), 200u64))
		} else {
			ideal_staking_rate
		};

		// We assume un-delayed 6h eras.
		let era_duration = 6 * (HOURS as Moment) * MILLISECS_PER_BLOCK;
		let next_mint = <Self as pallet_staking::Config>::EraPayout::era_payout(
			staked,
			stake_able_issuance,
			era_duration,
		);

		InflationInfo { inflation, next_mint }
	}
}

sp_api::impl_runtime_apis! {
	impl relay_common::apis::Inflation<Block> for Runtime {
		fn experimental_inflation_prediction_info() -> InflationInfo {
			Runtime::impl_experimental_inflation_info()
		}
	}

	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
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

	#[api_version(11)]
	impl polkadot_primitives::runtime_api::ParachainHost<Block> for Runtime {
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
			outputs: polkadot_primitives::CandidateCommitments,
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
			#[allow(deprecated)]
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
			stmt: polkadot_primitives::PvfCheckStatement,
			signature: polkadot_primitives::ValidatorSignature,
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
			use codec::Encode;

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

		fn minimum_backing_votes() -> u32 {
			parachains_runtime_api_impl::minimum_backing_votes::<Runtime>()
		}

		fn para_backing_state(para_id: ParaId) -> Option<polkadot_primitives::async_backing::BackingState> {
			parachains_runtime_api_impl::backing_state::<Runtime>(para_id)
		}

		fn async_backing_params() -> polkadot_primitives::AsyncBackingParams {
			parachains_runtime_api_impl::async_backing_params::<Runtime>()
		}

		fn disabled_validators() -> Vec<ValidatorIndex> {
			parachains_runtime_api_impl::disabled_validators::<Runtime>()
		}

		fn node_features() -> NodeFeatures {
			parachains_runtime_api_impl::node_features::<Runtime>()
		}

		fn approval_voting_params() -> ApprovalVotingParams {
			parachains_runtime_api_impl::approval_voting_params::<Runtime>()
		}

		fn claim_queue() -> BTreeMap<CoreIndex, VecDeque<ParaId>> {
			parachains_vstaging_api_impl::claim_queue::<Runtime>()
		}

		fn candidates_pending_availability(para_id: ParaId) -> Vec<CommittedCandidateReceipt<Hash>> {
			parachains_vstaging_api_impl::candidates_pending_availability::<Runtime>(para_id)
		}
	}

	impl beefy_primitives::BeefyApi<Block, BeefyId> for Runtime {
		fn beefy_genesis() -> Option<BlockNumber> {
			pallet_beefy::GenesisBlock::<Runtime>::get()
		}

		fn validator_set() -> Option<beefy_primitives::ValidatorSet<BeefyId>> {
			Beefy::validator_set()
		}

		fn submit_report_double_voting_unsigned_extrinsic(
			equivocation_proof:
				beefy_primitives::DoubleVotingProof<BlockNumber, BeefyId, BeefySignature>,
			key_owner_proof: OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Beefy::submit_unsigned_double_voting_report(
				equivocation_proof,
				key_owner_proof,
			)
		}

		fn submit_report_fork_voting_unsigned_extrinsic(
			equivocation_proof: beefy_primitives::ForkVotingProof<Header, BeefyId, OpaqueValue>,
			key_owner_proof: OpaqueKeyOwnershipProof,
		) -> Option<()> {
			Beefy::submit_unsigned_fork_voting_report(
				equivocation_proof.try_into()?,
				key_owner_proof.decode()?,
			)
		}

		fn submit_report_future_block_voting_unsigned_extrinsic(
			equivocation_proof: beefy_primitives::FutureBlockVotingProof<BlockNumber,BeefyId> ,
			key_owner_proof: OpaqueKeyOwnershipProof,
		) -> Option<()> {
			Beefy::submit_unsigned_future_block_voting_report(
				equivocation_proof,
				key_owner_proof.decode()?,
			)
		}

		fn generate_key_ownership_proof(
			_set_id: beefy_primitives::ValidatorSetId,
			authority_id: BeefyId,
		) -> Option<beefy_primitives::OpaqueKeyOwnershipProof> {
			use codec::Encode;

			Historical::prove((beefy_primitives::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(beefy_primitives::OpaqueKeyOwnershipProof::new)
		}

		fn generate_ancestry_proof(
			prev_block_number: BlockNumber,
			best_known_block_number: Option<BlockNumber>,
		) -> Option<sp_runtime::OpaqueValue> {
			Mmr::generate_ancestry_proof(prev_block_number, best_known_block_number)
				.map(|p| p.encode())
				.map(OpaqueKeyOwnershipProof::new)
				.ok()
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
		) -> Result<(Vec<mmr::EncodableOpaqueLeaf>, mmr::LeafProof<mmr::Hash>), mmr::Error> {
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

		fn verify_proof(leaves: Vec<mmr::EncodableOpaqueLeaf>, proof: mmr::LeafProof<mmr::Hash>)
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
			proof: mmr::LeafProof<mmr::Hash>
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
			use codec::Encode;

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
			use codec::Encode;

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

	impl xcm_runtime_apis::fees::XcmPaymentApi<Block> for Runtime {
		fn query_acceptable_payment_assets(xcm_version: xcm::Version) -> Result<Vec<VersionedAssetId>, XcmPaymentApiError> {
			let acceptable_assets = vec![AssetId(xcm_config::TokenLocation::get())];
			XcmPallet::query_acceptable_payment_assets(xcm_version, acceptable_assets)
		}

		fn query_weight_to_asset_fee(weight: Weight, asset: VersionedAssetId) -> Result<u128, XcmPaymentApiError> {
			match asset.try_as::<AssetId>() {
				Ok(asset_id) if asset_id.0 == xcm_config::TokenLocation::get() => {
					// for native token
					Ok(WeightToFee::weight_to_fee(&weight))
				},
				Ok(asset_id) => {
					log::trace!(target: "xcm::xcm_runtime_apis", "query_weight_to_asset_fee - unhandled asset_id: {asset_id:?}!");
					Err(XcmPaymentApiError::AssetNotFound)
				},
				Err(_) => {
					log::trace!(target: "xcm::xcm_runtime_apis", "query_weight_to_asset_fee - failed to convert asset: {asset:?}!");
					Err(XcmPaymentApiError::VersionedConversionFailed)
				}
			}
		}

		fn query_xcm_weight(message: VersionedXcm<()>) -> Result<Weight, XcmPaymentApiError> {
			XcmPallet::query_xcm_weight(message)
		}

		fn query_delivery_fees(destination: VersionedLocation, message: VersionedXcm<()>) -> Result<VersionedAssets, XcmPaymentApiError> {
			XcmPallet::query_delivery_fees(destination, message)
		}
	}

	impl xcm_runtime_apis::dry_run::DryRunApi<Block, RuntimeCall, RuntimeEvent, OriginCaller> for Runtime {
		fn dry_run_call(origin: OriginCaller, call: RuntimeCall) -> Result<CallDryRunEffects<RuntimeEvent>, XcmDryRunApiError> {
			XcmPallet::dry_run_call::<Runtime, xcm_config::XcmRouter, OriginCaller, RuntimeCall>(origin, call)
		}

		fn dry_run_xcm(origin_location: VersionedLocation, xcm: VersionedXcm<RuntimeCall>) -> Result<XcmDryRunEffects<RuntimeEvent>, XcmDryRunApiError> {
			XcmPallet::dry_run_xcm::<Runtime, xcm_config::XcmRouter, RuntimeCall, xcm_config::XcmConfig>(origin_location, xcm)
		}
	}

	impl xcm_runtime_apis::conversions::LocationToAccountApi<Block, AccountId> for Runtime {
		fn convert_location(location: VersionedLocation) -> Result<
			AccountId,
			xcm_runtime_apis::conversions::Error
		> {
			xcm_runtime_apis::conversions::LocationToAccountHelper::<
				AccountId,
				xcm_config::SovereignAccountOf,
			>::convert_location(location)
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

		fn pool_pending_slash(pool_id: pallet_nomination_pools::PoolId) -> Balance {
			NominationPools::api_pool_pending_slash(pool_id)
		}

		fn member_pending_slash(member: AccountId) -> Balance {
			NominationPools::api_member_pending_slash(member)
		}

		fn pool_needs_delegate_migration(pool_id: pallet_nomination_pools::PoolId) -> bool {
			NominationPools::api_pool_needs_delegate_migration(pool_id)
		}

		fn member_needs_delegate_migration(member: AccountId) -> bool {
			NominationPools::api_member_needs_delegate_migration(member)
		}

		fn member_total_balance(who: AccountId) -> Balance {
			NominationPools::api_member_total_balance(who)
		}

		fn pool_balance(pool_id: pallet_nomination_pools::PoolId) -> Balance {
			NominationPools::api_pool_balance(pool_id)
		}
	}

	impl pallet_staking_runtime_api::StakingApi<Block, Balance, AccountId> for Runtime {
		fn nominations_quota(balance: Balance) -> u32 {
			Staking::api_nominations_quota(balance)
		}

		fn eras_stakers_page_count(era: sp_staking::EraIndex, account: AccountId) -> sp_staking::Page {
			Staking::api_eras_stakers_page_count(era, account)
		}

		fn pending_rewards(era: sp_staking::EraIndex, account: AccountId) -> bool {
			Staking::api_pending_rewards(era, account)
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

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			log::info!("try-runtime::on_runtime_upgrade kusama.");
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
			use pallet_xcm::benchmarking::Pallet as PalletXcmExtrinsiscsBenchmark;
			use frame_system_benchmarking::Pallet as SystemBench;
			use pallet_nomination_pools_benchmarking::Pallet as NominationPoolsBench;
			use frame_benchmarking::baseline::Pallet as Baseline;

			// Benchmark files generated for `Balances/NisCounterpartBalances` instances are by default
			// `pallet_balances_balances.rs / pallet_balances_nis_counterpart_balances`, which is not really nice,
			// so with this redefinition we can change names to nicer:
			// `pallet_balances_native.rs / pallet_balances_nis_counterpart.rs`.
			type Native = pallet_balances::Pallet::<Runtime, ()>;
			type NisCounterpart = pallet_balances::Pallet::<Runtime, NisCounterpartInstance>;

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
			use frame_system_benchmarking::Pallet as SystemBench;
			use pallet_nomination_pools_benchmarking::Pallet as NominationPoolsBench;
			use frame_benchmarking::baseline::Pallet as Baseline;
			use xcm::latest::prelude::*;
			use xcm_config::{
				LocalCheckAccount, SovereignAccountOf, AssetHubLocation, TokenLocation, XcmConfig,
			};

			impl pallet_session_benchmarking::Config for Runtime {}
			impl pallet_offences_benchmarking::Config for Runtime {}
			impl pallet_election_provider_support_benchmarking::Config for Runtime {}
			impl frame_system_benchmarking::Config for Runtime {}
			impl frame_benchmarking::baseline::Config for Runtime {}
			impl pallet_nomination_pools_benchmarking::Config for Runtime {}
			impl runtime_parachains::disputes::slashing::benchmarking::Config for Runtime {}

			parameter_types! {
				pub ExistentialDepositAsset: Option<Asset> = Some((
					TokenLocation::get(),
					ExistentialDeposit::get()
				).into());
				pub AssetHubParaId: ParaId = kusama_runtime_constants::system_parachain::ASSET_HUB_ID.into();
				pub const RandomParaId: ParaId = ParaId::new(43211234);
			}

			use pallet_xcm::benchmarking::Pallet as PalletXcmExtrinsiscsBenchmark;
			impl pallet_xcm::benchmarking::Config for Runtime {
				type DeliveryHelper = (
					polkadot_runtime_common::xcm_sender::ToParachainDeliveryHelper<
						XcmConfig,
						ExistentialDepositAsset,
						xcm_config::PriceForChildParachainDelivery,
						AssetHubParaId,
						(),
					>,
					polkadot_runtime_common::xcm_sender::ToParachainDeliveryHelper<
						XcmConfig,
						ExistentialDepositAsset,
						xcm_config::PriceForChildParachainDelivery,
						RandomParaId,
						(),
					>
				);

				fn reachable_dest() -> Option<Location> {
					Some(crate::xcm_config::AssetHubLocation::get())
				}

				fn teleportable_asset_and_dest() -> Option<(Asset, Location)> {
					// Relay/native token can be teleported to/from AH.
					Some((
						Asset { fun: Fungible(ExistentialDeposit::get()), id: AssetId(Here.into()) },
						crate::xcm_config::AssetHubLocation::get(),
					))
				}

				fn reserve_transferable_asset_and_dest() -> Option<(Asset, Location)> {
					// Relay can reserve transfer native token to some random parachain.
					Some((
						Asset {
							fun: Fungible(ExistentialDeposit::get()),
							id: AssetId(Here.into())
						},
						Parachain(RandomParaId::get().into()).into(),
					))
				}

				fn set_up_complex_asset_transfer(
				) -> Option<(Assets, u32, Location, Box<dyn FnOnce()>)> {
					// Relay supports only native token, either reserve transfer it to non-system parachains,
					// or teleport it to system parachain. Use the teleport case for benchmarking as it's
					// slightly heavier.
					// Relay/native token can be teleported to/from AH.
					let native_location = Here.into();
					let dest = crate::xcm_config::AssetHubLocation::get();
					pallet_xcm::benchmarking::helpers::native_teleport_as_asset_transfer::<Runtime>(
						native_location,
						dest
					)
				}

				fn get_asset() -> Asset {
					Asset {
						id: AssetId(Location::here()),
						fun: Fungible(ExistentialDeposit::get()),
					}
				}
			}

			impl pallet_xcm_benchmarks::Config for Runtime {
				type XcmConfig = XcmConfig;
				type AccountIdConverter = SovereignAccountOf;
				type DeliveryHelper = polkadot_runtime_common::xcm_sender::ToParachainDeliveryHelper<
					XcmConfig,
					ExistentialDepositAsset,
					xcm_config::PriceForChildParachainDelivery,
					AssetHubParaId,
					(),
				>;
				fn valid_destination() -> Result<Location, BenchmarkError> {
					Ok(AssetHubLocation::get())
				}
				fn worst_case_holding(_depositable_count: u32) -> Assets {
					// Kusama only knows about KSM.
					vec![Asset{
						id: AssetId(TokenLocation::get()),
						fun: Fungible(1_000_000 * UNITS),
					}].into()
				}
			}

			parameter_types! {
				pub TrustedTeleporter: Option<(Location, Asset)> = Some((
					AssetHubLocation::get(),
					Asset { fun: Fungible(1 * UNITS), id: AssetId(TokenLocation::get()) },
				));
				pub const TrustedReserve: Option<(Location, Asset)> = None;
			}

			impl pallet_xcm_benchmarks::fungible::Config for Runtime {
				type TransactAsset = Balances;

				type CheckedAccount = LocalCheckAccount;
				type TrustedTeleporter = TrustedTeleporter;
				type TrustedReserve = TrustedReserve;

				fn get_asset() -> Asset {
					Asset {
						id: AssetId(TokenLocation::get()),
						fun: Fungible(1 * UNITS),
					}
				}
			}

			impl pallet_xcm_benchmarks::generic::Config for Runtime {
				type TransactAsset = Balances;
				type RuntimeCall = RuntimeCall;

				fn worst_case_response() -> (u64, Response) {
					(0u64, Response::Version(Default::default()))
				}

				fn worst_case_asset_exchange() -> Result<(Assets, Assets), BenchmarkError> {
					// Kusama doesn't support asset exchanges
					Err(BenchmarkError::Skip)
				}

				fn universal_alias() -> Result<(Location, Junction), BenchmarkError> {
					// The XCM executor of Kusama doesn't have a configured `UniversalAliases`
					Err(BenchmarkError::Skip)
				}

				fn transact_origin_and_runtime_call() -> Result<(Location, RuntimeCall), BenchmarkError> {
					Ok((AssetHubLocation::get(), frame_system::Call::remark_with_event { remark: vec![] }.into()))
				}

				fn subscribe_origin() -> Result<Location, BenchmarkError> {
					Ok(AssetHubLocation::get())
				}

				fn claimable_asset() -> Result<(Location, Location, Assets), BenchmarkError> {
					let origin = AssetHubLocation::get();
					let assets: Assets = (AssetId(TokenLocation::get()), 1_000 * UNITS).into();
					let ticket = Location { parents: 0, interior: Here };
					Ok((origin, ticket, assets))
				}

				fn fee_asset() -> Result<Asset, BenchmarkError> {
					Ok(Asset {
						id: AssetId(TokenLocation::get()),
						fun: Fungible(1_000_000 * UNITS),
					})
				}

				fn unlockable_asset() -> Result<(Location, Location, Asset), BenchmarkError> {
					// Kusama doesn't support asset locking
					Err(BenchmarkError::Skip)
				}

				fn export_message_origin_and_destination(
				) -> Result<(Location, NetworkId, InteriorLocation), BenchmarkError> {
					// Kusama doesn't support exporting messages
					Err(BenchmarkError::Skip)
				}

				fn alias_origin() -> Result<(Location, Location), BenchmarkError> {
					// The XCM executor of Kusama doesn't have a configured `Aliasers`
					Err(BenchmarkError::Skip)
				}
			}

			type Native = pallet_balances::Pallet::<Runtime, ()>;
			type NisCounterpart = pallet_balances::Pallet::<Runtime, NisCounterpartInstance>;

			let mut whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();
			let treasury_key = frame_system::Account::<Runtime>::hashed_key_for(Treasury::account_id());
			whitelist.push(treasury_key.to_vec().into());

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);

			add_benchmarks!(params, batches);

			Ok(batches)
		}
	}
}

#[cfg(test)]
mod fees_tests {
	use super::*;
	use sp_runtime::assert_eq_error_rate;

	#[test]
	fn signed_deposit_is_sensible() {
		// ensure this number does not change, or that it is checked after each change.
		// a 1 MB solution should need around 0.16 KSM deposit
		let deposit = SignedFixedDeposit::get() + (SignedDepositByte::get() * 1024 * 1024);
		assert_eq_error_rate!(deposit, UNITS * 167 / 100, UNITS / 100);
	}
}

#[cfg(test)]
mod multiplier_tests {
	use super::*;
	use frame_support::{
		dispatch::DispatchInfo,
		traits::{OnFinalize, PalletInfoAccess},
	};
	use polkadot_runtime_common::{MinimumMultiplier, TargetBlockFullness};
	use separator::Separatable;
	use sp_runtime::traits::Convert;

	fn run_with_system_weight<F>(w: Weight, mut assertions: F)
	where
		F: FnMut(),
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
			1000,
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
		Builder, Mode, OfflineConfig, OnlineConfig, SnapshotConfig, Transport,
	};
	use std::env::var;

	#[tokio::test]
	async fn run_migrations() {
		if var("RUN_MIGRATION_TESTS").is_err() {
			return
		}

		sp_tracing::try_init_simple();
		let transport: Transport =
			var("WS").unwrap_or("wss://kusama-rpc.polkadot.io:443".to_string()).into();
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
		ext.execute_with(|| Runtime::on_runtime_upgrade(UpgradeCheckSelect::PreAndPost));
	}

	#[tokio::test]
	async fn next_inflation() {
		use hex_literal::hex;
		sp_tracing::try_init_simple();
		let transport: Transport =
			var("WS").unwrap_or("wss://rpc.dotters.network/kusama".to_string()).into();
		let mut ext = Builder::<Block>::default()
			.mode(Mode::Online(OnlineConfig {
				transport,
				hashed_prefixes: vec![
					// entire nis pallet
					hex!("928fa8b8d92aa31f47ed74f188a43f70").to_vec(),
					// staking eras total stake
					hex!("5f3e4907f716ac89b6347d15ececedcaa141c4fe67c2d11f4a10c6aca7a79a04")
						.to_vec(),
				],
				hashed_keys: vec![
					// staking active era
					hex!("5f3e4907f716ac89b6347d15ececedca487df464e44a534ba6b0cbb32407b587")
						.to_vec(),
					// balances ti
					hex!("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80")
						.to_vec(),
					// timestamp now
					hex!("f0c365c3cf59d671eb72da0e7a4113c49f1f0515f462cdcf84e0f1d6045dfcbb")
						.to_vec(),
					// para-ids
					hex!("cd710b30bd2eab0352ddcc26417aa1940b76934f4cc08dee01012d059e1b83ee")
						.to_vec(),
				],
				..Default::default()
			}))
			.build()
			.await
			.unwrap();
		ext.execute_with(|| {
			use pallet_staking::EraPayout;
			let (total_staked, started) = pallet_staking::ActiveEra::<Runtime>::get()
				.map(|ae| {
					(pallet_staking::ErasTotalStake::<Runtime>::get(ae.index), ae.start.unwrap())
				})
				.unwrap();
			let total_issuance = Nis::issuance().other;
			let _real_era_duration_millis =
				pallet_timestamp::Now::<Runtime>::get().saturating_sub(started);
			// 6h in milliseconds
			let average_era_duration_millis = 6 * 60 * 60 * 1000;
			let (staking, leftover) = <Runtime as pallet_staking::Config>::EraPayout::era_payout(
				total_staked,
				total_issuance,
				average_era_duration_millis,
			);
			use ss58_registry::TokenRegistry;
			let token: ss58_registry::Token = TokenRegistry::Ksm.into();

			log::info!(target: LOG_TARGET, "total-staked = {:?}", token.amount(total_staked));
			log::info!(target: LOG_TARGET, "total-issuance = {:?}", token.amount(total_issuance));
			log::info!(target: LOG_TARGET, "staking-rate = {:?}", Perquintill::from_rational(total_staked, total_issuance));
			log::info!(target: LOG_TARGET, "era-duration = {:?}", average_era_duration_millis);
			log::info!(target: LOG_TARGET, "min-inflation = {:?}", dynamic_params::inflation::MinInflation::get());
			log::info!(target: LOG_TARGET, "max-inflation = {:?}", dynamic_params::inflation::MaxInflation::get());
			log::info!(target: LOG_TARGET, "falloff = {:?}", dynamic_params::inflation::Falloff::get());
			log::info!(target: LOG_TARGET, "useAuctionSlots = {:?}", dynamic_params::inflation::UseAuctionSlots::get());
			log::info!(target: LOG_TARGET, "idealStake = {:?}", dynamic_params::inflation::IdealStake::get());
			log::info!(target: LOG_TARGET, "maxStakingRewards = {:?}", pallet_staking::MaxStakedRewards::<Runtime>::get());
			log::info!(target: LOG_TARGET, "💰 Inflation ==> staking = {:?} / leftover = {:?}", token.amount(staking), token.amount(leftover));
		});
	}

	#[tokio::test]
	#[ignore = "this test is meant to be executed manually"]
	async fn try_fast_unstake_all() {
		sp_tracing::try_init_simple();
		let transport: Transport =
			var("WS").unwrap_or("wss://kusama-rpc.polkadot.io:443".to_string()).into();
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
			polkadot_runtime_common::try_runtime::migrate_all_inactive_nominators::<Runtime>()
		});
	}
}
