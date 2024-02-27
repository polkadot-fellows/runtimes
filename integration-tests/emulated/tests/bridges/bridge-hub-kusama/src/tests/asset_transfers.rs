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

fn send_asset_from_asset_hub_kusama_to_asset_hub_polkadot(id: MultiLocation, amount: u128) {
	let destination = asset_hub_polkadot_location();

	// fund the AHK's SA on BHK for paying bridge transport fees
	BridgeHubKusama::fund_para_sovereign(AssetHubKusama::para_id(), 10_000_000_000_000u128);

	// set XCM versions
	AssetHubKusama::force_xcm_version(destination, XCM_VERSION);
	BridgeHubKusama::force_xcm_version(bridge_hub_polkadot_location(), XCM_VERSION);

	// send message over bridge
	assert_ok!(send_asset_from_asset_hub_kusama(destination, (id, amount)));
	assert_bridge_hub_kusama_message_accepted(true);
	assert_bridge_hub_polkadot_message_received();
}

#[test]
fn send_ksms_from_asset_hub_kusama_to_asset_hub_polkadot() {
	let ksm_at_asset_hub_kusama: MultiLocation = Parent.into();
	let ksm_at_asset_hub_polkadot =
		MultiLocation { parents: 2, interior: X1(GlobalConsensus(NetworkId::Kusama)) };
	let owner: AccountId = AssetHubPolkadot::account_id_of(ALICE);
	AssetHubPolkadot::force_create_foreign_asset(
		ksm_at_asset_hub_polkadot,
		owner,
		true,
		ASSET_MIN_BALANCE,
		vec![],
	);
	let sov_ahp_on_ahk = AssetHubKusama::sovereign_account_of_parachain_on_other_global_consensus(
		NetworkId::Polkadot,
		AssetHubPolkadot::para_id(),
	);

	let ksms_in_reserve_on_ahk_before =
		<AssetHubKusama as Chain>::account_data_of(sov_ahp_on_ahk.clone()).free;
	let sender_ksms_before =
		<AssetHubKusama as Chain>::account_data_of(AssetHubKusamaSender::get()).free;
	let receiver_ksms_before = AssetHubPolkadot::execute_with(|| {
		type Assets = <AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(ksm_at_asset_hub_polkadot, &AssetHubPolkadotReceiver::get())
	});

	let amount = ASSET_HUB_KUSAMA_ED * 1_000;
	send_asset_from_asset_hub_kusama_to_asset_hub_polkadot(ksm_at_asset_hub_kusama, amount);
	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				// issue KSMs on AHP
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == ksm_at_asset_hub_kusama,
					owner: *owner == AssetHubPolkadotReceiver::get(),
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_ksms_after =
		<AssetHubKusama as Chain>::account_data_of(AssetHubKusamaSender::get()).free;
	let receiver_ksms_after = AssetHubPolkadot::execute_with(|| {
		type Assets = <AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(ksm_at_asset_hub_polkadot, &AssetHubPolkadotReceiver::get())
	});
	let ksms_in_reserve_on_ahk_after =
		<AssetHubKusama as Chain>::account_data_of(sov_ahp_on_ahk.clone()).free;

	// Sender's balance is reduced
	assert!(sender_ksms_before > sender_ksms_after);
	// Receiver's balance is increased
	assert!(receiver_ksms_after > receiver_ksms_before);
	// Reserve balance is reduced by sent amount
	assert_eq!(ksms_in_reserve_on_ahk_after, ksms_in_reserve_on_ahk_before + amount);
}

#[test]
fn send_dots_from_asset_hub_kusama_to_asset_hub_polkadot() {
	let prefund_amount = 10_000_000_000_000u128;
	let dot_at_asset_hub_kusama =
		MultiLocation { parents: 2, interior: X1(GlobalConsensus(NetworkId::Polkadot)) };
	let owner: AccountId = AssetHubPolkadot::account_id_of(ALICE);
	AssetHubKusama::force_create_foreign_asset(
		dot_at_asset_hub_kusama,
		owner,
		true,
		ASSET_MIN_BALANCE,
		vec![(AssetHubKusamaSender::get(), prefund_amount)],
	);

	// fund the AHK's SA on AHP with the DOT tokens held in reserve
	let sov_ahk_on_ahp = AssetHubPolkadot::sovereign_account_of_parachain_on_other_global_consensus(
		NetworkId::Kusama,
		AssetHubKusama::para_id(),
	);
	AssetHubPolkadot::fund_accounts(vec![(sov_ahk_on_ahp.clone(), prefund_amount)]);

	let dots_in_reserve_on_ahp_before =
		<AssetHubPolkadot as Chain>::account_data_of(sov_ahk_on_ahp.clone()).free;
	assert_eq!(dots_in_reserve_on_ahp_before, prefund_amount);
	let sender_dots_before = AssetHubKusama::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(dot_at_asset_hub_kusama, &AssetHubKusamaSender::get())
	});
	assert_eq!(sender_dots_before, prefund_amount);
	let receiver_dots_before =
		<AssetHubPolkadot as Chain>::account_data_of(AssetHubPolkadotReceiver::get()).free;

	let amount_to_send = ASSET_HUB_POLKADOT_ED * 1_000;
	send_asset_from_asset_hub_kusama_to_asset_hub_polkadot(dot_at_asset_hub_kusama, amount_to_send);
	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				// DOT is withdrawn from AHK's SA on AHP
				RuntimeEvent::Balances(
					pallet_balances::Event::Burned { who, amount }
				) => {
					who: *who == sov_ahk_on_ahp,
					amount: *amount == amount_to_send,
				},
				// DOTs deposited to beneficiary
				RuntimeEvent::Balances(pallet_balances::Event::Minted { who, .. }) => {
					who: *who == AssetHubPolkadotReceiver::get(),
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_dots_after = AssetHubKusama::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(dot_at_asset_hub_kusama, &AssetHubKusamaSender::get())
	});
	let receiver_dots_after =
		<AssetHubPolkadot as Chain>::account_data_of(AssetHubPolkadotReceiver::get()).free;
	let dots_in_reserve_on_ahp_after =
		<AssetHubPolkadot as Chain>::account_data_of(sov_ahk_on_ahp).free;

	// Sender's balance is reduced
	assert!(sender_dots_before > sender_dots_after);
	// Receiver's balance is increased
	assert!(receiver_dots_after > receiver_dots_before);
	// Reserve balance is reduced by sent amount
	assert_eq!(dots_in_reserve_on_ahp_after, dots_in_reserve_on_ahp_before - amount_to_send);
}
