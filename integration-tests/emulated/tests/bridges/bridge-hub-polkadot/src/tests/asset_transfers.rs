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
use bp_bridge_hub_polkadot::snowbridge::EthereumNetwork;
use snowbridge_router_primitives::inbound::EthereumLocationsConverterFor;
use xcm_executor::traits::ConvertLocation;

fn send_assets_over_bridge<F: FnOnce()>(send_fn: F) {
	// fund the PAH's SA on PBH for paying bridge transport fees
	BridgeHubPolkadot::fund_para_sovereign(AssetHubPolkadot::para_id(), 10_000_000_000_000u128);

	// set XCM versions
	let local_asset_hub = PenpalB::sibling_location_of(AssetHubPolkadot::para_id());
	PenpalB::force_xcm_version(local_asset_hub.clone(), XCM_VERSION);
	AssetHubPolkadot::force_xcm_version(asset_hub_kusama_location(), XCM_VERSION);
	BridgeHubPolkadot::force_xcm_version(bridge_hub_kusama_location(), XCM_VERSION);

	// send message over bridge
	send_fn();

	// process and verify intermediary hops
	assert_bridge_hub_polkadot_message_accepted(true);
	assert_bridge_hub_kusama_message_received();
}

fn set_up_dot_for_penpal_polkadot_through_pah_to_kah(
	sender: &AccountId,
	amount: u128,
) -> (xcm::v4::Location, xcm::v5::Location, xcm::v4::Location, xcm::v5::Location) {
	let dot_at_polkadot_parachains = dot_at_ah_polkadot();
	let dot_at_polkadot_parachains_latest: Location =
		dot_at_polkadot_parachains.clone().try_into().unwrap();
	let dot_at_asset_hub_kusama = bridged_dot_at_ah_kusama();
	let dot_at_asset_hub_kusama_latest: Location =
		dot_at_asset_hub_kusama.clone().try_into().unwrap();
	create_foreign_on_ah_kusama(dot_at_asset_hub_kusama.clone(), true);

	let penpal_location = AssetHubPolkadot::sibling_location_of(PenpalB::para_id());
	let sov_penpal_on_pah = AssetHubPolkadot::sovereign_account_id_of(penpal_location);
	// fund Penpal's sovereign account on AssetHub
	AssetHubPolkadot::fund_accounts(vec![(sov_penpal_on_pah, amount * 2)]);
	// fund Penpal's sender account
	PenpalB::mint_foreign_asset(
		<PenpalB as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		dot_at_polkadot_parachains_latest.clone(),
		sender.clone(),
		amount * 2,
	);
	(
		dot_at_polkadot_parachains,
		dot_at_polkadot_parachains_latest,
		dot_at_asset_hub_kusama,
		dot_at_asset_hub_kusama_latest,
	)
}

fn send_assets_from_polkadot_chain_through_polkadot_ah_to_kusama_ah<F: FnOnce()>(send_fn: F) {
	send_assets_over_bridge(|| {
		let sov_kah_on_pah =
			AssetHubPolkadot::sovereign_account_of_parachain_on_other_global_consensus(
				Kusama,
				AssetHubKusama::para_id(),
			);
		// call transfer extrinsic on sender chain
		send_fn();
		// verify intermediary AH Polkadot hop
		AssetHubPolkadot::execute_with(|| {
			type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
			assert_expected_events!(
				AssetHubPolkadot,
				vec![
					// Amount deposited in KAH's sovereign account
					RuntimeEvent::Balances(pallet_balances::Event::Minted { who, .. }) => {
						who: *who == sov_kah_on_pah.clone(),
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
/// Test transfer of DOT, USDT and wETH from AssetHub Polkadot to AssetHub Kusama.
///
/// This mix of assets should cover the whole range:
/// - native assets: DOT,
/// - trust-based assets: USDT (exists only on Polkadot, Kusama gets it from Polkadot over bridge),
/// - foreign asset / bridged asset (other bridge / Snowfork): wETH (bridged from Ethereum to
///   Polkadot over Snowbridge, then bridged over to Kusama through this bridge).
fn send_dot_usdt_and_weth_from_asset_hub_polkadot_to_asset_hub_kusama() {
	let amount = ASSET_HUB_POLKADOT_ED * 1_000;
	let sender = AssetHubPolkadotSender::get();
	let receiver = AssetHubKusamaReceiver::get();
	let dot_at_asset_hub_polkadot_latest: Location = dot_at_ah_polkadot().try_into().unwrap();
	let bridged_dot_at_asset_hub_kusama = bridged_dot_at_ah_kusama();

	create_foreign_on_ah_kusama(bridged_dot_at_asset_hub_kusama.clone(), true);
	set_up_pool_with_ksm_on_ah_kusama(bridged_dot_at_asset_hub_kusama.clone(), true);

	////////////////////////////////////////////////////////////
	// Let's first send over just some DOTs as a simple example
	////////////////////////////////////////////////////////////
	let sov_kah_on_pah = AssetHubPolkadot::sovereign_account_of_parachain_on_other_global_consensus(
		Kusama,
		AssetHubKusama::para_id(),
	);
	let dot_in_reserve_on_pah_before =
		<AssetHubPolkadot as Chain>::account_data_of(sov_kah_on_pah.clone()).free;
	let sender_dot_before = <AssetHubPolkadot as Chain>::account_data_of(sender.clone()).free;
	let receiver_dot_before =
		foreign_balance_on_ah_kusama(bridged_dot_at_asset_hub_kusama.clone(), &receiver);

	// send DOTs, use them for fees
	send_assets_over_bridge(|| {
		let destination = asset_hub_kusama_location();
		let assets: Assets = (dot_at_asset_hub_polkadot_latest, amount).into();
		let fee_idx = 0;
		assert_ok!(send_assets_from_asset_hub_polkadot(destination, assets, fee_idx));
	});

	// verify expected events on final destination
	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubKusama,
			vec![
				// issue DOTs on KAH
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == bridged_dot_at_asset_hub_kusama,
					owner: *owner == receiver,
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_dot_after = <AssetHubPolkadot as Chain>::account_data_of(sender.clone()).free;
	let receiver_dot_after =
		foreign_balance_on_ah_kusama(bridged_dot_at_asset_hub_kusama, &receiver);
	let dot_in_reserve_on_pah_after =
		<AssetHubPolkadot as Chain>::account_data_of(sov_kah_on_pah.clone()).free;

	// Sender's balance is reduced
	assert!(sender_dot_before > sender_dot_after);
	// Receiver's balance is increased
	assert!(receiver_dot_after > receiver_dot_before);
	// Reserve balance is increased by sent amount
	assert_eq!(dot_in_reserve_on_pah_after, dot_in_reserve_on_pah_before + amount);

	/////////////////////////////////////////////////////////////
	// Now let's send over USDTs + wETH (and pay fees with USDT)
	/////////////////////////////////////////////////////////////
	let usdt_at_asset_hub_polkadot = usdt_at_ah_polkadot();
	let usdt_at_asset_hub_polkadot_latest: Location =
		usdt_at_asset_hub_polkadot.clone().try_into().unwrap();
	let bridged_usdt_at_asset_hub_kusama = bridged_usdt_at_ah_kusama();
	// wETH has same relative location on both Polkadot and Kusama AssetHubs
	let bridged_weth_at_ah = weth_at_asset_hubs();
	let bridged_weth_at_ah_latest: Location = bridged_weth_at_ah.clone().try_into().unwrap();

	let snowbridge_sovereign: AccountId =
		EthereumLocationsConverterFor::<[u8; 32]>::convert_location(&Location::new(
			2,
			[GlobalConsensus(EthereumNetwork::get())],
		))
		.unwrap()
		.into();

	// mint USDT in sender's account (USDT already created in genesis)
	AssetHubPolkadot::mint_asset(
		<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotAssetOwner::get()),
		USDT_ID,
		sender.clone(),
		amount * 2,
	);
	AssetHubPolkadot::mint_foreign_asset(
		<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(snowbridge_sovereign),
		bridged_weth_at_ah.clone(),
		sender.clone(),
		MIN_ETHER_BALANCE,
	);
	create_foreign_on_ah_kusama(bridged_usdt_at_asset_hub_kusama.clone(), true);
	set_up_pool_with_ksm_on_ah_kusama(bridged_usdt_at_asset_hub_kusama.clone(), true);

	let receiver_usdts_before =
		foreign_balance_on_ah_kusama(bridged_usdt_at_asset_hub_kusama.clone(), &receiver);
	let receiver_weth_before = foreign_balance_on_ah_kusama(bridged_weth_at_ah.clone(), &receiver);

	// send USDTs and wETHs
	let assets: Assets = vec![
		(usdt_at_asset_hub_polkadot_latest.clone(), amount).into(),
		(bridged_weth_at_ah_latest.clone(), MIN_ETHER_BALANCE).into(),
	]
	.into();
	// use USDT for fees
	let fee: AssetId = usdt_at_asset_hub_polkadot_latest.into();

	// use the more involved transfer extrinsic
	let custom_xcm_on_dest = Xcm::<()>(vec![DepositAsset {
		assets: Wild(AllCounted(assets.len() as u32)),
		beneficiary: AccountId32Junction { network: None, id: receiver.clone().into() }.into(),
	}]);
	assert_ok!(AssetHubPolkadot::execute_with(|| {
		<AssetHubPolkadot as AssetHubPolkadotPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(sender),
			bx!(asset_hub_kusama_location().into()),
			bx!(assets.into()),
			bx!(TransferType::LocalReserve),
			bx!(fee.into()),
			bx!(TransferType::LocalReserve),
			bx!(VersionedXcm::from(custom_xcm_on_dest)),
			WeightLimit::Unlimited,
		)
	}));
	// verify hops (also advances the message through the hops)
	assert_bridge_hub_polkadot_message_accepted(true);
	assert_bridge_hub_kusama_message_received();
	AssetHubKusama::execute_with(|| {
		AssetHubKusama::assert_xcmp_queue_success(None);
	});

	let receiver_usdts_after =
		foreign_balance_on_ah_kusama(bridged_usdt_at_asset_hub_kusama, &receiver);
	let receiver_weth_after = foreign_balance_on_ah_kusama(bridged_weth_at_ah, &receiver);

	// Receiver's USDT balance is increased by almost `amount` (minus fees)
	assert!(receiver_usdts_after > receiver_usdts_before);
	assert!(receiver_usdts_after < receiver_usdts_before + amount);
	// Receiver's wETH balance is increased by sent amount
	assert_eq!(receiver_weth_after, receiver_weth_before + MIN_ETHER_BALANCE);
}

#[test]
/// Send bridged KSM "back" from AssetHub Polkadot to AssetHub Kusama.
fn send_back_ksm_from_asset_hub_polkadot_to_asset_hub_kusama() {
	let prefund_amount = 10_000_000_000_000u128;
	let amount_to_send = ASSET_HUB_KUSAMA_ED * 1_000;
	let sender = AssetHubPolkadotSender::get();
	let receiver = AssetHubKusamaReceiver::get();
	let bridged_ksm_at_asset_hub_polkadot = bridged_ksm_at_ah_polkadot();
	let bridged_ksm_at_asset_hub_polkadot_latest: Location =
		bridged_ksm_at_asset_hub_polkadot.clone().try_into().unwrap();
	let prefund_accounts = vec![(sender.clone(), prefund_amount)];
	create_foreign_on_ah_polkadot(
		bridged_ksm_at_asset_hub_polkadot.clone(),
		true,
		prefund_accounts,
	);

	// fund the PAH's SA on KAH with the KSM tokens held in reserve
	let sov_pah_on_kah = AssetHubKusama::sovereign_account_of_parachain_on_other_global_consensus(
		Polkadot,
		AssetHubPolkadot::para_id(),
	);
	AssetHubKusama::fund_accounts(vec![(sov_pah_on_kah.clone(), prefund_amount)]);

	let ksm_in_reserve_on_kah_before =
		<AssetHubKusama as Chain>::account_data_of(sov_pah_on_kah.clone()).free;
	assert_eq!(ksm_in_reserve_on_kah_before, prefund_amount);

	let sender_ksm_before =
		foreign_balance_on_ah_polkadot(bridged_ksm_at_asset_hub_polkadot.clone(), &sender);
	assert_eq!(sender_ksm_before, prefund_amount);
	let receiver_ksm_before = <AssetHubKusama as Chain>::account_data_of(receiver.clone()).free;

	// send back KSMs, use them for fees
	send_assets_over_bridge(|| {
		let destination = asset_hub_kusama_location();
		let assets: Assets =
			(bridged_ksm_at_asset_hub_polkadot_latest.clone(), amount_to_send).into();
		let fee_idx = 0;
		assert_ok!(send_assets_from_asset_hub_polkadot(destination, assets, fee_idx));
	});

	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubKusama,
			vec![
				// KSM is withdrawn from PAH's SA on KAH
				RuntimeEvent::Balances(
					pallet_balances::Event::Burned { who, amount }
				) => {
					who: *who == sov_pah_on_kah,
					amount: *amount == amount_to_send,
				},
				// KSMs deposited to beneficiary
				RuntimeEvent::Balances(pallet_balances::Event::Minted { who, .. }) => {
					who: *who == receiver,
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_ksm_after =
		foreign_balance_on_ah_polkadot(bridged_ksm_at_asset_hub_polkadot, &sender);
	let receiver_ksm_after = <AssetHubKusama as Chain>::account_data_of(receiver.clone()).free;
	let ksm_in_reserve_on_kah_after =
		<AssetHubKusama as Chain>::account_data_of(sov_pah_on_kah.clone()).free;

	// Sender's balance is reduced
	assert!(sender_ksm_before > sender_ksm_after);
	// Receiver's balance is increased
	assert!(receiver_ksm_after > receiver_ksm_before);
	// Reserve balance is reduced by sent amount
	assert_eq!(ksm_in_reserve_on_kah_after, ksm_in_reserve_on_kah_before - amount_to_send);
}

#[test]
/// Test transfer of DOT from Polkadot Relay through AssetHub Polkadot to AssetHub Kusama.
fn send_dot_from_polkadot_relay_through_asset_hub_polkadot_to_asset_hub_kusama() {
	let amount = POLKADOT_ED * 100;
	let sender = PolkadotSender::get();
	let receiver = AssetHubKusamaReceiver::get();
	let dot_at_polkadot: Location = Here.into();
	let bridged_dot_at_ah_kusama_latest = Location::try_from(bridged_dot_at_ah_kusama()).unwrap();
	let bridged_dot_at_ah_kusama = bridged_dot_at_ah_kusama();

	create_foreign_on_ah_kusama(bridged_dot_at_ah_kusama.clone(), true);
	set_up_pool_with_ksm_on_ah_kusama(bridged_dot_at_ah_kusama.clone(), true);

	let sov_ahk_on_ahp = AssetHubPolkadot::sovereign_account_of_parachain_on_other_global_consensus(
		Kusama,
		AssetHubKusama::para_id(),
	);
	let sender_dots_before = <Polkadot as Chain>::account_data_of(sender.clone()).free;
	let dots_in_reserve_on_ahp_before =
		<AssetHubPolkadot as Chain>::account_data_of(sov_ahk_on_ahp.clone()).free;
	let receiver_dots_before =
		foreign_balance_on_ah_kusama(bridged_dot_at_ah_kusama.clone(), &receiver);

	// send DOTs over the bridge, teleport to local AH, reserve deposit to remote AH
	{
		let final_destination = Location::new(
			1,
			[GlobalConsensus(Kusama), Parachain(AssetHubKusama::para_id().into())],
		);
		let intermediary_hop = Polkadot::child_location_of(AssetHubPolkadot::para_id());
		let context = Polkadot::execute_with(PolkadotRelayUniversalLocation::get);

		// what happens at final destination
		let beneficiary = AccountId32Junction { network: None, id: receiver.clone().into() }.into();
		// use DOT as fees on the final destination (KAH), only use half the amount as some
		// of it was already spent on intermediate hop (PAH)
		let remote_fees: Asset = (bridged_dot_at_ah_kusama_latest, amount / 2).into();
		// buy execution using DOTs, then deposit all unspent DOTs
		let xcm_on_final_dest = Xcm::<()>(vec![
			BuyExecution { fees: remote_fees, weight_limit: WeightLimit::Unlimited },
			DepositAsset { assets: Wild(AllCounted(1)), beneficiary },
		]);

		// what happens at intermediary hop
		// reanchor final dest (Asset Hub Kusama) to the view of hop (Asset Hub Polkadot)
		let mut final_destination = final_destination.clone();
		final_destination.reanchor(&intermediary_hop, &context).unwrap();
		// on Asset Hub Polkadot, forward a deposit reserve DOTs to Asset Hub Kusama
		let xcm_on_hop = Xcm::<()>(vec![DepositReserveAsset {
			assets: Wild(AllCounted(1)), // DOTs
			dest: final_destination,     // KAH
			xcm: xcm_on_final_dest,      // XCM to execute on KAH
		}]);
		// assets to send from Polkadot Relay and how they reach the intermediary hop
		let assets: Assets = vec![(dot_at_polkadot.clone(), amount).into()].into();
		let asset_transfer_type = TransferType::Teleport;
		let fees_id: AssetId = dot_at_polkadot.into();
		let fees_transfer_type = TransferType::Teleport;

		// initiate the transfer
		send_assets_from_polkadot_chain_through_polkadot_ah_to_kusama_ah(|| {
			// send message over bridge
			assert_ok!(Polkadot::execute_with(|| {
				let signed_origin = <Polkadot as Chain>::RuntimeOrigin::signed(sender.clone());
				<Polkadot as PolkadotPallet>::XcmPallet::transfer_assets_using_type_and_then(
					signed_origin,
					bx!(intermediary_hop.into()),
					bx!(assets.into()),
					bx!(asset_transfer_type),
					bx!(fees_id.into()),
					bx!(fees_transfer_type),
					bx!(VersionedXcm::from(xcm_on_hop)),
					WeightLimit::Unlimited,
				)
			}));
		});
	}

	// verify expected events on final destination
	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubKusama,
			vec![
				// issue DOTs on KAH
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == bridged_dot_at_ah_kusama,
					owner: *owner == receiver.clone(),
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_dots_after = <Polkadot as Chain>::account_data_of(sender.clone()).free;
	let receiver_dots_after = foreign_balance_on_ah_kusama(bridged_dot_at_ah_kusama, &receiver);
	let dots_in_reserve_on_ahp_after =
		<AssetHubPolkadot as Chain>::account_data_of(sov_ahk_on_ahp.clone()).free;

	// Sender's balance is reduced
	assert!(sender_dots_before > sender_dots_after);
	// Reserve balance on KAH increased
	assert!(dots_in_reserve_on_ahp_after > dots_in_reserve_on_ahp_before);
	// Receiver's balance is increased
	assert!(receiver_dots_after > receiver_dots_before);
}

#[test]
fn send_dot_from_penpal_polkadot_through_asset_hub_polkadot_to_asset_hub_kusama() {
	let amount = ASSET_HUB_POLKADOT_ED * 10_000_000;
	let sender = PenpalBSender::get();
	let receiver = AssetHubKusamaReceiver::get();
	let local_asset_hub = PenpalB::sibling_location_of(AssetHubPolkadot::para_id());
	let (dot_at_polkadot_parachains, dot_at_polkadot_parachains_latest, dot_at_asset_hub_kusama, _) =
		set_up_dot_for_penpal_polkadot_through_pah_to_kah(&sender, amount);

	let sov_kah_on_pah = AssetHubPolkadot::sovereign_account_of_parachain_on_other_global_consensus(
		Kusama,
		AssetHubKusama::para_id(),
	);
	let dot_in_reserve_on_pah_before =
		<AssetHubPolkadot as Chain>::account_data_of(sov_kah_on_pah.clone()).free;
	let sender_dot_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(dot_at_polkadot_parachains_latest.clone(), &sender)
	});
	let receiver_dot_before =
		foreign_balance_on_ah_kusama(dot_at_asset_hub_kusama.clone(), &receiver);

	set_up_pool_with_ksm_on_ah_kusama(dot_at_asset_hub_kusama.clone(), true);

	// Send DOTs over bridge
	{
		let destination = asset_hub_kusama_location();
		let assets: Assets = (dot_at_polkadot_parachains_latest.clone(), amount).into();
		let asset_transfer_type = TransferType::RemoteReserve(local_asset_hub.clone().into());
		let fees_id: AssetId = dot_at_polkadot_parachains_latest.clone().into();
		let fees_transfer_type = TransferType::RemoteReserve(local_asset_hub.into());
		let beneficiary: Location =
			AccountId32Junction { network: None, id: receiver.clone().into() }.into();
		let custom_xcm_on_dest = Xcm::<()>(vec![DepositAsset {
			assets: Wild(AllCounted(assets.len() as u32)),
			beneficiary,
		}]);
		send_assets_from_polkadot_chain_through_polkadot_ah_to_kusama_ah(|| {
			assert_ok!(PenpalB::execute_with(|| {
				let signed_origin = <PenpalB as Chain>::RuntimeOrigin::signed(sender.clone());
				<PenpalB as PenpalBPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
					signed_origin,
					bx!(destination.into()),
					bx!(assets.into()),
					bx!(asset_transfer_type),
					bx!(fees_id.into()),
					bx!(fees_transfer_type),
					bx!(VersionedXcm::from(custom_xcm_on_dest)),
					WeightLimit::Unlimited,
				)
			}));
		});
	}

	// process KAH incoming message and check events
	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubKusama,
			vec![
				// issue DOTs on KAH
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == dot_at_polkadot_parachains.clone(),
					owner: owner == &receiver,
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_dot_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(dot_at_polkadot_parachains_latest, &sender)
	});
	let receiver_dot_after = foreign_balance_on_ah_kusama(dot_at_asset_hub_kusama, &receiver);
	let dot_in_reserve_on_pah_after =
		<AssetHubPolkadot as Chain>::account_data_of(sov_kah_on_pah.clone()).free;

	// Sender's balance is reduced
	assert!(sender_dot_after < sender_dot_before);
	// Receiver's balance is increased
	assert!(receiver_dot_after > receiver_dot_before);
	// Reserve balance is increased by sent amount (less fess)
	assert!(dot_in_reserve_on_pah_after > dot_in_reserve_on_pah_before);
	assert!(dot_in_reserve_on_pah_after <= dot_in_reserve_on_pah_before + amount);
}

#[test]
fn send_back_ksm_from_penpal_polkadot_through_asset_hub_polkadot_to_asset_hub_kusama() {
	let ksm_at_polkadot_parachains = bridged_ksm_at_ah_polkadot();
	let ksm_at_polkadot_parachains_latest: Location =
		ksm_at_polkadot_parachains.clone().try_into().unwrap();
	let amount = ASSET_HUB_POLKADOT_ED * 10_000_000;
	let sender = PenpalBSender::get();
	let receiver = AssetHubKusamaReceiver::get();

	// set up DOTs for transfer
	let (_, dot_at_polkadot_parachains_latest, _, _) =
		set_up_dot_for_penpal_polkadot_through_pah_to_kah(&sender, amount);

	// set up KSMs for transfer
	let penpal_location = AssetHubPolkadot::sibling_location_of(PenpalB::para_id());
	let sov_penpal_on_kah = AssetHubPolkadot::sovereign_account_id_of(penpal_location);
	let prefund_accounts = vec![(sov_penpal_on_kah, amount * 2)];
	create_foreign_on_ah_polkadot(ksm_at_polkadot_parachains.clone(), true, prefund_accounts);
	let asset_owner: AccountId = AssetHubPolkadot::account_id_of(ALICE);
	PenpalB::force_create_foreign_asset(
		ksm_at_polkadot_parachains_latest.clone(),
		asset_owner.clone(),
		true,
		ASSET_MIN_BALANCE,
		vec![(sender.clone(), amount * 2)],
	);
	// Configure source Penpal chain to trust local AH as reserve of bridged KSM
	PenpalB::execute_with(|| {
		assert_ok!(<PenpalB as Chain>::System::set_storage(
			<PenpalB as Chain>::RuntimeOrigin::root(),
			vec![(
				PenpalCustomizableAssetFromSystemAssetHub::key().to_vec(),
				ksm_at_polkadot_parachains_latest.encode(),
			)],
		));
	});

	// fund the PAH's SA on KAH with the KSM tokens held in reserve
	let sov_pah_on_kah = AssetHubKusama::sovereign_account_of_parachain_on_other_global_consensus(
		Polkadot,
		AssetHubPolkadot::para_id(),
	);
	AssetHubKusama::fund_accounts(vec![(sov_pah_on_kah.clone(), amount * 2)]);

	// balances before
	let sender_ksm_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(ksm_at_polkadot_parachains_latest.clone(), &sender)
	});
	let receiver_ksm_before = <AssetHubKusama as Chain>::account_data_of(receiver.clone()).free;

	// send KSMs over the bridge, DOTs only used to pay fees on local AH, pay with KSM on remote AH
	{
		let final_destination = asset_hub_kusama_location();
		let intermediary_hop = PenpalB::sibling_location_of(AssetHubPolkadot::para_id());
		let context = PenpalB::execute_with(PenpalUniversalLocation::get);

		// what happens at final destination
		let beneficiary = AccountId32Junction { network: None, id: receiver.clone().into() }.into();
		// use KSM as fees on the final destination (PAH)
		let remote_fees: Asset = (ksm_at_polkadot_parachains_latest.clone(), amount).into();
		let remote_fees = remote_fees.reanchored(&final_destination, &context).unwrap();
		// buy execution using KSMs, then deposit all remaining KSMs
		let xcm_on_final_dest = Xcm::<()>(vec![
			BuyExecution { fees: remote_fees, weight_limit: WeightLimit::Unlimited },
			DepositAsset { assets: Wild(AllCounted(1)), beneficiary },
		]);

		// what happens at intermediary hop
		// reanchor final dest (Asset Hub Kusama) to the view of hop (Asset Hub Polkadot)
		let mut final_destination = final_destination.clone();
		final_destination.reanchor(&intermediary_hop, &context).unwrap();
		// reanchor KSMs to the view of hop (Asset Hub Polkadot)
		let asset: Asset = (ksm_at_polkadot_parachains_latest.clone(), amount).into();
		let asset = asset.reanchored(&intermediary_hop, &context).unwrap();
		// on Asset Hub Polkadot, forward a request to withdraw KSMs from reserve on Asset Hub
		// Kusama
		let xcm_on_hop = Xcm::<()>(vec![InitiateReserveWithdraw {
			assets: Definite(asset.into()), // KSMs
			reserve: final_destination,     // KAH
			xcm: xcm_on_final_dest,         // XCM to execute on KAH
		}]);
		// assets to send from Penpal and how they reach the intermediary hop
		let assets: Assets = vec![
			(ksm_at_polkadot_parachains_latest.clone(), amount).into(),
			(dot_at_polkadot_parachains_latest.clone(), amount).into(),
		]
		.into();
		let asset_transfer_type = TransferType::DestinationReserve;
		let fees_id: AssetId = dot_at_polkadot_parachains_latest.into();
		let fees_transfer_type = TransferType::DestinationReserve;

		// initiate the transfer
		send_assets_from_polkadot_chain_through_polkadot_ah_to_kusama_ah(|| {
			assert_ok!(PenpalB::execute_with(|| {
				let signed_origin = <PenpalB as Chain>::RuntimeOrigin::signed(sender.clone());
				<PenpalB as PenpalBPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
					signed_origin,
					bx!(intermediary_hop.into()),
					bx!(assets.into()),
					bx!(asset_transfer_type),
					bx!(fees_id.into()),
					bx!(fees_transfer_type),
					bx!(VersionedXcm::from(xcm_on_hop)),
					WeightLimit::Unlimited,
				)
			}));
		});
	}

	// process KAH incoming message and check events
	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubKusama,
			vec![
				// issue DOTs on KAH
				RuntimeEvent::Balances(pallet_balances::Event::Issued { .. }) => {},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_ksm_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(ksm_at_polkadot_parachains_latest, &sender)
	});
	let receiver_ksm_after = <AssetHubKusama as Chain>::account_data_of(receiver).free;

	// Sender's balance is reduced by sent "amount"
	assert_eq!(sender_ksm_after, sender_ksm_before - amount);
	// Receiver's balance is increased by no more than "amount"
	assert!(receiver_ksm_after > receiver_ksm_before);
	assert!(receiver_ksm_after <= receiver_ksm_before + amount);
}
