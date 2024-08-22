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

fn send_assets_over_bridge<F: FnOnce()>(send_fn: F) {
	// fund the KAH's SA on BHR for paying bridge transport fees
	BridgeHubKusama::fund_para_sovereign(AssetHubKusama::para_id(), 10_000_000_000_000u128);

	// set XCM versions
	let local_asset_hub = PenpalA::sibling_location_of(AssetHubKusama::para_id());
	PenpalA::force_xcm_version(local_asset_hub.clone(), XCM_VERSION);
	AssetHubKusama::force_xcm_version(asset_hub_polkadot_location(), XCM_VERSION);
	BridgeHubKusama::force_xcm_version(bridge_hub_polkadot_location(), XCM_VERSION);

	// send message over bridge
	send_fn();

	// process and verify intermediary hops
	assert_bridge_hub_kusama_message_accepted(true);
	assert_bridge_hub_polkadot_message_received();
}

fn set_up_ksm_for_penpal_kusama_through_kah_to_pah(
	sender: &AccountId,
	amount: u128,
) -> (Location, v3::Location) {
	let ksm_at_kusama_parachains = ksm_at_ah_kusama();
	let ksm_at_asset_hub_polkadot = v3::Location::try_from(bridged_ksm_at_ah_polkadot()).unwrap();
	create_foreign_on_ah_polkadot(ksm_at_asset_hub_polkadot, true);

	let penpal_location = AssetHubKusama::sibling_location_of(PenpalA::para_id());
	let sov_penpal_on_kah = AssetHubKusama::sovereign_account_id_of(penpal_location);
	// fund Penpal's sovereign account on AssetHub
	AssetHubKusama::fund_accounts(vec![(sov_penpal_on_kah, amount * 2)]);
	// fund Penpal's sender account
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		ksm_at_kusama_parachains.clone(),
		sender.clone(),
		amount * 2,
	);
	(ksm_at_kusama_parachains, ksm_at_asset_hub_polkadot)
}

fn send_assets_from_penpal_kusama_through_kusama_ah_to_polkadot_ah(
	destination: Location,
	assets: (Assets, TransferType),
	fees: (AssetId, TransferType),
	custom_xcm_on_dest: Xcm<()>,
) {
	send_assets_over_bridge(|| {
		let sov_penpal_on_kah = AssetHubKusama::sovereign_account_id_of(
			AssetHubKusama::sibling_location_of(PenpalA::para_id()),
		);
		let sov_pah_on_kah =
			AssetHubKusama::sovereign_account_of_parachain_on_other_global_consensus(
				Polkadot,
				AssetHubPolkadot::para_id(),
			);
		// send message over bridge
		assert_ok!(PenpalA::execute_with(|| {
			let signed_origin = <PenpalA as Chain>::RuntimeOrigin::signed(PenpalASender::get());
			<PenpalA as PenpalAPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
				signed_origin,
				bx!(destination.into()),
				bx!(assets.0.into()),
				bx!(assets.1),
				bx!(fees.0.into()),
				bx!(fees.1),
				bx!(VersionedXcm::from(custom_xcm_on_dest)),
				WeightLimit::Unlimited,
			)
		}));
		// verify intermediary AH Kusama hop
		AssetHubKusama::execute_with(|| {
			type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
			assert_expected_events!(
				AssetHubKusama,
				vec![
					// Amount to reserve transfer is withdrawn from Penpal's sovereign account
					RuntimeEvent::Balances(
						pallet_balances::Event::Burned { who, .. }
					) => {
						who: *who == sov_penpal_on_kah.clone(),
					},
					// Amount deposited in PAH's sovereign account
					RuntimeEvent::Balances(pallet_balances::Event::Minted { who, .. }) => {
						who: *who == sov_pah_on_kah.clone(),
					},
					RuntimeEvent::XcmpQueue(
						cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }
					) => {},
				]
			);
		});
	});
}

#[test]
/// Test transfer of KSM from AssetHub Kusama to AssetHub Polkadot.
fn send_ksm_from_asset_hub_kusama_to_asset_hub_polkadot() {
	let amount = ASSET_HUB_KUSAMA_ED * 1_000;
	let sender = AssetHubKusamaSender::get();
	let receiver = AssetHubPolkadotReceiver::get();
	let ksm_at_asset_hub_kusama = v3::Location::try_from(ksm_at_ah_kusama()).unwrap();
	let bridged_ksm_at_ah_polkadot = v3::Location::try_from(bridged_ksm_at_ah_polkadot()).unwrap();

	create_foreign_on_ah_polkadot(bridged_ksm_at_ah_polkadot, true);
	set_up_pool_with_dot_on_ah_polkadot(bridged_ksm_at_ah_polkadot, true);

	let sov_ahp_on_ahk = AssetHubKusama::sovereign_account_of_parachain_on_other_global_consensus(
		Polkadot,
		AssetHubPolkadot::para_id(),
	);
	let ksms_in_reserve_on_ahk_before =
		<AssetHubKusama as Chain>::account_data_of(sov_ahp_on_ahk.clone()).free;
	let sender_ksms_before = <AssetHubKusama as Chain>::account_data_of(sender.clone()).free;
	let receiver_ksms_before =
		foreign_balance_on_ah_polkadot(bridged_ksm_at_ah_polkadot, &receiver);

	let ksm_at_ah_kusama_latest = ksm_at_ah_kusama();

	// send KSMs, use them for fees
	send_assets_over_bridge(|| {
		let destination = asset_hub_polkadot_location();
		let assets: Assets = (ksm_at_ah_kusama_latest, amount).into();
		let fee_idx = 0;
		assert_ok!(send_assets_from_asset_hub_kusama(destination, assets, fee_idx));
	});

	// verify expected events on final destination
	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				// issue KSMs on PAH
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

	let sender_ksms_after = <AssetHubKusama as Chain>::account_data_of(sender.clone()).free;
	let receiver_ksms_after = foreign_balance_on_ah_polkadot(bridged_ksm_at_ah_polkadot, &receiver);
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
/// Send bridged assets "back" from AssetHub Kusama to AssetHub Polkadot.
///
/// This mix of assets should cover the whole range:
/// - bridged native assets: KSM,
/// - bridged trust-based assets: USDT (exists only on Polkadot, Kusama gets it from Polkadot over
///   bridge),
/// - bridged foreign asset / double-bridged asset (other bridge / Snowfork): wETH (bridged from
///   Ethereum to Polkadot over Snowbridge, then bridged over to Kusama through this bridge).
fn send_back_dot_usdt_and_weth_from_asset_hub_kusama_to_asset_hub_polkadot() {
	let prefund_amount = 10_000_000_000_000u128;
	let amount_to_send = ASSET_HUB_POLKADOT_ED * 1_000;
	let sender = AssetHubKusamaSender::get();
	let receiver = AssetHubPolkadotReceiver::get();
	let dot_at_asset_hub_kusama = v3::Location::try_from(bridged_dot_at_ah_kusama()).unwrap();
	let prefund_accounts = vec![(sender.clone(), prefund_amount)];
	create_foreign_on_ah_kusama(dot_at_asset_hub_kusama, true, prefund_accounts);

	////////////////////////////////////////////////////////////
	// Let's first send back just some DOTs as a simple example
	////////////////////////////////////////////////////////////

	// fund the KAH's SA on PAH with the DOT tokens held in reserve
	let sov_kah_on_pah = AssetHubPolkadot::sovereign_account_of_parachain_on_other_global_consensus(
		Kusama,
		AssetHubKusama::para_id(),
	);
	AssetHubPolkadot::fund_accounts(vec![(sov_kah_on_pah.clone(), prefund_amount)]);

	let dot_in_reserve_on_pah_before =
		<AssetHubPolkadot as Chain>::account_data_of(sov_kah_on_pah.clone()).free;
	assert_eq!(dot_in_reserve_on_pah_before, prefund_amount);

	let sender_dot_before = foreign_balance_on_ah_kusama(dot_at_asset_hub_kusama, &sender);
	assert_eq!(sender_dot_before, prefund_amount);
	let receiver_dot_before = <AssetHubPolkadot as Chain>::account_data_of(receiver.clone()).free;

	// send back DOTs, use them for fees
	send_assets_over_bridge(|| {
		let destination = asset_hub_polkadot_location();
		let assets: Assets = (bridged_dot_at_ah_kusama(), amount_to_send).into();
		let fee_idx = 0;
		assert_ok!(send_assets_from_asset_hub_kusama(destination, assets, fee_idx));
	});

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				// DOT is withdrawn from KAH's SA on PAH
				RuntimeEvent::Balances(
					pallet_balances::Event::Burned { who, amount }
				) => {
					who: *who == sov_kah_on_pah,
					amount: *amount == amount_to_send,
				},
				// DOTs deposited to beneficiary
				RuntimeEvent::Balances(pallet_balances::Event::Minted { who, .. }) => {
					who: who == &receiver,
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_dot_after = foreign_balance_on_ah_kusama(dot_at_asset_hub_kusama, &sender);
	let receiver_dot_after = <AssetHubPolkadot as Chain>::account_data_of(receiver.clone()).free;
	let dot_in_reserve_on_pah_after =
		<AssetHubPolkadot as Chain>::account_data_of(sov_kah_on_pah).free;

	// Sender's balance is reduced
	assert!(sender_dot_before > sender_dot_after);
	// Receiver's balance is increased
	assert!(receiver_dot_after > receiver_dot_before);
	// Reserve balance is reduced by sent amount
	assert_eq!(dot_in_reserve_on_pah_after, dot_in_reserve_on_pah_before - amount_to_send);

	//////////////////////////////////////////////////////////////////
	// Now let's send back over USDTs + wETH (and pay fees with USDT)
	//////////////////////////////////////////////////////////////////

	// wETH has same relative location on both Polkadot and Kusama AssetHubs
	let bridged_weth_at_ah = v3::Location::try_from(weth_at_asset_hubs()).unwrap();
	let bridged_usdt_at_asset_hub_kusama =
		v3::Location::try_from(bridged_usdt_at_ah_kusama()).unwrap();

	// set up destination chain AH Polkadot:
	// create a DOT/USDT pool to be able to pay fees with USDT (USDT created in genesis)
	set_up_pool_with_dot_on_ah_polkadot(usdt_at_ah_polkadot().try_into().unwrap(), false);
	// create wETH on Polkadot (IRL it's already created by Snowbridge)
	create_foreign_on_ah_polkadot(bridged_weth_at_ah, true);
	// prefund KAH's sovereign account on PAH to be able to withdraw USDT and wETH from reserves
	let sov_kah_on_pah = AssetHubPolkadot::sovereign_account_of_parachain_on_other_global_consensus(
		Kusama,
		AssetHubKusama::para_id(),
	);
	AssetHubPolkadot::mint_asset(
		<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotAssetOwner::get()),
		USDT_ID,
		sov_kah_on_pah.clone(),
		amount_to_send * 2,
	);
	AssetHubPolkadot::mint_foreign_asset(
		<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadot::account_id_of(ALICE)),
		bridged_weth_at_ah,
		sov_kah_on_pah,
		amount_to_send * 2,
	);

	// set up source chain AH Kusama:
	// create wETH and USDT foreign assets on Kusama and prefund sender's account
	let prefund_accounts = vec![(sender.clone(), amount_to_send * 2)];
	create_foreign_on_ah_kusama(bridged_weth_at_ah, true, prefund_accounts.clone());
	create_foreign_on_ah_kusama(bridged_usdt_at_asset_hub_kusama, true, prefund_accounts);

	// check balances before
	let receiver_usdts_before = AssetHubPolkadot::execute_with(|| {
		type Assets = <AssetHubPolkadot as AssetHubPolkadotPallet>::Assets;
		<Assets as Inspect<_>>::balance(USDT_ID, &receiver)
	});
	let receiver_weth_before = foreign_balance_on_ah_polkadot(bridged_weth_at_ah, &receiver);

	let usdt_id: AssetId = Location::try_from(bridged_usdt_at_asset_hub_kusama).unwrap().into();
	// send USDTs and wETHs
	let assets: Assets = vec![
		(usdt_id.clone(), amount_to_send).into(),
		(Location::try_from(bridged_weth_at_ah).unwrap(), amount_to_send).into(),
	]
	.into();
	// use USDT for fees
	let fee = usdt_id;

	// use the more involved transfer extrinsic
	let custom_xcm_on_dest = Xcm::<()>(vec![DepositAsset {
		assets: Wild(AllCounted(assets.len() as u32)),
		beneficiary: AccountId32Junction { network: None, id: receiver.clone().into() }.into(),
	}]);
	assert_ok!(AssetHubKusama::execute_with(|| {
		<AssetHubKusama as AssetHubKusamaPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(sender),
			bx!(asset_hub_polkadot_location().into()),
			bx!(assets.into()),
			bx!(TransferType::DestinationReserve),
			bx!(fee.into()),
			bx!(TransferType::DestinationReserve),
			bx!(VersionedXcm::from(custom_xcm_on_dest)),
			WeightLimit::Unlimited,
		)
	}));
	// verify hops (also advances the message through the hops)
	assert_bridge_hub_kusama_message_accepted(true);
	assert_bridge_hub_polkadot_message_received();
	AssetHubPolkadot::execute_with(|| {
		AssetHubPolkadot::assert_xcmp_queue_success(None);
	});

	let receiver_usdts_after = AssetHubPolkadot::execute_with(|| {
		type Assets = <AssetHubPolkadot as AssetHubPolkadotPallet>::Assets;
		<Assets as Inspect<_>>::balance(USDT_ID, &receiver)
	});
	let receiver_weth_after = foreign_balance_on_ah_polkadot(bridged_weth_at_ah, &receiver);

	// Receiver's USDT balance is increased by almost `amount_to_send` (minus fees)
	assert!(receiver_usdts_after > receiver_usdts_before);
	assert!(receiver_usdts_after < receiver_usdts_before + amount_to_send);
	// Receiver's wETH balance is increased by `amount_to_send`
	assert_eq!(receiver_weth_after, receiver_weth_before + amount_to_send);
}

#[test]
fn send_ksm_from_penpal_kusama_through_asset_hub_kusama_to_asset_hub_polkadot() {
	let amount = ASSET_HUB_KUSAMA_ED * 10_000_000;
	let sender = PenpalASender::get();
	let receiver = AssetHubPolkadotReceiver::get();
	let local_asset_hub = PenpalA::sibling_location_of(AssetHubKusama::para_id());
	let (ksm_at_kusama_parachains, ksm_at_asset_hub_polkadot) =
		set_up_ksm_for_penpal_kusama_through_kah_to_pah(&sender, amount);

	let sov_pah_on_kah = AssetHubKusama::sovereign_account_of_parachain_on_other_global_consensus(
		Polkadot,
		AssetHubPolkadot::para_id(),
	);
	let ksm_in_reserve_on_kah_before =
		<AssetHubKusama as Chain>::account_data_of(sov_pah_on_kah.clone()).free;
	let sender_ksm_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(ksm_at_kusama_parachains.clone(), &sender)
	});
	let receiver_ksm_before = foreign_balance_on_ah_polkadot(ksm_at_asset_hub_polkadot, &receiver);

	// Send KSMs over bridge
	{
		let destination = asset_hub_polkadot_location();
		let assets: Assets = (ksm_at_kusama_parachains.clone(), amount).into();
		let asset_transfer_type = TransferType::RemoteReserve(local_asset_hub.clone().into());
		let fees_id: AssetId = ksm_at_kusama_parachains.clone().into();
		let fees_transfer_type = TransferType::RemoteReserve(local_asset_hub.into());
		let beneficiary: Location =
			AccountId32Junction { network: None, id: receiver.clone().into() }.into();
		let custom_xcm_on_dest = Xcm::<()>(vec![DepositAsset {
			assets: Wild(AllCounted(assets.len() as u32)),
			beneficiary,
		}]);
		send_assets_from_penpal_kusama_through_kusama_ah_to_polkadot_ah(
			destination,
			(assets, asset_transfer_type),
			(fees_id, fees_transfer_type),
			custom_xcm_on_dest,
		);
	}

	// process PAH incoming message and check events
	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				// issue KSMs on PAH
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == v3::Location::try_from(ksm_at_kusama_parachains.clone()).unwrap(),
					owner: owner == &receiver,
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_ksm_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(ksm_at_kusama_parachains, &sender)
	});
	let receiver_ksm_after = foreign_balance_on_ah_polkadot(ksm_at_asset_hub_polkadot, &receiver);
	let ksm_in_reserve_on_kah_after =
		<AssetHubKusama as Chain>::account_data_of(sov_pah_on_kah.clone()).free;

	// Sender's balance is reduced
	assert!(sender_ksm_after < sender_ksm_before);
	// Receiver's balance is increased
	assert!(receiver_ksm_after > receiver_ksm_before);
	// Reserve balance is increased by sent amount (less fess)
	assert!(ksm_in_reserve_on_kah_after > ksm_in_reserve_on_kah_before);
	assert!(ksm_in_reserve_on_kah_after <= ksm_in_reserve_on_kah_before + amount);
}

#[test]
fn send_back_dot_from_penpal_kusama_through_asset_hub_kusama_to_asset_hub_polkadot() {
	let dot_at_kusama_parachains_latest = bridged_dot_at_ah_kusama();
	let dot_at_kusama_parachains =
		v3::Location::try_from(dot_at_kusama_parachains_latest.clone()).unwrap();
	let amount = ASSET_HUB_KUSAMA_ED * 10_000_000;
	let sender = PenpalASender::get();
	let receiver = AssetHubPolkadotReceiver::get();

	// set up KSMs for transfer
	let (ksm_at_kusama_parachains, _) =
		set_up_ksm_for_penpal_kusama_through_kah_to_pah(&sender, amount);

	// set up DOTs for transfer
	let penpal_location = AssetHubKusama::sibling_location_of(PenpalA::para_id());
	let sov_penpal_on_kah = AssetHubKusama::sovereign_account_id_of(penpal_location);
	let prefund_accounts = vec![(sov_penpal_on_kah, amount * 2)];
	create_foreign_on_ah_kusama(dot_at_kusama_parachains, true, prefund_accounts);
	let asset_owner: AccountId = AssetHubKusama::account_id_of(ALICE);
	PenpalA::force_create_foreign_asset(
		dot_at_kusama_parachains_latest.clone(),
		asset_owner.clone(),
		true,
		ASSET_MIN_BALANCE,
		vec![(sender.clone(), amount * 2)],
	);

	// fund the KAH's SA on PAH with the DOT tokens held in reserve
	let sov_kah_on_pah = AssetHubPolkadot::sovereign_account_of_parachain_on_other_global_consensus(
		NetworkId::Kusama,
		AssetHubKusama::para_id(),
	);
	AssetHubPolkadot::fund_accounts(vec![(sov_kah_on_pah.clone(), amount * 2)]);

	// balances before
	let sender_dot_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(dot_at_kusama_parachains_latest.clone(), &sender)
	});
	let receiver_dot_before = <AssetHubPolkadot as Chain>::account_data_of(receiver.clone()).free;

	// send DOTs over the bridge, KSMs only used to pay fees on local AH, pay with DOT on remote AH
	{
		let final_destination = asset_hub_polkadot_location();
		let intermediary_hop = PenpalA::sibling_location_of(AssetHubKusama::para_id());
		let context = PenpalA::execute_with(PenpalUniversalLocation::get);

		// what happens at final destination
		let beneficiary = AccountId32Junction { network: None, id: receiver.clone().into() }.into();
		// use DOT as fees on the final destination (PAH)
		let remote_fees: Asset = (dot_at_kusama_parachains_latest.clone(), amount).into();
		let remote_fees = remote_fees.reanchored(&final_destination, &context).unwrap();
		// buy execution using DOTs, then deposit all remaining DOTs
		let xcm_on_final_dest = Xcm::<()>(vec![
			BuyExecution { fees: remote_fees, weight_limit: WeightLimit::Unlimited },
			DepositAsset { assets: Wild(AllCounted(1)), beneficiary },
		]);

		// what happens at intermediary hop
		// reanchor final dest (Asset Hub Polkadot) to the view of hop (Asset Hub Kusama)
		let mut final_destination = final_destination.clone();
		final_destination.reanchor(&intermediary_hop, &context).unwrap();
		// reanchor DOTs to the view of hop (Asset Hub Kusama)
		let asset: Asset = (dot_at_kusama_parachains_latest.clone(), amount).into();
		let asset = asset.reanchored(&intermediary_hop, &context).unwrap();
		// on Asset Hub Kusama, forward a request to withdraw DOTs from reserve on Asset Hub
		// Polkadot
		let xcm_on_hop = Xcm::<()>(vec![InitiateReserveWithdraw {
			assets: Definite(asset.into()), // DOTs
			reserve: final_destination,     // PAH
			xcm: xcm_on_final_dest,         // XCM to execute on PAH
		}]);
		// assets to send from Penpal and how they reach the intermediary hop
		let assets: Assets = vec![
			(dot_at_kusama_parachains_latest.clone(), amount).into(),
			(ksm_at_kusama_parachains.clone(), amount).into(),
		]
		.into();
		let asset_transfer_type = TransferType::DestinationReserve;
		let fees_id: AssetId = ksm_at_kusama_parachains.into();
		let fees_transfer_type = TransferType::DestinationReserve;

		// initiate the transfer
		send_assets_from_penpal_kusama_through_kusama_ah_to_polkadot_ah(
			intermediary_hop,
			(assets, asset_transfer_type),
			(fees_id, fees_transfer_type),
			xcm_on_hop,
		);
	}

	// process PAH incoming message and check events
	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				// issue KSMs on PAH
				RuntimeEvent::Balances(pallet_balances::Event::Issued { .. }) => {},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_dot_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(dot_at_kusama_parachains_latest, &sender)
	});
	let receiver_dot_after = <AssetHubPolkadot as Chain>::account_data_of(receiver).free;

	// Sender's balance is reduced by sent "amount"
	assert_eq!(sender_dot_after, sender_dot_before - amount);
	// Receiver's balance is increased by no more than "amount"
	assert!(receiver_dot_after > receiver_dot_before);
	assert!(receiver_dot_after <= receiver_dot_before + amount);
}
