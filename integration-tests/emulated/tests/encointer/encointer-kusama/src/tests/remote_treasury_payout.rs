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
use emulated_integration_tests_common::{xcm_emulator::ConvertLocation, USDT_ID};
use encointer_kusama_runtime::{
	treasuries_xcm_payout::{ConstantKsmFee, GetRemoteFee},
	xcm_config::KsmLocation,
	AccountId, CommunityIdentifier,
};
use frame_support::{
	assert_ok,
	traits::{fungible::Mutate as M, fungibles::Mutate},
};
use kusama_system_emulated_network::asset_hub_kusama_emulated_chain::AssetHubKusamaParaPallet;
use polkadot_runtime_common::impls::VersionedLocatableAsset;
use xcm::latest::Junctions::X2;
use xcm_runtime_apis::fees::runtime_decl_for_xcm_payment_api::XcmPaymentApiV1;

fn remote_fee() -> u128 {
	let fee_asset = ConstantKsmFee::get_remote_fee(Xcm::new(), None);
	let Asset { id: _, ref fun } = fee_asset;

	match fun {
		Fungible(fee) => *fee,
		NonFungible(_) => panic!("Invalid fee"),
	}
}

fn treasury_account(maybe_community_identifier: Option<CommunityIdentifier>) -> AccountId {
	<EncointerKusama as TestExt>::execute_with(|| {
		encointer_kusama_runtime::EncointerTreasuries::get_community_treasury_account_unchecked(
			maybe_community_identifier,
		)
	})
}

fn treasury_location_on_ah() -> Location {
	// Transact the parents native asset on parachain 1000.
	let asset_kind = VersionedLocatableAsset::V5 {
		location: (Parent, Parachain(1000)).into(),
		asset_id: v5::AssetId(Location::parent()),
	};

	let treasury_account = treasury_account(None);

	<EncointerKusama as TestExt>::execute_with(|| {
		encointer_kusama_runtime::TransferOverXcm::from_on_remote(
			&treasury_account,
			asset_kind.clone(),
		)
		.unwrap()
	})
}

fn encointer_treasury_sov_account_on_ah() -> AccountId {
	let treasury_location_on_ah = treasury_location_on_ah();

	encointer_kusama_runtime::xcm_config::LocationToAccountId::convert_location(
		&treasury_location_on_ah,
	)
	.unwrap()
}

#[test]
fn treasury_location_on_ah_works() {
	let treasury = treasury_account(None);
	assert_eq!(
		treasury_location_on_ah(),
		Location::new(
			1,
			X2([Parachain(1001), Junction::AccountId32 { network: None, id: treasury.into() }]
				.into(),),
		)
	);
}

#[test]
fn constant_remote_execution_fees_are_correct() {
	let sender = AccountId::new([1u8; 32]);
	let recipient = AccountId::new([5u8; 32]);

	// Transact the parents native asset on parachain 1000.
	let asset_kind = VersionedLocatableAsset::V5 {
		location: (Parent, Parachain(1000)).into(),
		asset_id: v5::AssetId(Location::parent()),
	};

	let transfer_amount = 1_000_000_000_000u128;

	let mut remote_message = Xcm::<()>::new();
	<EncointerKusama as TestExt>::execute_with(|| {
		let (message, _, _) = encointer_kusama_runtime::TransferOverXcm::get_remote_transfer_xcm(
			&sender,
			&recipient,
			asset_kind.clone(),
			transfer_amount,
		)
		.unwrap();
		remote_message = message;
	});

	let mut execution_fees = 0;

	<AssetHubKusama as TestExt>::execute_with(|| {
		type Runtime = <AssetHubKusama as Chain>::Runtime;

		let weight = Runtime::query_xcm_weight(VersionedXcm::V5(remote_message.clone())).unwrap();
		execution_fees = Runtime::query_weight_to_asset_fee(
			weight,
			VersionedAssetId::from(AssetId(Location::parent())),
		)
		.unwrap();
	});

	assert_eq!(
		// The constant fee ignores the xcm anyhow
		ConstantKsmFee::get_remote_fee(Xcm::new(), None),
		(Location::parent(), execution_fees).into()
	);
}

#[test]
fn remote_treasury_usdt_payout_works() {
	const SPEND_AMOUNT: u128 = 10_000_000;
	const ONE_KSM: u128 = 1_000_000_000_000;
	const TREASURY_INITIAL_BALANCE: u128 = 100 * ONE_KSM;
	let recipient = AccountId::new([5u8; 32]);

	let asset_kind = VersionedLocatableAsset::V5 {
		location: (Parent, Parachain(1000)).into(),
		asset_id: AssetId((PalletInstance(50), GeneralIndex(USDT_ID.into())).into()),
	};

	let treasury_account_on_ah = encointer_treasury_sov_account_on_ah();
	println!("treasury_account: {treasury_account_on_ah:?}");

	<AssetHubKusama as TestExt>::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaParaPallet>::Assets;
		type Balances = <AssetHubKusama as AssetHubKusamaParaPallet>::Balances;

		// USDT created at genesis, mint some assets to the treasury account.
		assert_ok!(<Assets as Mutate<_>>::mint_into(
			USDT_ID,
			&treasury_account_on_ah,
			SPEND_AMOUNT * 4
		));
		assert_ok!(<Balances as M<_>>::mint_into(
			&treasury_account_on_ah,
			TREASURY_INITIAL_BALANCE
		));

		// // Check starting balance
		assert_eq!(Assets::balance(USDT_ID, &treasury_account_on_ah), SPEND_AMOUNT * 4);
		assert_eq!(Balances::free_balance(&treasury_account_on_ah), TREASURY_INITIAL_BALANCE);
		assert_eq!(Assets::balance(USDT_ID, &recipient), 0);
	});

	<EncointerKusama as TestExt>::execute_with(|| {
		encointer_kusama_runtime::EncointerTreasuries::do_spend_asset(
			None,
			&recipient,
			asset_kind.clone(),
			SPEND_AMOUNT,
		)
		.unwrap();
	});

	assert_asset_hub_kusama_tokens_received(recipient.clone());

	<AssetHubKusama as TestExt>::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaParaPallet>::Assets;
		type Balances = <AssetHubKusama as AssetHubKusamaParaPallet>::Balances;

		// Check ending balance
		assert_eq!(
			Balances::free_balance(&treasury_account_on_ah),
			TREASURY_INITIAL_BALANCE - remote_fee()
		);
		assert_eq!(Assets::balance(USDT_ID, &treasury_account_on_ah), SPEND_AMOUNT * 3);
		assert_eq!(Assets::balance(USDT_ID, &recipient), SPEND_AMOUNT);
	});
}

#[test]
fn remote_treasury_native_payout_works() {
	const ONE_KSM: u128 = 1_000_000_000_000;
	const SPEND_AMOUNT: u128 = ONE_KSM;
	const TREASURY_INITIAL_BALANCE: u128 = 100 * ONE_KSM;
	let recipient = AccountId::new([5u8; 32]);

	let asset_kind = VersionedLocatableAsset::V5 {
		location: (Parent, Parachain(1000)).into(),
		asset_id: AssetId(KsmLocation::get()),
	};

	let treasury_account_on_ah = encointer_treasury_sov_account_on_ah();

	<AssetHubKusama as TestExt>::execute_with(|| {
		type Balances = <AssetHubKusama as AssetHubKusamaParaPallet>::Balances;

		assert_ok!(<Balances as M<_>>::mint_into(
			&treasury_account_on_ah,
			TREASURY_INITIAL_BALANCE
		));

		// // Check starting balance
		assert_eq!(Balances::free_balance(&treasury_account_on_ah), TREASURY_INITIAL_BALANCE);
	});

	<EncointerKusama as TestExt>::execute_with(|| {
		encointer_kusama_runtime::EncointerTreasuries::do_spend_asset(
			None,
			&recipient,
			asset_kind.clone(),
			SPEND_AMOUNT,
		)
		.unwrap();
	});

	<AssetHubKusama as TestExt>::execute_with(|| {
		type Balances = <AssetHubKusama as AssetHubKusamaParaPallet>::Balances;

		// Check ending balance
		assert_eq!(
			Balances::free_balance(&treasury_account_on_ah),
			TREASURY_INITIAL_BALANCE - remote_fee() - SPEND_AMOUNT
		);
		assert_eq!(Balances::free_balance(&recipient), SPEND_AMOUNT);
	});
}


fn assert_asset_hub_kusama_tokens_received(who: AccountId) {
	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubKusama,
			vec![
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
				RuntimeEvent::Assets(pallet_assets::Event::Transferred { to, .. }) => {
					to: *to == who,
				},
			]
		);
	});
}
