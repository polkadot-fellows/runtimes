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
use asset_hub_polkadot_runtime::xcm_config::DotLocation;
use emulated_integration_tests_common::RESERVABLE_ASSET_ID;
use polkadot_system_emulated_network::penpal_emulated_chain::LocalReservableFromAssetHub as PenpalLocalReservableFromAssetHub;

fn relay_to_para_sender_assertions(t: RelayToParaTest) {
	type RuntimeEvent = <Polkadot as Chain>::RuntimeEvent;
	Polkadot::assert_xcm_pallet_attempted_complete(Some(Weight::from_parts(864_610_000, 8_799)));

	assert_expected_events!(
		Polkadot,
		vec![
			// Amount to reserve transfer is transferred to Parachain's Sovereign account
			RuntimeEvent::Balances(
				pallet_balances::Event::Transfer { from, to, amount }
			) => {
				from: *from == t.sender.account_id,
				to: *to == Polkadot::sovereign_account_id_of(
					t.args.dest.clone()
				),
				amount: *amount == t.args.amount,
			},
		]
	);
}

fn system_para_to_para_sender_assertions(t: SystemParaToParaTest) {
	type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
	AssetHubPolkadot::assert_xcm_pallet_attempted_complete(Some(Weight::from_parts(
		676_119_000,
		6196,
	)));

	assert_expected_events!(
		AssetHubPolkadot,
		vec![
			// Amount to reserve transfer is transferred to Parachain's Sovereign account
			RuntimeEvent::Balances(
				pallet_balances::Event::Transfer { from, to, amount }
			) => {
				from: *from == t.sender.account_id,
				to: *to == AssetHubPolkadot::sovereign_account_id_of(
					t.args.dest.clone()
				),
				amount: *amount == t.args.amount,
			},
		]
	);
}

pub fn system_para_to_para_receiver_assertions(t: SystemParaToParaTest) {
	type RuntimeEvent = <PenpalB as Chain>::RuntimeEvent;

	PenpalB::assert_xcmp_queue_success(None);
	for asset in t.args.assets.into_inner().into_iter() {
		let expected_id = asset.id.0.try_into().unwrap();
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
	type RuntimeEvent = <PenpalB as Chain>::RuntimeEvent;

	assert_expected_events!(
		PenpalB,
		vec![
			RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
				asset_id: *asset_id == DotLocation::get(),
				owner: *owner == t.receiver.account_id,
			},
			RuntimeEvent::MessageQueue(
				pallet_message_queue::Event::Processed { success: true, .. }
			) => {},
		]
	);
}

pub fn para_to_system_para_sender_assertions(t: ParaToSystemParaTest) {
	type RuntimeEvent = <PenpalB as Chain>::RuntimeEvent;

	PenpalB::assert_xcm_pallet_attempted_complete(None);
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

fn para_to_system_para_receiver_assertions(t: ParaToSystemParaTest) {
	type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
	let sov_penpal_on_ahp = AssetHubPolkadot::sovereign_account_id_of(
		AssetHubPolkadot::sibling_location_of(PenpalB::para_id()),
	);

	assert_expected_events!(
		AssetHubPolkadot,
		vec![
			// Amount to reserve transfer is transferred to Parachain's Sovereign account
			RuntimeEvent::Balances(
				pallet_balances::Event::Burned { who, amount }
			) => {
				who: *who == sov_penpal_on_ahp.clone(),
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
	type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
	AssetHubPolkadot::assert_xcm_pallet_attempted_complete(Some(Weight::from_parts(
		676_119_000,
		6196,
	)));

	assert_expected_events!(
		AssetHubPolkadot,
		vec![
			// Amount to reserve transfer is transferred to Parachain's Sovereign account
			RuntimeEvent::Assets(
				pallet_assets::Event::Transferred { asset_id, from, to, amount }
			) => {
				asset_id: *asset_id == ASSET_ID,
				from: *from == t.sender.account_id,
				to: *to == AssetHubPolkadot::sovereign_account_id_of(
					t.args.dest.clone()
				),
				amount: *amount == t.args.amount,
			},
		]
	);
}

fn system_para_to_para_assets_receiver_assertions(t: SystemParaToParaTest) {
	type RuntimeEvent = <PenpalB as Chain>::RuntimeEvent;

	let system_para_asset_location = PenpalLocalReservableFromAssetHub::get();
	PenpalB::assert_xcmp_queue_success(None);
	assert_expected_events!(
		PenpalB,
		vec![
			RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
				asset_id: *asset_id == DotLocation::get(),
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

fn para_to_para_through_hop_sender_assertions(t: ParaToParaThroughRelayTest) {
	type RuntimeEvent = <PenpalB as Chain>::RuntimeEvent;

	PenpalB::assert_xcm_pallet_attempted_complete(None);
	for asset in t.args.assets.into_inner() {
		let expected_id = asset.id.0.clone().try_into().unwrap();
		let amount = if let Fungible(a) = asset.fun { Some(a) } else { None }.unwrap();
		assert_expected_events!(
			PenpalB,
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
	type RuntimeEvent = <Polkadot as Chain>::RuntimeEvent;
	let sov_penpal_a_on_polkadot =
		Polkadot::sovereign_account_id_of(Polkadot::child_location_of(PenpalA::para_id()));
	let sov_penpal_b_on_polkadot =
		Polkadot::sovereign_account_id_of(Polkadot::child_location_of(PenpalB::para_id()));
	assert_expected_events!(
		Polkadot,
		vec![
			// Withdrawn from sender parachain SA
			RuntimeEvent::Balances(
				pallet_balances::Event::Withdraw { who, amount } | pallet_balances::Event::Burned { who, amount }
			) => {
				who: *who == sov_penpal_b_on_polkadot,
				amount: *amount == t.args.amount,
			},
			// Deposited to receiver parachain SA
			RuntimeEvent::Balances(
				pallet_balances::Event::Deposit { who, .. } | pallet_balances::Event::Minted { who, .. }
			) => {
				who: *who == sov_penpal_a_on_polkadot,
			},
			RuntimeEvent::MessageQueue(
				pallet_message_queue::Event::Processed { success: true, .. }
			) => {},
		]
	);
}

pub fn para_to_para_through_hop_receiver_assertions<Hop: Clone>(t: Test<PenpalB, PenpalA, Hop>) {
	type RuntimeEvent = <PenpalA as Chain>::RuntimeEvent;

	PenpalA::assert_xcmp_queue_success(None);
	for asset in t.args.assets.into_inner().into_iter() {
		let expected_id = asset.id.0.try_into().unwrap();
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
}

fn relay_to_para_reserve_transfer_assets(t: RelayToParaTest) -> DispatchResult {
	<Polkadot as PolkadotPallet>::XcmPallet::limited_reserve_transfer_assets(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.beneficiary.into()),
		bx!(t.args.assets.into()),
		t.args.fee_asset_item,
		t.args.weight_limit,
	)
}

fn system_para_to_para_reserve_transfer_assets(t: SystemParaToParaTest) -> DispatchResult {
	<AssetHubPolkadot as AssetHubPolkadotPallet>::PolkadotXcm::limited_reserve_transfer_assets(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.beneficiary.into()),
		bx!(t.args.assets.into()),
		t.args.fee_asset_item,
		t.args.weight_limit,
	)
}

fn para_to_system_para_reserve_transfer_assets(t: ParaToSystemParaTest) -> DispatchResult {
	<PenpalB as PenpalBPallet>::PolkadotXcm::limited_reserve_transfer_assets(
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
	<PenpalB as PenpalBPallet>::PolkadotXcm::limited_reserve_transfer_assets(
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
	let signed_origin = <Polkadot as Chain>::RuntimeOrigin::signed(PolkadotSender::get());
	let destination = Polkadot::child_location_of(AssetHubPolkadot::para_id());
	let beneficiary: Location =
		AccountId32Junction { network: None, id: AssetHubPolkadotReceiver::get().into() }.into();
	let amount_to_send: Balance = POLKADOT_ED * 1000;
	let assets: Assets = (Here, amount_to_send).into();
	let fee_asset_item = 0;

	// this should fail
	Polkadot::execute_with(|| {
		let result = <Polkadot as PolkadotPallet>::XcmPallet::limited_reserve_transfer_assets(
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
		<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotSender::get());
	let destination = AssetHubPolkadot::parent_location();
	let beneficiary_id = PolkadotReceiver::get();
	let beneficiary: Location =
		AccountId32Junction { network: None, id: beneficiary_id.into() }.into();
	let amount_to_send: Balance = ASSET_HUB_POLKADOT_ED * 1000;

	let assets: Assets = (Parent, amount_to_send).into();
	let fee_asset_item = 0;

	// this should fail
	AssetHubPolkadot::execute_with(|| {
		let result =
			<AssetHubPolkadot as AssetHubPolkadotPallet>::PolkadotXcm::limited_reserve_transfer_assets(
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
	let mut test = RelayToParaTest::new(test_args);

	// Query initial balances
	let sender_balance_before = test.sender.balance;
	let receiver_assets_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location.clone(), &receiver)
	});

	// Set assertions and dispatchables
	test.set_assertion::<Polkadot>(relay_to_para_sender_assertions);
	test.set_assertion::<PenpalB>(relay_to_para_assets_receiver_assertions);
	test.set_dispatchable::<Polkadot>(relay_to_para_reserve_transfer_assets);
	test.assert();

	// Query final balances
	let sender_balance_after = test.sender.balance;
	let receiver_assets_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
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

/// Reserve Transfers of native asset from System Parachain to Parachain should work
#[test]
fn reserve_transfer_native_asset_from_system_para_to_para() {
	// Init values for System Parachain
	let destination = AssetHubPolkadot::sibling_location_of(PenpalB::para_id());
	let sender = AssetHubPolkadotSender::get();
	let amount_to_send: Balance = ASSET_HUB_POLKADOT_ED * 10000;
	let assets: Assets = (Parent, amount_to_send).into();

	// Init values for Parachain
	let system_para_native_asset_location = DotLocation::get();
	let receiver = PenpalBReceiver::get();

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
	let receiver_assets_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(system_para_native_asset_location.clone(), &receiver)
	});

	// Set assertions and dispatchables
	test.set_assertion::<AssetHubPolkadot>(system_para_to_para_sender_assertions);
	test.set_assertion::<PenpalB>(system_para_to_para_receiver_assertions);
	test.set_dispatchable::<AssetHubPolkadot>(system_para_to_para_reserve_transfer_assets);
	test.assert();

	// Query final balances
	let sender_balance_after = test.sender.balance;
	let receiver_assets_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
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

/// Reserve Transfers of native asset from Parachain to System Parachain should work
#[test]
fn reserve_transfer_native_asset_from_para_to_system_para() {
	// Init values for Parachain
	let destination = PenpalB::sibling_location_of(AssetHubPolkadot::para_id());
	let sender = PenpalBSender::get();
	let amount_to_send: Balance = ASSET_HUB_POLKADOT_ED * 10000;
	let assets: Assets = (Parent, amount_to_send).into();
	let system_para_native_asset_location = DotLocation::get();
	let asset_owner = PenpalAssetOwner::get();

	// fund Parachain's sender account
	PenpalB::mint_foreign_asset(
		<PenpalB as Chain>::RuntimeOrigin::signed(asset_owner),
		system_para_native_asset_location.clone(),
		sender.clone(),
		amount_to_send * 2,
	);

	// Init values for System Parachain
	let receiver = AssetHubPolkadotReceiver::get();
	let penpal_location_as_seen_by_ahr = AssetHubPolkadot::sibling_location_of(PenpalB::para_id());
	let sov_penpal_on_ahr =
		AssetHubPolkadot::sovereign_account_id_of(penpal_location_as_seen_by_ahr);

	// fund Parachain's SA on System Parachain with the native tokens held in reserve
	AssetHubPolkadot::fund_accounts(vec![(sov_penpal_on_ahr.into(), amount_to_send * 2)]);

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
	let sender_assets_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(system_para_native_asset_location.clone(), &sender)
	});
	let receiver_balance_before = test.receiver.balance;

	// Set assertions and dispatchables
	test.set_assertion::<PenpalB>(para_to_system_para_sender_assertions);
	test.set_assertion::<AssetHubPolkadot>(para_to_system_para_receiver_assertions);
	test.set_dispatchable::<PenpalB>(para_to_system_para_reserve_transfer_assets);
	test.assert();

	// Query final balances
	let sender_assets_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
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

/// Reserve Transfers of a local asset and native asset from System Parachain to Parachain should
/// work
#[test]
fn reserve_transfer_assets_from_system_para_to_para() {
	// Init values for System Parachain
	let destination = AssetHubPolkadot::sibling_location_of(PenpalB::para_id());
	let sov_penpal_on_ahr = AssetHubPolkadot::sovereign_account_id_of(destination.clone());
	let sender = AssetHubPolkadotSender::get();
	let fee_amount_to_send = ASSET_HUB_POLKADOT_ED * 10000;
	let asset_amount_to_send = PENPAL_ED * 10000;
	let asset_owner = AssetHubPolkadotAssetOwner::get();
	let asset_owner_signer =
		<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(asset_owner.clone());
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
	AssetHubPolkadot::mint_asset(
		asset_owner_signer,
		RESERVABLE_ASSET_ID,
		asset_owner,
		asset_amount_to_send * 2,
	);

	// Create SA-of-Penpal-on-AHR with ED.
	AssetHubPolkadot::fund_accounts(vec![(sov_penpal_on_ahr.into(), ASSET_HUB_POLKADOT_ED)]);

	// Init values for Parachain
	let receiver = PenpalBReceiver::get();
	let system_para_native_asset_location = DotLocation::get();
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
	let sender_assets_before = AssetHubPolkadot::execute_with(|| {
		type Assets = <AssetHubPolkadot as AssetHubPolkadotPallet>::Assets;
		<Assets as Inspect<_>>::balance(RESERVABLE_ASSET_ID, &sender)
	});
	let receiver_system_native_assets_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(system_para_native_asset_location.clone(), &receiver)
	});
	let receiver_foreign_assets_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(
			system_para_foreign_asset_location.clone(),
			&receiver,
		)
	});

	// Set assertions and dispatchables
	test.set_assertion::<AssetHubPolkadot>(system_para_to_para_assets_sender_assertions);
	test.set_assertion::<PenpalB>(system_para_to_para_assets_receiver_assertions);
	test.set_dispatchable::<AssetHubPolkadot>(system_para_to_para_reserve_transfer_assets);
	test.assert();

	// Query final balances
	let sender_balance_after = test.sender.balance;
	let sender_assets_after = AssetHubPolkadot::execute_with(|| {
		type Assets = <AssetHubPolkadot as AssetHubPolkadotPallet>::Assets;
		<Assets as Inspect<_>>::balance(RESERVABLE_ASSET_ID, &sender)
	});
	let receiver_system_native_assets_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(system_para_native_asset_location.clone(), &receiver)
	});
	let receiver_foreign_assets_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
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

/// Reserve Transfers of native asset from Parachain to Parachain (through Relay reserve) should
/// work
#[test]
fn reserve_transfer_native_asset_from_para_to_para_through_relay() {
	// Init values for Parachain Origin
	let destination = PenpalB::sibling_location_of(PenpalA::para_id());
	let sender = PenpalBSender::get();
	let amount_to_send: Balance = POLKADOT_ED * 10000;
	let asset_owner = PenpalAssetOwner::get();
	let assets = (Parent, amount_to_send).into();
	let relay_native_asset_location = DotLocation::get();
	let sender_as_seen_by_relay = Polkadot::child_location_of(PenpalB::para_id());
	let sov_of_sender_on_relay = Polkadot::sovereign_account_id_of(sender_as_seen_by_relay);

	// fund Parachain's sender account
	PenpalB::mint_foreign_asset(
		<PenpalB as Chain>::RuntimeOrigin::signed(asset_owner),
		relay_native_asset_location.clone(),
		sender.clone(),
		amount_to_send * 2,
	);

	// fund the Parachain Origin's SA on Relay Chain with the native tokens held in reserve
	Polkadot::fund_accounts(vec![(sov_of_sender_on_relay.into(), amount_to_send * 2)]);

	// Init values for Parachain Destination
	let receiver = PenpalAReceiver::get();

	// Init Test
	let test_args = TestContext {
		sender: sender.clone(),
		receiver: receiver.clone(),
		args: TestArgs::new_para(destination, receiver.clone(), amount_to_send, assets, None, 0),
	};
	let mut test = ParaToParaThroughRelayTest::new(test_args);

	// Query initial balances
	let sender_assets_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location.clone(), &sender)
	});
	let receiver_assets_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location.clone(), &receiver)
	});

	// Set assertions and dispatchables
	test.set_assertion::<PenpalB>(para_to_para_through_hop_sender_assertions);
	test.set_assertion::<Polkadot>(para_to_para_relay_hop_assertions);
	test.set_assertion::<PenpalA>(para_to_para_through_hop_receiver_assertions);
	test.set_dispatchable::<PenpalB>(para_to_para_through_relay_limited_reserve_transfer_assets);
	test.assert();

	// Query final balances
	let sender_assets_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location.clone(), &sender)
	});
	let receiver_assets_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location, &receiver)
	});

	// Sender's balance is reduced by amount sent plus delivery fees
	assert!(sender_assets_after < sender_assets_before - amount_to_send);
	// Receiver's balance is increased
	assert!(receiver_assets_after > receiver_assets_before);
}
