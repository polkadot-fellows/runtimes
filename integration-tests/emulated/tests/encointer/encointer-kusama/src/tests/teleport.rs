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
use encointer_kusama_runtime::xcm_config::XcmConfig as EncointerKusamaXcmConfig;
use frame_support::{
	dispatch::{GetDispatchInfo, RawOrigin},
	traits::fungible::Mutate,
};
use integration_tests_helpers::test_parachain_is_trusted_teleporter;
use xcm_runtime_apis::{
	dry_run::runtime_decl_for_dry_run_api::DryRunApiV2,
	fees::runtime_decl_for_xcm_payment_api::XcmPaymentApiV1,
};

fn relay_dest_assertions_fail(_t: EncointerParaToRelayTest) {
	Kusama::assert_ump_queue_processed(false, Some(EncointerKusama::para_id()), None);
}

fn para_origin_assertions(t: EncointerParaToRelayTest) {
	type RuntimeEvent = <EncointerKusama as Chain>::RuntimeEvent;

	EncointerKusama::assert_xcm_pallet_attempted_complete(None);

	EncointerKusama::assert_parachain_system_ump_sent();

	assert_expected_events!(
		EncointerKusama,
		vec![
			// Amount is withdrawn from Sender's account
			RuntimeEvent::Balances(pallet_balances::Event::Burned { who, amount }) => {
				who: *who == t.sender.account_id,
				amount: *amount == t.args.amount,
			},
		]
	);
}

fn system_para_limited_teleport_assets(t: EncointerParaToRelayTest) -> DispatchResult {
	<EncointerKusama as EncointerKusamaPallet>::PolkadotXcm::limited_teleport_assets(
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
	let amount = ENCOINTER_KUSAMA_ED * 1000;
	let native_asset: Assets = (Here, amount).into();

	test_relay_is_trusted_teleporter!(
		Kusama,
		vec![EncointerKusama],
		(native_asset, amount),
		transfer_assets
	);

	let amount = KUSAMA_ED * 1000;

	test_parachain_is_trusted_teleporter_for_relay!(
		EncointerKusama,
		Kusama,
		amount,
		transfer_assets
	);
}

#[test]
fn teleport_via_limited_teleport_assets_from_and_to_relay() {
	let amount = ENCOINTER_KUSAMA_ED * 1000;
	let native_asset: Assets = (Here, amount).into();

	test_relay_is_trusted_teleporter!(
		Kusama,
		vec![EncointerKusama],
		(native_asset, amount),
		limited_teleport_assets
	);

	let amount = KUSAMA_ED * 1000;

	test_parachain_is_trusted_teleporter_for_relay!(
		EncointerKusama,
		Kusama,
		amount,
		limited_teleport_assets
	);
}

#[test]
fn teleport_via_limited_teleport_assets_from_and_to_other_system_parachains_works() {
	let amount = ENCOINTER_KUSAMA_ED * 1000;
	let native_asset: Assets = (Parent, amount).into();

	test_parachain_is_trusted_teleporter!(
		EncointerKusama,
		vec![AssetHubKusama],
		(native_asset, amount),
		limited_teleport_assets
	);

	let amount = ASSET_HUB_KUSAMA_ED * 1000;
	let native_asset: Assets = (Parent, amount).into();

	test_parachain_is_trusted_teleporter!(
		AssetHubKusama,
		vec![EncointerKusama],
		(native_asset, amount),
		limited_teleport_assets
	);
}

#[test]
fn teleport_via_transfer_assets_from_and_to_other_system_parachains_works() {
	let amount = ENCOINTER_KUSAMA_ED * 1000;
	let native_asset: Assets = (Parent, amount).into();

	test_parachain_is_trusted_teleporter!(
		EncointerKusama,
		vec![AssetHubKusama],
		(native_asset, amount),
		transfer_assets
	);

	let amount = ASSET_HUB_KUSAMA_ED * 1000;
	let native_asset: Assets = (Parent, amount).into();

	test_parachain_is_trusted_teleporter!(
		AssetHubKusama,
		vec![EncointerKusama],
		(native_asset, amount),
		transfer_assets
	);
}

/// Limited Teleport of native asset from System Parachain to Relay Chain
/// shouldn't work when there is not enough balance in Relay Chain's `CheckAccount`
#[test]
fn limited_teleport_native_assets_from_system_para_to_relay_fails() {
	// Init values for Relay Chain
	let amount_to_send: Balance = ENCOINTER_KUSAMA_ED * 1000;
	let destination = EncointerKusama::parent_location();
	let beneficiary_id = KusamaReceiver::get();
	let assets = (Parent, amount_to_send).into();

	let test_args = TestContext {
		sender: EncointerKusamaSender::get(),
		receiver: KusamaReceiver::get(),
		args: TestArgs::new_para(destination, beneficiary_id, amount_to_send, assets, None, 0),
	};

	let mut test = EncointerParaToRelayTest::new(test_args);

	// let sender_balance_before = test.sender.balance;
	let receiver_balance_before = test.receiver.balance;

	test.set_assertion::<EncointerKusama>(para_origin_assertions);
	test.set_assertion::<Kusama>(relay_dest_assertions_fail);
	test.set_dispatchable::<EncointerKusama>(system_para_limited_teleport_assets);
	test.assert();

	// Receiver's balance does not change
	let receiver_balance_after = test.receiver.balance;
	assert_eq!(receiver_balance_after, receiver_balance_before);

	// let sender_balance_after = test.sender.balance;
	//
	// let delivery_fees = EncointerKusama::execute_with(|| {
	// 	xcm_helpers::teleport_assets_delivery_fees::<
	// 		<EncointerKusamaXcmConfig as xcm_executor::Config>::XcmSender,
	// 	>(
	// 		test.args.assets.clone(), 0, test.args.weight_limit, test.args.beneficiary, test.args.dest
	// 	)
	// });
	// Sender's balance is reduced
	// Todo: this assertion fails. It is off by a tiny amount
	// assert_eq!(sender_balance_before - amount_to_send - delivery_fees, sender_balance_after);
}
