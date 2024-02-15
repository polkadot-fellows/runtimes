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
use asset_hub_kusama_runtime::xcm_config::XcmConfig as AssetHubKusamaXcmConfig;
use kusama_runtime::xcm_config::XcmConfig as KusamaXcmConfig;

fn relay_origin_assertions(t: RelayToSystemParaTest) {
	type RuntimeEvent = <Kusama as Chain>::RuntimeEvent;

	Kusama::assert_xcm_pallet_attempted_complete(Some(Weight::from_parts(631_531_000, 7_186)));

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

fn relay_dest_assertions(t: SystemParaToRelayTest) {
	type RuntimeEvent = <Kusama as Chain>::RuntimeEvent;

	Kusama::assert_ump_queue_processed(
		true,
		Some(AssetHubKusama::para_id()),
		Some(Weight::from_parts(307_225_000, 7_186)),
	);

	assert_expected_events!(
		Kusama,
		vec![
			// Amount is withdrawn from Relay Chain's `CheckAccount`
			RuntimeEvent::Balances(pallet_balances::Event::Burned { who, amount }) => {
				who: *who == <Kusama as KusamaPallet>::XcmPallet::check_account(),
				amount: *amount == t.args.amount,
			},
			// Amount minus fees are deposited in Receiver's account
			RuntimeEvent::Balances(pallet_balances::Event::Minted { who, .. }) => {
				who: *who == t.receiver.account_id,
			},
		]
	);
}

fn relay_dest_assertions_fail(_t: SystemParaToRelayTest) {
	Kusama::assert_ump_queue_processed(
		false,
		Some(AssetHubKusama::para_id()),
		Some(Weight::from_parts(157_718_000, 3_593)),
	);
}

fn para_origin_assertions(t: SystemParaToRelayTest) {
	type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;

	AssetHubKusama::assert_xcm_pallet_attempted_complete(Some(Weight::from_parts(
		720_053_000,
		7_203,
	)));

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

fn para_dest_assertions(t: RelayToSystemParaTest) {
	type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;

	AssetHubKusama::assert_dmp_queue_complete(Some(Weight::from_parts(157_718_000, 3593)));

	assert_expected_events!(
		AssetHubKusama,
		vec![
			// Amount minus fees are deposited in Receiver's account
			RuntimeEvent::Balances(pallet_balances::Event::Minted { who, .. }) => {
				who: *who == t.receiver.account_id,
			},
		]
	);
}

fn penpal_to_ah_foreign_assets_sender_assertions(t: ParaToSystemParaTest) {
	type RuntimeEvent = <PenpalKusamaA as Chain>::RuntimeEvent;
	PenpalKusamaA::assert_xcm_pallet_attempted_complete(None);
	let expected_asset_id = t.args.asset_id.unwrap();
	let (_, expected_asset_amount) =
		non_fee_asset(&t.args.assets, t.args.fee_asset_item as usize).unwrap();
	assert_expected_events!(
		PenpalKusamaA,
		vec![
			RuntimeEvent::Balances(
				pallet_balances::Event::Withdraw { who, amount }
			) => {
				who: *who == t.sender.account_id,
				amount: *amount == t.args.amount,
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
	let sov_penpal_on_ahr = AssetHubKusama::sovereign_account_id_of(
		AssetHubKusama::sibling_location_of(PenpalKusamaA::para_id()),
	);
	let (expected_foreign_asset_id, expected_foreign_asset_amount) =
		non_fee_asset(&t.args.assets, t.args.fee_asset_item as usize).unwrap();
	assert_expected_events!(
		AssetHubKusama,
		vec![
			// native asset reserve transfer for paying fees, withdrawn from Penpal's sov account
			RuntimeEvent::Balances(
				pallet_balances::Event::Burned { who, amount }
			) => {
				who: *who == sov_penpal_on_ahr.clone().into(),
				amount: *amount == t.args.amount,
			},
			RuntimeEvent::Balances(pallet_balances::Event::Minted { who, .. }) => {
				who: *who == t.receiver.account_id,
			},
			RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, amount }) => {
				asset_id: *asset_id == expected_foreign_asset_id,
				owner: *owner == t.receiver.account_id,
				amount: *amount == expected_foreign_asset_amount,
			},
			RuntimeEvent::Balances(pallet_balances::Event::Deposit { .. }) => {},
			RuntimeEvent::MessageQueue(
				pallet_message_queue::Event::Processed { success: true, .. }
			) => {},
		]
	);
}

fn ah_to_penpal_foreign_assets_sender_assertions(t: SystemParaToParaTest) {
	type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
	AssetHubKusama::assert_xcm_pallet_attempted_complete(None);
	let (expected_foreign_asset_id, expected_foreign_asset_amount) =
		non_fee_asset(&t.args.assets, t.args.fee_asset_item as usize).unwrap();
	assert_expected_events!(
		AssetHubKusama,
		vec![
			// native asset used for fees is transferred to Parachain's Sovereign account as reserve
			RuntimeEvent::Balances(
				pallet_balances::Event::Transfer { from, to, amount }
			) => {
				from: *from == t.sender.account_id,
				to: *to == AssetHubKusama::sovereign_account_id_of(
					t.args.dest
				),
				amount: *amount == t.args.amount,
			},
			// foreign asset is burned locally as part of teleportation
			RuntimeEvent::ForeignAssets(pallet_assets::Event::Burned { asset_id, owner, balance }) => {
				asset_id: *asset_id == expected_foreign_asset_id,
				owner: *owner == t.sender.account_id,
				balance: *balance == expected_foreign_asset_amount,
			},
		]
	);
}

fn ah_to_penpal_foreign_assets_receiver_assertions(t: SystemParaToParaTest) {
	type RuntimeEvent = <PenpalKusamaA as Chain>::RuntimeEvent;
	let expected_asset_id = t.args.asset_id.unwrap();
	let (_, expected_asset_amount) =
		non_fee_asset(&t.args.assets, t.args.fee_asset_item as usize).unwrap();
	let checking_account = <PenpalKusamaA as PenpalKusamaAPallet>::PolkadotXcm::check_account();
	assert_expected_events!(
		PenpalKusamaA,
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
			RuntimeEvent::Balances(pallet_balances::Event::Deposit { who, .. }) => {
				who: *who == t.receiver.account_id,
			},
			RuntimeEvent::MessageQueue(
				pallet_message_queue::Event::Processed { success: true, .. }
			) => {},
		]
	);
}

fn relay_limited_teleport_assets(t: RelayToSystemParaTest) -> DispatchResult {
	<Kusama as KusamaPallet>::XcmPallet::limited_teleport_assets(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.beneficiary.into()),
		bx!(t.args.assets.into()),
		t.args.fee_asset_item,
		t.args.weight_limit,
	)
}

fn relay_teleport_assets(t: RelayToSystemParaTest) -> DispatchResult {
	<Kusama as KusamaPallet>::XcmPallet::teleport_assets(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.beneficiary.into()),
		bx!(t.args.assets.into()),
		t.args.fee_asset_item,
	)
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

fn system_para_teleport_assets(t: SystemParaToRelayTest) -> DispatchResult {
	<AssetHubKusama as AssetHubKusamaPallet>::PolkadotXcm::teleport_assets(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.beneficiary.into()),
		bx!(t.args.assets.into()),
		t.args.fee_asset_item,
	)
}

fn para_to_system_para_transfer_assets(t: ParaToSystemParaTest) -> DispatchResult {
	<PenpalKusamaA as PenpalKusamaAPallet>::PolkadotXcm::transfer_assets(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.beneficiary.into()),
		bx!(t.args.assets.into()),
		t.args.fee_asset_item,
		t.args.weight_limit,
	)
}

fn system_para_to_para_transfer_assets(t: SystemParaToParaTest) -> DispatchResult {
	<AssetHubKusama as AssetHubKusamaPallet>::PolkadotXcm::transfer_assets(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.beneficiary.into()),
		bx!(t.args.assets.into()),
		t.args.fee_asset_item,
		t.args.weight_limit,
	)
}

/// Limited Teleport of native asset from Relay Chain to the System Parachain should work
#[test]
fn limited_teleport_native_assets_from_relay_to_system_para_works() {
	// Init values for Relay Chain
	let amount_to_send: Balance = KUSAMA_ED * 1000;
	let dest = Kusama::child_location_of(AssetHubKusama::para_id());
	let beneficiary_id = AssetHubKusamaReceiver::get();
	let test_args = TestContext {
		sender: KusamaSender::get(),
		receiver: AssetHubKusamaReceiver::get(),
		args: relay_test_args(dest, beneficiary_id, amount_to_send),
	};

	let mut test = RelayToSystemParaTest::new(test_args);

	let sender_balance_before = test.sender.balance;
	let receiver_balance_before = test.receiver.balance;

	test.set_assertion::<Kusama>(relay_origin_assertions);
	test.set_assertion::<AssetHubKusama>(para_dest_assertions);
	test.set_dispatchable::<Kusama>(relay_limited_teleport_assets);
	test.assert();

	let delivery_fees = Kusama::execute_with(|| {
		xcm_helpers::transfer_assets_delivery_fees::<
			<KusamaXcmConfig as xcm_executor::Config>::XcmSender,
		>(test.args.assets.clone(), 0, test.args.weight_limit, test.args.beneficiary, test.args.dest)
	});

	let sender_balance_after = test.sender.balance;
	let receiver_balance_after = test.receiver.balance;

	// Sender's balance is reduced
	assert_eq!(sender_balance_before - amount_to_send - delivery_fees, sender_balance_after);
	// Receiver's balance is increased
	assert!(receiver_balance_after > receiver_balance_before);
}

/// Limited Teleport of native asset from System Parachain to Relay Chain
/// should work when there is enough balance in Relay Chain's `CheckAccount`
#[test]
fn limited_teleport_native_assets_back_from_system_para_to_relay_works() {
	// Dependency - Relay Chain's `CheckAccount` should have enough balance
	limited_teleport_native_assets_from_relay_to_system_para_works();

	// Init values for Relay Chain
	let amount_to_send: Balance = ASSET_HUB_KUSAMA_ED * 1000;
	let destination = AssetHubKusama::parent_location();
	let beneficiary_id = KusamaReceiver::get();
	let assets = (Parent, amount_to_send).into();

	let test_args = TestContext {
		sender: AssetHubKusamaSender::get(),
		receiver: KusamaReceiver::get(),
		args: para_test_args(destination, beneficiary_id, amount_to_send, assets, None, 0),
	};

	let mut test = SystemParaToRelayTest::new(test_args);

	let sender_balance_before = test.sender.balance;
	let receiver_balance_before = test.receiver.balance;

	test.set_assertion::<AssetHubKusama>(para_origin_assertions);
	test.set_assertion::<Kusama>(relay_dest_assertions);
	test.set_dispatchable::<AssetHubKusama>(system_para_limited_teleport_assets);
	test.assert();

	let sender_balance_after = test.sender.balance;
	let receiver_balance_after = test.receiver.balance;

	let delivery_fees = AssetHubKusama::execute_with(|| {
		xcm_helpers::transfer_assets_delivery_fees::<
			<AssetHubKusamaXcmConfig as xcm_executor::Config>::XcmSender,
		>(test.args.assets.clone(), 0, test.args.weight_limit, test.args.beneficiary, test.args.dest)
	});

	// Sender's balance is reduced
	assert_eq!(sender_balance_before - amount_to_send - delivery_fees, sender_balance_after);
	// Receiver's balance is increased
	assert!(receiver_balance_after > receiver_balance_before);
}

/// Limited Teleport of native asset from System Parachain to Relay Chain
/// should't work when there is not enough balance in Relay Chain's `CheckAccount`
#[test]
fn limited_teleport_native_assets_from_system_para_to_relay_fails() {
	// Init values for Relay Chain
	let amount_to_send: Balance = ASSET_HUB_KUSAMA_ED * 1000;
	let destination = AssetHubKusama::parent_location().into();
	let beneficiary_id = KusamaReceiver::get().into();
	let assets = (Parent, amount_to_send).into();

	let test_args = TestContext {
		sender: AssetHubKusamaSender::get(),
		receiver: KusamaReceiver::get(),
		args: para_test_args(destination, beneficiary_id, amount_to_send, assets, None, 0),
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
		xcm_helpers::transfer_assets_delivery_fees::<
			<AssetHubKusamaXcmConfig as xcm_executor::Config>::XcmSender,
		>(test.args.assets.clone(), 0, test.args.weight_limit, test.args.beneficiary, test.args.dest)
	});

	// Sender's balance is reduced
	assert_eq!(sender_balance_before - amount_to_send - delivery_fees, sender_balance_after);
	// Receiver's balance does not change
	assert_eq!(receiver_balance_after, receiver_balance_before);
}

/// Teleport of native asset from Relay Chain to the System Parachain should work
#[test]
fn teleport_native_assets_from_relay_to_system_para_works() {
	// Init values for Relay Chain
	let amount_to_send: Balance = KUSAMA_ED * 1000;
	let dest = Kusama::child_location_of(AssetHubKusama::para_id());
	let beneficiary_id = AssetHubKusamaReceiver::get();
	let test_args = TestContext {
		sender: KusamaSender::get(),
		receiver: AssetHubKusamaReceiver::get(),
		args: relay_test_args(dest, beneficiary_id, amount_to_send),
	};

	let mut test = RelayToSystemParaTest::new(test_args);

	let sender_balance_before = test.sender.balance;
	let receiver_balance_before = test.receiver.balance;

	test.set_assertion::<Kusama>(relay_origin_assertions);
	test.set_assertion::<AssetHubKusama>(para_dest_assertions);
	test.set_dispatchable::<Kusama>(relay_teleport_assets);
	test.assert();

	let delivery_fees = Kusama::execute_with(|| {
		xcm_helpers::transfer_assets_delivery_fees::<
			<KusamaXcmConfig as xcm_executor::Config>::XcmSender,
		>(test.args.assets.clone(), 0, test.args.weight_limit, test.args.beneficiary, test.args.dest)
	});

	let sender_balance_after = test.sender.balance;
	let receiver_balance_after = test.receiver.balance;

	// Sender's balance is reduced
	assert_eq!(sender_balance_before - amount_to_send - delivery_fees, sender_balance_after);
	// Receiver's balance is increased
	assert!(receiver_balance_after > receiver_balance_before);
}

/// Teleport of native asset from System Parachains to the Relay Chain
/// should work when there is enough balance in Relay Chain's `CheckAccount`
#[test]
fn teleport_native_assets_back_from_system_para_to_relay_works() {
	// Dependency - Relay Chain's `CheckAccount` should have enough balance
	teleport_native_assets_from_relay_to_system_para_works();

	// Init values for Relay Chain
	let amount_to_send: Balance = ASSET_HUB_KUSAMA_ED * 1000;
	let destination = AssetHubKusama::parent_location();
	let beneficiary_id = KusamaReceiver::get();
	let assets = (Parent, amount_to_send).into();

	let test_args = TestContext {
		sender: AssetHubKusamaSender::get(),
		receiver: KusamaReceiver::get(),
		args: para_test_args(destination, beneficiary_id, amount_to_send, assets, None, 0),
	};

	let mut test = SystemParaToRelayTest::new(test_args);

	let sender_balance_before = test.sender.balance;
	let receiver_balance_before = test.receiver.balance;

	test.set_assertion::<AssetHubKusama>(para_origin_assertions);
	test.set_assertion::<Kusama>(relay_dest_assertions);
	test.set_dispatchable::<AssetHubKusama>(system_para_teleport_assets);
	test.assert();

	let sender_balance_after = test.sender.balance;
	let receiver_balance_after = test.receiver.balance;

	let delivery_fees = AssetHubKusama::execute_with(|| {
		xcm_helpers::transfer_assets_delivery_fees::<
			<AssetHubKusamaXcmConfig as xcm_executor::Config>::XcmSender,
		>(test.args.assets.clone(), 0, test.args.weight_limit, test.args.beneficiary, test.args.dest)
	});

	// Sender's balance is reduced
	assert_eq!(sender_balance_before - amount_to_send - delivery_fees, sender_balance_after);
	// Receiver's balance is increased
	assert!(receiver_balance_after > receiver_balance_before);
}

/// Teleport of native asset from System Parachain to Relay Chain
/// shouldn't work when there is not enough balance in Relay Chain's `CheckAccount`
#[test]
fn teleport_native_assets_from_system_para_to_relay_fails() {
	// Init values for Relay Chain
	let amount_to_send: Balance = ASSET_HUB_KUSAMA_ED * 1000;
	let destination = AssetHubKusama::parent_location();
	let beneficiary_id = KusamaReceiver::get();
	let assets = (Parent, amount_to_send).into();

	let test_args = TestContext {
		sender: AssetHubKusamaSender::get(),
		receiver: KusamaReceiver::get(),
		args: para_test_args(destination, beneficiary_id, amount_to_send, assets, None, 0),
	};

	let mut test = SystemParaToRelayTest::new(test_args);

	let sender_balance_before = test.sender.balance;
	let receiver_balance_before = test.receiver.balance;

	test.set_assertion::<AssetHubKusama>(para_origin_assertions);
	test.set_assertion::<Kusama>(relay_dest_assertions_fail);
	test.set_dispatchable::<AssetHubKusama>(system_para_teleport_assets);
	test.assert();

	let delivery_fees = AssetHubKusama::execute_with(|| {
		xcm_helpers::transfer_assets_delivery_fees::<
			<AssetHubKusamaXcmConfig as xcm_executor::Config>::XcmSender,
		>(test.args.assets.clone(), 0, test.args.weight_limit, test.args.beneficiary, test.args.dest)
	});

	let sender_balance_after = test.sender.balance;
	let receiver_balance_after = test.receiver.balance;

	// Sender's balance is reduced
	assert_eq!(sender_balance_before - amount_to_send - delivery_fees, sender_balance_after);
	// Receiver's balance does not change
	assert_eq!(receiver_balance_after, receiver_balance_before);
}

#[test]
fn teleport_to_other_system_parachains_works() {
	let amount = ASSET_HUB_KUSAMA_ED * 100;
	let native_asset: MultiAssets = (Parent, amount).into();

	test_parachain_is_trusted_teleporter!(
		AssetHubKusama,          // Origin
		AssetHubKusamaXcmConfig, // XCM Configuration
		vec![BridgeHubKusama],   // Destinations
		(native_asset, amount)
	);
}

/// Bidirectional teleports of local Penpal assets to Asset Hub as foreign assets should work
/// (using native reserve-based transfer for fees)
#[test]
fn bidirectional_teleport_foreign_assets_between_para_and_asset_hub() {
	let ah_as_seen_by_penpal = PenpalKusamaA::sibling_location_of(AssetHubKusama::para_id());
	let asset_location_on_penpal = PenpalLocalTeleportableToAssetHub::get();
	let asset_id_on_penpal = match asset_location_on_penpal.last() {
		Some(GeneralIndex(id)) => *id as u32,
		_ => unreachable!(),
	};
	let asset_owner_on_penpal = PenpalKusamaASender::get();
	let foreign_asset_at_asset_hub_kusama =
		MultiLocation { parents: 1, interior: X1(Parachain(PenpalKusamaA::para_id().into())) }
			.appended_with(asset_location_on_penpal)
			.unwrap();
	super::penpal_create_foreign_asset_on_asset_hub(
		asset_id_on_penpal,
		foreign_asset_at_asset_hub_kusama,
		ah_as_seen_by_penpal,
		false,
		asset_owner_on_penpal,
		ASSET_MIN_BALANCE * 1_000_000,
	);
	let penpal_to_ah_beneficiary_id = AssetHubKusamaReceiver::get();

	let fee_amount_to_send = ASSET_HUB_KUSAMA_ED * 10_000;
	let asset_amount_to_send = ASSET_MIN_BALANCE * 1000;

	let penpal_assets: MultiAssets = vec![
		(Parent, fee_amount_to_send).into(),
		(asset_location_on_penpal, asset_amount_to_send).into(),
	]
	.into();
	let fee_asset_index = penpal_assets
		.inner()
		.iter()
		.position(|r| r == &(Parent, fee_amount_to_send).into())
		.unwrap() as u32;

	// Penpal to AH test args
	let penpal_to_ah_test_args = TestContext {
		sender: PenpalKusamaASender::get(),
		receiver: AssetHubKusamaReceiver::get(),
		args: para_test_args(
			ah_as_seen_by_penpal,
			penpal_to_ah_beneficiary_id,
			asset_amount_to_send,
			penpal_assets,
			Some(asset_id_on_penpal),
			fee_asset_index,
		),
	};
	let mut penpal_to_ah = ParaToSystemParaTest::new(penpal_to_ah_test_args);

	let penpal_sender_balance_before = penpal_to_ah.sender.balance;
	let ah_receiver_balance_before = penpal_to_ah.receiver.balance;

	let penpal_sender_assets_before = PenpalKusamaA::execute_with(|| {
		type Assets = <PenpalKusamaA as PenpalKusamaAPallet>::Assets;
		<Assets as Inspect<_>>::balance(asset_id_on_penpal, &PenpalKusamaASender::get())
	});
	let ah_receiver_assets_before = AssetHubKusama::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(
			foreign_asset_at_asset_hub_kusama,
			&AssetHubKusamaReceiver::get(),
		)
	});

	penpal_to_ah.set_assertion::<PenpalKusamaA>(penpal_to_ah_foreign_assets_sender_assertions);
	penpal_to_ah.set_assertion::<AssetHubKusama>(penpal_to_ah_foreign_assets_receiver_assertions);
	penpal_to_ah.set_dispatchable::<PenpalKusamaA>(para_to_system_para_transfer_assets);
	penpal_to_ah.assert();

	let penpal_sender_balance_after = penpal_to_ah.sender.balance;
	let ah_receiver_balance_after = penpal_to_ah.receiver.balance;

	let penpal_sender_assets_after = PenpalKusamaA::execute_with(|| {
		type Assets = <PenpalKusamaA as PenpalKusamaAPallet>::Assets;
		<Assets as Inspect<_>>::balance(asset_id_on_penpal, &PenpalKusamaASender::get())
	});
	let ah_receiver_assets_after = AssetHubKusama::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(
			foreign_asset_at_asset_hub_kusama,
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
			foreign_asset_at_asset_hub_kusama,
			AssetHubKusamaSender::get().into(),
			asset_amount_to_send,
		));
	});

	let ah_to_penpal_beneficiary_id = PenpalKusamaAReceiver::get();
	let penpal_as_seen_by_ah = AssetHubKusama::sibling_location_of(PenpalKusamaA::para_id());
	let ah_assets: MultiAssets = vec![
		(Parent, fee_amount_to_send).into(),
		(foreign_asset_at_asset_hub_kusama, asset_amount_to_send).into(),
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
		receiver: PenpalKusamaAReceiver::get(),
		args: para_test_args(
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
	let penpal_receiver_balance_before = ah_to_penpal.receiver.balance;

	let ah_sender_assets_before = AssetHubKusama::execute_with(|| {
		type ForeignAssets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(
			foreign_asset_at_asset_hub_kusama,
			&AssetHubKusamaSender::get(),
		)
	});
	let penpal_receiver_assets_before = PenpalKusamaA::execute_with(|| {
		type Assets = <PenpalKusamaA as PenpalKusamaAPallet>::Assets;
		<Assets as Inspect<_>>::balance(asset_id_on_penpal, &PenpalKusamaAReceiver::get())
	});

	ah_to_penpal.set_assertion::<AssetHubKusama>(ah_to_penpal_foreign_assets_sender_assertions);
	ah_to_penpal.set_assertion::<PenpalKusamaA>(ah_to_penpal_foreign_assets_receiver_assertions);
	ah_to_penpal.set_dispatchable::<AssetHubKusama>(system_para_to_para_transfer_assets);
	ah_to_penpal.assert();

	let ah_sender_balance_after = ah_to_penpal.sender.balance;
	let penpal_receiver_balance_after = ah_to_penpal.receiver.balance;

	let ah_sender_assets_after = AssetHubKusama::execute_with(|| {
		type ForeignAssets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(
			foreign_asset_at_asset_hub_kusama,
			&AssetHubKusamaSender::get(),
		)
	});
	let penpal_receiver_assets_after = PenpalKusamaA::execute_with(|| {
		type Assets = <PenpalKusamaA as PenpalKusamaAPallet>::Assets;
		<Assets as Inspect<_>>::balance(asset_id_on_penpal, &PenpalKusamaAReceiver::get())
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
