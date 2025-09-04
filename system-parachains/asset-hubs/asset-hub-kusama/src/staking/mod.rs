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
use sp_runtime::{transaction_validity::TransactionPriority, Perquintill};
use sp_staking::SessionIndex;
use system_parachains_constants::async_backing::MINUTES;
use xcm::v5::prelude::*;

// NOTES:
// * a lot of the parameters are defined as `storage` such that they can be upgraded with a smaller
//   overhead by the root origin, as opposed to a code upgrade. We expect to remove them. If all
//   goes well in Kusama AHM, we can stop using `storage` in Polkadot.
// * The EPMB pallets only use local block times. They can one day be moved to use the relay-chain
//   block, based on how the core-count and fast-blocks evolve, they might benefit from moving to
//   relay-chain blocks. As of now, the duration of all phases are more about "POV" than "time", so
//   AH deciding to use 2 or 3 cores is not an issue, as its PoV capacity also increases. The signed
//   and unsigned phase are more about "time", yet the values used here are generous and should
//   leave plenty of time for solution mining and submission.
parameter_types! {
	/// Kusama election pages, 1.6m snapshot.
	pub Pages: u32 = 16;

	/// 20 mins worth of local 6s blocks for signed phase.
	pub storage SignedPhase: u32 = 20 * MINUTES;

	/// Allow up to 16 signed solutions to be submitted.
	///
	/// Safety note: Larger signed submission increases the weight of the `OnRoundRotation` data cleanup. Double check TODO @kianenigma.
	pub storage MaxSignedSubmissions: u32 = 16;

	/// Verify all of them.
	pub storage SignedValidationPhase: u32 = Pages::get() * MaxSignedSubmissions::get();

	/// 30m for unsigned phase.
	pub storage UnsignedPhase: u32 = 30 * MINUTES;

	/// In which we try and mine a 4-page solution.
	pub storage MinerPages: u32 = 4;

	/// Allow OCW miner to at most run 4 times in the entirety of the 10m Unsigned Phase.
	pub OffchainRepeat: u32 = UnsignedPhase::get() / 4;

	/// Kusama allows up to 12_500 active nominators (aka. electing voters).
	pub storage MaxElectingVoters: u32 = 12_500;

	/// Which leads to ~782 nominators in each snapshot page (and consequently solution page, at most).
	pub VoterSnapshotPerBlock: u32 = MaxElectingVoters::get().div_ceil(Pages::get());

	/// An upper bound on the number of anticipated kusama "validator candidates".
	///
	/// At the time of writing, Kusama has 1000 active validators, and ~2k validator candidates.
	///
	/// Safety note: This increases the weight of `on_initialize_into_snapshot_msp` weight. Double check TODO @kianenigma.
	pub storage TargetSnapshotPerBlock: u32 = 3000;

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
		if cfg!(feature = "runtime-benchmarks") {
			5
		} else {
			if matches!(
				pallet_ah_migrator::AhMigrationStage::<Runtime>::get(),
				pallet_ah_migrator::MigrationStage::MigrationDone
			) {
				5
			} else {
				0
			}
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

parameter_types! {
	pub const GeometricDepositStart: Balance = UNITS / 10;
	pub const GeometricDepositCommon: Balance = 4;
}

/// ## Example
/// ```
/// fn main() {
/// 	use asset_hub_kusama_runtime::staking::GeometricDeposit;
/// 	use pallet_election_provider_multi_block::signed::CalculateBaseDeposit;
/// 	use kusama_runtime_constants::currency::UNITS;
///
/// 	// Base deposit
/// 	assert_eq!(GeometricDeposit::calculate_base_deposit(0), UNITS / 10); // 0.1 KSM
/// 	assert_eq!(GeometricDeposit::calculate_base_deposit(1), 4 * UNITS / 10); // 0.4 KSM
/// 	assert_eq!(GeometricDeposit::calculate_base_deposit(2), 16 * UNITS / 10); // 1.6 KSM
/// 	// and so on
///
/// 	// Full 16 page deposit, to be paid on top of the above base
/// 	sp_io::TestExternalities::default().execute_with(|| {
/// 		let deposit = asset_hub_kusama_runtime::staking::DepositPerPage::get() * 16;
/// 		assert_eq!(deposit, 515_519_591_040); // 0.5 KSM
/// 	})
/// }
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
	pub DepositPerPage: Balance = system_para_deposit(1, NposCompactSolution24::max_encoded_len() as u32);
	/// Bailing is rather disincentivized, as it can allow attackers to submit bad solutions, but get away with it last minute. We only return 25% of the deposit in case someone bails. In Polkadot, this value will be lower or simply zero.
	pub BailoutGraceRatio: Perbill = Perbill::from_percent(25);
	/// Invulnerable miners will pay this deposit only.
	pub InvulnerableFixedDeposit: Balance = UNITS;
	/// Being ejected is already paid for by the new submitter replacing you; no need to charge deposit.
	pub EjectGraceRatio: Perbill = Perbill::from_percent(0);
	/// .2 KSM as the reward for the best signed submission.
	pub RewardBase: Balance = UNITS / 5;
}

impl multi_block::signed::Config for Runtime {
	type Currency = Balances;
	type BailoutGraceRatio = BailoutGraceRatio;
	type EjectGraceRatio = EjectGraceRatio;
	type DepositBase = DepositBase;
	type DepositPerPage = DepositPerPage;
	type InvulnerableDeposit = InvulnerableFixedDeposit;
	type RewardBase = RewardBase;
	type MaxSubmissions = MaxSignedSubmissions;
	type EstimateCallFee = TransactionPayment;
	type WeightInfo = weights::pallet_election_provider_multi_block_signed::WeightInfo<Self>;
}

parameter_types! {
	/// Priority of the "offchain" miner transactions.
	pub MinerTxPriority: TransactionPriority = TransactionPriority::max_value() / 2;

	/// Offchain miner transaction can fill up to 75% of the block size.
	pub MinerMaxLength: u32 = Perbill::from_rational(75u32, 100) *
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
	type Solver = <Runtime as multi_block::unsigned::Config>::OffchainSolver;
	type Pages = Pages;
	type Solution = NposCompactSolution24;
	type VoterSnapshotPerBlock = <Runtime as multi_block::Config>::VoterSnapshotPerBlock;
	type TargetSnapshotPerBlock = <Runtime as multi_block::Config>::TargetSnapshotPerBlock;
}

// AUDIT: This is the inflation formula of Kusama prior to
pub struct EraPayout;
impl pallet_staking_async::EraPayout<Balance> for EraPayout {
	fn era_payout(
		total_staked: Balance,
		_total_issuance: Balance,
		era_duration_millis: u64,
	) -> (Balance, Balance) {
		const MILLISECONDS_PER_YEAR: u64 = 1000 * 3600 * 24 * 36525 / 100;

		// TOOD @kianenigma one sanity check test for this as we had in the RC
		// TODO @kianenigma: use parameters pallet
		let params = polkadot_runtime_common::impls::EraPayoutParams {
			total_staked,
			total_stakable: Balances::total_issuance(),
			ideal_stake: Perquintill::from_percent(75),
			max_annual_inflation: Perquintill::from_percent(10),
			min_annual_inflation: Perquintill::from_rational(25u64, 1000),
			falloff: Perquintill::from_percent(5),
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

// See: TODO @kianenigma
// https://github.com/paseo-network/runtimes/blob/7904882933075551e23d32d86dbb97b971e84bca/relay/paseo/src/lib.rs#L662
// https://github.com/paseo-network/runtimes/blob/7904882933075551e23d32d86dbb97b971e84bca/relay/paseo/constants/src/lib.rs#L49
parameter_types! {
	pub const SessionsPerEra: SessionIndex = 6;
	/// Note: This is measured in RC block time. Our calculation of when to plan a new era might get
	/// confused in case AH block times change. Ideally, this value should be updated alongside AH's
	/// block time. If AH blocks progress faster, our eras will become shorter, which is not a
	/// critical issue.
	pub const RelaySessionDuration: BlockNumber = 1 * HOURS;
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

	/// This is the upper bound on how much we are willing to inflate per era. We also emit a
	/// warning event in case an era is longer than this amount.
	///
	/// Under normal conditions, this upper bound is never needed. Yet, since this is the first
	/// deployment of pallet-staking-async, eras might become longer due to misconfiguration, and we
	/// don't want to reduce the validator payouts by too much because of this. Therefore, we allow
	/// each era to be at most 2x the expected value
	pub const MaxEraDuration: u64 = 2 * (
		// the expected era duration in milliseconds.
		RelaySessionDuration::get() as u64 * RELAY_CHAIN_SLOT_DURATION_MILLIS as u64 * SessionsPerEra::get() as u64
	);
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
	type PlanningEraOffset =
		pallet_staking_async::PlanningEraOffsetOf<Self, RelaySessionDuration, ConstU32<10>>;
	type RcClientInterface = StakingRcClient;
	type MaxEraDuration = MaxEraDuration;
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
		// TODO: after https://github.com/paritytech/polkadot-sdk/pull/9619, use `XCMSender::send`
		let message = ValidatorSetToXcm::convert(report);
		let dest = RelayLocation::get();
		let _ = crate::send_xcm::<xcm_config::XcmRouter>(dest, message).inspect_err(|err| {
			log::error!(target: "runtime::ah-client", "Failed to send validator set report: {:?}", err);
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
		<UncheckedExtrinsic as TypeInfo>::Identity::new_transaction(call, extension).into()
	}
}

impl<LocalCall> frame_system::offchain::CreateBare<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_bare(call: RuntimeCall) -> UncheckedExtrinsic {
		<UncheckedExtrinsic as TypeInfo>::Identity::new_bare(call).into()
	}
}

pub struct InitiateStakingAsync<const EXPECTED_SPEC: u32>;
impl<const EXPECTED_SPEC: u32> frame_support::traits::OnRuntimeUpgrade
	for InitiateStakingAsync<EXPECTED_SPEC>
{
	fn on_runtime_upgrade() -> Weight {
		if crate::VERSION.spec_version == EXPECTED_SPEC {
			use pallet_election_provider_multi_block::verifier::Verifier;
			// set parity staking miner as the invulnerable submitter in `multi-block`.
			// https://kusama.subscan.io/account/GtGGqmjQeRt7Q5ggrjmSHsEEfeXUMvPuF8mLun2ApaiotVr
			let acc = hex_literal::hex!(
				"bea06e6ad606b2a80822a72aaae84a9a80bec27f1beef1880ad4970b72227601"
			);
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
			// * sum_stake_squared: 67_504_538_161_651_736_253_970_267_717_229_279 (0.8 the minimum,
			//   the lower the better)
			let minimum_score = sp_npos_elections::ElectionScore {
				minimal_stake: 2957640724907066,
				sum_stake: 3471819933857856584,
				sum_stake_squared: 78133097080615021100202963085417458,
			};
			<Runtime as multi_block::Config>::Verifier::set_minimum_score(minimum_score);

			<Runtime as frame_system::Config>::DbWeight::get().writes(2)
		} else {
			Default::default()
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	// TODO: @kianenigma
	mod message_sizes {
		use super::*;
	}

	mod incoming_xcm_weights {
		#[test]
		fn offence_report() {
			todo!("@kianenigma")
		}

		#[test]
		fn session_report() {
			todo!("@kianenigma")
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
		use sp_runtime::Percent;
		use sp_weights::constants::{WEIGHT_PROOF_SIZE_PER_KB, WEIGHT_REF_TIME_PER_MILLIS};

		fn analyze_weight(
			op_name: &str,
			op_weight: Weight,
			limit_weight: Weight,
			maybe_max_ratio: Option<Percent>,
		) {
			sp_tracing::try_init_simple();
			let ref_time_ms = op_weight.ref_time() / WEIGHT_REF_TIME_PER_MILLIS;
			let ref_time_ratio =
				Percent::from_rational(op_weight.ref_time(), limit_weight.ref_time());
			let proof_size_kb = op_weight.proof_size() / WEIGHT_PROOF_SIZE_PER_KB;
			let proof_size_ratio =
				Percent::from_rational(op_weight.proof_size(), limit_weight.proof_size());
			let limit_ms = limit_weight.ref_time() / WEIGHT_REF_TIME_PER_MILLIS;
			let limit_kb = limit_weight.proof_size() / WEIGHT_PROOF_SIZE_PER_KB;
			log::info!(target: "runtime::asset-hub-kusama", "weight of {:?} is: ref-time: {}ms, {:?} of total, proof-size: {}KiB, {:?} of total (total: {}ms, {}KiB)",
				op_name,
				ref_time_ms,
				ref_time_ratio,
				proof_size_kb,
				proof_size_ratio,
				limit_ms,
				limit_kb
			);

			if let Some(max_ratio) = maybe_max_ratio {
				assert!(ref_time_ratio <= max_ratio && proof_size_ratio <= max_ratio,)
			}
		}

		#[test]
		fn snapshot_msp_weight() {
			use multi_block::WeightInfo;
			analyze_weight(
				"snapshot_msp",
				<Runtime as multi_block::Config>::WeightInfo::on_initialize_into_snapshot_msp(),
				<Runtime as frame_system::Config>::BlockWeights::get().max_block,
				Some(Percent::from_percent(70)),
			);
		}

		#[test]
		fn snapshot_rest_weight() {
			use multi_block::WeightInfo;
			analyze_weight(
				"snapshot_rest",
				<Runtime as multi_block::Config>::WeightInfo::on_initialize_into_snapshot_rest(),
				<Runtime as frame_system::Config>::BlockWeights::get().max_block,
				Some(Percent::from_percent(70)),
			);
		}

		#[test]
		fn verifier_weight() {
			use multi_block::verifier::WeightInfo;
			analyze_weight(
				"verifier valid terminal",
				<Runtime as multi_block::verifier::Config>::WeightInfo::on_initialize_valid_terminal(),
				<Runtime as frame_system::Config>::BlockWeights::get().max_block,
				Some(Percent::from_percent(70)),
			);

			analyze_weight(
				"verifier invalid terminal",
				<Runtime as multi_block::verifier::Config>::WeightInfo::on_initialize_invalid_terminal(),
				<Runtime as frame_system::Config>::BlockWeights::get().max_block,
				Some(Percent::from_percent(70)),
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
				Some(Percent::from_percent(70)),
			);
			analyze_weight(
				"full solution cleanup",
				<Runtime as multi_block::signed::Config>::WeightInfo::clear_old_round_data(
					Pages::get(),
				)
				.mul(16 as u64),
				<Runtime as frame_system::Config>::BlockWeights::get().max_block,
				Some(Percent::from_percent(70)),
			);
		}

		#[test]
		fn export_weight() {
			// TODO @kianenigma this fails now, but should be fine after we re-benchmark the pallet.
			use multi_block::WeightInfo;
			analyze_weight(
				"export terminal",
				<Runtime as multi_block::Config>::WeightInfo::export_terminal(),
				<Runtime as frame_system::Config>::BlockWeights::get().max_block,
				Some(Percent::from_percent(70)),
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
