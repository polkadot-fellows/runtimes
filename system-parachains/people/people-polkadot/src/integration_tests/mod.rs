// Copyright (C) Parity Technologies and the various Polkadot contributors, see Contributions.md
// for a list of specific contributors.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Integration-style tests for the people-polkadot runtime.
//!
//! Ported from `runtimes/next-people-paseo/src/integration_tests` of the
//! `individuality-community` repository. Differences from the original test suite:
//!
//! * Every transaction is built as an extension-version-1 *general* extrinsic
//!   ([`crate::TxExtensionV1`]), since People Polkadot versions its transaction extension pipeline
//!   (version 0 is the frozen pre-Individuality pipeline). The implication base every
//!   proof/signature commits to is therefore `(1u8, call)` instead of `(0u8, call)`.
//! * The external asset is HOLLAR (18 decimals, reserve-backed from Hydration) instead of the Paseo
//!   test asset (6 decimals, teleported from Asset Hub).
//! * The game, score, airdrop, people-airdrops, proof-of-ink, honour, NFT-credits and mob-rule
//!   pallets are not deployed on People Polkadot, so the original `lite_people_game_flow`,
//!   `score_game_*`, `people_airdrops_flow` tests and the game part of `statement_allowance` have
//!   no equivalent here.
//! * `key_migration_flow` was already disabled upstream (individuality#1127).

use crate::{
	assets::hollar::{HollarLocation, HOLLAR_UNITS},
	individuality::{
		asset_hub_subscription_whitelist, AssetsWithHolder, LitePeopleOnboardingSize,
		PaidUnloadTokenRingExponent, PeopleCollectionOwner, RecyclerRingExponent,
	},
	xcm_config::RelayLocation,
	*,
};
use codec::{Decode, Encode};
use cumulus_pallet_parachain_system::RelaychainDataProvider;
use cumulus_primitives_core::relay_chain::BlockNumber as RelayBlockNumber;
use frame_support::{
	traits::{
		fungible::{Inspect, InspectHold, Mutate},
		Get, Hooks, OffchainWorker, OnIdle,
	},
	weights::{Weight, WeightMeter},
	BoundedVec,
};
use indiv_support::{
	crypto::{BandersnatchSuite, BandersnatchVrfVerifiable as Crypto, GenerateVerifiable},
	traits::{AppendOnlyMembers as _, RevisionIndex, RingIndex},
};
use parachains_common::{AccountId, Balance, BlockNumber};
use sp_core::{sr25519, Pair};
use sp_io::TestExternalities;
use sp_keyring::Sr25519Keyring;
use sp_runtime::{
	generic,
	offchain::{
		testing::{PoolState, TestOffchainExt, TestTransactionPoolExt},
		OffchainDbExt, OffchainWorkerExt, TransactionPoolExt,
	},
	traits::{BlockNumberProvider, ExtensionVariant, PipelineAtVers, TransactionExtension, Zero},
	AccountId32, FixedU128, MultiSignature,
};
use std::{
	cell::{Cell, RefCell},
	sync::Arc,
};
use system_parachains_constants::polkadot::{
	consensus::elastic_scaling::BLOCK_PROCESSING_VELOCITY,
	currency::{CENTS, UNITS},
};

mod coinage_fee_sanity;
mod coinage_infallible_unpaid_load;
mod coinage_non_anonymous_flow;
mod coinage_paid_flow;
mod coinage_people_flow;
mod coinage_token_allowance;
mod external_asset_transfers;
mod lite_people_free_tx;
mod members_notifier_whitelist;
mod migrations;
mod network_suffix;
mod parameters;
mod statement_allowance;
mod transaction_era;
mod tx_payment_external_asset;

type VrfSecret = <Crypto as GenerateVerifiable>::Secret;

/// The external asset the coinage instance is backed with, and that transaction fees can be paid
/// in: HOLLAR.
type ExternalAssetLocation = HollarLocation;

/// The full-featured fungible view over the external asset.
pub(crate) type FungibleExternalAsset =
	frame_support::traits::fungible::ItemOf<AssetsWithHolder, ExternalAssetLocation, AccountId>;

/// The extension version the tests build transactions at: the Individuality pipeline
/// [`crate::TxExtensionV1`].
const TX_EXT_VERSION: u8 = 1;

fn recycler_ring_exponent() -> indiv_support::traits::RingExponent {
	RecyclerRingExponent::get()
}

fn paid_unload_token_ring_exponent() -> indiv_support::traits::RingExponent {
	PaidUnloadTokenRingExponent::get()
}

fn ring_domain_size(
	ring_exponent: indiv_support::traits::RingExponent,
) -> verifiable::ring::RingDomainSize {
	ring_exponent.try_into().expect("ring exponent should map to a ring domain")
}

fn recycler_ring_size() -> <Crypto as GenerateVerifiable>::Config {
	ring_domain_size(recycler_ring_exponent())
}

fn paid_unload_token_ring_size() -> <Crypto as GenerateVerifiable>::Config {
	ring_domain_size(paid_unload_token_ring_exponent())
}

// Tests are executed in their own thread and only use one thread. This setups a global variable
// for each test. If we ever need multi-threaded tests, this will need to be reworked.
thread_local! {
	static TRANSACTION_POOL: RefCell<Arc<parking_lot::RwLock<PoolState>>> =
		RefCell::new(Arc::new(parking_lot::RwLock::new(PoolState {
			transactions: Vec::new(),
		})));
	static UNIQUE_SECRET_COUNTER: Cell<u64> = const { Cell::new(10_000) };
}

fn pair_to_account_id(pair: &sr25519::Pair) -> AccountId32 {
	(*pair.public().as_array_ref()).into()
}

fn create_unique_secret() -> VrfSecret {
	let seed = UNIQUE_SECRET_COUNTER.with(|counter| {
		let value = counter.get();
		counter.set(value.checked_add(1).expect("unique secret counter overflowed"));
		sp_io::hashing::twox_256(&(b"integration_unique_secrets", value).encode())
	});
	Crypto::new_secret(seed)
}

fn register_lite_person_for_integration(
	lite_pair: &sr25519::Pair,
) -> <Crypto as verifiable::GenerateVerifiable>::Secret {
	let attester = Sr25519Keyring::Bob.to_account_id();
	let onboarding_size = LitePeopleOnboardingSize::get();
	indiv_pallet_people_lite::Pallet::<Runtime>::increase_attestation_allowance(
		RuntimeOrigin::root(),
		attester.clone(),
		onboarding_size,
	)
	.expect("root can grant lite-person attestation allowance");

	let filler_pairs =
		[sr25519::Pair::from_seed(&[201u8; 32]), sr25519::Pair::from_seed(&[202u8; 32])];
	let mut target_secret = None;
	let mut target_member = None;
	for pair in core::iter::once(lite_pair)
		.chain(filler_pairs.iter())
		.take(onboarding_size as usize)
	{
		let lite_account = pair_to_account_id(pair);
		let ring_secret = create_unique_secret();
		let ring_member = Crypto::member_from_secret(&ring_secret);

		let msg = lite_account.using_encoded(|account_bytes| {
			ring_member.using_encoded(|ring_bytes| {
				[&indiv_pallet_people_lite::MSG_PREFIX[..], account_bytes, ring_bytes].concat()
			})
		});

		let candidate_signature = MultiSignature::from(pair.sign(&msg));
		let proof = Crypto::sign(&ring_secret, &msg)
			.expect("ring key can sign the lite attestation payload");

		indiv_pallet_people_lite::Pallet::<Runtime>::attest(
			RuntimeOrigin::signed(attester.clone()),
			lite_account.clone(),
			candidate_signature,
			ring_member,
			proof,
			None,
		)
		.expect("lite person attestation should succeed");

		assert!(
			indiv_pallet_people_lite::LitePeople::<Runtime>::contains_key(&lite_account),
			"lite person must be registered for runtime integration tests"
		);

		if lite_account == pair_to_account_id(lite_pair) {
			target_secret = Some(ring_secret);
			target_member = Some(ring_member);
		}
	}

	let identifier = *indiv_pallet_people_lite::LITE_PEOPLE_MEMBER_IDENTIFIER;
	let ring_index = indiv_pallet_members::CurrentRingIndex::<Runtime>::get(identifier);
	let (head, _) = indiv_pallet_members::QueuePageIndices::<Runtime>::get(identifier);
	let first_member = indiv_pallet_members::OnboardingQueue::<Runtime>::get(identifier, head)
		.first()
		.cloned();
	Members::onboard_members_authorized(
		frame_system::RawOrigin::Authorized.into(),
		identifier,
		ring_index,
		head,
		first_member,
		0,
	)
	.expect("lite people onboarding should succeed in runtime integration tests");
	let revision = indiv_pallet_members::Root::<Runtime>::get(identifier, ring_index)
		.map(|root| root.revision)
		.unwrap_or_default();
	let ring_exponent = indiv_pallet_members::Collections::<Runtime>::get(identifier)
		.expect("collection must exist")
		.ring_size;
	let to_include = indiv_pallet_members::Pallet::<Runtime>::should_build_ring(
		&identifier,
		ring_index,
		<Runtime as indiv_pallet_members::Config>::RingBuildingMemberLimit::get(),
	)
	.expect("ring should be ready to build in runtime integration tests");
	Members::build_ring_authorized(
		frame_system::RawOrigin::Authorized.into(),
		identifier,
		ring_index,
		ring_exponent,
		Some(revision),
		to_include,
		0,
	)
	.expect("lite people ring build should succeed in runtime integration tests");
	let ring_member = target_member.expect("target lite member must be recorded");
	assert!(
		indiv_pallet_members::Pallet::<Runtime>::member_status(
			indiv_pallet_people_lite::LITE_PEOPLE_MEMBER_IDENTIFIER,
			&ring_member,
		)
		.and_then(|status| status.ring_index())
		.is_some(),
		"lite person ring member must be included before building notification proofs"
	);

	target_secret.expect("target lite secret must be recorded")
}

fn create_lite_people_collection() {
	use frame_support::traits::OnRuntimeUpgrade;
	indiv_pallet_people_lite::migration::CreateLitePeopleCollection::<Runtime>::on_runtime_upgrade(
	);
}

fn new_test_ext() -> TestExternalities {
	let alice = Sr25519Keyring::Alice.to_account_id();
	let storage = crate::RuntimeGenesisConfig {
		system: frame_system::GenesisConfig::default(),
		balances: pallet_balances::GenesisConfig {
			balances: vec![(alice, 10_000_000_000_000)],
			dev_accounts: None,
		},
		// Same whitelist the shipped migration seeds, so the tests exercise the live config.
		members_notifier: indiv_pallet_members_notifier::GenesisConfig {
			subscription_whitelist: asset_hub_subscription_whitelist(),
			_phantom: Default::default(),
		},
		..Default::default()
	}
	.build_storage()
	.expect("runtime genesis storage builds");

	let mut ext: TestExternalities = storage.into();
	let (offchain, _state) = TestOffchainExt::new();
	let (pool, state) = TestTransactionPoolExt::new();
	TRANSACTION_POOL.set(state);
	ext.register_extension(OffchainDbExt::new(offchain.clone()));
	ext.register_extension(OffchainWorkerExt::new(offchain));
	ext.register_extension(TransactionPoolExt::new(pool));

	// Initialize chunks and the people collection, which the shipped runtime bootstraps through
	// its migrations plus `ChunksManager::add_chunks` uploads.
	ext.execute_with(|| {
		use indiv_support::traits::{RingExponent, RingMode};

		let page_size =
			<<Runtime as indiv_pallet_chunks_manager::Config>::PageSize as Get<u32>>::get()
				as usize;
		let insert_chunks = |ring_exponent: RingExponent| {
			let chunks = indiv_support::genesis::ring_verifier_builder_params::<BandersnatchSuite>(
				ring_domain_size(ring_exponent),
			);
			for (page_index, page_chunks) in chunks.chunks(page_size).enumerate() {
				let page: BoundedVec<
					_,
					<Runtime as indiv_pallet_chunks_manager::Config>::PageSize,
				> = page_chunks
					.iter()
					.cloned()
					.map(indiv_pallet_chunks_manager::UncheckedChunk)
					.collect::<Vec<_>>()
					.try_into()
					.expect("chunks must fit into page");
				indiv_pallet_chunks_manager::Chunks::<Runtime>::insert(
					ring_exponent,
					page_index as u32,
					page,
				);
			}
		};
		insert_chunks(RingExponent::R2e9);
		for ring_exponent in [recycler_ring_exponent(), paid_unload_token_ring_exponent()] {
			insert_chunks(ring_exponent);
		}

		Members::create_collection(
			PeopleCollectionOwner::get(),
			indiv_pallet_people::PEOPLE_MEMBER_IDENTIFIER,
			1u32,
			RingMode::Flexible,
			RingExponent::R2e9,
			None,
		)
		.expect("create people collection");
		indiv_pallet_people::PeopleCollectionCreated::<Runtime>::put(true);
	});

	ext.execute_with(|| {
		frame_system::Pallet::<Runtime>::set_block_number(1);
		RelaychainDataProvider::<Runtime>::set_block_number(relay_block_for(1));
		pallet_timestamp::Now::<Runtime>::put(1_000u64);
		create_lite_people_collection();
		setup_external_asset();
	});

	ext
}

fn finalize_uxt(call: RuntimeCall, tx_ext: TxExtensionV1) -> UncheckedExtrinsic {
	UncheckedExtrinsic::from_parts(
		call,
		generic::Preamble::General(ExtensionVariant::Other(PipelineAtVers::new(tx_ext))),
	)
}

// Some basic transaction extension to be modified as needed.
//
// It builds the extension-version-1 pipeline: `StorageWeightReclaim` wrapping the inner tuple.
// Field paths used by the helpers below:
// * `tx_ext.0 .0 .1` `VerifySignature`, `.2` `AsPerson`, `.3` `PeopleLiteAuth`, `.4` `AsMember`,
//   `.5` `AsCoinage`, `.6` `AsResources`, `.7` `AuthorizeCall`;
// * `tx_ext.0 .1` `RestrictOrigin` … `tx_ext.0 .7` `CheckNonce` … `tx_ext.0 .9`
//   `ChargeAssetTxPayment`, `tx_ext.0 .10` `CheckMetadataHash`.
fn base_tx_ext(_call: RuntimeCall) -> TxExtensionV1 {
	cumulus_pallet_weight_reclaim::StorageWeightReclaim::new((
		(
			(),
			pallet_verify_signature::VerifySignature::<Runtime>::Disabled,
			indiv_pallet_people::extension::AsPerson::<Runtime>::new(None),
			indiv_pallet_people_lite::extension::PeopleLiteAuth::<Runtime>::new(None),
			indiv_pallet_members::extension::AsMember::<Runtime>::new(None),
			indiv_pallet_coinage::extension::AsCoinage::<Runtime>::new(None),
			indiv_pallet_resources::extension::AsResources::<Runtime>::new(None),
			frame_system::AuthorizeCall::<Runtime>::new(),
		),
		indiv_pallet_origin_restriction::RestrictOrigin::<Runtime>::new(true),
		frame_system::CheckNonZeroSender::<Runtime>::new(),
		frame_system::CheckSpecVersion::<Runtime>::new(),
		frame_system::CheckTxVersion::<Runtime>::new(),
		frame_system::CheckGenesis::<Runtime>::new(),
		frame_system::CheckEra::<Runtime>::from(generic::Era::Immortal),
		frame_system::CheckNonce::<Runtime>::from(0),
		frame_system::CheckWeight::<Runtime>::new(),
		pallet_asset_tx_payment::ChargeAssetTxPayment::<Runtime>::from(0u128, None),
		frame_metadata_hash_extension::CheckMetadataHash::<Runtime>::new(false),
	))
}

fn ring_revision(
	identifier: &indiv_support::traits::Identifier,
	ring_index: RingIndex,
) -> RevisionIndex {
	indiv_pallet_members::Root::<Runtime>::get(*identifier, ring_index)
		.map(|root| root.revision)
		.expect("ring root should exist in runtime integration tests")
}

fn build_as_alias_with_proof_ext(
	who_secret: &VrfSecret,
	context: [u8; 32],
	call: RuntimeCall,
) -> UncheckedExtrinsic {
	let mut tx_ext = base_tx_ext(call.clone());

	let rest_ext = (
		(
			tx_ext.0 .0 .3.clone(),
			tx_ext.0 .0 .4.clone(),
			tx_ext.0 .0 .5.clone(),
			tx_ext.0 .0 .6.clone(),
			tx_ext.0 .0 .7.clone(),
		),
		tx_ext.0 .1.clone(),
		tx_ext.0 .2.clone(),
		tx_ext.0 .3.clone(),
		tx_ext.0 .4.clone(),
		tx_ext.0 .5.clone(),
		tx_ext.0 .6.clone(),
		tx_ext.0 .7.clone(),
		tx_ext.0 .8.clone(),
		tx_ext.0 .9.clone(),
		tx_ext.0 .10.clone(),
	);

	let msg = {
		let implication_base = (TX_EXT_VERSION, &call);
		let implication_explicit = &rest_ext;
		let implication_implicit = &rest_ext.implicit().unwrap();
		let encoded_implications =
			(implication_base, implication_explicit, implication_implicit).encode();
		sp_io::hashing::blake2_256(&encoded_implications)
	};

	let member = Crypto::member_from_secret(who_secret);
	let ring_index = indiv_pallet_members::Pallet::<Runtime>::member_status(
		indiv_pallet_people::PEOPLE_MEMBER_IDENTIFIER,
		&member,
	)
	.unwrap()
	.ring_index()
	.unwrap();
	let members = indiv_pallet_members::RingKeys::<Runtime>::get((
		indiv_pallet_people::PEOPLE_MEMBER_IDENTIFIER,
		ring_index,
		0u32,
	));
	let commitment =
		Crypto::open(verifiable::ring::RingDomainSize::Domain11, &member, members.into_iter())
			.unwrap();
	let proof = Crypto::create(commitment, who_secret, &context[..], &msg[..]).unwrap().0;
	let revision = ring_revision(indiv_pallet_people::PEOPLE_MEMBER_IDENTIFIER, ring_index);

	tx_ext.0 .0 .2 = indiv_pallet_people::extension::AsPerson::new(Some(
		indiv_pallet_people::extension::AsPersonInfo::AsPersonalAliasWithProof(
			proof, ring_index, revision, context,
		),
	));

	finalize_uxt(call, tx_ext)
}

fn build_notification_for_collection_ext(
	who_secret: &VrfSecret,
	period: u32,
	seq: u8,
	call: RuntimeCall,
	identifier: &indiv_support::traits::Identifier,
	collection: indiv_pallet_resources::types::MembershipCollection,
) -> UncheckedExtrinsic {
	let mut tx_ext = base_tx_ext(call.clone());
	let context =
		Resources::notification_context(indiv_pallet_resources::types::NotificationReference {
			period,
			seq,
		});

	let rest_ext = (
		(tx_ext.0 .0 .7.clone(),),
		tx_ext.0 .1.clone(),
		tx_ext.0 .2.clone(),
		tx_ext.0 .3.clone(),
		tx_ext.0 .4.clone(),
		tx_ext.0 .5.clone(),
		tx_ext.0 .6.clone(),
		tx_ext.0 .7.clone(),
		tx_ext.0 .8.clone(),
		tx_ext.0 .9.clone(),
		tx_ext.0 .10.clone(),
	);

	let msg = {
		let implication_base = (TX_EXT_VERSION, &call);
		let implication_explicit = &rest_ext;
		let implication_implicit = &rest_ext.implicit().unwrap();
		let encoded_implications =
			(implication_base, implication_explicit, implication_implicit).encode();
		sp_io::hashing::blake2_256(&encoded_implications)
	};

	let member = Crypto::member_from_secret(who_secret);
	let ring_index = indiv_pallet_members::Pallet::<Runtime>::member_status(identifier, &member)
		.unwrap()
		.ring_index()
		.unwrap();
	let members = indiv_pallet_members::RingKeys::<Runtime>::get((identifier, ring_index, 0u32));
	let commitment =
		Crypto::open(verifiable::ring::RingDomainSize::Domain11, &member, members.into_iter())
			.unwrap();
	let proof = Crypto::create(commitment, who_secret, &context[..], &msg[..]).unwrap().0;
	let revision = ring_revision(identifier, ring_index);

	tx_ext.0 .0 .6 = indiv_pallet_resources::extension::AsResources::new(Some(
		indiv_pallet_resources::extension::AsResourcesInfo::RegisterNotificationForCollection(
			proof, ring_index, revision, collection,
		),
	));

	finalize_uxt(call, tx_ext)
}

fn build_notification_registration_ext(
	who_secret: &VrfSecret,
	period: u32,
	seq: u8,
	call: RuntimeCall,
) -> UncheckedExtrinsic {
	let mut tx_ext = base_tx_ext(call.clone());
	let context =
		Resources::notification_context(indiv_pallet_resources::types::NotificationReference {
			period,
			seq,
		});

	let rest_ext = (
		(tx_ext.0 .0 .7.clone(),),
		tx_ext.0 .1.clone(),
		tx_ext.0 .2.clone(),
		tx_ext.0 .3.clone(),
		tx_ext.0 .4.clone(),
		tx_ext.0 .5.clone(),
		tx_ext.0 .6.clone(),
		tx_ext.0 .7.clone(),
		tx_ext.0 .8.clone(),
		tx_ext.0 .9.clone(),
		tx_ext.0 .10.clone(),
	);

	let msg = {
		let implication_base = (TX_EXT_VERSION, &call);
		let implication_explicit = &rest_ext;
		let implication_implicit = &rest_ext.implicit().unwrap();
		let encoded_implications =
			(implication_base, implication_explicit, implication_implicit).encode();
		sp_io::hashing::blake2_256(&encoded_implications)
	};

	let member = Crypto::member_from_secret(who_secret);
	let ring_index = indiv_pallet_members::Pallet::<Runtime>::member_status(
		indiv_pallet_people::PEOPLE_MEMBER_IDENTIFIER,
		&member,
	)
	.unwrap()
	.ring_index()
	.unwrap();
	let members = indiv_pallet_members::RingKeys::<Runtime>::get((
		indiv_pallet_people::PEOPLE_MEMBER_IDENTIFIER,
		ring_index,
		0u32,
	));
	let commitment =
		Crypto::open(verifiable::ring::RingDomainSize::Domain11, &member, members.into_iter())
			.unwrap();
	let proof = Crypto::create(commitment, who_secret, &context[..], &msg[..]).unwrap().0;
	let revision = ring_revision(indiv_pallet_people::PEOPLE_MEMBER_IDENTIFIER, ring_index);

	tx_ext.0 .0 .6 = indiv_pallet_resources::extension::AsResources::new(Some(
		indiv_pallet_resources::extension::AsResourcesInfo::RegisterNotificationWithProof(
			proof, ring_index, revision,
		),
	));

	finalize_uxt(call, tx_ext)
}

fn build_lite_notification_registration_ext(
	who_secret: &VrfSecret,
	period: u32,
	seq: u8,
	call: RuntimeCall,
) -> UncheckedExtrinsic {
	build_notification_for_collection_ext(
		who_secret,
		period,
		seq,
		call,
		indiv_pallet_people_lite::LITE_PEOPLE_MEMBER_IDENTIFIER,
		indiv_pallet_resources::types::MembershipCollection::LitePeople,
	)
}

fn exec_notification_registration_with_proof(
	who_secret: &VrfSecret,
	period: u32,
	seq: u8,
	call: RuntimeCall,
) {
	let uxt = build_notification_registration_ext(who_secret, period, seq, call);
	Executive::apply_extrinsic(uxt)
		.expect("transaction is valid")
		.expect("dispatch succeeds");
}

fn build_signed_ext(who: &sr25519::Pair, call: RuntimeCall) -> UncheckedExtrinsic {
	let mut tx_ext = base_tx_ext(call.clone());

	let who_account = pair_to_account_id(who);

	// update CheckNonce
	{
		let nonce = frame_system::Pallet::<Runtime>::account_nonce(&who_account);
		tx_ext.0 .7 = frame_system::CheckNonce::<Runtime>::from(nonce);
	}

	// update VerifySignature
	{
		let rest_ext = (
			(
				tx_ext.0 .0 .2.clone(),
				tx_ext.0 .0 .3.clone(),
				tx_ext.0 .0 .4.clone(),
				tx_ext.0 .0 .5.clone(),
				tx_ext.0 .0 .6.clone(),
				tx_ext.0 .0 .7.clone(),
			),
			tx_ext.0 .1.clone(),
			tx_ext.0 .2.clone(),
			tx_ext.0 .3.clone(),
			tx_ext.0 .4.clone(),
			tx_ext.0 .5.clone(),
			tx_ext.0 .6.clone(),
			tx_ext.0 .7.clone(),
			tx_ext.0 .8.clone(),
			tx_ext.0 .9.clone(),
			tx_ext.0 .10.clone(),
		);

		let msg = {
			let implication_base = (TX_EXT_VERSION, &call);
			let implication_explicit = &rest_ext;
			let implication_implicit = &rest_ext.implicit().unwrap();
			let encoded_implications =
				(implication_base, implication_explicit, implication_implicit).encode();
			sp_io::hashing::blake2_256(&encoded_implications)
		};

		// Sign the message with the sr25519 key.
		let raw_sig = who.sign(&msg);

		tx_ext.0 .0 .1 = pallet_verify_signature::VerifySignature::<Runtime>::new_with_signature(
			MultiSignature::from(raw_sig),
			who_account,
		);
	}

	finalize_uxt(call, tx_ext)
}

#[track_caller]
fn exec_signed(who: &sr25519::Pair, call: RuntimeCall) {
	let uxt = build_signed_ext(who, call);
	Executive::apply_extrinsic(uxt)
		.expect("transaction is valid")
		.expect("dispatch succeeds");
}

/// Build a transaction dispatching `call` through the `PeopleLiteAuth` extension with `auth_data`,
/// signed by `who` at its current nonce. Serves every `PeopleLiteAuthData` variant that
/// authenticates with a signed account and a nonce.
fn build_people_lite_auth_ext(
	who: &sr25519::Pair,
	auth_data: impl FnOnce(u32) -> indiv_pallet_people_lite::extension::PeopleLiteAuthDataOf<Runtime>,
	call: RuntimeCall,
) -> UncheckedExtrinsic {
	let mut tx_ext = base_tx_ext(call.clone());
	let who_account = pair_to_account_id(who);
	let nonce = frame_system::Pallet::<Runtime>::account_nonce(&who_account);

	tx_ext.0 .7 = frame_system::CheckNonce::<Runtime>::from(nonce);
	tx_ext.0 .0 .3 =
		indiv_pallet_people_lite::extension::PeopleLiteAuth::<Runtime>::new(Some(auth_data(nonce)));

	// The implication of the `VerifySignature` extension: every extension after it.
	let rest_ext = (
		(
			tx_ext.0 .0 .2.clone(),
			tx_ext.0 .0 .3.clone(),
			tx_ext.0 .0 .4.clone(),
			tx_ext.0 .0 .5.clone(),
			tx_ext.0 .0 .6.clone(),
			tx_ext.0 .0 .7.clone(),
		),
		tx_ext.0 .1.clone(),
		tx_ext.0 .2.clone(),
		tx_ext.0 .3.clone(),
		tx_ext.0 .4.clone(),
		tx_ext.0 .5.clone(),
		tx_ext.0 .6.clone(),
		tx_ext.0 .7.clone(),
		tx_ext.0 .8.clone(),
		tx_ext.0 .9.clone(),
		tx_ext.0 .10.clone(),
	);

	let msg = {
		let implication_base = (TX_EXT_VERSION, &call);
		let implication_explicit = &rest_ext;
		let implication_implicit = &rest_ext.implicit().unwrap();
		let encoded_implications =
			(implication_base, implication_explicit, implication_implicit).encode();
		sp_io::hashing::blake2_256(&encoded_implications)
	};

	tx_ext.0 .0 .1 = pallet_verify_signature::VerifySignature::<Runtime>::new_with_signature(
		MultiSignature::from(who.sign(&msg)),
		who_account,
	);

	finalize_uxt(call, tx_ext)
}

// Helper function to build an extrinsic signed by a shielded coin account (using AsCoin extension)
fn build_as_coin_ext(who_pair: &sr25519::Pair, call: RuntimeCall) -> UncheckedExtrinsic {
	let mut tx_ext = base_tx_ext(call.clone());
	let who_account = pair_to_account_id(who_pair);

	// 1. Set the extension to AsCoin (index 0.0.5)
	tx_ext.0 .0 .5 = indiv_pallet_coinage::extension::AsCoinage::<Runtime>::new(Some(
		indiv_pallet_coinage::extension::AsCoinageInfo::AsCoin,
	));

	// 2. Update Nonce (index 0.7)
	// Coin accounts might not exist in System if they never held native balance.
	let nonce = frame_system::Pallet::<Runtime>::account_nonce(&who_account);
	// base_tx_ext initializes nonce to 0, so we only update if non-zero.
	if !nonce.is_zero() {
		tx_ext.0 .7 = frame_system::CheckNonce::<Runtime>::from(nonce);
	}

	// 3. Calculate the signature payload (rest_ext) and update VerifySignature (index 0.0.1)
	let rest_ext = (
		(
			tx_ext.0 .0 .2.clone(),
			tx_ext.0 .0 .3.clone(),
			tx_ext.0 .0 .4.clone(),
			tx_ext.0 .0 .5.clone(),
			tx_ext.0 .0 .6.clone(),
			tx_ext.0 .0 .7.clone(),
		),
		tx_ext.0 .1.clone(),
		tx_ext.0 .2.clone(),
		tx_ext.0 .3.clone(),
		tx_ext.0 .4.clone(),
		tx_ext.0 .5.clone(),
		tx_ext.0 .6.clone(),
		tx_ext.0 .7.clone(),
		tx_ext.0 .8.clone(),
		tx_ext.0 .9.clone(),
		tx_ext.0 .10.clone(),
	);

	let msg = {
		let implication_base = (TX_EXT_VERSION, &call);
		let implication_explicit = &rest_ext;
		let implication_implicit = &rest_ext.implicit().unwrap();
		let encoded_implications =
			(implication_base, implication_explicit, implication_implicit).encode();
		sp_io::hashing::blake2_256(&encoded_implications)
	};

	// Sign the message with the coin's sr25519 key.
	let raw_sig = who_pair.sign(&msg);

	// Update VerifySignature
	tx_ext.0 .0 .1 = pallet_verify_signature::VerifySignature::<Runtime>::new_with_signature(
		MultiSignature::from(raw_sig),
		who_account,
	);

	finalize_uxt(call, tx_ext)
}

#[track_caller]
fn exec_as_coin(who_pair: &sr25519::Pair, call: RuntimeCall) {
	let uxt = build_as_coin_ext(who_pair, call);
	Executive::apply_extrinsic(uxt)
		.expect("transaction is valid")
		.expect("dispatch succeeds");
}

/// The relay chain block number the parachain sees at parachain block `para_block`.
fn relay_block_for(para_block: BlockNumber) -> RelayBlockNumber {
	para_block / BLOCK_PROCESSING_VELOCITY
}

/// Advance the chain to `target_block`
fn advance_to_block(target_block: frame_system::pallet_prelude::BlockNumberFor<Runtime>) {
	loop {
		let current = frame_system::Pallet::<Runtime>::block_number();
		if current >= target_block {
			break;
		}

		// Execute previous block idle and offchain worker
		AllPalletsWithSystem::on_idle(current, Weight::MAX);
		AllPalletsWithSystem::offchain_worker(current);

		// Advance time by 2 seconds (2000 ms), the block time of this runtime.
		let now_ms: u64 = pallet_timestamp::Now::<Runtime>::get();
		pallet_timestamp::Now::<Runtime>::put(now_ms.saturating_add(2_000));

		// Advance block number by 1
		let next = current.saturating_add(1u32);
		frame_system::Pallet::<Runtime>::initialize(
			&next,
			&Default::default(),
			&Default::default(),
		);

		// Simulate the parachain-system inherent moving the relay chain forward.
		RelaychainDataProvider::<Runtime>::set_block_number(relay_block_for(next));

		// Simulate the parachain-system inherent refreshing the relay randomness, with a
		// value that varies per block like the relay per-block VRF does.
		indiv_pallet_relay_randomness::Randomness::<Runtime>::mutate(|values| {
			values.block = Some(indiv_pallet_relay_randomness::RandomnessEntry {
				randomness: sp_io::hashing::blake2_256(&next.encode()),
				moment: next,
			})
		});

		// Run on_poll for pallets that drive state forward
		let mut wm_people = WeightMeter::with_limit(Weight::MAX);
		indiv_pallet_people::Pallet::<Runtime>::on_poll(next, &mut wm_people);

		// Run transaction from the transaction pool submitted from the offchain worker
		let transactions = {
			TRANSACTION_POOL.with_borrow_mut(|pool| std::mem::take(&mut pool.write().transactions))
		};
		for tx in transactions {
			let tx: UncheckedExtrinsic = Decode::decode(&mut &tx[..]).unwrap();
			Executive::apply_extrinsic(tx)
				.expect("transaction is valid")
				.expect("dispatch succeeds");
		}
	}
}

/// Advance exactly one block.
fn advance_block() {
	let next_block = frame_system::Pallet::<Runtime>::block_number().saturating_add(1u32);
	advance_to_block(next_block);
}

/// Set the current time without executing blocks.
fn set_time(secs: u64) {
	pallet_timestamp::Now::<Runtime>::put(secs * 1000);
}

/// One "coinage dollar", in external-asset base units: the smallest coin denomination of the
/// coinage instance created by [`setup_external_asset`]. Matches the value the deployment-order
/// test in `crate::tests` uses.
pub(crate) const COINAGE_ASSET_UNIT: Balance = HOLLAR_UNITS / 100;

/// The minimum balance the external asset is registered with in these tests.
const EXTERNAL_ASSET_MIN_BALANCE: Balance = 1;

/// The coinage instance created by [`setup_external_asset`].
const COINAGE_INSTANCE_ID: indiv_pallet_coinage::InstanceId = 0;

/// Setup the external asset (HOLLAR) used by many pallets.
fn setup_external_asset() {
	Assets::force_create(
		RuntimeOrigin::root(),
		ExternalAssetLocation::get(),
		pair_to_account_id(&Sr25519Keyring::Alice.pair()).into(),
		true,
		EXTERNAL_ASSET_MIN_BALANCE,
	)
	.expect("create asset should work");

	// Set up the asset rate for native <-> external asset conversion.
	// Native (DOT) has 10 decimals, HOLLAR has 18 decimals; the tests price 1 DOT at 1 HOLLAR, so
	// 1 raw native ($10^-10) = 10^8 raw asset ($10^-18): rate = 10^-8.
	AssetRate::create(
		RuntimeOrigin::root(),
		alloc::boxed::Box::new(ExternalAssetLocation::get()),
		FixedU128::from_rational(UNITS, HOLLAR_UNITS),
	)
	.expect("create asset rate should work");

	setup_fee_conversion_pool();

	// These operations mirror the post-upgrade governance steps that operators will run on live
	// chains: the pallet account is given the asset's minimum balance as a buffer against being
	// dusted, which `create_sufficient_instance` requires of it, and only then is the instance
	// created.
	Assets::mint(
		RuntimeOrigin::signed(pair_to_account_id(&Sr25519Keyring::Alice.pair())),
		ExternalAssetLocation::get(),
		sp_runtime::MultiAddress::Id(Coinage::pallet_account()),
		<Assets as frame_support::traits::fungibles::Inspect<AccountId>>::minimum_balance(
			ExternalAssetLocation::get(),
		),
	)
	.expect("mint the pallet account's buffer should work");

	Coinage::create_sufficient_instance(
		RuntimeOrigin::root(),
		ExternalAssetLocation::get(),
		COINAGE_ASSET_UNIT,
	)
	.expect("create_sufficient_instance should succeed");
}

/// The account holding the native/external-asset pool's reserves, which is where the asset a fee
/// costs ends up once it is converted.
fn fee_conversion_pool_account() -> AccountId32 {
	use pallet_asset_conversion::PoolLocator;
	<<Runtime as pallet_asset_conversion::Config>::PoolLocator>::pool_address(
		&RelayLocation::get(),
		&ExternalAssetLocation::get(),
	)
	.expect("the fee conversion pool is seeded")
}

/// The asset amount that pays exactly one unload token fee at the current price, as tests pass it
/// for `max_fee`.
fn unload_token_fee_in_asset() -> Balance {
	Coinage::get_paid_unload_token_fee_in_asset(COINAGE_INSTANCE_ID)
		.expect("the fee conversion pool is seeded")
}

/// The native amount that pays exactly one unload token fee at the current fee multiplier, as
/// tests paying in `FeeCurrency::Native` pass it for `max_fee`.
fn unload_token_fee_in_native() -> Balance {
	Coinage::get_paid_unload_token_fee_in_native()
}

/// Seed the native/external-asset pool that coinage converts through to pay a fee with the asset.
///
/// Mirrors the operational step that has to be done before the runtime starts charging fees this
/// way: without a pool, every asset-denominated fee path is unavailable.
fn setup_fee_conversion_pool() {
	use frame_support::traits::fungible::Mutate as _;

	// The pool holds the same 1:10^8 planck ratio as the asset rate above, deep enough that a
	// fee-sized conversion does not move the price noticeably.
	let native_liquidity: Balance = 10_000 * UNITS;
	let asset_liquidity: Balance = 10_000 * HOLLAR_UNITS;

	let provider = pair_to_account_id(&Sr25519Keyring::Ferdie.pair());
	Balances::mint_into(&provider, native_liquidity.saturating_mul(2))
		.expect("mint native to the liquidity provider should work");
	FungibleExternalAsset::mint_into(&provider, asset_liquidity.saturating_mul(2))
		.expect("mint the asset to the liquidity provider should work");

	let native = RelayLocation::get();
	let asset = ExternalAssetLocation::get();
	AssetConversion::create_pool(
		RuntimeOrigin::signed(provider.clone()),
		alloc::boxed::Box::new(native.clone()),
		alloc::boxed::Box::new(asset.clone()),
	)
	.expect("create pool should work");
	AssetConversion::add_liquidity(
		RuntimeOrigin::signed(provider.clone()),
		alloc::boxed::Box::new(native),
		alloc::boxed::Box::new(asset),
		native_liquidity,
		asset_liquidity,
		1,
		1,
		provider,
	)
	.expect("add liquidity should work");
}
