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

use super::reserve_transfer::*;
use crate::{
	foreign_balance_on,
	tests::teleport::do_bidirectional_teleport_foreign_assets_between_para_and_asset_hub_using_xt,
	*,
};
use emulated_integration_tests_common::USDT_ID;
use kusama_system_emulated_network::kusama_emulated_chain::kusama_runtime::Dmp;

fn para_to_para_assethub_hop_assertions(t: ParaToParaThroughAHTest) {
	type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
	let sov_penpal_a_on_ah = AssetHubKusama::sovereign_account_id_of(
		AssetHubKusama::sibling_location_of(PenpalA::para_id()),
	);
	let sov_penpal_b_on_ah = AssetHubKusama::sovereign_account_id_of(
		AssetHubKusama::sibling_location_of(PenpalB::para_id()),
	);

	assert_expected_events!(
		AssetHubKusama,
		vec![
			// Withdrawn from sender parachain SA
			RuntimeEvent::Balances(
				pallet_balances::Event::Burned { who, amount }
			) => {
				who: *who == sov_penpal_a_on_ah,
				amount: *amount == t.args.amount,
			},
			// Deposited to receiver parachain SA
			RuntimeEvent::Balances(
				pallet_balances::Event::Minted { who, .. }
			) => {
				who: *who == sov_penpal_b_on_ah,
			},
			RuntimeEvent::MessageQueue(
				pallet_message_queue::Event::Processed { success: true, .. }
			) => {},
		]
	);
}

fn ah_to_para_transfer_assets(t: SystemParaToParaTest) -> DispatchResult {
	let fee_idx = t.args.fee_asset_item as usize;
	let fee: Asset = t.args.assets.inner().get(fee_idx).cloned().unwrap();
	let custom_xcm_on_dest = Xcm::<()>(vec![DepositAsset {
		assets: Wild(AllCounted(t.args.assets.len() as u32)),
		beneficiary: t.args.beneficiary,
	}]);
	<AssetHubKusama as AssetHubKusamaPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.assets.into()),
		bx!(TransferType::LocalReserve),
		bx!(fee.id.into()),
		bx!(TransferType::LocalReserve),
		bx!(VersionedXcm::from(custom_xcm_on_dest)),
		t.args.weight_limit,
	)
}

fn para_to_ah_transfer_assets(t: ParaToSystemParaTest) -> DispatchResult {
	let fee_idx = t.args.fee_asset_item as usize;
	let fee: Asset = t.args.assets.inner().get(fee_idx).cloned().unwrap();
	let custom_xcm_on_dest = Xcm::<()>(vec![DepositAsset {
		assets: Wild(AllCounted(t.args.assets.len() as u32)),
		beneficiary: t.args.beneficiary,
	}]);
	<PenpalA as PenpalAPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.assets.into()),
		bx!(TransferType::DestinationReserve),
		bx!(fee.id.into()),
		bx!(TransferType::DestinationReserve),
		bx!(VersionedXcm::from(custom_xcm_on_dest)),
		t.args.weight_limit,
	)
}

fn para_to_para_transfer_assets_through_ah(t: ParaToParaThroughAHTest) -> DispatchResult {
	let fee_idx = t.args.fee_asset_item as usize;
	let fee: Asset = t.args.assets.inner().get(fee_idx).cloned().unwrap();
	let asset_hub_location: Location = PenpalA::sibling_location_of(AssetHubKusama::para_id());
	let custom_xcm_on_dest = Xcm::<()>(vec![DepositAsset {
		assets: Wild(AllCounted(t.args.assets.len() as u32)),
		beneficiary: t.args.beneficiary,
	}]);
	<PenpalA as PenpalAPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.assets.into()),
		bx!(TransferType::RemoteReserve(asset_hub_location.clone().into())),
		bx!(fee.id.into()),
		bx!(TransferType::RemoteReserve(asset_hub_location.into())),
		bx!(VersionedXcm::from(custom_xcm_on_dest)),
		t.args.weight_limit,
	)
}

fn para_to_asset_hub_teleport_foreign_assets(t: ParaToSystemParaTest) -> DispatchResult {
	let fee_idx = t.args.fee_asset_item as usize;
	let fee: Asset = t.args.assets.inner().get(fee_idx).cloned().unwrap();
	let custom_xcm_on_dest = Xcm::<()>(vec![DepositAsset {
		assets: Wild(AllCounted(t.args.assets.len() as u32)),
		beneficiary: t.args.beneficiary,
	}]);
	<PenpalA as PenpalAPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.assets.into()),
		bx!(TransferType::Teleport),
		bx!(fee.id.into()),
		bx!(TransferType::DestinationReserve),
		bx!(VersionedXcm::from(custom_xcm_on_dest)),
		t.args.weight_limit,
	)
}

fn asset_hub_to_para_teleport_foreign_assets(t: SystemParaToParaTest) -> DispatchResult {
	let fee_idx = t.args.fee_asset_item as usize;
	let fee: Asset = t.args.assets.inner().get(fee_idx).cloned().unwrap();
	let custom_xcm_on_dest = Xcm::<()>(vec![DepositAsset {
		assets: Wild(AllCounted(t.args.assets.len() as u32)),
		beneficiary: t.args.beneficiary,
	}]);
	<AssetHubKusama as AssetHubKusamaPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.assets.into()),
		bx!(TransferType::Teleport),
		bx!(fee.id.into()),
		bx!(TransferType::LocalReserve),
		bx!(VersionedXcm::from(custom_xcm_on_dest)),
		t.args.weight_limit,
	)
}

// ===========================================================================
// ======= Transfer - Native + Bridged Assets - AssetHub->Parachain ==========
// ===========================================================================
/// Transfers of native asset plus bridged asset from AssetHub to some Parachain
/// while paying fees using native asset.
#[test]
fn transfer_foreign_assets_from_asset_hub_to_para() {
	let destination = AssetHubKusama::sibling_location_of(PenpalA::para_id());
	let sender = AssetHubKusamaSender::get();
	let native_amount_to_send: Balance = ASSET_HUB_KUSAMA_ED * 10000;
	let native_asset_location = KsmLocation::get();
	let receiver = PenpalAReceiver::get();
	let assets_owner = PenpalAssetOwner::get();
	// Foreign asset used: bridged DOT
	let foreign_amount_to_send = ASSET_HUB_KUSAMA_ED * 10_000_000;
	let dot_at_kusama_parachains = Location::new(2, [GlobalConsensus(Polkadot)]);

	// Configure destination chain to trust AH as reserve of DOT
	PenpalA::execute_with(|| {
		assert_ok!(<PenpalA as Chain>::System::set_storage(
			<PenpalA as Chain>::RuntimeOrigin::root(),
			vec![(
				CustomizableAssetFromSystemAssetHub::key().to_vec(),
				Location::new(2, [GlobalConsensus(Polkadot)]).encode(),
			)],
		));
	});
	PenpalA::force_create_foreign_asset(
		dot_at_kusama_parachains.clone(),
		assets_owner.clone(),
		false,
		ASSET_MIN_BALANCE,
		vec![],
	);
	AssetHubKusama::force_create_foreign_asset(
		dot_at_kusama_parachains.clone(),
		assets_owner.clone(),
		false,
		ASSET_MIN_BALANCE,
		vec![],
	);
	AssetHubKusama::mint_foreign_asset(
		<AssetHubKusama as Chain>::RuntimeOrigin::signed(assets_owner),
		dot_at_kusama_parachains.clone(),
		sender.clone(),
		foreign_amount_to_send * 2,
	);

	let dot_at_kusama_parachains_latest: Location = dot_at_kusama_parachains.clone();
	// Assets to send
	let assets: Vec<Asset> = vec![
		(Parent, native_amount_to_send).into(),
		(dot_at_kusama_parachains_latest, foreign_amount_to_send).into(),
	];
	let fee_asset_id = AssetId(Parent.into());
	let fee_asset_item = assets.iter().position(|a| a.id == fee_asset_id).unwrap() as u32;

	// Init Test
	let test_args = TestContext {
		sender: sender.clone(),
		receiver: receiver.clone(),
		args: TestArgs::new_para(
			destination.clone(),
			receiver.clone(),
			native_amount_to_send,
			assets.into(),
			None,
			fee_asset_item,
		),
	};
	let mut test = SystemParaToParaTest::new(test_args);

	// Query initial balances
	let sender_balance_before = test.sender.balance;
	let sender_dots_before = AssetHubKusama::execute_with(|| {
		type ForeignAssets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(dot_at_kusama_parachains.clone(), &sender)
	});
	let receiver_assets_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(native_asset_location.clone(), &receiver)
	});
	let receiver_dots_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(dot_at_kusama_parachains.clone(), &receiver)
	});

	// Set assertions and dispatchables
	test.set_assertion::<AssetHubKusama>(system_para_to_para_sender_assertions);
	test.set_assertion::<PenpalA>(system_para_to_para_receiver_assertions);
	test.set_dispatchable::<AssetHubKusama>(ah_to_para_transfer_assets);
	test.assert();

	// Query final balances
	let sender_balance_after = test.sender.balance;
	let sender_dots_after = AssetHubKusama::execute_with(|| {
		type ForeignAssets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(dot_at_kusama_parachains.clone(), &sender)
	});
	let receiver_assets_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(native_asset_location, &receiver)
	});
	let receiver_dots_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(dot_at_kusama_parachains, &receiver)
	});

	// Sender's balance is reduced by amount sent plus delivery fees
	assert!(sender_balance_after < sender_balance_before - native_amount_to_send);
	// Sender's balance is reduced by foreign amount sent
	assert_eq!(sender_dots_after, sender_dots_before - foreign_amount_to_send);
	// Receiver's assets is increased
	assert!(receiver_assets_after > receiver_assets_before);
	// Receiver's assets increased by `amount_to_send - delivery_fees - bought_execution`;
	// `delivery_fees` might be paid from transfer or JIT, also `bought_execution` is unknown but
	// should be non-zero
	assert!(receiver_assets_after < receiver_assets_before + native_amount_to_send);
	// Receiver's balance is increased by foreign amount sent
	assert_eq!(receiver_dots_after, receiver_dots_before + foreign_amount_to_send);
}

/// Reserve Transfers of native asset from Parachain to System Parachain should work
// ===========================================================================
// ======= Transfer - Native + Bridged Assets - Parachain->AssetHub ==========
// ===========================================================================
/// Transfers of native asset plus bridged asset from some Parachain to AssetHub
/// while paying fees using native asset.
#[test]
fn transfer_foreign_assets_from_para_to_asset_hub() {
	// Init values for Parachain
	let destination = PenpalA::sibling_location_of(AssetHubKusama::para_id());
	let sender = PenpalASender::get();
	let native_amount_to_send: Balance = ASSET_HUB_KUSAMA_ED * 10000;
	let native_asset_location = KsmLocation::get();
	let assets_owner = PenpalAssetOwner::get();

	// Foreign asset used: bridged DOT
	let foreign_amount_to_send = ASSET_HUB_KUSAMA_ED * 10_000_000;
	let dot_at_kusama_parachains = Location::new(2, [GlobalConsensus(Polkadot)]);

	// Configure destination chain to trust AH as reserve of DOT
	PenpalA::execute_with(|| {
		assert_ok!(<PenpalA as Chain>::System::set_storage(
			<PenpalA as Chain>::RuntimeOrigin::root(),
			vec![(
				CustomizableAssetFromSystemAssetHub::key().to_vec(),
				Location::new(2, [GlobalConsensus(Polkadot)]).encode(),
			)],
		));
	});
	PenpalA::force_create_foreign_asset(
		dot_at_kusama_parachains.clone(),
		assets_owner.clone(),
		false,
		ASSET_MIN_BALANCE,
		vec![],
	);
	AssetHubKusama::force_create_foreign_asset(
		dot_at_kusama_parachains.clone(),
		assets_owner.clone(),
		false,
		ASSET_MIN_BALANCE,
		vec![],
	);

	// fund Parachain's sender account
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(assets_owner.clone()),
		native_asset_location.clone(),
		sender.clone(),
		native_amount_to_send * 2,
	);
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(assets_owner.clone()),
		dot_at_kusama_parachains.clone(),
		sender.clone(),
		foreign_amount_to_send * 2,
	);

	// Init values for System Parachain
	let receiver = AssetHubKusamaReceiver::get();
	let penpal_location_as_seen_by_ahk = AssetHubKusama::sibling_location_of(PenpalA::para_id());
	let sov_penpal_on_ahk = AssetHubKusama::sovereign_account_id_of(penpal_location_as_seen_by_ahk);

	// fund Parachain's SA on AssetHub with the assets held in reserve
	AssetHubKusama::fund_accounts(vec![(sov_penpal_on_ahk.clone(), native_amount_to_send * 2)]);
	AssetHubKusama::mint_foreign_asset(
		<AssetHubKusama as Chain>::RuntimeOrigin::signed(assets_owner),
		dot_at_kusama_parachains.clone(),
		sov_penpal_on_ahk,
		foreign_amount_to_send * 2,
	);

	let dot_at_kusama_parachains_latest: Location = dot_at_kusama_parachains.clone();
	// Assets to send
	let assets: Vec<Asset> = vec![
		(Parent, native_amount_to_send).into(),
		(dot_at_kusama_parachains_latest.clone(), foreign_amount_to_send).into(),
	];
	let fee_asset_id = AssetId(Parent.into());
	let fee_asset_item = assets.iter().position(|a| a.id == fee_asset_id).unwrap() as u32;

	// Init Test
	let test_args = TestContext {
		sender: sender.clone(),
		receiver: receiver.clone(),
		args: TestArgs::new_para(
			destination.clone(),
			receiver.clone(),
			native_amount_to_send,
			assets.into(),
			None,
			fee_asset_item,
		),
	};
	let mut test = ParaToSystemParaTest::new(test_args);

	// Query initial balances
	let sender_native_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(native_asset_location.clone(), &sender)
	});
	let sender_dots_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(dot_at_kusama_parachains_latest.clone(), &sender)
	});
	let receiver_native_before = test.receiver.balance;
	let receiver_dots_before = AssetHubKusama::execute_with(|| {
		type ForeignAssets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(dot_at_kusama_parachains.clone(), &receiver)
	});

	// Set assertions and dispatchables
	test.set_assertion::<PenpalA>(para_to_system_para_sender_assertions);
	test.set_assertion::<AssetHubKusama>(para_to_system_para_receiver_assertions);
	test.set_dispatchable::<PenpalA>(para_to_ah_transfer_assets);
	test.assert();

	// Query final balances
	let sender_native_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(native_asset_location, &sender)
	});
	let sender_dots_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(dot_at_kusama_parachains_latest.clone(), &sender)
	});
	let receiver_native_after = test.receiver.balance;
	let receiver_dots_after = AssetHubKusama::execute_with(|| {
		type ForeignAssets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(dot_at_kusama_parachains, &receiver)
	});

	// Sender's balance is reduced by amount sent plus delivery fees
	assert!(sender_native_after < sender_native_before - native_amount_to_send);
	// Sender's balance is reduced by foreign amount sent
	assert_eq!(sender_dots_after, sender_dots_before - foreign_amount_to_send);
	// Receiver's balance is increased
	assert!(receiver_native_after > receiver_native_before);
	// Receiver's balance increased by `amount_to_send - delivery_fees - bought_execution`;
	// `delivery_fees` might be paid from transfer or JIT, also `bought_execution` is unknown but
	// should be non-zero
	assert!(receiver_native_after < receiver_native_before + native_amount_to_send);
	// Receiver's balance is increased by foreign amount sent
	assert_eq!(receiver_dots_after, receiver_dots_before + foreign_amount_to_send);
}

// ==============================================================================
// ===== Transfer - Native + Bridged Assets - Parachain->AssetHub->Parachain ====
// ==============================================================================
/// Transfers of native asset plus bridged asset from Parachain to Parachain
/// (through AssetHub reserve) with fees paid using native asset.
#[test]
fn transfer_foreign_assets_from_para_to_para_through_asset_hub() {
	// Init values for Parachain Origin
	let destination = PenpalA::sibling_location_of(PenpalB::para_id());
	let sender = PenpalASender::get();
	let ksm_to_send: Balance = KUSAMA_ED * 10000;
	let assets_owner = PenpalAssetOwner::get();
	let ksm_location = KsmLocation::get();
	let sender_as_seen_by_ah = AssetHubKusama::sibling_location_of(PenpalA::para_id());
	let sov_of_sender_on_ah = AssetHubKusama::sovereign_account_id_of(sender_as_seen_by_ah);
	let receiver_as_seen_by_ah = AssetHubKusama::sibling_location_of(PenpalB::para_id());
	let sov_of_receiver_on_ah = AssetHubKusama::sovereign_account_id_of(receiver_as_seen_by_ah);
	let dot_to_send = ASSET_HUB_KUSAMA_ED * 10_000_000;

	// Configure source and destination chains to trust AH as reserve of DOT
	PenpalA::execute_with(|| {
		assert_ok!(<PenpalA as Chain>::System::set_storage(
			<PenpalA as Chain>::RuntimeOrigin::root(),
			vec![(
				CustomizableAssetFromSystemAssetHub::key().to_vec(),
				Location::new(2, [GlobalConsensus(Polkadot)]).encode(),
			)],
		));
	});
	PenpalB::execute_with(|| {
		assert_ok!(<PenpalB as Chain>::System::set_storage(
			<PenpalB as Chain>::RuntimeOrigin::root(),
			vec![(
				CustomizableAssetFromSystemAssetHub::key().to_vec(),
				Location::new(2, [GlobalConsensus(Polkadot)]).encode(),
			)],
		));
	});

	// Register DOT as foreign asset and transfer it around the Kusama ecosystem
	let dot_at_kusama_parachains = Location::new(2, [GlobalConsensus(Polkadot)]);
	AssetHubKusama::force_create_foreign_asset(
		dot_at_kusama_parachains.clone(),
		assets_owner.clone(),
		false,
		ASSET_MIN_BALANCE,
		vec![],
	);
	PenpalA::force_create_foreign_asset(
		dot_at_kusama_parachains.clone(),
		assets_owner.clone(),
		false,
		ASSET_MIN_BALANCE,
		vec![],
	);
	PenpalB::force_create_foreign_asset(
		dot_at_kusama_parachains.clone(),
		assets_owner.clone(),
		false,
		ASSET_MIN_BALANCE,
		vec![],
	);

	// fund Parachain's sender account
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(assets_owner.clone()),
		ksm_location.clone(),
		sender.clone(),
		ksm_to_send * 2,
	);
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(assets_owner.clone()),
		dot_at_kusama_parachains.clone(),
		sender.clone(),
		dot_to_send * 2,
	);
	// fund the Parachain Origin's SA on Asset Hub with the assets held in reserve
	AssetHubKusama::fund_accounts(vec![(sov_of_sender_on_ah.clone(), ksm_to_send * 2)]);
	AssetHubKusama::mint_foreign_asset(
		<AssetHubKusama as Chain>::RuntimeOrigin::signed(assets_owner),
		dot_at_kusama_parachains.clone(),
		sov_of_sender_on_ah.clone(),
		dot_to_send * 2,
	);

	// Init values for Parachain Destination
	let receiver = PenpalBReceiver::get();

	let dot_at_kusama_parachains_latest: Location = dot_at_kusama_parachains.clone();
	// Assets to send
	let assets: Vec<Asset> = vec![
		(ksm_location.clone(), ksm_to_send).into(),
		(dot_at_kusama_parachains_latest.clone(), dot_to_send).into(),
	];
	let fee_asset_id: AssetId = ksm_location.clone().into();
	let fee_asset_item = assets.iter().position(|a| a.id == fee_asset_id).unwrap() as u32;

	// Init Test
	let test_args = TestContext {
		sender: sender.clone(),
		receiver: receiver.clone(),
		args: TestArgs::new_para(
			destination,
			receiver.clone(),
			ksm_to_send,
			assets.into(),
			None,
			fee_asset_item,
		),
	};
	let mut test = ParaToParaThroughAHTest::new(test_args);

	// Query initial balances
	let sender_ksms_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(ksm_location.clone(), &sender)
	});
	let sender_dots_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(dot_at_kusama_parachains_latest.clone(), &sender)
	});
	let ksms_in_sender_reserve_on_ahk_before =
		<AssetHubKusama as Chain>::account_data_of(sov_of_sender_on_ah.clone()).free;
	let dots_in_sender_reserve_on_ahk_before = AssetHubKusama::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(dot_at_kusama_parachains.clone(), &sov_of_sender_on_ah)
	});
	let ksms_in_receiver_reserve_on_ahk_before =
		<AssetHubKusama as Chain>::account_data_of(sov_of_receiver_on_ah.clone()).free;
	let dots_in_receiver_reserve_on_ahk_before = AssetHubKusama::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(dot_at_kusama_parachains.clone(), &sov_of_receiver_on_ah)
	});
	let receiver_ksms_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(ksm_location.clone(), &receiver)
	});
	let receiver_dots_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(dot_at_kusama_parachains_latest.clone(), &receiver)
	});

	// Set assertions and dispatchables
	test.set_assertion::<PenpalA>(para_to_para_through_hop_sender_assertions);
	test.set_assertion::<AssetHubKusama>(para_to_para_assethub_hop_assertions);
	test.set_assertion::<PenpalB>(para_to_para_through_hop_receiver_assertions);
	test.set_dispatchable::<PenpalA>(para_to_para_transfer_assets_through_ah);
	test.assert();

	// Query final balances
	let sender_ksms_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(ksm_location.clone(), &sender)
	});
	let sender_dots_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(dot_at_kusama_parachains_latest.clone(), &sender)
	});
	let dots_in_sender_reserve_on_ahk_after = AssetHubKusama::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(dot_at_kusama_parachains.clone(), &sov_of_sender_on_ah)
	});
	let ksms_in_sender_reserve_on_ahk_after =
		<AssetHubKusama as Chain>::account_data_of(sov_of_sender_on_ah).free;
	let dots_in_receiver_reserve_on_ahk_after = AssetHubKusama::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(dot_at_kusama_parachains.clone(), &sov_of_receiver_on_ah)
	});
	let ksms_in_receiver_reserve_on_ahk_after =
		<AssetHubKusama as Chain>::account_data_of(sov_of_receiver_on_ah).free;
	let receiver_ksms_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(ksm_location, &receiver)
	});
	let receiver_dots_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(dot_at_kusama_parachains_latest, &receiver)
	});

	// Sender's balance is reduced by amount sent plus delivery fees
	assert!(sender_ksms_after < sender_ksms_before - ksm_to_send);
	assert_eq!(sender_dots_after, sender_dots_before - dot_to_send);
	// Sovereign accounts on reserve are changed accordingly
	assert_eq!(
		ksms_in_sender_reserve_on_ahk_after,
		ksms_in_sender_reserve_on_ahk_before - ksm_to_send
	);
	assert_eq!(
		dots_in_sender_reserve_on_ahk_after,
		dots_in_sender_reserve_on_ahk_before - dot_to_send
	);
	assert!(ksms_in_receiver_reserve_on_ahk_after > ksms_in_receiver_reserve_on_ahk_before);
	assert_eq!(
		dots_in_receiver_reserve_on_ahk_after,
		dots_in_receiver_reserve_on_ahk_before + dot_to_send
	);
	// Receiver's balance is increased
	assert!(receiver_ksms_after > receiver_ksms_before);
	assert_eq!(receiver_dots_after, receiver_dots_before + dot_to_send);
}

// ==============================================================================================
// ==== Bidirectional Transfer - Native + Teleportable Foreign Assets - Parachain<->AssetHub ====
// ==============================================================================================
/// Transfers of native asset plus teleportable foreign asset from Parachain to AssetHub and back
/// with fees paid using native asset.
#[test]
fn bidirectional_teleport_foreign_asset_between_para_and_asset_hub_using_explicit_transfer_types() {
	do_bidirectional_teleport_foreign_assets_between_para_and_asset_hub_using_xt(
		para_to_asset_hub_teleport_foreign_assets,
		asset_hub_to_para_teleport_foreign_assets,
	);
}

// ===============================================================
// ===== Transfer - Native Asset - Relay->AssetHub->Parachain ====
// ===============================================================
/// Transfers of native asset Relay to Parachain (using AssetHub reserve). Parachains want to avoid
/// managing SAs on all system chains, thus want all their DOT-in-reserve to be held in their
/// Sovereign Account on Asset Hub.
#[test]
fn transfer_native_asset_from_relay_to_para_through_asset_hub() {
	// Init values for Relay
	let destination = Kusama::child_location_of(PenpalA::para_id());
	let sender = KusamaSender::get();
	let amount_to_send: Balance = KUSAMA_ED * 1000;

	// Init values for Parachain
	let relay_native_asset_location = KsmLocation::get();
	let receiver = PenpalAReceiver::get();

	// Init Test
	let test_args = TestContext {
		sender,
		receiver: receiver.clone(),
		args: TestArgs::new_relay(destination.clone(), receiver.clone(), amount_to_send),
	};
	let mut test = RelayToParaThroughAHTest::new(test_args);

	let sov_penpal_on_ah = AssetHubKusama::sovereign_account_id_of(
		AssetHubKusama::sibling_location_of(PenpalA::para_id()),
	);
	// Query initial balances
	let sender_balance_before = test.sender.balance;
	let sov_penpal_on_ah_before = AssetHubKusama::execute_with(|| {
		<AssetHubKusama as AssetHubKusamaPallet>::Balances::free_balance(sov_penpal_on_ah.clone())
	});
	let receiver_assets_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location.clone(), &receiver)
	});

	fn relay_assertions(t: RelayToParaThroughAHTest) {
		type RuntimeEvent = <Kusama as Chain>::RuntimeEvent;
		Kusama::assert_xcm_pallet_attempted_complete(None);
		assert_expected_events!(
			Kusama,
			vec![
				// Amount to teleport is withdrawn from Sender
				RuntimeEvent::Balances(pallet_balances::Event::Burned { who, amount }) => {
					who: *who == t.sender.account_id,
					amount: *amount == t.args.amount,
				},
				// Amount to teleport is deposited in Relay's `CheckAccount`
				RuntimeEvent::Balances(pallet_balances::Event::Minted { who, amount }) => {
					who: *who == <Kusama as KusamaPallet>::XcmPallet::check_account(),
					amount:  *amount == t.args.amount,
				},
			]
		);
	}
	fn asset_hub_assertions(_: RelayToParaThroughAHTest) {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
		let sov_penpal_on_ah = AssetHubKusama::sovereign_account_id_of(
			AssetHubKusama::sibling_location_of(PenpalA::para_id()),
		);
		assert_expected_events!(
			AssetHubKusama,
			vec![
				// Deposited to receiver parachain SA
				RuntimeEvent::Balances(
					pallet_balances::Event::Minted { who, .. }
				) => {
					who: *who == sov_penpal_on_ah,
				},
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	}
	fn penpal_assertions(t: RelayToParaThroughAHTest) {
		type RuntimeEvent = <PenpalA as Chain>::RuntimeEvent;
		let expected_id = Location { parents: 1, interior: Here };
		assert_expected_events!(
			PenpalA,
			vec![
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == expected_id,
					owner: *owner == t.receiver.account_id,
				},
			]
		);
	}
	fn transfer_assets_dispatchable(t: RelayToParaThroughAHTest) -> DispatchResult {
		let fee_idx = t.args.fee_asset_item as usize;
		let fee: Asset = t.args.assets.inner().get(fee_idx).cloned().unwrap();
		let asset_hub_location = Kusama::child_location_of(AssetHubKusama::para_id());
		let context = KusamaUniversalLocation::get();

		// reanchor fees to the view of destination (Penpal)
		let mut remote_fees = fee.clone().reanchored(&t.args.dest, &context).unwrap();
		if let Fungible(ref mut amount) = remote_fees.fun {
			// we already spent some fees along the way, just use half of what we started with
			*amount /= 2;
		}
		let xcm_on_final_dest = Xcm::<()>(vec![
			BuyExecution { fees: remote_fees, weight_limit: t.args.weight_limit.clone() },
			DepositAsset {
				assets: Wild(AllCounted(t.args.assets.len() as u32)),
				beneficiary: t.args.beneficiary,
			},
		]);

		// reanchor final dest (Penpal) to the view of hop (Asset Hub)
		let mut dest = t.args.dest.clone();
		dest.reanchor(&asset_hub_location, &context).unwrap();
		// on Asset Hub, forward assets to Penpal
		let xcm_on_hop = Xcm::<()>(vec![DepositReserveAsset {
			assets: Wild(AllCounted(t.args.assets.len() as u32)),
			dest,
			xcm: xcm_on_final_dest,
		}]);

		Dmp::make_parachain_reachable(AssetHubKusama::para_id());

		// First leg is a teleport, from there a local-reserve-transfer to final dest
		<Kusama as KusamaPallet>::XcmPallet::transfer_assets_using_type_and_then(
			t.signed_origin,
			bx!(asset_hub_location.into()),
			bx!(t.args.assets.into()),
			bx!(TransferType::Teleport),
			bx!(fee.id.into()),
			bx!(TransferType::Teleport),
			bx!(VersionedXcm::from(xcm_on_hop)),
			t.args.weight_limit,
		)
	}

	// Set assertions and dispatchables
	test.set_assertion::<Kusama>(relay_assertions);
	test.set_assertion::<AssetHubKusama>(asset_hub_assertions);
	test.set_assertion::<PenpalA>(penpal_assertions);
	test.set_dispatchable::<Kusama>(transfer_assets_dispatchable);
	test.assert();

	// Query final balances
	let sender_balance_after = test.sender.balance;
	let sov_penpal_on_ah_after = AssetHubKusama::execute_with(|| {
		<AssetHubKusama as AssetHubKusamaPallet>::Balances::free_balance(sov_penpal_on_ah)
	});
	let receiver_assets_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location, &receiver)
	});

	// Sender's balance is reduced by amount sent plus delivery fees
	assert!(sender_balance_after < sender_balance_before - amount_to_send);
	// SA on AH balance is increased
	assert!(sov_penpal_on_ah_after > sov_penpal_on_ah_before);
	// Receiver's asset balance is increased
	assert!(receiver_assets_after > receiver_assets_before);
	// Receiver's asset balance increased by `amount_to_send - delivery_fees - bought_execution`;
	// `delivery_fees` might be paid from transfer or JIT, also `bought_execution` is unknown but
	// should be non-zero
	assert!(receiver_assets_after < receiver_assets_before + amount_to_send);
}

// We transfer USDT from PenpalA to PenpalB through Asset Hub.
// The sender on PenpalA pays delivery fees in KSM.
// When the message arrives to Asset Hub, execution and delivery fees are paid in USDT
// swapping for KSM automatically.
// When it arrives to PenpalB, execution fees are paid with USDT by swapping for KSM.
#[test]
fn usdt_only_transfer_from_para_to_para_through_asset_hub() {
	// Initialize necessary variables.
	let amount_to_send = 1_000_000_000_000;
	let sender = PenpalASender::get();
	let destination = PenpalA::sibling_location_of(PenpalB::para_id());
	let penpal_a_as_seen_by_ah = AssetHubKusama::sibling_location_of(PenpalA::para_id());
	let sov_penpal_on_ah = AssetHubKusama::sovereign_account_id_of(penpal_a_as_seen_by_ah);
	let receiver = PenpalBReceiver::get();
	let fee_asset_item = 0;
	let usdt_location: Location =
		(Parent, Parachain(1000), PalletInstance(50), GeneralIndex(1984)).into();
	let usdt_location_ah: Location = (PalletInstance(50), GeneralIndex(1984)).into();
	let ksm_location = Location::parent();
	let usdt_location_latest: Location = usdt_location.clone();
	let assets: Vec<Asset> = vec![(usdt_location_latest.clone(), amount_to_send).into()];

	// Sender needs some ksm to pay for delivery fees.
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		ksm_location.clone(),
		sender.clone(),
		10_000_000_000_000,
	);

	// The sovereign account of PenpalA in AssetHubKusama needs to have the same amount of USDT
	// since it's the reserve.
	AssetHubKusama::mint_asset(
		<AssetHubKusama as Chain>::RuntimeOrigin::signed(AssetHubKusamaAssetOwner::get()),
		USDT_ID,
		sov_penpal_on_ah,
		10_000_000_000_000,
	);

	// Mint USDT to sender to be able to transfer.
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		usdt_location_latest.clone(),
		sender.clone(),
		10_000_000_000_000,
	);

	// AssetHubKusama has a pool between USDT and ksm so fees can be paid with USDT by automatically
	// swapping them for ksm.
	create_pool_with_ksm_on!(
		AssetHubKusama,
		usdt_location_ah,
		false,
		AssetHubKusamaAssetOwner::get()
	);

	// PenpalB has a pool between USDT and ksm so fees can be paid with USDT by automatically
	// swapping them for ksm.
	create_pool_with_ksm_on!(PenpalB, usdt_location.clone(), true, PenpalAssetOwner::get());

	// Sender starts with a lot of USDT.
	let sender_balance_before = foreign_balance_on!(PenpalA, usdt_location.clone(), &sender);
	assert_eq!(sender_balance_before, 10_000_000_000_000);

	// Receiver has no USDT.
	let receiver_balance_before = foreign_balance_on!(PenpalB, usdt_location.clone(), &receiver);
	assert_eq!(receiver_balance_before, 0);

	let test_args = TestContext {
		sender: sender.clone(),
		receiver: receiver.clone(),
		args: TestArgs::new_para(
			destination.clone(),
			receiver.clone(),
			amount_to_send,
			assets.into(),
			None,
			fee_asset_item,
		),
	};
	let mut test = ParaToParaThroughAHTest::new(test_args);

	// Assertions executed on the sender, PenpalA.
	fn sender_assertions(_: ParaToParaThroughAHTest) {
		type Event = <PenpalA as Chain>::RuntimeEvent;

		let transfer_amount = 1_000_000_000_000;
		let usdt_location: Location =
			(Parent, Parachain(1000), PalletInstance(50), GeneralIndex(1984)).into();

		assert_expected_events!(
			PenpalA,
			vec![
				Event::ForeignAssets(
					pallet_assets::Event::Burned { asset_id, balance, .. }
				) => {
					asset_id: *asset_id == usdt_location.clone(),
					balance: *balance == transfer_amount,
				},
			]
		);
	}

	// Assertions executed on the intermediate hop, AssetHubKusama.
	fn ah_assertions(_: ParaToParaThroughAHTest) {
		type Event = <AssetHubKusama as Chain>::RuntimeEvent;

		let transfer_amount = 1_000_000_000_000;
		let penpal_a_as_seen_by_ah = AssetHubKusama::sibling_location_of(PenpalA::para_id());
		let sov_penpal_on_ah = AssetHubKusama::sovereign_account_id_of(penpal_a_as_seen_by_ah);

		assert_expected_events!(
			AssetHubKusama,
			vec![
				// USDT is burned from sovereign account of PenpalA.
				Event::Assets(
					pallet_assets::Event::Burned { asset_id, owner, balance }
				) => {
					asset_id: *asset_id == 1984,
					owner: *owner == sov_penpal_on_ah,
					balance: *balance == transfer_amount,
				},
				// Credit is swapped.
				Event::AssetConversion(
					pallet_asset_conversion::Event::SwapCreditExecuted { .. }
				) => {},
				// Message from PenpalA was processed.
				Event::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	}

	// Assertions executed on the receiver, PenpalB.
	fn receiver_assertions(_: ParaToParaThroughAHTest) {
		type Event = <PenpalB as Chain>::RuntimeEvent;
		let usdt_location: Location =
			(Parent, Parachain(1000), PalletInstance(50), GeneralIndex(1984)).into();
		let receiver = PenpalBReceiver::get();
		assert_expected_events!(
			PenpalB,
			vec![
				// Final amount gets deposited to receiver.
				Event::ForeignAssets(
					pallet_assets::Event::Issued { asset_id, owner, .. }
				) => {
					asset_id: *asset_id == usdt_location,
					owner: *owner == receiver,
				},
				// Swap was made to pay fees with USDT.
				Event::AssetConversion(
					pallet_asset_conversion::Event::SwapCreditExecuted { .. }
				) => {},
			]
		);
	}

	// Run test and assert.
	test.set_assertion::<PenpalA>(sender_assertions);
	test.set_assertion::<AssetHubKusama>(ah_assertions);
	test.set_assertion::<PenpalB>(receiver_assertions);
	test.set_dispatchable::<PenpalA>(para_to_para_transfer_assets_through_ah);
	test.assert();

	// Sender has less USDT after the transfer.
	let sender_balance_after = foreign_balance_on!(PenpalA, usdt_location.clone(), &sender);
	assert_eq!(sender_balance_after, 9_000_000_000_000);

	// Receiver gets `transfer_amount` minus fees.
	let receiver_balance_after = foreign_balance_on!(PenpalB, usdt_location.clone(), &receiver);
	assert!(receiver_balance_after > receiver_balance_before);
}
