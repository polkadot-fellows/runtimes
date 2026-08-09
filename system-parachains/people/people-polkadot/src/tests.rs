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
	xcm_config::{AssetHubLocation, LocationToAccountId, RelayChainLocation},
	Block, DotWeightToFee as WeightToFee, Runtime, RuntimeCall, RuntimeOrigin,
};
use cumulus_primitives_core::relay_chain::AccountId;
use sp_core::crypto::Ss58Codec;
use xcm::prelude::*;
use xcm_runtime_apis::conversions::LocationToAccountHelper;

use frame_support::{assert_err, assert_noop, assert_ok};
use parachains_runtimes_test_utils::GovernanceOrigin;
use sp_runtime::Either;

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
		"ScoreAsParticipant",
		"GameAsInvited",
		"PeopleLiteAuth",
		"AsMember",
		"AsCoinage",
		"AsResources",
		"HonourAuth",
		"RestrictOrigins",
	];
	let v1_without_indiv: Vec<&str> = v1.iter().copied().filter(|id| !indiv.contains(id)).collect();
	assert_eq!(v1_without_indiv, v0, "version 1 must extend version 0, not reshuffle it");
	for id in indiv {
		assert!(v1.contains(&id), "version 1 must carry `{id}`");
	}
}

#[test]
fn individuality_storage_parameters_are_governance_mutable() {
	use crate::{
		parameters::{dynamic_params, RuntimeParameters, StatementAllowanceParameter},
		Parameters, RuntimeGenesisConfig,
	};
	use frame_support::traits::Get;
	use polkadot_runtime_constants::{
		fellowship::FELLOWS_RANK,
		system_parachain::{ASSET_HUB_ID, COLLECTIVES_ID},
		xcm::body::TECHNICAL_MAINTENANCE_INDEX,
	};
	use sp_runtime::BuildStorage;

	let mut ext = sp_io::TestExternalities::new(
		RuntimeGenesisConfig::default().build_storage().expect("runtime genesis builds"),
	);
	ext.execute_with(|| {
		assert_eq!(
			<<Runtime as indiv_pallet_resources::Config>::StmtStoreGraceWindow as Get<u32>>::get(),
			2 * 24 * 60 * 60,
		);
		assert_noop!(
			Parameters::set_parameter(
				RuntimeOrigin::signed(ALICE.into()),
				RuntimeParameters::StatementStorage(
					dynamic_params::statement_storage::Parameters::StmtStoreGraceWindow(
						dynamic_params::statement_storage::StmtStoreGraceWindow,
						Some(60 * 60),
					),
				),
			),
			sp_runtime::DispatchError::BadOrigin,
		);
		assert_ok!(Parameters::set_parameter(
			RuntimeOrigin::root(),
			RuntimeParameters::StatementStorage(
				dynamic_params::statement_storage::Parameters::StmtStoreGraceWindow(
					dynamic_params::statement_storage::StmtStoreGraceWindow,
					Some(60 * 60),
				),
			),
		));
		assert_eq!(
			<<Runtime as indiv_pallet_resources::Config>::StmtStoreGraceWindow as Get<u32>>::get(),
			60 * 60,
		);

		assert_ok!(Parameters::set_parameter(
			RuntimeOrigin::root(),
			RuntimeParameters::StatementStorage(
				dynamic_params::statement_storage::Parameters::PersonStatementLimit(
					dynamic_params::statement_storage::PersonStatementLimit,
					Some(StatementAllowanceParameter { max_size: 42, max_count: 3 }),
				),
			),
		));
		assert_eq!(
			<<Runtime as indiv_pallet_resources::Config>::PersonStatementLimit as Get<
				sp_statement_store::StatementAllowance,
			>>::get(),
			sp_statement_store::StatementAllowance { max_size: 42, max_count: 3 },
		);

		assert_ok!(Parameters::set_parameter(
			RuntimeOrigin::from(pallet_xcm::Origin::Xcm(Location::new(
				1,
				[
					Parachain(COLLECTIVES_ID),
					Plurality { id: BodyId::Technical, part: BodyPart::Voice },
					GeneralIndex(FELLOWS_RANK),
				],
			))),
			RuntimeParameters::BulletinStorage(
				dynamic_params::bulletin_storage::Parameters::LongTermStorageGraceWindow(
					dynamic_params::bulletin_storage::LongTermStorageGraceWindow,
					Some(2 * 60 * 60),
				),
			),
		));
		assert_eq!(
			<<Runtime as indiv_pallet_resources::Config>::LongTermStorageGraceWindow as Get<u32>>::get(),
			2 * 60 * 60,
		);

		assert_ok!(Parameters::set_parameter(
			RuntimeOrigin::from(pallet_xcm::Origin::Xcm(Location::new(
				1,
				[
					Parachain(ASSET_HUB_ID),
					Plurality {
						id: BodyId::Index(TECHNICAL_MAINTENANCE_INDEX),
						part: BodyPart::Voice,
					},
				],
			))),
			RuntimeParameters::StatementStorage(
				dynamic_params::statement_storage::Parameters::StmtStoreGraceWindow(
					dynamic_params::statement_storage::StmtStoreGraceWindow,
					Some(30 * 60),
				),
			),
		));
		assert_eq!(
			<<Runtime as indiv_pallet_resources::Config>::StmtStoreGraceWindow as Get<u32>>::get(),
			30 * 60,
		);

		assert_noop!(
			Parameters::set_parameter(
				RuntimeOrigin::from(pallet_xcm::Origin::Xcm(Location::new(
					1,
					[
						Parachain(ASSET_HUB_ID),
						Plurality {
							id: BodyId::Index(TECHNICAL_MAINTENANCE_INDEX + 1),
							part: BodyPart::Voice,
						},
					],
				))),
				RuntimeParameters::StatementStorage(
					dynamic_params::statement_storage::Parameters::StmtStoreGraceWindow(
						dynamic_params::statement_storage::StmtStoreGraceWindow,
						Some(15 * 60),
					),
				),
			),
			sp_runtime::DispatchError::BadOrigin,
		);
	});
}

#[test]
fn individuality_cross_runtime_pallet_indices_are_pinned() {
	use crate::MembersNotifier;
	use asset_hub_polkadot_runtime::individuality::RingRootsNotifierEndpoint;
	use frame_support::traits::PalletInfoAccess;

	assert_eq!(MembersNotifier::index(), 69);
	assert_eq!(RingRootsNotifierEndpoint::get().pallet_index, MembersNotifier::index() as u8,);
	assert_eq!(asset_hub_polkadot_runtime::MembersSubscriber::index(), 97);
}

#[test]
fn individuality_deployment_order_guards_are_enforced() {
	use crate::{
		individuality::StableAssetLocation, Assets, ChunksManager, Coinage, RuntimeGenesisConfig,
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

		// Coinage refuses an unregistered backing asset, then deliberately refuses a second choice.
		let stable = StableAssetLocation::get();
		assert_noop!(
			Coinage::set_underlying_asset_id(RuntimeOrigin::root(), stable.clone()),
			indiv_pallet_coinage::Error::<Runtime>::UnknownAsset,
		);
		assert_ok!(Assets::force_create(
			RuntimeOrigin::root(),
			stable.clone(),
			AccountId::from(ALICE).into(),
			true,
			1,
		));
		assert_ok!(Coinage::set_underlying_asset_id(RuntimeOrigin::root(), stable.clone()));
		assert_noop!(
			Coinage::set_underlying_asset_id(RuntimeOrigin::root(), stable),
			indiv_pallet_coinage::Error::<Runtime>::AssetIdAlreadySet,
		);

		// No schedule means no game state or score round is active.
		assert!(indiv_pallet_game::Game::<Runtime>::get().is_none());
		assert!(indiv_pallet_game::GameSchedules::<Runtime>::get().is_empty());
		assert!(indiv_pallet_score::RoundPlanning::<Runtime>::get().is_none());
		assert!(indiv_pallet_score::Participants::<Runtime>::iter().next().is_none());
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

#[test]
fn individuality_dynamic_parameter_extremes_do_not_brick_parameter_updates() {
	use crate::{
		individuality::BulletinDataStore,
		parameters::{dynamic_params, RuntimeParameters, StatementAllowanceParameter},
		Parameters, RuntimeGenesisConfig,
	};
	use indiv_pallet_resources::types::LongTermStorageAllocation;
	use sp_runtime::BuildStorage;

	macro_rules! statement_parameter {
		($name:ident, $value:expr) => {
			assert_ok!(Parameters::set_parameter(
				RuntimeOrigin::root(),
				RuntimeParameters::StatementStorage(
					dynamic_params::statement_storage::Parameters::$name(
						dynamic_params::statement_storage::$name,
						Some($value),
					),
				),
			));
		};
	}
	macro_rules! bulletin_parameter {
		($name:ident, $value:expr) => {
			assert_ok!(Parameters::set_parameter(
				RuntimeOrigin::root(),
				RuntimeParameters::BulletinStorage(
					dynamic_params::bulletin_storage::Parameters::$name(
						dynamic_params::bulletin_storage::$name,
						Some($value),
					),
				),
			));
		};
	}

	let mut ext = sp_io::TestExternalities::new(
		RuntimeGenesisConfig::default().build_storage().expect("runtime genesis builds"),
	);
	ext.execute_with(|| {
		let zero_allowance = StatementAllowanceParameter { max_size: 0, max_count: 0 };
		let max_allowance = StatementAllowanceParameter { max_size: u32::MAX, max_count: u32::MAX };
		statement_parameter!(AccountsApiAllowance, zero_allowance.clone());
		statement_parameter!(AccountsApiAllowance, max_allowance.clone());
		statement_parameter!(StmtStoreSlotsPerPeriod, 0u32);
		statement_parameter!(StmtStoreSlotsPerPeriod, u32::MAX);
		statement_parameter!(LiteStmtStoreSlotsPerPeriod, 0u32);
		statement_parameter!(LiteStmtStoreSlotsPerPeriod, u32::MAX);
		statement_parameter!(StmtStoreCleanupLimit, 0u32);
		statement_parameter!(StmtStoreCleanupLimit, u32::MAX);
		statement_parameter!(StmtStoreReplacementCooldown, 0u32);
		statement_parameter!(StmtStoreReplacementCooldown, u32::MAX);
		statement_parameter!(StmtStoreGraceWindow, 0u32);
		statement_parameter!(StmtStoreGraceWindow, u32::MAX);
		statement_parameter!(NotificationAllowance, zero_allowance.clone());
		statement_parameter!(NotificationAllowance, max_allowance.clone());
		statement_parameter!(NotificationSlotsPerPeriod, 0u8);
		statement_parameter!(NotificationSlotsPerPeriod, u8::MAX);
		statement_parameter!(LiteNotificationSlotsPerPeriod, 0u8);
		statement_parameter!(LiteNotificationSlotsPerPeriod, u8::MAX);
		statement_parameter!(NotificationPeriodDuration, 0u32);
		statement_parameter!(NotificationPeriodDuration, u32::MAX);
		statement_parameter!(LitePersonStatementLimit, zero_allowance.clone());
		statement_parameter!(LitePersonStatementLimit, max_allowance.clone());
		statement_parameter!(PersonStatementLimit, zero_allowance);
		statement_parameter!(PersonStatementLimit, max_allowance);

		let zero_allocation = LongTermStorageAllocation { transactions: 0, bytes: 0 };
		let max_allocation = LongTermStorageAllocation { transactions: u32::MAX, bytes: u64::MAX };
		bulletin_parameter!(LongTermStoragePeriodDuration, 0u32);
		bulletin_parameter!(LongTermStoragePeriodDuration, u32::MAX);
		bulletin_parameter!(LongTermStorageGraceWindow, 0u32);
		bulletin_parameter!(LongTermStorageGraceWindow, u32::MAX);
		bulletin_parameter!(LongTermStorageClaimsPerPeriod, 0u8);
		bulletin_parameter!(LongTermStorageClaimsPerPeriod, u8::MAX);
		bulletin_parameter!(LongTermStorageCleanupLimit, 0u32);
		bulletin_parameter!(LongTermStorageCleanupLimit, u32::MAX);
		bulletin_parameter!(LongTermStorageAllowanceForPeople, zero_allocation);
		bulletin_parameter!(LongTermStorageAllowanceForPeople, max_allocation);
		bulletin_parameter!(LongTermStorageAllowanceForLitePeople, zero_allocation);
		bulletin_parameter!(LongTermStorageAllowanceForLitePeople, max_allocation);
		let zero_para = Location::new(1, [Parachain(0)]);
		bulletin_parameter!(BulletinChainLocation, zero_para.clone());
		assert_eq!(BulletinDataStore::bulletin_chain_location(), Ok(zero_para));
		let max_para = Location::new(1, [Parachain(u32::MAX)]);
		bulletin_parameter!(BulletinChainLocation, max_para.clone());
		assert_eq!(BulletinDataStore::bulletin_chain_location(), Ok(max_para));
		bulletin_parameter!(BulletinTransactionStoragePalletIndex, 0u8);
		bulletin_parameter!(BulletinTransactionStoragePalletIndex, u8::MAX);
	});
}
