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
use frame_support::{traits::tokens::imbalance::ResolveTo, BoundedVec};
use pallet_election_provider_multi_block::{self as multi_block, SolutionAccuracyOf};
use pallet_staking_async::UseValidatorsMap;
use pallet_staking_async_rc_client as rc_client;
use sp_arithmetic::FixedU128;
use sp_runtime::{
	traits::Convert, transaction_validity::TransactionPriority, FixedPointNumber,
	SaturatedConversion,
};
use sp_staking::SessionIndex;
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
			matches!(
				pallet_ah_migrator::AhMigrationStage::<Runtime>::get(),
				pallet_ah_migrator::MigrationStage::MigrationDone
			) {
			5
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

/// ## Example
/// ```
/// use asset_hub_polkadot_runtime::staking::GeometricDeposit;
/// use pallet_election_provider_multi_block::signed::CalculateBaseDeposit;
/// use polkadot_runtime_constants::currency::UNITS;
///
/// // Base deposit
/// assert_eq!(GeometricDeposit::calculate_base_deposit(0), 4 * UNITS);
/// assert_eq!(GeometricDeposit::calculate_base_deposit(1), 8 * UNITS );
/// assert_eq!(GeometricDeposit::calculate_base_deposit(2), 16 * UNITS);
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
		let start: Balance = UNITS * 4;
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
	type InvulnerableDeposit = ();
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
	type OffchainSolver = SequentialPhragmen<AccountId, SolutionAccuracyOf<Runtime>>;
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
	type Solver = <Runtime as multi_block::unsigned::Config>::OffchainSolver;
	type Pages = Pages;
	type Solution = NposCompactSolution16;
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
		const MILLISECONDS_PER_YEAR: u64 = (1000 * 3600 * 24 * 36525) / 100;
		// A normal-sized era will have 1 / 365.25 here:
		let relative_era_len =
			FixedU128::from_rational(era_duration_millis.into(), MILLISECONDS_PER_YEAR.into());

		// TI at the time of execution of [Referendum 1139](https://polkadot.subsquare.io/referenda/1139), block hash: `0x39422610299a75ef69860417f4d0e1d94e77699f45005645ffc5e8e619950f9f`.
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

parameter_types! {
	pub const SessionsPerEra: SessionIndex = prod_or_fast!(6, 1);
	pub const RelaySessionDuration: BlockNumber = prod_or_fast!(4 * RC_HOURS, RC_MINUTES);
	pub const BondingDuration: sp_staking::EraIndex = 28;
	pub const SlashDeferDuration: sp_staking::EraIndex = 27;
	pub const MaxControllersInDeprecationBatch: u32 = 512;
	// alias for 16, which is the max nominations per nominator in the runtime.
	pub const MaxNominations: u32 = <NposCompactSolution16 as frame_election_provider_support::NposSolution>::LIMIT as u32;

	/// Maximum numbers that we prune from pervious eras in each `prune_era` tx.
	pub MaxPruningItems: u32 = 100;
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
	type MaxInvulnerables = frame_support::traits::ConstU32<20>;
	type PlanningEraOffset =
		pallet_staking_async::PlanningEraOffsetOf<Self, RelaySessionDuration, ConstU32<10>>;
	type RcClientInterface = StakingRcClient;
	type MaxEraDuration = MaxEraDuration;
	type MaxPruningItems = MaxPruningItems;
	type WeightInfo = weights::pallet_staking_async::WeightInfo<Runtime>;
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
	#[codec(index = 42)]
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
		// TODO: after https://github.com/paritytech/polkadot-sdk/pull/9619, use `XCMSender::send`
		let message = ValidatorSetToXcm::convert(report);
		let dest = RelayLocation::get();
		let _ = crate::send_xcm::<xcm_config::XcmRouter>(dest, message).inspect_err(|err| {
			log::error!(target: "runtime::ah-client", "Failed to send validator set report: {err:?}");
		});
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
		UncheckedExtrinsic::new_transaction(call, extension)
	}
}

impl<LocalCall> frame_system::offchain::CreateBare<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_bare(call: RuntimeCall) -> UncheckedExtrinsic {
		UncheckedExtrinsic::new_bare(call)
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
			return <Runtime as frame_system::Config>::DbWeight::get().writes(1)
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
		// This value is set from block 27,730,872 of Polkadot RC.
		// Recent election scores in Polkadot can be found on:
		// https://polkadot.subscan.io/event?page=1&time_dimension=date&module=electionprovidermultiphase&event_id=electionfinalized
		//
		// The last example, at block [27721215](https://polkadot.subscan.io/event/27721215-0) being:
		//
		// * minimal_stake: 10907549130714057 (1.28x the minimum)
		// * sum_stake: 8028519336725652293 (2.44x the minimum)
		// * sum_stake_squared: 108358993218278434700023844467997545 (0.4 the minimum, the lower the
		//   better)
		let minimum_score = sp_npos_elections::ElectionScore {
			minimal_stake: 8474057820699941,
			sum_stake: 3276970719352749444,
			sum_stake_squared: 244059208045236715654727835467163294,
		};
		<Runtime as multi_block::Config>::Verifier::set_minimum_score(minimum_score);

		<Runtime as frame_system::Config>::DbWeight::get().writes(3)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use sp_runtime::Percent;
	use sp_weights::constants::{WEIGHT_PROOF_SIZE_PER_KB, WEIGHT_REF_TIME_PER_MILLIS};
	// TODO: in the future, make these tests use remote-ext and increase their longevity.

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
				// up to a 1/3 of the validators are reported in a single batch of offences
				let hefty_offences = (0..333)
					.map(|i| {
						rc_client::Offence {
							offender: <AccountId>::from([i as u8; 32]), /* overflows, but
							                                             * whatever,
							                                             * don't matter */
							reporters: vec![<AccountId>::from([1u8; 32])],
							slash_fraction: Perbill::from_percent(10),
						}
					})
					.collect();
				let di = rc_client::Call::<Runtime>::relay_new_offence {
					slash_session: 42,
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
				Some(Percent::from_percent(95)), // TODO: reduce to 75 once re-benchmarked.
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
