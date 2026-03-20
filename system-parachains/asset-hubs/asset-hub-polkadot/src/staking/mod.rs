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

//! Staking related config of the Asset Hub.
//!
//! The large pallets have their config in a sub-module, the smaller ones are defined here.

pub mod bags_thresholds;
pub mod nom_pools;
pub mod stepped_curve;

use crate::{governance::StakingAdmin, *};
use codec::Encode;
use cumulus_pallet_parachain_system::RelaychainDataProvider;
use frame_election_provider_support::{ElectionDataProvider, SequentialPhragmen};
use frame_support::{
	pallet_prelude::OptionQuery, storage_alias, traits::tokens::imbalance::ResolveTo, BoundedVec,
};
use pallet_election_provider_multi_block::{self as multi_block, SolutionAccuracyOf};
use pallet_staking_async::UseValidatorsMap;
use pallet_staking_async_rc_client as rc_client;
use sp_arithmetic::FixedU128;
use sp_runtime::{
	generic,
	traits::{BlockNumberProvider, Convert, OpaqueKeys},
	transaction_validity::TransactionPriority,
	FixedPointNumber, Perquintill, SaturatedConversion,
};
use sp_staking::SessionIndex;
use stepped_curve::*;
use system_parachains_common::apis::InflationInfo;
use xcm::v5::prelude::*;

// stuff aliased to `parameters` pallet.
use dynamic_params::staking_election::{
	MaxElectingVoters, MaxEraDuration, MaxSignedSubmissions, MinerPages, SignedPhase,
	TargetSnapshotPerBlock, UnsignedPhase,
};

parameter_types! {
	/// Number of election pages that we operate upon. 32 * 6s block = 192s = 3.2min snapshots
	pub Pages: u32 = 32;

	/// Verify all pages.
	pub SignedValidationPhase: u32 = prod_or_fast!(
		Pages::get() * crate::dynamic_params::staking_election::MaxSignedSubmissions::get(),
		Pages::get()
	);

	/// Allow OCW miner to at most run 4 times in the entirety of the Unsigned Phase.
	pub OffchainRepeat: u32 = UnsignedPhase::get() / 4;

	/// Number of nominators per page of the snapshot, and consequently number of backers in the solution.
	pub VoterSnapshotPerBlock: u32 = MaxElectingVoters::get().div_ceil(Pages::get());

	/// Maximum number of validators that we may want to elect. 1000 is the end target.
	pub const MaxValidatorSet: u32 = 1000;

	/// In each page, we may observe up to all of the validators.
	pub MaxWinnersPerPage: u32 = MaxValidatorSet::get();

	/// In each page of the election, we allow up to all of the nominators of that page to be present.
	///
	/// Translates to "no limit" as of now.
	pub MaxBackersPerWinner: u32 = VoterSnapshotPerBlock::get();

	/// Total number of backers per winner across all pages.
	///
	/// Translates to "no limit" as of now.
	pub MaxBackersPerWinnerFinal: u32 = MaxElectingVoters::get();

	/// Size of the exposures. This should be small enough to make the reward payouts feasible.
	pub MaxExposurePageSize: u32 = 512;
}

frame_election_provider_support::generate_solution_type!(
	#[compact]
	pub struct NposCompactSolution16::<
		// allows up to 4bn nominators
		VoterIndex = u32,
		// allows up to 64k validators
		TargetIndex = u16,
		Accuracy = sp_runtime::PerU16,
		MaxVoters = VoterSnapshotPerBlock,
	>(16)
);

parameter_types! {
	pub const BagThresholds: &'static [u64] = &bags_thresholds::THRESHOLDS;
}

/// We don't want to do any auto-rebags in pallet-bags while the migration is not started or
/// ongoing.
pub struct RebagIffMigrationDone;
impl sp_runtime::traits::Get<u32> for RebagIffMigrationDone {
	fn get() -> u32 {
		if cfg!(feature = "runtime-benchmarks") ||
			pallet_ah_migrator::MigrationEndBlock::<Runtime>::get()
				.is_some_and(|n| frame_system::Pallet::<Runtime>::block_number() > n + 1)
		{
			10
		} else {
			0
		}
	}
}

type VoterBagsListInstance = pallet_bags_list::Instance1;
impl pallet_bags_list::Config<VoterBagsListInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ScoreProvider = Staking;
	type BagThresholds = BagThresholds;
	type Score = sp_npos_elections::VoteWeight;
	type MaxAutoRebagPerBlock = RebagIffMigrationDone;
	type WeightInfo = weights::pallet_bags_list::WeightInfo<Runtime>;
}

parameter_types! {
	pub const DelegatedStakingPalletId: PalletId = PalletId(*b"py/dlstk");
	pub const SlashRewardFraction: Perbill = Perbill::from_percent(1);
	pub const DapPalletId: PalletId = PalletId(*b"dap/buff");
}

impl pallet_delegated_staking::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = DelegatedStakingPalletId;
	type Currency = Balances;
	type OnSlash = Dap;
	type SlashRewardFraction = SlashRewardFraction;
	type RuntimeHoldReason = RuntimeHoldReason;
	type CoreStaking = Staking;
}

impl pallet_dap::Config for Runtime {
	type Currency = Balances;
	type PalletId = DapPalletId;
}

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub BenchElectionBounds: frame_election_provider_support::bounds::ElectionBounds =
		frame_election_provider_support::bounds::ElectionBoundsBuilder::default().build();
}

#[cfg(feature = "runtime-benchmarks")]
pub struct OnChainConfig;

#[cfg(feature = "runtime-benchmarks")]
impl frame_election_provider_support::onchain::Config for OnChainConfig {
	// unbounded
	type Bounds = BenchElectionBounds;
	// We should not need sorting, as our bounds are large enough for the number of
	// nominators/validators in this test setup.
	type Sort = ConstBool<false>;
	type DataProvider = Staking;
	type MaxBackersPerWinner = MaxBackersPerWinner;
	type MaxWinnersPerPage = MaxWinnersPerPage;
	type Solver = frame_election_provider_support::SequentialPhragmen<AccountId, Perbill>;
	type System = Runtime;
	type WeightInfo = ();
}

impl multi_block::Config for Runtime {
	type Pages = Pages;
	type UnsignedPhase = UnsignedPhase;
	type SignedPhase = SignedPhase;
	type SignedValidationPhase = SignedValidationPhase;
	type VoterSnapshotPerBlock = VoterSnapshotPerBlock;
	type TargetSnapshotPerBlock = TargetSnapshotPerBlock;
	type AdminOrigin = EitherOfDiverse<EnsureRoot<AccountId>, StakingAdmin>;
	type ManagerOrigin = EitherOfDiverse<EnsureRoot<AccountId>, StakingAdmin>;
	type DataProvider = Staking;
	type MinerConfig = Self;
	type Verifier = MultiBlockElectionVerifier;
	// we chill and do nothing in the fallback.
	#[cfg(not(feature = "runtime-benchmarks"))]
	type Fallback = multi_block::Continue<Self>;
	#[cfg(feature = "runtime-benchmarks")]
	type Fallback = frame_election_provider_support::onchain::OnChainExecution<OnChainConfig>;
	// Revert back to signed phase if nothing is submitted and queued, so we prolong the election.
	type AreWeDone = multi_block::RevertToSignedIfNotQueuedOf<Self>;
	type OnRoundRotation = multi_block::CleanRound<Self>;
	type WeightInfo = weights::pallet_election_provider_multi_block::WeightInfo<Runtime>;
}

impl multi_block::verifier::Config for Runtime {
	type MaxWinnersPerPage = MaxWinnersPerPage;
	type MaxBackersPerWinner = MaxBackersPerWinner;
	type MaxBackersPerWinnerFinal = MaxBackersPerWinnerFinal;
	type SolutionDataProvider = MultiBlockElectionSigned;
	type SolutionImprovementThreshold = ();
	type WeightInfo = weights::pallet_election_provider_multi_block_verifier::WeightInfo<Runtime>;
}

parameter_types! {
	/// Initial base deposit for signed NPoS solution submissions
	pub InitialBaseDeposit: Balance = 100 * UNITS;
}

/// ## Example
/// ```
/// use asset_hub_polkadot_runtime::staking::{GeometricDeposit, InitialBaseDeposit};
/// use pallet_election_provider_multi_block::signed::CalculateBaseDeposit;
/// use polkadot_runtime_constants::currency::UNITS;
///
/// // Base deposit
/// assert_eq!(GeometricDeposit::calculate_base_deposit(0), InitialBaseDeposit::get());
/// assert_eq!(GeometricDeposit::calculate_base_deposit(1), 2 * InitialBaseDeposit::get());
/// assert_eq!(GeometricDeposit::calculate_base_deposit(2), 4 * InitialBaseDeposit::get());
/// // and so on
///
/// // Full 16 page deposit, to be paid on top of the above base
/// sp_io::TestExternalities::default().execute_with(|| {
/// let deposit = asset_hub_polkadot_runtime::staking::SignedDepositPerPage::get() * 16;
///     assert_eq!(deposit, 10_6_368_000_000); // around 10.6 DOTs
/// })
/// ```
pub struct GeometricDeposit;
impl multi_block::signed::CalculateBaseDeposit<Balance> for GeometricDeposit {
	fn calculate_base_deposit(existing_submitters: usize) -> Balance {
		let start: Balance = InitialBaseDeposit::get();
		let common: Balance = 2;
		start.saturating_mul(common.saturating_pow(existing_submitters as u32))
	}
}

// Parameters only regarding signed submission deposits/rewards.
parameter_types! {
	pub SignedDepositPerPage: Balance = system_para_deposit(1, NposCompactSolution16::max_encoded_len() as u32);
	/// Bailing is rather disincentivized, as it can allow attackers to submit bad solutions, but
	/// get away with it last minute. We don't refund any deposit.
	pub BailoutGraceRatio: Perbill = Perbill::from_percent(0);
	/// Invulnerable miners will pay this deposit only.
	pub InvulnerableFixedDeposit: Balance = 10 * UNITS;
	/// Being ejected is already paid for by the new submitter replacing you; no need to charge deposit.
	pub EjectGraceRatio: Perbill = Perbill::from_percent(100);
	/// 5 DOT as the reward for the best signed submission.
	pub RewardBase: Balance = UNITS * 5;
}

impl multi_block::signed::Config for Runtime {
	type Currency = Balances;
	type BailoutGraceRatio = BailoutGraceRatio;
	type EjectGraceRatio = EjectGraceRatio;
	type DepositBase = GeometricDeposit;
	type DepositPerPage = SignedDepositPerPage;
	type InvulnerableDeposit = InvulnerableFixedDeposit;
	type RewardBase = RewardBase;
	type MaxSubmissions = MaxSignedSubmissions;
	type EstimateCallFee = TransactionPayment;
	type WeightInfo = weights::pallet_election_provider_multi_block_signed::WeightInfo<Runtime>;
}

parameter_types! {
	/// Priority of the offchain miner transactions.
	pub MinerTxPriority: TransactionPriority = TransactionPriority::MAX / 2;
}

impl multi_block::unsigned::Config for Runtime {
	type MinerPages = MinerPages;
	type OffchainStorage = ConstBool<true>;
	type OffchainSolver = SequentialPhragmen<AccountId, SolutionAccuracyOf<Runtime>, ()>;
	type MinerTxPriority = MinerTxPriority;
	type OffchainRepeat = OffchainRepeat;
	type WeightInfo = weights::pallet_election_provider_multi_block_unsigned::WeightInfo<Runtime>;
}

parameter_types! {
	/// Miner transaction can fill up to 75% of the block size.
	pub MinerMaxLength: u32 = Perbill::from_rational(75u32, 100) *
		*RuntimeBlockLength::get()
		.max
		.get(DispatchClass::Normal);
}

impl multi_block::unsigned::miner::MinerConfig for Runtime {
	type AccountId = AccountId;
	type Hash = Hash;
	type MaxBackersPerWinner = <Self as multi_block::verifier::Config>::MaxBackersPerWinner;
	type MaxBackersPerWinnerFinal =
		<Self as multi_block::verifier::Config>::MaxBackersPerWinnerFinal;
	type MaxWinnersPerPage = <Self as multi_block::verifier::Config>::MaxWinnersPerPage;
	type MaxVotesPerVoter =
		<<Self as multi_block::Config>::DataProvider as ElectionDataProvider>::MaxVotesPerVoter;
	type MaxLength = MinerMaxLength;
	#[cfg(feature = "runtime-benchmarks")]
	type Solver = frame_election_provider_support::QuickDirtySolver<AccountId, Perbill>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type Solver = <Runtime as multi_block::unsigned::Config>::OffchainSolver;
	type Pages = Pages;
	type Solution = NposCompactSolution16;
	type VoterSnapshotPerBlock = <Runtime as multi_block::Config>::VoterSnapshotPerBlock;
	type TargetSnapshotPerBlock = <Runtime as multi_block::Config>::TargetSnapshotPerBlock;
}

pub struct EraPayout;

impl EraPayout {
	const MILLISECONDS_PER_YEAR: u64 = (1000 * 3600 * 24 * 36525) / 100;

	// TI at the time of execution of [Referendum 1139](https://polkadot.subsquare.io/referenda/1139)
	// block hash: `0x39422610299a75ef69860417f4d0e1d94e77699f45005645ffc5e8e619950f9f`.
	pub const FIXED_PRE_HARD_CAP_TI: Balance = 15_011_657_390_566_252_333;

	// The amount emitted daily pre hard cap.
	// Taken from [AH Block 10469901](https://assethub-polkadot.subscan.io/event/10469901-6).
	const PRE_HARD_CAP_DAILY_EMISSION: Balance = 328797u128 * UNITS;

	// Calculated assuming a 11.7 minute per day time drift (A block time of 6.04875 seconds).
	// https://polkadot.subscan.io/block/30349908
	const HARD_CAP_START: BlockNumber = 30_349_908;

	// The hard issuance cap ratified in Referendum 1710.
	const HARD_CAP_TARGET: Balance = 2_100_000_000u128 * UNITS;

	// 26.28% over two years, 13.14% per year as per ref 1710.
	pub const BI_ANNUAL_RATE: Perbill = Perbill::from_parts(262_800_000);

	// The maximum amount an era can emit. Used as a final safeguard.
	pub const MAX_ERA_EMISSION: Balance = Self::PRE_HARD_CAP_DAILY_EMISSION * 7;

	// The TI at the time of `HARD_CAP_START`.
	pub const MARCH_2026_TI: Balance = 16_743_421_533_310_057_487;

	// The yearly emission post hard pressure enactment.
	fn yearly_after_hard_cap(relay_block_num: BlockNumber) -> Balance {
		let march_14_2026_ti = FixedU128::saturating_from_integer(Self::MARCH_2026_TI);
		let target_ti = FixedU128::saturating_from_integer(Self::HARD_CAP_TARGET);

		// Start date of the curve is set two years prior, thus ensuring first step in March,
		// 2026.
		let two_years_before_march =
			FixedU128::saturating_from_integer(Self::HARD_CAP_START - (2 * RC_YEARS));
		let relay_block_fp = FixedU128::saturating_from_integer(relay_block_num);
		let step_duration = FixedU128::saturating_from_integer(2 * RC_YEARS);

		let two_year_rate = Self::BI_ANNUAL_RATE;

		let Ok(ti_curve) = SteppedCurve::try_new(
			// The start date of the curve.
			two_years_before_march,
			// The initial value of the curve.
			march_14_2026_ti,
			// Target TI.
			RemainingPct { target: target_ti, pct: two_year_rate },
			// Step every two years.
			step_duration,
		) else {
			return 0
		};

		// The last step size tells us the expected TI increase over the current two year
		// period.
		let two_year_emission_fp = ti_curve.last_step_size(relay_block_fp);
		let two_year_emission: u128 = two_year_emission_fp.into_inner() / FixedU128::DIV;
		FixedU128::from_rational(1, 2).saturating_mul_int(two_year_emission)
	}

	pub(crate) fn impl_experimental_inflation_info() -> InflationInfo {
		// We assume un-delayed 24h eras.
		let era_duration = 24 * 60 * 60 * 1000;
		let next_mint =
			<Self as pallet_staking_async::EraPayout<Balance>>::era_payout(0, 0, era_duration);

		// What is our effective issuance rate now?
		let total = next_mint.0 + next_mint.1;
		let annual_issuance = total * 36525 / 100;
		let ti = pallet_balances::TotalIssuance::<Runtime>::get();
		let issuance = Perquintill::from_rational(annual_issuance, ti);

		InflationInfo { issuance, next_mint }
	}
}

impl pallet_staking_async::EraPayout<Balance> for EraPayout {
	fn era_payout(
		_total_staked: Balance,
		_total_issuance: Balance,
		era_duration_millis: u64,
	) -> (Balance, Balance) {
		// A normal-sized era will have 1 / 365.25 here, though the value wobbles a bit:
		let relative_era_len = FixedU128::from_rational(
			era_duration_millis.into(),
			Self::MILLISECONDS_PER_YEAR.into(),
		);

		let relay_block_num =
			<RelaychainDataProvider<Runtime> as BlockNumberProvider>::current_block_number();
		let yearly_emission = Self::yearly_after_hard_cap(relay_block_num);

		let era_emission =
			relative_era_len.saturating_mul_int(yearly_emission).min(Self::MAX_ERA_EMISSION);
		// 15% to treasury, as per Polkadot ref 1139.
		let to_treasury = FixedU128::from_rational(15, 100).saturating_mul_int(era_emission);
		let to_stakers = era_emission.saturating_sub(to_treasury);

		(to_stakers.saturated_into(), to_treasury.saturated_into())
	}
}

parameter_types! {
	pub const SessionsPerEra: SessionIndex = prod_or_fast!(6, 1);
	pub const RelaySessionDuration: BlockNumber = prod_or_fast!(4 * RC_HOURS, RC_MINUTES);
	pub const BondingDuration: sp_staking::EraIndex = 28;
	/// Nominators are expected to be slashable and support fast unbonding
	/// depending on AreNominatorSlashable storage value, as set by governance.
	/// NominatorFastUnbondDuration value below is ignored if nominators are slashable.
	pub const NominatorFastUnbondDuration: sp_staking::EraIndex = 2;
	pub const SlashDeferDuration: sp_staking::EraIndex = 27;
	pub const MaxControllersInDeprecationBatch: u32 = 512;
	// alias for 16, which is the max nominations per nominator in the runtime.
	pub const MaxNominations: u32 = <NposCompactSolution16 as frame_election_provider_support::NposSolution>::LIMIT as u32;

	/// Maximum numbers that we prune from pervious eras in each `prune_era` tx.
	pub MaxPruningItems: u32 = 100;
	/// Session index at which to export the validator set to the relay chain.
	pub const ValidatorSetExportSession: SessionIndex = 4;
}

impl pallet_staking_async::Config for Runtime {
	type Filter = ();
	type OldCurrency = Balances;
	type Currency = Balances;
	type CurrencyBalance = Balance;
	type RuntimeHoldReason = RuntimeHoldReason;
	type CurrencyToVote = sp_staking::currency_to_vote::SaturatingCurrencyToVote;
	type RewardRemainder = ResolveTo<xcm_config::TreasuryAccount, Balances>;
	type Slash = Dap;
	type Reward = ();
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type NominatorFastUnbondDuration = NominatorFastUnbondDuration;
	type SlashDeferDuration = SlashDeferDuration;
	type AdminOrigin = EitherOf<EnsureRoot<AccountId>, StakingAdmin>;
	type EraPayout = EraPayout;
	type MaxExposurePageSize = MaxExposurePageSize;
	type ElectionProvider = MultiBlockElection;
	type VoterList = VoterList;
	type TargetList = UseValidatorsMap<Self>;
	type MaxValidatorSet = MaxValidatorSet;
	type NominationsQuota = pallet_staking_async::FixedNominationsQuota<{ MaxNominations::get() }>;
	type MaxUnlockingChunks = frame_support::traits::ConstU32<32>;
	type HistoryDepth = frame_support::traits::ConstU32<84>;
	type MaxControllersInDeprecationBatch = MaxControllersInDeprecationBatch;
	type EventListeners = (NominationPools, DelegatedStaking);
	type MaxInvulnerables = frame_support::traits::ConstU32<20>;
	// This will start election for the next era as soon as an era starts.
	type PlanningEraOffset = ConstU32<6>;
	type RcClientInterface = StakingRcClient;
	type MaxEraDuration = MaxEraDuration;
	type MaxPruningItems = MaxPruningItems;
	type WeightInfo = weights::pallet_staking_async::WeightInfo<Runtime>;
}

// Must match Polkadot relay chain's `SessionKeys` structure for encoding/decoding compatibility.
sp_runtime::impl_opaque_keys! {
	pub struct RelayChainSessionKeys {
		pub grandpa: grandpa_primitives::AuthorityId,
		pub babe: babe_primitives::AuthorityId,
		pub para_validator: polkadot_primitives::ValidatorId,
		pub para_assignment: polkadot_primitives::AssignmentId,
		pub authority_discovery: authority_discovery_primitives::AuthorityId,
		pub beefy: beefy_primitives::ecdsa_crypto::AuthorityId,
	}
}

parameter_types! {
	// Deposit for one NextKeys entry and multiple KeyOwner entries and ExternallySetKeys.
	pub KeyDeposit: Balance = polkadot_runtime_constants::currency::deposit(1, SessionKeys::max_encoded_len() as u32)
		.saturating_add(
			polkadot_runtime_constants::currency::deposit(<Runtime as pallet_session::Config>::Keys::key_ids().len() as u32,
								<Runtime as pallet_session::Config>::ValidatorId::max_encoded_len() as u32
			)
		).saturating_add(
			polkadot_runtime_constants::currency::deposit(1, AccountId::max_encoded_len() as u32)
		);
}

impl pallet_staking_async_rc_client::Config for Runtime {
	type RelayChainOrigin = EnsureRoot<AccountId>;
	type AHStakingInterface = Staking;
	type SendToRelayChain = StakingXcmToRelayChain;
	type MaxValidatorSetRetries = ConstU32<64>;
	type ValidatorSetExportSession = ValidatorSetExportSession;
	type RelayChainSessionKeys = RelayChainSessionKeys;
	type Currency = Balances;
	// Need a smaller value since the benchmarks do not properly fund the account.
	#[cfg(not(feature = "runtime-benchmarks"))]
	type KeyDeposit = KeyDeposit;
	#[cfg(feature = "runtime-benchmarks")]
	type KeyDeposit = ConstU128<UNITS>;
	type WeightInfo = weights::pallet_staking_async_rc_client::WeightInfo<Runtime>;
}

#[derive(Encode, Decode)]
// Call indices taken from westend-next runtime.
pub enum RelayChainRuntimePallets {
	// Audit: index of `StakingAhClient` on the Relay Chain.
	#[codec(index = 42)]
	AhClient(AhClientCalls),
}

#[derive(Encode, Decode)]
pub enum AhClientCalls {
	// index of `fn validator_set` in `staking-async-ah-client`.
	#[codec(index = 0)]
	ValidatorSet(rc_client::ValidatorSetReport<AccountId>),
	// index of `fn set_keys_from_ah` in `staking-async-ah-client`.
	#[codec(index = 3)]
	SetKeys { stash: AccountId, keys: Vec<u8> },
	// index of `fn purge_keys_from_ah` in `staking-async-ah-client`.
	#[codec(index = 4)]
	PurgeKeys { stash: AccountId },
}

pub struct ValidatorSetToXcm;
impl Convert<rc_client::ValidatorSetReport<AccountId>, Xcm<()>> for ValidatorSetToXcm {
	fn convert(report: rc_client::ValidatorSetReport<AccountId>) -> Xcm<()> {
		rc_client::build_transact_xcm(
			RelayChainRuntimePallets::AhClient(AhClientCalls::ValidatorSet(report)).encode(),
		)
	}
}

pub struct KeysMessageToXcm;
impl Convert<rc_client::KeysMessage<AccountId>, Xcm<()>> for KeysMessageToXcm {
	fn convert(msg: rc_client::KeysMessage<AccountId>) -> Xcm<()> {
		let encoded_call = match msg {
			rc_client::KeysMessage::SetKeys { stash, keys } =>
				RelayChainRuntimePallets::AhClient(AhClientCalls::SetKeys { stash, keys }).encode(),
			rc_client::KeysMessage::PurgeKeys { stash } =>
				RelayChainRuntimePallets::AhClient(AhClientCalls::PurgeKeys { stash }).encode(),
		};
		rc_client::build_transact_xcm(encoded_call)
	}
}

parameter_types! {
	pub RelayLocation: Location = Location::parent();
	/// Conservative RC execution cost for set/purge keys operations.
	/// ~3x of Polkadot relay benchmarked session set/purge_keys (~58-60M ref_time, ~16538 proof).
	pub RemoteKeysExecutionWeight: Weight = Weight::from_parts(180_000_000, 50_000);
}

pub struct StakingXcmToRelayChain;

impl rc_client::SendToRelayChain for StakingXcmToRelayChain {
	type AccountId = AccountId;
	type Balance = Balance;

	fn validator_set(report: rc_client::ValidatorSetReport<Self::AccountId>) -> Result<(), ()> {
		rc_client::XCMSender::<
			xcm_config::XcmRouter,
			RelayLocation,
			rc_client::ValidatorSetReport<Self::AccountId>,
			ValidatorSetToXcm,
		>::send(report)
	}

	fn set_keys(
		stash: Self::AccountId,
		keys: Vec<u8>,
		max_delivery_and_remote_execution_fee: Option<Self::Balance>,
	) -> Result<Self::Balance, rc_client::SendKeysError<Self::Balance>> {
		let execution_cost =
			<DotWeightToFee<Runtime> as frame_support::weights::WeightToFee>::weight_to_fee(
				&RemoteKeysExecutionWeight::get(),
			);

		rc_client::XCMSender::<
			xcm_config::XcmRouter,
			RelayLocation,
			rc_client::KeysMessage<Self::AccountId>,
			KeysMessageToXcm,
		>::send_with_fees::<
			xcm_executor::XcmExecutor<xcm_config::XcmConfig>,
			RuntimeCall,
			AccountId,
			rc_client::AccountId32ToLocation,
			Self::Balance,
		>(
			rc_client::KeysMessage::set_keys(stash.clone(), keys),
			stash,
			max_delivery_and_remote_execution_fee,
			execution_cost,
		)
	}

	fn purge_keys(
		stash: Self::AccountId,
		max_delivery_and_remote_execution_fee: Option<Self::Balance>,
	) -> Result<Self::Balance, rc_client::SendKeysError<Self::Balance>> {
		let execution_cost =
			<DotWeightToFee<Runtime> as frame_support::weights::WeightToFee>::weight_to_fee(
				&RemoteKeysExecutionWeight::get(),
			);

		rc_client::XCMSender::<
			xcm_config::XcmRouter,
			RelayLocation,
			rc_client::KeysMessage<Self::AccountId>,
			KeysMessageToXcm,
		>::send_with_fees::<
			xcm_executor::XcmExecutor<xcm_config::XcmConfig>,
			RuntimeCall,
			AccountId,
			rc_client::AccountId32ToLocation,
			Self::Balance,
		>(
			rc_client::KeysMessage::purge_keys(stash.clone()),
			stash,
			max_delivery_and_remote_execution_fee,
			execution_cost,
		)
	}
}

impl<C> frame_system::offchain::CreateTransactionBase<C> for Runtime
where
	RuntimeCall: From<C>,
{
	type RuntimeCall = RuntimeCall;
	type Extrinsic = UncheckedExtrinsic;
}

impl<LocalCall> frame_system::offchain::CreateTransaction<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	type Extension = TxExtension;

	fn create_transaction(call: RuntimeCall, extension: TxExtension) -> UncheckedExtrinsic {
		generic::UncheckedExtrinsic::new_transaction(call, extension).into()
	}
}

impl<LocalCall> frame_system::offchain::CreateBare<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_bare(call: RuntimeCall) -> UncheckedExtrinsic {
		generic::UncheckedExtrinsic::new_bare(call).into()
	}
}

pub struct InitiateStakingAsync;

impl InitiateStakingAsync {
	fn needs_init() -> bool {
		// A good proxy whether this pallet is initialized or not is that no invulnerable is set in
		// `epmb::signed`. The rest are more fuzzy or are inaccessble.
		multi_block::signed::Invulnerables::<Runtime>::get().is_empty()
	}
}

impl frame_support::traits::OnRuntimeUpgrade for InitiateStakingAsync {
	fn on_runtime_upgrade() -> Weight {
		if !Self::needs_init() {
			return <Runtime as frame_system::Config>::DbWeight::get().writes(1);
		}
		use pallet_election_provider_multi_block::verifier::Verifier;
		// set parity staking miner as the invulnerable submitter in `multi-block`.
		// https://polkadot.subscan.io/account/16ciP5rjt4Yqivi1SWCGh7XsA8BDguV4tnTuyr937u2NME6h
		let acc =
			hex_literal::hex!("f86a0e73c498fa0c135fae2e66da58346e777a6687cc7f7d234b0cb09c021232");
		if let Ok(bounded) = BoundedVec::<AccountId, _>::try_from(vec![acc.into()]) {
			multi_block::signed::Invulnerables::<Runtime>::put(bounded);
		}

		// Set the minimum score for the election, as per the Polkadot RC state.
		//
		// These values are created using script:
		//
		// https://github.com/paritytech/polkadot-scripts/blob/master/src/services/election_score_stats.ts
		//
		// At https://polkadot.subscan.io/block/28207264.
		//
		// Note: the script looks at the last 30 elections, gets their average, and calculates 70%
		// threshold thereof.
		//
		// Recent election scores in Polkadot can be found on:
		// https://polkadot.subscan.io/event?page=1&time_dimension=date&module=electionprovidermultiphase&event_id=electionfinalized
		//
		// The last example, at block [27721215](https://polkadot.subscan.io/event/27721215-0)
		// being:
		//
		// * minimal_stake: 10907549130714057 (1.38x the minimum)
		// * sum_stake: 8028519336725652293 (1.49x the minimum)
		// * sum_stake_squared: 108358993218278434700023844467997545 (0.57 the minimum, the lower
		//   the better)
		let minimum_score = sp_npos_elections::ElectionScore {
			minimal_stake: 7895552765679931,
			sum_stake: 5655838551978860651,
			sum_stake_squared: 187148285683372481445131595645808873,
		};
		<Runtime as multi_block::Config>::Verifier::set_minimum_score(minimum_score);

		<Runtime as frame_system::Config>::DbWeight::get().writes(3)
	}
}

// Deprecated: Holds the TI from March 14, 2026
#[storage_alias(verbatim)]
pub type March2026TI = StorageValue<Runtime, Balance, OptionQuery>;

pub struct RemoveMarchTIValue;

impl frame_support::traits::OnRuntimeUpgrade for RemoveMarchTIValue {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		frame_support::ensure!(March2026TI::exists(), "March2026TI should exist");
		Ok(Vec::new())
	}

	fn on_runtime_upgrade() -> Weight {
		March2026TI::kill();
		<Runtime as frame_system::Config>::DbWeight::get().writes(1)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		frame_support::ensure!(!March2026TI::exists(), "March2026TI should be removed");
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{Runtime, UNITS};
	use approx::assert_relative_eq;
	use asset_test_utils::ExtBuilder;
	use cumulus_pallet_parachain_system::pallet::ValidationData;
	use cumulus_primitives_core::{
		relay_chain::BlockNumber as RC_BlockNumber, PersistedValidationData,
	};
	use pallet_staking_async::EraPayout as _;
	use polkadot_runtime_constants::time::YEARS as RC_YEARS;
	use sp_runtime::Percent;
	use sp_weights::constants::{WEIGHT_PROOF_SIZE_PER_KB, WEIGHT_REF_TIME_PER_MILLIS};

	// TODO: in the future, make these tests use remote-ext and increase their longevity.

	const MILLISECONDS_PER_DAY: u64 = 24 * 60 * 60 * 1000;
	const MARCH_14_2026: RC_BlockNumber = 30_349_908;
	const MARCH_TI: u128 = EraPayout::MARCH_2026_TI;
	const TARGET_TI: u128 = 2_100_000_000 * UNITS;

	// Sets the view of the relay chain block number.
	fn set_relay_number(n: RC_BlockNumber) {
		ValidationData::<Runtime>::set(Some(PersistedValidationData {
			parent_head: vec![].into(),
			relay_parent_number: n,
			max_pov_size: Default::default(),
			relay_parent_storage_root: Default::default(),
		}));
	}

	// With a March 2026 TI value of 16,743,421,533,310,057,487 planck,
	// the daily era payout is ~153,000 DOT. As seen here:
	// https://assethub-polkadot.subscan.io/event/13377452-23
	#[test]
	fn new_march_ti_const_is_empirally_correct() {
		ExtBuilder::<Runtime>::default().build().execute_with(|| {
			pallet_balances::pallet::TotalIssuance::<Runtime, ()>::set(EraPayout::MARCH_2026_TI);
			set_relay_number(MARCH_14_2026);

			let (to_stakers, to_treasury) = EraPayout::era_payout(
				123, // ignored
				456, // ignored
				MILLISECONDS_PER_DAY,
			);

			let total_era_payout = to_stakers + to_treasury;
			let expected = 153_000 * UNITS;
			assert_relative_eq!(
				total_era_payout as f64,
				expected as f64,
				max_relative = 0.005, // within 0.5%
			);
		});
	}

	// The emission values for the two year periods are as expected.
	#[test]
	fn stepped_inflation_two_year_values_correct() {
		ExtBuilder::<Runtime>::default().build().execute_with(|| {
			let two_years: RC_BlockNumber = RC_YEARS * 2;
			pallet_balances::pallet::TotalIssuance::<Runtime, ()>::set(MARCH_TI);

			// First period - March 14, 2026 -> March 14, 2028.
			set_relay_number(MARCH_14_2026);
			let (to_stakers, to_treasury) = EraPayout::era_payout(
				123, // ignored
				456, // ignored
				MILLISECONDS_PER_DAY,
			);
			let two_year_rate = EraPayout::BI_ANNUAL_RATE;
			let first_period_emission = two_year_rate * (TARGET_TI - MARCH_TI);
			assert_relative_eq!(
				(to_stakers as f64 + to_treasury as f64) * 365.25 * 2.0,
				first_period_emission as f64,
				max_relative = 0.00001
			);

			// Second period - March 14, 2028 -> March 14, 2030.
			let march_14_2028 = MARCH_14_2026 + two_years;
			set_relay_number(march_14_2028);
			let (to_stakers, to_treasury) = EraPayout::era_payout(
				123, // ignored
				456, // ignored
				MILLISECONDS_PER_DAY,
			);
			let ti_at_2028 = MARCH_TI + first_period_emission;
			let second_period_emission = two_year_rate * (TARGET_TI - ti_at_2028);
			assert_relative_eq!(
				(to_stakers as f64 + to_treasury as f64) * 365.25 * 2.0,
				second_period_emission as f64,
				max_relative = 0.00001
			);

			// Third period - March 14, 2030 -> March 14, 2032.
			let march_14_2030 = march_14_2028 + two_years;
			set_relay_number(march_14_2030);
			let (to_stakers, to_treasury) = EraPayout::era_payout(
				123, // ignored
				456, // ignored
				MILLISECONDS_PER_DAY,
			);
			let ti_at_2030 = ti_at_2028 + second_period_emission;
			let third_period_emission = two_year_rate * (TARGET_TI - ti_at_2030);
			assert_relative_eq!(
				(to_stakers as f64 + to_treasury as f64) * 365.25 * 2.0,
				third_period_emission as f64,
				max_relative = 0.00001
			);
		});
	}

	// Emission value does not change mid period.
	#[test]
	fn emission_value_static_throughout_period() {
		ExtBuilder::<Runtime>::default().build().execute_with(|| {
			let two_years: RC_BlockNumber = RC_YEARS * 2;
			pallet_balances::pallet::TotalIssuance::<Runtime, ()>::set(MARCH_TI);

			// Get payout at the beginning of the first stepped period.
			set_relay_number(MARCH_14_2026);
			let (to_stakers_start, to_treasury_start) = EraPayout::era_payout(
				123, // ignored
				456, // ignored
				MILLISECONDS_PER_DAY,
			);

			// Get payout just before the end of the first stepped period.
			let almost_two_years_later: RC_BlockNumber = MARCH_14_2026 + two_years - 1;
			set_relay_number(almost_two_years_later);
			let (to_stakers_end, to_treasury_end) = EraPayout::era_payout(
				123, // ignored
				456, // ignored
				MILLISECONDS_PER_DAY,
			);

			// Payout identical.
			assert_eq!(to_stakers_start + to_treasury_start, to_stakers_end + to_treasury_end);
		});
	}

	// The emission is eventually zero.
	#[test]
	fn emission_eventually_zero() {
		ExtBuilder::<Runtime>::default().build().execute_with(|| {
			pallet_balances::pallet::TotalIssuance::<Runtime, ()>::set(MARCH_TI);

			let forseeable_future: RC_BlockNumber = MARCH_14_2026 + (RC_YEARS * 80);
			set_relay_number(forseeable_future);
			let (to_stakers, to_treasury) = EraPayout::era_payout(
				123, // ignored
				456, // ignored
				MILLISECONDS_PER_DAY,
			);

			// Payout is less than 1 UNIT after 41 steps.
			assert!(to_stakers + to_treasury < UNITS);

			let far_future: RC_BlockNumber = MARCH_14_2026 + (RC_YEARS * 500);
			set_relay_number(far_future);
			let (to_stakers, to_treasury) = EraPayout::era_payout(
				123, // ignored
				456, // ignored
				MILLISECONDS_PER_DAY,
			);

			// TI has converged on asymptote. Payout is zero.
			assert_eq!(to_stakers + to_treasury, 0);
		});
	}

	// TI stays <= 2.1B.
	#[test]
	fn ti_is_asymptotic_to_desired_value() {
		ExtBuilder::<Runtime>::default().build().execute_with(|| {
			pallet_balances::pallet::TotalIssuance::<Runtime, ()>::set(MARCH_TI);

			let mut current_ti = MARCH_TI;
			let mut current_bn = MARCH_14_2026;

			// Run for 250 periods (500 years) and check TI and emissions.
			// We know from `emission_eventually_zero` that at this point era emissions are 0
			// and from `emission_value_static_throughout_period` that the emission
			// throughout a period is static.
			for _ in 0..250 {
				set_relay_number(current_bn);

				let (to_stakers, to_treasury) =
					EraPayout::era_payout(123, 456, MILLISECONDS_PER_DAY);

				let daily_emission = to_stakers + to_treasury;
				let period_emission = (daily_emission * 7305) / 10;
				current_ti += period_emission;

				// Step forward a period.
				current_bn += 2 * RC_YEARS;
			}

			// TI has hit asymptote.
			assert!(current_ti > TARGET_TI - UNITS);
			assert!(current_ti < TARGET_TI);
		});
	}

	// Emission is capped under anamolous era duration.
	#[test]
	fn emission_capped_with_anomalous_era_duration() {
		ExtBuilder::<Runtime>::default().build().execute_with(|| {
			set_relay_number(MARCH_14_2026);

			// Simulate an era that lasted 100 years (anomalous).
			let anomalous_duration = 36525 * MILLISECONDS_PER_DAY;
			let (to_stakers, to_treasury) = EraPayout::era_payout(
				123, // ignored
				456, // ignored
				anomalous_duration,
			);

			// Capped at MAX_ERA_EMISSION.
			assert_eq!(to_stakers + to_treasury, EraPayout::MAX_ERA_EMISSION);
		});
	}

	fn analyze_weight(
		op_name: &str,
		op_weight: Weight,
		limit_weight: Weight,
		maybe_max_ratio: Option<Percent>,
	) {
		sp_tracing::try_init_simple();
		let ref_time_ms = op_weight.ref_time() / WEIGHT_REF_TIME_PER_MILLIS;
		let ref_time_ratio = Percent::from_rational(op_weight.ref_time(), limit_weight.ref_time());
		let proof_size_kb = op_weight.proof_size() / WEIGHT_PROOF_SIZE_PER_KB;
		let proof_size_ratio =
			Percent::from_rational(op_weight.proof_size(), limit_weight.proof_size());
		let limit_ms = limit_weight.ref_time() / WEIGHT_REF_TIME_PER_MILLIS;
		let limit_kb = limit_weight.proof_size() / WEIGHT_PROOF_SIZE_PER_KB;
		log::info!(target: "runtime::asset-hub-polkadot", "weight of {op_name:?} is: ref-time: {ref_time_ms}ms, {ref_time_ratio:?} of total, proof-size: {proof_size_kb}KiB, {proof_size_ratio:?} of total (total: {limit_ms}ms, {limit_kb}KiB)",
		);

		if let Some(max_ratio) = maybe_max_ratio {
			assert!(ref_time_ratio <= max_ratio && proof_size_ratio <= max_ratio,)
		}
	}

	mod incoming_xcm_weights {
		use crate::staking::tests::analyze_weight;
		use sp_runtime::{traits::Get, Perbill, Percent};

		#[test]
		fn offence_report() {
			use crate::{AccountId, Runtime};
			use frame_support::dispatch::GetDispatchInfo;
			use pallet_staking_async_rc_client as rc_client;

			sp_io::TestExternalities::new_empty().execute_with(|| {
				// MaxOffenceBatchSize in RC is 32;
				let hefty_offences = (0..32)
					.map(|i| {
						(
							42,
							rc_client::Offence {
								offender: <AccountId>::from([i as u8; 32]),
								reporters: vec![<AccountId>::from([1u8; 32])],
								slash_fraction: Perbill::from_percent(10),
							},
						)
					})
					.collect();
				let di = rc_client::Call::<Runtime>::relay_new_offence_paged {
					offences: hefty_offences,
				}
				.get_dispatch_info();

				let offence_report = di.call_weight + di.extension_weight;
				let mq_service_weight =
					<Runtime as pallet_message_queue::Config>::ServiceWeight::get()
						.unwrap_or_default();

				analyze_weight(
					"offence_report",
					offence_report,
					mq_service_weight,
					Some(Percent::from_percent(95)),
				);
			});
		}

		#[test]
		fn session_report() {
			use crate::{AccountId, Runtime};
			use frame_support::{dispatch::GetDispatchInfo, traits::Get};
			use pallet_staking_async_rc_client as rc_client;

			sp_io::TestExternalities::new_empty().execute_with(|| {
				// this benchmark is a function of current active validator count
				pallet_staking_async::ValidatorCount::<Runtime>::put(600);
				let hefty_report = rc_client::SessionReport {
					activation_timestamp: Some((42, 42)),
					end_index: 42,
					leftover: false,
					validator_points: (0..600u32)
						.map(|i| {
							let unique = i.to_le_bytes();
							let mut acc = [0u8; 32];
							// first 4 bytes should be `unique`, rest 0
							acc[..4].copy_from_slice(&unique);
							(AccountId::from(acc), i)
						})
						.collect(),
				};
				let di = rc_client::Call::<Runtime>::relay_session_report { report: hefty_report }
					.get_dispatch_info();
				let session_report_weight = di.call_weight + di.extension_weight;
				let mq_service_weight =
					<Runtime as pallet_message_queue::Config>::ServiceWeight::get()
						.unwrap_or_default();
				analyze_weight(
					"session_report",
					session_report_weight,
					mq_service_weight,
					Some(Percent::from_percent(50)),
				);
			})
		}
	}

	/// The staking/election weights to check.
	///
	/// * Snapshot-MSP weight (when we take validator snapshot, function of
	///   `TargetSnapshotPerBlock`)
	/// * Snapshot-rest weight (when we take nominator snapshot, function of
	///   `VoterSnapshotPerBlock`)
	/// * Verification of the last page (the most expensive)
	/// * The time it takes to mine a solution via OCW (function of `MinerPages`)
	/// * The weight of the on-the-spot-verification of an OCW-mined solution (function of
	///   `MinerPages`)
	/// * Election export terminal (which is the most expensive, and has round cleanup in it)
	mod weights {
		use super::*;
		#[test]
		fn snapshot_msp_weight() {
			use multi_block::WeightInfo;
			analyze_weight(
				"snapshot_msp",
				<Runtime as multi_block::Config>::WeightInfo::on_initialize_into_snapshot_msp(),
				<Runtime as frame_system::Config>::BlockWeights::get().max_block,
				Some(Percent::from_percent(75)),
			);
		}

		#[test]
		fn snapshot_rest_weight() {
			use multi_block::WeightInfo;
			analyze_weight(
				"snapshot_rest",
				<Runtime as multi_block::Config>::WeightInfo::on_initialize_into_snapshot_rest(),
				<Runtime as frame_system::Config>::BlockWeights::get().max_block,
				Some(Percent::from_percent(75)),
			);
		}

		#[test]
		fn verifier_weight() {
			use multi_block::verifier::WeightInfo;
			analyze_weight(
				"verifier valid terminal",
				<Runtime as multi_block::verifier::Config>::WeightInfo::on_initialize_valid_terminal(),
				<Runtime as frame_system::Config>::BlockWeights::get().max_block,
				Some(Percent::from_percent(75)),
			);

			analyze_weight(
				"verifier invalid terminal",
				<Runtime as multi_block::verifier::Config>::WeightInfo::on_initialize_invalid_terminal(),
				<Runtime as frame_system::Config>::BlockWeights::get().max_block,
				Some(Percent::from_percent(75)),
			);
		}

		#[test]
		fn round_cleanup() {
			use multi_block::signed::WeightInfo;
			analyze_weight(
				"single solution cleanup",
				<Runtime as multi_block::signed::Config>::WeightInfo::clear_old_round_data(
					Pages::get(),
				),
				<Runtime as frame_system::Config>::BlockWeights::get().max_block,
				Some(Percent::from_percent(75)),
			);
			analyze_weight(
				"full solution cleanup",
				<Runtime as multi_block::signed::Config>::WeightInfo::clear_old_round_data(
					Pages::get(),
				)
				.mul(16_u64),
				<Runtime as frame_system::Config>::BlockWeights::get().max_block,
				Some(Percent::from_percent(75)),
			);
		}

		#[test]
		fn export_weight() {
			use multi_block::WeightInfo;
			analyze_weight(
				"export terminal",
				<Runtime as multi_block::Config>::WeightInfo::export_terminal(),
				<Runtime as frame_system::Config>::BlockWeights::get().max_block,
				Some(Percent::from_percent(75)),
			);
		}

		#[test]
		fn verify_unsigned_solution() {
			use multi_block::unsigned::WeightInfo;
			analyze_weight(
				"unsigned solution verify",
				<Runtime as multi_block::unsigned::Config>::WeightInfo::submit_unsigned(),
				<Runtime as frame_system::Config>::BlockWeights::get()
					.per_class
					.get(DispatchClass::Operational)
					.max_extrinsic
					.unwrap(),
				Some(Percent::from_percent(50)),
			);
		}

		#[test]
		fn session_key_deposit_at_most_61_dot() {
			assert!(
				<<Runtime as pallet_staking_async_rc_client::Config>::KeyDeposit as Get<u128>>::get(
				) <= 61 * UNITS
			);
			assert!(
				<<Runtime as pallet_staking_async_rc_client::Config>::KeyDeposit as Get<u128>>::get(
				) > 0
			);
		}
	}
}
