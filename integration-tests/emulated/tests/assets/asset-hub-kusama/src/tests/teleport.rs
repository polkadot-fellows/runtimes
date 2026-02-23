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
use emulated_integration_tests_common::xcm_helpers::{fee_asset, non_fee_asset};
use frame_support::{
	dispatch::{GetDispatchInfo, RawOrigin},
	traits::fungible::Mutate,
};
use kusama_system_emulated_network::penpal_emulated_chain::LocalTeleportableToAssetHub as PenpalLocalTeleportableToAssetHub;
use xcm_runtime_apis::dry_run::runtime_decl_for_dry_run_api::DryRunApiV2;

fn relay_dest_assertions_fail(_t: SystemParaToRelayTest) {
	Kusama::assert_ump_queue_processed(false, Some(AssetHubKusama::para_id()), None);
}

fn para_origin_assertions(t: SystemParaToRelayTest) {
	type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;

	AssetHubKusama::assert_xcm_pallet_attempted_complete(None);

	AssetHubKusama::assert_parachain_system_ump_sent();

	assert_expected_events!(
		AssetHubKusama,
		vec![
			// Amount is withdrawn from Sender's account
			RuntimeEvent::Balances(pallet_balances::Event::Burned { who, amount }) => {
				who: *who == t.sender.account_id,
				amount: *amount == t.args.amount,
			},
		]
	);
}

fn penpal_to_ah_foreign_assets_sender_assertions(t: ParaToSystemParaTest) {
	type RuntimeEvent = <PenpalA as Chain>::RuntimeEvent;
	let system_para_native_asset_location = KsmLocation::get();
	let expected_asset_id = t.args.asset_id.unwrap();
	let (_, expected_asset_amount) =
		non_fee_asset(&t.args.assets, t.args.fee_asset_item as usize).unwrap();

	PenpalA::assert_xcm_pallet_attempted_complete(None);
	assert_expected_events!(
		PenpalA,
		vec![
			RuntimeEvent::ForeignAssets(
				pallet_assets::Event::Burned { asset_id, owner, .. }
			) => {
				asset_id: *asset_id == system_para_native_asset_location,
				owner: *owner == t.sender.account_id,
			},
			RuntimeEvent::Assets(pallet_assets::Event::Burned { asset_id, owner, balance }) => {
				asset_id: *asset_id == expected_asset_id,
				owner: *owner == t.sender.account_id,
				balance: *balance == expected_asset_amount,
			},
		]
	);
}

fn penpal_to_ah_foreign_assets_receiver_assertions(t: ParaToSystemParaTest) {
	type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
	let (_, expected_foreign_asset_amount) =
		non_fee_asset(&t.args.assets, t.args.fee_asset_item as usize).unwrap();
	AssetHubKusama::assert_xcmp_queue_success(None);
	assert_expected_events!(
		AssetHubKusama,
		vec![
			RuntimeEvent::Balances(pallet_balances::Event::Minted { who, .. }) => {
				who: *who == t.receiver.account_id,
			},
			RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { owner, amount, .. }) => {
				owner: *owner == t.receiver.account_id,
				amount: *amount == expected_foreign_asset_amount,
			},
			RuntimeEvent::Balances(pallet_balances::Event::Issued { .. }) => {},
		]
	);
}

fn ah_to_penpal_foreign_assets_sender_assertions(t: SystemParaToParaTest) {
	type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
	AssetHubKusama::assert_xcm_pallet_attempted_complete(None);
	let (_, expected_native_amount) =
		fee_asset(&t.args.assets, t.args.fee_asset_item as usize).unwrap();
	let (_, expected_foreign_asset_amount) =
		non_fee_asset(&t.args.assets, t.args.fee_asset_item as usize).unwrap();
	assert_expected_events!(
		AssetHubKusama,
		vec![
			// native asset used for fees is transferred to Parachain's Sovereign account as reserve
			RuntimeEvent::Balances(
				pallet_balances::Event::Transfer { from, amount, .. }
			) => {
				from: *from == t.sender.account_id,
				amount: *amount == expected_native_amount,
			},
			// foreign asset is burned locally as part of teleportation
			RuntimeEvent::ForeignAssets(pallet_assets::Event::Burned { owner, balance, .. }) => {
				owner: *owner == t.sender.account_id,
				balance: *balance == expected_foreign_asset_amount,
			},
		]
	);
}

fn ah_to_penpal_foreign_assets_receiver_assertions(t: SystemParaToParaTest) {
	type RuntimeEvent = <PenpalA as Chain>::RuntimeEvent;
	let expected_asset_id = t.args.asset_id.unwrap();
	let (_, expected_asset_amount) =
		non_fee_asset(&t.args.assets, t.args.fee_asset_item as usize).unwrap();
	let checking_account = <PenpalA as PenpalAPallet>::PolkadotXcm::check_account();
	PenpalA::assert_xcmp_queue_success(None);
	assert_expected_events!(
		PenpalA,
		vec![
			// checking account burns local asset as part of incoming teleport
			RuntimeEvent::Assets(pallet_assets::Event::Burned { asset_id, owner, balance }) => {
				asset_id: *asset_id == expected_asset_id,
				owner: *owner == checking_account,
				balance: *balance == expected_asset_amount,
			},
			// local asset is teleported into account of receiver
			RuntimeEvent::Assets(pallet_assets::Event::Issued { asset_id, owner, amount }) => {
				asset_id: *asset_id == expected_asset_id,
				owner: *owner == t.receiver.account_id,
				amount: *amount == expected_asset_amount,
			},
			// native asset for fee is deposited to receiver
			RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
				asset_id: *asset_id == Location::parent(),
				owner: *owner == t.receiver.account_id,
			},
		]
	);
}

fn system_para_limited_teleport_assets(t: SystemParaToRelayTest) -> DispatchResult {
	<AssetHubKusama as AssetHubKusamaPallet>::PolkadotXcm::limited_teleport_assets(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.beneficiary.into()),
		bx!(t.args.assets.into()),
		t.args.fee_asset_item,
		t.args.weight_limit,
	)
}

fn para_to_system_para_transfer_assets(t: ParaToSystemParaTest) -> DispatchResult {
	type Runtime = <PenpalA as Chain>::Runtime;
	let remote_fee_id: AssetId = t
		.args
		.assets
		.clone()
		.into_inner()
		.get(t.args.fee_asset_item as usize)
		.ok_or(pallet_xcm::Error::<Runtime>::Empty)?
		.clone()
		.id;
	<PenpalA as PenpalAPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.assets.into()),
		bx!(TransferType::Teleport),
		bx!(remote_fee_id.into()),
		bx!(TransferType::DestinationReserve),
		bx!(VersionedXcm::from(
			Xcm::<()>::builder_unsafe()
				.deposit_asset(AllCounted(2), t.args.beneficiary)
				.build()
		)),
		t.args.weight_limit,
	)
}

fn system_para_to_para_transfer_assets(t: SystemParaToParaTest) -> DispatchResult {
	type Runtime = <AssetHubKusama as Chain>::Runtime;
	let remote_fee_id: AssetId = t
		.args
		.assets
		.clone()
		.into_inner()
		.get(t.args.fee_asset_item as usize)
		.ok_or(pallet_xcm::Error::<Runtime>::Empty)?
		.clone()
		.id;
	<AssetHubKusama as AssetHubKusamaPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.assets.into()),
		bx!(TransferType::Teleport),
		bx!(remote_fee_id.into()),
		bx!(TransferType::LocalReserve),
		bx!(VersionedXcm::from(
			Xcm::<()>::builder_unsafe()
				.deposit_asset(AllCounted(2), t.args.beneficiary)
				.build()
		)),
		t.args.weight_limit,
	)
}

#[test]
fn teleport_via_transfer_assets_from_and_to_relay() {
	let amount = ASSET_HUB_KUSAMA_ED * 1000;
	let native_asset: Assets = (Here, amount).into();

	test_relay_is_trusted_teleporter!(
		Kusama,
		vec![AssetHubKusama],
		(native_asset, amount),
		transfer_assets
	);

	let amount = KUSAMA_ED * 1000;

	test_parachain_is_trusted_teleporter_for_relay!(
		AssetHubKusama,
		Kusama,
		amount,
		transfer_assets
	);
}

#[test]
fn teleport_via_limited_teleport_assets_from_and_to_relay() {
	let amount = ASSET_HUB_KUSAMA_ED * 1000;
	let native_asset: Assets = (Here, amount).into();

	test_relay_is_trusted_teleporter!(
		Kusama,
		vec![AssetHubKusama],
		(native_asset, amount),
		limited_teleport_assets
	);

	let amount = KUSAMA_ED * 1000;

	test_parachain_is_trusted_teleporter_for_relay!(
		AssetHubKusama,
		Kusama,
		amount,
		limited_teleport_assets
	);
}

/// Limited Teleport of native asset from System Parachain to Relay Chain
/// shouldn't work when there is not enough balance in Relay Chain's `CheckAccount`
#[test]
fn limited_teleport_native_assets_from_system_para_to_relay_fails() {
	// Init values for Relay Chain
	let amount_to_send: Balance = ASSET_HUB_KUSAMA_ED * 1000;
	let destination = AssetHubKusama::parent_location();
	let beneficiary_id = KusamaReceiver::get();
	let assets = (Parent, amount_to_send).into();

	let test_args = TestContext {
		sender: AssetHubKusamaSender::get(),
		receiver: KusamaReceiver::get(),
		args: TestArgs::new_para(destination, beneficiary_id, amount_to_send, assets, None, 0),
	};

	let mut test = SystemParaToRelayTest::new(test_args);

	let sender_balance_before = test.sender.balance;
	let receiver_balance_before = test.receiver.balance;

	test.set_assertion::<AssetHubKusama>(para_origin_assertions);
	test.set_assertion::<Kusama>(relay_dest_assertions_fail);
	test.set_dispatchable::<AssetHubKusama>(system_para_limited_teleport_assets);
	test.assert();

	let sender_balance_after = test.sender.balance;
	let receiver_balance_after = test.receiver.balance;

	let delivery_fees = AssetHubKusama::execute_with(|| {
		xcm_helpers::teleport_assets_delivery_fees::<
			<AssetHubKusamaXcmConfig as xcm_executor::Config>::XcmSender,
		>(
			test.args.assets.clone(), 0, test.args.weight_limit, test.args.beneficiary, test.args.dest
		)
	});

	// Sender's balance is reduced
	assert_eq!(sender_balance_before - amount_to_send - delivery_fees, sender_balance_after);
	// Receiver's balance does not change
	assert_eq!(receiver_balance_after, receiver_balance_before);
}

/// Bidirectional teleports of local Penpal assets to Asset Hub as foreign assets while paying
/// fees using (reserve transferred) native asset.
pub fn do_bidirectional_teleport_foreign_assets_between_para_and_asset_hub_using_xt(
	para_to_ah_dispatchable: fn(ParaToSystemParaTest) -> DispatchResult,
	ah_to_para_dispatchable: fn(SystemParaToParaTest) -> DispatchResult,
) {
	// Init values for Parachain
	let fee_amount_to_send: Balance = ASSET_HUB_KUSAMA_ED * 10000;
	let asset_location_on_penpal = PenpalA::execute_with(PenpalLocalTeleportableToAssetHub::get);
	let asset_id_on_penpal = match asset_location_on_penpal.last() {
		Some(Junction::GeneralIndex(id)) => *id as u32,
		_ => unreachable!(),
	};
	let asset_amount_to_send = ASSET_HUB_KUSAMA_ED * 1000;
	let asset_owner = PenpalAssetOwner::get();
	let system_para_native_asset_location = KsmLocation::get();
	let sender = PenpalASender::get();
	let penpal_check_account = <PenpalA as PenpalAPallet>::PolkadotXcm::check_account();
	let ah_as_seen_by_penpal = PenpalA::sibling_location_of(AssetHubKusama::para_id());
	let penpal_assets: Assets = vec![
		(Parent, fee_amount_to_send).into(),
		(asset_location_on_penpal.clone(), asset_amount_to_send).into(),
	]
	.into();
	let fee_asset_index = penpal_assets
		.inner()
		.iter()
		.position(|r| r == &(Parent, fee_amount_to_send).into())
		.unwrap() as u32;

	// fund Parachain's sender account
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(asset_owner.clone()),
		system_para_native_asset_location.clone(),
		sender.clone(),
		fee_amount_to_send * 2,
	);
	// No need to create the asset (only mint) as it exists in genesis.
	PenpalA::mint_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(asset_owner.clone()),
		asset_id_on_penpal,
		sender.clone(),
		asset_amount_to_send,
	);
	// fund Parachain's check account to be able to teleport
	PenpalA::fund_accounts(vec![(penpal_check_account.clone(), ASSET_HUB_KUSAMA_ED * 1000)]);

	// prefund SA of Penpal on AssetHub with enough native tokens to pay for fees
	let penpal_as_seen_by_ah = AssetHubKusama::sibling_location_of(PenpalA::para_id());
	let sov_penpal_on_ah = AssetHubKusama::sovereign_account_id_of(penpal_as_seen_by_ah);
	AssetHubKusama::fund_accounts(vec![(
		sov_penpal_on_ah.clone(),
		ASSET_HUB_KUSAMA_ED * 100_000_000_000,
	)]);

	// Init values for System Parachain
	let foreign_asset_at_asset_hub_kusama =
		Location::new(1, [Parachain(PenpalA::para_id().into())])
			.appended_with(asset_location_on_penpal)
			.unwrap();
	let penpal_to_ah_beneficiary_id = AssetHubKusamaReceiver::get();

	// Penpal to AH test args
	let penpal_to_ah_test_args = TestContext {
		sender: PenpalASender::get(),
		receiver: AssetHubKusamaReceiver::get(),
		args: TestArgs::new_para(
			ah_as_seen_by_penpal,
			penpal_to_ah_beneficiary_id,
			asset_amount_to_send,
			penpal_assets,
			Some(asset_id_on_penpal),
			fee_asset_index,
		),
	};
	let mut penpal_to_ah = ParaToSystemParaTest::new(penpal_to_ah_test_args);
	let penpal_sender_balance_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(
			system_para_native_asset_location.clone(),
			&PenpalASender::get(),
		)
	});

	let ah_receiver_balance_before = penpal_to_ah.receiver.balance;

	let penpal_sender_assets_before = PenpalA::execute_with(|| {
		type Assets = <PenpalA as PenpalAPallet>::Assets;
		<Assets as Inspect<_>>::balance(asset_id_on_penpal, &PenpalASender::get())
	});
	let ah_receiver_assets_before = AssetHubKusama::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(
			foreign_asset_at_asset_hub_kusama.clone(),
			&AssetHubKusamaReceiver::get(),
		)
	});

	penpal_to_ah.set_assertion::<PenpalA>(penpal_to_ah_foreign_assets_sender_assertions);
	penpal_to_ah.set_assertion::<AssetHubKusama>(penpal_to_ah_foreign_assets_receiver_assertions);
	penpal_to_ah.set_dispatchable::<PenpalA>(para_to_ah_dispatchable);
	penpal_to_ah.assert();

	let penpal_sender_balance_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(
			system_para_native_asset_location.clone(),
			&PenpalASender::get(),
		)
	});

	let ah_receiver_balance_after = penpal_to_ah.receiver.balance;

	let penpal_sender_assets_after = PenpalA::execute_with(|| {
		type Assets = <PenpalA as PenpalAPallet>::Assets;
		<Assets as Inspect<_>>::balance(asset_id_on_penpal, &PenpalASender::get())
	});
	let ah_receiver_assets_after = AssetHubKusama::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(
			foreign_asset_at_asset_hub_kusama.clone(),
			&AssetHubKusamaReceiver::get(),
		)
	});

	// Sender's balance is reduced
	assert!(penpal_sender_balance_after < penpal_sender_balance_before);
	// Receiver's balance is increased
	assert!(ah_receiver_balance_after > ah_receiver_balance_before);
	// Receiver's balance increased by `amount_to_send - delivery_fees - bought_execution`;
	// `delivery_fees` might be paid from transfer or JIT, also `bought_execution` is unknown but
	// should be non-zero
	assert!(ah_receiver_balance_after < ah_receiver_balance_before + fee_amount_to_send);

	// Sender's balance is reduced by exact amount
	assert_eq!(penpal_sender_assets_before - asset_amount_to_send, penpal_sender_assets_after);
	// Receiver's balance is increased by exact amount
	assert_eq!(ah_receiver_assets_after, ah_receiver_assets_before + asset_amount_to_send);

	///////////////////////////////////////////////////////////////////////
	// Now test transferring foreign assets back from AssetHub to Penpal //
	///////////////////////////////////////////////////////////////////////

	// Move funds on AH from AHReceiver to AHSender
	AssetHubKusama::execute_with(|| {
		type ForeignAssets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		assert_ok!(ForeignAssets::transfer(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(AssetHubKusamaReceiver::get()),
			foreign_asset_at_asset_hub_kusama.clone(),
			AssetHubKusamaSender::get().into(),
			asset_amount_to_send,
		));
	});

	let ah_to_penpal_beneficiary_id = PenpalAReceiver::get();
	let penpal_as_seen_by_ah = AssetHubKusama::sibling_location_of(PenpalA::para_id());
	let foreign_asset_at_asset_hub_kusama_latest: Location =
		foreign_asset_at_asset_hub_kusama.clone();
	let ah_assets: Assets = vec![
		(Parent, fee_amount_to_send).into(),
		(foreign_asset_at_asset_hub_kusama_latest.clone(), asset_amount_to_send).into(),
	]
	.into();
	let fee_asset_index = ah_assets
		.inner()
		.iter()
		.position(|r| r == &(Parent, fee_amount_to_send).into())
		.unwrap() as u32;

	// AH to Penpal test args
	let ah_to_penpal_test_args = TestContext {
		sender: AssetHubKusamaSender::get(),
		receiver: PenpalAReceiver::get(),
		args: TestArgs::new_para(
			penpal_as_seen_by_ah,
			ah_to_penpal_beneficiary_id,
			asset_amount_to_send,
			ah_assets,
			Some(asset_id_on_penpal),
			fee_asset_index,
		),
	};
	let mut ah_to_penpal = SystemParaToParaTest::new(ah_to_penpal_test_args);

	let ah_sender_balance_before = ah_to_penpal.sender.balance;
	let penpal_receiver_balance_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(
			system_para_native_asset_location.clone(),
			&PenpalAReceiver::get(),
		)
	});

	let ah_sender_assets_before = AssetHubKusama::execute_with(|| {
		type ForeignAssets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(
			foreign_asset_at_asset_hub_kusama.clone(),
			&AssetHubKusamaSender::get(),
		)
	});
	let penpal_receiver_assets_before = PenpalA::execute_with(|| {
		type Assets = <PenpalA as PenpalAPallet>::Assets;
		<Assets as Inspect<_>>::balance(asset_id_on_penpal, &PenpalAReceiver::get())
	});

	ah_to_penpal.set_assertion::<AssetHubKusama>(ah_to_penpal_foreign_assets_sender_assertions);
	ah_to_penpal.set_assertion::<PenpalA>(ah_to_penpal_foreign_assets_receiver_assertions);
	ah_to_penpal.set_dispatchable::<AssetHubKusama>(ah_to_para_dispatchable);
	ah_to_penpal.assert();

	let ah_sender_balance_after = ah_to_penpal.sender.balance;
	let penpal_receiver_balance_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(
			system_para_native_asset_location,
			&PenpalAReceiver::get(),
		)
	});

	let ah_sender_assets_after = AssetHubKusama::execute_with(|| {
		type ForeignAssets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(
			foreign_asset_at_asset_hub_kusama,
			&AssetHubKusamaSender::get(),
		)
	});
	let penpal_receiver_assets_after = PenpalA::execute_with(|| {
		type Assets = <PenpalA as PenpalAPallet>::Assets;
		<Assets as Inspect<_>>::balance(asset_id_on_penpal, &PenpalAReceiver::get())
	});

	// Sender's balance is reduced
	assert!(ah_sender_balance_after < ah_sender_balance_before);
	// Receiver's balance is increased
	assert!(penpal_receiver_balance_after > penpal_receiver_balance_before);
	// Receiver's balance increased by `amount_to_send - delivery_fees - bought_execution`;
	// `delivery_fees` might be paid from transfer or JIT, also `bought_execution` is unknown but
	// should be non-zero
	assert!(penpal_receiver_balance_after < penpal_receiver_balance_before + fee_amount_to_send);

	// Sender's balance is reduced by exact amount
	assert_eq!(ah_sender_assets_before - asset_amount_to_send, ah_sender_assets_after);
	// Receiver's balance is increased by exact amount
	assert_eq!(penpal_receiver_assets_after, penpal_receiver_assets_before + asset_amount_to_send);
}

/// Bidirectional teleports of local Penpal assets to Asset Hub as foreign assets should work
/// (using native reserve-based transfer for fees)
#[test]
fn bidirectional_teleport_foreign_assets_between_para_and_asset_hub() {
	do_bidirectional_teleport_foreign_assets_between_para_and_asset_hub_using_xt(
		para_to_system_para_transfer_assets,
		system_para_to_para_transfer_assets,
	);
}
