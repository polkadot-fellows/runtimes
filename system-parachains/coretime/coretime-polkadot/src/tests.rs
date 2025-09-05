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
	coretime::{BrokerPalletId, CoretimeBurnAccount},
	xcm_config::LocationToAccountId,
	*,
};
use coretime::CoretimeAllocator;
use cumulus_pallet_parachain_system::ValidationData;
use cumulus_primitives_core::PersistedValidationData;
use frame_support::{
	assert_err, assert_ok,
	traits::{
		fungible::{Inspect, Mutate},
		Get, OnInitialize,
	},
};
use pallet_broker::{ConfigRecordOf, RCBlockNumberOf, SaleInfo};
use parachains_runtimes_test_utils::{ExtBuilder, GovernanceOrigin};
use polkadot_runtime_constants::system_parachain::coretime::TIMESLICE_PERIOD;
use sp_core::crypto::Ss58Codec;
use sp_runtime::{traits::AccountIdConversion, Either};
use xcm_runtime_apis::conversions::LocationToAccountHelper;

const ALICE: [u8; 32] = [1u8; 32];

// We track the relay chain block number via the RelayChainDataProvider, but `set_block_number` is
// not currently available in tests (only runtime-benchmarks).
// See https://github.com/paritytech/polkadot-sdk/pull/8537
fn set_relay_block_number(b: BlockNumber) {
	let mut validation_data = ValidationData::<Runtime>::get().unwrap_or_else(||
			// PersistedValidationData does not impl default in non-std
			PersistedValidationData {
				parent_head: vec![].into(),
				relay_parent_number: Default::default(),
				max_pov_size: Default::default(),
				relay_parent_storage_root: Default::default(),
			});
	validation_data.relay_parent_number = b;
	ValidationData::<Runtime>::put(validation_data)
}

fn advance_to(b: BlockNumber) {
	while System::block_number() < b {
		let block_number = System::block_number() + 1;
		System::set_block_number(block_number);
		set_relay_block_number(block_number);
		Broker::on_initialize(block_number);
	}
}

#[test]
fn bulk_revenue_is_burnt() {
	ExtBuilder::<Runtime>::default()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			// Configure broker and start sales
			let config = ConfigRecordOf::<Runtime> {
				advance_notice: 1,
				interlude_length: 1,
				leadin_length: 2,
				region_length: 1,
				ideal_bulk_proportion: Perbill::from_percent(100),
				limit_cores_offered: None,
				renewal_bump: Perbill::from_percent(3),
				contribution_timeout: 1,
			};
			assert_ok!(Broker::configure(RuntimeOrigin::root(), config.clone()));
			assert_ok!(Broker::start_sales(RuntimeOrigin::root(), UNITS, 1));

			let sale_start = SaleInfo::<Runtime>::get().unwrap().sale_start;
			advance_to(sale_start + config.interlude_length);

			// Check and set initial balances.
			let broker_account = BrokerPalletId::get().into_account_truncating();
			let coretime_burn_account = CoretimeBurnAccount::get();
			let treasury_account = xcm_config::RelayTreasuryPalletAccount::get();
			assert_ok!(Balances::mint_into(&AccountId::from(ALICE), 200 * UNITS));
			let alice_balance_before = Balances::balance(&AccountId::from(ALICE));
			let treasury_balance_before = Balances::balance(&treasury_account);
			let broker_balance_before = Balances::balance(&broker_account);
			let burn_balance_before = Balances::balance(&coretime_burn_account);

			// Purchase coretime.
			assert_ok!(Broker::purchase(
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				100 * UNITS
			));

			// Alice decreases.
			assert!(Balances::balance(&AccountId::from(ALICE)) < alice_balance_before);
			// Treasury balance does not increase.
			assert_eq!(Balances::balance(&treasury_account), treasury_balance_before);
			// Broker pallet account does not increase.
			assert_eq!(Balances::balance(&broker_account), broker_balance_before);
			// Coretime burn pot gets the funds.
			assert!(Balances::balance(&coretime_burn_account) > burn_balance_before);

			// They're burnt when a day has passed on chain.
			// This needs to be asserted in an emulated test.
		});
}

#[test]
fn timeslice_period_is_sane() {
	// Config TimeslicePeriod is set to this constant - assumption in burning logic.
	let timeslice_period_config: RCBlockNumberOf<CoretimeAllocator> =
		<Runtime as pallet_broker::Config>::TimeslicePeriod::get();
	assert_eq!(timeslice_period_config, TIMESLICE_PERIOD);

	// Timeslice period constant non-zero - assumption in burning logic.
	#[cfg(feature = "fast-runtime")]
	assert_eq!(TIMESLICE_PERIOD, 20);
	#[cfg(not(feature = "fast-runtime"))]
	assert_eq!(TIMESLICE_PERIOD, 80);
}

#[test]
fn location_conversion_works() {
	let alice_32 =
		AccountId32 { network: None, id: polkadot_core_primitives::AccountId::from(ALICE).into() };
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
		let expected = polkadot_core_primitives::AccountId::from_string(tc.expected_account_id_str)
			.expect("Invalid AccountId string");

		let got = LocationToAccountHelper::<polkadot_core_primitives::AccountId, LocationToAccountId>::convert_location(
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
		WeightToFee,
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
