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

use frame_support::{assert_err, assert_ok};
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

/// Execution and delivery fees can both be paid in any asset governance registered a rate for, at
/// that rate. HOLLAR is the first such asset, but nothing here is specific to it.
#[test]
fn xcm_fees_can_be_paid_in_any_asset_with_a_registered_rate() {
	use crate::{
		xcm_config::{FeesAtAssetRate, RelayLocation, XcmConfig},
		AssetRate, Assets as AssetsPallet, PolkadotXcm, RuntimeGenesisConfig,
	};
	use cumulus_primitives_core::{relay_chain::AsyncBackingParams, AbridgedHostConfiguration};
	use frame_support::weights::WeightToFee as WeightToFeeT;
	use sp_runtime::{BuildStorage, FixedU128};
	use xcm_executor::traits::AssetExchange;

	// An asset worth 4 DOT apiece, and one governance never registered a rate for.
	let rated = Location::new(1, [Parachain(2034), GeneralIndex(222)]);
	let unrated = Location::new(1, [Parachain(2034), GeneralIndex(333)]);
	let rate = FixedU128::from_u32(4);

	let mut ext = sp_io::TestExternalities::new(
		RuntimeGenesisConfig::default().build_storage().expect("runtime genesis builds"),
	);
	ext.execute_with(|| {
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

		// Delivery fees are quoted by the router in DOT, and settled in the asset at the same rate.
		// Sending upwards needs the relay chain's limits, which only the inherent brings in.
		cumulus_pallet_parachain_system::HostConfiguration::<Runtime>::put(
			AbridgedHostConfiguration {
				max_code_size: 3 * 1024 * 1024,
				max_head_data_size: 20 * 1024,
				max_upward_queue_count: 10,
				max_upward_queue_size: 51_200,
				max_upward_message_size: 51_200,
				max_upward_message_num_per_candidate: 10,
				hrmp_max_message_num_per_candidate: 10,
				validation_upgrade_cooldown: 6,
				validation_upgrade_delay: 6,
				async_backing_params: AsyncBackingParams {
					allowed_ancestry_len: 0,
					max_candidate_depth: 0,
				},
			},
		);
		type AssetExchanger = <XcmConfig as xcm_executor::Config>::AssetExchanger;
		let message = Xcm::<()>::builder_unsafe().clear_origin().build();
		let quote = |asset: Location| -> Result<Assets, ()> {
			PolkadotXcm::query_delivery_fees::<AssetExchanger>(
				VersionedLocation::from(Location::parent()),
				VersionedXcm::from(message.clone()),
				AssetId(asset).into(),
			)
			.map(|fees| Assets::try_from(fees).expect("fees are in the latest version"))
			.map_err(|_| ())
		};
		let in_dot = quote(RelayLocation::get()).expect("the relay chain is routable");
		let Some(Asset { fun: Fungible(dot_fee), .. }) = in_dot.get(0).cloned() else {
			panic!("delivery fees are a single fungible asset: {in_dot:?}");
		};
		assert_eq!(quote(rated.clone()).unwrap(), (rated.clone(), dot_fee / 4).into());
		// And an asset without a rate cannot pay for delivery either.
		assert!(quote(unrated.clone()).is_err());

		// The same rate prices what the executor asks for, in both directions.
		let dot_asked: Assets = (RelayLocation::get(), 400u128).into();
		assert_eq!(
			FeesAtAssetRate::quote_exchange_price(
				&Asset { id: AssetId(rated.clone()), fun: Fungible(0) }.into(),
				&dot_asked,
				false,
			),
			Some((rated.clone(), 100u128).into()),
		);
		assert_eq!(
			FeesAtAssetRate::quote_exchange_price(
				&(rated.clone(), 100u128).into(),
				&(RelayLocation::get(), 1u128).into(),
				true,
			),
			Some((RelayLocation::get(), 400u128).into()),
		);
		assert_eq!(
			FeesAtAssetRate::quote_exchange_price(
				&Asset { id: AssetId(unrated), fun: Fungible(0) }.into(),
				&dot_asked,
				false,
			),
			None,
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
