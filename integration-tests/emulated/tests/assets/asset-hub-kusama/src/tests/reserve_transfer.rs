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
use asset_hub_kusama_runtime::xcm_config::KsmLocation;
use kusama_system_emulated_network::penpal_emulated_chain::LocalReservableFromAssetHub as PenpalLocalReservableFromAssetHub;

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

fn para_to_relay_sender_assertions(t: ParaToRelayTest) {
	type RuntimeEvent = <PenpalA as Chain>::RuntimeEvent;
	PenpalA::assert_xcm_pallet_attempted_complete(None);
	assert_expected_events!(
		PenpalA,
		vec![
			// Amount to reserve transfer is transferred to Parachain's Sovereign account
			RuntimeEvent::ForeignAssets(
				pallet_assets::Event::Burned { asset_id, owner, balance, .. }
			) => {
				asset_id: *asset_id == KsmLocation::get(),
				owner: *owner == t.sender.account_id,
				balance: *balance == t.args.amount,
			},
		]
	);
}

pub fn system_para_to_para_sender_assertions(t: SystemParaToParaTest) {
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

pub fn system_para_to_para_receiver_assertions(t: SystemParaToParaTest) {
	type RuntimeEvent = <PenpalA as Chain>::RuntimeEvent;

	PenpalB::assert_xcmp_queue_success(None);
	for asset in t.args.assets.into_inner().into_iter() {
		let expected_id = asset.id.0;
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
}

fn relay_to_para_assets_receiver_assertions(t: RelayToParaTest) {
	type RuntimeEvent = <PenpalA as Chain>::RuntimeEvent;

	assert_expected_events!(
		PenpalA,
		vec![
			RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
				asset_id: *asset_id == KsmLocation::get(),
				owner: *owner == t.receiver.account_id,
			},
			RuntimeEvent::MessageQueue(
				pallet_message_queue::Event::Processed { success: true, .. }
			) => {},
		]
	);
}

fn para_to_relay_receiver_assertions(t: ParaToRelayTest) {
	type RuntimeEvent = <Kusama as Chain>::RuntimeEvent;
	let sov_penpal_on_relay =
		Kusama::sovereign_account_id_of(Kusama::child_location_of(PenpalA::para_id()));

	Kusama::assert_ump_queue_processed(true, Some(PenpalA::para_id()), None);
	assert_expected_events!(
		Kusama,
		vec![
			// Amount to reserve transfer is withdrawn from Parachain's Sovereign account
			RuntimeEvent::Balances(
				pallet_balances::Event::Burned { who, amount }
			) => {
				who: *who == sov_penpal_on_relay.clone(),
				amount: *amount == t.args.amount,
			},
			RuntimeEvent::Balances(pallet_balances::Event::Minted { .. }) => {},
			RuntimeEvent::MessageQueue(
				pallet_message_queue::Event::Processed { success: true, .. }
			) => {},
		]
	);
}

pub fn para_to_system_para_sender_assertions(t: ParaToSystemParaTest) {
	type RuntimeEvent = <PenpalA as Chain>::RuntimeEvent;

	PenpalA::assert_xcm_pallet_attempted_complete(None);
	for asset in t.args.assets.into_inner().into_iter() {
		let expected_id = asset.id.0;
		let asset_amount = if let Fungible(a) = asset.fun { Some(a) } else { None }.unwrap();
		assert_expected_events!(
			PenpalA,
			vec![
				RuntimeEvent::ForeignAssets(
					pallet_assets::Event::Burned { asset_id, owner, balance }
				) => {
					asset_id: *asset_id == expected_id,
					owner: *owner == t.sender.account_id,
					balance: *balance == asset_amount,
				},
			]
		);
	}
}

pub fn para_to_system_para_receiver_assertions(t: ParaToSystemParaTest) {
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
				asset_id: *asset_id == RESERVABLE_ASSET_ID,
				from: *from == t.sender.account_id,
				to: *to == AssetHubKusama::sovereign_account_id_of(
					t.args.dest.clone()
				),
				amount: *amount == t.args.amount,
			},
			// Native asset to pay for fees is transferred to Parachain's Sovereign account
			RuntimeEvent::Balances(pallet_balances::Event::Minted { who, .. }) => {
				who: *who == AssetHubKusama::sovereign_account_id_of(
					t.args.dest.clone()
				),
			},
			// Transport fees are paid
			RuntimeEvent::PolkadotXcm(
				pallet_xcm::Event::FeesPaid { .. }
			) => {},
		]
	);
}

fn para_to_system_para_assets_sender_assertions(t: ParaToSystemParaTest) {
	type RuntimeEvent = <PenpalA as Chain>::RuntimeEvent;
	let system_para_native_asset_location = KsmLocation::get();
	let reservable_asset_location = PenpalLocalReservableFromAssetHub::get();
	PenpalA::assert_xcm_pallet_attempted_complete(Some(Weight::from_parts(864_610_000, 8799)));
	assert_expected_events!(
		PenpalA,
		vec![
			// Fees amount to reserve transfer is burned from Parachains's sender account
			RuntimeEvent::ForeignAssets(
				pallet_assets::Event::Burned { asset_id, owner, .. }
			) => {
				asset_id: *asset_id == system_para_native_asset_location,
				owner: *owner == t.sender.account_id,
			},
			// Amount to reserve transfer is burned from Parachains's sender account
			RuntimeEvent::ForeignAssets(
				pallet_assets::Event::Burned { asset_id, owner, balance }
			) => {
				asset_id: *asset_id == reservable_asset_location,
				owner: *owner == t.sender.account_id,
				balance: *balance == t.args.amount,
			},
			// Transport fees are paid
			RuntimeEvent::PolkadotXcm(
				pallet_xcm::Event::FeesPaid { .. }
			) => {},
		]
	);
}

fn system_para_to_para_assets_receiver_assertions(t: SystemParaToParaTest) {
	type RuntimeEvent = <PenpalA as Chain>::RuntimeEvent;

	let system_para_asset_location = PenpalLocalReservableFromAssetHub::get();
	PenpalA::assert_xcmp_queue_success(None);
	assert_expected_events!(
		PenpalA,
		vec![
			RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
				asset_id: *asset_id == KsmLocation::get(),
				owner: *owner == t.receiver.account_id,
			},
			RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, amount }) => {
				asset_id: *asset_id == system_para_asset_location,
				owner: *owner == t.receiver.account_id,
				amount: *amount == t.args.amount,
			},
		]
	);
}

fn para_to_system_para_assets_receiver_assertions(t: ParaToSystemParaTest) {
	type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
	let sov_penpal_on_ahr = AssetHubKusama::sovereign_account_id_of(
		AssetHubKusama::sibling_location_of(PenpalA::para_id()),
	);
	AssetHubKusama::assert_xcmp_queue_success(None);
	assert_expected_events!(
		AssetHubKusama,
		vec![
			// Amount to reserve transfer is burned from Parachain's Sovereign account
			RuntimeEvent::Assets(pallet_assets::Event::Burned { asset_id, owner, balance }) => {
				asset_id: *asset_id == RESERVABLE_ASSET_ID,
				owner: *owner == sov_penpal_on_ahr,
				balance: *balance == t.args.amount,
			},
			// Fee amount is burned from Parachain's Sovereign account
			RuntimeEvent::Balances(pallet_balances::Event::Burned { who, .. }) => {
				who: *who == sov_penpal_on_ahr,
			},
			// Amount to reserve transfer is issued for beneficiary
			RuntimeEvent::Assets(pallet_assets::Event::Issued { asset_id, owner, amount }) => {
				asset_id: *asset_id == RESERVABLE_ASSET_ID,
				owner: *owner == t.receiver.account_id,
				amount: *amount == t.args.amount,
			},
			// Remaining fee amount is minted for for beneficiary
			RuntimeEvent::Balances(pallet_balances::Event::Minted { who, .. }) => {
				who: *who == t.receiver.account_id,
			},
		]
	);
}

pub fn para_to_para_through_hop_sender_assertions<Hop: Clone>(t: Test<PenpalA, PenpalB, Hop>) {
	type RuntimeEvent = <PenpalA as Chain>::RuntimeEvent;

	PenpalA::assert_xcm_pallet_attempted_complete(None);
	for asset in t.args.assets.into_inner() {
		let expected_id = asset.id.0.clone();
		let amount = if let Fungible(a) = asset.fun { Some(a) } else { None }.unwrap();
		assert_expected_events!(
			PenpalA,
			vec![
				// Amount to reserve transfer is transferred to Parachain's Sovereign account
				RuntimeEvent::ForeignAssets(
					pallet_assets::Event::Burned { asset_id, owner, balance },
				) => {
					asset_id: *asset_id == expected_id,
					owner: *owner == t.sender.account_id,
					balance: *balance == amount,
				},
			]
		);
	}
}

fn para_to_para_relay_hop_assertions(t: ParaToParaThroughRelayTest) {
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

pub fn para_to_para_through_hop_receiver_assertions<Hop: Clone>(t: Test<PenpalA, PenpalB, Hop>) {
	type RuntimeEvent = <PenpalB as Chain>::RuntimeEvent;

	PenpalB::assert_xcmp_queue_success(None);
	for asset in t.args.assets.into_inner().into_iter() {
		let expected_id = asset.id.0;
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

fn para_to_relay_reserve_transfer_assets(t: ParaToRelayTest) -> DispatchResult {
	<PenpalA as PenpalAPallet>::PolkadotXcm::limited_reserve_transfer_assets(
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

fn para_to_para_through_relay_limited_reserve_transfer_assets(
	t: ParaToParaThroughRelayTest,
) -> DispatchResult {
	<PenpalA as PenpalAPallet>::PolkadotXcm::limited_reserve_transfer_assets(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.beneficiary.into()),
		bx!(t.args.assets.into()),
		t.args.fee_asset_item,
		t.args.weight_limit,
	)
}

/// Reserve Transfers of KSM from Relay Chain to the Asset Hub shouldn't work
#[test]
fn reserve_transfer_ksm_from_relay_to_asset_hub_fails() {
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

/// Reserve Transfers of KSM from Asset Hub to Relay Chain shouldn't work
#[test]
fn reserve_transfer_ksm_from_asset_hub_to_relay_fails() {
	// Init values for Asset Hub
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

/// Reserve Transfers of KSM from Relay to Parachain should work
#[test]
fn reserve_transfer_ksm_from_relay_to_para() {
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
	let mut test = RelayToParaTest::new(test_args);

	// Query initial balances
	let sender_balance_before = test.sender.balance;
	let receiver_assets_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location.clone(), &receiver)
	});

	// Set assertions and dispatchables
	test.set_assertion::<Kusama>(relay_to_para_sender_assertions);
	test.set_assertion::<PenpalA>(relay_to_para_assets_receiver_assertions);
	test.set_dispatchable::<Kusama>(relay_to_para_reserve_transfer_assets);
	test.assert();

	// Query final balances
	let sender_balance_after = test.sender.balance;
	let receiver_assets_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location, &receiver)
	});

	// Sender's balance is reduced by amount sent plus delivery fees
	assert!(sender_balance_after < sender_balance_before - amount_to_send);
	// Receiver's asset balance is increased
	assert!(receiver_assets_after > receiver_assets_before);
	// Receiver's asset balance increased by `amount_to_send - delivery_fees - bought_execution`;
	// `delivery_fees` might be paid from transfer or JIT, also `bought_execution` is unknown but
	// should be non-zero
	assert!(receiver_assets_after < receiver_assets_before + amount_to_send);
}

/// Reserve Transfers of KSM from Parachain to Relay should work
#[test]
fn reserve_transfer_ksm_from_para_to_relay() {
	// Init values for Parachain
	let destination = PenpalA::parent_location();
	let sender = PenpalASender::get();
	let amount_to_send: Balance = KUSAMA_ED * 1000;
	let assets: Assets = (Parent, amount_to_send).into();
	let asset_owner = PenpalAssetOwner::get();
	let relay_native_asset_location = KsmLocation::get();

	// fund Parachain's sender account
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(asset_owner),
		relay_native_asset_location.clone(),
		sender.clone(),
		amount_to_send * 2,
	);

	// Init values for Relay
	let receiver = KusamaReceiver::get();
	let penpal_location_as_seen_by_relay = Kusama::child_location_of(PenpalA::para_id());
	let sov_penpal_on_relay = Kusama::sovereign_account_id_of(penpal_location_as_seen_by_relay);

	// fund Parachain's SA on Relay with the native tokens held in reserve
	Kusama::fund_accounts(vec![(sov_penpal_on_relay, amount_to_send * 2)]);

	// Init Test
	let test_args = TestContext {
		sender: sender.clone(),
		receiver: receiver.clone(),
		args: TestArgs::new_para(
			destination.clone(),
			receiver,
			amount_to_send,
			assets.clone(),
			None,
			0,
		),
	};
	let mut test = ParaToRelayTest::new(test_args);

	// Query initial balances
	let sender_assets_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location.clone(), &sender)
	});
	let receiver_balance_before = test.receiver.balance;

	// Set assertions and dispatchables
	test.set_assertion::<PenpalA>(para_to_relay_sender_assertions);
	test.set_assertion::<Kusama>(para_to_relay_receiver_assertions);
	test.set_dispatchable::<PenpalA>(para_to_relay_reserve_transfer_assets);
	test.assert();

	// Query final balances
	let sender_assets_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location, &sender)
	});
	let receiver_balance_after = test.receiver.balance;

	// Sender's balance is reduced by amount sent plus delivery fees
	assert!(sender_assets_after < sender_assets_before - amount_to_send);
	// Receiver's asset balance is increased
	assert!(receiver_balance_after > receiver_balance_before);
	// Receiver's asset balance increased by `amount_to_send - delivery_fees - bought_execution`;
	// `delivery_fees` might be paid from transfer or JIT, also `bought_execution` is unknown but
	// should be non-zero
	assert!(receiver_balance_after < receiver_balance_before + amount_to_send);
}

/// Reserve Transfers of KSM from Asset Hub to Parachain should work
#[test]
fn reserve_transfer_ksm_from_asset_hub_to_para() {
	// Init values for Asset Hub
	let destination = AssetHubKusama::sibling_location_of(PenpalA::para_id());
	let sender = AssetHubKusamaSender::get();
	let amount_to_send: Balance = ASSET_HUB_KUSAMA_ED * 10000;
	let assets: Assets = (Parent, amount_to_send).into();

	// Init values for Parachain
	let system_para_native_asset_location = KsmLocation::get();
	let receiver = PenpalAReceiver::get();

	// Init Test
	let test_args = TestContext {
		sender,
		receiver: receiver.clone(),
		args: TestArgs::new_para(
			destination.clone(),
			receiver.clone(),
			amount_to_send,
			assets.clone(),
			None,
			0,
		),
	};
	let mut test = SystemParaToParaTest::new(test_args);

	// Query initial balances
	let sender_balance_before = test.sender.balance;
	let receiver_assets_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(system_para_native_asset_location.clone(), &receiver)
	});

	// Set assertions and dispatchables
	test.set_assertion::<AssetHubKusama>(system_para_to_para_sender_assertions);
	test.set_assertion::<PenpalA>(system_para_to_para_receiver_assertions);
	test.set_dispatchable::<AssetHubKusama>(system_para_to_para_reserve_transfer_assets);
	test.assert();

	// Query final balances
	let sender_balance_after = test.sender.balance;
	let receiver_assets_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(system_para_native_asset_location, &receiver)
	});

	// Sender's balance is reduced by amount sent plus delivery fees
	assert!(sender_balance_after < sender_balance_before - amount_to_send);
	// Receiver's assets is increased
	assert!(receiver_assets_after > receiver_assets_before);
	// Receiver's assets increased by `amount_to_send - delivery_fees - bought_execution`;
	// `delivery_fees` might be paid from transfer or JIT, also `bought_execution` is unknown but
	// should be non-zero
	assert!(receiver_assets_after < receiver_assets_before + amount_to_send);
}

/// Reserve Transfers of KSM from Parachain to Asset Hub should work
#[test]
fn reserve_transfer_ksm_from_para_to_asset_hub() {
	// Init values for Parachain
	let destination = PenpalA::sibling_location_of(AssetHubKusama::para_id());
	let sender = PenpalASender::get();
	let amount_to_send: Balance = ASSET_HUB_KUSAMA_ED * 10000;
	let assets: Assets = (Parent, amount_to_send).into();
	let system_para_native_asset_location = KsmLocation::get();
	let asset_owner = PenpalAssetOwner::get();

	// fund Parachain's sender account
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(asset_owner),
		system_para_native_asset_location.clone(),
		sender.clone(),
		amount_to_send * 2,
	);

	// Init values for Asset Hub
	let receiver = AssetHubKusamaReceiver::get();
	let penpal_location_as_seen_by_ahr = AssetHubKusama::sibling_location_of(PenpalA::para_id());
	let sov_penpal_on_ahr = AssetHubKusama::sovereign_account_id_of(penpal_location_as_seen_by_ahr);

	// fund Parachain's SA on Asset Hub with the native tokens held in reserve
	AssetHubKusama::fund_accounts(vec![(sov_penpal_on_ahr, amount_to_send * 2)]);

	// Init Test
	let test_args = TestContext {
		sender: sender.clone(),
		receiver: receiver.clone(),
		args: TestArgs::new_para(
			destination.clone(),
			receiver.clone(),
			amount_to_send,
			assets.clone(),
			None,
			0,
		),
	};
	let mut test = ParaToSystemParaTest::new(test_args);

	// Query initial balances
	let sender_assets_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(system_para_native_asset_location.clone(), &sender)
	});
	let receiver_balance_before = test.receiver.balance;

	// Set assertions and dispatchables
	test.set_assertion::<PenpalA>(para_to_system_para_sender_assertions);
	test.set_assertion::<AssetHubKusama>(para_to_system_para_receiver_assertions);
	test.set_dispatchable::<PenpalA>(para_to_system_para_reserve_transfer_assets);
	test.assert();

	// Query final balances
	let sender_assets_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(system_para_native_asset_location, &sender)
	});
	let receiver_balance_after = test.receiver.balance;

	// Sender's balance is reduced by amount sent plus delivery fees
	assert!(sender_assets_after < sender_assets_before - amount_to_send);
	// Receiver's balance is increased
	assert!(receiver_balance_after > receiver_balance_before);
	// Receiver's balance increased by `amount_to_send - delivery_fees - bought_execution`;
	// `delivery_fees` might be paid from transfer or JIT, also `bought_execution` is unknown but
	// should be non-zero
	assert!(receiver_balance_after < receiver_balance_before + amount_to_send);
}

/// Reserve Transfers of a local asset and KSM from Asset Hub to Parachain should work
#[test]
fn reserve_transfer_multiple_assets_from_asset_hub_to_para() {
	// Init values for Asset Hub
	let destination = AssetHubKusama::sibling_location_of(PenpalA::para_id());
	let sov_penpal_on_ahr = AssetHubKusama::sovereign_account_id_of(destination.clone());
	let sender = AssetHubKusamaSender::get();
	let fee_amount_to_send = ASSET_HUB_KUSAMA_ED * 10000;
	let asset_amount_to_send = PENPAL_ED * 10000;
	let asset_owner = AssetHubKusamaAssetOwner::get();
	let asset_owner_signer = <AssetHubKusama as Chain>::RuntimeOrigin::signed(asset_owner.clone());
	let assets: Assets = vec![
		(Parent, fee_amount_to_send).into(),
		(
			[PalletInstance(ASSETS_PALLET_ID), GeneralIndex(RESERVABLE_ASSET_ID.into())],
			asset_amount_to_send,
		)
			.into(),
	]
	.into();
	let fee_asset_index = assets
		.inner()
		.iter()
		.position(|r| r == &(Parent, fee_amount_to_send).into())
		.unwrap() as u32;
	AssetHubKusama::mint_asset(
		asset_owner_signer,
		RESERVABLE_ASSET_ID,
		asset_owner,
		asset_amount_to_send * 2,
	);

	// Create SA-of-Penpal-on-AHR with ED.
	AssetHubKusama::fund_accounts(vec![(sov_penpal_on_ahr, ASSET_HUB_KUSAMA_ED)]);

	// Init values for Parachain
	let receiver = PenpalAReceiver::get();
	let system_para_native_asset_location = KsmLocation::get();
	let system_para_foreign_asset_location = PenpalLocalReservableFromAssetHub::get();

	// Init Test
	let para_test_args = TestContext {
		sender: sender.clone(),
		receiver: receiver.clone(),
		args: TestArgs::new_para(
			destination,
			receiver.clone(),
			asset_amount_to_send,
			assets,
			None,
			fee_asset_index,
		),
	};
	let mut test = SystemParaToParaTest::new(para_test_args);

	// Query initial balances
	let sender_balance_before = test.sender.balance;
	let sender_assets_before = AssetHubKusama::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaPallet>::Assets;
		<Assets as Inspect<_>>::balance(RESERVABLE_ASSET_ID, &sender)
	});
	let receiver_system_native_assets_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(system_para_native_asset_location.clone(), &receiver)
	});
	let receiver_foreign_assets_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(
			system_para_foreign_asset_location.clone(),
			&receiver,
		)
	});

	// Set assertions and dispatchables
	test.set_assertion::<AssetHubKusama>(system_para_to_para_assets_sender_assertions);
	test.set_assertion::<PenpalA>(system_para_to_para_assets_receiver_assertions);
	test.set_dispatchable::<AssetHubKusama>(system_para_to_para_reserve_transfer_assets);
	test.assert();

	// Query final balances
	let sender_balance_after = test.sender.balance;
	let sender_assets_after = AssetHubKusama::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaPallet>::Assets;
		<Assets as Inspect<_>>::balance(RESERVABLE_ASSET_ID, &sender)
	});
	let receiver_system_native_assets_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(system_para_native_asset_location.clone(), &receiver)
	});
	let receiver_foreign_assets_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(system_para_foreign_asset_location, &receiver)
	});
	// Sender's balance is reduced
	assert!(sender_balance_after < sender_balance_before);
	// Receiver's foreign asset balance is increased
	assert!(receiver_foreign_assets_after > receiver_foreign_assets_before);
	// Receiver's system asset balance increased by `amount_to_send - delivery_fees -
	// bought_execution`; `delivery_fees` might be paid from transfer or JIT, also
	// `bought_execution` is unknown but should be non-zero
	assert!(
		receiver_system_native_assets_after <
			receiver_system_native_assets_before + fee_amount_to_send
	);

	// Sender's asset balance is reduced by exact amount
	assert_eq!(sender_assets_before - asset_amount_to_send, sender_assets_after);
	// Receiver's foreign asset balance is increased by exact amount
	assert_eq!(
		receiver_foreign_assets_after,
		receiver_foreign_assets_before + asset_amount_to_send
	);
}

/// Reserve Transfers of a random asset and KSM from Parachain to Asset Hub should work
#[test]
fn reserve_transfer_multiple_assets_from_para_to_asset_hub() {
	// Init values for Parachain
	let destination = PenpalA::sibling_location_of(AssetHubKusama::para_id());
	let sender = PenpalASender::get();
	let fee_amount_to_send = ASSET_HUB_KUSAMA_ED * 10000;
	let asset_amount_to_send = ASSET_HUB_KUSAMA_ED * 10000;
	let penpal_asset_owner = PenpalAssetOwner::get();
	let penpal_asset_owner_signer = <PenpalA as Chain>::RuntimeOrigin::signed(penpal_asset_owner);
	let asset_location_on_penpal = PenpalLocalReservableFromAssetHub::get();
	let system_asset_location_on_penpal = KsmLocation::get();
	let assets: Assets = vec![
		(Parent, fee_amount_to_send).into(),
		(asset_location_on_penpal.clone(), asset_amount_to_send).into(),
	]
	.into();
	let fee_asset_index = assets
		.inner()
		.iter()
		.position(|r| r == &(Parent, fee_amount_to_send).into())
		.unwrap() as u32;
	// Fund Parachain's sender account with some foreign assets
	PenpalA::mint_foreign_asset(
		penpal_asset_owner_signer.clone(),
		asset_location_on_penpal.clone(),
		sender.clone(),
		asset_amount_to_send * 2,
	);
	// Fund Parachain's sender account with some system assets
	PenpalA::mint_foreign_asset(
		penpal_asset_owner_signer,
		system_asset_location_on_penpal.clone(),
		sender.clone(),
		fee_amount_to_send * 2,
	);

	// Init values for Asset Hub
	let receiver = AssetHubKusamaReceiver::get();
	let penpal_location_as_seen_by_ahr = AssetHubKusama::sibling_location_of(PenpalA::para_id());
	let sov_penpal_on_ahr = AssetHubKusama::sovereign_account_id_of(penpal_location_as_seen_by_ahr);
	let ah_asset_owner = AssetHubKusamaAssetOwner::get();
	let ah_asset_owner_signer = <AssetHubKusama as Chain>::RuntimeOrigin::signed(ah_asset_owner);

	// Fund SA-of-Penpal-on-AHR to be able to pay for the fees.
	AssetHubKusama::fund_accounts(vec![(
		sov_penpal_on_ahr.clone(),
		ASSET_HUB_KUSAMA_ED * 10000000,
	)]);
	// Fund SA-of-Penpal-on-AHR to be able to pay for the sent amount.
	AssetHubKusama::mint_asset(
		ah_asset_owner_signer,
		RESERVABLE_ASSET_ID,
		sov_penpal_on_ahr,
		asset_amount_to_send * 2,
	);

	// Init Test
	let para_test_args = TestContext {
		sender: sender.clone(),
		receiver: receiver.clone(),
		args: TestArgs::new_para(
			destination,
			receiver.clone(),
			asset_amount_to_send,
			assets,
			None,
			fee_asset_index,
		),
	};
	let mut test = ParaToSystemParaTest::new(para_test_args);

	// Query initial balances
	let sender_system_assets_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(system_asset_location_on_penpal.clone(), &sender)
	});
	let sender_foreign_assets_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(asset_location_on_penpal.clone(), &sender)
	});
	let receiver_balance_before = test.receiver.balance;
	let receiver_assets_before = AssetHubKusama::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaPallet>::Assets;
		<Assets as Inspect<_>>::balance(RESERVABLE_ASSET_ID, &receiver)
	});

	// Set assertions and dispatchables
	test.set_assertion::<PenpalA>(para_to_system_para_assets_sender_assertions);
	test.set_assertion::<AssetHubKusama>(para_to_system_para_assets_receiver_assertions);
	test.set_dispatchable::<PenpalA>(para_to_system_para_reserve_transfer_assets);
	test.assert();

	// Query final balances
	let sender_system_assets_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(system_asset_location_on_penpal, &sender)
	});
	let sender_foreign_assets_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(asset_location_on_penpal, &sender)
	});
	let receiver_balance_after = test.receiver.balance;
	let receiver_assets_after = AssetHubKusama::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaPallet>::Assets;
		<Assets as Inspect<_>>::balance(RESERVABLE_ASSET_ID, &receiver)
	});
	// Sender's system asset balance is reduced
	assert!(sender_system_assets_after < sender_system_assets_before);
	// Receiver's balance is increased
	assert!(receiver_balance_after > receiver_balance_before);
	// Receiver's balance increased by `amount_to_send - delivery_fees - bought_execution`;
	// `delivery_fees` might be paid from transfer or JIT, also `bought_execution` is unknown but
	// should be non-zero
	assert!(receiver_balance_after < receiver_balance_before + fee_amount_to_send);

	// Sender's asset balance is reduced by exact amount
	assert_eq!(sender_foreign_assets_before - asset_amount_to_send, sender_foreign_assets_after);
	// Receiver's foreign asset balance is increased by exact amount
	assert_eq!(receiver_assets_after, receiver_assets_before + asset_amount_to_send);
}

/// Reserve Transfers of KSM from Parachain to Parachain (through Relay reserve) should work
#[test]
fn reserve_transfer_ksm_from_para_to_para_through_relay() {
	// Init values for Parachain Origin
	let destination = PenpalA::sibling_location_of(PenpalB::para_id());
	let sender = PenpalASender::get();
	let amount_to_send: Balance = KUSAMA_ED * 10000;
	let asset_owner = PenpalAssetOwner::get();
	let assets = (Parent, amount_to_send).into();
	let relay_native_asset_location = KsmLocation::get();
	let sender_as_seen_by_relay = Kusama::child_location_of(PenpalA::para_id());
	let sov_of_sender_on_relay = Kusama::sovereign_account_id_of(sender_as_seen_by_relay);

	// fund Parachain's sender account
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(asset_owner),
		relay_native_asset_location.clone(),
		sender.clone(),
		amount_to_send * 2,
	);

	// fund the Parachain Origin's SA on Relay Chain with the native tokens held in reserve
	Kusama::fund_accounts(vec![(sov_of_sender_on_relay, amount_to_send * 2)]);

	// Init values for Parachain Destination
	let receiver = PenpalBReceiver::get();

	// Init Test
	let test_args = TestContext {
		sender: sender.clone(),
		receiver: receiver.clone(),
		args: TestArgs::new_para(destination, receiver.clone(), amount_to_send, assets, None, 0),
	};
	let mut test = ParaToParaThroughRelayTest::new(test_args);

	// Query initial balances
	let sender_assets_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location.clone(), &sender)
	});
	let receiver_assets_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location.clone(), &receiver)
	});

	// Set assertions and dispatchables
	test.set_assertion::<PenpalA>(para_to_para_through_hop_sender_assertions);
	test.set_assertion::<Kusama>(para_to_para_relay_hop_assertions);
	test.set_assertion::<PenpalB>(para_to_para_through_hop_receiver_assertions);
	test.set_dispatchable::<PenpalA>(para_to_para_through_relay_limited_reserve_transfer_assets);
	test.assert();

	// Query final balances
	let sender_assets_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location.clone(), &sender)
	});
	let receiver_assets_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location, &receiver)
	});

	// Sender's balance is reduced by amount sent plus delivery fees
	assert!(sender_assets_after < sender_assets_before - amount_to_send);
	// Receiver's balance is increased
	assert!(receiver_assets_after > receiver_assets_before);
}

/// Reserve Withdraw Native Asset from AssetHub to Parachain fails.
#[test]
fn reserve_withdraw_from_untrusted_reserve_fails() {
	// Init values for Parachain Origin
	let destination = AssetHubKusama::sibling_location_of(PenpalA::para_id());
	let signed_origin =
		<AssetHubKusama as Chain>::RuntimeOrigin::signed(AssetHubKusamaSender::get());
	let ksm_to_send: Balance = KUSAMA_ED * 10000;
	let ksm_location = KsmLocation::get();

	// Assets to send
	let assets: Vec<Asset> = vec![(ksm_location.clone(), ksm_to_send).into()];
	let fee_id: AssetId = ksm_location.into();

	// this should fail
	AssetHubKusama::execute_with(|| {
		let result = <AssetHubKusama as AssetHubKusamaPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
			signed_origin.clone(),
			bx!(destination.clone().into()),
			bx!(assets.clone().into()),
			bx!(TransferType::DestinationReserve),
			bx!(fee_id.into()),
			bx!(TransferType::DestinationReserve),
			bx!(VersionedXcm::from(Xcm::<()>::new())),
			Unlimited,
		);
		assert_err!(
			result,
			DispatchError::Module(sp_runtime::ModuleError {
				index: 31,
				error: [22, 0, 0, 0],
				message: Some("InvalidAssetUnsupportedReserve")
			})
		);
	});

	// this should also fail
	AssetHubKusama::execute_with(|| {
		let xcm: Xcm<asset_hub_kusama_runtime::RuntimeCall> = Xcm(vec![
			WithdrawAsset(assets.into()),
			InitiateReserveWithdraw {
				assets: Wild(All),
				reserve: destination,
				xcm: Xcm::<()>::new(),
			},
		]);
		let result = <AssetHubKusama as AssetHubKusamaPallet>::PolkadotXcm::execute(
			signed_origin,
			bx!(xcm::VersionedXcm::V4(xcm)),
			Weight::MAX,
		);
		assert!(result.is_err());
	});
}
