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
//! * [`indiv_pallet_dotns_gateway`] is the personhood-gated front door to the dotNS name registry:
//!   one name per person, claimed through a ring proof.
//! * [`indiv_pallet_scarcity`] implements NFTs held by purse keys on Coinage's model: one NFT per
//!   key, with feeless transfers authorized through its own origin modifier.
//! * [`indiv_pallet_nft_claims`] holds the NFT claim credit trees the game pallet on People
//!   Polkadot awards, and mints the Scarcity NFTs claimed against them.
//! * [`indiv_pallet_origin_restriction`] rate-limits the anonymous origins the extensions above
//!   produce, since those origins pay no fee from an account.

use super::*;

use frame_support::traits::{ConstU16, ContainsPair, EnsureOrigin, Get};
#[cfg(feature = "runtime-benchmarks")]
use indiv_support::traits::{Context, Identifier, RingIndex};
use indiv_support::{
	parameters::{AtLeastOne, AtMost, BenchmarkMax},
	traits::{Alias, RingExponent},
};
use polkadot_runtime_constants::system_parachain::{ASSET_HUB_ID, PEOPLE_ID};
use sp_runtime::traits::AccountIdConversion;
use system_parachains_constants::polkadot::INDIVIDUALITY_NETWORK_SUFFIX;

/// Root or the whitelisted caller origin.
pub type RootOrWhitelist = EitherOfDiverse<EnsureRoot<AccountId>, WhitelistedCaller>;

/// PGAS, the non-transferable gas allowance a proven person may claim.
///
/// The id sits just below `50_000_000`, where the `AutoIncAssetId` sequence started
/// (<https://github.com/polkadot-fellows/runtimes/pull/414>). Ids below `NextAssetId` can no longer
/// be claimed by a permissionless `create`, only assigned by `ForceOrigin`, so the asset can never
/// collide with a user-registered trust-backed asset, and creating it leaves the sequence untouched
/// (see `migrations::ForceCreatePgasAsset`).
pub const PGAS_ASSET_ID: AssetIdForTrustBackedAssets = 49_999_999;

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
	pub DefaultNetworkSuffix: indiv_support::context::ProductContextNetworkSuffix =
		INDIVIDUALITY_NETWORK_SUFFIX.to_vec().try_into().expect("default network suffix fits");
}

impl indiv_pallet_network_suffix::Config for Runtime {
	type UpdateOrigin = EnsureRoot<Self::AccountId>;
	type DefaultSuffix = DefaultNetworkSuffix;
	type WeightInfo = weights::indiv_pallet_network_suffix::WeightInfo<Runtime>;
}

/// Origin check restricted to the sibling parachain that publishes the ring roots.
pub struct EnsureNotifierSibling;
impl EnsureOrigin<RuntimeOrigin> for EnsureNotifierSibling {
	type Success = ();

	fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
		match o.clone().into() {
			Ok(cumulus_pallet_xcm::Origin::SiblingParachain(id)) if u32::from(id) == PEOPLE_ID =>
				Ok(()),
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

pub type MaxStaleAliasBatch = BenchmarkMax<
	AtMost<AtLeastOne<dynamic_params::individuality::MaxStaleAliasBatch>, ConstU32<32>>,
	ConstU32<32>,
>;

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
	type MaxStaleAliasBatch = MaxStaleAliasBatch;
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
	type MaxPgasClaimsPerBatch = dynamic_params::individuality::MaxPgasClaimsPerBatch;
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

/// Bridges `pallet-dotns-gateway`'s `AddressMapper` trait to `pallet_revive`'s.
pub struct ReviveAddressMapper;
impl indiv_pallet_dotns_gateway::AddressMapper<AccountId> for ReviveAddressMapper {
	fn to_address(account_id: &AccountId) -> sp_core::H160 {
		<pallet_revive::AccountId32Mapper<Runtime> as pallet_revive::AddressMapper<Runtime>>::to_address(
			account_id,
		)
	}
}

/// Adapter exposing `pallet_revive::Pallet::bare_call` to `pallet-dotns-gateway`, which drives the
/// dotNS registry contract.
pub struct ReviveContractCaller;
impl indiv_pallet_dotns_gateway::ContractCaller for ReviveContractCaller {
	fn call(
		dest: sp_core::H160,
		data: Vec<u8>,
		value: u128,
	) -> Result<(Vec<u8>, Weight), indiv_pallet_dotns_gateway::ContractCallError> {
		use pallet_revive::{ExecConfig, TransactionLimits};
		let cr = pallet_revive::Pallet::<Runtime>::bare_call(
			RuntimeOrigin::root(),
			dest,
			value.into(),
			TransactionLimits::WeightAndDeposit {
				weight_limit: dynamic_params::individuality::DotnsMaxContractCallWeight::get(),
				// The root origin does not pay the deposit cost; per-call storage growth is bounded
				// by `weight_limit.proof_size`.
				deposit_limit: u128::MAX,
			},
			data,
			&ExecConfig::new_substrate_tx(),
		);
		match cr.result {
			Ok(ret) if !ret.did_revert() => Ok((ret.data, cr.weight_consumed)),
			Ok(ret) => Err(indiv_pallet_dotns_gateway::ContractCallError {
				dispatch: sp_runtime::DispatchError::Other("contract reverted"),
				revert_data: Some(ret.data),
			}),
			Err(e) => Err(indiv_pallet_dotns_gateway::ContractCallError::from(e)),
		}
	}
}

impl indiv_pallet_dotns_gateway::Config for Runtime {
	type WeightInfo = weights::indiv_pallet_dotns_gateway::WeightInfo<Runtime>;
	type Suffix = NetworkSuffix;
	type MemberService = MembersSubscriber;
	type ContractCaller = ReviveContractCaller;
	type AddressMapper = ReviveAddressMapper;
	type MaxContractCallWeight = dynamic_params::individuality::DotnsMaxContractCallWeight;
	type MaxValiditySeconds = dynamic_params::individuality::DotnsMaxValiditySeconds;
	type MaxFutureSkewSeconds = dynamic_params::individuality::DotnsMaxFutureSkewSeconds;
	type UnixTime = Timestamp;
	type AttestationAllowanceManager = RootOrWhitelist;
	type DispatcherAddressManager = RootOrWhitelist;
	type AttestationSignature = Signature;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = benchmark_utils::DotnsGatewayBenchHelper;
}

parameter_types! {
	pub const ScarcityDepositBase: Balance = system_para_deposit(1, 0);
	pub const ScarcityDepositPerByte: Balance = system_para_deposit(0, 1);
	pub const ScarcityHoldReason: RuntimeHoldReason =
		RuntimeHoldReason::Scarcity(indiv_pallet_scarcity::HoldReason::StorageDeposit);
}

/// Storage price shared by every Scarcity deposit converter: a per-record base plus a per-byte
/// price over the footprint's logical encoded size.
pub type ScarcityStoragePrice =
	LinearStoragePrice<ScarcityDepositBase, ScarcityDepositPerByte, Balance>;

impl indiv_pallet_scarcity::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::indiv_pallet_scarcity::WeightInfo<Runtime>;
	type UnixTime = Timestamp;
	type Balance = Balance;
	// The pallet aggregates exact deposit sums per collection; the consideration ticket receives
	// that sum directly, hence the `Identity` conversion over `Balance`.
	type Consideration = HoldConsideration<
		AccountId,
		Balances,
		ScarcityHoldReason,
		sp_runtime::traits::Identity,
		Balance,
	>;
	type CollectionDeposit = ScarcityStoragePrice;
	type ItemDeposit = ScarcityStoragePrice;
	type InstanceDeposit = ScarcityStoragePrice;
	type MetadataDeposit = ScarcityStoragePrice;
	type MaxKeyLen = ConstU32<32>;
	type MaxValueLen = ConstU32<256>;
	type MaxCollectionMetadata = ConstU32<100>;
	type MaxItemMetadata = ConstU32<100>;
	type MaxInstanceMetadata = ConstU32<100>;
	// Purse keys follow Coinage's retry model, so a failing purse key is paced like a failing coin:
	// one hour, matching `CoinFailureLockPeriod` on People Polkadot.
	type LockPeriod = ConstU64<3600>;
	type MaxTransferPriority = ConstU64<1_000_000>;
	// The feeless moves one mint buys before a move is paid for, matching `MaximumAge` of People
	// Polkadot's Coinage, the age a coin may reach before it must be recycled.
	type MaximumMoves = ConstU16<16>;
	// Clears a collection's nft-claims minter registration when the collection is deleted, so no
	// registration outlives the collection it names.
	type OnCollectionDeleted = indiv_pallet_nft_claims::ClearCollectionMinter<Runtime>;
	// Clears the registration on an ownership handover too, so a round trip back to the
	// registering owner cannot reactivate it.
	type OnCollectionOwnerChanged = indiv_pallet_nft_claims::ClearCollectionMinter<Runtime>;
	// Purse keys are not mapped to contract addresses: no ERC-721 view of Scarcity exists here.
	type OnPurseOccupied = ();
	// Metadata stays opaque bytes: no interface on this chain types any key.
	type MetadataPolicy = ();
}

/// Resolves the origin of an NFT claim to the identity the credit's leaf binds.
///
/// The origin stays signed in both cases, since no transaction extension promotes it to an alias
/// origin. Under [`ClaimantKind::Person`] the alias comes from the signer's `AccountToAlias`
/// binding, in whichever collection and context it was registered. An alias from another context
/// is a different value, so it rehashes to a leaf that no tree holds.
///
/// The claim does not apply the alias-accounts grace policy to the binding's ring revision, unlike
/// a `personhood_info` lookup. The game chain awards the credit to the alias before the claim, so a
/// revision that goes stale after the award still resolves to its person. A stale binding stays
/// claimable until someone calls `clean_up_stale_alias`, which deletes the `AccountToAlias` entry.
/// The claim then fails until the person binds the alias again with a proof against a live
/// revision.
pub struct EnsureCreditClaimant;
impl
	frame_support::traits::EnsureOriginWithArg<RuntimeOrigin, indiv_pallet_nft_claims::ClaimantKind>
	for EnsureCreditClaimant
{
	type Success = indiv_support::identity::AccountOrPerson<AccountId>;

	fn try_origin(
		o: RuntimeOrigin,
		kind: &indiv_pallet_nft_claims::ClaimantKind,
	) -> Result<Self::Success, RuntimeOrigin> {
		let Ok(frame_system::RawOrigin::Signed(who)) = o.clone().into() else {
			return Err(o);
		};

		// Neither kind replaces the signed origin, so `ChargePGAS` bills the claimant.
		match kind {
			indiv_pallet_nft_claims::ClaimantKind::Account =>
				Ok(indiv_support::identity::AccountOrPerson::Account(who)),

			// The direct read skips the grace policy, so a stale binding resolves.
			indiv_pallet_nft_claims::ClaimantKind::Person =>
				match indiv_pallet_alias_accounts::AccountToAlias::<Runtime>::get(&who) {
					Some(info) =>
						Ok(indiv_support::identity::AccountOrPerson::Person(info.ca.alias)),
					None => Err(o),
				},
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin(
		kind: &indiv_pallet_nft_claims::ClaimantKind,
	) -> Result<RuntimeOrigin, ()> {
		let who = AccountId::new([1u8; 32]);
		if matches!(kind, indiv_pallet_nft_claims::ClaimantKind::Person) {
			indiv_pallet_alias_accounts::AccountToAlias::<Runtime>::insert(
				&who,
				indiv_pallet_alias_accounts::AliasAccountInfo {
					collection: *indiv_pallet_alias_accounts::PEOPLE_IDENTIFIER,
					revision: 0,
					ring: 0,
					ca: indiv_support::traits::ContextualAlias {
						context: [0u8; 32],
						alias: [1u8; 32],
					},
				},
			);
		}

		Ok(frame_system::RawOrigin::Signed(who).into())
	}
}

parameter_types! {
	/// Metered ceiling for one collection minter contract call. A claim into a
	/// contract-registered collection reserves this plus revive's dispatch base, refunded to what
	/// the call really consumed. Any other claim reserves nothing.
	///
	/// Deliberately far below the DotNS contract budget: a minter only picks an item index. The
	/// nft-claims `integrity_test` holds the claim worst case plus this ceiling to the block
	/// budget.
	pub const NftClaimsSelectorWeightLimit: Weight =
		Weight::from_parts(5_000_000_000, 512 * 1024);
	/// Maximum storage deposit a collection owner may pay for one minter call.
	///
	/// One PGAS is enough for modest per-claim accounting while bounding the owner's exposure to
	/// a contract they registered.
	pub const NftClaimsSelectorDepositLimit: Balance = UNITS;
}

/// Executes a collection's registered minter contract for `indiv-pallet-nft-claims`.
///
/// The contract is called as the current collection owner, who pays its storage deposit up to
/// [`NftClaimsSelectorDepositLimit`]. `PGasDeposit` takes that deposit in PGAS where the owner
/// holds it and in the native token otherwise, so a claim fails once the owner can pay neither.
/// The return must be one canonical ABI `uint32` naming the item to mint.
pub struct NftClaimsCollectionSelector;
impl NftClaimsCollectionSelector {
	/// Calls a minter contract as `owner` under the selector's weight and deposit ceilings.
	/// ABI construction and return validation remain the responsibility of the selector.
	pub fn call(
		owner: AccountId,
		contract: sp_core::H160,
		data: Vec<u8>,
	) -> pallet_revive::ContractResult<pallet_revive::ExecReturnValue, Balance> {
		use pallet_revive::{ExecConfig, TransactionLimits};

		pallet_revive::Pallet::<Runtime>::bare_call(
			RuntimeOrigin::signed(owner),
			contract,
			0u128.into(),
			TransactionLimits::WeightAndDeposit {
				weight_limit: NftClaimsSelectorWeightLimit::get(),
				deposit_limit: NftClaimsSelectorDepositLimit::get(),
			},
			data,
			&ExecConfig::new_substrate_tx(),
		)
	}
}

impl indiv_pallet_nft_claims::CollectionSelector<AccountId> for NftClaimsCollectionSelector {
	fn max_weight(collection: indiv_pallet_scarcity::CollectionId) -> Weight {
		// Only a contract-registered collection reserves the minter ceiling. The claim's weight
		// function makes this read, where it is not charged.
		match indiv_pallet_nft_claims::CollectionMinters::<Runtime>::get(collection) {
			Some(minter)
				if matches!(
					minter.selection,
					indiv_pallet_nft_claims::ItemSelection::Contract(_)
				) =>
				Self::contract_max_weight(),
			_ => Weight::zero(),
		}
	}

	fn contract_max_weight() -> Weight {
		NftClaimsSelectorWeightLimit::get().saturating_add(revive_call_overhead())
	}

	fn validate(contract: sp_core::H160) -> sp_runtime::DispatchResult {
		frame_support::ensure!(
			pallet_revive::AccountInfo::<Runtime>::is_contract(&contract),
			indiv_pallet_nft_claims::Error::<Runtime>::MinterNotAContract
		);
		Ok(())
	}

	fn select(
		owner: AccountId,
		contract: sp_core::H160,
		collection: indiv_pallet_scarcity::CollectionId,
		credit: indiv_support::credit_trees::NftClaimCredit,
	) -> Result<indiv_pallet_nft_claims::Selection, indiv_pallet_nft_claims::SelectionError> {
		let cr = Self::call(owner, contract, minter_call_data(collection, credit));
		// A trap, a revert and a malformed return all consumed metered weight, which the claim
		// charges: refunding it would let a gas-burning contract occupy block space for free.
		// Every path adds the dispatch base, which `bare_call` spends outside
		// `weight_consumed`.
		let weight_consumed = cr.weight_consumed.saturating_add(revive_call_overhead());
		let fail = |error: sp_runtime::DispatchError| indiv_pallet_nft_claims::SelectionError {
			error,
			weight_consumed,
		};
		let ret = cr.result.map_err(fail)?;
		if ret.did_revert() {
			log::debug!(
				target: "runtime::nft-claims",
				"minter contract {contract:?} reverted with 0x{}",
				sp_core::hexdisplay::HexDisplay::from(&ret.data)
			);
			return Err(fail(
				indiv_pallet_nft_claims::Error::<Runtime>::MinterContractReverted.into(),
			));
		}
		let item = decode_minter_item(&ret.data).ok_or_else(|| {
			log::debug!(
				target: "runtime::nft-claims",
				"minter contract {contract:?} returned no canonical uint32 item: 0x{}",
				sp_core::hexdisplay::HexDisplay::from(&ret.data)
			);
			fail(indiv_pallet_nft_claims::Error::<Runtime>::MinterContractInvalidReturn.into())
		})?;
		Ok(indiv_pallet_nft_claims::Selection { item, weight_consumed })
	}
}

/// Weight of revive's dispatch base for one contract call, spent on top of the metered weight
/// `bare_call` reports.
fn revive_call_overhead() -> Weight {
	<<Runtime as pallet_revive::Config>::WeightInfo as pallet_revive::WeightInfo>::call()
}

/// ABI-encode `mint(uint32 collection, bytes32 credit)`.
fn minter_call_data(
	collection: indiv_pallet_scarcity::CollectionId,
	credit: indiv_support::credit_trees::NftClaimCredit,
) -> Vec<u8> {
	let mut data = Vec::with_capacity(68);
	data.extend_from_slice(&sp_io::hashing::keccak_256(b"mint(uint32,bytes32)")[..4]);
	data.extend_from_slice(&[0u8; 28]);
	data.extend_from_slice(&collection.to_be_bytes());
	data.extend_from_slice(&credit);
	data
}

/// ABI-decode one canonical `uint32` word, which is the only return a minter may give.
fn decode_minter_item(data: &[u8]) -> Option<indiv_pallet_scarcity::ItemIndex> {
	if data.len() != 32 || data[..28] != [0u8; 28] {
		return None;
	}
	Some(u32::from_be_bytes(data[28..].try_into().ok()?))
}

parameter_types! {
	/// How long a credit stays claimable, counted from the People-chain block that awarded it.
	///
	/// This is the claim deadline. The sweep removes a tree past it, and nothing can mint that
	/// tree's unclaimed credits again. Three months covers a player who mints a season's worth of
	/// games at once, and keeps Asset Hub from holding a tree per non-empty block for the chain's
	/// lifetime.
	///
	/// People Polkadot keeps a copy of this constant as `ClaimsChainTreeTtl` and derives the TTL
	/// for its own roots from it, so a root outlives the tree built from it.
	pub const CreditTreeTtl: u64 = 90 * 24 * 60 * 60;
}

impl indiv_pallet_nft_claims::Config for Runtime {
	type WeightInfo = weights::indiv_pallet_nft_claims::WeightInfo<Runtime>;
	// The game pallet, which awards the credits, runs on People Polkadot.
	type EnsureGameChainOrigin = EnsureNotifierSibling;
	// At least the game chain's `MaxCreditTreesPerMessage`, otherwise the batches it sends fail
	// to decode here and their trees never arrive.
	type MaxTreesPerMessage = ConstU32<32>;
	type EnsureClaimant = EnsureCreditClaimant;
	type Nfts = Scarcity;
	type CollectionSelector = NftClaimsCollectionSelector;
	// A credit tree holds at most the game chain's `AWARDS_PER_TREE` leaves, 2048, so a proof
	// carries at most 11 sibling hashes. 16 covers 65536 leaves, leaving room for that constant to
	// grow without stranding the tail of a tree.
	type MaxProofNodes = ConstU32<16>;
	// The game chain's `AWARDS_PER_TREE`, which is the most leaves one tree carries.
	// `ClaimedLeaves` holds one bit per leaf, so a tree costs 256 bytes there, and a tree over this
	// bound is refused rather than stored with leaves this chain cannot spend.
	type MaxCreditsPerTree = ConstU32<2048>;
	type UnixTime = Timestamp;
	type TreeTtl = CreditTreeTtl;
	// Above `MaxTreeDeletionsPerMessage`, so one sweep drops no deletion of its own, and with room
	// for the trees fully claimed in the meantime. A deletion is four bytes, so the whole
	// queue costs under a kilobyte of the proof budget.
	type MaxQueuedTreeDeletions = ConstU32<128>;
	// At or below the game chain's `MaxTreeDeletionsPerMessage`. A larger message fails to decode
	// there, and that chain's own TTL then removes the roots its deletions named.
	//
	// One sweep removes this many trees as well, once a block. One People-chain block awards at
	// most one tree, so 64 a block clears expired trees faster than People Polkadot produces them
	// and leaves the rest of each block free for ordinary traffic.
	type MaxTreeDeletionsPerMessage = ConstU32<64>;
	type XcmRouter = xcm_config::XcmRouter;
	// The chain `EnsureGameChainOrigin` authenticates, and where the roots come from.
	type GameChainLocation = system_parachains_constants::polkadot::locations::PeopleLocation;
	// Matches the `NftCredits` index in People Polkadot's `construct_runtime!`.
	type GameChainPalletIndex = ConstU8<57>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = benchmark_utils::NftClaimsBenchmarkHelper;
}

/// The anonymous origins this runtime rate-limits, and the key their allowance is tracked under.
#[derive(
	Clone,
	Encode,
	Decode,
	Debug,
	MaxEncodedLen,
	scale_info::TypeInfo,
	Eq,
	PartialEq,
	DecodeWithMemTracking,
)]
pub enum RestrictedEntity {
	/// A full-person dotNS registration attempt, keyed by the anonymous alias derived from the
	/// ring proof in `indiv_pallet_dotns_gateway::AsDotnsGateway`.
	DotnsPersonRegistration(Alias),
}

impl indiv_pallet_origin_restriction::RestrictedEntity<OriginCaller, Balance> for RestrictedEntity {
	fn allowance(&self) -> indiv_pallet_origin_restriction::Allowance<Balance> {
		match self {
			RestrictedEntity::DotnsPersonRegistration(_) =>
				indiv_pallet_origin_restriction::Allowance {
					max: dynamic_params::individuality::DotnsPersonRegistrationAllowanceMax::get(),
					recovery_per_block:
						dynamic_params::individuality::DotnsPersonRegistrationAllowanceRecovery::get(
						),
				},
		}
	}

	fn restricted_entity(origin_caller: &OriginCaller) -> Option<Self> {
		match origin_caller {
			OriginCaller::DotnsGateway(indiv_pallet_dotns_gateway::Origin::PersonRegistration(
				alias,
			)) => Some(RestrictedEntity::DotnsPersonRegistration(*alias)),
			_ => None,
		}
	}
}

/// Calls an entity with an exhausted allowance may still dispatch once, going into debt.
pub struct OperationAllowedOneTimeExcess;
impl ContainsPair<RestrictedEntity, RuntimeCall> for OperationAllowedOneTimeExcess {
	fn contains(entity: &RestrictedEntity, call: &RuntimeCall) -> bool {
		match entity {
			RestrictedEntity::DotnsPersonRegistration(_) => matches!(
				call,
				RuntimeCall::DotnsGateway(indiv_pallet_dotns_gateway::Call::register_name { .. })
			),
		}
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

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmark_utils {
	use super::*;
	use frame_support::{
		traits::{
			fungibles::{Create, Inspect, Mutate},
			UnixTime,
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
			contexts: &[Context],
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
			let context_slices = contexts.iter().map(|c| &c[..]).collect::<alloc::vec::Vec<_>>();
			let (proof, _aliases) =
				Crypto::create_multi_context(commitment, &secret, &context_slices, message)
					.expect("create proof");
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

	/// sr25519 helpers for the dotNS gateway benchmark, which needs a signature over a message the
	/// runtime cannot otherwise produce.
	mod dotns_keys {
		use super::*;
		use sp_core::{crypto::KeyTypeId, sr25519};
		use sp_runtime::{traits::IdentifyAccount, MultiSignature, MultiSigner};

		const DOTNS_BENCH_KEY_TYPE: KeyTypeId = KeyTypeId(*b"dnbe");
		const DOTNS_BENCH_SEED: &[u8] = b"//DotnsGatewayBench";

		fn ensure_bench_key() -> sr25519::Public {
			if let Some(pk) = sp_io::crypto::sr25519_public_keys(DOTNS_BENCH_KEY_TYPE).first() {
				return *pk;
			}
			sp_io::crypto::sr25519_generate(DOTNS_BENCH_KEY_TYPE, Some(DOTNS_BENCH_SEED.to_vec()))
		}

		pub fn bench_candidate() -> AccountId {
			MultiSigner::Sr25519(ensure_bench_key()).into_account()
		}

		pub fn bench_sign(message: &[u8]) -> Signature {
			let pk = ensure_bench_key();
			let sig = sp_io::crypto::sr25519_sign(DOTNS_BENCH_KEY_TYPE, &pk, message)
				.expect("bench key was just inserted");
			MultiSignature::Sr25519(sig)
		}
	}

	pub struct DotnsGatewayBenchHelper;
	impl indiv_pallet_dotns_gateway::benchmarking::BenchmarkHelper<Runtime>
		for DotnsGatewayBenchHelper
	{
		fn setup_ring_root(identifier: &Identifier, ring_index: RingIndex) -> RevisionIndex {
			let ring_exponent = ring_exponent_for(identifier);
			let real_root = ring_setup(ring_exponent, [42u8; 32]).0;

			// Fill the sliding window to capacity for the worst-case `verify_membership` iteration.
			let now = <Timestamp as UnixTime>::now().as_secs();
			let max_recent =
				<<Runtime as indiv_pallet_members_subscriber::Config>::MaxRecentRootsPerRing as Get<
					u32,
				>>::get();
			let mut roots: BoundedVec<_, _> = BoundedVec::new();
			for i in 0..max_recent {
				roots
					.try_push(indiv_pallet_members_subscriber::types::RingCommitmentRecord::<
						Runtime,
					> {
						root: <Runtime as indiv_pallet_members_subscriber::benchmarking::BenchmarkHelper<Runtime>>::mock_ring_root(i),
						revision: i + 1,
						source_time: now,
						source_sequence: 0,
					})
					.expect("within MaxRecentRootsPerRing bound");
			}
			let last_idx = roots.len() - 1;
			roots[last_idx].root = real_root;
			indiv_pallet_members_subscriber::Pallet::<Runtime>::set_current_ring_roots(
				identifier, ring_index, roots,
			);
			indiv_pallet_members_subscriber::RingCollectionExponents::<Runtime>::insert(
				*identifier,
				ring_exponent,
			);
			max_recent
		}

		fn valid_proof(
			collection: &indiv_pallet_dotns_gateway::Collection,
			message: &[u8],
		) -> indiv_pallet_dotns_gateway::ProofOf<Runtime> {
			let ring_exponent = if collection == &indiv_pallet_dotns_gateway::Collection::PeopleLite
			{
				<Runtime as indiv_pallet_alias_accounts::Config>::PeopleLiteRingExponent::get()
			} else {
				<Runtime as indiv_pallet_alias_accounts::Config>::PeopleRingExponent::get()
			};
			let (_root, member, secret, domain) = ring_setup(ring_exponent, [42u8; 32]);
			let commitment = Crypto::open(domain, &member, core::iter::once(member))
				.expect("benchmark: open for a single-member ring");
			let (proof, _alias) = Crypto::create(
				commitment,
				&secret,
				&indiv_pallet_dotns_gateway::Pallet::<Runtime>::proof_context()[..],
				message,
			)
			.expect("benchmark: create proof");
			proof
		}

		fn candidate() -> AccountId {
			dotns_keys::bench_candidate()
		}

		fn sign(message: &[u8]) -> Signature {
			dotns_keys::bench_sign(message)
		}

		fn set_time(seconds: u64) {
			pallet_timestamp::Now::<Runtime>::put(seconds.saturating_mul(1_000));
		}
	}

	/// What the claims benchmarks cannot set up themselves: the collection and item a claim mints
	/// into, the minter contract, the clock, and the HRMP channel a deletion message goes out
	/// over. On a live chain a collection owner and the relay chain's configuration provide these.
	pub struct NftClaimsBenchmarkHelper;
	impl indiv_pallet_nft_claims::BenchmarkHelper<AccountId> for NftClaimsBenchmarkHelper {
		fn prepare_collection(
			owner: &AccountId,
			collection: indiv_pallet_scarcity::CollectionId,
			item: indiv_pallet_scarcity::ItemIndex,
		) {
			use frame_support::traits::fungible::Mutate;

			let owner = owner.clone();
			let _ =
				Balances::set_balance(&owner, ExistentialDeposit::get().saturating_mul(1_000_000));

			while indiv_pallet_scarcity::NextCollectionId::<Runtime>::get() <= collection {
				Scarcity::do_create_collection(owner.clone()).expect("collection is created; qed");
			}
			while indiv_pallet_scarcity::Collections::<Runtime>::get(collection)
				.expect("the collection was just created; qed")
				.next_item_index <=
				item
			{
				Scarcity::do_define_item(
					owner.clone(),
					collection,
					indiv_pallet_scarcity::Transferability::Transferable,
					Vec::new(),
				)
				.expect("item is defined; qed");
			}
		}

		fn prepare_contract(owner: &AccountId) -> sp_core::H160 {
			use pallet_revive::call_builder::{Contract, VmBinaryModule};

			Contract::<Runtime>::with_caller(owner.clone(), VmBinaryModule::dummy(), Vec::new())
				.expect("benchmark minter contract is deployed; qed")
				.address
		}

		fn set_unix_time(secs: u64) {
			// `pallet_timestamp` holds the clock in milliseconds, and its `set` is an inherent, so
			// this writes the value straight to storage.
			pallet_timestamp::Now::<Runtime>::put(secs.saturating_mul(1_000));
		}

		fn open_game_chain_channel(max_message_size: u32) {
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
			let game_chain = ParaId::from(PEOPLE_ID);
			let mut messaging_state = RelevantMessagingState::<Runtime>::get().unwrap_or(
				cumulus_pallet_parachain_system::relay_state_snapshot::MessagingStateSnapshot {
					dmq_mqc_head: Default::default(),
					relay_dispatch_queue_remaining_capacity: Default::default(),
					ingress_channels: Vec::new(),
					egress_channels: Vec::new(),
				},
			);
			messaging_state.egress_channels.retain(|(id, _)| *id != game_chain);
			messaging_state.egress_channels.push((game_chain, channel));
			messaging_state.egress_channels.sort_by_key(|(id, _)| *id);
			RelevantMessagingState::<Runtime>::put(messaging_state);
		}
	}

	pub struct OriginRestrictionBenchmarkHelper;
	impl indiv_pallet_origin_restriction::BenchmarkHelper<OriginCaller, RuntimeCall>
		for OriginRestrictionBenchmarkHelper
	{
		fn excess_pair() -> (OriginCaller, RuntimeCall) {
			(
				OriginCaller::DotnsGateway(indiv_pallet_dotns_gateway::Origin::PersonRegistration(
					[0u8; 32],
				)),
				RuntimeCall::DotnsGateway(indiv_pallet_dotns_gateway::Call::register_name {
					who: AccountId::from([0u8; 32]),
					label: indiv_pallet_dotns_gateway::BaseLabel::try_from(b"a".to_vec())
						.expect("single byte label fits"),
					link: indiv_pallet_dotns_gateway::Link::None(
						indiv_pallet_dotns_gateway::ChatKey::from([0u8; 65]),
					),
				}),
			)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn minter_abi_is_canonical() {
		let credit = [0x42u8; 32];
		let data = minter_call_data(0x0102_0304, credit);
		assert_eq!(data.len(), 68);
		// The Solidity selector for `mint(uint32,bytes32)`, pinned independently of the
		// keccak call that produces it.
		assert_eq!(&data[..4], &[0xb3, 0x18, 0x24, 0xf2]);
		assert_eq!(&data[4..32], &[0u8; 28]);
		assert_eq!(&data[32..36], &0x0102_0304u32.to_be_bytes());
		assert_eq!(&data[36..], &credit);
	}

	/// The selector reservation follows the collection's registration, and the contract ceiling
	/// carries revive's dispatch base on top of the metered limit.
	#[test]
	fn minter_selection_reservation_follows_the_registration() {
		use indiv_pallet_nft_claims::{
			CollectionMinter, CollectionMinters, CollectionSelector, ItemSelection,
		};

		sp_io::TestExternalities::default().execute_with(|| {
			let collection = 7u32;
			assert_eq!(NftClaimsCollectionSelector::max_weight(collection), Weight::zero());

			CollectionMinters::<Runtime>::insert(
				collection,
				CollectionMinter {
					owner: AccountId::new([1u8; 32]),
					selection: ItemSelection::Random,
				},
			);
			assert_eq!(NftClaimsCollectionSelector::max_weight(collection), Weight::zero());

			CollectionMinters::<Runtime>::insert(
				collection,
				CollectionMinter {
					owner: AccountId::new([1u8; 32]),
					selection: ItemSelection::Contract(sp_core::H160::zero()),
				},
			);
			assert_eq!(
				NftClaimsCollectionSelector::max_weight(collection),
				NftClaimsCollectionSelector::contract_max_weight()
			);
			// Above the metered limit in both dimensions, so a ceiling that drops the dispatch
			// base fails here.
			assert!(NftClaimsSelectorWeightLimit::get()
				.all_lt(NftClaimsCollectionSelector::contract_max_weight()));
		});
	}

	#[test]
	fn minter_return_must_be_one_canonical_u32_word() {
		let mut valid = [0u8; 32];
		valid[28..].copy_from_slice(&42u32.to_be_bytes());
		assert_eq!(decode_minter_item(&valid), Some(42));
		// Too short, too long and non-zero padding are all rejected.
		assert_eq!(decode_minter_item(&valid[..31]), None);
		assert_eq!(decode_minter_item(&[valid, valid].concat()), None);
		valid[0] = 1;
		assert_eq!(decode_minter_item(&valid), None);
	}
}
