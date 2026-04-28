// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Cumulus.
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

#![cfg(test)]

use bulletin_polkadot_runtime::{
	xcm_config::{GovernanceLocation, LocationToAccountId, PeopleLocation},
	Block, Runtime, RuntimeCall, RuntimeOrigin, TransactionStorage,
};
use frame_support::{assert_err, assert_noop, assert_ok};
use parachains_common::AccountId;
use parachains_runtimes_test_utils::GovernanceOrigin;
use pallet_bulletin_transaction_storage::AuthorizationExtent;
use sp_core::crypto::Ss58Codec;
use sp_io::TestExternalities;
use sp_keyring::Sr25519Keyring;
use sp_runtime::Either;
use system_parachains_constants::polkadot::fee::WeightToFee;
use xcm::latest::prelude::*;
use xcm_runtime_apis::conversions::LocationToAccountHelper;

const ALICE: [u8; 32] = [1u8; 32];

#[test]
fn location_conversion_works() {
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
			location: Location::new(
				1,
				[Junction::AccountId32 { network: None, id: AccountId::from(ALICE).into() }],
			),
			expected_account_id_str: "5DN5SGsuUG7PAqFL47J9meViwdnk9AdeSWKFkcHC45hEzVz4",
		},
		TestCase {
			description: "DescribeAccountId32Terminal Sibling",
			location: Location::new(
				1,
				[
					Parachain(1111),
					Junction::AccountId32 { network: None, id: AccountId::from(ALICE).into() },
				],
			),
			expected_account_id_str: "5DGRXLYwWGce7wvm14vX1Ms4Vf118FSWQbJkyQigY2pfm6bg",
		},
		// DescribeAccountKey20Terminal
		TestCase {
			description: "DescribeAccountKey20Terminal Parent",
			location: Location::new(1, [AccountKey20 { network: None, key: [0u8; 20] }]),
			expected_account_id_str: "5F5Ec11567pa919wJkX6VHtv2ZXS5W698YCW35EdEbrg14cg",
		},
		TestCase {
			description: "DescribeAccountKey20Terminal Sibling",
			location: Location::new(
				1,
				[Parachain(1111), AccountKey20 { network: None, key: [0u8; 20] }],
			),
			expected_account_id_str: "5CB2FbUds2qvcJNhDiTbRZwiS3trAy6ydFGMSVutmYijpPAg",
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
	use polkadot_runtime_constants::system_parachain::{ASSET_HUB_ID, COLLECTIVES_ID};

	// no - random para
	assert_err!(
		parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::Location(Location::new(1, Parachain(12334)))),
		Either::Right(InstructionError { index: 0, error: XcmError::Barrier })
	);
	// ok - AssetHub
	assert_ok!(parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
		Runtime,
		RuntimeOrigin,
	>(GovernanceOrigin::Location(Location::new(1, Parachain(ASSET_HUB_ID)))));
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
	>(GovernanceOrigin::Location(Location::parent())));

	// ok - governance location
	assert_ok!(parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
		Runtime,
		RuntimeOrigin,
	>(GovernanceOrigin::Location(GovernanceLocation::get())));
}

fn new_test_ext() -> TestExternalities {
	use bulletin_polkadot_runtime::{BuildStorage, RuntimeGenesisConfig};
	let genesis = RuntimeGenesisConfig {
		transaction_storage: pallet_bulletin_transaction_storage::GenesisConfig {
			retention_period: 10,
			byte_fee: 0,
			entry_fee: 0,
			..Default::default()
		},
		..Default::default()
	};
	sp_io::TestExternalities::new(genesis.build_storage().unwrap())
}

#[test]
fn authorize_account_via_root_works() {
	new_test_ext().execute_with(|| {
		let who: AccountId = Sr25519Keyring::Alice.to_account_id();
		assert_ok!(TransactionStorage::authorize_account(
			RuntimeOrigin::root(),
			who.clone(),
			5,
			1024 * 1024,
		));
		assert_eq!(
			TransactionStorage::account_authorization_extent(who),
			AuthorizationExtent { transactions: 5, bytes: 1024 * 1024 },
		);
	});
}

#[test]
fn authorize_account_rejects_unsigned() {
	new_test_ext().execute_with(|| {
		let who: AccountId = Sr25519Keyring::Alice.to_account_id();
		assert_noop!(
			TransactionStorage::authorize_account(RuntimeOrigin::none(), who, 1, 100),
			sp_runtime::DispatchError::BadOrigin,
		);
	});
}

#[test]
fn authorize_account_rejects_signed_non_authorizer() {
	new_test_ext().execute_with(|| {
		let who: AccountId = Sr25519Keyring::Alice.to_account_id();
		assert_noop!(
			TransactionStorage::authorize_account(
				RuntimeOrigin::signed(who.clone()),
				who,
				1,
				100,
			),
			sp_runtime::DispatchError::BadOrigin,
		);
	});
}

#[test]
fn xcm_from_people_chain_is_accepted_as_authorizer() {
	// Construct the XCM origin as it would arrive from the People chain (a sibling parachain).
	// `EnsureXcm<Equals<PeopleLocation>>` accepts origins whose location equals PeopleLocation.
	let people_origin = RuntimeOrigin::from(pallet_xcm::Origin::Xcm(PeopleLocation::get()));
	new_test_ext().execute_with(|| {
		let who: AccountId = Sr25519Keyring::Bob.to_account_id();
		assert_ok!(TransactionStorage::authorize_account(
			people_origin,
			who.clone(),
			3,
			512 * 1024,
		));
		assert_eq!(
			TransactionStorage::account_authorization_extent(who),
			AuthorizationExtent { transactions: 3, bytes: 512 * 1024 },
		);
	});
}

#[test]
fn xcm_from_non_people_sibling_is_rejected_as_authorizer() {
	use polkadot_runtime_constants::system_parachain::ASSET_HUB_ID;
	let asset_hub_location = Location::new(1, [Parachain(ASSET_HUB_ID)]);
	let non_people_origin = RuntimeOrigin::from(pallet_xcm::Origin::Xcm(asset_hub_location));
	new_test_ext().execute_with(|| {
		let who: AccountId = Sr25519Keyring::Bob.to_account_id();
		assert_noop!(
			TransactionStorage::authorize_account(non_people_origin, who, 1, 100),
			sp_runtime::DispatchError::BadOrigin,
		);
	});
}

#[test]
fn authorize_preimage_via_root_works() {
	new_test_ext().execute_with(|| {
		let content_hash = [42u8; 32];
		assert_ok!(TransactionStorage::authorize_preimage(
			RuntimeOrigin::root(),
			content_hash,
			8 * 1024 * 1024,
		));
		assert_eq!(
			TransactionStorage::preimage_authorization_extent(content_hash),
			AuthorizationExtent { transactions: 1, bytes: 8 * 1024 * 1024 },
		);
	});
}
