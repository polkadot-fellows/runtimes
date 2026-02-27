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
use polkadot_system_emulated_network::polkadot_emulated_chain::polkadot_runtime::Dmp;

fn para_to_para_assethub_hop_assertions(t: ParaToParaThroughAHTest) {
	type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
	let sov_penpal_a_on_ah = AssetHubPolkadot::sovereign_account_id_of(
		AssetHubPolkadot::sibling_location_of(PenpalB::para_id()),
	);
	let sov_penpal_b_on_ah = AssetHubPolkadot::sovereign_account_id_of(
		AssetHubPolkadot::sibling_location_of(PenpalA::para_id()),
	);

	assert_expected_events!(
		AssetHubPolkadot,
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
	<AssetHubPolkadot as AssetHubPolkadotPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
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
	<PenpalB as PenpalBPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
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
	let asset_hub_location: Location = PenpalB::sibling_location_of(AssetHubPolkadot::para_id());
	let custom_xcm_on_dest = Xcm::<()>(vec![DepositAsset {
		assets: Wild(AllCounted(t.args.assets.len() as u32)),
		beneficiary: t.args.beneficiary,
	}]);
	<PenpalB as PenpalBPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
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

fn para_to_para_transfer_assets_through_ah_inverted(
	t: Test<PenpalA, PenpalB, AssetHubPolkadot>,
) -> DispatchResult {
	let fee_idx = t.args.fee_asset_item as usize;
	let fee: Asset = t.args.assets.inner().get(fee_idx).cloned().unwrap();
	let asset_hub_location: Location = PenpalA::sibling_location_of(AssetHubPolkadot::para_id());
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
	<PenpalB as PenpalBPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
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
	<AssetHubPolkadot as AssetHubPolkadotPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
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
	let destination = AssetHubPolkadot::sibling_location_of(PenpalB::para_id());
	let sender = AssetHubPolkadotSender::get();
	let native_amount_to_send: Balance = ASSET_HUB_POLKADOT_ED * 10000;
	let native_asset_location = DotLocation::get();
	let receiver = PenpalBReceiver::get();
	let assets_owner = PenpalAssetOwner::get();
	// Foreign asset used: bridged KSM
	let foreign_amount_to_send = ASSET_HUB_POLKADOT_ED * 10_000_000;
	let ksm_at_polkadot_parachains = Location::new(2, [GlobalConsensus(NetworkId::Kusama)]);
	let ksm_at_polkadot_parachains_latest: Location = ksm_at_polkadot_parachains.clone();

	// Configure destination chain to trust AH as reserve of KSM
	PenpalB::execute_with(|| {
		assert_ok!(<PenpalB as Chain>::System::set_storage(
			<PenpalB as Chain>::RuntimeOrigin::root(),
			vec![(
				CustomizableAssetFromSystemAssetHub::key().to_vec(),
				Location::new(2, [GlobalConsensus(Kusama)]).encode(),
			)],
		));
	});
	PenpalB::force_create_foreign_asset(
		ksm_at_polkadot_parachains_latest.clone(),
		assets_owner.clone(),
		false,
		ASSET_MIN_BALANCE,
		vec![],
	);
	AssetHubPolkadot::force_create_foreign_asset(
		ksm_at_polkadot_parachains.clone(),
		assets_owner.clone(),
		false,
		ASSET_MIN_BALANCE,
		vec![],
	);
	AssetHubPolkadot::mint_foreign_asset(
		<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(assets_owner),
		ksm_at_polkadot_parachains.clone(),
		sender.clone(),
		foreign_amount_to_send * 2,
	);

	// Assets to send
	let assets: Vec<Asset> = vec![
		(Parent, native_amount_to_send).into(),
		(ksm_at_polkadot_parachains_latest.clone(), foreign_amount_to_send).into(),
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
	let sender_ksm_before = AssetHubPolkadot::execute_with(|| {
		type ForeignAssets = <AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(ksm_at_polkadot_parachains.clone(), &sender)
	});
	let receiver_assets_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(native_asset_location.clone(), &receiver)
	});
	let receiver_ksm_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(ksm_at_polkadot_parachains_latest.clone(), &receiver)
	});

	// Set assertions and dispatchables
	test.set_assertion::<AssetHubPolkadot>(system_para_to_para_sender_assertions);
	test.set_assertion::<PenpalB>(system_para_to_para_receiver_assertions);
	test.set_dispatchable::<AssetHubPolkadot>(ah_to_para_transfer_assets);
	test.assert();

	// Query final balances
	let sender_balance_after = test.sender.balance;
	let sender_ksm_after = AssetHubPolkadot::execute_with(|| {
		type ForeignAssets = <AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(ksm_at_polkadot_parachains.clone(), &sender)
	});
	let receiver_assets_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(native_asset_location, &receiver)
	});
	let receiver_ksm_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(ksm_at_polkadot_parachains_latest, &receiver)
	});

	// Sender's balance is reduced by amount sent plus delivery fees
	assert!(sender_balance_after < sender_balance_before - native_amount_to_send);
	// Sender's balance is reduced by foreign amount sent
	assert_eq!(sender_ksm_after, sender_ksm_before - foreign_amount_to_send);
	// Receiver's assets is increased
	assert!(receiver_assets_after > receiver_assets_before);
	// Receiver's assets increased by `amount_to_send - delivery_fees - bought_execution`;
	// `delivery_fees` might be paid from transfer or JIT, also `bought_execution` is unknown but
	// should be non-zero
	assert!(receiver_assets_after < receiver_assets_before + native_amount_to_send);
	// Receiver's balance is increased by foreign amount sent
	assert_eq!(receiver_ksm_after, receiver_ksm_before + foreign_amount_to_send);
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
	let destination = PenpalB::sibling_location_of(AssetHubPolkadot::para_id());
	let sender = PenpalBSender::get();
	let native_amount_to_send: Balance = ASSET_HUB_POLKADOT_ED * 10000;
	let native_asset_location = DotLocation::get();
	let assets_owner = PenpalAssetOwner::get();

	// Foreign asset used: bridged KSM
	let foreign_amount_to_send = ASSET_HUB_POLKADOT_ED * 10_000_000;
	let ksm_at_polkadot_parachains = Location::new(2, [GlobalConsensus(NetworkId::Kusama)]);
	let ksm_at_polkadot_parachains_latest: Location = ksm_at_polkadot_parachains.clone();

	// Configure destination chain to trust AH as reserve of KSM
	PenpalB::execute_with(|| {
		assert_ok!(<PenpalB as Chain>::System::set_storage(
			<PenpalB as Chain>::RuntimeOrigin::root(),
			vec![(
				CustomizableAssetFromSystemAssetHub::key().to_vec(),
				Location::new(2, [GlobalConsensus(Kusama)]).encode(),
			)],
		));
	});
	PenpalB::force_create_foreign_asset(
		ksm_at_polkadot_parachains_latest.clone(),
		assets_owner.clone(),
		false,
		ASSET_MIN_BALANCE,
		vec![],
	);
	AssetHubPolkadot::force_create_foreign_asset(
		ksm_at_polkadot_parachains.clone(),
		assets_owner.clone(),
		false,
		ASSET_MIN_BALANCE,
		vec![],
	);

	// fund Parachain's sender account
	PenpalB::mint_foreign_asset(
		<PenpalB as Chain>::RuntimeOrigin::signed(assets_owner.clone()),
		native_asset_location.clone(),
		sender.clone(),
		native_amount_to_send * 2,
	);
	PenpalB::mint_foreign_asset(
		<PenpalB as Chain>::RuntimeOrigin::signed(assets_owner.clone()),
		ksm_at_polkadot_parachains_latest.clone(),
		sender.clone(),
		foreign_amount_to_send * 2,
	);

	// Init values for System Parachain
	let receiver = AssetHubPolkadotReceiver::get();
	let penpal_location_as_seen_by_ahp = AssetHubPolkadot::sibling_location_of(PenpalB::para_id());
	let sov_penpal_on_ahp =
		AssetHubPolkadot::sovereign_account_id_of(penpal_location_as_seen_by_ahp);

	// fund Parachain's SA on AssetHub with the assets held in reserve
	AssetHubPolkadot::fund_accounts(vec![(sov_penpal_on_ahp.clone(), native_amount_to_send * 2)]);
	AssetHubPolkadot::mint_foreign_asset(
		<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(assets_owner),
		ksm_at_polkadot_parachains.clone(),
		sov_penpal_on_ahp,
		foreign_amount_to_send * 2,
	);

	// Assets to send
	let assets: Vec<Asset> = vec![
		(Parent, native_amount_to_send).into(),
		(ksm_at_polkadot_parachains_latest.clone(), foreign_amount_to_send).into(),
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
	let sender_native_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(native_asset_location.clone(), &sender)
	});
	let sender_ksm_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(ksm_at_polkadot_parachains_latest.clone(), &sender)
	});
	let receiver_native_before = test.receiver.balance;
	let receiver_ksm_before = AssetHubPolkadot::execute_with(|| {
		type ForeignAssets = <AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(ksm_at_polkadot_parachains.clone(), &receiver)
	});

	// Set assertions and dispatchables
	test.set_assertion::<PenpalB>(para_to_system_para_sender_assertions);
	test.set_assertion::<AssetHubPolkadot>(para_to_system_para_receiver_assertions);
	test.set_dispatchable::<PenpalB>(para_to_ah_transfer_assets);
	test.assert();

	// Query final balances
	let sender_native_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(native_asset_location, &sender)
	});
	let sender_ksm_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(ksm_at_polkadot_parachains_latest.clone(), &sender)
	});
	let receiver_native_after = test.receiver.balance;
	let receiver_ksm_after = AssetHubPolkadot::execute_with(|| {
		type ForeignAssets = <AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(ksm_at_polkadot_parachains, &receiver)
	});

	// Sender's balance is reduced by amount sent plus delivery fees
	assert!(sender_native_after < sender_native_before - native_amount_to_send);
	// Sender's balance is reduced by foreign amount sent
	assert_eq!(sender_ksm_after, sender_ksm_before - foreign_amount_to_send);
	// Receiver's balance is increased
	assert!(receiver_native_after > receiver_native_before);
	// Receiver's balance increased by `amount_to_send - delivery_fees - bought_execution`;
	// `delivery_fees` might be paid from transfer or JIT, also `bought_execution` is unknown but
	// should be non-zero
	assert!(receiver_native_after < receiver_native_before + native_amount_to_send);
	// Receiver's balance is increased by foreign amount sent
	assert_eq!(receiver_ksm_after, receiver_ksm_before + foreign_amount_to_send);
}

// ==============================================================================
// ===== Transfer - Native + Bridged Assets - Parachain->AssetHub->Parachain ====
// ==============================================================================
/// Transfers of native asset plus bridged asset from Parachain to Parachain
/// (through AssetHub reserve) with fees paid using native asset.
#[test]
fn transfer_foreign_assets_from_para_to_para_through_asset_hub() {
	// Init values for Parachain Origin
	let destination = PenpalB::sibling_location_of(PenpalA::para_id());
	let sender = PenpalBSender::get();
	let dot_to_send: Balance = POLKADOT_ED * 10000;
	let assets_owner = PenpalAssetOwner::get();
	let dot_location = DotLocation::get();
	let sender_as_seen_by_ah = AssetHubPolkadot::sibling_location_of(PenpalB::para_id());
	let sov_of_sender_on_ah = AssetHubPolkadot::sovereign_account_id_of(sender_as_seen_by_ah);
	let receiver_as_seen_by_ah = AssetHubPolkadot::sibling_location_of(PenpalA::para_id());
	let sov_of_receiver_on_ah = AssetHubPolkadot::sovereign_account_id_of(receiver_as_seen_by_ah);
	let ksm_to_send = ASSET_HUB_POLKADOT_ED * 10_000_000;

	// Configure source and destination chains to trust AH as reserve of KSM
	PenpalA::execute_with(|| {
		assert_ok!(<PenpalA as Chain>::System::set_storage(
			<PenpalA as Chain>::RuntimeOrigin::root(),
			vec![(
				CustomizableAssetFromSystemAssetHub::key().to_vec(),
				Location::new(2, [GlobalConsensus(Kusama)]).encode(),
			)],
		));
	});
	PenpalB::execute_with(|| {
		assert_ok!(<PenpalB as Chain>::System::set_storage(
			<PenpalB as Chain>::RuntimeOrigin::root(),
			vec![(
				CustomizableAssetFromSystemAssetHub::key().to_vec(),
				Location::new(2, [GlobalConsensus(Kusama)]).encode(),
			)],
		));
	});

	// Register KSM as foreign asset and transfer it around the Polkadot ecosystem
	let ksm_at_polkadot_parachains = Location::new(2, [GlobalConsensus(Kusama)]);
	let ksm_at_polkadot_parachains_latest: Location = ksm_at_polkadot_parachains.clone();
	AssetHubPolkadot::force_create_foreign_asset(
		ksm_at_polkadot_parachains.clone(),
		assets_owner.clone(),
		false,
		ASSET_MIN_BALANCE,
		vec![],
	);
	PenpalB::force_create_foreign_asset(
		ksm_at_polkadot_parachains_latest.clone(),
		assets_owner.clone(),
		false,
		ASSET_MIN_BALANCE,
		vec![],
	);
	PenpalA::force_create_foreign_asset(
		ksm_at_polkadot_parachains_latest.clone(),
		assets_owner.clone(),
		false,
		ASSET_MIN_BALANCE,
		vec![],
	);

	// fund Parachain's sender account
	PenpalB::mint_foreign_asset(
		<PenpalB as Chain>::RuntimeOrigin::signed(assets_owner.clone()),
		dot_location.clone(),
		sender.clone(),
		dot_to_send * 2,
	);
	PenpalB::mint_foreign_asset(
		<PenpalB as Chain>::RuntimeOrigin::signed(assets_owner.clone()),
		ksm_at_polkadot_parachains_latest.clone(),
		sender.clone(),
		ksm_to_send * 2,
	);
	// fund the Parachain Origin's SA on Asset Hub with the assets held in reserve
	AssetHubPolkadot::fund_accounts(vec![(sov_of_sender_on_ah.clone(), dot_to_send * 2)]);
	AssetHubPolkadot::mint_foreign_asset(
		<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(assets_owner),
		ksm_at_polkadot_parachains.clone(),
		sov_of_sender_on_ah.clone(),
		ksm_to_send * 2,
	);

	// Init values for Parachain Destination
	let receiver = PenpalAReceiver::get();

	// Assets to send
	let assets: Vec<Asset> = vec![
		(dot_location.clone(), dot_to_send).into(),
		(ksm_at_polkadot_parachains_latest.clone(), ksm_to_send).into(),
	];
	let fee_asset_id: AssetId = dot_location.clone().into();
	let fee_asset_item = assets.iter().position(|a| a.id == fee_asset_id).unwrap() as u32;

	// Init Test
	let test_args = TestContext {
		sender: sender.clone(),
		receiver: receiver.clone(),
		args: TestArgs::new_para(
			destination,
			receiver.clone(),
			dot_to_send,
			assets.into(),
			None,
			fee_asset_item,
		),
	};
	let mut test = ParaToParaThroughAHTest::new(test_args);

	// Query initial balances
	let sender_dot_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(dot_location.clone(), &sender)
	});
	let sender_ksm_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(ksm_at_polkadot_parachains_latest.clone(), &sender)
	});
	let dot_in_sender_reserve_on_ahp_before =
		<AssetHubPolkadot as Chain>::account_data_of(sov_of_sender_on_ah.clone()).free;
	let ksm_in_sender_reserve_on_ahp_before = AssetHubPolkadot::execute_with(|| {
		type Assets = <AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(ksm_at_polkadot_parachains.clone(), &sov_of_sender_on_ah)
	});
	let dot_in_receiver_reserve_on_ahp_before =
		<AssetHubPolkadot as Chain>::account_data_of(sov_of_receiver_on_ah.clone()).free;
	let ksm_in_receiver_reserve_on_ahp_before = AssetHubPolkadot::execute_with(|| {
		type Assets = <AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(ksm_at_polkadot_parachains.clone(), &sov_of_receiver_on_ah)
	});
	let receiver_dot_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(dot_location.clone(), &receiver)
	});
	let receiver_ksm_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(ksm_at_polkadot_parachains_latest.clone(), &receiver)
	});

	// Set assertions and dispatchables
	test.set_assertion::<PenpalB>(para_to_para_through_hop_sender_assertions);
	test.set_assertion::<AssetHubPolkadot>(para_to_para_assethub_hop_assertions);
	test.set_assertion::<PenpalA>(para_to_para_through_hop_receiver_assertions);
	test.set_dispatchable::<PenpalB>(para_to_para_transfer_assets_through_ah);
	test.assert();

	// Query final balances
	let sender_dot_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(dot_location.clone(), &sender)
	});
	let sender_ksm_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(ksm_at_polkadot_parachains_latest.clone(), &sender)
	});
	let ksm_in_sender_reserve_on_ahp_after = AssetHubPolkadot::execute_with(|| {
		type Assets = <AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(ksm_at_polkadot_parachains.clone(), &sov_of_sender_on_ah)
	});
	let dot_in_sender_reserve_on_ahp_after =
		<AssetHubPolkadot as Chain>::account_data_of(sov_of_sender_on_ah).free;
	let ksm_in_receiver_reserve_on_ahp_after = AssetHubPolkadot::execute_with(|| {
		type Assets = <AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(ksm_at_polkadot_parachains.clone(), &sov_of_receiver_on_ah)
	});
	let dot_in_receiver_reserve_on_ahp_after =
		<AssetHubPolkadot as Chain>::account_data_of(sov_of_receiver_on_ah).free;
	let receiver_dot_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(dot_location, &receiver)
	});
	let receiver_ksm_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(ksm_at_polkadot_parachains_latest, &receiver)
	});

	// Sender's balance is reduced by amount sent plus delivery fees
	assert!(sender_dot_after < sender_dot_before - dot_to_send);
	assert_eq!(sender_ksm_after, sender_ksm_before - ksm_to_send);
	// Sovereign accounts on reserve are changed accordingly
	assert_eq!(
		dot_in_sender_reserve_on_ahp_after,
		dot_in_sender_reserve_on_ahp_before - dot_to_send
	);
	assert_eq!(
		ksm_in_sender_reserve_on_ahp_after,
		ksm_in_sender_reserve_on_ahp_before - ksm_to_send
	);
	assert!(dot_in_receiver_reserve_on_ahp_after > dot_in_receiver_reserve_on_ahp_before);
	assert_eq!(
		ksm_in_receiver_reserve_on_ahp_after,
		ksm_in_receiver_reserve_on_ahp_before + ksm_to_send
	);
	// Receiver's balance is increased
	assert!(receiver_dot_after > receiver_dot_before);
	assert_eq!(receiver_ksm_after, receiver_ksm_before + ksm_to_send);
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
	let destination = Polkadot::child_location_of(PenpalB::para_id());
	let sender = PolkadotSender::get();
	let amount_to_send: Balance = POLKADOT_ED * 1000;

	// Init values for Parachain
	let relay_native_asset_location = DotLocation::get();
	let receiver = PenpalBReceiver::get();

	// Init Test
	let test_args = TestContext {
		sender,
		receiver: receiver.clone(),
		args: TestArgs::new_relay(destination.clone(), receiver.clone(), amount_to_send),
	};
	let mut test = RelayToParaThroughAHTest::new(test_args);

	let sov_penpal_on_ah = AssetHubPolkadot::sovereign_account_id_of(
		AssetHubPolkadot::sibling_location_of(PenpalB::para_id()),
	);
	// Query initial balances
	let sender_balance_before = test.sender.balance;
	let sov_penpal_on_ah_before = AssetHubPolkadot::execute_with(|| {
		<AssetHubPolkadot as AssetHubPolkadotPallet>::Balances::free_balance(
			sov_penpal_on_ah.clone(),
		)
	});
	let receiver_assets_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location.clone(), &receiver)
	});

	fn relay_assertions(t: RelayToParaThroughAHTest) {
		type RuntimeEvent = <Polkadot as Chain>::RuntimeEvent;
		Polkadot::assert_xcm_pallet_attempted_complete(None);
		assert_expected_events!(
			Polkadot,
			vec![
				// Amount to teleport is withdrawn from Sender
				RuntimeEvent::Balances(pallet_balances::Event::Burned { who, amount }) => {
					who: *who == t.sender.account_id,
					amount: *amount == t.args.amount,
				},
				// Amount to teleport is deposited in Relay's `CheckAccount`
				RuntimeEvent::Balances(pallet_balances::Event::Minted { who, amount }) => {
					who: *who == <Polkadot as PolkadotPallet>::XcmPallet::check_account(),
					amount:  *amount == t.args.amount,
				},
			]
		);
	}
	fn asset_hub_assertions(_: RelayToParaThroughAHTest) {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		let sov_penpal_on_ah = AssetHubPolkadot::sovereign_account_id_of(
			AssetHubPolkadot::sibling_location_of(PenpalB::para_id()),
		);
		assert_expected_events!(
			AssetHubPolkadot,
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
		type RuntimeEvent = <PenpalB as Chain>::RuntimeEvent;
		let expected_id = Location { parents: 1, interior: Here };
		assert_expected_events!(
			PenpalB,
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
		let asset_hub_location = Polkadot::child_location_of(AssetHubPolkadot::para_id());
		let context = PolkadotUniversalLocation::get();

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

		Dmp::make_parachain_reachable(AssetHubPolkadot::para_id());

		// First leg is a teleport, from there a local-reserve-transfer to final dest
		<Polkadot as PolkadotPallet>::XcmPallet::transfer_assets_using_type_and_then(
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
	test.set_assertion::<Polkadot>(relay_assertions);
	test.set_assertion::<AssetHubPolkadot>(asset_hub_assertions);
	test.set_assertion::<PenpalB>(penpal_assertions);
	test.set_dispatchable::<Polkadot>(transfer_assets_dispatchable);
	test.assert();

	// Query final balances
	let sender_balance_after = test.sender.balance;
	let sov_penpal_on_ah_after = AssetHubPolkadot::execute_with(|| {
		<AssetHubPolkadot as AssetHubPolkadotPallet>::Balances::free_balance(sov_penpal_on_ah)
	});
	let receiver_assets_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
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
// The sender on PenpalA pays delivery fees in DOT.
// When the message arrives to Asset Hub, execution and delivery fees are paid in USDT
// swapping for DOT automatically.
// When it arrives to PenpalB, execution fees are paid with USDT by swapping for DOT.
#[test]
fn usdt_only_transfer_from_para_to_para_through_asset_hub() {
	// ParaToParaThroughAHTest has the source and destination chains inverted.
	type PenpalAToPenpalBTest = Test<PenpalA, PenpalB, AssetHubPolkadot>;

	// Initialize necessary variables.
	let amount_to_send = 1_000_000_000_000;
	let sender = PenpalASender::get();
	let destination = PenpalA::sibling_location_of(PenpalB::para_id());
	let penpal_a_as_seen_by_ah = AssetHubPolkadot::sibling_location_of(PenpalA::para_id());
	let sov_penpal_on_ah = AssetHubPolkadot::sovereign_account_id_of(penpal_a_as_seen_by_ah);
	let receiver = PenpalBReceiver::get();
	let fee_asset_item = 0;
	let usdt_location: Location =
		(Parent, Parachain(1000), PalletInstance(50), GeneralIndex(1984)).into();
	let usdt_location_ah: Location = (PalletInstance(50), GeneralIndex(1984)).into();
	let dot_location = Location::parent();
	let assets: Vec<Asset> = vec![(usdt_location.clone(), amount_to_send).into()];

	// Sender needs some DOT to pay for delivery fees.
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		dot_location.clone(),
		sender.clone(),
		10_000_000_000_000,
	);

	// The sovereign account of PenpalA in AssetHubPolkadot needs to have the same amount of USDT
	// since it's the reserve.
	AssetHubPolkadot::mint_asset(
		<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotAssetOwner::get()),
		USDT_ID,
		sov_penpal_on_ah,
		10_000_000_000_000,
	);

	// Mint USDT to sender to be able to transfer.
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		usdt_location.clone(),
		sender.clone(),
		10_000_000_000_000,
	);

	// AssetHubPolkadot has a pool between USDT and DOT so fees can be paid with USDT by
	// automatically swapping them for DOT.
	create_pool_with_dot_on!(
		AssetHubPolkadot,
		usdt_location_ah,
		false,
		AssetHubPolkadotAssetOwner::get()
	);

	// PenpalB has a pool between USDT and DOT so fees can be paid with USDT by automatically
	// swapping them for DOT.
	create_pool_with_dot_on!(PenpalB, usdt_location.clone(), true, PenpalAssetOwner::get());

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
	let mut test = PenpalAToPenpalBTest::new(test_args);

	// Assertions executed on the sender, PenpalA.
	fn sender_assertions(_: PenpalAToPenpalBTest) {
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

	// Assertions executed on the intermediate hop, AssetHubPolkadot.
	fn ah_assertions(_: PenpalAToPenpalBTest) {
		type Event = <AssetHubPolkadot as Chain>::RuntimeEvent;

		let transfer_amount = 1_000_000_000_000;
		let penpal_a_as_seen_by_ah = AssetHubPolkadot::sibling_location_of(PenpalA::para_id());
		let sov_penpal_on_ah = AssetHubPolkadot::sovereign_account_id_of(penpal_a_as_seen_by_ah);

		assert_expected_events!(
			AssetHubPolkadot,
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
	fn receiver_assertions(_: PenpalAToPenpalBTest) {
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
	test.set_assertion::<AssetHubPolkadot>(ah_assertions);
	test.set_assertion::<PenpalB>(receiver_assertions);
	test.set_dispatchable::<PenpalA>(para_to_para_transfer_assets_through_ah_inverted);
	test.assert();

	// Sender has less USDT after the transfer.
	let sender_balance_after = foreign_balance_on!(PenpalA, usdt_location.clone(), &sender);
	assert_eq!(sender_balance_after, 9_000_000_000_000);

	// Receiver gets `transfer_amount` minus fees.
	let receiver_balance_after = foreign_balance_on!(PenpalB, usdt_location.clone(), &receiver);
	assert!(receiver_balance_after > receiver_balance_before);
}
