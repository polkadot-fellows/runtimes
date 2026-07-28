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

//! Coinage: bearer-instrument "coins" backed by an external stablecoin.
//!
//! A coin is an anonymous, transferable claim on a fixed amount of an underlying fungible asset.
//! Coins are held by ring-VRF aliases rather than accounts, so `pallet-coinage` needs a ring
//! membership service ([`indiv_pallet_members`]) which in turn needs the ring-VRF SRS chunks
//! ([`indiv_pallet_chunks_manager`]).
//!
//! # Scope on People Polkadot
//!
//! People Polkadot deploys neither `pallet-people` nor `pallet-people-lite`, so there is no
//! source of personhood on this chain. Consequently the two *free* unload token flows are
//! permanently disabled — see [`NoMembershipProof`]. The paid flows
//! (`AsUnloadTokenPaid`, `AsUnloadTokenFromOutput`) and the plain `AsCoin` flow are fully
//! functional; they authenticate against coinage's own recycler and paid-unload-token rings,
//! which coinage populates itself.
//!
//! # Deployment steps
//!
//! Coinage is inert until the Bandersnatch ring-VRF SRS is on chain and a backing asset has been
//! nominated:
//!
//! 1. `ChunksManager::set_chunk_page_hashes` (root) — commit the expected hash of each SRS chunk
//!    page, per ring exponent. This must come first: `add_chunks` rejects any page that has no
//!    committed hash to match against. At genesis the same thing can be done through
//!    `ChunksManagerConfig::encoded_chunk_page_hashes`.
//! 2. `ChunksManager::add_chunks` — upload the chunk pages themselves. This call is *permissionless
//!    and authorized*, not root: its validity comes from the page hashing to the committed value,
//!    so anyone can supply the data. Until the chunks are present no ring root can be built and no
//!    coin can be created.
//! 3. `Coinage::set_underlying_asset_id` (root) — nominate the backing asset. It can only be set
//!    once. See `Config::UnderlyingAssetUnit` below for the constraint this places on which asset
//!    may be chosen.

use super::*;

use frame_support::{
	parameter_types,
	traits::{ConstU128, ConstU32, ConstU64, ConstUint, PalletInfoAccess},
};
use frame_system::EnsureRoot;
use indiv_support::{
	crypto::{BandersnatchVrfVerifiable, GenerateVerifiable},
	traits::{Alias, Identifier, RingExponent},
	utils::TypedGetToGet,
};
use sp_runtime::traits::{ConstI8, ConstU16};
use xcm::v5::{Junction::PalletInstance, Location};

/// The full-featured fungibles implementation, combining `pallet-assets` balances with the
/// hold functionality supplied by `pallet-assets-holder`.
///
/// `pallet-assets` alone does not implement `fungibles::MutateHold`, which coinage requires in
/// order to lock the underlying asset backing a coin.
pub type AssetsWithHolder = indiv_support::fungibles::CombineAssetsWithHolder<Assets, AssetsHolder>;

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

	/// Largest ring exponent usable by `Flexible` collections in `pallet-members`. This also
	/// determines the page size of paginated ring key storage.
	pub const MembersFlexibleRingExponent: RingExponent = RingExponent::R2e9;

	/// Ring exponent for coinage's recycler collections.
	///
	/// `R2e10` reserves 257 of the 2^10 domain slots for proof-system overhead, giving an
	/// effective capacity of 767 members per ring.
	///
	/// NOTE: changing this on a live chain requires substantial migration work.
	pub const RecyclerRingExponent: RingExponent = RingExponent::R2e10;

	/// Ring exponent for coinage's paid unload token collections.
	pub const PaidUnloadTokenRingExponent: RingExponent = RingExponent::R2e10;

	/// Coinage's pallet id, used to derive the account holding the assets backing all coins.
	pub const CoinagePalletId: PalletId = PalletId(*b"coinage ");

	/// Owner of the coinage collections in `pallet-members`. Set to coinage's own location so that
	/// no other origin can manage them.
	///
	/// The index is read from the pallet itself rather than written out: this value identifies the
	/// owner of existing ring collections, so if it ever drifted from coinage's real index the
	/// collections would be silently orphaned.
	pub CoinageCollectionOwner: Location =
		Location::new(0, [PalletInstance(<Coinage as PalletInfoAccess>::index() as u8)]);
}

impl indiv_pallet_chunks_manager::Config for Runtime {
	// TODO: replace with People Polkadot weights once benchmarks have been run for this runtime.
	// `()` is the zero-weight implementation and must not reach production as-is.
	type WeightInfo = ();
	type Chunk = <BandersnatchVrfVerifiable as GenerateVerifiable>::StaticChunk;
	type PageSize = ChunkPageSize;
	type ManagerOrigin = EnsureRoot<AccountId>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = benchmark_utils::ChunksManagerBenchHelper;
}

impl indiv_pallet_members::Config for Runtime {
	// TODO: replace with People Polkadot weights once benchmarks have been run for this runtime.
	// `()` is the zero-weight implementation and must not reach production as-is.
	type WeightInfo = ();
	type Crypto = BandersnatchVrfVerifiable;
	type Location = Location;
	type ChunksManager = ChunksManager;
	type Clock = RuntimeClock;
	type MaxCollections = ConstU32<100>;
	type OnboardingQueuePageSize = ConstU32<255>;
	type MaxFlexibleRingExponent = MembersFlexibleRingExponent;
	type RingBuildingMemberLimit = ConstU32<100>;
	/// 10 minutes, so proofs against a superseded root stay valid for a grace period.
	type OldRootRetentionDuration = ConstU64<600>;
	/// Nothing on this chain consumes ring roots: `pallet-members-notifier` is not deployed, so
	/// roots are not forwarded to other parachains.
	type OnRingRootChange = ();
	type OffchainWorkerInterval = ConstU32<1>;
	type ManagerOrigin = EnsureRoot<AccountId>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = benchmark_utils::MembersBenchHelper;
}

/// Membership proof validator for coinage's *free* unload token flows, which always rejects.
///
/// Free unload tokens are granted to proven people and lite people. People Polkadot deploys
/// neither `pallet-people` nor `pallet-people-lite`, so no such proof can legitimately exist
/// here and no free unload token should ever be issued.
///
/// Rejecting unconditionally makes `AsCoinageInfo::AsUnloadTokenPeople` and
/// `AsCoinageInfo::AsUnloadTokenLitePeople` fail transaction validation with
/// `CustomInvalidity::InvalidUnloadTokenProof`. Because validation fails, such transactions never
/// enter a block. The paid unload token flows are unaffected.
///
/// `Proof = ()` keeps the two disabled extension variants free of dead payload bytes. Enabling
/// personhood later means swapping this for a real ring-membership validator over
/// [`Identifier`]-keyed collections in `pallet-members`, which changes the transaction encoding
/// and therefore requires a `transaction_version` bump.
pub struct NoMembershipProof;

impl indiv_pallet_coinage::ValidateProof for NoMembershipProof {
	type Proof = ();

	fn validate_proof(
		_identifier: &Identifier,
		_proof: &Self::Proof,
		_context: &[u8],
		_msg: &[u8],
	) -> Result<Alias, ()> {
		Err(())
	}
}

/// Membership proof validator used **only** when benchmarking, which accepts any proof.
///
/// [`NoMembershipProof`] rejects unconditionally, so `as_unload_token_people_tx_ext` and
/// `as_unload_token_lite_people_tx_ext` would abort instead of yielding a weight. Substituting a
/// permissive validator under `runtime-benchmarks` lets the whole coinage benchmark set run.
///
/// The resulting weights understate the real cost, because this skips the ring-VRF verification a
/// genuine validator would perform. That is immaterial here: both flows are unreachable in
/// production, where [`NoMembershipProof`] is used instead.
#[cfg(feature = "runtime-benchmarks")]
pub struct AnyMembershipProof;

#[cfg(feature = "runtime-benchmarks")]
impl indiv_pallet_coinage::ValidateProof for AnyMembershipProof {
	/// The alias the proof claims to attest to. `validate_proof` echoes it back, so each benchmark
	/// iteration gets a distinct alias and exercises the real `ConsumedFreeUnloadTokens` lookup.
	type Proof = Alias;

	fn validate_proof(
		_identifier: &Identifier,
		proof: &Self::Proof,
		_context: &[u8],
		_msg: &[u8],
	) -> Result<Alias, ()> {
		Ok(*proof)
	}
}

impl indiv_pallet_coinage::Config for Runtime {
	// TODO: replace with People Polkadot weights once benchmarks have been run for this runtime.
	// `()` is the zero-weight implementation and must not reach production as-is.
	type WeightInfo = ();
	type PalletId = CoinagePalletId;
	type UnixTime = RuntimeClock;
	type MemberService = Members;
	type CollectionOwner = CoinageCollectionOwner;
	type RecyclerRingExponent = RecyclerRingExponent;
	type PaidUnloadTokenRingExponent = PaidUnloadTokenRingExponent;

	type NativeFungible = Balances;
	type Fungibles = AssetsWithHolder;
	type UnderlyingAssetIdManager = EnsureRoot<AccountId>;
	type ConversionToAssetBalance = AssetRate;

	// Coin values are `2^exponent * UnderlyingAssetUnit`, so with a unit of $0.01 the
	// denominations run from $0.01 (exponent 0) up to $163.84 (exponent 14).
	type MinimumExponent = ConstI8<0>;
	type MaximumExponent = ConstI8<14>;
	type MinimumExponentForOutputUnloadFee = ConstI8<0>;
	/// The underlying amount backing one coin at exponent 0.
	///
	/// NOTE: this is a compile-time constant while the backing asset is chosen at runtime via
	/// `set_underlying_asset_id`. The value below assumes a 6-decimal stablecoin (as used by
	/// USDT/USDC on Asset Hub Polkadot), for which `10^4` is $0.01. Nominating an asset with
	/// different decimals silently rescales every denomination, so the asset must either have 6
	/// decimals or this constant must be changed in the same runtime upgrade that sets it.
	type UnderlyingAssetUnit = ConstUint<{ 10u128.pow(4) }>;

	type MaximumAge = ConstU16<16>;
	type MaxSplitOutputs = ConstU32<32>;
	type MaxConsolidation = ConstU32<64>;
	type MaxBatchUnpaidLoad = ConstU32<10>;

	/// ~3 months before a full recycler ring can be cleaned up.
	type RecyclerExpirationTime = ConstU32<{ 90 * 24 * 60 * 60 }>;
	type PaidUnloadTokenTimePeriod = ConstU32<{ 3 * 24 * 60 * 60 }>;
	type PaidUnloadTokenRingExpirationTime = ConstU32<{ 4 * 24 * 60 * 60 }>;

	// Free unload tokens are disabled on this chain: `NoMembershipProof` rejects every proof, and
	// the allowances are kept at zero so the constants in metadata state the same thing.
	//
	// Under `runtime-benchmarks` both are relaxed, because either one alone short-circuits the
	// flow and prevents the two free-unload-token benchmarks from producing a weight: a zero
	// allowance makes `free_unload_token_limit_*` return 0, which fails the `counter >= limit`
	// check *before* the proof is ever validated. The values mirror the individuality reference
	// runtime.
	#[cfg(not(feature = "runtime-benchmarks"))]
	type MembershipProof = NoMembershipProof;
	#[cfg(feature = "runtime-benchmarks")]
	type MembershipProof = AnyMembershipProof;

	type UnloadTokenTimePeriodPeopleLitePeople = ConstU32<{ 24 * 60 * 60 }>;

	#[cfg(not(feature = "runtime-benchmarks"))]
	type UnloadTokenAllowancePerTimePeriodForPeople = ConstU128<0>;
	#[cfg(feature = "runtime-benchmarks")]
	type UnloadTokenAllowancePerTimePeriodForPeople = ConstU128<{ 2000 * 10u128.pow(4) }>;

	#[cfg(not(feature = "runtime-benchmarks"))]
	type UnloadTokenAllowancePerTimePeriodForLitePeople = ConstU128<0>;
	#[cfg(feature = "runtime-benchmarks")]
	type UnloadTokenAllowancePerTimePeriodForLitePeople = ConstU128<{ 1000 * 10u128.pow(4) }>;

	#[cfg(not(feature = "runtime-benchmarks"))]
	type MaxFreeUnloadTokensPerTimePeriod = ConstU32<0>;
	#[cfg(feature = "runtime-benchmarks")]
	type MaxFreeUnloadTokensPerTimePeriod = ConstU32<1000>;

	type FeeDestination = TypedGetToGet<pallet_collator_selection::StakingPotAccountId<Runtime>>;
	type WeightToFee = TransactionPayment;
	type OffchainWorkerInterval = ConstU32<4>;
	/// Base lock applied to a coin after a failed `AsCoin` dispatch; grows as `2^retries * base`.
	type CoinFailureLockPeriod = ConstU64<60>;

	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = benchmark_utils::CoinageBenchHelper;
}

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmark_utils {
	use super::*;
	use alloc::vec::Vec;
	use frame_support::traits::{
		fungibles::{Create, Inspect, Mutate},
		UnixTime,
	};
	use indiv_support::genesis::ring_verifier_builder_params;
	use sp_runtime::{traits::AccountIdConversion, FixedU128};
	use verifiable::ring::RingDomainSize;
	use xcm::v5::Junction::{GeneralIndex, Parachain};

	/// Reads `pallet_timestamp::Now` directly, deliberately skipping
	/// `pallet_timestamp::Pallet`'s `UnixTime` impl so that the `log::error!` it emits for a zero
	/// timestamp does not fire on every call during benchmarking. Behaviour is otherwise identical.
	pub struct BenchmarkClock;
	impl UnixTime for BenchmarkClock {
		fn now() -> core::time::Duration {
			core::time::Duration::from_millis(pallet_timestamp::Now::<Runtime>::get())
		}
	}

	/// Asset used as the coin backing during benchmarks. Mirrors the shape of a stablecoin
	/// held on Asset Hub Polkadot; the real asset is chosen on-chain by root.
	pub fn benchmark_asset() -> Location {
		Location::new(1, [Parachain(1000), PalletInstance(50), GeneralIndex(1984)])
	}

	pub struct ChunksManagerBenchHelper;
	impl
		indiv_pallet_chunks_manager::BenchmarkHelper<
			<BandersnatchVrfVerifiable as GenerateVerifiable>::StaticChunk,
		> for ChunksManagerBenchHelper
	{
		fn chunk_page() -> Vec<<BandersnatchVrfVerifiable as GenerateVerifiable>::StaticChunk> {
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

	pub struct CoinageBenchHelper;
	impl indiv_pallet_coinage::BenchmarkHelper<Runtime> for CoinageBenchHelper {
		fn setup_assets() {
			let asset = benchmark_asset();
			if !Assets::asset_exists(asset.clone()) {
				<Assets as Create<_>>::create(
					asset.clone(),
					CoinagePalletId::get().into_account_truncating(),
					true,
					1u128,
				)
				.expect("benchmark: backing asset must be creatable");
			}
			if !indiv_pallet_coinage::UnderlyingAssetId::<Runtime>::exists() {
				indiv_pallet_coinage::UnderlyingAssetId::<Runtime>::put(asset);
			}
		}

		fn fund_account(who: &AccountId, amount: Balance) {
			<AssetsWithHolder as Mutate<_>>::mint_into(benchmark_asset(), who, amount)
				.expect("benchmark: account must be fundable");
		}

		fn set_time(now: core::time::Duration) {
			pallet_timestamp::Now::<Runtime>::put(now.as_millis() as u64);
		}

		fn setup_conversion_rate() {
			// DOT has 10 decimals and the backing asset 6, so one raw asset unit ($10^-6) is
			// 10^4 raw DOT ($10^-10).
			pallet_asset_rate::ConversionRateToNative::<Runtime>::insert(
				benchmark_asset(),
				FixedU128::from_u32(10_000),
			);
		}

		// `AnyMembershipProof::Proof` is the alias itself, so the "proof" these produce is just
		// the alias the caller asked to be attested. See `AnyMembershipProof` for why the
		// resulting weights understate the real cost of a genuine ring-VRF verification.
		fn create_people_proof(_context: &[u8], _msg: &[u8], alias: Alias) -> Alias {
			alias
		}

		fn create_lite_people_proof(_context: &[u8], _msg: &[u8], alias: Alias) -> Alias {
			alias
		}
	}
}
