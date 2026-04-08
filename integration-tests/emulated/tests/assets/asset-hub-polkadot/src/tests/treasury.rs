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
use emulated_integration_tests_common::accounts::{ALICE, BOB};
use frame_support::traits::fungibles::Mutate;
use parachains_common::pay::VersionedLocatableAccount;
use polkadot_runtime_common::impls::VersionedLocatableAsset;

#[test]
fn create_and_claim_treasury_spend_in_usdt() {
	const USDT_ID: u32 = 1984;
	const SPEND_AMOUNT: u128 = 1_000_000_000;

	let treasury_account = asset_hub_polkadot_runtime::Treasury::account_id();
	let root = <AssetHubPolkadot as Chain>::RuntimeOrigin::root();
	// asset kind to be spent from the treasury.
	let asset_kind = VersionedLocatableAsset::V5 {
		location: Location::new(0, []),
		asset_id: v5::AssetId(
			(v5::Junction::PalletInstance(50), v5::Junction::GeneralIndex(USDT_ID.into())).into(),
		),
	};
	// treasury spend beneficiary.
	let alice: AccountId = Polkadot::account_id_of(ALICE);
	let bob: AccountId = AssetHubPolkadot::account_id_of(BOB);
	let bob_signed = <AssetHubPolkadot as Chain>::RuntimeOrigin::signed(bob.clone());

	AssetHubPolkadot::execute_with(|| {
		type Assets = <AssetHubPolkadot as AssetHubPolkadotPallet>::Assets;

		// USDT created at genesis, mint some assets to the treasury account.
		assert_ok!(<Assets as Mutate<_>>::mint_into(USDT_ID, &treasury_account, SPEND_AMOUNT * 4));
		// beneficiary has zero balance.
		assert_eq!(<Assets as Inspect<_>>::balance(USDT_ID, &alice,), 0u128,);
	});

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		type Treasury = <AssetHubPolkadot as AssetHubPolkadotPallet>::Treasury;
		type AssetRate = <AssetHubPolkadot as AssetHubPolkadotPallet>::AssetRate;

		// create a conversion rate from `asset_kind` to the native currency.
		assert_ok!(AssetRate::create(root.clone(), Box::new(asset_kind.clone()), 2.into()));

		// create and approve a treasury spend.
		assert_ok!(Treasury::spend(
			root,
			Box::new(asset_kind),
			SPEND_AMOUNT,
			Box::new(VersionedLocatableAccount::V5 {
				location: Location::new(0, []),
				account_id: Location::new(0, Into::<[u8; 32]>::into(alice.clone())),
			}),
			None,
		));
		// claim the spend.
		assert_ok!(Treasury::payout(bob_signed.clone(), 0));
		// check the payment status.
		assert_ok!(Treasury::check_status(bob_signed, 0));

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::Treasury(pallet_treasury::Event::AssetSpendApproved { .. }) => {},
				RuntimeEvent::Treasury(pallet_treasury::Event::Paid { .. }) => {},
				RuntimeEvent::Treasury(pallet_treasury::Event::SpendProcessed { .. }) => {},
			]
		);

		// beneficiary received the assets from the treasury.
		type Assets = <AssetHubPolkadot as AssetHubPolkadotPallet>::Assets;
		assert_eq!(<Assets as Inspect<_>>::balance(USDT_ID, &alice,), SPEND_AMOUNT,);
	});
}
