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
use bridge_hub_kusama_runtime::RuntimeEvent;
use frame_support::{dispatch::RawOrigin, traits::fungible::Mutate};
use xcm_runtime_apis::dry_run::runtime_decl_for_dry_run_api::DryRunApiV1;

fn send_asset_from_asset_hub_kusama_to_asset_hub_polkadot(id: Location, amount: u128) {
	let destination = asset_hub_polkadot_location();

	// fund the AHK's SA on BHK for paying bridge transport fees
	BridgeHubKusama::fund_para_sovereign(AssetHubKusama::para_id(), 10_000_000_000_000u128);

	// set XCM versions
	AssetHubKusama::force_xcm_version(destination.clone(), XCM_VERSION);
	BridgeHubKusama::force_xcm_version(bridge_hub_polkadot_location(), XCM_VERSION);

	// send message over bridge
	assert_ok!(send_asset_from_asset_hub_kusama(destination, (id, amount)));
	assert_bridge_hub_kusama_message_accepted(true);
	assert_bridge_hub_polkadot_message_received();
}

fn dry_run_send_asset_from_asset_hub_kusama_to_asset_hub_polkadot(id: Location, amount: u128) {
	let destination = asset_hub_polkadot_location();

	// fund the AHK's SA on BHK for paying bridge transport fees
	BridgeHubKusama::fund_para_sovereign(AssetHubKusama::para_id(), 10_000_000_000_000u128);

	// set XCM versions
	AssetHubKusama::force_xcm_version(destination.clone(), XCM_VERSION);
	BridgeHubKusama::force_xcm_version(bridge_hub_polkadot_location(), XCM_VERSION);

	let beneficiary: Location =
		AccountId32Junction { id: AssetHubPolkadotReceiver::get().into(), network: None }.into();
	let assets: Assets = (id, amount).into();
	let fee_asset_item = 0;
	let call =
		send_asset_from_asset_hub_kusama_call(destination, beneficiary, assets, fee_asset_item);

	// `remote_message` should contain `ExportMessage`
	let remote_message = AssetHubKusama::execute_with(|| {
		type Runtime = <AssetHubKusama as Chain>::Runtime;
		type OriginCaller = <AssetHubKusama as Chain>::OriginCaller;

		let origin = OriginCaller::system(RawOrigin::Signed(AssetHubKusamaSender::get()));
		let result = Runtime::dry_run_call(origin, call).unwrap();

		// We filter the result to get only the messages we are interested in.
		let (_, messages_to_query) = result
			.forwarded_xcms
			.into_iter()
			.find(|(destination, _)| {
				*destination == VersionedLocation::from(Location::new(1, [Parachain(1002)]))
			})
			.unwrap();
		assert_eq!(messages_to_query.len(), 1);

		messages_to_query[0].clone()
	});

	// dry run extracted `remote_message` on local BridgeHub
	BridgeHubKusama::execute_with(|| {
		type Runtime = <BridgeHubKusama as Chain>::Runtime;
		type RuntimeCall = <BridgeHubKusama as Chain>::RuntimeCall;

		// We have to do this to turn `VersionedXcm<()>` into `VersionedXcm<RuntimeCall>`.
		let xcm_program = VersionedXcm::from(Xcm::<RuntimeCall>::from(
			remote_message.clone().try_into().unwrap(),
		));

		// dry run program
		let asset_hub_as_seen_by_bridge_hub: Location = Location::new(1, [Parachain(1000)]);
		let result =
			Runtime::dry_run_xcm(asset_hub_as_seen_by_bridge_hub.into(), xcm_program).unwrap();

		// check dry run result
		assert_ok!(result.execution_result.ensure_complete());
		assert!(result.emitted_events.iter().any(|event| matches!(
			event,
			RuntimeEvent::BridgePolkadotMessages(
				pallet_bridge_messages::Event::MessageAccepted { .. }
			)
		)));
	});

	// After dry-running we reset.
	AssetHubKusama::reset_ext();
	BridgeHubKusama::reset_ext();
}

#[test]
fn send_ksms_from_asset_hub_kusama_to_asset_hub_polkadot() {
	let ksm_at_asset_hub_kusama: v3::Location = v3::Parent.into();
	let ksm_at_asset_hub_polkadot =
		v3::Location::new(2, [v3::Junction::GlobalConsensus(v3::NetworkId::Kusama)]);
	let owner: AccountId = AssetHubPolkadot::account_id_of(ALICE);
	AssetHubPolkadot::force_create_foreign_asset(
		ksm_at_asset_hub_polkadot,
		owner,
		true,
		ASSET_MIN_BALANCE,
		vec![],
	);
	let sov_ahp_on_ahk = AssetHubKusama::sovereign_account_of_parachain_on_other_global_consensus(
		NetworkId::Polkadot,
		AssetHubPolkadot::para_id(),
	);

	let ksms_in_reserve_on_ahk_before =
		<AssetHubKusama as Chain>::account_data_of(sov_ahp_on_ahk.clone()).free;
	let sender_ksms_before =
		<AssetHubKusama as Chain>::account_data_of(AssetHubKusamaSender::get()).free;
	let receiver_ksms_before = AssetHubPolkadot::execute_with(|| {
		type Assets = <AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(ksm_at_asset_hub_polkadot, &AssetHubPolkadotReceiver::get())
	});

	let ksm_at_asset_hub_kusama_latest: Location = ksm_at_asset_hub_kusama.try_into().unwrap();
	let amount = ASSET_HUB_KUSAMA_ED * 1_000;
	// First dry-run.
	dry_run_send_asset_from_asset_hub_kusama_to_asset_hub_polkadot(
		ksm_at_asset_hub_kusama_latest.clone(),
		amount,
	);
	// Then send.
	send_asset_from_asset_hub_kusama_to_asset_hub_polkadot(ksm_at_asset_hub_kusama_latest, amount);
	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				// issue KSMs on AHP
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == ksm_at_asset_hub_kusama,
					owner: *owner == AssetHubPolkadotReceiver::get(),
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_ksms_after =
		<AssetHubKusama as Chain>::account_data_of(AssetHubKusamaSender::get()).free;
	let receiver_ksms_after = AssetHubPolkadot::execute_with(|| {
		type Assets = <AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(ksm_at_asset_hub_polkadot, &AssetHubPolkadotReceiver::get())
	});
	let ksms_in_reserve_on_ahk_after =
		<AssetHubKusama as Chain>::account_data_of(sov_ahp_on_ahk.clone()).free;

	// Sender's balance is reduced
	assert!(sender_ksms_before > sender_ksms_after);
	// Receiver's balance is increased
	assert!(receiver_ksms_after > receiver_ksms_before);
	// Reserve balance is reduced by sent amount
	assert_eq!(ksms_in_reserve_on_ahk_after, ksms_in_reserve_on_ahk_before + amount);
}

#[test]
fn send_dots_from_asset_hub_kusama_to_asset_hub_polkadot() {
	let prefund_amount = 10_000_000_000_000u128;
	let dot_at_asset_hub_kusama =
		v3::Location::new(2, [v3::Junction::GlobalConsensus(v3::NetworkId::Polkadot)]);
	let owner: AccountId = AssetHubPolkadot::account_id_of(ALICE);
	AssetHubKusama::force_create_foreign_asset(
		dot_at_asset_hub_kusama,
		owner,
		true,
		ASSET_MIN_BALANCE,
		vec![(AssetHubKusamaSender::get(), prefund_amount)],
	);

	// fund the AHK's SA on AHP with the DOT tokens held in reserve
	let sov_ahk_on_ahp = AssetHubPolkadot::sovereign_account_of_parachain_on_other_global_consensus(
		NetworkId::Kusama,
		AssetHubKusama::para_id(),
	);
	AssetHubPolkadot::fund_accounts(vec![(sov_ahk_on_ahp.clone(), prefund_amount)]);

	let dots_in_reserve_on_ahp_before =
		<AssetHubPolkadot as Chain>::account_data_of(sov_ahk_on_ahp.clone()).free;
	assert_eq!(dots_in_reserve_on_ahp_before, prefund_amount);
	let sender_dots_before = AssetHubKusama::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(dot_at_asset_hub_kusama, &AssetHubKusamaSender::get())
	});
	assert_eq!(sender_dots_before, prefund_amount);
	let receiver_dots_before =
		<AssetHubPolkadot as Chain>::account_data_of(AssetHubPolkadotReceiver::get()).free;

	let dot_at_asset_hub_kusama_latest: Location = dot_at_asset_hub_kusama.try_into().unwrap();
	let amount_to_send = ASSET_HUB_POLKADOT_ED * 1_000;
	send_asset_from_asset_hub_kusama_to_asset_hub_polkadot(
		dot_at_asset_hub_kusama_latest.clone(),
		amount_to_send,
	);
	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				// DOT is withdrawn from AHK's SA on AHP
				RuntimeEvent::Balances(
					pallet_balances::Event::Burned { who, amount }
				) => {
					who: *who == sov_ahk_on_ahp,
					amount: *amount == amount_to_send,
				},
				// DOTs deposited to beneficiary
				RuntimeEvent::Balances(pallet_balances::Event::Minted { who, .. }) => {
					who: *who == AssetHubPolkadotReceiver::get(),
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_dots_after = AssetHubKusama::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(dot_at_asset_hub_kusama, &AssetHubKusamaSender::get())
	});
	let receiver_dots_after =
		<AssetHubPolkadot as Chain>::account_data_of(AssetHubPolkadotReceiver::get()).free;
	let dots_in_reserve_on_ahp_after =
		<AssetHubPolkadot as Chain>::account_data_of(sov_ahk_on_ahp).free;

	// Sender's balance is reduced
	assert!(sender_dots_before > sender_dots_after);
	// Receiver's balance is increased
	assert!(receiver_dots_after > receiver_dots_before);
	// Reserve balance is reduced by sent amount
	assert_eq!(dots_in_reserve_on_ahp_after, dots_in_reserve_on_ahp_before - amount_to_send);
}

#[test]
fn send_ksms_from_asset_hub_kusama_to_asset_hub_polkadot_fee_from_pool() {
	let ksm_at_asset_hub_kusama: v3::Location = v3::Parent.into();
	let ksm_at_asset_hub_polkadot =
		v3::Location::new(2, [v3::Junction::GlobalConsensus(v3::NetworkId::Kusama)]);
	let owner: AccountId = AssetHubPolkadot::account_id_of(ALICE);
	AssetHubPolkadot::force_create_foreign_asset(
		ksm_at_asset_hub_polkadot,
		owner,
		false,
		ASSET_MIN_BALANCE,
		vec![],
	);
	let sov_ahp_on_ahk = AssetHubKusama::sovereign_account_of_parachain_on_other_global_consensus(
		NetworkId::Polkadot,
		AssetHubPolkadot::para_id(),
	);

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

		// setup a pool to pay xcm fees with `ksm_at_asset_hub_polkadot` tokens
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotSender::get()),
			ksm_at_asset_hub_polkadot,
			AssetHubPolkadotSender::get().into(),
			3_000_000_000_000,
		));

		<AssetHubPolkadot as AssetHubPolkadotPallet>::Balances::set_balance(
			&AssetHubPolkadotSender::get(),
			3_000_000_000_000,
		);

		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::AssetConversion::create_pool(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotSender::get()),
			Box::new(ksm_at_asset_hub_kusama),
			Box::new(ksm_at_asset_hub_polkadot),
		));

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::PoolCreated { .. }) => {},
			]
		);

		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::AssetConversion::add_liquidity(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotSender::get()),
			Box::new(ksm_at_asset_hub_kusama),
			Box::new(ksm_at_asset_hub_polkadot),
			1_000_000_000_000,
			2_000_000_000_000,
			1,
			1,
			AssetHubPolkadotSender::get()
		));

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::LiquidityAdded {..}) => {},
			]
		);
	});

	let ksms_in_reserve_on_ahk_before =
		<AssetHubKusama as Chain>::account_data_of(sov_ahp_on_ahk.clone()).free;
	let sender_ksms_before =
		<AssetHubKusama as Chain>::account_data_of(AssetHubKusamaSender::get()).free;
	let receiver_ksms_before = AssetHubPolkadot::execute_with(|| {
		type Assets = <AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(ksm_at_asset_hub_polkadot, &AssetHubPolkadotReceiver::get())
	});

	let ksm_at_asset_hub_kusama_latest: Location = ksm_at_asset_hub_kusama.try_into().unwrap();
	let amount = ASSET_HUB_KUSAMA_ED * 1_000;
	send_asset_from_asset_hub_kusama_to_asset_hub_polkadot(ksm_at_asset_hub_kusama_latest, amount);
	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				// issue KSMs on AHP
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == ksm_at_asset_hub_kusama,
					owner: *owner == AssetHubPolkadotReceiver::get(),
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_ksms_after =
		<AssetHubKusama as Chain>::account_data_of(AssetHubKusamaSender::get()).free;
	let receiver_ksms_after = AssetHubPolkadot::execute_with(|| {
		type Assets = <AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(ksm_at_asset_hub_polkadot, &AssetHubPolkadotReceiver::get())
	});
	let ksms_in_reserve_on_ahk_after =
		<AssetHubKusama as Chain>::account_data_of(sov_ahp_on_ahk.clone()).free;

	// Sender's balance is reduced
	assert!(sender_ksms_before >= sender_ksms_after + amount);
	// Receiver's balance is increased
	assert!(receiver_ksms_after > receiver_ksms_before);
	// Reserve balance has increased by sent amount
	assert_eq!(ksms_in_reserve_on_ahk_after, ksms_in_reserve_on_ahk_before + amount);
}

#[test]
fn send_dots_from_asset_hub_polkadot_to_asset_hub_kusama_fee_from_pool() {
	let prefund_amount = 10_000_000_000_000u128;
	let dot_at_asset_hub_kusama =
		v3::Location::new(2, [v3::Junction::GlobalConsensus(v3::NetworkId::Polkadot)]);
	let owner: AccountId = AssetHubPolkadot::account_id_of(BOB);
	AssetHubKusama::force_create_foreign_asset(
		dot_at_asset_hub_kusama,
		owner.clone(),
		false,
		ASSET_MIN_BALANCE,
		vec![(AssetHubKusamaSender::get(), prefund_amount)],
	);

	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;

		// setup a pool to pay xcm fees with `dot_at_asset_hub_kusama` tokens
		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets::mint(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(owner.clone()),
			dot_at_asset_hub_kusama,
			owner.clone().into(),
			3_000_000_000_000,
		));

		<AssetHubKusama as AssetHubKusamaPallet>::Balances::set_balance(&owner, 3_000_000_000_000);

		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::AssetConversion::create_pool(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(owner.clone()),
			Box::new(xcm::v3::Parent.into()),
			Box::new(dot_at_asset_hub_kusama),
		));

		assert_expected_events!(
			AssetHubKusama,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::PoolCreated { .. }) => {},
			]
		);

		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::AssetConversion::add_liquidity(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(owner.clone()),
			Box::new(xcm::v3::Parent.into()),
			Box::new(dot_at_asset_hub_kusama),
			1_000_000_000_000,
			2_000_000_000_000,
			1,
			1,
			owner
		));

		assert_expected_events!(
			AssetHubKusama,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::LiquidityAdded {..}) => {},
			]
		);
	});

	// fund the AHK's SA on AHP with the DOT tokens held in reserve
	let sov_ahk_on_ahp = AssetHubPolkadot::sovereign_account_of_parachain_on_other_global_consensus(
		NetworkId::Kusama,
		AssetHubKusama::para_id(),
	);
	AssetHubPolkadot::fund_accounts(vec![(sov_ahk_on_ahp.clone(), prefund_amount)]);

	let dots_in_reserve_on_ahp_before =
		<AssetHubPolkadot as Chain>::account_data_of(sov_ahk_on_ahp.clone()).free;
	assert_eq!(dots_in_reserve_on_ahp_before, prefund_amount);
	let sender_dots_before = AssetHubKusama::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(dot_at_asset_hub_kusama, &AssetHubKusamaSender::get())
	});
	assert_eq!(sender_dots_before, prefund_amount);
	let receiver_dots_before =
		<AssetHubPolkadot as Chain>::account_data_of(AssetHubPolkadotReceiver::get()).free;

	let dot_at_asset_hub_kusama_latest: Location = dot_at_asset_hub_kusama.try_into().unwrap();
	let amount_to_send = ASSET_HUB_POLKADOT_ED * 1_000;
	send_asset_from_asset_hub_kusama_to_asset_hub_polkadot(
		dot_at_asset_hub_kusama_latest.clone(),
		amount_to_send,
	);
	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				// DOT is withdrawn from AHK's SA on AHP
				RuntimeEvent::Balances(
					pallet_balances::Event::Burned { who, amount }
				) => {
					who: *who == sov_ahk_on_ahp,
					amount: *amount == amount_to_send,
				},
				// DOTs deposited to beneficiary
				RuntimeEvent::Balances(pallet_balances::Event::Minted { who, .. }) => {
					who: *who == AssetHubPolkadotReceiver::get(),
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_dots_after = AssetHubKusama::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(dot_at_asset_hub_kusama, &AssetHubKusamaSender::get())
	});
	let receiver_dots_after =
		<AssetHubPolkadot as Chain>::account_data_of(AssetHubPolkadotReceiver::get()).free;
	let dots_in_reserve_on_ahp_after =
		<AssetHubPolkadot as Chain>::account_data_of(sov_ahk_on_ahp).free;

	// Sender's balance is reduced
	assert!(sender_dots_before >= sender_dots_after + amount_to_send);
	// Receiver's balance is increased
	assert!(receiver_dots_after > receiver_dots_before);
	// Reserve balance is reduced by sent amount
	assert_eq!(dots_in_reserve_on_ahp_after, dots_in_reserve_on_ahp_before - amount_to_send);
}
