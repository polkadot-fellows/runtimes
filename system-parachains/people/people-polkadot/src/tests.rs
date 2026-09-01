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

use crate::{
	assets::hollar::HOLLAR_UNITS,
	xcm_config::{AssetHubLocation, LocationToAccountId, RelayChainLocation},
	Block, DotWeightToFee as WeightToFee, Runtime, RuntimeCall, RuntimeOrigin,
};
use cumulus_primitives_core::relay_chain::AccountId;
use sp_core::crypto::Ss58Codec;
use xcm::prelude::*;
use xcm_runtime_apis::conversions::LocationToAccountHelper;

use frame_support::{assert_err, assert_noop, assert_ok};
use parachains_runtimes_test_utils::GovernanceOrigin;
use sp_runtime::{Either, MultiAddress};

const ALICE: [u8; 32] = [1u8; 32];

#[test]
fn location_conversion_works() {
	let alice_32 = AccountId32 { network: None, id: AccountId::from(ALICE).into() };
	let bob_20 = AccountKey20 { network: None, key: [123u8; 20] };

	// the purpose of hardcoded values is to catch an unintended location conversion logic change.
	struct TestCase {
		description: &'static str,
		location: Location,
		expected_account_id_str: &'static str,
	}

	let test_cases = vec![
		// DescribeTerminus
		TestCase {
			description: "DescribeTerminus Parent",
			location: Location::new(1, Here),
			expected_account_id_str: "5Dt6dpkWPwLaH4BBCKJwjiWrFVAGyYk3tLUabvyn4v7KtESG",
		},
		TestCase {
			description: "DescribeTerminus Sibling",
			location: Location::new(1, [Parachain(1111)]),
			expected_account_id_str: "5Eg2fnssmmJnF3z1iZ1NouAuzciDaaDQH7qURAy3w15jULDk",
		},
		// DescribePalletTerminal
		TestCase {
			description: "DescribePalletTerminal Parent",
			location: Location::new(1, [PalletInstance(50)]),
			expected_account_id_str: "5CnwemvaAXkWFVwibiCvf2EjqwiqBi29S5cLLydZLEaEw6jZ",
		},
		TestCase {
			description: "DescribePalletTerminal Sibling",
			location: Location::new(1, [Parachain(1111), PalletInstance(50)]),
			expected_account_id_str: "5GFBgPjpEQPdaxEnFirUoa51u5erVx84twYxJVuBRAT2UP2g",
		},
		// DescribeAccountId32Terminal
		TestCase {
			description: "DescribeAccountId32Terminal Parent",
			location: Location::new(1, [alice_32]),
			expected_account_id_str: "5DN5SGsuUG7PAqFL47J9meViwdnk9AdeSWKFkcHC45hEzVz4",
		},
		TestCase {
			description: "DescribeAccountId32Terminal Sibling",
			location: Location::new(1, [Parachain(1111), alice_32]),
			expected_account_id_str: "5DGRXLYwWGce7wvm14vX1Ms4Vf118FSWQbJkyQigY2pfm6bg",
		},
		// DescribeAccountKey20Terminal
		TestCase {
			description: "DescribeAccountKey20Terminal Parent",
			location: Location::new(1, [bob_20]),
			expected_account_id_str: "5CJeW9bdeos6EmaEofTUiNrvyVobMBfWbdQvhTe6UciGjH2n",
		},
		TestCase {
			description: "DescribeAccountKey20Terminal Sibling",
			location: Location::new(1, [Parachain(1111), bob_20]),
			expected_account_id_str: "5CE6V5AKH8H4rg2aq5KMbvaVUDMumHKVPPQEEDMHPy3GmJQp",
		},
		// DescribeTreasuryVoiceTerminal
		TestCase {
			description: "DescribeTreasuryVoiceTerminal Parent",
			location: Location::new(1, [Plurality { id: BodyId::Treasury, part: BodyPart::Voice }]),
			expected_account_id_str: "5CUjnE2vgcUCuhxPwFoQ5r7p1DkhujgvMNDHaF2bLqRp4D5F",
		},
		TestCase {
			description: "DescribeTreasuryVoiceTerminal Sibling",
			location: Location::new(
				1,
				[Parachain(1111), Plurality { id: BodyId::Treasury, part: BodyPart::Voice }],
			),
			expected_account_id_str: "5G6TDwaVgbWmhqRUKjBhRRnH4ry9L9cjRymUEmiRsLbSE4gB",
		},
		// DescribeBodyTerminal
		TestCase {
			description: "DescribeBodyTerminal Parent",
			location: Location::new(1, [Plurality { id: BodyId::Unit, part: BodyPart::Voice }]),
			expected_account_id_str: "5EBRMTBkDisEXsaN283SRbzx9Xf2PXwUxxFCJohSGo4jYe6B",
		},
		TestCase {
			description: "DescribeBodyTerminal Sibling",
			location: Location::new(
				1,
				[Parachain(1111), Plurality { id: BodyId::Unit, part: BodyPart::Voice }],
			),
			expected_account_id_str: "5DBoExvojy8tYnHgLL97phNH975CyT45PWTZEeGoBZfAyRMH",
		},
	];

	for tc in test_cases {
		let expected =
			AccountId::from_string(tc.expected_account_id_str).expect("Invalid AccountId string");

		let got = LocationToAccountHelper::<AccountId, LocationToAccountId>::convert_location(
			tc.location.into(),
		)
		.unwrap();

		assert_eq!(got, expected, "{}", tc.description);
	}
}

#[test]
fn xcm_payment_api_works() {
	parachains_runtimes_test_utils::test_cases::xcm_payment_api_with_native_token_works::<
		Runtime,
		RuntimeCall,
		RuntimeOrigin,
		Block,
		WeightToFee<Runtime>,
	>();
}

/// XCM execution fees can be paid in any asset governance registered a rate for, at that rate.
/// HOLLAR is the first such asset, but nothing here is specific to it.
#[test]
fn xcm_execution_fees_can_be_paid_in_any_asset_with_a_registered_rate() {
	use crate::{
		xcm_config::{RelayLocation, XcmConfig},
		AssetRate, Assets as AssetsPallet, PolkadotXcm,
	};
	use frame_support::weights::WeightToFee as WeightToFeeT;
	use parachains_runtimes_test_utils::ExtBuilder;
	use sp_runtime::FixedU128;
	use xcm_runtime_apis::fees::runtime_decl_for_xcm_payment_api::XcmPaymentApi;

	// An asset worth 4 DOT apiece, and one governance never registered a rate for.
	let rated = Location::new(1, [Parachain(2034), GeneralIndex(222)]);
	let unrated = Location::new(1, [Parachain(2034), GeneralIndex(333)]);
	let rate = FixedU128::from_u32(4);

	ExtBuilder::<Runtime>::default().build().execute_with(|| {
		assert_ok!(AssetsPallet::force_create(
			RuntimeOrigin::root(),
			rated.clone(),
			AccountId::from(ALICE).into(),
			true,
			1,
		));
		assert_ok!(AssetRate::create(RuntimeOrigin::root(), Box::new(rated.clone()), rate));

		// Execution fees are priced in DOT, then converted at the registered rate.
		let weight = Weight::from_parts(1_000_000_000, 10_000);
		let native_fee = WeightToFee::<Runtime>::weight_to_fee(&weight);
		type Trader = <XcmConfig as xcm_executor::Config>::Trader;
		assert_eq!(
			PolkadotXcm::query_weight_to_asset_fee::<Trader>(weight, AssetId(rated.clone()).into())
				.unwrap(),
			native_fee / 4,
		);
		// An asset without a rate buys no execution.
		assert!(PolkadotXcm::query_weight_to_asset_fee::<Trader>(
			weight,
			AssetId(unrated.clone()).into()
		)
		.is_err());

		// And the runtime API advertises DOT and every rated asset, nothing else.
		let acceptable = Runtime::query_acceptable_payment_assets(XCM_VERSION).unwrap();
		assert_eq!(
			acceptable,
			vec![
				VersionedAssetId::from(AssetId(RelayLocation::get())),
				VersionedAssetId::from(AssetId(rated)),
			],
		);
	});
}

#[test]
fn governance_authorize_upgrade_works() {
	use polkadot_runtime_constants::system_parachain::COLLECTIVES_ID;

	// no - random non-system para
	assert_err!(
		parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::Location(Location::new(1, Parachain(12334)))),
		Either::Right(InstructionError { index: 0, error: XcmError::Barrier })
	);
	// no - random system para
	assert_err!(
		parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::Location(Location::new(1, Parachain(1765)))),
		Either::Right(InstructionError { index: 0, error: XcmError::Barrier })
	);

	// no - Collectives
	assert_err!(
		parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::Location(Location::new(1, Parachain(COLLECTIVES_ID)))),
		Either::Right(InstructionError { index: 0, error: XcmError::Barrier })
	);
	// no - Collectives Voice of Fellows plurality
	assert_err!(
		parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::LocationAndDescendOrigin(
			Location::new(1, Parachain(COLLECTIVES_ID)),
			Plurality { id: BodyId::Technical, part: BodyPart::Voice }.into()
		)),
		Either::Right(InstructionError { index: 2, error: XcmError::BadOrigin })
	);

	// ok - relaychain
	assert_ok!(parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
		Runtime,
		RuntimeOrigin,
	>(GovernanceOrigin::Location(RelayChainLocation::get())));

	// ok - AssetHub
	assert_ok!(parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
		Runtime,
		RuntimeOrigin,
	>(GovernanceOrigin::Location(AssetHubLocation::get())));
}

/// The transaction extension pipeline is versioned: version 0 is the pipeline that predates the
/// Individuality deployment and must stay frozen so already-built signers keep working, while
/// version 1 carries the Individuality origin modifiers.
///
/// This pins both: the identifiers of version 0 in order, and the fact that version 1 exists and is
/// version 0 plus the Individuality extensions. Any reordering of version 0 breaks live signers, so
/// it should only ever change together with `transaction_version`.
#[test]
fn transaction_extension_versions_are_stable() {
	use sp_runtime::traits::{Pipeline, PipelineMetadataBuilder, TransactionExtension};

	let v0: Vec<&str> = <crate::TxExtensionV0 as TransactionExtension<RuntimeCall>>::metadata()
		.into_iter()
		.map(|m| m.identifier)
		.collect();
	assert_eq!(
		v0,
		vec![
			"AuthorizeCall",
			"CheckNonZeroSender",
			"CheckSpecVersion",
			"CheckTxVersion",
			"CheckGenesis",
			"CheckMortality",
			"CheckNonce",
			"CheckWeight",
			"ChargeAssetTxPayment",
			"CheckMetadataHash",
			"StorageWeightReclaim",
		],
	);

	// Version 1 must be advertised in the metadata, otherwise no wallet can construct it.
	let mut builder = PipelineMetadataBuilder::new();
	<crate::TxExtensionOtherVersions as Pipeline<RuntimeCall>>::build_metadata(&mut builder);
	let v1_indices = builder.by_version.get(&1).expect("extension version 1 must be advertised");
	let v1: Vec<&str> =
		v1_indices.iter().map(|i| builder.in_versions[*i as usize].identifier).collect();
	assert_eq!(builder.by_version.len(), 1, "only version 1 lives outside version 0");

	// Version 1 is version 0 plus the Individuality pipeline: same non-Individuality identifiers,
	// in the same relative order.
	let indiv = [
		"UnitTransactionExtension",
		"VerifyMultiSignature",
		"AsPerson",
		"PeopleLiteAuth",
		"AsMember",
		"AsCoinage",
		"AsResources",
		"RestrictOrigins",
	];
	let v1_without_indiv: Vec<&str> = v1.iter().copied().filter(|id| !indiv.contains(id)).collect();
	assert_eq!(v1_without_indiv, v0, "version 1 must extend version 0, not reshuffle it");
	for id in indiv {
		assert!(v1.contains(&id), "version 1 must carry `{id}`");
	}
}

#[test]
fn individuality_cross_runtime_pallet_indices_are_pinned() {
	use crate::{individuality::ASSET_HUB_MEMBERS_SUBSCRIBER_INDEX, MembersNotifier};
	use asset_hub_polkadot_runtime::individuality::RingRootsNotifierEndpoint;
	use frame_support::traits::PalletInfoAccess;

	assert_eq!(MembersNotifier::index(), 69);
	assert_eq!(RingRootsNotifierEndpoint::get().pallet_index, MembersNotifier::index() as u8,);
	assert_eq!(asset_hub_polkadot_runtime::MembersSubscriber::index(), 97);
	assert_eq!(
		ASSET_HUB_MEMBERS_SUBSCRIBER_INDEX,
		asset_hub_polkadot_runtime::MembersSubscriber::index() as u8,
	);
}

#[test]
fn asset_hub_subscription_whitelist_matches_asset_hub() {
	use crate::individuality::{
		asset_hub_subscription_whitelist, LitePeopleRingExponent, MembersFlexibleRingExponent,
	};
	use cumulus_primitives_core::ParaId;
	use frame_support::traits::PalletInfoAccess;
	use indiv_support::traits::{PEOPLE_IDENTIFIER, PEOPLE_LITE_IDENTIFIER};
	use polkadot_runtime_constants::system_parachain::ASSET_HUB_ID;

	let whitelist = asset_hub_subscription_whitelist();
	assert_eq!(whitelist.len(), 1);
	let entry = &whitelist[0];
	assert_eq!(entry.para_id, ParaId::from(ASSET_HUB_ID));
	assert_eq!(entry.pallet_index, asset_hub_polkadot_runtime::MembersSubscriber::index() as u8);
	assert_eq!(
		entry.collections,
		vec![
			(*PEOPLE_IDENTIFIER, MembersFlexibleRingExponent::get().exponent()),
			(*PEOPLE_LITE_IDENTIFIER, LitePeopleRingExponent::get().exponent()),
		],
	);
	assert!(entry.collections.windows(2).all(|pair| pair[0].0 < pair[1].0));
}

#[test]
fn seed_asset_hub_subscription_whitelist_migration_seeds_the_entry() {
	use crate::{
		individuality::asset_hub_subscription_whitelist,
		migrations::SeedAssetHubSubscriptionWhitelist, Runtime, RuntimeGenesisConfig,
	};
	use cumulus_primitives_core::ParaId;
	use frame_support::traits::OnRuntimeUpgrade;
	use indiv_pallet_members_notifier::{Pallet as MembersNotifierPallet, SubscriptionWhitelist};
	use polkadot_runtime_constants::system_parachain::ASSET_HUB_ID;
	use sp_runtime::BuildStorage;

	let mut ext =
		sp_io::TestExternalities::new(RuntimeGenesisConfig::default().build_storage().unwrap());
	ext.execute_with(|| {
		let para_id = ParaId::from(ASSET_HUB_ID);
		assert!(SubscriptionWhitelist::<Runtime>::get(para_id).is_none());

		SeedAssetHubSubscriptionWhitelist::on_runtime_upgrade();

		let stored = SubscriptionWhitelist::<Runtime>::get(para_id)
			.expect("the migration seeds the Asset Hub whitelist entry");
		let expected = MembersNotifierPallet::<Runtime>::resolve_whitelist_entry(
			&asset_hub_subscription_whitelist()[0],
		)
		.expect("the whitelist entry is well-formed");
		assert_eq!(stored, expected);
	});
}

#[test]
fn individuality_deployment_order_guards_are_enforced() {
	use crate::{
		assets::hollar::HollarLocation, Assets, ChunksManager, Coinage, RuntimeGenesisConfig,
	};
	use indiv_support::traits::RingExponent;
	use sp_runtime::{transaction_validity::TransactionValidityError, BuildStorage};

	let mut ext = sp_io::TestExternalities::new(
		RuntimeGenesisConfig::default().build_storage().expect("runtime genesis builds"),
	);
	ext.execute_with(|| {
		// `add_chunks` is authorized only after its page hash is committed.
		assert!(matches!(
			ChunksManager::authorize_add_chunks(&RingExponent::R2e9, &0, &[]),
			Err(TransactionValidityError::Invalid(
				sp_runtime::transaction_validity::InvalidTransaction::Call
			))
		));

		// Coinage refuses an unregistered backing asset. Once governance funds the pallet account's
		// minimum balance, it can create sufficient instances for that backing asset.
		let stable = HollarLocation::get();
		assert_noop!(
			Coinage::create_sufficient_instance(
				RuntimeOrigin::root(),
				stable.clone(),
				HOLLAR_UNITS / 100,
			),
			indiv_pallet_coinage::Error::<Runtime>::UnknownAsset,
		);
		assert_ok!(Assets::force_create(
			RuntimeOrigin::root(),
			stable.clone(),
			AccountId::from(ALICE).into(),
			true,
			1,
		));
		assert_ok!(Assets::mint(
			RuntimeOrigin::signed(AccountId::from(ALICE)),
			stable.clone(),
			MultiAddress::Id(Coinage::pallet_account()),
			1,
		));
		assert_ok!(Coinage::create_sufficient_instance(
			RuntimeOrigin::root(),
			stable.clone(),
			HOLLAR_UNITS / 100,
		));
		assert_ok!(Coinage::create_sufficient_instance(
			RuntimeOrigin::root(),
			stable.clone(),
			HOLLAR_UNITS / 100,
		));
		let mut instances = Coinage::get_instance_ids(stable);
		instances.sort();
		assert_eq!(instances, vec![0, 1]);
	});
}

#[test]
fn bulletin_destination_is_governable_but_must_remain_a_sibling_parachain() {
	use crate::{
		individuality::BulletinDataStore,
		parameters::{dynamic_params, RuntimeParameters},
		Parameters, RuntimeGenesisConfig,
	};
	use sp_runtime::BuildStorage;

	let mut ext = sp_io::TestExternalities::new(
		RuntimeGenesisConfig::default().build_storage().expect("runtime genesis builds"),
	);
	ext.execute_with(|| {
		assert_ok!(Parameters::set_parameter(
			RuntimeOrigin::root(),
			RuntimeParameters::BulletinStorage(
				dynamic_params::bulletin_storage::Parameters::BulletinChainLocation(
					dynamic_params::bulletin_storage::BulletinChainLocation,
					Some(Location::parent()),
				),
			),
		));
		assert_eq!(
			BulletinDataStore::bulletin_chain_location(),
			Err(sp_runtime::DispatchError::Other(
				"Bulletin destination must be a sibling parachain"
			)),
		);
	});
}

/// Only assets Asset Hub itself issues may be reserve transferred here from Asset Hub.
///
/// Accepting an asset from a chain that is not its real reserve gives it two reserves, and
/// `ReserveAssetDeposited` mints locally — so the impostor reserve can credit this chain with
/// holdings nobody is backing. The cases below are the ones that would actually hurt.
#[test]
fn only_asset_hub_native_assets_are_reserve_accepted_from_asset_hub() {
	use crate::{
		assets::hollar::{HollarLocation, HydrationLocation},
		xcm_config::TrustedReserves,
	};
	use frame_support::traits::ContainsPair;

	let asset_hub = AssetHubLocation::get();
	let hydration = HydrationLocation::get();
	let accepted = |asset: Location, from: &Location| {
		TrustedReserves::contains(&Asset { id: AssetId(asset), fun: Fungible(1) }, from)
	};
	// A trust backed asset (`pallet-assets` instance 50) and a pool token (instance 54) on AH.
	let ah_asset = Location::new(1, [Parachain(1000), PalletInstance(50), GeneralIndex(4242)]);
	let ah_pool_token = Location::new(1, [Parachain(1000), PalletInstance(54), GeneralIndex(7)]);

	// Asset Hub is the reserve for the assets it issues.
	assert!(accepted(ah_asset.clone(), &asset_hub));
	assert!(accepted(ah_pool_token, &asset_hub));

	// DOT must never arrive as a reserve asset: `FungibleTransactor` has no checking account, so
	// depositing it mints into `Balances`. DOT only ever arrives by teleport.
	assert!(!accepted(Location::parent(), &asset_hub));

	// HOLLAR has a reserve already — Hydration — and must not gain a second one.
	assert!(accepted(HollarLocation::get(), &hydration), "Hydration backs HOLLAR");
	assert!(!accepted(HollarLocation::get(), &asset_hub));

	// Neither may Asset Hub vouch for assets it merely custodies.
	assert!(!accepted(
		Location::new(2, [GlobalConsensus(NetworkId::Ethereum { chain_id: 1 })]),
		&asset_hub,
	));
	assert!(!accepted(Location::new(2, [GlobalConsensus(NetworkId::Kusama)]), &asset_hub));
	assert!(!accepted(Location::new(1, [Parachain(2000), GeneralIndex(1)]), &asset_hub));

	// And an Asset Hub asset is only accepted *from* Asset Hub.
	assert!(!accepted(ah_asset.clone(), &hydration));
	assert!(!accepted(ah_asset, &Location::parent()));
}
