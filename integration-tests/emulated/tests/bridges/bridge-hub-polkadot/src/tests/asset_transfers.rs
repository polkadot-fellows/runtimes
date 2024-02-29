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
use crate::tests::*;

fn send_asset_from_asset_hub_polkadot_to_asset_hub_kusama(id: MultiLocation, amount: u128) {
	let destination = asset_hub_kusama_location();

	// fund the AHP's SA on BHP for paying bridge transport fees
	BridgeHubPolkadot::fund_para_sovereign(AssetHubPolkadot::para_id(), 10_000_000_000_000u128);

	// set XCM versions
	AssetHubPolkadot::force_xcm_version(destination, XCM_VERSION);
	BridgeHubPolkadot::force_xcm_version(bridge_hub_kusama_location(), XCM_VERSION);

	// send message over bridge
	assert_ok!(send_asset_from_asset_hub_polkadot(destination, (id, amount)));
	assert_bridge_hub_polkadot_message_accepted(true);
	assert_bridge_hub_kusama_message_received();
}

#[test]
fn send_dots_from_asset_hub_polkadot_to_asset_hub_kusama() {
	let dot_at_asset_hub_polkadot: MultiLocation = Parent.into();
	let dot_at_asset_hub_kusama =
		MultiLocation { parents: 2, interior: X1(GlobalConsensus(NetworkId::Polkadot)) };
	let owner: AccountId = AssetHubKusama::account_id_of(ALICE);
	AssetHubKusama::force_create_foreign_asset(
		dot_at_asset_hub_kusama,
		owner,
		true,
		ASSET_MIN_BALANCE,
		vec![],
	);
	let sov_ahk_on_ahp = AssetHubPolkadot::sovereign_account_of_parachain_on_other_global_consensus(
		NetworkId::Kusama,
		AssetHubKusama::para_id(),
	);

	let dots_in_reserve_on_ahp_before =
		<AssetHubPolkadot as Chain>::account_data_of(sov_ahk_on_ahp.clone()).free;
	let sender_dots_before =
		<AssetHubPolkadot as Chain>::account_data_of(AssetHubPolkadotSender::get()).free;
	let receiver_dots_before = AssetHubKusama::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(dot_at_asset_hub_kusama, &AssetHubKusamaReceiver::get())
	});

	let amount = ASSET_HUB_POLKADOT_ED * 1_000;
	send_asset_from_asset_hub_polkadot_to_asset_hub_kusama(dot_at_asset_hub_polkadot, amount);
	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubKusama,
			vec![
				// issue DOTs on AHK
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == dot_at_asset_hub_kusama,
					owner: *owner == AssetHubKusamaReceiver::get(),
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_dots_after =
		<AssetHubPolkadot as Chain>::account_data_of(AssetHubPolkadotSender::get()).free;
	let receiver_dots_after = AssetHubKusama::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(dot_at_asset_hub_kusama, &AssetHubKusamaReceiver::get())
	});
	let dots_in_reserve_on_ahp_after =
		<AssetHubPolkadot as Chain>::account_data_of(sov_ahk_on_ahp).free;

	// Sender's balance is reduced
	assert!(sender_dots_before > sender_dots_after);
	// Receiver's balance is increased
	assert!(receiver_dots_after > receiver_dots_before);
	// Reserve balance is increased by sent amount
	assert_eq!(dots_in_reserve_on_ahp_after, dots_in_reserve_on_ahp_before + amount);
}

#[test]
fn send_ksms_from_asset_hub_polkadot_to_asset_hub_kusama() {
	let prefund_amount = 10_000_000_000_000u128;
	let ksm_at_asset_hub_polkadot =
		MultiLocation { parents: 2, interior: X1(GlobalConsensus(NetworkId::Kusama)) };
	let owner: AccountId = AssetHubPolkadot::account_id_of(ALICE);
	AssetHubPolkadot::force_create_foreign_asset(
		ksm_at_asset_hub_polkadot,
		owner,
		true,
		ASSET_MIN_BALANCE,
		vec![(AssetHubPolkadotSender::get(), prefund_amount)],
	);

	// fund the AHP's SA on AHK with the KSM tokens held in reserve
	let sov_ahp_on_ahk = AssetHubKusama::sovereign_account_of_parachain_on_other_global_consensus(
		NetworkId::Polkadot,
		AssetHubPolkadot::para_id(),
	);
	AssetHubKusama::fund_accounts(vec![(sov_ahp_on_ahk.clone(), prefund_amount)]);

	let ksms_in_reserve_on_ahk_before =
		<AssetHubKusama as Chain>::account_data_of(sov_ahp_on_ahk.clone()).free;
	assert_eq!(ksms_in_reserve_on_ahk_before, prefund_amount);
	let sender_ksms_before = AssetHubPolkadot::execute_with(|| {
		type Assets = <AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(ksm_at_asset_hub_polkadot, &AssetHubPolkadotSender::get())
	});
	assert_eq!(sender_ksms_before, prefund_amount);
	let receiver_ksms_before =
		<AssetHubKusama as Chain>::account_data_of(AssetHubKusamaReceiver::get()).free;

	let amount_to_send = ASSET_HUB_KUSAMA_ED * 1_000;
	send_asset_from_asset_hub_polkadot_to_asset_hub_kusama(
		ksm_at_asset_hub_polkadot,
		amount_to_send,
	);
	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubKusama,
			vec![
				// KSM is withdrawn from AHP's SA on AHK
				RuntimeEvent::Balances(
					pallet_balances::Event::Burned { who, amount }
				) => {
					who: *who == sov_ahp_on_ahk,
					amount: *amount == amount_to_send,
				},
				// KSMs deposited to beneficiary
				RuntimeEvent::Balances(pallet_balances::Event::Minted { who, .. }) => {
					who: *who == AssetHubKusamaReceiver::get(),
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_ksms_after = AssetHubPolkadot::execute_with(|| {
		type Assets = <AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(ksm_at_asset_hub_polkadot, &AssetHubPolkadotSender::get())
	});
	let receiver_ksms_after =
		<AssetHubKusama as Chain>::account_data_of(AssetHubKusamaReceiver::get()).free;
	let ksms_in_reserve_on_ahk_after =
		<AssetHubKusama as Chain>::account_data_of(sov_ahp_on_ahk.clone()).free;

	// Sender's balance is reduced
	assert!(sender_ksms_before > sender_ksms_after);
	// Receiver's balance is increased
	assert!(receiver_ksms_after > receiver_ksms_before);
	// Reserve balance is reduced by sent amount
	assert_eq!(ksms_in_reserve_on_ahk_after, ksms_in_reserve_on_ahk_before - amount_to_send);
}
