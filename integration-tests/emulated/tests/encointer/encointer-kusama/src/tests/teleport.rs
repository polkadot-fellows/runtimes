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
use asset_hub_kusama_runtime::xcm_config::{KsmLocation, XcmConfig as AssetHubKusamaXcmConfig};
use emulated_integration_tests_common::xcm_helpers::{fee_asset, non_fee_asset};
use frame_support::{
	dispatch::{GetDispatchInfo, RawOrigin},
	traits::fungible::Mutate,
};
use kusama_system_emulated_network::penpal_emulated_chain::LocalTeleportableToAssetHub as PenpalLocalTeleportableToAssetHub;
use xcm_runtime_apis::{
	dry_run::runtime_decl_for_dry_run_api::DryRunApiV2,
	fees::runtime_decl_for_xcm_payment_api::XcmPaymentApiV1,
};

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

fn system_para_limited_teleport_assets(t: SystemParaToRelayTest) -> DispatchResult {
	<EncointerKusama as EncointerKusamaPallet>::PolkadotXcm::limited_teleport_assets(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.beneficiary.into()),
		bx!(t.args.assets.into()),
		t.args.fee_asset_item,
		t.args.weight_limit,
	)
}

fn para_to_system_para_transfer_assets(t: ParaToSystemParaTest) -> DispatchResult {
	<EncointerKusama as EncointerKusamaPallet>::PolkadotXcm::transfer_assets(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.beneficiary.into()),
		bx!(t.args.assets.into()),
		t.args.fee_asset_item,
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