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

//! The Individuality SDK on Asset Hub Polkadot.
//!
//! Personhood itself lives on People Polkadot; this is the consumer side.
//!
//! # The pieces
//!
//! * [`indiv_pallet_members_subscriber`] mirrors the ring roots that People Polkadot's
//!   `pallet-members-notifier` publishes over XCM, so that ring-VRF membership proofs can be
//!   verified locally without trusting a bridge. Everything below depends on it.
//! * [`indiv_pallet_alias_accounts`] binds an account to a context-scoped anonymous alias, which is
//!   how a contract or a dApp learns "this account belongs to a distinct person" without learning
//!   who.
//! * [`indiv_precompile_personhood`] exposes that check to `pallet-revive` contracts.
//! * [`indiv_pallet_pgas`] lets a proven person periodically claim PGAS, an execution allowance
//!   asset. [`pallet_pgas_allowance`] then lets PGAS pay the fees of contract calls, and
//!   `pallet_revive::PGasDeposit` makes contract storage deposits PGAS-denominated — so a person
//!   can use contracts without holding DOT.
//!
//! # Deployment steps
//!
//! Enacting the runtime upgrade activates none of this on its own:
//!
//! 1. The PGAS asset must exist before any PGAS flow works. This is meant to happen automatically
//!    via `indiv_pallet_pgas::migration::CreatePgasAsset` in `migrations.rs` — but see the TODO
//!    there, because that migration cannot currently succeed on this chain.
//! 2. Subscription to People Polkadot's ring roots is driven from *there*, by
//!    `MembersNotifier::subscribe` naming this chain and the `MembersSubscriber` pallet index (97);
//!    there is no local call to make. Until the first batch of roots arrives, every personhood
//!    proof on this chain fails. Requires an open HRMP channel in both directions.
//! 3. Run collators with offchain workers enabled so the alias-account stale-mapping sweep can
//!    submit authorized maintenance calls. The alias fee is governance-mutable.

use super::*;

use frame_support::traits::{EnsureOrigin, Get};
use indiv_support::traits::RingExponent;
#[cfg(feature = "runtime-benchmarks")]
use indiv_support::traits::{Alias, Context, Identifier, RingIndex};
use polkadot_runtime_constants::system_parachain::{ASSET_HUB_ID, PEOPLE_ID};
use sp_runtime::traits::AccountIdConversion;

/// Root, the Technical Fellowship voice, or TechnicalMaintenance may administer Individuality
/// settings.
pub type RootOrFellows =
	EitherOfDiverse<EnsureRoot<AccountId>, EnsureXcm<IsFellowshipVoice<FellowshipLocation>>>;

/// The full administration origin shared by Individuality pallet managers and dynamic parameters.
pub type RootOrFellowsOrTechnicalMaintenance =
	EitherOfDiverse<RootOrFellows, TechnicalMaintenance>;

/// PGAS, the non-transferable gas allowance a proven person may claim.
///
/// The id sits far above the `AutoIncAssetId` range so that it can never collide with a
/// user-registered trust-backed asset.
///
/// NOTE: what actually keeps this id unreachable by a signed caller is `pallet-assets`' `id ==
/// NextAssetId` guard, not the size of the id. That guard also prevents the asset from being
/// *created* at this id — see the `CreatePgasAsset` TODO in `migrations.rs`.
pub const PGAS_ASSET_ID: AssetIdForTrustBackedAssets = 2_000_000_000;

parameter_types! {
	/// XCM location and pallet index of the `pallet-members-notifier` instance publishing ring
	/// roots.
	pub RingRootsNotifierEndpoint: indiv_pallet_members_subscriber::types::NotifierEndpoint =
		indiv_pallet_members_subscriber::types::NotifierEndpoint {
			location: Location::new(1, [Junction::Parachain(PEOPLE_ID)]),
			// Matches the `MembersNotifier` index in People Polkadot's `construct_runtime!`.
			pallet_index: 69,
		};
	pub const MembersSubscriberSelfParaId: u32 = ASSET_HUB_ID;

	/// Ring exponent of the people collection on People Polkadot. Must match
	/// `MembersFlexibleRingExponent` there, or proofs will not verify.
	pub const PeopleRingExponent: RingExponent = RingExponent::R2e9;
	/// Ring exponent of the lite people collection on People Polkadot.
	pub const PeopleLiteRingExponent: RingExponent = RingExponent::R2e9;
	/// Product-context suffix for the Polkadot deployment.
	pub const NetworkSuffix: &'static [u8] = b"polkadot";
}

/// Origin check restricted to the sibling parachain that publishes the ring roots.
pub struct EnsureNotifierSibling;
impl EnsureOrigin<RuntimeOrigin> for EnsureNotifierSibling {
	type Success = ();

	fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
		match o.clone().into() {
			Ok(cumulus_pallet_xcm::Origin::SiblingParachain(id)) if u32::from(id) == PEOPLE_ID => {
				Ok(())
			},
			_ => Err(o),
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
		Ok(cumulus_pallet_xcm::Origin::SiblingParachain(PEOPLE_ID.into()).into())
	}
}

impl indiv_pallet_members_subscriber::Config for Runtime {
	type WeightInfo = weights::indiv_pallet_members_subscriber::WeightInfo<Runtime>;
	type Crypto = indiv_support::crypto::BandersnatchVrfVerifiable;
	type XcmSender = xcm_config::XcmRouter;
	type RingRootsNotifier = RingRootsNotifierEndpoint;
	type SelfParaId = MembersSubscriberSelfParaId;
	type MaxMissingRootsPerCollection = ConstU32<255>;
	type MaxDeletedRingsPerCollection = ConstU32<100>;
	type MaxGapScanPerBatch = ConstU32<32>;
	type PurgePageSize = ConstU32<100>;
	type EnsureNotifierOrigin = EnsureNotifierSibling;
	type EnsureTerminationOrigin = EitherOfDiverse<EnsureRoot<AccountId>, EnsureNotifierSibling>;
	type MaxCollections = ConstU32<20>;
	type UnixTime = Timestamp;
	type ReplayCooldownSeconds = ConstU64<60>;
	type MaxUpdatesPerBatch = ConstU32<10>;
	type ReplayWarningThreshold = ConstU32<5>;
	type ReplayAbandonThreshold = ConstU32<10>;
	type MaxRecentRootsPerRing = ConstU32<3>;
	type OldRootRetentionDuration = ConstU64<600>;
	type OffchainWorkerInterval = ConstU32<3>;
}

/// Adapts the runtime's required alias fee to the alias-accounts pallet configuration.
pub struct AliasFee;
impl Get<Option<Balance>> for AliasFee {
	fn get() -> Option<Balance> {
		Some(dynamic_params::individuality::AliasFee::get())
	}
}

impl indiv_pallet_alias_accounts::Config for Runtime {
	type WeightInfo = weights::indiv_pallet_alias_accounts::WeightInfo<Runtime>;
	type MemberService = MembersSubscriber;
	type UnixTime = Timestamp;
	/// The default proof-validity window is five minutes after the timestamp it commits to.
	type ProofValidityWindow = dynamic_params::individuality::AliasProofValidityWindow;
	/// Retain released mappings for longer than any accepted ring-root revision.
	type MappingRetention = ConstU64<{ 90 * 24 * 60 * 60 }>;
	type PeopleLiteRingExponent = PeopleLiteRingExponent;
	type PeopleRingExponent = PeopleRingExponent;
	type Fungibles = Assets;
	type PgasAssetId = PgasAssetId;
	type AliasFee = AliasFee;
	type OffchainWorkerInterval = indiv_support::parameters::AtLeastOne<
		dynamic_params::individuality::StaleAliasSweepInterval,
	>;
	type MaxStaleAliasBatch = ConstU32<32>;
}

impl indiv_precompile_personhood::Config for Runtime {
	type Proof = indiv_pallet_alias_accounts::ProofOf<Runtime>;
	type PersonhoodResolver = AliasAccounts;
}

parameter_types! {
	pub const PgasPalletId: PalletId = PalletId(*b"py/pgas ");
	/// Owner and admin of the PGAS asset. PGAS is minted only by `pallet-pgas`, so the admin is a
	/// pallet-derived account nobody controls.
	pub PgasAdmin: AccountId = PgasPalletId::get().into_account_truncating();
	pub PgasAssetId: AssetIdForTrustBackedAssets = PGAS_ASSET_ID;
	pub PgasMinBalance: Balance = ExistentialDeposit::get() / 10;
}

impl indiv_pallet_pgas::Config for Runtime {
	type WeightInfo = weights::indiv_pallet_pgas::WeightInfo<Runtime>;
	type Suffix = NetworkSuffix;
	type MembershipProver = MembersSubscriber;
	type Clock = Timestamp;
	type Fungibles = Assets;
	type PgasAssetId = PgasAssetId;
	type PgasClaimAmount = dynamic_params::individuality::PgasClaimAmount;
	type MaxClaimsPerPeriodPerPerson = dynamic_params::individuality::MaxClaimsPerPeriodPerPerson;
	type MaxClaimsPerPeriodPerLitePerson =
		dynamic_params::individuality::MaxClaimsPerPeriodPerLitePerson;
	type MaxPgasClaimRecordCleanupPerCall =
		dynamic_params::individuality::MaxPgasClaimRecordCleanupPerCall;
	type PgasAdmin = PgasAdmin;
	type PgasMinBalance = PgasMinBalance;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = benchmark_utils::PgasBenchHelper;
}

impl pallet_pgas_allowance::Config for Runtime {
	type Assets = Assets;
	type PGASAssetId = PgasAssetId;
	// PGAS is a general Asset Hub fee asset, so every RuntimeCall may be paid with it.
	type CallFilter = frame_support::traits::Everything;
	type WeightInfo = weights::pallet_pgas_allowance::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = benchmark_utils::PGASBenchmarkHelper;
}

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmark_utils {
	use super::*;
	use frame_support::{
		traits::{
			fungibles::{Create, Inspect, Mutate},
		},
		BoundedVec,
	};
	use indiv_support::{
		crypto::{BandersnatchSuite, BandersnatchVrfVerifiable},
		genesis::ring_verifier_builder_params,
		traits::{RevisionIndex, PEOPLE_LITE_IDENTIFIER},
	};
	use verifiable::{ring::RingDomainSize, GenerateVerifiable};

	type Crypto = BandersnatchVrfVerifiable;

	pub const BENCH_ALIAS_CONTEXT: Context = *b"pop:ah-bench-context            ";

	pub fn alias_bench_entropy(seed: u32) -> [u8; 32] {
		let mut entropy = [0u8; 32];
		entropy[..4].copy_from_slice(&seed.to_le_bytes());
		entropy
	}

	/// Idempotently creates the PGAS asset so the paid flows have a destination for fee transfers.
	pub fn ensure_pgas_asset() {
		if !<Assets as Inspect<AccountId>>::asset_exists(PgasAssetId::get()) {
			<Assets as Create<AccountId>>::create(
				PgasAssetId::get(),
				PgasAdmin::get(),
				true,
				PgasMinBalance::get(),
			)
			.expect("benchmark: PGAS asset must be creatable");
		}
	}

	/// Returns the ring exponent `alias-accounts` verifies `identifier`'s proofs against.
	fn ring_exponent_for(identifier: &Identifier) -> RingExponent {
		if identifier == PEOPLE_LITE_IDENTIFIER {
			<Runtime as indiv_pallet_alias_accounts::Config>::PeopleLiteRingExponent::get()
		} else {
			<Runtime as indiv_pallet_alias_accounts::Config>::PeopleRingExponent::get()
		}
	}

	/// Builds a one-member Bandersnatch ring and returns everything needed to both seed its root
	/// and prove membership of it.
	///
	/// This mirrors `ring_setup` in Individuality's
	/// `runtimes/next-asset-hub-paseo/src/lib.rs`. `indiv_support::crypto` does not export the
	/// benchmark helper at this pinned revision; keep this single local mirror in sync with review
	/// r3734171704 until the SDK exports it.
	fn ring_setup(
		ring_exponent: RingExponent,
		entropy: [u8; 32],
	) -> (
		<Crypto as GenerateVerifiable>::Members,
		<Crypto as GenerateVerifiable>::Member,
		<Crypto as GenerateVerifiable>::Secret,
		RingDomainSize,
	) {
		let domain: RingDomainSize =
			ring_exponent.try_into().expect("RingExponent maps to RingDomainSize");
		let chunks = ring_verifier_builder_params::<BandersnatchSuite>(domain);

		let secret = Crypto::new_secret(entropy);
		let member = Crypto::member_from_secret(&secret);

		let mut intermediate = Crypto::start_members(domain);
		Crypto::push_members(&mut intermediate, core::iter::once(member), |range| {
			Ok(chunks[range].to_vec())
		})
		.expect("benchmark: push_members for a single member");
		(Crypto::finish_members(intermediate), member, secret, domain)
	}

	impl indiv_pallet_members_subscriber::benchmarking::BenchmarkHelper<Runtime> for Runtime {
		fn init() {
			use cumulus_pallet_parachain_system::RelevantMessagingState;
			use cumulus_primitives_core::relay_chain::AbridgedHrmpChannel;

			// The timestamp must exceed `ReplayCooldownSeconds` (60s) so
			// `authorize_replay_missing_roots` passes the cooldown check.
			pallet_timestamp::Now::<Runtime>::put(120_000u64);

			// Fake an HRMP egress channel to the publisher parachain so benchmarks that send a
			// replay request do not fail with `NoChannel`.
			let channel = AbridgedHrmpChannel {
				max_capacity: 1000,
				max_total_size: 1_000_000,
				max_message_size: 100_000,
				msg_count: 0,
				total_size: 0,
				mqc_head: None,
			};
			let messaging_state =
				cumulus_pallet_parachain_system::relay_state_snapshot::MessagingStateSnapshot {
					dmq_mqc_head: Default::default(),
					relay_dispatch_queue_remaining_capacity: Default::default(),
					ingress_channels: Vec::new(),
					egress_channels: vec![(ParaId::from(PEOPLE_ID), channel)],
				};
			RelevantMessagingState::<Runtime>::put(messaging_state);
		}

		fn mock_ring_root(seed: u32) -> indiv_pallet_members_subscriber::types::MembersOf<Runtime> {
			ring_setup(
				<Runtime as indiv_pallet_alias_accounts::Config>::PeopleRingExponent::get(),
				alias_bench_entropy(seed),
			)
			.0
		}
	}

	impl indiv_pallet_alias_accounts::benchmarking::BenchmarkHelper<Runtime> for Runtime {
		fn set_time(seconds: u64) {
			pallet_timestamp::Now::<Runtime>::put(seconds.saturating_mul(1_000));
		}

		fn allowed_context() -> Context {
			BENCH_ALIAS_CONTEXT
		}

		fn mock_proof(
			seed: u32,
			context: Context,
			msg: &[u8],
		) -> (indiv_pallet_alias_accounts::ProofOf<Runtime>, Alias) {
			let (_root, member, secret, domain) = ring_setup(
				<Runtime as indiv_pallet_alias_accounts::Config>::PeopleRingExponent::get(),
				alias_bench_entropy(seed),
			);
			let commitment = Crypto::open(domain, &member, core::iter::once(member))
				.expect("benchmark: open for a single-member ring");
			Crypto::create(commitment, &secret, &context[..], msg)
				.expect("benchmark: create for a valid commitment")
		}

		/// Seeds a single-member Bandersnatch ring at `(identifier, ring_index)` in
		/// members-subscriber storage and returns a real ring-VRF proof against it.
		fn create_proof_for_revision(
			identifier: &Identifier,
			ring_index: RingIndex,
			revision: RevisionIndex,
			context: &Context,
			message: &[u8],
		) -> indiv_pallet_alias_accounts::ProofOf<Runtime> {
			let ring_exponent = ring_exponent_for(identifier);
			let (root, member, secret, domain) = ring_setup(ring_exponent, [42u8; 32]);

			// The benchmark fills the sliding window with mock records before calling us; replace
			// the record matching `revision` with our real commitment so verification against
			// the bench-chosen target revision succeeds.
			let mut roots = indiv_pallet_members_subscriber::Pallet::<Runtime>::current_ring_roots(
				identifier, ring_index,
			)
			.expect("seed_ring populates RingRoots before create_proof_for_revision");
			let idx = roots
				.iter()
				.position(|r| r.revision == revision)
				.expect("requested revision must be present in seeded roots");
			roots[idx].root = root;
			indiv_pallet_members_subscriber::Pallet::<Runtime>::set_current_ring_roots(
				identifier, ring_index, roots,
			);
			indiv_pallet_members_subscriber::RingCollectionExponents::<Runtime>::insert(
				*identifier,
				ring_exponent,
			);

			let commitment = Crypto::open(domain, &member, core::iter::once(member))
				.expect("benchmark: open for a single-member ring");
			let (proof, _alias) = Crypto::create(commitment, &secret, &context[..], message)
				.expect("benchmark: create proof");
			proof
		}

		fn setup_pgas_asset() {
			ensure_pgas_asset();
		}

		fn set_alias_fee(fee: Balance) {
			pallet_parameters::Pallet::<Runtime>::set_parameter(
				RuntimeOrigin::root(),
				RuntimeParameters::Individuality(
					dynamic_params::individuality::Parameters::AliasFee(
						dynamic_params::individuality::AliasFee,
						Some(fee),
					),
				),
			)
			.expect("root may set the alias fee");
		}

		fn max_ring_revisions() -> u32 {
			<<Runtime as indiv_pallet_members_subscriber::Config>::MaxRecentRootsPerRing as Get<
				u32,
			>>::get()
		}

		fn seed_ring(collection: Identifier, ring: RingIndex, revisions: u32, source_time: u64) {
			use indiv_pallet_members_subscriber::types::RingCommitmentRecord;

			let ring_exponent = ring_exponent_for(&collection);
			indiv_pallet_members_subscriber::RingCollectionExponents::<Runtime>::insert(
				collection,
				ring_exponent,
			);

			let mut roots: BoundedVec<
				RingCommitmentRecord<Runtime>,
				<Runtime as indiv_pallet_members_subscriber::Config>::MaxRecentRootsPerRing,
			> = BoundedVec::new();
			for i in 0..revisions {
				let root =
					<Runtime as indiv_pallet_members_subscriber::benchmarking::BenchmarkHelper<
						Runtime,
					>>::mock_ring_root(i);
				roots
					.try_push(RingCommitmentRecord {
						root,
						revision: i,
						source_time,
						source_sequence: 1,
					})
					.expect("revisions bounded by max_ring_revisions");
			}
			indiv_pallet_members_subscriber::Pallet::<Runtime>::set_current_ring_roots(
				&collection,
				ring,
				roots,
			);
		}
	}

	pub struct PgasBenchHelper;
	impl indiv_pallet_pgas::benchmarking::BenchmarkHelper<Runtime> for PgasBenchHelper {
		fn set_time(now: core::time::Duration) {
			pallet_timestamp::Now::<Runtime>::put(now.as_millis() as u64);
		}

		fn seed_and_create_proof(
			identifier: &Identifier,
			ring_index: RingIndex,
			context: &Context,
			message: &[u8],
		) -> indiv_pallet_pgas::ProofOf<Runtime> {
			let ring_exponent = ring_exponent_for(identifier);
			let (root, member, secret, domain) = ring_setup(ring_exponent, [42u8; 32]);

			let record = indiv_pallet_members_subscriber::types::RingCommitmentRecord::<Runtime> {
				root,
				revision: 1,
				source_time: pallet_timestamp::Now::<Runtime>::get() / 1_000,
				source_sequence: 1,
			};
			let mut roots: BoundedVec<_, _> = Default::default();
			roots.try_push(record).expect("MaxRecentRootsPerRing > 0");
			indiv_pallet_members_subscriber::Pallet::<Runtime>::set_current_ring_roots(
				identifier, ring_index, roots,
			);
			indiv_pallet_members_subscriber::RingCollectionExponents::<Runtime>::insert(
				*identifier,
				ring_exponent,
			);

			let commitment = Crypto::open(domain, &member, core::iter::once(member))
				.expect("benchmark: open for a single-member ring");
			let (proof, _alias) = Crypto::create(commitment, &secret, &context[..], message)
				.expect("benchmark: create proof");
			proof
		}
	}

	pub struct PGASBenchmarkHelper;
	impl
		pallet_pgas_allowance::BenchmarkHelperTrait<AccountId, AssetIdForTrustBackedAssets, Balance>
		for PGASBenchmarkHelper
	{
		fn mint_pgas(who: &AccountId, asset_id: AssetIdForTrustBackedAssets, amount: Balance) {
			if !<Assets as Inspect<AccountId>>::asset_exists(asset_id) {
				<Assets as Create<AccountId>>::create(
					asset_id,
					PgasAdmin::get(),
					true,
					PgasMinBalance::get(),
				)
				.expect("benchmark: PGAS asset must be creatable");
			}
			<Assets as Mutate<AccountId>>::mint_into(asset_id, who, amount)
				.expect("benchmark: PGAS must be mintable");
		}
	}

}
