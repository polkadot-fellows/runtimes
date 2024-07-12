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

//! Tests concerning the Kusama Treasury.

use crate::*;
use emulated_integration_tests_common::accounts::{ALICE, BOB};
use frame_support::{
	dispatch::RawOrigin,
	sp_runtime::traits::Dispatchable,
	traits::{
		fungible::Inspect,
		fungibles::{Create, Inspect as FungiblesInspect, Mutate},
	},
};
use kusama_runtime::OriginCaller;
use kusama_runtime_constants::currency::GRAND;
use polkadot_runtime_common::impls::VersionedLocatableAsset;
use xcm_executor::traits::ConvertLocation;

// Fund Treasury account on Asset Hub from Treasury account on Relay Chain with KSMs.
#[test]
fn spend_ksm_on_asset_hub() {
	// initial treasury balance on Asset Hub in KSMs.
	let treasury_balance = 9_000 * GRAND;
	// the balance spend on Asset Hub.
	let treasury_spend_balance = 1_000 * GRAND;

	let init_alice_balance = AssetHubKusama::execute_with(|| {
		<<AssetHubKusama as AssetHubKusamaPallet>::Balances as Inspect<_>>::balance(
			&AssetHubKusama::account_id_of(ALICE),
		)
	});

	Kusama::execute_with(|| {
		type RuntimeEvent = <Kusama as Chain>::RuntimeEvent;
		type RuntimeCall = <Kusama as Chain>::RuntimeCall;
		type Runtime = <Kusama as Chain>::Runtime;
		type Balances = <Kusama as KusamaPallet>::Balances;
		type Treasury = <Kusama as KusamaPallet>::Treasury;

		// Fund Treasury account on Asset Hub with KSMs.

		let root = <Kusama as Chain>::RuntimeOrigin::root();
		let treasury_account = Treasury::account_id();

		// Mint assets to Treasury account on Relay Chain.
		assert_ok!(Balances::force_set_balance(
			root.clone(),
			treasury_account.clone().into(),
			treasury_balance * 2,
		));

		let native_asset = Location::here();
		let asset_hub_location: Location = [Parachain(1000)].into();
		let treasury_location: Location = (Parent, PalletInstance(18)).into();

		let teleport_call = RuntimeCall::Utility(pallet_utility::Call::<Runtime>::dispatch_as {
			as_origin: bx!(OriginCaller::system(RawOrigin::Signed(treasury_account))),
			call: bx!(RuntimeCall::XcmPallet(pallet_xcm::Call::<Runtime>::teleport_assets {
				dest: bx!(VersionedLocation::V4(asset_hub_location.clone())),
				beneficiary: bx!(VersionedLocation::V4(treasury_location)),
				assets: bx!(VersionedAssets::V4(
					Asset { id: native_asset.clone().into(), fun: treasury_balance.into() }.into()
				)),
				fee_asset_item: 0,
			})),
		});

		// Dispatched from Root to `dispatch_as` `Signed(treasury_account)`.
		assert_ok!(teleport_call.dispatch(root));

		assert_expected_events!(
			Kusama,
			vec![
				RuntimeEvent::XcmPallet(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	Kusama::execute_with(|| {
		type RuntimeEvent = <Kusama as Chain>::RuntimeEvent;
		type RuntimeCall = <Kusama as Chain>::RuntimeCall;
		type RuntimeOrigin = <Kusama as Chain>::RuntimeOrigin;
		type Runtime = <Kusama as Chain>::Runtime;
		type Treasury = <Kusama as KusamaPallet>::Treasury;

		// Fund Alice account from Kusama Treasury account on Asset Hub.

		let treasury_origin: RuntimeOrigin =
			kusama_runtime::governance::pallet_custom_origins::Origin::Treasurer.into();

		let alice_location: Location =
			[Junction::AccountId32 { network: None, id: Kusama::account_id_of(ALICE).into() }]
				.into();
		let asset_hub_location: Location = [Parachain(1000)].into();
		let native_asset_on_asset_hub = Location::parent();

		let treasury_spend_call = RuntimeCall::Treasury(pallet_treasury::Call::<Runtime>::spend {
			asset_kind: bx!(VersionedLocatableAsset::V4 {
				location: asset_hub_location.clone(),
				asset_id: native_asset_on_asset_hub.into(),
			}),
			amount: treasury_spend_balance,
			beneficiary: bx!(VersionedLocation::V4(alice_location)),
			valid_from: None,
		});

		assert_ok!(treasury_spend_call.dispatch(treasury_origin));

		// Claim the spend.

		let bob_signed = RuntimeOrigin::signed(Kusama::account_id_of(BOB));
		assert_ok!(Treasury::payout(bob_signed.clone(), 0));

		assert_expected_events!(
			Kusama,
			vec![
				RuntimeEvent::Treasury(pallet_treasury::Event::AssetSpendApproved { .. }) => {},
				RuntimeEvent::Treasury(pallet_treasury::Event::Paid { .. }) => {},
			]
		);
	});

	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
		type Balances = <AssetHubKusama as AssetHubKusamaPallet>::Balances;

		// Ensure that the funds deposited to Alice account.

		let alice_account = AssetHubKusama::account_id_of(ALICE);
		assert_eq!(
			<Balances as Inspect<_>>::balance(&alice_account),
			treasury_spend_balance + init_alice_balance
		);

		// Assert events triggered by xcm pay program:
		// 1. treasury asset transferred to spend beneficiary;
		// 2. response to Relay Chain Treasury pallet instance sent back;
		// 3. XCM program completed;
		assert_expected_events!(
			AssetHubKusama,
			vec![
				RuntimeEvent::Balances(pallet_balances::Event::Transfer { .. }) => {},
				RuntimeEvent::ParachainSystem(cumulus_pallet_parachain_system::Event::UpwardMessageSent { .. }) => {},
				RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true ,.. }) => {},
			]
		);
	});
}

#[test]
fn create_and_claim_treasury_spend_in_usdt() {
	const ASSET_ID: u32 = 1984;
	const SPEND_AMOUNT: u128 = 1_000_000;
	// treasury location from a sibling parachain.
	let treasury_location: Location = Location::new(1, PalletInstance(18));
	// treasury account on a sibling parachain.
	let treasury_account =
		asset_hub_kusama_runtime::xcm_config::LocationToAccountId::convert_location(
			&treasury_location,
		)
		.unwrap();
	let asset_hub_location =
		v3::Location::new(0, v3::Junction::Parachain(AssetHubKusama::para_id().into()));
	let root = <Kusama as Chain>::RuntimeOrigin::root();
	// asset kind to be spend from the treasury.
	let asset_kind = VersionedLocatableAsset::V3 {
		location: asset_hub_location,
		asset_id: v3::AssetId::Concrete(
			(v3::Junction::PalletInstance(50), v3::Junction::GeneralIndex(ASSET_ID.into())).into(),
		),
	};
	// treasury spend beneficiary.
	let alice: AccountId = Kusama::account_id_of(ALICE);
	let bob: AccountId = Kusama::account_id_of(BOB);
	let bob_signed = <Kusama as Chain>::RuntimeOrigin::signed(bob.clone());

	AssetHubKusama::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaPallet>::Assets;

		// create an asset class and mint some assets to the treasury account.
		assert_ok!(<Assets as Create<_>>::create(
			ASSET_ID,
			treasury_account.clone(),
			true,
			SPEND_AMOUNT / 2
		));
		assert_ok!(<Assets as Mutate<_>>::mint_into(ASSET_ID, &treasury_account, SPEND_AMOUNT * 4));
		// beneficiary has zero balance.
		assert_eq!(<Assets as FungiblesInspect<_>>::balance(ASSET_ID, &alice,), 0u128,);
	});

	Kusama::execute_with(|| {
		type RuntimeEvent = <Kusama as Chain>::RuntimeEvent;
		type Treasury = <Kusama as KusamaPallet>::Treasury;
		type AssetRate = <Kusama as KusamaPallet>::AssetRate;

		// create a conversion rate from `asset_kind` to the native currency.
		assert_ok!(AssetRate::create(root.clone(), Box::new(asset_kind.clone()), 2.into()));

		// create and approve a treasury spend.
		assert_ok!(Treasury::spend(
			root,
			Box::new(asset_kind),
			SPEND_AMOUNT,
			Box::new(Location::new(0, Into::<[u8; 32]>::into(alice.clone())).into()),
			None,
		));
		// claim the spend.
		assert_ok!(Treasury::payout(bob_signed.clone(), 0));

		assert_expected_events!(
			Kusama,
			vec![
				RuntimeEvent::Treasury(pallet_treasury::Event::Paid { .. }) => {},
			]
		);
	});

	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
		type Assets = <AssetHubKusama as AssetHubKusamaPallet>::Assets;

		// assert events triggered by xcm pay program
		// 1. treasury asset transferred to spend beneficiary
		// 2. response to Relay Chain treasury pallet instance sent back
		// 3. XCM program completed
		assert_expected_events!(
			AssetHubKusama,
			vec![
				RuntimeEvent::Assets(pallet_assets::Event::Transferred { asset_id: id, from, to, amount }) => {
					id: id == &ASSET_ID,
					from: from == &treasury_account,
					to: to == &alice,
					amount: amount == &SPEND_AMOUNT,
				},
				RuntimeEvent::ParachainSystem(cumulus_pallet_parachain_system::Event::UpwardMessageSent { .. }) => {},
				RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true ,.. }) => {},
			]
		);
		// beneficiary received the assets from the treasury.
		assert_eq!(<Assets as FungiblesInspect<_>>::balance(ASSET_ID, &alice,), SPEND_AMOUNT,);
	});

	Kusama::execute_with(|| {
		type RuntimeEvent = <Kusama as Chain>::RuntimeEvent;
		type Treasury = <Kusama as KusamaPallet>::Treasury;

		// check the payment status to ensure the response from the AssetHub was received.
		assert_ok!(Treasury::check_status(bob_signed, 0));
		assert_expected_events!(
			Kusama,
			vec![
				RuntimeEvent::Treasury(pallet_treasury::Event::SpendProcessed { .. }) => {},
			]
		);
	});
}
