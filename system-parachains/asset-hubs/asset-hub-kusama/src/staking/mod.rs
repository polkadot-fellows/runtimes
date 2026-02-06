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

use crate::{governance::StakingAdmin, *};
use codec::Encode;
use frame_election_provider_support::{ElectionDataProvider, SequentialPhragmen};
use frame_support::traits::tokens::imbalance::ResolveTo;
use pallet_election_provider_multi_block::{self as multi_block, SolutionAccuracyOf};
use pallet_staking_async::UseValidatorsMap;
use pallet_staking_async_rc_client as rc_client;
use sp_runtime::{generic, transaction_validity::TransactionPriority, Perquintill};
use sp_staking::SessionIndex;
use system_parachains_common::apis::InflationInfo;
use xcm::v5::prelude::*;

// alias for the ones backed by parameters-pallet.
use dynamic_params::staking_election::{
	MaxElectingVoters, MaxEraDuration, MaxSignedSubmissions, MinerPages, SignedPhase,
	TargetSnapshotPerBlock, UnsignedPhase,
};

// NOTES:
// * The EPMB pallets only use local block times. They can one day be moved to use the relay-chain
//   block, based on how the core-count and fast-blocks evolve, they might benefit from moving to
//   relay-chain blocks. As of now, the duration of all phases are more about "POV" than "time", so
//   AH deciding to use 2 or 3 cores is not an issue, as its PoV capacity also increases. The signed
//   and unsigned phase are more about "time", yet the values used here are generous and should
//   leave plenty of time for solution mining and submission.
parameter_types! {
	/// Kusama election pages, 1.6m snapshot.
	pub Pages: u32 = 16;

	/// Verify all signed submissions.
	pub SignedValidationPhase: u32 = Pages::get() * MaxSignedSubmissions::get();

	/// Allow OCW miner to at most run 4 times in the entirety of the 10m Unsigned Phase.
	pub OffchainRepeat: u32 = UnsignedPhase::get() / 4;

	/// 782 nominators in each snapshot page (and consequently solution page, at most).
	pub VoterSnapshotPerBlock: u32 = MaxElectingVoters::get().div_ceil(Pages::get());

	/// Kusama will at most have 1000 validators.
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
	///
	/// Safety note: during AHM, this value must be the same as it was on the RC
	pub const MaxExposurePageSize: u32 = 512;
}

// Solution type.
//
// * Voter index: u32 will scale to a near-infinite number of validators/nominators as npos-voters.
// While u16 is also enough, it might very well lead to issues if we wish to increase
// `MaxElectingVoters`. Numbers show that the byte-size of the solution is far from being a
// bottleneck, ergo using u32.
// * Target index: 65k is well enough for a network with 1000 validators
// max.
// * 24: Note that kusama allows for 24 nominations per nominator.
frame_election_provider_support::generate_solution_type!(
	#[compact]
	pub struct NposCompactSolution24::<
		VoterIndex = u32,
		TargetIndex = u16,
		Accuracy = sp_runtime::PerU16,
		MaxVoters = VoterSnapshotPerBlock,
	>(24)
);

parameter_types! {
	/// AHM Audit: should be the same as RC.
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
	type WeightInfo = weights::pallet_bags_list::WeightInfo<Runtime>;
	type BagThresholds = BagThresholds;
	type Score = sp_npos_elections::VoteWeight;
	type MaxAutoRebagPerBlock = RebagIffMigrationDone;
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
	type OnSlash = ResolveTo<xcm_config::TreasuryAccount, Balances>;
	type SlashRewardFraction = SlashRewardFraction;
	type RuntimeHoldReason = RuntimeHoldReason;
	type CoreStaking = Staking;
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
	// Clean all data on round rotation. Later on, we can move to lazy deletion.
	type OnRoundRotation = multi_block::CleanRound<Self>;
	type WeightInfo = weights::pallet_election_provider_multi_block::WeightInfo<Self>;
}

impl multi_block::verifier::Config for Runtime {
	type MaxWinnersPerPage = MaxWinnersPerPage;
	type MaxBackersPerWinner = MaxBackersPerWinner;
	type MaxBackersPerWinnerFinal = MaxBackersPerWinnerFinal;
	type SolutionDataProvider = MultiBlockElectionSigned;
	// Deliberate choice: we want any solution, even an epsilon better, to be considered superior.
	type SolutionImprovementThreshold = ();
	type WeightInfo = weights::pallet_election_provider_multi_block_verifier::WeightInfo<Self>;
}

/// ## Example
/// ```
/// use asset_hub_kusama_runtime::staking::GeometricDeposit;
/// use pallet_election_provider_multi_block::signed::CalculateBaseDeposit;
/// use kusama_runtime_constants::currency::UNITS;
///
/// // Base deposit
/// assert_eq!(GeometricDeposit::calculate_base_deposit(0), UNITS / 10); // 0.1 KSM
/// assert_eq!(GeometricDeposit::calculate_base_deposit(1), 4 * UNITS / 10); // 0.4 KSM
/// assert_eq!(GeometricDeposit::calculate_base_deposit(2), 16 * UNITS / 10); // 1.6 KSM
/// // and so on
///
/// // Full 16 page deposit, to be paid on top of the above base
/// sp_io::TestExternalities::default().execute_with(|| {
///     let deposit = asset_hub_kusama_runtime::staking::SignedDepositPerPage::get() * 16;
///     assert_eq!(deposit, 515_519_591_040); // 0.5 KSM
/// })
/// ```
pub struct GeometricDeposit;
impl multi_block::signed::CalculateBaseDeposit<Balance> for GeometricDeposit {
	fn calculate_base_deposit(existing_submitters: usize) -> Balance {
		let start: Balance = UNITS / 10;
		let common: Balance = 4;
		start.saturating_mul(common.saturating_pow(existing_submitters as u32))
	}
}

// Parameters only regarding signed submission deposits/rewards.
parameter_types! {
	pub SignedDepositPerPage: Balance = system_para_deposit(1, NposCompactSolution24::max_encoded_len() as u32);
	/// Bailing is rather disincentivized, as it can allow attackers to submit bad solutions, but
	/// get away with it last minute. We only return 25% of the deposit in case someone bails. In
	/// Polkadot, this value will be lower or simply zero.
	pub BailoutGraceRatio: Perbill = Perbill::from_percent(25);
	/// Invulnerable miners will pay this deposit only.
	pub InvulnerableFixedDeposit: Balance = UNITS;
	/// Being ejected is already paid for by the new submitter replacing you; no need to charge deposit.
	pub EjectGraceRatio: Perbill = Perbill::from_percent(100);
	/// .2 KSM as the reward for the best signed submission.
	pub RewardBase: Balance = UNITS / 5;
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
	type WeightInfo = weights::pallet_election_provider_multi_block_signed::WeightInfo<Self>;
}

parameter_types! {
	/// Priority of the "offchain" miner transactions.
	pub MinerTxPriority: TransactionPriority = TransactionPriority::MAX / 2;

	/// Offchain miner transaction can fill up to 75% of the block size.
	pub MinerMaxLength: u32 = Perbill::from_percent(75) *
		*RuntimeBlockLength::get()
		.max
		.get(DispatchClass::Normal);

	/// Whether the offchain worker should use its offchain cache or not. Set as a storage, so it can be tweaked slightly easier than with a code-upgrade.
	pub storage OffchainStorage: bool = true;
}

impl multi_block::unsigned::Config for Runtime {
	type MinerPages = MinerPages;
	type OffchainStorage = OffchainStorage;
	// Note: we don't want the offchain miner to run balancing, as it might be too expensive to run
	// in WASM, ergo the last `()`.
	type OffchainSolver = SequentialPhragmen<AccountId, SolutionAccuracyOf<Runtime>, ()>;
	type MinerTxPriority = MinerTxPriority;
	type OffchainRepeat = OffchainRepeat;
	type WeightInfo = weights::pallet_election_provider_multi_block_unsigned::WeightInfo<Self>;
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
	type Solution = NposCompactSolution24;
	type VoterSnapshotPerBlock = <Runtime as multi_block::Config>::VoterSnapshotPerBlock;
	type TargetSnapshotPerBlock = <Runtime as multi_block::Config>::TargetSnapshotPerBlock;
}

// AUDIT: This is the inflation formula of Kusama prior to AHM. Source:
// https://github.com/polkadot-fellows/runtimes/blob/18cbc8b3004f3cff44f6de053bb4220a9f85a7b1/relay/kusama/src/lib.rs#L793-L823
pub struct EraPayout;
impl pallet_staking_async::EraPayout<Balance> for EraPayout {
	fn era_payout(
		total_staked: Balance,
		total_issuance: Balance,
		era_duration_millis: u64,
	) -> (Balance, Balance) {
		const MILLISECONDS_PER_YEAR: u64 = 1000 * 3600 * 24 * 36525 / 100;
		use crate::dynamic_params;

		let params = polkadot_runtime_common::impls::EraPayoutParams {
			total_staked,
			total_stakable: total_issuance,
			ideal_stake: dynamic_params::issuance::IdealStake::get(),
			max_annual_inflation: dynamic_params::issuance::MaxInflation::get(),
			min_annual_inflation: dynamic_params::issuance::MinInflation::get(),
			falloff: dynamic_params::issuance::Falloff::get(),
			period_fraction: Perquintill::from_rational(era_duration_millis, MILLISECONDS_PER_YEAR),
			// Note: Kusama RC had the code for reserving a subset of its "ideal-staked-ratio" to be
			// allocated to parachain auctions. Yet, this code was buggy in the RC, and was actually
			// not doing this. Even if otherwise, in the absence of auctions, this code made no
			// sense, and Kusama governance can alter the `ideal_stake` parameter if need be.
			// Finally, this information about the parachain count is not even available in AHM
			// state.
			legacy_auction_proportion: None,
		};
		polkadot_runtime_common::impls::relay_era_payout(params)
	}
}

impl EraPayout {
	pub(crate) fn impl_experimental_inflation_info() -> InflationInfo {
		use pallet_staking_async::{ActiveEra, ActiveEraInfo, ErasTotalStake};
		let staked = ActiveEra::<Runtime>::get()
			.map(|ActiveEraInfo { index, .. }| ErasTotalStake::<Runtime>::get(index))
			.unwrap_or(0);
		let ti = pallet_balances::Pallet::<Runtime>::total_issuance();

		// We assume un-delayed 6h eras.
		let era_duration = 6 * 60 * 60 * 1000;
		let next_mint = <Self as pallet_staking_async::EraPayout<Balance>>::era_payout(
			staked,
			ti,
			era_duration,
		);
		let total = next_mint.0 + next_mint.1;
		const NUM_ERAS_PER_DAY: u128 = 4;
		let annual_issuance = total * 36525 * NUM_ERAS_PER_DAY / 100;
		let issuance = Perquintill::from_rational(annual_issuance, ti);

		InflationInfo { issuance, next_mint }
	}
}

parameter_types! {
	pub const SessionsPerEra: SessionIndex = 6;
	/// Note: This is measured in RC block time. Our calculation of when to plan a new era might get
	/// confused in case AH block times change. Ideally, this value should be updated alongside AH's
	/// block time. If AH blocks progress faster, our eras will become shorter, which is not a
	/// critical issue.
	pub const RelaySessionDuration: BlockNumber = RC_HOURS;
	pub const BondingDuration: sp_staking::EraIndex = 28;
	pub const SlashDeferDuration: sp_staking::EraIndex = 27;
	/// Note: smaller value than in RC as parachain PVF is more sensitive to over-weight execution.
	pub const MaxControllersInDeprecationBatch: u32 = 512;
	/// alias for 24, which is the max nominations per nominator in the runtime.
	pub const MaxNominations: u32 = <
		NposCompactSolution24
		as
		frame_election_provider_support::NposSolution
	>::LIMIT as u32;

	/// Maximum numbers that we prune from previous eras in each `prune_era` tx.
	pub MaxPruningItems: u32 = 100;

	/// Unlike Polkadot, Kusama nominators are expected to be slashable and do not
	/// support fast unbonding. Consequently, AreNominatorSlashable is intended to
	/// remain set to true and should not be modified via governance.
	/// NominatorFastUnbondDuration value below is therefore ignored.
	pub const NominatorFastUnbondDuration: sp_staking::EraIndex = 2;
	pub const ValidatorSetExportSession: SessionIndex = 4;
}

impl pallet_staking_async::Config for Runtime {
	type Filter = ();
	type OldCurrency = Balances;
	type Currency = Balances;
	type CurrencyBalance = Balance;
	type RuntimeHoldReason = RuntimeHoldReason;
	// Note: Previously, we used `U128CurrencyToVote`, which donwscaled as the TI moved closer to
	// u64::MAX. Both chains are rather close to this value, so we move to saturating. This is a
	// good option, as it means some whales, if in any crazy scenario, have more than u64::MAX in
	// their balance, the excess will be ignored in staking election voting. Contrary, if we use
	// `U128CurrencyToVote`, the presence of one whale with more than u64::MAX will cause everyone's
	// staking election vote to be downscaled by two.
	type CurrencyToVote = sp_staking::currency_to_vote::SaturatingCurrencyToVote;
	type RewardRemainder = ResolveTo<xcm_config::TreasuryAccount, Balances>;
	type Slash = ResolveTo<xcm_config::TreasuryAccount, Balances>;
	type Reward = ();
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
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
	// Note used; don't care.
	type MaxInvulnerables = frame_support::traits::ConstU32<20>;
	// This will start election for the next era as soon as an era starts.
	type PlanningEraOffset = ConstU32<6>;
	type RcClientInterface = StakingRcClient;
	type MaxEraDuration = MaxEraDuration;
	type WeightInfo = weights::pallet_staking_async::WeightInfo<Runtime>;
	type MaxPruningItems = MaxPruningItems;
	type NominatorFastUnbondDuration = NominatorFastUnbondDuration;
}

// Must match Kusama relay chain's `SessionKeys` structure for encoding/decoding compatibility.
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

impl pallet_staking_async_rc_client::Config for Runtime {
	type RelayChainOrigin = EnsureRoot<AccountId>;
	type AHStakingInterface = Staking;
	type SendToRelayChain = StakingXcmToRelayChain;
	type MaxValidatorSetRetries = ConstU32<64>;
	type ValidatorSetExportSession = ValidatorSetExportSession;
	type RelayChainSessionKeys = RelayChainSessionKeys;
	type Balance = Balance;
	// | Key                 | Crypto  | Public Key | Signature |
	// |---------------------|---------|------------|-----------|
	// | grandpa             | Ed25519 | 32 bytes   | 64 bytes  |
	// | babe                | Sr25519 | 32 bytes   | 64 bytes  |
	// | para_validator      | Sr25519 | 32 bytes   | 64 bytes  |
	// | para_assignment     | Sr25519 | 32 bytes   | 64 bytes  |
	// | authority_discovery | Sr25519 | 32 bytes   | 64 bytes  |
	// | beefy               | ECDSA   | 33 bytes   | 65 bytes  |
	// Buffer for SCALE encoding overhead and future expansions.
	type MaxSessionKeysLength = ConstU32<256>;
	type WeightInfo = ();
}

#[derive(Encode, Decode)]
// Call indices taken from westend-next runtime.
pub enum RelayChainRuntimePallets {
	// Audit: index of `StakingAhClient` on the Relay Chain.
	#[codec(index = 48)]
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
impl sp_runtime::traits::Convert<rc_client::ValidatorSetReport<AccountId>, Xcm<()>>
	for ValidatorSetToXcm
{
	fn convert(report: rc_client::ValidatorSetReport<AccountId>) -> Xcm<()> {
		rc_client::build_transact_xcm(
			RelayChainRuntimePallets::AhClient(AhClientCalls::ValidatorSet(report)).encode(),
		)
	}
}

pub struct KeysMessageToXcm;
impl sp_runtime::traits::Convert<rc_client::KeysMessage<AccountId>, Xcm<()>> for KeysMessageToXcm {
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
	/// ~3x of Kusama relay benchmarked session set/purge_keys (~61-62M ref_time, ~16538 proof).
	pub RemoteKeysExecutionWeight: Weight = Weight::from_parts(190_000_000, 50_000);
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
			<KsmWeightToFee<Runtime> as frame_support::weights::WeightToFee>::weight_to_fee(
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
			<KsmWeightToFee<Runtime> as frame_support::weights::WeightToFee>::weight_to_fee(
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
		// https://kusama.subscan.io/account/FyrGiYDGVxg5UUpN3qR5nxKGMxCe5Ddfkb3BXjxybG6j8gX
		let acc =
			hex_literal::hex!("96a6df31a112d610277c818fd9a8443d265fb5ab83cba47c5e89cff16cf9e011");
		if let Ok(bounded) = BoundedVec::<AccountId, _>::try_from(vec![acc.into()]) {
			multi_block::signed::Invulnerables::<Runtime>::put(bounded);
		}

		// set the minimum score for the election, as per the kusama RC state.
		//
		// This value is set from block [29,940,247](https://dev.papi.how/explorer/0xf8e2599cd04321369810cd6b4c520f4bc3a8f08f76089d0e467d4a0967179a94#networkId=kusama&endpoint=wss%3A%2F%2Frpc.ibp.network%2Fkusama) of Kusama RC.
		// Recent election scores in Kusama can be found on:
		// https://kusama.subscan.io/event?page=1&time_dimension=date&module=electionprovidermultiphase&event_id=electionfinalized
		//
		// The last example, at block [29939392](https://kusama.subscan.io/event/29939392-0) being:
		//
		// * minimal_stake: 6543_701_618_936_726 (2.12x the minimum -- 6.5k KSM)
		// * sum_stake: 8_062_560_594_210_938_663 (2.3x the minimum -- 8M KSM)
		// * sum_stake_squared: 67_504_538_161_651_736_253_970_267_717_229_279 (0.8 the minimum, the
		//   lower the better)
		let minimum_score = sp_npos_elections::ElectionScore {
			minimal_stake: 2957640724907066,
			sum_stake: 3471819933857856584,
			sum_stake_squared: 78133097080615021100202963085417458,
		};
		<Runtime as multi_block::Config>::Verifier::set_minimum_score(minimum_score);

		<Runtime as frame_system::Config>::DbWeight::get().writes(3)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use frame_election_provider_support::ElectionProvider;
	use pallet_staking_async::EraPayout;
	use sp_runtime::Percent;
	use sp_weights::constants::{WEIGHT_PROOF_SIZE_PER_KB, WEIGHT_REF_TIME_PER_MILLIS};

	#[test]
	fn inflation_sanity_check() {
		// values taken from a recent Kusama snapshot:
		// active era: 8546
		// total_staked: 8085567183241128549
		// TI: 17016510054564053390
		// Ext needed because of parameters, which are not set in kusama, so the defaults are gud.
		// recent era paid: https://kusama.subscan.io/event/30011049-0
		// 835 KSM / 291 KSM
		sp_io::TestExternalities::new_empty().execute_with(|| {
			let average_era_duration_millis = 6 * 60 * 60 * 1000; // 6h
			let (staking, treasury) = super::EraPayout::era_payout(
				8085567183241128549,
				17016510054564053390,
				average_era_duration_millis,
			);
			assert_eq!(staking, 844_606070970705);
			assert_eq!(treasury, 320_110565207524);

			pallet_balances::TotalIssuance::<Runtime>::put(17016510054564053390u128);
			pallet_staking_async::ActiveEra::<Runtime>::put(pallet_staking_async::ActiveEraInfo {
				index: 777,
				start: None,
			});
			pallet_staking_async::ErasTotalStake::<Runtime>::insert(777, 8085567183241128549u128);
			let expected_issuance_parts = 99999999999999249;
			assert_eq!(
				super::EraPayout::impl_experimental_inflation_info(),
				InflationInfo {
					issuance: Perquintill::from_parts(99999999999999249),
					next_mint: (staking, treasury),
				}
			);
			// around 9% now
			assert_eq!(expected_issuance_parts * 100 / 10u64.pow(18), 9)
		});
	}

	#[test]
	fn election_duration_less_than_session() {
		// parameters of kusama are such that the election is intended to kick of at the start of
		// session `n` and the results to be ready before the end of that session. Atm RC and KAH
		// have the same block time, 6s.
		sp_io::TestExternalities::new_empty().execute_with(|| {
			sp_tracing::try_init_simple();
			let duration = <<Runtime as pallet_staking_async::Config>::ElectionProvider as ElectionProvider>::duration();
			let session = RelaySessionDuration::get();
			log::info!(target: "runtime::asset-hub-kusama", "election duration is {duration:?}, relay session {session:?}",);
			assert!(duration < session);
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
		log::info!(target: "runtime::asset-hub-kusama", "weight of {op_name:?} is: ref-time: {ref_time_ms}ms, {ref_time_ratio:?} of total, proof-size: {proof_size_kb}KiB, {proof_size_ratio:?} of total (total: {limit_ms}ms, {limit_kb}KiB)",
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
				pallet_staking_async::ValidatorCount::<Runtime>::put(1000);
				let hefty_report = rc_client::SessionReport {
					activation_timestamp: Some((42, 42)),
					end_index: 42,
					leftover: false,
					validator_points: (0..1000u32)
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
	}
}
