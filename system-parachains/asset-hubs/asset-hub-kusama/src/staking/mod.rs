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
use frame_election_provider_support::{ElectionDataProvider, SequentialPhragmen};
use frame_support::traits::tokens::imbalance::ResolveTo;
use pallet_election_provider_multi_block::{self as multi_block, SolutionAccuracyOf};
use pallet_staking_async::UseValidatorsMap;
use pallet_staking_async_rc_client as rc_client;
use scale_info::TypeInfo;
use sp_arithmetic::FixedU128;
use sp_runtime::{
	transaction_validity::TransactionPriority, FixedPointNumber, SaturatedConversion,
};
use sp_staking::SessionIndex;
use system_parachains_constants::async_backing::MINUTES;
use xcm::v5::prelude::*;

parameter_types! {
	/// Number of election pages that we operate upon. 32 * 6s block = 192s = 3.2min snapshots
	pub Pages: u32 = 32;

	/// Verify 8 solutions at most.
	pub storage SignedValidationPhase: u32 = Pages::get() * 8;

	// TODO: local or RC minutes?
	// TODO: 15 minutes like in RC?
	/// 20 mins for signed phase.
	pub storage SignedPhase: u32 = 20 * MINUTES;

	// TODO: local or RC minutes?
	/// Offchain miner shall mine at most 4 pages.
	pub storage MinerPages: u32 = 4;

	// TODO: local or RC minutes?
	// TODO: 15 minutes like in RC?
	/// 30m for unsigned phase.
	pub storage UnsignedPhase: u32 = 30 * MINUTES;

	// TODO: devide by 8 like in RC?
	/// Allow OCW miner to at most run 4 times in the entirety of the 10m Unsigned Phase.
	pub OffchainRepeat: u32 = UnsignedPhase::get() / 4;

	// TODO: 12_500 like in RC?
	pub storage MaxElectingVoters: u32 = 12_500;

	/// Always equal to `staking.maxValidatorCount`.
	pub storage TargetSnapshotPerBlock: u32 = 2000;

	/// Number of nominators per page of the snapshot, and consequently number of backers in the solution.
	pub VoterSnapshotPerBlock: u32 = MaxElectingVoters::get().div_ceil(Pages::get());

	// TODO: 2000 like in RC?
	/// Maximum number of validators that we may want to elect. 1000 is the end target.
	pub const MaxValidatorSet: u32 = 2000;

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
	pub struct NposCompactSolution24::<
		VoterIndex = u32,
		TargetIndex = u16,
		Accuracy = sp_runtime::PerU16,
		MaxVoters = MaxElectingVoters,
	>(24)
);

parameter_types! {
	pub const BagThresholds: &'static [u64] = &bags_thresholds::THRESHOLDS;
}

type VoterBagsListInstance = pallet_bags_list::Instance1;
impl pallet_bags_list::Config<VoterBagsListInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ScoreProvider = Staking;
	type WeightInfo = weights::pallet_bags_list::WeightInfo<Runtime>;
	type BagThresholds = BagThresholds;
	type Score = sp_npos_elections::VoteWeight;
	// We have to enable it for benchmarks since the benchmark otherwise panics.
	#[cfg(feature = "runtime-benchmarks")]
	type MaxAutoRebagPerBlock = ConstU32<5>;
	#[cfg(not(any(feature = "runtime-benchmarks")))]
	type MaxAutoRebagPerBlock = ConstU32<0>;
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
	// Note: these pallets are currently not "easily" benchmark-able in CIs. They provide a set of
	// weights for polkadot/kusama/westend. Using the polkadot-variant is good enough for now.
	type WeightInfo = multi_block::weights::kusama::MultiBlockWeightInfo<Self>;
}

impl multi_block::verifier::Config for Runtime {
	type MaxWinnersPerPage = MaxWinnersPerPage;
	type MaxBackersPerWinner = MaxBackersPerWinner;
	type MaxBackersPerWinnerFinal = MaxBackersPerWinnerFinal;
	type SolutionDataProvider = MultiBlockElectionSigned;
	type SolutionImprovementThreshold = ();
	type WeightInfo = multi_block::weights::polkadot::MultiBlockVerifierWeightInfo<Self>;
}

parameter_types! {
	pub MaxSubmissions: u32 = 16; // TODO: 16 like in RC?
	pub DepositBase: Balance = 5 * UNITS; // TODO: new deposit base?
	pub DepositPerPage: Balance = 1 * UNITS; // TODO: new deposit per page?
	pub BailoutGraceRatio: Perbill = Perbill::from_percent(50);
	pub EjectGraceRatio: Perbill = Perbill::from_percent(50);
	pub RewardBase: Balance = 10 * UNITS; // TODO: new reward base?
}

impl multi_block::signed::Config for Runtime {
	type Currency = Balances;
	type BailoutGraceRatio = BailoutGraceRatio;
	type EjectGraceRatio = EjectGraceRatio;
	type DepositBase = DepositBase;
	type DepositPerPage = DepositPerPage;
	type InvulnerableDeposit = ();
	type RewardBase = RewardBase;
	type MaxSubmissions = MaxSubmissions;
	type EstimateCallFee = TransactionPayment;
	type WeightInfo = multi_block::weights::polkadot::MultiBlockSignedWeightInfo<Self>;
}

parameter_types! {
	/// Priority of the offchain miner transactions.
	pub MinerTxPriority: TransactionPriority = TransactionPriority::max_value() / 2;
}

impl multi_block::unsigned::Config for Runtime {
	type MinerPages = MinerPages;
	type OffchainStorage = ConstBool<true>;
	type OffchainSolver = SequentialPhragmen<AccountId, SolutionAccuracyOf<Runtime>>;
	type MinerTxPriority = MinerTxPriority;
	type OffchainRepeat = OffchainRepeat;
	type WeightInfo = multi_block::weights::polkadot::MultiBlockUnsignedWeightInfo<Self>;
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
	type Solver = <Runtime as multi_block::unsigned::Config>::OffchainSolver;
	type Pages = Pages;
	type Solution = NposCompactSolution24;
	type VoterSnapshotPerBlock = <Runtime as multi_block::Config>::VoterSnapshotPerBlock;
	type TargetSnapshotPerBlock = <Runtime as multi_block::Config>::TargetSnapshotPerBlock;
}

// We cannot re-use the one from the relay since that is for pallet-staking and will be removed soon
// anyway.
pub struct EraPayout;
impl pallet_staking_async::EraPayout<Balance> for EraPayout {
	fn era_payout(
		_total_staked: Balance,
		_total_issuance: Balance,
		era_duration_millis: u64,
	) -> (Balance, Balance) {
		// TODO: review, copied from Polkadot.
		const MILLISECONDS_PER_YEAR: u64 = (1000 * 3600 * 24 * 36525) / 100;
		// A normal-sized era will have 1 / 365.25 here:
		let relative_era_len =
			FixedU128::from_rational(era_duration_millis.into(), MILLISECONDS_PER_YEAR.into());

		// Fixed total TI that we use as baseline for the issuance.
		let fixed_total_issuance: i128 = 5_216_342_402_773_185_773;
		let fixed_inflation_rate = FixedU128::from_rational(8, 100);
		let yearly_emission = fixed_inflation_rate.saturating_mul_int(fixed_total_issuance);

		let era_emission = relative_era_len.saturating_mul_int(yearly_emission);
		// 15% to treasury, as per Polkadot ref 1139.
		let to_treasury = FixedU128::from_rational(15, 100).saturating_mul_int(era_emission);
		let to_stakers = era_emission.saturating_sub(to_treasury);

		(to_stakers.saturated_into(), to_treasury.saturated_into())
	}
}

// See: TODO @kianenigma
// https://github.com/paseo-network/runtimes/blob/7904882933075551e23d32d86dbb97b971e84bca/relay/paseo/src/lib.rs#L662
// https://github.com/paseo-network/runtimes/blob/7904882933075551e23d32d86dbb97b971e84bca/relay/paseo/constants/src/lib.rs#L49
parameter_types! {
	pub const SessionsPerEra: SessionIndex = 6;
	pub const RelaySessionDuration: BlockNumber = 1 * HOURS; // TODO: RC hours/minutes?
	pub const BondingDuration: sp_staking::EraIndex = 28;
	pub const SlashDeferDuration: sp_staking::EraIndex = 27;
	pub const MaxControllersInDeprecationBatch: u32 = 5169; // TODO: 5169 like in RC?
	// alias for 16, which is the max nominations per nominator in the runtime.
	pub const MaxNominations: u32 = <NposCompactSolution24 as frame_election_provider_support::NposSolution>::LIMIT as u32;
	pub const MaxEraDuration: u64 = RelaySessionDuration::get() as u64 * RELAY_CHAIN_SLOT_DURATION_MILLIS as u64 * SessionsPerEra::get() as u64;
}

impl pallet_staking_async::Config for Runtime {
	type Filter = ();
	type OldCurrency = Balances;
	type Currency = Balances;
	type CurrencyBalance = Balance;
	type RuntimeHoldReason = RuntimeHoldReason;
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
	type WeightInfo = weights::pallet_staking_async::WeightInfo<Runtime>;
	type MaxInvulnerables = frame_support::traits::ConstU32<20>;
	type PlanningEraOffset =
		pallet_staking_async::PlanningEraOffsetOf<Self, RelaySessionDuration, ConstU32<10>>;
	type RcClientInterface = StakingRcClient;
	type MaxEraDuration = MaxEraDuration;
}

impl pallet_staking_async_rc_client::Config for Runtime {
	type RelayChainOrigin = EnsureRoot<AccountId>;
	type AHStakingInterface = Staking;
	type SendToRelayChain = StakingXcmToRelayChain;
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
	// index of `fn validator_set` in `staking-async-ah-client`. It has only one call.
	#[codec(index = 0)]
	ValidatorSet(rc_client::ValidatorSetReport<AccountId>),
}

pub struct ValidatorSetToXcm;
impl sp_runtime::traits::Convert<rc_client::ValidatorSetReport<AccountId>, Xcm<()>>
	for ValidatorSetToXcm
{
	fn convert(report: rc_client::ValidatorSetReport<AccountId>) -> Xcm<()> {
		Xcm(vec![
			Instruction::UnpaidExecution {
				weight_limit: WeightLimit::Unlimited,
				check_origin: None,
			},
			Instruction::Transact {
				origin_kind: OriginKind::Native,
				fallback_max_weight: None,
				call: RelayChainRuntimePallets::AhClient(AhClientCalls::ValidatorSet(report))
					.encode()
					.into(),
			},
		])
	}
}

parameter_types! {
	pub RelayLocation: Location = Location::parent();
}

pub struct StakingXcmToRelayChain;

impl rc_client::SendToRelayChain for StakingXcmToRelayChain {
	type AccountId = AccountId;
	fn validator_set(report: rc_client::ValidatorSetReport<Self::AccountId>) {
		rc_client::XCMSender::<
			xcm_config::XcmRouter,
			RelayLocation,
			rc_client::ValidatorSetReport<Self::AccountId>,
			ValidatorSetToXcm,
		>::split_then_send(report, Some(8));
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
		// TODO: double check. the only way I found to access the `new_transaction` method.
		// `UncheckedExtrinsic` is not generic::UncheckedExtrinsic, its wrapped by pallet revive's
		// type `UncheckedExtrinsic`.
		<UncheckedExtrinsic as TypeInfo>::Identity::new_transaction(call, extension).into()
	}
}

impl<LocalCall> frame_system::offchain::CreateBare<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_bare(call: RuntimeCall) -> UncheckedExtrinsic {
		// TODO: double check. the only way I found to access the `new_bare` method.
		// `UncheckedExtrinsic` is not generic::UncheckedExtrinsic, its wrapped by pallet revive's
		// type `UncheckedExtrinsic`.
		<UncheckedExtrinsic as TypeInfo>::Identity::new_bare(call).into()
	}
}
