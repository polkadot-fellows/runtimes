// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The Individuality SDK on People Polkadot.
//!
//! This module configures the whole [Individuality](https://github.com/paritytech/individuality)
//! pallet set that lives on the People Chain. It is a port of the `next-people-paseo` reference
//! runtime of that repository; the configuration values are kept identical wherever they are not
//! network specific.
//!
//! TODO: audit every hard-written value in this module before release. Keeping the reference
//! runtime's values made the port reviewable, but "identical to Paseo" is the wrong default for the
//! ones that carry real value or real time:
//!
//! * Currency-denominated constants mean something different here. A literal such as `2 * UNITS` is
//!   2 PAS on the reference chain and 2 DOT here — same number, ~3 orders of magnitude apart in
//!   value.
//! * Constants counted in blocks must use this chain's 2s block time; see [`time`] and
//!   [`relay_time`] for the two paces in play, and pick deliberately between them.
//! * Anything denominated in the backing stablecoin ties this chain's economics to
//!   [`StableAssetLocation`], a Hydration-issued asset.
//!
//! This is expected to be revisited in a follow-up refactor; the individual sites are marked with
//! their own TODOs.
//!
//! # The pieces
//!
//! * [`indiv_pallet_chunks_manager`] holds the Bandersnatch ring-VRF SRS. Everything below that
//!   proves ring membership needs it, and it must be uploaded before any ring root can be built.
//! * [`indiv_pallet_members`] is the ring-membership service: it owns member collections, bakes
//!   ring roots, and verifies ring-VRF proofs against them. Every "prove you are a member of set X
//!   without saying which member" flow goes through it.
//! * [`indiv_pallet_members_notifier`] publishes those ring roots to subscribing parachains (Asset
//!   Hub) over XCM so that they can verify the same proofs locally.
//! * [`indiv_pallet_people`] is the personhood registry proper: one ring per generation of proven
//!   people, plus context-scoped aliases (`PersonalAlias`) and the `PersonalIdentity` origin.
//! * [`indiv_pallet_people_lite`] is the weaker, device-attestation based flavour of personhood.
//! * [`indiv_pallet_game`] and [`indiv_pallet_score`] implement the in-person meetup game which
//!   builds up a personhood score, with [`indiv_pallet_airdrop`] handing out prizes and game
//!   reports recording NFT claim credits for Asset Hub to mint against.
//! * [`indiv_pallet_honour`] lets people vote on calls with their personhood weight.
//! * [`indiv_pallet_resources`] rations the off-chain resources (statement store, notifications,
//!   long-term storage) a person may consume.
//! * [`indiv_pallet_coinage`] implements bearer-instrument "coins" backed by a stablecoin, held by
//!   ring aliases rather than accounts.
//! * [`indiv_pallet_dummy_dim`] is the governance-driven DIM: it lets the root origin recognise
//!   personhood directly.
//! * [`indiv_pallet_origin_restriction`] rate-limits the anonymous origins the extensions above
//!   produce, since those origins pay no fee from an account.
//! * [`indiv_pallet_relay_randomness`] surfaces relay chain randomness, used to seed airdrop draws.
//!
//! # Not deployed here
//!
//! `indiv-pallet-storage-initialization` is a bootstrapping pallet for fresh development chains
//! (it funds hard-coded development accounts and seeds a first game), so it has no place on
//! Polkadot.
//!
//! `indiv-pallet-mob-rule`, the juror pallet, is not deployed either. Its cases can only be opened
//! through its `StatementOracle` implementation, and the only pallet that calls it in the reference
//! runtime is `indiv-pallet-proof-of-ink`, which is not deployed here — so no case could ever be
//! created and the whole pallet would be inert. It has to come back together with a pallet that
//! judges statements.
//!
//! # Deployment steps
//!
//! Enacting the runtime upgrade activates none of this on its own: the ring-VRF machinery is inert
//! until the SRS is on chain, Asset Hub cannot verify anything until it is subscribed, and the
//! value-carrying flows need their assets and accounts set up. In order:
//!
//! 1. `ChunksManager::set_chunk_page_hashes` (Fellowship or root) — commit the expected hash of
//!    each SRS chunk page, per ring exponent. This must come first: `add_chunks` rejects any page
//!    that has no committed hash to match against. Needed for every exponent this runtime uses:
//!    [`MembersFlexibleRingExponent`] and [`LitePeopleRingExponent`] (`R2e9`), plus
//!    [`RecyclerRingExponent`] and [`PaidUnloadTokenRingExponent`] (`R2e10`) for coinage.
//! 2. `ChunksManager::add_chunks` — upload the chunk pages themselves. This call is *permissionless
//!    and authorized*, not root: its validity comes from the page hashing to the committed value,
//!    so anyone can supply the data.
//! 3. `MembersNotifier::subscribe` (Fellowship or root) — register Asset Hub Polkadot (para 1000)
//!    as a ring-root subscriber, listing the collections it needs (the people and lite-people
//!    identifiers with their exponents, in strictly ascending identifier order) and `pallet_index`
//!    = the `MembersSubscriber` index in Asset Hub Polkadot's `construct_runtime!` (97). Until this
//!    runs, Asset Hub has no ring roots and every personhood proof there fails. Requires an open
//!    HRMP channel in both directions.
//! 4. `Assets::force_create` (root) for [`StableAssetLocation`], unless HOLLAR is already
//!    registered locally. `CreateOrigin` is `EnsureNever` on this chain, so root is the only way.
//!    This is a prerequisite for step 6, which rejects an unknown asset.
//! 5. `Assets::force_set_metadata` (root) — set the HOLLAR asset metadata after creating it.
//! 6. Mint the coinage pallet account at least HOLLAR's minimum balance, then call
//!    `Coinage::create_sufficient_instance` (Fellowship or root) with [`StableAssetLocation`] and
//!    its `$0.01` asset unit. The public pallet uses explicit, permanent instances rather than a
//!    single globally selected backing asset.
//! 7. Fund the pallet-derived accounts that pay out: [`GameAirdropSource`] (`pop/gads`) with the
//!    airdrop asset, and the [`ScorePotId`] (`scorepot`) pot for score cash-outs. Both are derived
//!    accounts nobody controls, so they can only be funded by transfer.
//! 8. `Game::schedule_games` (Fellowship or root) — no meetup game exists until one is scheduled,
//!    so `pallet-game` and `pallet-score` stay dormant without this.
//! 9. `People::create_people_collection` (Fellowship or root) — create the people collection; this
//!    is not done by the runtime upgrade and must precede people onboarding.
//!
//! Optional, per-provider: `PeopleLite::set_attestation_allowance` (Fellowship or root) to admit
//! a device-attestation provider, and `DummyDim`'s recognition calls (Fellowship or root) to grant
//! personhood directly.

use super::*;

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use cumulus_primitives_core::ParaId;
#[cfg(not(feature = "runtime-benchmarks"))]
use frame_support::traits::NeverEnsureOrigin;
use frame_support::{
	parameter_types,
	traits::{
		fungible::{HoldConsideration, ItemOf},
		tokens::ConversionToAssetBalance,
		AsEnsureOriginWithArg, ConstBool, ConstU128, ConstUint, ConstantStoragePrice, ContainsPair,
		Get, PalletInfoAccess,
	},
	weights::WeightToFee,
};
use indiv_pallet_origin_restriction::Allowance;
use indiv_support::{
	crypto::{BandersnatchVrfVerifiable, GenerateVerifiable},
	fungibles::CombineAssetsWithHolder,
	traits::{Alias, AllocateStorage, Context, Identifier, RevisionIndex, RingExponent, RingIndex},
	utils::TypedGetToGet,
};
#[cfg(feature = "runtime-benchmarks")]
use polkadot_runtime_constants::system_parachain::ASSET_HUB_ID;
use scale_info::TypeInfo;
#[cfg(feature = "runtime-benchmarks")]
use sp_runtime::MultiSigner;
use sp_runtime::{
	traits::{AccountIdConversion, ConstI8, ConstU16, Convert, Verify},
	DispatchError, DispatchResult, MultiSignature,
};
use sp_statement_store::StatementAllowance;
// NOTE: deliberately not `xcm::latest::prelude::*` — its `Assets` would shadow the `Assets` pallet
// this module configures.
#[cfg(feature = "runtime-benchmarks")]
use xcm::latest::Junction::GeneralIndex;
#[cfg(feature = "runtime-benchmarks")]
use xcm::latest::Junction::Parachain;
use xcm::latest::{
	send_xcm,
	Instruction::{Transact, UnpaidExecution},
	Junction,
	Junction::PalletInstance,
	Location, OriginKind, WeightLimit, Xcm,
};

use crate::{
	assets::hollar::{HollarLocation, HOLLAR_UNITS},
	parameters::dynamic_params,
};

/// Wall-clock durations expressed in block numbers.
///
/// People Polkadot authors a block every
/// `RELAY_CHAIN_SLOT_DURATION_MILLIS / BLOCK_PROCESSING_VELOCITY` = 2s under elastic scaling. The
/// `MINUTES`/`HOURS`/`DAYS` re-exported by `parachains_common` assume a 12s block and so cannot be
/// used for the durations below.
pub mod time {
	use super::BlockNumber;
	use system_parachains_constants::polkadot::consensus::{
		elastic_scaling::BLOCK_PROCESSING_VELOCITY, RELAY_CHAIN_SLOT_DURATION_MILLIS,
	};

	/// Target block time, in milliseconds.
	pub const MILLISECS_PER_BLOCK: BlockNumber =
		RELAY_CHAIN_SLOT_DURATION_MILLIS / BLOCK_PROCESSING_VELOCITY;

	pub const MINUTES: BlockNumber = 60_000 / MILLISECS_PER_BLOCK;
	pub const HOURS: BlockNumber = MINUTES * 60;
	pub const DAYS: BlockNumber = HOURS * 24;
}

/// Wall-clock durations expressed in *relay chain* block numbers.
///
/// For the few parameters that are compared against a `RelaychainDataProvider` block number rather
/// than this chain's own. Mixing the two silently scales the duration by 3x.
pub mod relay_time {
	use super::BlockNumber;
	use system_parachains_constants::polkadot::consensus::RELAY_CHAIN_SLOT_DURATION_MILLIS;

	pub const MINUTES: BlockNumber = 60_000 / RELAY_CHAIN_SLOT_DURATION_MILLIS;
	pub const HOURS: BlockNumber = MINUTES * 60;
	pub const DAYS: BlockNumber = HOURS * 24;
}

/// The stablecoin backing the value-carrying flows of this module: juror rewards, score cash-outs
/// and, once nominated by root, coins.
///
/// HOLLAR is the only non-native fungible People Polkadot accepts (see
/// `xcm_config::XcmConfig::IsReserve`), and it already has a `pallet-asset-rate` entry because the
/// XCM trader prices weight in it.
pub type StableAssetLocation = HollarLocation;

/// The full-featured fungibles implementation, combining `pallet-assets` balances with the hold
/// functionality supplied by `pallet-assets-holder`.
///
/// `pallet-assets` alone does not implement `fungibles::MutateHold`, which coinage requires in
/// order to lock the asset backing a coin.
pub type AssetsWithHolder = CombineAssetsWithHolder<Assets, AssetsHolder>;

/// A `fungible` (single-asset) view of [`StableAssetLocation`], with hold support.
pub type FungibleStableAsset = ItemOf<AssetsWithHolder, StableAssetLocation, AccountId>;

/// Wall-clock source used by the pallet `Config`s in this module.
///
/// `pallet_timestamp`'s `UnixTime::now` logs at `error` level whenever `Now` is still zero. That is
/// the normal state throughout benchmarking, which runs at genesis and sets `Now` directly, so the
/// real provider would emit that error on essentially every call and bury the benchmark output.
/// Under `runtime-benchmarks` we therefore read the raw storage value instead. Both paths return
/// the same value; only the logging differs.
#[cfg(not(feature = "runtime-benchmarks"))]
pub type RuntimeClock = Timestamp;
#[cfg(feature = "runtime-benchmarks")]
pub type RuntimeClock = benchmark_utils::BenchmarkClock;

parameter_types! {
	/// Page size for the ring-VRF SRS chunk storage.
	pub const ChunkPageSize: u32 = 255;

	/// Largest ring exponent usable by `Flexible` collections in `pallet-members`, which is also
	/// the exponent of the people ring. `R2e9` reserves 257 of the 2^9 domain slots for
	/// proof-system overhead (blinding and internal rows), giving an effective capacity of 255
	/// members per ring.
	pub const MembersFlexibleRingExponent: RingExponent = RingExponent::R2e9;

	/// Ring exponent for the lite people collection.
	pub const LitePeopleRingExponent: RingExponent = RingExponent::R2e9;

	/// Number of queued lite people onboarded into a ring at a time.
	pub const LitePeopleOnboardingSize: u32 = 3;

	/// Ring exponent for coinage's recycler collections.
	///
	/// NOTE: changing this on a live chain requires substantial migration work.
	pub const RecyclerRingExponent: RingExponent = RingExponent::R2e10;

	/// Ring exponent for coinage's paid unload token collections.
	pub const PaidUnloadTokenRingExponent: RingExponent = RingExponent::R2e10;

	/// How long a person must wait before they may prove membership of the ring they were just
	/// added to, in seconds. Without the delay, the act of joining would narrow the anonymity set
	/// down to the newest member.
	pub const SelfInclusionDelayValue: u64 = 300;

	/// Owner of the people collection in `pallet-members`. Set to the pallet's own location so
	/// that no other origin can manage it.
	///
	/// The index is read from the pallet itself rather than written out: this value identifies the
	/// owner of an existing ring collection, so if it ever drifted from the pallet's real index
	/// the collection would be silently orphaned.
	pub PeopleCollectionOwner: Location =
		Location::new(0, [PalletInstance(<People as PalletInfoAccess>::index() as u8)]);

	/// Owner of the lite people collection. See [`PeopleCollectionOwner`].
	pub LitePeopleCollectionOwner: Location =
		Location::new(0, [PalletInstance(<PeopleLite as PalletInfoAccess>::index() as u8)]);

	/// Owner of the coinage collections. See [`PeopleCollectionOwner`].
	pub CoinageCollectionOwner: Location =
		Location::new(0, [PalletInstance(<Coinage as PalletInfoAccess>::index() as u8)]);
}

impl indiv_pallet_relay_randomness::Config for Runtime {
	type WeightInfo = weights::indiv_pallet_relay_randomness::WeightInfo<Runtime>;
}

impl indiv_pallet_chunks_manager::Config for Runtime {
	type WeightInfo = weights::indiv_pallet_chunks_manager::WeightInfo<Runtime>;
	type Chunk = <BandersnatchVrfVerifiable as GenerateVerifiable>::StaticChunk;
	type PageSize = ChunkPageSize;
	type ManagerOrigin = RootOrFellows;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = benchmark_utils::ChunksManagerBenchHelper;
}

impl indiv_pallet_members::Config for Runtime {
	type WeightInfo = weights::indiv_pallet_members::WeightInfo<Runtime>;
	type Crypto = BandersnatchVrfVerifiable;
	type Location = Location;
	type ChunksManager = ChunksManager;
	type Clock = RuntimeClock;
	type MaxCollections = ConstU32<100>;
	type OnboardingQueuePageSize = ConstU32<255>;
	type MaxFlexibleRingExponent = MembersFlexibleRingExponent;
	type RingBuildingMemberLimit = ConstU32<60>;
	/// 10 minutes, so proofs against a superseded root stay valid for a grace period.
	type OldRootRetentionDuration = ConstU64<600>;
	type OnRingRootChange = MembersNotifier;
	type OffchainWorkerInterval = ConstU32<1>;
	type ManagerOrigin = RootOrFellows;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = benchmark_utils::MembersBenchHelper;
}

/// The alias contexts this runtime supports, i.e. the ones for which a person may derive a
/// `PersonalAlias`. Each corresponds to a pallet that keys state by alias.
pub struct AccountContexts;
impl frame_support::traits::Contains<Context> for AccountContexts {
	fn contains(context: &Context) -> bool {
		context == &indiv_pallet_score::SCORE_CONTEXT ||
			context == &indiv_pallet_resources::RESOURCES_CONTEXT
	}
}

parameter_types! {
	pub const StaleAliasCleanupInterval: BlockNumber = 5 * time::MINUTES;
}

impl indiv_pallet_people::Config for Runtime {
	type WeightInfo = weights::indiv_pallet_people::WeightInfo<Runtime>;
	type MemberService = Members;
	type RingExponent = MembersFlexibleRingExponent;
	type CollectionOwner = PeopleCollectionOwner;
	type AccountContexts = AccountContexts;
	type OnboardingQueuePageSize = ConstU32<30>;
	type StaleAliasCleanupInterval = StaleAliasCleanupInterval;
	type SelfInclusionDelay = SelfInclusionDelayValue;
	type ManagerOrigin = RootOrFellows;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = benchmark_utils::PeopleBenchHelper;
}

impl indiv_pallet_people_lite::Config for Runtime {
	type WeightInfo = indiv_pallet_people_lite::weights::SubstrateWeight<Runtime>;
	type AttestationAllowanceManager = RootOrFellows;
	type MemberService = Members;
	type CollectionOwner = LitePeopleCollectionOwner;
	type LiteRingExponent = LitePeopleRingExponent;
	type LiteOnboardingSize = LitePeopleOnboardingSize;
	type AttestationSignature = Signature;
	type LiteConsumerRegistrar = Resources;
	type AccountContexts = AccountContexts;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

impl indiv_pallet_dummy_dim::Config for Runtime {
	type WeightInfo = weights::indiv_pallet_dummy_dim::WeightInfo<Runtime>;
	type UpdateOrigin = RootOrFellows;
	type MaxPersonBatchSize = ConstU32<1000>;
	type People = People;
}

parameter_types! {
	pub const ScorePotId: PalletId = PalletId(*b"scorepot");
}

impl indiv_pallet_score::Config for Runtime {
	type WeightInfo = weights::indiv_pallet_score::WeightInfo<Runtime>;
	type EnsurePerson = indiv_pallet_people::EnsurePersonalAliasInContext<Runtime>;
	type ScorePotId = ScorePotId;
	type Currency = FungibleStableAsset;
	type CurrencyLocationInfo = StableAssetLocation;
	type ManagerOrigin = RootOrFellows;
	type MaxPayoutRoundSchedules = ConstU32<10>;
	type OffchainWorkInterval = ConstU32<2>;
	type People = People;
	type Crypto = BandersnatchVrfVerifiable;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = benchmark_utils::ScoreBenchmarkHelper;
}

parameter_types! {
	pub NftsPalletFeatures: pallet_nfts::PalletFeatures = pallet_nfts::PalletFeatures::all_enabled();
	/// One year, counted in *relay chain* blocks: `pallet-nfts` below sets
	/// `BlockNumberProvider = RelaychainDataProvider`, so this must use the relay pace (6s) and not
	/// this chain's own 2s [`time`] constants.
	pub const NftsMaxDeadlineDuration: BlockNumber = 12 * 30 * relay_time::DAYS;
}

// All deposits are zero: the attestation NFT collection is owned by a pallet-derived account with
// no funds and minting must be free. Public collection creation is closed off via `CreateOrigin`
// instead.
#[cfg(not(feature = "runtime-benchmarks"))]
parameter_types! {
	pub const NftsCollectionDeposit: Balance = 0;
	pub const NftsItemDeposit: Balance = 0;
	pub const NftsMetadataDepositBase: Balance = 0;
	pub const NftsAttributeDepositBase: Balance = 0;
	pub const NftsDepositPerByte: Balance = 0;
}

// The upstream `redeposit` benchmark asserts deposit updates, which zero deposits can never
// produce, so the benchmarks run with Asset Hub-like deposit amounts. This slightly overweighs the
// zero-deposit production paths, which is the conservative direction.
#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub const NftsCollectionDeposit: Balance = system_para_deposit(1, 130);
	pub const NftsItemDeposit: Balance = system_para_deposit(1, 164) / 40;
	pub const NftsMetadataDepositBase: Balance = system_para_deposit(1, 129) / 10;
	pub const NftsAttributeDepositBase: Balance = system_para_deposit(1, 0) / 10;
	pub const NftsDepositPerByte: Balance = system_para_deposit(0, 1);
}

/// A `pallet-nfts` instance compatible with the Asset Hub one (`u32` collection and item
/// identifiers) so items can eventually be transferred there via XCM. Public collection creation is
/// disabled: collections are created internally (by the game) or by the root origin via
/// `force_create`.
impl pallet_nfts::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type CollectionId = u32;
	type ItemId = u32;
	type Currency = Balances;
	// The benchmarks exercise the public `create` call, so they need an origin that can succeed.
	#[cfg(not(feature = "runtime-benchmarks"))]
	type CreateOrigin = AsEnsureOriginWithArg<NeverEnsureOrigin<AccountId>>;
	#[cfg(feature = "runtime-benchmarks")]
	type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type Locker = ();
	type CollectionDeposit = NftsCollectionDeposit;
	type ItemDeposit = NftsItemDeposit;
	type MetadataDepositBase = NftsMetadataDepositBase;
	type AttributeDepositBase = NftsAttributeDepositBase;
	type DepositPerByte = NftsDepositPerByte;
	type StringLimit = ConstU32<256>;
	type KeyLimit = ConstU32<64>;
	type ValueLimit = ConstU32<256>;
	type ApprovalsLimit = ConstU32<20>;
	type ItemAttributesApprovalsLimit = ConstU32<30>;
	type MaxTips = ConstU32<10>;
	type MaxDeadlineDuration = NftsMaxDeadlineDuration;
	type MaxAttributesPerCall = ConstU32<10>;
	type Features = NftsPalletFeatures;
	type OffchainSignature = Signature;
	type OffchainPublic = <Signature as Verify>::Signer;
	type WeightInfo = weights::pallet_nfts::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = ();
	type BlockNumberProvider = RelaychainDataProvider<Runtime>;
}

parameter_types! {
	pub const PlayDepositReason: RuntimeHoldReason =
		RuntimeHoldReason::Game(indiv_pallet_game::HoldReason::PlayDeposit);
	pub const PlayDepositDefault: Balance = 5 * UNITS;
	pub PlayerStatementLimit: StatementAllowance =
		StatementAllowance { max_size: 1_000_000, max_count: 1_000_000 };
	pub GameAirdropSource: AccountId = PalletId(*b"pop/gads").into_account_truncating();
}

/// Duration of each phase of a game, in seconds.
pub struct GamePhaseDurations;
impl Get<indiv_pallet_game::PhaseDurationValues> for GamePhaseDurations {
	fn get() -> indiv_pallet_game::PhaseDurationValues {
		indiv_pallet_game::PhaseDurationValues {
			registration: 5 * 60,
			shuffle: 60,
			post_shuffle_margin: 30,
			reporting: 10 * 60,
			player_process: 60,
		}
	}
}

const PRODUCTION_MAX_GROUP_SIZE: u32 = 6;
const BENCHMARK_MAX_GROUP_SIZE: u32 = 10;
const PRODUCTION_MAX_ROUNDS: u32 = 3;
const BENCHMARK_MAX_ROUNDS: u32 = 10;
const _: () = assert!(PRODUCTION_MAX_GROUP_SIZE <= BENCHMARK_MAX_GROUP_SIZE);
const _: () = assert!(PRODUCTION_MAX_ROUNDS <= BENCHMARK_MAX_ROUNDS);

impl indiv_pallet_game::Config for Runtime {
	type WeightInfo = weights::indiv_pallet_game::WeightInfo<Runtime>;
	// The game benchmarks sweep `1..=MaxGroupSize` and `1..=MaxRounds` for their linear
	// regressions. The production bounds (6/3) are too small to fit accurate per-player and
	// per-round slopes, so the benchmarking build widens them to 10. The fitted weight formulas
	// stay valid at the production bounds, which only interpolate within the measured range.
	#[cfg(not(feature = "runtime-benchmarks"))]
	type MaxGroupSize = ConstU32<PRODUCTION_MAX_GROUP_SIZE>;
	#[cfg(feature = "runtime-benchmarks")]
	type MaxGroupSize = ConstU32<BENCHMARK_MAX_GROUP_SIZE>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type MaxRounds = ConstU32<PRODUCTION_MAX_ROUNDS>;
	#[cfg(feature = "runtime-benchmarks")]
	type MaxRounds = ConstU32<BENCHMARK_MAX_ROUNDS>;
	type UnixTime = RuntimeClock;
	type ManagerOrigin = RootOrFellows;
	type InviteIssuer = RootOrFellows;
	type EnsureLiteAlias = indiv_pallet_people_lite::EnsureLiteAliasInContext<Runtime>;
	type NonPlayingKickoutTime = ConstU32<{ 90 * time::DAYS }>;
	type NativeFungible = Balances;
	type PlayDeposit = HoldConsideration<
		AccountId,
		Balances,
		PlayDepositReason,
		sp_runtime::traits::Identity,
		Balance,
	>;
	type DefaultPlayDeposit = PlayDepositDefault;
	type TicketSignature = MultiSignature;
	type MaxGameSchedules = ConstU32<12>;
	type MaxAttendanceHistoryDepth = ConstU32<12>;
	type NftClaimCredits = NftCredits;
	type DefaultPhaseDurations = GamePhaseDurations;
	type AccountSignature = Signature;
	type PlayerStatementLimit = PlayerStatementLimit;
	type PeopleVoteWeight = ConstUint<2>;
	type CandidateVoteWeight = ConstUint<1>;
	/// A group of one cannot corroborate anything, so a real game needs at least two players per
	/// group. Only the development runtimes lower this to zero.
	type MinGroupSize = ConstUint<2>;
	type AirdropAssetId = <Runtime as pallet_assets::Config>::AssetId;
	type AirdropAssetBalance = Balance;
	type Airdrop = Airdrop;
	type AirdropSource = GameAirdropSource;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = benchmark_utils::GamePalletBenchmarkHelper;
}

/// What the NFT-credit benchmarks cannot set up themselves: only the runtime knows how its HRMP
/// channels are represented in the parachain-system messaging snapshot.
#[cfg(feature = "runtime-benchmarks")]
pub struct NftCreditsBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl indiv_pallet_nft_credits::benchmarking::BenchmarkHelper for NftCreditsBenchmarkHelper {
	fn open_nft_claims_channel(max_message_size: u32) {
		use cumulus_pallet_parachain_system::RelevantMessagingState;
		use cumulus_primitives_core::relay_chain::AbridgedHrmpChannel;

		let channel = AbridgedHrmpChannel {
			max_capacity: 1000,
			max_total_size: 1_000_000,
			max_message_size,
			msg_count: 0,
			total_size: 0,
			mqc_head: None,
		};
		let claims_chain = <Runtime as indiv_pallet_nft_credits::Config>::NftClaimsParaId::get();
		let mut messaging_state = RelevantMessagingState::<Runtime>::get().unwrap_or(
			cumulus_pallet_parachain_system::relay_state_snapshot::MessagingStateSnapshot {
				dmq_mqc_head: Default::default(),
				relay_dispatch_queue_remaining_capacity: Default::default(),
				ingress_channels: Vec::new(),
				egress_channels: Vec::new(),
			},
		);
		messaging_state.egress_channels.retain(|(id, _)| *id != claims_chain);
		messaging_state.egress_channels.push((claims_chain, channel));
		messaging_state.egress_channels.sort_by_key(|(id, _)| *id);
		RelevantMessagingState::<Runtime>::put(messaging_state);
	}
}

parameter_types! {
	/// Upper bound on the remote Asset Hub execution cost for one delivered credit tree.
	pub NftClaimsRemoteWeight: Weight = Weight::from_parts(150_000_000, 2_600);
}

impl indiv_pallet_nft_credits::Config for Runtime {
	type WeightInfo = indiv_pallet_nft_credits::weights::SubstrateWeight<Runtime>;
	// Sized conservatively above what a full People Polkadot block can award. The pallet's
	// integrity test verifies this against the runtime's block limits and game weights.
	type MaxCreditsPerBlock = ConstU32<3000>;
	type XcmRouter = xcm_config::XcmRouter;
	type NftClaimsParaId = polkadot_runtime_constants::system_parachain::AssetHubParaId;
	// Reserved for `NftClaims` in the corresponding Asset Hub runtime.
	type NftClaimsPalletIndex = ConstU8<96>;
	type ChannelInfo = ParachainSystem;
	type MaxQueuedCreditTrees = ConstU32<256>;
	type MaxCreditTreesPerMessage = ConstU32<32>;
	type NftClaimsRemoteWeight = NftClaimsRemoteWeight;
	type MaxCreditBlocksPerClaimant = ConstU32<32>;
	type MaxRetainedAwardBlocks = ConstU32<256>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = NftCreditsBenchmarkHelper;
}

parameter_types! {
	pub const HonourPointFreezeDuration: indiv_pallet_honour::Seconds = 24 * 60 * 60;
	pub const HonourCallMortality: indiv_pallet_honour::Seconds = 5 * 60;
}

impl indiv_pallet_honour::Config for Runtime {
	type WeightInfo = weights::indiv_pallet_honour::WeightInfo<Runtime>;
	type MemberService = Members;
	type Clock = Timestamp;
	type PointFreezeDuration = HonourPointFreezeDuration;
	type CallMortality = HonourCallMortality;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = benchmark_utils::HonourBenchmarkHelper;
}

parameter_types! {
	pub const AirdropPalletId: PalletId = PalletId(*b"pop/adrp");
}

impl indiv_pallet_airdrop::Config for Runtime {
	type WeightInfo = weights::indiv_pallet_airdrop::WeightInfo<Runtime>;
	type MemberService = Members;
	type Fungibles = AssetsWithHolder;
	type ManagerOrigin = RootOrFellows;
	type PalletId = AirdropPalletId;
	type UnixTime = RuntimeClock;
	// The pallet doesn't wait for the freshness of the randomness. It is used alongside
	// `indiv-pallet-game`, so a new player could register with new keys after the randomness is
	// publicly known and reach personhood in one game (when there are fewer than 5K players) and
	// win.
	type Randomness = indiv_pallet_relay_randomness::RelayBlockRandomness<Runtime>;
	type AccountIdToPublic = AccountIdToSr25519Public;
	type ClearLimit = ConstU32<100>;
	type DrawLimit = ConstU32<100>;
	type OffchainWorkerInterval = ConstU32<2>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = benchmark_utils::AirdropBenchmarkHelper;
}

/// Direct byte-level reinterpretation of an `AccountId32` as an sr25519 public key.
pub struct AccountIdToSr25519Public;
impl sp_runtime::traits::TryConvert<AccountId, sp_core::sr25519::Public>
	for AccountIdToSr25519Public
{
	fn try_convert(account: AccountId) -> Result<sp_core::sr25519::Public, AccountId> {
		let raw: [u8; 32] = account.clone().into();
		Ok(sp_core::sr25519::Public::from_raw(raw))
	}
}

parameter_types! {
	pub const MaxUsernameLength: u32 = 32;
	pub const MinUsernameLength: u32 = 6;
	pub const PersonAuthDuration: u32 = 2 * 24 * 60 * 60; // 2 days
	pub const MinPersonAuthUpdateInterval: u32 = 24 * 60 * 60; // 1 day
	pub const MaxReservationQueueLength: u32 = 10;
}

impl indiv_pallet_resources::Config for Runtime {
	type WeightInfo = weights::indiv_pallet_resources::WeightInfo<Runtime>;
	type MemberService = Members;
	type MinUsernameLength = MinUsernameLength;
	type PersonAuthDuration = PersonAuthDuration;
	type AccountsApiAllowance = crate::parameters::AccountsApiAllowance;
	type StmtStoreSlotsPerPeriod = dynamic_params::statement_storage::StmtStoreSlotsPerPeriod;
	type LiteStmtStoreSlotsPerPeriod =
		dynamic_params::statement_storage::LiteStmtStoreSlotsPerPeriod;
	type StmtStoreCleanupLimit = dynamic_params::statement_storage::StmtStoreCleanupLimit;
	type StmtStoreReplacementCooldown =
		dynamic_params::statement_storage::StmtStoreReplacementCooldown;
	type StmtStoreGraceWindow = dynamic_params::statement_storage::StmtStoreGraceWindow;
	type NotificationAllowance = crate::parameters::NotificationAllowance;
	type NotificationSlotsPerPeriod = dynamic_params::statement_storage::NotificationSlotsPerPeriod;
	type LiteNotificationSlotsPerPeriod =
		dynamic_params::statement_storage::LiteNotificationSlotsPerPeriod;
	type NotificationPeriodDuration = dynamic_params::statement_storage::NotificationPeriodDuration;
	type OffchainWorkerInterval = ConstU32<1>;
	type MinPersonAuthUpdateInterval = MinPersonAuthUpdateInterval;
	type EnsurePerson = indiv_pallet_people::EnsurePersonalAliasInContext<Runtime>;
	type EnsureLitePerson = indiv_pallet_people_lite::EnsureLitePerson<Runtime>;
	type Clock = RuntimeClock;
	type OffchainSignature = Signature;
	type LitePersonStatementLimit = crate::parameters::LitePersonStatementLimit;
	type PersonStatementLimit = crate::parameters::PersonStatementLimit;
	type MaxReservationQueueLength = MaxReservationQueueLength;
	type ManagerOrigin = RootOrFellows;
	type LongTermStoragePeriodDuration =
		dynamic_params::bulletin_storage::LongTermStoragePeriodDuration;
	type LongTermStorageGraceWindow = dynamic_params::bulletin_storage::LongTermStorageGraceWindow;
	type LongTermStorageClaimsPerPeriod =
		dynamic_params::bulletin_storage::LongTermStorageClaimsPerPeriod;
	type LongTermStorageAllowanceForPeople =
		dynamic_params::bulletin_storage::LongTermStorageAllowanceForPeople;
	type LongTermStorageAllowanceForLitePeople =
		dynamic_params::bulletin_storage::LongTermStorageAllowanceForLitePeople;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type LongTermStorageDataStore = BulletinDataStore;
	#[cfg(feature = "runtime-benchmarks")]
	type LongTermStorageDataStore = benchmark_utils::BenchmarkDataStore;
	type LongTermStorageCleanupLimit =
		dynamic_params::bulletin_storage::LongTermStorageCleanupLimit;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = benchmark_utils::ResourcesBenchHelper;
}

/// Runtime-local ring membership proof, used by coinage to authenticate the free unload token
/// flows against the people and lite people rings.
// TODO(<https://github.com/paritytech/individuality/issues/1125>): move this into pallet-people.
#[derive(
	Clone, PartialEq, Eq, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen,
)]
pub struct MembershipProof {
	pub proof: <BandersnatchVrfVerifiable as GenerateVerifiable>::Proof,
	pub ring: RingIndex,
	pub revision: RevisionIndex,
}

impl indiv_pallet_coinage::ValidateProof for MembershipProof {
	type Proof = MembershipProof;

	fn validate_proof(
		identifier: &Identifier,
		proof: &Self::Proof,
		context: &[u8; 32],
		msg: &[u8],
	) -> Result<Alias, ()> {
		use indiv_support::traits::MembershipProver;
		let result = Members::verify_membership(
			identifier,
			&proof.proof,
			proof.ring,
			proof.revision,
			*context,
			msg,
		)
		.inspect_err(|e| log::debug!("validate proof fail: verify membership: {e:?}"))
		.map_err(|_| ())?;
		Ok(result.alias)
	}
}

parameter_types! {
	/// Coinage's pallet id, used to derive the account holding the assets backing all coins.
	pub const CoinagePalletId: PalletId = PalletId(*b"coinage ");
	pub const CoinageInstanceCreationHoldReason: RuntimeHoldReason =
		RuntimeHoldReason::Coinage(indiv_pallet_coinage::HoldReason::InstanceCreationDeposit);
	pub const CoinageInstanceCreationDepositAmount: Balance = 0;
	pub CoinageLoadDeposit: (Location, Balance) =
		(StableAssetLocation::get(), HOLLAR_UNITS / 100);
}

pub type CoinageInstanceCreationDeposit = HoldConsideration<
	AccountId,
	Balances,
	CoinageInstanceCreationHoldReason,
	ConstantStoragePrice<CoinageInstanceCreationDepositAmount, Balance>,
>;

/// Coinage only supports the configured HOLLAR instance. Other assets cannot be converted and
/// therefore cannot be used to pay an unload fee.
pub struct CoinageFeeConversion;

impl pallet_asset_conversion::Swap<AccountId> for CoinageFeeConversion {
	type Balance = Balance;
	type AssetKind = Location;

	fn max_path_len() -> u32 {
		0
	}

	fn swap_exact_tokens_for_tokens(
		_sender: AccountId,
		_path: Vec<Self::AssetKind>,
		_amount_in: Self::Balance,
		_amount_out_min: Option<Self::Balance>,
		_send_to: AccountId,
		_keep_alive: bool,
	) -> Result<Self::Balance, DispatchError> {
		Err(DispatchError::Other("coinage only supports HOLLAR"))
	}

	fn swap_tokens_for_exact_tokens(
		_sender: AccountId,
		_path: Vec<Self::AssetKind>,
		_amount_out: Self::Balance,
		_amount_in_max: Option<Self::Balance>,
		_send_to: AccountId,
		_keep_alive: bool,
	) -> Result<Self::Balance, DispatchError> {
		Err(DispatchError::Other("coinage only supports HOLLAR"))
	}
}

impl pallet_asset_conversion::QuotePrice for CoinageFeeConversion {
	type Balance = Balance;
	type AssetKind = Location;

	fn quote_price_tokens_for_exact_tokens(
		_asset1: Self::AssetKind,
		_asset2: Self::AssetKind,
		_amount: Self::Balance,
		_include_fee: bool,
	) -> Option<Self::Balance> {
		None
	}

	fn quote_price_exact_tokens_for_tokens(
		_asset1: Self::AssetKind,
		_asset2: Self::AssetKind,
		_amount: Self::Balance,
		_include_fee: bool,
	) -> Option<Self::Balance> {
		None
	}
}

/// Converts lifecycle weight to HOLLAR, the only coinage asset this runtime enables.
pub struct CoinageWeightToFee;

impl Convert<Weight, Balance> for CoinageWeightToFee {
	fn convert(weight: Weight) -> Balance {
		AssetRate::to_asset_balance(
			<crate::DotWeightToFee<Runtime> as WeightToFee>::weight_to_fee(&weight),
			StableAssetLocation::get(),
		)
		.unwrap_or(Balance::MAX)
	}
}

impl indiv_pallet_coinage::Config for Runtime {
	type WeightInfo = indiv_pallet_coinage::weights::SubstrateWeight<Runtime>;
	type PalletId = CoinagePalletId;
	type UnixTime = RuntimeClock;
	type MemberService = Members;
	type RecyclerRingExponent = RecyclerRingExponent;
	type PaidUnloadTokenRingExponent = PaidUnloadTokenRingExponent;

	type NativeFungible = Balances;
	type Fungibles = AssetsWithHolder;
	type AdminOrigin = RootOrFellows;
	type SponsorOrigin = frame_system::EnsureSigned<AccountId>;
	type EnablePermissionless = ConstBool<false>;
	type LoadDeposit = CoinageLoadDeposit;
	type InstanceCreationDeposit = CoinageInstanceCreationDeposit;

	// Coin values are `2^exponent * asset_unit`, so with a unit of $0.01 the denominations
	// run from $0.01 (exponent 0) up to $163.84 (exponent 14).
	//
	// TODO: double-check the coinage economics for Polkadot. The denomination range, the free
	// unload allowances below and the choice of [`StableAssetLocation`] as backing asset are all
	// ported from the reference runtime, so nobody has yet decided that Polkadot wants bearer
	// coins in this exact range backed by a Hydration-issued stablecoin.
	type MinimumExponent = ConstI8<0>;
	type MaximumExponent = ConstI8<14>;
	type MinimumExponentForOutputUnloadFee = ConstI8<0>;
	type MaximumAge = ConstU16<16>;
	type MaxSplitOutputs = ConstU32<32>;
	type MaxConsolidation = ConstU32<64>;
	type MaxBatchUnpaidLoad = ConstU32<10>;

	/// ~3 months before a full recycler ring can be cleaned up.
	type RecyclerExpirationTime = ConstU32<{ 90 * 24 * 60 * 60 }>;
	type PaidUnloadTokenTimePeriod = ConstU32<{ 3 * 24 * 60 * 60 }>;
	type PaidUnloadTokenRingExpirationTime = ConstU32<{ 4 * 24 * 60 * 60 }>;

	type MembershipProof = MembershipProof;
	type UnloadTokenTimePeriodPeopleLitePeople = ConstU32<{ 24 * 60 * 60 }>; // 1 day
																		  // Free unload token allowance of $20 per time period for people and $10 for lite people. The
																		  // fee is dynamic (it follows the fee multiplier), and usage is additionally capped by
																		  // `MaxFreeUnloadTokensPerTimePeriod`.
	type UnloadTokenAllowancePerTimePeriodForPeople = ConstU128<{ 2000 * (HOLLAR_UNITS / 100) }>;
	type UnloadTokenAllowancePerTimePeriodForLitePeople =
		ConstU128<{ 1000 * (HOLLAR_UNITS / 100) }>;
	type MaxFreeUnloadTokensPerTimePeriod = ConstU32<1000>;

	type FeeConversion = CoinageFeeConversion;
	type NativeAssetKind = StableAssetLocation;
	type FeeDestination = TypedGetToGet<pallet_collator_selection::StakingPotAccountId<Runtime>>;
	type WeightToFee = CoinageWeightToFee;
	type OffchainWorkerInterval = ConstU32<4>;
	/// Base lock applied to a coin after a failed `AsCoin` dispatch; grows as `2^retries * base`.
	type CoinFailureLockPeriod = ConstU64<60>;

	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = benchmark_utils::CoinageBenchHelper;
}

/// Origin check that validates the caller is a sibling parachain and extracts its `ParaId`.
pub struct EnsureSiblingParachain;
impl frame_support::traits::EnsureOrigin<RuntimeOrigin> for EnsureSiblingParachain {
	type Success = ParaId;

	fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
		match o.clone().into() {
			Ok(cumulus_pallet_xcm::Origin::SiblingParachain(id)) => Ok(id),
			_ => Err(o),
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
		Ok(cumulus_pallet_xcm::Origin::SiblingParachain(ASSET_HUB_ID.into()).into())
	}
}

parameter_types! {
	/// Weight charged locally for the `request_replay` call a subscriber dispatches on us.
	pub ConstantWeight: Weight = Weight::from_parts(10_000, 0);
}

impl indiv_pallet_members_notifier::Config for Runtime {
	type WeightInfo = indiv_pallet_members_notifier::weights::SubstrateWeight<Runtime>;
	type XcmRouter = xcm_config::XcmRouter;
	type ChannelInfo = ParachainSystem;
	type ManageOrigin = RootOrFellows;
	type EnsureSubscriberOrigin = EnsureSiblingParachain;
	type Crypto = BandersnatchVrfVerifiable;
	type RingRootsProvider = Members;
	type Clock = RuntimeClock;
	type MaxSubscribers = ConstU32<10>;
	type MaxUpdatesPerBatch = ConstU32<10>;
	type MaxCollectionsPerSubscriber = ConstU32<10>;
	type MaxCollections = ConstU32<100>;
	type UpdateTriggerBlocks = ConstU32<1>;
	type UpdateTriggerThreshold = ConstU32<1>;
	type RequestReplayRemoteWeight = ConstantWeight;
	type OffchainWorkerInterval = ConstU32<1>;
	type StuckBatchTimeout = ConstU32<100>;
	type ReplayCooldownSeconds = ConstU64<60>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = benchmark_utils::MembersNotifierBenchHelper;
}

// The allowances below bound how much block weight an *anonymous* origin may consume. Such origins
// are not backed by an account, so nothing else limits them: without this a single alias could
// spam the chain for free. The allowance regenerates linearly per block.
//
// TODO(<https://github.com/paritytech/individuality/issues/1124>): choose good values.
const PEOPLE_IDENTITY_AND_ALIAS_ALLOWANCE_MAX: Balance = UNITS;
const PEOPLE_IDENTITY_AND_ALIAS_ALLOWANCE_RECOVERY: Balance = CENTS;
const ACCOUNT_PARTICIPANT_RECOVERY: Balance = CENTS;
const LITE_PERSON_ALLOWANCE_MAX: Balance = UNITS;
const LITE_PERSON_ALLOWANCE_RECOVERY: Balance = MILLICENTS;

/// The anonymous origins this runtime rate-limits, and the key their allowance is tracked under.
#[derive(
	Clone, Encode, Decode, Debug, MaxEncodedLen, TypeInfo, Eq, PartialEq, DecodeWithMemTracking,
)]
pub enum RestrictedEntity {
	PersonalAlias(Alias),
	PersonalIdentity(u64),
	AccountParticipant(AccountId),
	LitePerson(AccountId),
	LiteAlias(Alias),
}

impl indiv_pallet_origin_restriction::RestrictedEntity<OriginCaller, Balance> for RestrictedEntity {
	fn allowance(&self) -> Allowance<Balance> {
		match self {
			RestrictedEntity::PersonalAlias(_) | RestrictedEntity::PersonalIdentity(_) =>
				Allowance {
					max: PEOPLE_IDENTITY_AND_ALIAS_ALLOWANCE_MAX,
					recovery_per_block: PEOPLE_IDENTITY_AND_ALIAS_ALLOWANCE_RECOVERY,
				},
			RestrictedEntity::AccountParticipant(_) =>
				Allowance { max: 0, recovery_per_block: ACCOUNT_PARTICIPANT_RECOVERY },
			RestrictedEntity::LitePerson(_) | RestrictedEntity::LiteAlias(_) => Allowance {
				max: LITE_PERSON_ALLOWANCE_MAX,
				recovery_per_block: LITE_PERSON_ALLOWANCE_RECOVERY,
			},
		}
	}

	fn restricted_entity(origin_caller: &OriginCaller) -> Option<Self> {
		use indiv_pallet_people::Origin::*;
		use indiv_pallet_people_lite::Origin::*;
		use indiv_pallet_score::Origin::*;
		match origin_caller {
			OriginCaller::People(PersonalIdentity(id)) =>
				Some(RestrictedEntity::PersonalIdentity(*id)),
			OriginCaller::People(PersonalAlias(rev_ca)) =>
				Some(RestrictedEntity::PersonalAlias(rev_ca.ca.alias)),
			OriginCaller::Score(AccountParticipant(account_id)) =>
				Some(RestrictedEntity::AccountParticipant(account_id.clone())),
			OriginCaller::PeopleLite(LitePerson(account_id)) =>
				Some(RestrictedEntity::LitePerson(account_id.clone())),
			OriginCaller::PeopleLite(LiteAlias(rev_ca)) =>
				Some(RestrictedEntity::LiteAlias(rev_ca.ca.alias)),
			_ => None,
		}
	}
}

/// Calls that an entity with a zero allowance may still dispatch once, going into debt.
///
/// A candidate has no allowance at all until it becomes a person, yet it must be able to run the
/// calls that get it there. Allowing a single overdraft per entity gives it exactly that, while
/// the negative balance then has to recover before the next attempt.
pub struct OperationAllowedOneTimeExcess;
impl ContainsPair<RestrictedEntity, RuntimeCall> for OperationAllowedOneTimeExcess {
	fn contains(entity: &RestrictedEntity, call: &RuntimeCall) -> bool {
		use indiv_pallet_game::Call::*;
		use indiv_pallet_score::Call::*;
		match entity {
			RestrictedEntity::AccountParticipant(_) => matches!(
				call,
				RuntimeCall::Score(cash_out { .. }) |
					RuntimeCall::Score(redeem_credit { .. }) |
					RuntimeCall::Score(register { .. }) |
					RuntimeCall::Game(sign_up_with_account { .. }) |
					RuntimeCall::Game(report { .. }) |
					RuntimeCall::Game(offboard { .. }) |
					RuntimeCall::Game(claim_airdrop { .. })
			),
			RestrictedEntity::PersonalAlias(_) |
			RestrictedEntity::PersonalIdentity(_) |
			RestrictedEntity::LitePerson(_) |
			RestrictedEntity::LiteAlias(_) => false,
		}
	}
}

impl indiv_pallet_origin_restriction::Config for Runtime {
	type WeightInfo = weights::indiv_pallet_origin_restriction::WeightInfo<Runtime>;
	type RestrictedEntity = RestrictedEntity;
	type OperationAllowedOneTimeExcess = OperationAllowedOneTimeExcess;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = benchmark_utils::OriginRestrictionBenchmarkHelper;
}

/// Call encoding for the Bulletin Chain `TransactionStorage` calls invoked over XCM.
///
/// The pallet index is a dynamic parameter because Bulletin does not deploy
/// `pallet-transaction-storage` yet. Governance must set it to the actual index before any
/// long-term-storage allocation can succeed.
///
/// TODO: long-term storage allocation is a silent no-op until Bulletin Polkadot deploys
/// `pallet-transaction-storage`, and this index must be re-verified against its
/// `construct_runtime!` when it does. Two things to resolve before relying on this path:
///
/// * Bulletin Polkadot does not deploy `pallet-transaction-storage` yet, so the messages sent below
///   are accepted locally (XCM delivery is fire-and-forget, and `send_xcm` only reports a *send*
///   failure) and then discarded by the receiver. `pallet-resources` nevertheless records the
///   allocation as granted on this chain, so a person can consume their long-term storage allowance
///   against storage that was never actually reserved.
/// * Nothing replays the grants issued during that window once the pallet does land, so the two
///   sides start out inconsistent and need a reconciliation plan.
#[derive(Encode, Decode)]
enum TransactionStorageCalls<AccountId: Encode> {
	/// `authorize_account(who, transactions, bytes)`
	#[codec(index = 3)]
	AuthorizeAccount(AccountId, u32, u64),
	/// `refresh_account_authorization(who)`
	#[codec(index = 7)]
	RefreshAccountAuthorization(AccountId),
}

/// Grants an account a data allowance on the Bulletin Chain, which is where long-term person data
/// lives.
pub struct BulletinDataStore;
impl AllocateStorage<AccountId> for BulletinDataStore {
	fn allocate_storage(who: &AccountId, len: u64, count: u32) -> DispatchResult {
		let call = Self::encode_transaction_storage_call(
			TransactionStorageCalls::AuthorizeAccount(who.clone(), count, len),
		);
		Self::send(call)
	}

	fn refresh_allocation(who: &AccountId) -> DispatchResult {
		let call = Self::encode_transaction_storage_call(
			TransactionStorageCalls::RefreshAccountAuthorization(who.clone()),
		);
		Self::send(call)
	}
}

impl BulletinDataStore {
	/// The long-term storage protocol is a sibling-parachain protocol. Governance may correct the
	/// Bulletin para-id, but may not redirect allocations to the relay chain or an arbitrary XCM
	/// interior, where a successful local send would otherwise remain a silent remote no-op.
	pub(crate) fn bulletin_chain_location() -> Result<Location, DispatchError> {
		let destination = dynamic_params::bulletin_storage::BulletinChainLocation::get();
		if destination.parents == 1 &&
			matches!(destination.interior.as_slice(), [Junction::Parachain(_)])
		{
			Ok(destination)
		} else {
			Err(DispatchError::Other("Bulletin destination must be a sibling parachain"))
		}
	}

	fn encode_transaction_storage_call(
		call: TransactionStorageCalls<AccountId>,
	) -> alloc::vec::Vec<u8> {
		let mut encoded = alloc::vec![
			dynamic_params::bulletin_storage::BulletinTransactionStoragePalletIndex::get(),
		];
		encoded.extend(call.encode());
		encoded
	}

	fn send(call: alloc::vec::Vec<u8>) -> DispatchResult {
		let program = Xcm(alloc::vec![
			UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
			Transact { origin_kind: OriginKind::Xcm, fallback_max_weight: None, call: call.into() },
		]);

		send_xcm::<xcm_config::XcmRouter>(Self::bulletin_chain_location()?, program)
			.map(|_| ())
			.map_err(|_| pallet_xcm::Error::<Runtime>::SendFailure)?;
		Ok(())
	}
}

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmark_utils {
	use super::*;
	use alloc::{vec, vec::Vec};
	use frame_support::{
		dispatch::RawOrigin,
		traits::{
			fungibles::{Create, Inspect, Mutate},
			UnixTime,
		},
	};
	use indiv_support::{
		crypto::BandersnatchSuite,
		genesis::ring_verifier_builder_params,
		traits::{AddOnlyPeopleTrait, AppendOnlyMembers, RingMode, PEOPLE_IDENTIFIER},
	};
	use sp_runtime::traits::IdentifyAccount;
	use verifiable::ring::RingDomainSize;

	type BenchRingSetup = (
		<BandersnatchVrfVerifiable as GenerateVerifiable>::Members,
		<BandersnatchVrfVerifiable as GenerateVerifiable>::Intermediate,
		<BandersnatchVrfVerifiable as GenerateVerifiable>::Member,
		<BandersnatchVrfVerifiable as GenerateVerifiable>::Secret,
		<BandersnatchVrfVerifiable as GenerateVerifiable>::Config,
	);

	/// Reads `pallet_timestamp::Now` directly, deliberately skipping `pallet_timestamp::Pallet`'s
	/// `UnixTime` impl so that the `log::error!` it emits for a zero timestamp does not fire on
	/// every call during benchmarking. Behaviour is otherwise identical.
	pub struct BenchmarkClock;
	impl UnixTime for BenchmarkClock {
		fn now() -> core::time::Duration {
			core::time::Duration::from_millis(pallet_timestamp::Now::<Runtime>::get())
		}
	}

	/// Stands in for [`BulletinDataStore`] so that benchmarks do not drive the XCMP queue for a
	/// destination the benchmark state has no channel to.
	pub struct BenchmarkDataStore;
	impl AllocateStorage<AccountId> for BenchmarkDataStore {
		fn allocate_storage(_who: &AccountId, _len: u64, _count: u32) -> DispatchResult {
			Ok(())
		}

		fn refresh_allocation(_who: &AccountId) -> DispatchResult {
			Ok(())
		}
	}

	pub fn member_from_seed(
		seed: u64,
	) -> <BandersnatchVrfVerifiable as GenerateVerifiable>::Member {
		let mut entropy = [0u8; 32];
		entropy[..8].copy_from_slice(&seed.to_le_bytes()[..]);
		let secret = BandersnatchVrfVerifiable::new_secret(entropy);
		BandersnatchVrfVerifiable::member_from_secret(&secret)
	}

	/// Builds a one-member Bandersnatch ring from a configured exponent.
	///
	/// This mirrors `ring_setup` in Individuality's
	/// `runtimes/next-asset-hub-paseo/src/lib.rs`. `indiv_support::crypto` does not export the
	/// benchmark helper at this pinned revision; keep this single local mirror in sync with review
	/// r3734171704 until the SDK exports it.
	fn ring_setup(ring_exponent: RingExponent, entropy: [u8; 32]) -> BenchRingSetup {
		let domain: RingDomainSize =
			ring_exponent.try_into().expect("RingExponent maps to RingDomainSize");
		let chunks = ring_verifier_builder_params::<BandersnatchSuite>(domain);
		let secret = BandersnatchVrfVerifiable::new_secret(entropy);
		let member = BandersnatchVrfVerifiable::member_from_secret(&secret);
		let mut intermediate = BandersnatchVrfVerifiable::start_members(domain);
		BandersnatchVrfVerifiable::push_members(
			&mut intermediate,
			core::iter::once(member),
			|range| Ok(chunks[range].to_vec()),
		)
		.expect("benchmark: push_members for a single member");
		let members = BandersnatchVrfVerifiable::finish_members(intermediate.clone());
		(members, intermediate, member, secret, domain)
	}

	pub fn account_from_seed(seed: u64) -> AccountId {
		use sp_core::Pair;
		let mut entropy = [0u8; 32];
		entropy[..8].copy_from_slice(&seed.to_le_bytes()[..]);
		sp_core::ed25519::Pair::from_seed(&entropy).public().into_account().into()
	}

	pub fn sign_with_seed(seed: u64, msg: &[u8]) -> sp_core::ed25519::Signature {
		let mut entropy = [0u8; 32];
		entropy[..8].copy_from_slice(&seed.to_le_bytes()[..]);
		// `sp-core` does not expose signing inside the runtime, so use the underlying library.
		let secret = ed25519_zebra::SigningKey::from(entropy);
		sp_core::ed25519::Signature::from_raw(secret.sign(msg).into())
	}

	/// Idempotently creates the stable asset so the value-carrying flows have something to move.
	pub fn ensure_stable_asset_exists() {
		let asset = StableAssetLocation::get();
		if !<Assets as Inspect<AccountId>>::asset_exists(asset.clone()) {
			<Assets as Create<AccountId>>::create(
				asset,
				CoinagePalletId::get().into_account_truncating(),
				true,
				1u128,
			)
			.expect("benchmark: stable asset must be creatable");
		}
	}

	pub struct ChunksManagerBenchHelper;
	impl
		indiv_pallet_chunks_manager::BenchmarkHelper<
			<BandersnatchVrfVerifiable as GenerateVerifiable>::StaticChunk,
		> for ChunksManagerBenchHelper
	{
		fn chunk_page() -> Vec<<BandersnatchVrfVerifiable as GenerateVerifiable>::StaticChunk> {
			// This fixed maximum domain is intentional: it selects the chunk-page-size fixture.
			ring_verifier_builder_params(RingDomainSize::Domain16)
				.into_iter()
				.take(ChunkPageSize::get() as usize)
				.collect()
		}
	}

	pub struct MembersBenchHelper;
	impl
		indiv_pallet_members::BenchmarkHelper<
			<BandersnatchVrfVerifiable as GenerateVerifiable>::StaticChunk,
		> for MembersBenchHelper
	{
		fn initialize_chunks(
			ring_size: RingExponent,
		) -> Vec<<BandersnatchVrfVerifiable as GenerateVerifiable>::StaticChunk> {
			let domain_size: RingDomainSize =
				ring_size.try_into().expect("ring exponent maps to a ring domain size; qed");
			ring_verifier_builder_params(domain_size)
		}

		fn set_time(now: core::time::Duration) {
			pallet_timestamp::Now::<Runtime>::put(now.as_millis() as u64);
		}

		fn set_valid_time() {
			pallet_timestamp::Now::<Runtime>::put(
				core::time::Duration::from_secs(5).as_millis() as u64
			);
		}
	}

	pub struct PeopleBenchHelper;
	impl
		indiv_pallet_people::BenchmarkHelper<
			<BandersnatchVrfVerifiable as GenerateVerifiable>::StaticChunk,
		> for PeopleBenchHelper
	{
		/// The benchmarks use this as their `worst_case_account_context` too, so return the last
		/// context [`AccountContexts`] checks: it is the one that makes `contains` evaluate every
		/// comparison before matching.
		fn valid_account_context() -> Context {
			indiv_pallet_resources::RESOURCES_CONTEXT
		}

		fn initialize_chunks() -> Vec<<BandersnatchVrfVerifiable as GenerateVerifiable>::StaticChunk>
		{
			let domain: RingDomainSize = MembersFlexibleRingExponent::get()
				.try_into()
				.expect("people ring exponent maps to a ring domain size");
			ring_verifier_builder_params(domain)
		}
	}

	pub struct ScoreBenchmarkHelper;
	impl indiv_pallet_score::benchmarking::BenchmarkHelper<Runtime> for ScoreBenchmarkHelper {
		fn create_member(seed: u64) -> indiv_pallet_score::MemberOf<Runtime> {
			member_from_seed(seed)
		}

		fn setup_currency() {
			ensure_stable_asset_exists();
		}
	}

	pub struct GamePalletBenchmarkHelper;
	impl
		indiv_pallet_game::BenchmarkHelper<
			Signature,
			MultiSignature,
			AccountId,
			AccountId,
			<Runtime as pallet_assets::Config>::AssetId,
		> for GamePalletBenchmarkHelper
	{
		fn create_account(seed: u64) -> AccountId {
			account_from_seed(seed)
		}

		fn sign_account(seed: u64, msg: &[u8]) -> Signature {
			sign_with_seed(seed, msg).into()
		}

		fn create_ticket(seed: u64) -> AccountId {
			account_from_seed(seed)
		}

		fn sign_ticket(seed: u64, msg: &[u8]) -> MultiSignature {
			sign_with_seed(seed, msg).into()
		}

		fn set_valid_time() {
			Timestamp::set_timestamp(1u32.into());
		}

		fn set_time(now: core::time::Duration) {
			// Not via `set_timestamp`, which triggers checks such as the Aura slot.
			pallet_timestamp::Now::<Runtime>::put(now.as_millis() as u64);
		}

		fn fund_account(acc: AccountId) {
			use frame_support::traits::fungible::Mutate as _;
			let _ = Balances::mint_into(&acc, 1_000_000_000_000_000u128);
		}

		fn airdrop_asset_id() -> <Runtime as pallet_assets::Config>::AssetId {
			StableAssetLocation::get()
		}
	}

	pub struct HonourBenchmarkHelper;
	impl indiv_pallet_honour::benchmarking::BenchmarkHelper<Runtime> for HonourBenchmarkHelper {
		fn set_time(now: indiv_pallet_honour::Seconds) {
			pallet_timestamp::Now::<Runtime>::put(now.saturating_mul(1_000));
		}

		fn seed_and_create_proof(
			vote: &indiv_pallet_honour::VoteData,
			message: &[u8],
		) -> indiv_pallet_honour::RingProofOf<Runtime> {
			let ring_exponent = <Runtime as indiv_pallet_people::Config>::RingExponent::get();
			let ring_index: RingIndex = 0;

			// Build a one-member people ring in the member service. Mirrors the targeted setup used
			// by `indiv_pallet_people`'s own proof benchmarks rather than the full
			// `process_maintenance` sweep, which is heavier and runs once per benchmark repeat.
			Members::create_collection(
				PeopleCollectionOwner::get(),
				indiv_pallet_people::PEOPLE_MEMBER_IDENTIFIER,
				1,
				RingMode::Flexible,
				ring_exponent,
				None,
			)
			.expect("benchmark: people collection must be created");

			let secret =
				BandersnatchVrfVerifiable::new_secret(sp_core::twox_256(b"honour-bench-voter"));
			let member = BandersnatchVrfVerifiable::member_from_secret(&secret);

			Members::add_members(indiv_pallet_people::PEOPLE_MEMBER_IDENTIFIER, vec![member])
				.expect("benchmark: ring member must be added");
			Members::initialize_chunks(ring_exponent);
			Members::onboard_all_and_build_ring(
				indiv_pallet_people::PEOPLE_MEMBER_IDENTIFIER,
				ring_index,
			)
			.expect("benchmark: people ring must be built");

			let ring_members =
				Members::ring_members(indiv_pallet_people::PEOPLE_MEMBER_IDENTIFIER, ring_index);
			let domain: RingDomainSize =
				ring_exponent.try_into().expect("people ring exponent maps to a domain size");
			let commitment =
				BandersnatchVrfVerifiable::open(domain, &member, ring_members.into_iter())
					.expect("benchmark: commitment must open");

			let contexts = vote.get_contexts();
			let contexts: Vec<&[u8]> = contexts.iter().map(|c| &c[..]).collect();
			let (proof, _) = BandersnatchVrfVerifiable::create_multi_context(
				commitment, &secret, &contexts, message,
			)
			.expect("benchmark: proof creation must succeed");
			proof
		}
	}

	pub struct AirdropBenchmarkHelper;
	impl indiv_pallet_airdrop::benchmarking::BenchmarkHelper<Runtime> for AirdropBenchmarkHelper {
		fn set_unix_time(now: core::time::Duration) {
			pallet_timestamp::Now::<Runtime>::put(now.as_millis() as u64);
		}

		fn create_asset_id_parameter(id: u32) -> <Runtime as pallet_assets::Config>::AssetId {
			let location = Location::new(1, [Parachain(ASSET_HUB_ID), GeneralIndex(id as u128)]);
			if !<Assets as Inspect<AccountId>>::asset_exists(location.clone()) {
				let owner: AccountId =
					parachain_info::Pallet::<Runtime>::parachain_id().into_account_truncating();
				<Assets as Create<AccountId>>::create(location.clone(), owner, true, 1u128)
					.expect("benchmark: airdrop asset must be creatable");
			}
			location
		}

		fn build_membership_proof(
			context: &Context,
			message: &[u8],
			member_seed: u32,
		) -> (indiv_pallet_airdrop::ProofOf<Runtime>, Alias) {
			let ring_exponent = MembersFlexibleRingExponent::get();
			let mut entropy = [0u8; 32];
			entropy[..4].copy_from_slice(&member_seed.to_le_bytes());
			let (members, intermediate, member, secret, domain) =
				ring_setup(ring_exponent, entropy);

			// Build a single-member ring with `member`. The resulting `members` value is the
			// on-chain ring root we seed below so that verification at
			// `(PEOPLE_IDENTIFIER, ring = 0, rev = 0)` succeeds.

			if indiv_pallet_members::Collections::<Runtime>::get(PEOPLE_IDENTIFIER).is_none() {
				indiv_pallet_members::Collections::<Runtime>::insert(
					PEOPLE_IDENTIFIER,
					indiv_pallet_members::types::CollectionInfo {
						owner: indiv_pallet_members::types::CollectionOwner::External(
							PeopleCollectionOwner::get(),
						),
						mode: RingMode::Flexible,
						ring_size: ring_exponent,
						self_inclusion_delay: Some(SelfInclusionDelayValue::get()),
					},
				);
			}
			indiv_pallet_members::Root::<Runtime>::insert(
				PEOPLE_IDENTIFIER,
				0u32,
				indiv_pallet_members::types::RingRoot { root: members, revision: 0, intermediate },
			);

			let commitment =
				BandersnatchVrfVerifiable::open(domain, &member, core::iter::once(member))
					.expect("benchmark: open commitment");
			let (proof, _aliases) = BandersnatchVrfVerifiable::create_multi_context(
				commitment,
				&secret,
				&[&context[..]],
				message,
			)
			.expect("benchmark: create membership proof");
			let alias = BandersnatchVrfVerifiable::alias_in_context(&secret, &context[..])
				.expect("benchmark: alias_in_context");
			(proof, alias)
		}

		fn account_keypair_for(seed: u32) -> (AccountId, sp_core::sr25519::Pair) {
			use sp_core::Pair as _;
			let mut entropy = [0u8; 32];
			entropy[..4].copy_from_slice(&seed.to_le_bytes());
			let pair = sp_core::sr25519::Pair::from_seed(&entropy);
			let account_id: AccountId = MultiSigner::Sr25519(pair.public()).into_account();
			(account_id, pair)
		}
	}

	pub struct ResourcesBenchHelper;
	impl indiv_pallet_resources::benchmarking::BenchmarkHelper<Runtime> for ResourcesBenchHelper {
		fn set_time(now: core::time::Duration) {
			pallet_timestamp::Now::<Runtime>::put(now.as_millis() as u64);
		}

		fn sign_message(message: &[u8]) -> (AccountId, MultiSignature) {
			use sp_core::Pair;
			let entropy = [1u8; 32];
			let pair = sp_core::ed25519::Pair::from_seed(&entropy);
			let account: AccountId = pair.public().into_account().into();
			let secret = ed25519_zebra::SigningKey::from(entropy);
			let signature = sp_core::ed25519::Signature::from_raw(secret.sign(message).into());
			(account, signature.into())
		}
	}

	pub struct MembersNotifierBenchHelper;
	impl indiv_pallet_members_notifier::benchmarking::BenchmarkHelper<Runtime>
		for MembersNotifierBenchHelper
	{
		fn init() {
			use cumulus_pallet_parachain_system::RelevantMessagingState;
			use cumulus_primitives_core::relay_chain::AbridgedHrmpChannel;

			// The timestamp must exceed `ReplayCooldownSeconds` (60s) so that the `request_replay`
			// benchmark passes the cooldown check.
			pallet_timestamp::Now::<Runtime>::put(120_000u64);

			// Fake HRMP egress channels so benchmarks that send XCM succeed. They use para ids
			// `0..MaxSubscribers` and `1000..1000 + MaxSubscribers`.
			let max_subscribers =
				<Runtime as indiv_pallet_members_notifier::Config>::MaxSubscribers::get();
			let channel = AbridgedHrmpChannel {
				max_capacity: 1000,
				max_total_size: 1_000_000,
				max_message_size: 100_000,
				msg_count: 0,
				total_size: 0,
				mqc_head: None,
			};
			let mut egress_channels: Vec<(ParaId, AbridgedHrmpChannel)> = (0..max_subscribers)
				.chain(1000..1000 + max_subscribers)
				.map(|i| (ParaId::from(i), channel.clone()))
				.collect();
			egress_channels.sort_by_key(|(id, _)| *id);
			egress_channels.dedup_by_key(|(id, _)| *id);

			let messaging_state =
				cumulus_pallet_parachain_system::relay_state_snapshot::MessagingStateSnapshot {
					dmq_mqc_head: Default::default(),
					relay_dispatch_queue_remaining_capacity: Default::default(),
					ingress_channels: Vec::new(),
					egress_channels,
				};
			RelevantMessagingState::<Runtime>::put(messaging_state);
		}

		fn setup_ring_roots(count: u32) {
			// This fixed smallest domain is intentional: this transport fixture never proves
			// membership.
			let intermediate = BandersnatchVrfVerifiable::start_members(RingDomainSize::Domain11);
			let root = BandersnatchVrfVerifiable::finish_members(intermediate.clone());

			// Matches the `test_identifier` helper in the notifier benchmarking module.
			fn test_identifier(index: u32) -> Identifier {
				let mut id = [0u8; 32];
				id[..4].copy_from_slice(&index.to_be_bytes());
				id
			}
			assert_eq!(
				test_identifier(0xDEADBEEF),
				hex_literal::hex!(
					"deadbeef00000000000000000000000000000000000000000000000000000000"
				),
				"test_identifier drifted — sync with pallets/members-notifier/src/benchmarking.rs",
			);

			// Populate ring roots for every identifier the benchmarks may reference: they spread
			// pending updates across `MaxCollections` identifiers.
			let max_collections =
				<Runtime as indiv_pallet_members_notifier::Config>::MaxCollections::get();
			for coll in 0..max_collections {
				let identifier = test_identifier(coll);
				for i in 0..count {
					let ring_root = indiv_pallet_members::RingRoot::<Runtime> {
						root: root.clone(),
						revision: 0,
						intermediate: intermediate.clone(),
					};
					indiv_pallet_members::Root::<Runtime>::insert(identifier, i, ring_root);
				}
				indiv_pallet_members::CurrentRingIndex::<Runtime>::insert(identifier, count - 1);
			}
		}

		fn set_max_message_size(size: u32) {
			use cumulus_pallet_parachain_system::RelevantMessagingState;

			// Shrinking each egress channel's `max_message_size` triggers the worst-case chunking
			// path in `send_batch`. `init` must have run before this.
			let mut state = RelevantMessagingState::<Runtime>::get()
				.expect("BenchmarkHelper::init must run before set_max_message_size");
			for (_, channel) in state.egress_channels.iter_mut() {
				channel.max_message_size = size;
			}
			RelevantMessagingState::<Runtime>::put(state);
		}
	}

	pub struct CoinageBenchHelper;
	impl indiv_pallet_coinage::BenchmarkHelper<Runtime> for CoinageBenchHelper {
		fn setup_assets() {
			use frame_support::traits::fungibles::{Inspect, Mutate};

			ensure_stable_asset_exists();
			if indiv_pallet_coinage::AssetToInstance::<Runtime>::iter_key_prefix(
				StableAssetLocation::get(),
			)
			.next()
			.is_none()
			{
				let asset = StableAssetLocation::get();
				<AssetsWithHolder as Mutate<_>>::mint_into(
					asset.clone(),
					&Coinage::pallet_account(),
					<AssetsWithHolder as Inspect<_>>::minimum_balance(asset.clone()),
				)
				.expect("benchmark: coinage pallet account must be fundable");
				Coinage::create_sufficient_instance(
					RuntimeOrigin::root(),
					asset,
					HOLLAR_UNITS / 100,
				)
				.expect("benchmark: sufficient coinage instance must be creatable");
			}
		}

		fn setup_asset_without_instance() -> Location {
			use frame_support::traits::fungibles::{Inspect, Mutate};

			ensure_stable_asset_exists();
			let asset = StableAssetLocation::get();
			<AssetsWithHolder as Mutate<_>>::mint_into(
				asset.clone(),
				&Coinage::pallet_account(),
				<AssetsWithHolder as Inspect<_>>::minimum_balance(asset.clone()),
			)
			.expect("benchmark: coinage pallet account must be fundable");
			asset
		}

		fn fund_account(who: &AccountId, amount: Balance) {
			<AssetsWithHolder as Mutate<_>>::mint_into(StableAssetLocation::get(), who, amount)
				.expect("benchmark: account must be fundable");
		}

		fn create_extra_asset(seed: u32, who: &AccountId) -> Location {
			use frame_support::traits::fungibles::{Create, Mutate};

			let asset = Self::extra_asset_id(seed);
			if !Assets::asset_exists(asset.clone()) {
				<Assets as Create<_>>::create(
					asset.clone(),
					CoinagePalletId::get().into_account_truncating(),
					true,
					1u128,
				)
				.expect("benchmark: extra asset must be creatable");
			}
			<AssetsWithHolder as Mutate<_>>::mint_into(asset.clone(), who, 1_000_000 * UNITS)
				.expect("benchmark: extra asset must be fundable");
			asset
		}

		fn extra_asset_id(seed: u32) -> Location {
			Location::new(
				1,
				[Parachain(1000), PalletInstance(50), GeneralIndex(1_000_000u128 + seed as u128)],
			)
		}

		fn set_time(now: core::time::Duration) {
			pallet_timestamp::Now::<Runtime>::put(now.as_millis() as u64);
		}

		fn setup_fee_conversion() {
			// The benchmark runtime retains the production HOLLAR-only fee conversion policy.
		}

		fn create_people_proof(context: &[u8], msg: &[u8], _alias: Alias) -> MembershipProof {
			// Initialize the people collection and chunks if not already created.
			indiv_pallet_people::Pallet::<Runtime>::initialize_people_collection();
			let ring_exponent = <Runtime as indiv_pallet_people::Config>::RingExponent::get();
			indiv_pallet_members::Pallet::<Runtime>::initialize_chunks(ring_exponent);

			let secret =
				BandersnatchVrfVerifiable::new_secret(sp_core::twox_256(b"people_for_coinage:42"));
			let member = BandersnatchVrfVerifiable::member_from_secret(&secret);

			// Onboard members immediately.
			indiv_pallet_members::OnboardingSize::<Runtime>::insert(
				indiv_pallet_people::PEOPLE_MEMBER_IDENTIFIER,
				1,
			);
			indiv_pallet_people::Pallet::<Runtime>::force_recognize_personhood(
				RawOrigin::Root.into(),
				vec![member],
			)
			.expect("benchmark: personhood must be recognized");
			indiv_pallet_members::Pallet::<Runtime>::process_maintenance();

			let ring_index: RingIndex = 0;
			let domain: RingDomainSize = ring_exponent
				.try_into()
				.expect("people ring exponent maps to a ring domain size");
			let ring_keys = indiv_pallet_members::RingKeys::<Runtime>::get((
				indiv_pallet_people::PEOPLE_MEMBER_IDENTIFIER,
				ring_index,
				0u32,
			));
			let commitment =
				BandersnatchVrfVerifiable::open(domain, &member, ring_keys.into_iter())
					.expect("benchmark: commitment must open");
			let (proof, _alias) =
				BandersnatchVrfVerifiable::create(commitment, &secret, context, msg)
					.expect("benchmark: proof must be creatable");

			MembershipProof { proof, ring: ring_index, revision: 0 }
		}

		fn create_lite_people_proof(context: &[u8], msg: &[u8], _alias: Alias) -> MembershipProof {
			use sp_core::Pair;

			let ring_exponent = LitePeopleRingExponent::get();
			indiv_pallet_members::Pallet::<Runtime>::initialize_chunks(ring_exponent);

			let pair = sp_core::ed25519::Pair::from_seed(&[77u8; 32]);
			let account: AccountId = pair.public().into_account().into();

			let ring_secret = BandersnatchVrfVerifiable::new_secret([88u8; 32]);
			let ring_member = BandersnatchVrfVerifiable::member_from_secret(&ring_secret);

			indiv_pallet_people_lite::LitePeople::<Runtime>::insert(
				&account,
				indiv_pallet_people_lite::types::LitePersonInfo {
					ring_vrf_key: ring_member,
					method: indiv_pallet_people_lite::types::RecognitionMethod::UniqueDevice(
						account.clone(),
					),
				},
			);
			frame_system::Pallet::<Runtime>::inc_sufficients(&account);
			Members::create_collection(
				LitePeopleCollectionOwner::get(),
				indiv_pallet_people_lite::LITE_PEOPLE_MEMBER_IDENTIFIER,
				LitePeopleOnboardingSize::get(),
				RingMode::AppendOnly,
				ring_exponent,
				None,
			)
			.expect("benchmark: lite people collection must be created");
			Members::add_members(
				indiv_pallet_people_lite::LITE_PEOPLE_MEMBER_IDENTIFIER,
				vec![ring_member],
			)
			.expect("benchmark: lite people member must be added");
			indiv_pallet_members::OnboardingSize::<Runtime>::insert(
				indiv_pallet_people_lite::LITE_PEOPLE_MEMBER_IDENTIFIER,
				1,
			);
			indiv_pallet_members::Pallet::<Runtime>::process_maintenance();

			let ring_index: RingIndex = 0;
			let domain: RingDomainSize = ring_exponent
				.try_into()
				.expect("lite people ring exponent maps to a ring domain size");
			let ring_keys = indiv_pallet_members::RingKeys::<Runtime>::get((
				indiv_pallet_people_lite::LITE_PEOPLE_MEMBER_IDENTIFIER,
				ring_index,
				0u32,
			));
			let commitment =
				BandersnatchVrfVerifiable::open(domain, &ring_member, ring_keys.into_iter())
					.expect("benchmark: commitment must open");
			let (proof, _) =
				BandersnatchVrfVerifiable::create(commitment, &ring_secret, context, msg)
					.expect("benchmark: lite proof must be creatable");

			MembershipProof { proof, ring: ring_index, revision: 0 }
		}
	}

	pub struct OriginRestrictionBenchmarkHelper;
	impl indiv_pallet_origin_restriction::BenchmarkHelper<OriginCaller, RuntimeCall>
		for OriginRestrictionBenchmarkHelper
	{
		fn excess_pair() -> (OriginCaller, RuntimeCall) {
			(
				OriginCaller::Score(indiv_pallet_score::Origin::AccountParticipant(
					AccountId::new([0u8; 32]),
				)),
				RuntimeCall::Score(indiv_pallet_score::Call::cash_out {}),
			)
		}
	}
}
