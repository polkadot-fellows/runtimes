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
use kusama_system_emulated_network::penpal_emulated_chain::XcmConfig as PenpalKusamaXcmConfig;

fn relay_to_para_sender_assertions(t: RelayToParaTest) {
	type RuntimeEvent = <Kusama as Chain>::RuntimeEvent;
	Kusama::assert_xcm_pallet_attempted_complete(Some(Weight::from_parts(864_610_000, 8_799)));

	assert_expected_events!(
		Kusama,
		vec![
			// Amount to reserve transfer is transferred to Parachain's Sovereign account
			RuntimeEvent::Balances(
				pallet_balances::Event::Transfer { from, to, amount }
			) => {
				from: *from == t.sender.account_id,
				to: *to == Kusama::sovereign_account_id_of(
					t.args.dest.clone()
				),
				amount: *amount == t.args.amount,
			},
		]
	);
}

fn system_para_to_para_sender_assertions(t: SystemParaToParaTest) {
	type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
	AssetHubKusama::assert_xcm_pallet_attempted_complete(Some(Weight::from_parts(
		864_610_000,
		8_799,
	)));

	assert_expected_events!(
		AssetHubKusama,
		vec![
			// Amount to reserve transfer is transferred to Parachain's Sovereign account
			RuntimeEvent::Balances(
				pallet_balances::Event::Transfer { from, to, amount }
			) => {
				from: *from == t.sender.account_id,
				to: *to == AssetHubKusama::sovereign_account_id_of(
					t.args.dest.clone()
				),
				amount: *amount == t.args.amount,
			},
		]
	);
}

fn para_receiver_assertions<Test>(_: Test) {
	type RuntimeEvent = <PenpalA as Chain>::RuntimeEvent;
	assert_expected_events!(
		PenpalA,
		vec![
			RuntimeEvent::Balances(pallet_balances::Event::Deposit { .. }) => {},
			RuntimeEvent::MessageQueue(
				pallet_message_queue::Event::Processed { success: true, .. }
			) => {},
		]
	);
}

fn para_to_system_para_sender_assertions(t: ParaToSystemParaTest) {
	type RuntimeEvent = <PenpalA as Chain>::RuntimeEvent;
	PenpalA::assert_xcm_pallet_attempted_complete(Some(Weight::from_parts(864_610_000, 8_799)));

	assert_expected_events!(
		PenpalA,
		vec![
			// Amount to reserve transfer is transferred to Parachain's Sovereign account
			RuntimeEvent::Balances(
				pallet_balances::Event::Withdraw { who, amount }
			) => {
				who: *who == t.sender.account_id,
				amount: *amount == t.args.amount,
			},
		]
	);
}

fn para_to_system_para_receiver_assertions(t: ParaToSystemParaTest) {
	type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
	let sov_penpal_on_ahk = AssetHubKusama::sovereign_account_id_of(
		AssetHubKusama::sibling_location_of(PenpalA::para_id()),
	);

	assert_expected_events!(
		AssetHubKusama,
		vec![
			// Amount to reserve transfer is withdrawn from Parachain's Sovereign account
			RuntimeEvent::Balances(
				pallet_balances::Event::Burned { who, amount }
			) => {
				who: *who == sov_penpal_on_ahk.clone(),
				amount: *amount == t.args.amount,
			},
			RuntimeEvent::Balances(pallet_balances::Event::Minted { .. }) => {},
			RuntimeEvent::MessageQueue(
				pallet_message_queue::Event::Processed { success: true, .. }
			) => {},
		]
	);
}

fn system_para_to_para_assets_sender_assertions(t: SystemParaToParaTest) {
	type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
	AssetHubKusama::assert_xcm_pallet_attempted_complete(Some(Weight::from_parts(
		864_610_000,
		8799,
	)));

	assert_expected_events!(
		AssetHubKusama,
		vec![
			// Amount to reserve transfer is transferred to Parachain's Sovereign account
			RuntimeEvent::Assets(
				pallet_assets::Event::Transferred { asset_id, from, to, amount }
			) => {
				asset_id: *asset_id == ASSET_ID,
				from: *from == t.sender.account_id,
				to: *to == AssetHubKusama::sovereign_account_id_of(
					t.args.dest.clone()
				),
				amount: *amount == t.args.amount,
			},
		]
	);
}

fn system_para_to_para_assets_receiver_assertions<Test>(_: Test) {
	type RuntimeEvent = <PenpalA as Chain>::RuntimeEvent;
	assert_expected_events!(
		PenpalA,
		vec![
			RuntimeEvent::Balances(pallet_balances::Event::Deposit { .. }) => {},
			RuntimeEvent::Assets(pallet_assets::Event::Issued { .. }) => {},
			RuntimeEvent::MessageQueue(
				pallet_message_queue::Event::Processed { success: true, .. }
			) => {},
		]
	);
}

fn para_to_para_sender_assertions(t: ParaToParaTest) {
	type RuntimeEvent = <PenpalA as Chain>::RuntimeEvent;
	PenpalA::assert_xcm_pallet_attempted_complete(None);
	assert_expected_events!(
		PenpalA,
		vec![
			// Amount to reserve transfer is transferred to Parachain's Sovereign account
			RuntimeEvent::Balances(
				pallet_balances::Event::Withdraw { who, amount }
			) => {
				who: *who == t.sender.account_id,
				amount: *amount == t.args.amount,
			},
			// XCM sent to relay reserve
			RuntimeEvent::ParachainSystem(
				cumulus_pallet_parachain_system::Event::UpwardMessageSent { .. }
			) => {},
		]
	);
}

fn para_to_para_relay_hop_assertions(t: ParaToParaTest) {
	type RuntimeEvent = <Kusama as Chain>::RuntimeEvent;
	let sov_penpal_a_on_kusama =
		Kusama::sovereign_account_id_of(Kusama::child_location_of(PenpalA::para_id()));
	let sov_penpal_b_on_kusama =
		Kusama::sovereign_account_id_of(Kusama::child_location_of(PenpalB::para_id()));
	assert_expected_events!(
		Kusama,
		vec![
			// Withdrawn from sender parachain SA
			RuntimeEvent::Balances(
				pallet_balances::Event::Withdraw { who, amount } | pallet_balances::Event::Burned { who, amount }
			) => {
				who: *who == sov_penpal_a_on_kusama,
				amount: *amount == t.args.amount,
			},
			// Deposited to receiver parachain SA
			RuntimeEvent::Balances(
				pallet_balances::Event::Deposit { who, .. } | pallet_balances::Event::Minted { who, .. }
			) => {
				who: *who == sov_penpal_b_on_kusama,
			},
			RuntimeEvent::MessageQueue(
				pallet_message_queue::Event::Processed { success: true, .. }
			) => {},
		]
	);
}

fn para_to_para_receiver_assertions(_: ParaToParaTest) {
	type RuntimeEvent = <PenpalB as Chain>::RuntimeEvent;
	assert_expected_events!(
		PenpalB,
		vec![
			RuntimeEvent::Balances(pallet_balances::Event::Deposit { .. }) => {},
			RuntimeEvent::MessageQueue(
				pallet_message_queue::Event::Processed { success: true, .. }
			) => {},
		]
	);
}

fn relay_to_para_reserve_transfer_assets(t: RelayToParaTest) -> DispatchResult {
	<Kusama as KusamaPallet>::XcmPallet::limited_reserve_transfer_assets(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.beneficiary.into()),
		bx!(t.args.assets.into()),
		t.args.fee_asset_item,
		t.args.weight_limit,
	)
}

fn system_para_to_para_reserve_transfer_assets(t: SystemParaToParaTest) -> DispatchResult {
	<AssetHubKusama as AssetHubKusamaPallet>::PolkadotXcm::limited_reserve_transfer_assets(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.beneficiary.into()),
		bx!(t.args.assets.into()),
		t.args.fee_asset_item,
		t.args.weight_limit,
	)
}

fn para_to_system_para_reserve_transfer_assets(t: ParaToSystemParaTest) -> DispatchResult {
	<PenpalA as PenpalAPallet>::PolkadotXcm::limited_reserve_transfer_assets(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.beneficiary.into()),
		bx!(t.args.assets.into()),
		t.args.fee_asset_item,
		t.args.weight_limit,
	)
}

fn para_to_para_limited_reserve_transfer_assets(t: ParaToParaTest) -> DispatchResult {
	<PenpalA as PenpalAPallet>::PolkadotXcm::limited_reserve_transfer_assets(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.beneficiary.into()),
		bx!(t.args.assets.into()),
		t.args.fee_asset_item,
		t.args.weight_limit,
	)
}

/// Reserve Transfers of native asset from Relay Chain to the System Parachain shouldn't work
#[test]
fn reserve_transfer_native_asset_from_relay_to_system_para_fails() {
	let signed_origin = <Kusama as Chain>::RuntimeOrigin::signed(KusamaSender::get());
	let destination = Kusama::child_location_of(AssetHubKusama::para_id());
	let beneficiary: Location =
		AccountId32Junction { network: None, id: AssetHubKusamaReceiver::get().into() }.into();
	let amount_to_send: Balance = KUSAMA_ED * 1000;
	let assets: Assets = (Here, amount_to_send).into();
	let fee_asset_item = 0;

	// this should fail
	Kusama::execute_with(|| {
		let result = <Kusama as KusamaPallet>::XcmPallet::limited_reserve_transfer_assets(
			signed_origin,
			bx!(destination.into()),
			bx!(beneficiary.into()),
			bx!(assets.into()),
			fee_asset_item,
			WeightLimit::Unlimited,
		);
		assert_err!(
			result,
			DispatchError::Module(sp_runtime::ModuleError {
				index: 99,
				error: [2, 0, 0, 0],
				message: Some("Filtered")
			})
		);
	});
}

/// Reserve Transfers of native asset from System Parachain to Relay Chain shouldn't work
#[test]
fn reserve_transfer_native_asset_from_system_para_to_relay_fails() {
	// Init values for System Parachain
	let signed_origin =
		<AssetHubKusama as Chain>::RuntimeOrigin::signed(AssetHubKusamaSender::get());
	let destination = AssetHubKusama::parent_location();
	let beneficiary_id = KusamaReceiver::get();
	let beneficiary: Location =
		AccountId32Junction { network: None, id: beneficiary_id.into() }.into();
	let amount_to_send: Balance = ASSET_HUB_KUSAMA_ED * 1000;

	let assets: Assets = (Parent, amount_to_send).into();
	let fee_asset_item = 0;

	// this should fail
	AssetHubKusama::execute_with(|| {
		let result =
			<AssetHubKusama as AssetHubKusamaPallet>::PolkadotXcm::limited_reserve_transfer_assets(
				signed_origin,
				bx!(destination.into()),
				bx!(beneficiary.into()),
				bx!(assets.into()),
				fee_asset_item,
				WeightLimit::Unlimited,
			);
		assert_err!(
			result,
			DispatchError::Module(sp_runtime::ModuleError {
				index: 31,
				error: [2, 0, 0, 0],
				message: Some("Filtered")
			})
		);
	});
}

/// Reserve Transfers of native asset from Relay to Parachain should work
#[test]
fn reserve_transfer_native_asset_from_relay_to_para() {
	// Init values for Relay
	let destination = Kusama::child_location_of(PenpalA::para_id());
	let beneficiary_id = PenpalAReceiver::get();
	let amount_to_send: Balance = KUSAMA_ED * 1000;

	let test_args = TestContext {
		sender: KusamaSender::get(),
		receiver: PenpalAReceiver::get(),
		args: TestArgs::new_relay(destination, beneficiary_id, amount_to_send),
	};

	let mut test = RelayToParaTest::new(test_args);

	let sender_balance_before = test.sender.balance;
	let receiver_balance_before = test.receiver.balance;

	test.set_assertion::<Kusama>(relay_to_para_sender_assertions);
	test.set_assertion::<PenpalA>(para_receiver_assertions);
	test.set_dispatchable::<Kusama>(relay_to_para_reserve_transfer_assets);
	test.assert();

	let delivery_fees = Kusama::execute_with(|| {
		xcm_helpers::teleport_assets_delivery_fees::<
			<KusamaXcmConfig as xcm_executor::Config>::XcmSender,
		>(test.args.assets.clone(), 0, test.args.weight_limit, test.args.beneficiary, test.args.dest)
	});

	let sender_balance_after = test.sender.balance;
	let receiver_balance_after = test.receiver.balance;

	// Sender's balance is reduced
	assert_eq!(sender_balance_before - amount_to_send - delivery_fees, sender_balance_after);
	// Receiver's balance is increased
	assert!(receiver_balance_after > receiver_balance_before);
	// Receiver's balance increased by `amount_to_send - delivery_fees - bought_execution`;
	// `delivery_fees` might be paid from transfer or JIT, also `bought_execution` is unknown but
	// should be non-zero
	assert!(receiver_balance_after < receiver_balance_before + amount_to_send);
}

/// Reserve Transfers of native asset from System Parachain to Parachain should work
#[test]
fn reserve_transfer_native_asset_from_system_para_to_para() {
	// Init values for System Parachain
	let destination = AssetHubKusama::sibling_location_of(PenpalA::para_id());
	let beneficiary_id = PenpalAReceiver::get();
	let amount_to_send: Balance = ASSET_HUB_KUSAMA_ED * 1000;
	let assets = (Parent, amount_to_send).into();

	let test_args = TestContext {
		sender: AssetHubKusamaSender::get(),
		receiver: PenpalAReceiver::get(),
		args: TestArgs::new_para(destination, beneficiary_id, amount_to_send, assets, None, 0),
	};

	let mut test = SystemParaToParaTest::new(test_args);

	let sender_balance_before = test.sender.balance;
	let receiver_balance_before = test.receiver.balance;

	test.set_assertion::<AssetHubKusama>(system_para_to_para_sender_assertions);
	test.set_assertion::<PenpalA>(para_receiver_assertions);
	test.set_dispatchable::<AssetHubKusama>(system_para_to_para_reserve_transfer_assets);
	test.assert();

	let sender_balance_after = test.sender.balance;
	let receiver_balance_after = test.receiver.balance;

	let delivery_fees = AssetHubKusama::execute_with(|| {
		xcm_helpers::teleport_assets_delivery_fees::<
			<AssetHubKusamaXcmConfig as xcm_executor::Config>::XcmSender,
		>(test.args.assets.clone(), 0, test.args.weight_limit, test.args.beneficiary, test.args.dest)
	});

	// Sender's balance is reduced
	assert_eq!(sender_balance_before - amount_to_send - delivery_fees, sender_balance_after);
	// Receiver's balance is increased
	assert!(receiver_balance_after > receiver_balance_before);
	// Receiver's balance increased by `amount_to_send - delivery_fees - bought_execution`;
	// `delivery_fees` might be paid from transfer or JIT, also `bought_execution` is unknown but
	// should be non-zero
	assert!(receiver_balance_after < receiver_balance_before + amount_to_send);
}

/// Reserve Transfers of native asset from Parachain to System Parachain should work
#[test]
fn reserve_transfer_native_asset_from_para_to_system_para() {
	// Init values for Penpal Parachain
	let destination = PenpalA::sibling_location_of(AssetHubKusama::para_id());
	let beneficiary_id = AssetHubKusamaReceiver::get();
	let amount_to_send: Balance = ASSET_HUB_KUSAMA_ED * 1000;
	let assets = (Parent, amount_to_send).into();

	let test_args = TestContext {
		sender: PenpalASender::get(),
		receiver: AssetHubKusamaReceiver::get(),
		args: TestArgs::new_para(destination, beneficiary_id, amount_to_send, assets, None, 0),
	};

	let mut test = ParaToSystemParaTest::new(test_args);

	let sender_balance_before = test.sender.balance;
	let receiver_balance_before = test.receiver.balance;

	let penpal_location_as_seen_by_ahk = AssetHubKusama::sibling_location_of(PenpalA::para_id());
	let sov_penpal_on_ahk = AssetHubKusama::sovereign_account_id_of(penpal_location_as_seen_by_ahk);

	// fund the Penpal's SA on AHK with the native tokens held in reserve
	AssetHubKusama::fund_accounts(vec![(sov_penpal_on_ahk, amount_to_send * 2)]);

	test.set_assertion::<PenpalA>(para_to_system_para_sender_assertions);
	test.set_assertion::<AssetHubKusama>(para_to_system_para_receiver_assertions);
	test.set_dispatchable::<PenpalA>(para_to_system_para_reserve_transfer_assets);
	test.assert();

	let sender_balance_after = test.sender.balance;
	let receiver_balance_after = test.receiver.balance;

	let delivery_fees = PenpalA::execute_with(|| {
		xcm_helpers::teleport_assets_delivery_fees::<
			<PenpalKusamaXcmConfig as xcm_executor::Config>::XcmSender,
		>(test.args.assets.clone(), 0, test.args.weight_limit, test.args.beneficiary, test.args.dest)
	});

	// Sender's balance is reduced
	assert_eq!(sender_balance_before - amount_to_send - delivery_fees, sender_balance_after);
	// Receiver's balance is increased
	assert!(receiver_balance_after > receiver_balance_before);
	// Receiver's balance increased by `amount_to_send - delivery_fees - bought_execution`;
	// `delivery_fees` might be paid from transfer or JIT, also `bought_execution` is unknown but
	// should be non-zero
	assert!(receiver_balance_after < receiver_balance_before + amount_to_send);
}

/// Reserve Transfers of a local asset and native asset from System Parachain to Parachain should
/// work
#[test]
fn reserve_transfer_assets_from_system_para_to_para() {
	// FAIL-CI @bkontur pls fix
}

/// Reserve Transfers of native asset from Parachain to Parachain (through Relay reserve) should
/// work
#[test]
fn reserve_transfer_native_asset_from_para_to_para() {
	// Init values for Penpal Parachain
	let destination = PenpalA::sibling_location_of(PenpalB::para_id());
	let beneficiary_id = PenpalBReceiver::get();
	let amount_to_send: Balance = ASSET_HUB_KUSAMA_ED * 10000;
	let assets = (Parent, amount_to_send).into();

	let test_args = TestContext {
		sender: PenpalASender::get(),
		receiver: PenpalBReceiver::get(),
		args: TestArgs::new_para(destination, beneficiary_id, amount_to_send, assets, None, 0),
	};

	let mut test = ParaToParaTest::new(test_args);

	let sender_balance_before = test.sender.balance;
	let receiver_balance_before = test.receiver.balance;

	let sender_as_seen_by_relay = Kusama::child_location_of(PenpalA::para_id());
	let sov_of_sender_on_relay = Kusama::sovereign_account_id_of(sender_as_seen_by_relay);

	// fund the PenpalA's SA on Kusama with the native tokens held in reserve
	Kusama::fund_accounts(vec![(sov_of_sender_on_relay, amount_to_send * 2)]);

	test.set_assertion::<PenpalA>(para_to_para_sender_assertions);
	test.set_assertion::<Kusama>(para_to_para_relay_hop_assertions);
	test.set_assertion::<PenpalB>(para_to_para_receiver_assertions);
	test.set_dispatchable::<PenpalA>(para_to_para_limited_reserve_transfer_assets);
	test.assert();

	let sender_balance_after = test.sender.balance;
	let receiver_balance_after = test.receiver.balance;

	let delivery_fees = PenpalA::execute_with(|| {
		xcm_helpers::teleport_assets_delivery_fees::<
			<PenpalKusamaXcmConfig as xcm_executor::Config>::XcmSender,
		>(test.args.assets.clone(), 0, test.args.weight_limit, test.args.beneficiary, test.args.dest)
	});

	// Sender's balance is reduced
	assert_eq!(sender_balance_before - amount_to_send - delivery_fees, sender_balance_after);
	// Receiver's balance is increased
	assert!(receiver_balance_after > receiver_balance_before);
}
