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

use crate::*;
use asset_hub_polkadot_runtime::xcm_config::LocationToAccountId as AssetHubLocationToAccountId;
use emulated_integration_tests_common::accounts::ALICE;
use frame_support::{
	assert_ok, instances::Instance1, sp_runtime::traits::Dispatchable, traits::fungible::Inspect,
};
use parachains_common::pay::VersionedLocatableAccount;
use polkadot_runtime_common::impls::VersionedLocatableAsset;
use polkadot_runtime_constants::currency::UNITS;
use xcm_executor::traits::ConvertLocation;

// Fund Fellowship Treasury from Asset Hub Treasury and spend from Fellowship Treasury.
#[test]
fn fellowship_treasury_spend() {
	// target fellowship balance on Asset Hub in DOTs.
	let fellowship_treasury_balance = 1_000_000 * UNITS;
	// fellowship first spend balance in DOTs.
	let fellowship_spend_balance = 10_000 * UNITS;

	let init_alice_balance = AssetHubPolkadot::execute_with(|| {
		<<AssetHubPolkadot as AssetHubPolkadotPallet>::Balances as Inspect<_>>::balance(
			&Polkadot::account_id_of(ALICE),
		)
	});

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		type Balances = <AssetHubPolkadot as AssetHubPolkadotPallet>::Balances;

		let root = <AssetHubPolkadot as Chain>::RuntimeOrigin::root();
		let treasury_account = asset_hub_polkadot_runtime::Treasury::account_id();

		// Fund Treasury account on Asset Hub with DOTs.
		assert_ok!(Balances::force_set_balance(
			root.clone(),
			treasury_account.clone().into(),
			fellowship_treasury_balance * 2,
		));

		let fellowship_treasury_location: Location =
			Location::new(1, [Parachain(1001), PalletInstance(65)]);

		// Spend from Asset Hub Treasury to Fellowship Treasury.
		assert_ok!(asset_hub_polkadot_runtime::Treasury::spend(
			root,
			Box::new(VersionedLocatableAsset::V5 {
				location: Location::new(0, []),
				asset_id: Location::parent().into(),
			}),
			fellowship_treasury_balance,
			Box::new(VersionedLocatableAccount::V5 {
				location: Location::new(0, []),
				account_id: fellowship_treasury_location,
			}),
			None,
		));

		// Claim the spend.
		let alice_signed =
			<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(Polkadot::account_id_of(ALICE));
		assert_ok!(asset_hub_polkadot_runtime::Treasury::payout(alice_signed, 0));

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::Treasury(pallet_treasury::Event::AssetSpendApproved { .. }) => {},
				RuntimeEvent::Treasury(pallet_treasury::Event::Paid { .. }) => {},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		type Balances = <AssetHubPolkadot as AssetHubPolkadotPallet>::Balances;

		// Ensure that the funds deposited to the Fellowship Treasury account.
		let fellowship_treasury_location: Location =
			Location::new(1, [Parachain(1001), PalletInstance(65)]);
		let fellowship_treasury_account =
			AssetHubLocationToAccountId::convert_location(&fellowship_treasury_location).unwrap();

		assert_eq!(
			<Balances as Inspect<_>>::balance(&fellowship_treasury_account),
			fellowship_treasury_balance
		);
	});

	CollectivesPolkadot::execute_with(|| {
		type RuntimeEvent = <CollectivesPolkadot as Chain>::RuntimeEvent;
		type RuntimeCall = <CollectivesPolkadot as Chain>::RuntimeCall;
		type RuntimeOrigin = <CollectivesPolkadot as Chain>::RuntimeOrigin;
		type Runtime = <CollectivesPolkadot as Chain>::Runtime;
		type FellowshipTreasury =
			<CollectivesPolkadot as CollectivesPolkadotPallet>::FellowshipTreasury;

		// Fund Alice account from Fellowship Treasury.

		let fellows_origin: RuntimeOrigin =
			collectives_polkadot_runtime::fellowship::pallet_fellowship_origins::Origin::Fellows
				.into();
		let asset_hub_location: Location = (Parent, Parachain(1000)).into();
		let native_asset_on_asset_hub = Location::parent();

		let alice_location: Location =
			[AccountId32 { network: None, id: Polkadot::account_id_of(ALICE).into() }].into();

		let fellowship_treasury_spend_call =
			RuntimeCall::FellowshipTreasury(pallet_treasury::Call::<Runtime, Instance1>::spend {
				asset_kind: bx!(VersionedLocatableAsset::V5 {
					location: asset_hub_location,
					asset_id: native_asset_on_asset_hub.into(),
				}),
				amount: fellowship_spend_balance,
				beneficiary: bx!(VersionedLocation::from(alice_location)),
				valid_from: None,
			});

		assert_ok!(fellowship_treasury_spend_call.dispatch(fellows_origin));

		// Claim the spend.

		let alice_signed = RuntimeOrigin::signed(Polkadot::account_id_of(ALICE));
		assert_ok!(FellowshipTreasury::payout(alice_signed.clone(), 0));

		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				RuntimeEvent::FellowshipTreasury(pallet_treasury::Event::AssetSpendApproved { .. }) => {},
				RuntimeEvent::FellowshipTreasury(pallet_treasury::Event::Paid { .. }) => {},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		type Balances = <AssetHubPolkadot as AssetHubPolkadotPallet>::Balances;

		// Ensure that the funds deposited to Alice account.

		let alice_account = Polkadot::account_id_of(ALICE);
		assert_eq!(
			<Balances as Inspect<_>>::balance(&alice_account),
			fellowship_spend_balance + init_alice_balance
		);

		// Assert events triggered by xcm pay program:
		// 1. treasury asset transferred to spend beneficiary;
		// 2. response to Collectives chain Fellowship Treasury pallet instance sent back;
		// 3. XCM program completed;
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::Balances(pallet_balances::Event::Transfer { .. }) => {},
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
				RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true ,.. }) => {},
			]
		);
	});
}
