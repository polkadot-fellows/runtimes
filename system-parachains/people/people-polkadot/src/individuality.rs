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
//! * [`indiv_pallet_honour`] lets people vote on calls with their personhood weight.
//! * [`indiv_pallet_resources`] rations the off-chain resources (statement store, notifications,
//!   long-term storage) a person may consume.
//! * [`indiv_pallet_coinage`] implements bearer-instrument "coins" backed by a stablecoin, held by
//!   ring aliases rather than accounts.
//! * [`indiv_pallet_dummy_dim`] is the governance-driven DIM: it lets the root origin recognise
//!   personhood directly.
//! * [`indiv_pallet_origin_restriction`] rate-limits the anonymous origins the extensions above
//!   produce, since those origins pay no fee from an account.
//! * [`indiv_pallet_relay_randomness`] surfaces relay chain randomness.

use super::*;

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use cumulus_primitives_core::ParaId;
use frame_support::{
	parameter_types,
	traits::{
		ConstBool, ConstU128, ConstantStoragePrice, ContainsPair, Get, PalletInfoAccess,
		fungible::HoldConsideration,
	},
};
use indiv_pallet_origin_restriction::Allowance;
#[cfg(feature = "runtime-benchmarks")]
use indiv_support::traits::{Identifier, RingIndex};
use indiv_support::{
	crypto::{BandersnatchVrfVerifiable, GenerateVerifiable},
	fungibles::CombineAssetsWithHolder,
	traits::{Alias, AllocateStorage, Context, PEOPLE_IDENTIFIER, PEOPLE_LITE_IDENTIFIER, RingExponent},
	utils::TypedGetToGet,
};
use polkadot_runtime_constants::system_parachain::ASSET_HUB_ID;
use scale_info::TypeInfo;
#[cfg(feature = "runtime-benchmarks")]
use sp_runtime::{MultiSignature, traits::AccountIdConversion};
use sp_runtime::{
	DispatchError, DispatchResult,
	traits::{ConstI8, ConstU16},
};
use polkadot_runtime_constants::time::MINUTES as RC_MINUTES;
// NOTE: deliberately not `xcm::latest::prelude::*` — its `Assets` would shadow the `Assets` pallet
// this module configures.
#[cfg(feature = "runtime-benchmarks")]
use xcm::latest::Junction::GeneralIndex;
#[cfg(feature = "runtime-benchmarks")]
use xcm::latest::Junction::Parachain;
use xcm::latest::{
	Instruction::{Transact, UnpaidExecution},
	Junction,
	Junction::PalletInstance,
	Location, OriginKind, WeightLimit, Xcm, send_xcm,
};

#[cfg(feature = "runtime-benchmarks")]
use crate::assets::hollar::{HOLLAR_UNITS, HollarLocation};
use crate::parameters::dynamic_params;

/// The full-featured fungibles implementation, combining `pallet-assets` balances with the hold
/// functionality supplied by `pallet-assets-holder`.
pub type AssetsWithHolder = CombineAssetsWithHolder<Assets, AssetsHolder>;

/// Wall-clock source used by the pallet `Config`s in this module.
#[cfg(not(feature = "runtime-benchmarks"))]
pub type RuntimeClock = Timestamp;
#[cfg(feature = "runtime-benchmarks")]
pub type RuntimeClock = benchmark_utils::BenchmarkClock;

parameter_types! {
	pub DefaultNetworkSuffix: indiv_support::context::ProductContextNetworkSuffix =
		b"polkadot".to_vec().try_into().expect("default network suffix fits");

	/// Largest ring exponent usable by `Flexible` collections in `pallet-members`.
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

	/// How long a person must wait before they may include themselves in the people ring collection,
	/// in seconds. This bypasses the normal onboarding queue mechanism, potentially reducing privacy.
	/// Without the delay, the act of joining would narrow the anonymity set down to the newest member
	/// in certain contexts.
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
}

impl indiv_pallet_network_suffix::Config for Runtime {
	type UpdateOrigin = EnsureRoot<Self::AccountId>;
	type DefaultSuffix = DefaultNetworkSuffix;
	type WeightInfo = weights::indiv_pallet_network_suffix::WeightInfo<Runtime>;
}

impl indiv_pallet_relay_randomness::Config for Runtime {
	type WeightInfo = weights::indiv_pallet_relay_randomness::WeightInfo<Runtime>;
}

impl indiv_pallet_chunks_manager::Config for Runtime {
	type WeightInfo = weights::indiv_pallet_chunks_manager::WeightInfo<Runtime>;
	type Chunk = <BandersnatchVrfVerifiable as GenerateVerifiable>::StaticChunk;
	type PageSize = ConstU32<255>;
	type ManagerOrigin = EnsureRoot<Self::AccountId>;
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
		context == &indiv_pallet_resources::Pallet::<Runtime>::resources_context()
	}
}

/// The alias contexts that lite people can authenticate against.
pub struct LiteAccountContexts;
impl frame_support::traits::Contains<Context> for LiteAccountContexts {
	fn contains(context: &Context) -> bool {
		context == &indiv_pallet_people_lite::Pallet::<Runtime>::auth_context()
	}
}

impl indiv_pallet_people::Config for Runtime {
	type WeightInfo = weights::indiv_pallet_people::WeightInfo<Runtime>;
	type MemberService = Members;
	type RingExponent = MembersFlexibleRingExponent;
	type CollectionOwner = PeopleCollectionOwner;
	type AccountContexts = AccountContexts;
	type OnboardingQueuePageSize = ConstU32<30>;
	type StaleAliasCleanupInterval = ConstU32<{ 5 * RC_MINUTES }>;
	type SelfInclusionDelay = SelfInclusionDelayValue;
	type ManagerOrigin = RootOrFellows;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = benchmark_utils::PeopleBenchHelper;
}

impl indiv_pallet_people_lite::Config for Runtime {
	type WeightInfo = weights::indiv_pallet_people_lite::WeightInfo<Runtime>;
	type Currency = Balances;
	type PotId = LitePeoplePotId;
	type RegistrationFee = crate::parameters::LitePersonRegistrationFee;
	type Suffix = NetworkSuffix;
	type AttestationAllowanceManager = RootOrFellows;
	type MemberService = Members;
	type CollectionOwner = LitePeopleCollectionOwner;
	type LiteRingExponent = LitePeopleRingExponent;
	type LiteOnboardingSize = LitePeopleOnboardingSize;
	type AttestationSignature = Signature;
	type LiteConsumerRegistrar = Resources;
	type AccountContexts = LiteAccountContexts;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = benchmark_utils::PeopleLiteBenchmarkHelper;
}

impl indiv_pallet_dummy_dim::Config for Runtime {
	type WeightInfo = weights::indiv_pallet_dummy_dim::WeightInfo<Runtime>;
	type UpdateOrigin = RootOrFellows;
	type MaxPersonBatchSize = ConstU32<1000>;
	type People = People;
}

parameter_types! {
	pub const LitePeoplePotId: PalletId = PalletId(*b"plitefee");
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
	pub const MaxUsernameLength: u32 = 32;
	pub const MinUsernameLength: u32 = 6;
	pub const PersonAuthDuration: u32 = 2 * 24 * 60 * 60; // 2 days
	pub const MinPersonAuthUpdateInterval: u32 = 24 * 60 * 60; // 1 day
	pub const MaxReservationQueueLength: u32 = 10;
}

impl indiv_pallet_resources::Config for Runtime {
	type WeightInfo = weights::indiv_pallet_resources::WeightInfo<Runtime>;
	type Suffix = NetworkSuffix;
	type MemberService = Members;
	type MinUsernameLength = MinUsernameLength;
	type PersonAuthDuration = PersonAuthDuration;
	type AccountsApiAllowance = crate::parameters::AccountsApiAllowance;
	type StmtStoreSlotsPerPeriod = crate::parameters::StmtStoreSlotsPerPeriod;
	type LiteStmtStoreSlotsPerPeriod = crate::parameters::LiteStmtStoreSlotsPerPeriod;
	type StmtStoreCleanupLimit = crate::parameters::StmtStoreCleanupLimit;
	type StmtStoreReplacementCooldown = crate::parameters::StmtStoreReplacementCooldown;
	type StmtStoreGraceWindow = crate::parameters::StmtStoreGraceWindow;
	type NotificationAllowance = crate::parameters::NotificationAllowance;
	type NotificationSlotsPerPeriod = crate::parameters::NotificationSlotsPerPeriod;
	type LiteNotificationSlotsPerPeriod = crate::parameters::LiteNotificationSlotsPerPeriod;
	type NotificationPeriodDuration = crate::parameters::NotificationPeriodDuration;
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
	type LongTermStoragePeriodDuration = crate::parameters::LongTermStoragePeriodDuration;
	type LongTermStorageGraceWindow = crate::parameters::LongTermStorageGraceWindow;
	type LongTermStorageClaimsPerPeriod = crate::parameters::LongTermStorageClaimsPerPeriod;
	type LongTermStorageAllowanceForPeople = crate::parameters::LongTermStorageAllowanceForPeople;
	type LongTermStorageAllowanceForLitePeople =
		crate::parameters::LongTermStorageAllowanceForLitePeople;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type LongTermStorageDataStore = BulletinDataStore;
	#[cfg(feature = "runtime-benchmarks")]
	type LongTermStorageDataStore = benchmark_utils::BenchmarkDataStore;
	type LongTermStorageCleanupLimit = crate::parameters::LongTermStorageCleanupLimit;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = benchmark_utils::ResourcesBenchHelper;
}

parameter_types! {
	/// Coinage's pallet id, used to derive the account holding the assets backing all coins.
	pub const CoinagePalletId: PalletId = PalletId(*b"coinage ");
	pub const CoinageInstanceCreationHoldReason: RuntimeHoldReason =
		RuntimeHoldReason::Coinage(indiv_pallet_coinage::HoldReason::InstanceCreationDeposit);
	/// The load deposit is held in DOT from a sponsored instance's pot, one per loaded recycler
	/// key. Governance sets the price through `pallet-parameters`
	/// (`dynamic_params::coinage::LoadDepositPrice`).
	pub CoinageLoadDeposit: (Location, Balance) =
		(xcm_config::RelayLocation::get(), parameters::CoinageLoadDepositPrice::get());
}

impl indiv_pallet_coinage::Config for Runtime {
	type WeightInfo = weights::indiv_pallet_coinage::WeightInfo<Runtime>;
	type PalletId = CoinagePalletId;
	type UnixTime = RuntimeClock;
	type MemberService = Members;
	type RecyclerRingExponent = RecyclerRingExponent;
	type PaidUnloadTokenRingExponent = PaidUnloadTokenRingExponent;
	type NativeFungible = Balances;
	type Fungibles = NativeAndAssets;
	type AdminOrigin = EitherOfDiverse<
		RootOrFellows,
		EnsureXcm<IsVoiceOfBody<AssetHubLocation, TechnicalMaintenanceBodyId>>,
	>;
	type SponsorOrigin = frame_system::EnsureSigned<AccountId>;
	type EnablePermissionless = ConstBool<true>;
	type LoadDeposit = CoinageLoadDeposit;
	type InstanceCreationDeposit = HoldConsideration<
		AccountId,
		Balances,
		CoinageInstanceCreationHoldReason,
		ConstantStoragePrice<parameters::CoinageInstanceCreationDeposit, Balance>,
	>;
	type MinimumExponent = ConstI8<0>;
	type MaximumExponent = ConstI8<14>;
	type MinimumExponentForOutputUnloadFee = ConstI8<0>;
	type MaximumAge = ConstU16<16>;
	type MaxSplitOutputs = ConstU32<32>;
	type MaxConsolidation = ConstU32<64>;
	type MaxBatchUnpaidLoad = ConstU32<10>;

	type RecyclerExpirationTime = ConstU32<{ 365 * 24 * 60 * 60 }>; // ~ 1 year
	type PaidUnloadTokenTimePeriod = ConstU32<{ 7 * 24 * 60 * 60 }>;
	type PaidUnloadTokenRingExpirationTime = ConstU32<{ 7 * 24 * 60 * 60 }>;

	type MembershipProof = People;
	type UnloadTokenTimePeriodPeopleLitePeople = ConstU32<{ 24 * 60 * 60 }>; // 1 day
	// Free unload token allowance per time period, expressed in DOT (the pallet's native balance,
	// not HOLLAR): 20 DOT for people and 10 DOT for lite people. The fee is dynamic (it follows
	// the fee multiplier), and usage is additionally capped by `MaxFreeUnloadTokensPerTimePeriod`.
	type UnloadTokenAllowancePerTimePeriodForPeople = ConstU128<{ 20 * UNITS }>;
	type UnloadTokenAllowancePerTimePeriodForLitePeople = ConstU128<{ 10 * UNITS }>;
	type MaxFreeUnloadTokensPerTimePeriod = ConstU32<1000>;

	type FeeConversion = AssetConversion;
	type NativeAssetKind = xcm_config::RelayLocation;
	type FeeDestination = TypedGetToGet<pallet_collator_selection::StakingPotAccountId<Runtime>>;
	type WeightToFee = TransactionPayment;
	type OffchainWorkerInterval = ConstU32<4>;
	/// Base lock applied to a coin after a failed `AsCoin` dispatch; grows as `2^retries * base`.
	type CoinFailureLockPeriod = ConstU64<3600>;

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

/// Pallet index of `MembersSubscriber` in Asset Hub Polkadot's `construct_runtime!`.
pub const ASSET_HUB_MEMBERS_SUBSCRIBER_INDEX: u8 = 97;

parameter_types! {
	pub AssetHubSubscriptionWhitelist:
		alloc::vec::Vec<indiv_pallet_members_notifier::GenesisWhitelistEntry> =
			asset_hub_subscription_whitelist();
}

/// One-shot subscriptions any signed account may activate with
/// `MembersNotifier::subscribe_whitelisted`, seeded into storage by
/// [`SeedAssetHubSubscriptionWhitelist`](crate::migrations::SeedAssetHubSubscriptionWhitelist).
///
/// This is what lets Asset Hub Polkadot subscribe without a governance call: the collections,
/// their exponents and the subscriber pallet index are fixed here, so the permissionless call can
/// only consume the entry exactly as written. Once consumed, only `ManageOrigin` can re-subscribe.
///
/// Identifiers must be strictly ascending, or the entry is rejected as malformed.
pub fn asset_hub_subscription_whitelist(
) -> alloc::vec::Vec<indiv_pallet_members_notifier::GenesisWhitelistEntry> {
	alloc::vec![indiv_pallet_members_notifier::GenesisWhitelistEntry {
		para_id: ParaId::from(ASSET_HUB_ID),
		collections: alloc::vec![
			(*PEOPLE_IDENTIFIER, MembersFlexibleRingExponent::get().exponent()),
			(*PEOPLE_LITE_IDENTIFIER, LitePeopleRingExponent::get().exponent()),
		],
		pallet_index: ASSET_HUB_MEMBERS_SUBSCRIBER_INDEX,
	}]
}

impl indiv_pallet_members_notifier::Config for Runtime {
	type WeightInfo = weights::indiv_pallet_members_notifier::WeightInfo<Runtime>;
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

/// The anonymous origins this runtime rate-limits, and the key their allowance is tracked under.
#[derive(
	Clone, Encode, Decode, Debug, MaxEncodedLen, TypeInfo, Eq, PartialEq, DecodeWithMemTracking,
)]
pub enum RestrictedEntity {
	PersonalAlias(Alias),
	PersonalIdentity(u64),
	LitePerson(AccountId),
	LiteAlias(Alias),
}

impl indiv_pallet_origin_restriction::RestrictedEntity<OriginCaller, Balance> for RestrictedEntity {
	fn allowance(&self) -> Allowance<Balance> {
		match self {
			RestrictedEntity::PersonalAlias(_) | RestrictedEntity::PersonalIdentity(_) => {
				Allowance {
					max: crate::parameters::PeopleIdentityAndAliasAllowanceMax::get(),
					recovery_per_block:
						crate::parameters::PeopleIdentityAndAliasAllowanceRecovery::get(),
				}
			},
			RestrictedEntity::LitePerson(_) | RestrictedEntity::LiteAlias(_) => Allowance {
				max: crate::parameters::LitePeopleAllowanceMax::get(),
				recovery_per_block: crate::parameters::LitePeopleAllowanceRecovery::get(),
			},
		}
	}

	fn restricted_entity(origin_caller: &OriginCaller) -> Option<Self> {
		use indiv_pallet_people::Origin::*;
		use indiv_pallet_people_lite::Origin::*;
		match origin_caller {
			OriginCaller::People(PersonalIdentity(id)) => {
				Some(RestrictedEntity::PersonalIdentity(*id))
			},
			OriginCaller::People(PersonalAlias(rev_ca)) => {
				Some(RestrictedEntity::PersonalAlias(rev_ca.ca.alias))
			},
			OriginCaller::PeopleLite(LitePerson(account_id)) => {
				Some(RestrictedEntity::LitePerson(account_id.clone()))
			},
			OriginCaller::PeopleLite(LiteAlias(rev_ca)) => {
				Some(RestrictedEntity::LiteAlias(rev_ca.ca.alias))
			},
			_ => None,
		}
	}
}

/// Calls that an entity with a zero allowance may still dispatch once, going into debt.
pub struct OperationAllowedOneTimeExcess;
impl ContainsPair<RestrictedEntity, RuntimeCall> for OperationAllowedOneTimeExcess {
	#[cfg(not(feature = "runtime-benchmarks"))]
	fn contains(_entity: &RestrictedEntity, _call: &RuntimeCall) -> bool {
		false
	}

	// We need to have one for benchmarks.
	#[cfg(feature = "runtime-benchmarks")]
	fn contains(entity: &RestrictedEntity, call: &RuntimeCall) -> bool {
		matches!(
			(entity, call),
			(
				RestrictedEntity::LitePerson(_),
				RuntimeCall::System(frame_system::Call::remark { .. })
			)
		)
	}
}

impl indiv_pallet_origin_restriction::Config for Runtime {
	type WeightInfo = weights::indiv_pallet_origin_restriction::WeightInfo<Runtime>;
	type BlockNumberProvider = RelaychainDataProvider<Runtime>;
	type RestrictedEntity = RestrictedEntity;
	type OperationAllowedOneTimeExcess = OperationAllowedOneTimeExcess;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = benchmark_utils::OriginRestrictionBenchmarkHelper;
}

/// Call encoding for the Bulletin Chain `TransactionStorage` calls invoked over XCM.
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
		if destination.parents == 1
			&& matches!(destination.interior.as_slice(), [Junction::Parachain(_)])
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
			UnixTime,
			fungibles::{Create, Inspect, Mutate},
		},
	};
	use indiv_support::{
		genesis::ring_verifier_builder_params,
		traits::{AddOnlyPeopleTrait, AppendOnlyMembers, RingMode},
	};
	use sp_runtime::traits::IdentifyAccount;
	use verifiable::ring::RingDomainSize;

	/// Reads `pallet_timestamp::Now` directly, deliberately skipping `pallet_timestamp::Pallet`'s
	/// `UnixTime` impl so that the `log::error!` it emits for a zero timestamp does not fire on
	/// every call during benchmarking. Behaviour is otherwise identical.
	pub struct BenchmarkClock;
	impl UnixTime for BenchmarkClock {
		fn now() -> core::time::Duration {
			core::time::Duration::from_millis(pallet_timestamp::Now::<Runtime>::get())
		}
	}

	pub struct PeopleLiteBenchmarkHelper;
	impl indiv_pallet_people_lite::BenchmarkHelper<AccountId, Signature> for PeopleLiteBenchmarkHelper {
		fn sign_message(message: &[u8]) -> (AccountId, Signature) {
			<() as indiv_pallet_people_lite::BenchmarkHelper<AccountId, Signature>>::sign_message(
				message,
			)
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

	/// Idempotently creates the stable asset so the value-carrying flows have something to move.
	pub fn ensure_stable_asset_exists() {
		let asset = HollarLocation::get();
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
				.take(255)
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
			indiv_pallet_resources::Pallet::<Runtime>::resources_context()
		}

		fn initialize_chunks() -> Vec<<BandersnatchVrfVerifiable as GenerateVerifiable>::StaticChunk>
		{
			let domain: RingDomainSize = MembersFlexibleRingExponent::get()
				.try_into()
				.expect("people ring exponent maps to a ring domain size");
			ring_verifier_builder_params(domain)
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
				BandersnatchVrfVerifiable::new_secret(sp_crypto_hashing::twox_256(b"honour-bench-voter"));
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
				HollarLocation::get(),
			)
			.next()
			.is_none()
			{
				let asset = HollarLocation::get();
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
			let asset = HollarLocation::get();
			<AssetsWithHolder as Mutate<_>>::mint_into(
				asset.clone(),
				&Coinage::pallet_account(),
				<AssetsWithHolder as Inspect<_>>::minimum_balance(asset.clone()),
			)
			.expect("benchmark: coinage pallet account must be fundable");
			asset
		}

		fn fund_account(who: &AccountId, amount: Balance) {
			<AssetsWithHolder as Mutate<_>>::mint_into(HollarLocation::get(), who, amount)
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
		}

		fn create_people_proof(
			context: &[u8],
			msg: &[u8],
			_alias: Alias,
		) -> indiv_pallet_people::MembershipProof<Runtime> {
			// Initialize the people collection and chunks if not already created.
			indiv_pallet_people::Pallet::<Runtime>::initialize_people_collection();
			let ring_exponent = <Runtime as indiv_pallet_people::Config>::RingExponent::get();
			indiv_pallet_members::Pallet::<Runtime>::initialize_chunks(ring_exponent);

			let secret =
				BandersnatchVrfVerifiable::new_secret(sp_crypto_hashing::twox_256(b"people_for_coinage:42"));
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

			indiv_pallet_people::MembershipProof { proof, ring: ring_index, revision: 0 }
		}

		fn create_lite_people_proof(
			context: &[u8],
			msg: &[u8],
			_alias: Alias,
		) -> indiv_pallet_people::MembershipProof<Runtime> {
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

			indiv_pallet_people::MembershipProof { proof, ring: ring_index, revision: 0 }
		}
	}

	pub struct OriginRestrictionBenchmarkHelper;
	impl indiv_pallet_origin_restriction::BenchmarkHelper<OriginCaller, RuntimeCall>
		for OriginRestrictionBenchmarkHelper
	{
		fn excess_pair() -> (OriginCaller, RuntimeCall) {
			(
				OriginCaller::PeopleLite(indiv_pallet_people_lite::Origin::LitePerson(
					AccountId::new([0u8; 32]),
				)),
				RuntimeCall::System(frame_system::Call::remark { remark: Vec::new() }),
			)
		}
	}
}
