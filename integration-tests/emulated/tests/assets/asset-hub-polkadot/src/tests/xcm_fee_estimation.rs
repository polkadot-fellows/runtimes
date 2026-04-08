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

//! Tests for XCM fee estimation in the runtime.

use crate::{
	assert_expected_events, bx, create_pool_with_dot_on, AssetHubPolkadot,
	AssetHubPolkadotAssetOwner, AssetHubPolkadotPallet, AssetHubPolkadotSender, Chain,
	ParaToParaThroughAHTest, PenpalA, PenpalAPallet, PenpalAReceiver, PenpalAssetOwner, PenpalB,
	PenpalBPallet, PenpalBSender, PenpalUsdtFromAssetHub, TestArgs, TestContext, TransferType,
	ASSETS_PALLET_ID,
};
use emulated_integration_tests_common::{
	impls::{Parachain, TestExt},
	xcm_helpers::get_amount_from_versioned_assets,
};
use frame_support::{
	assert_ok,
	dispatch::RawOrigin,
	sp_runtime::{traits::Dispatchable, DispatchResult},
	traits::fungibles::Inspect,
	BoundedVec,
};
use xcm::{latest::AssetTransferFilter, prelude::*};
use xcm_runtime_apis::{
	dry_run::runtime_decl_for_dry_run_api::DryRunApiV2,
	fees::runtime_decl_for_xcm_payment_api::XcmPaymentApiV2,
};

/// We are able to dry-run and estimate the fees for a multi-hop XCM journey.
/// Scenario: Alice on PenpalB has some DOTs and wants to send them to PenpalA.
/// We want to know the fees using the `DryRunApi` and `XcmPaymentApi`.
#[test]
fn multi_hop_works() {
	let destination = PenpalB::sibling_location_of(PenpalA::para_id());
	let sender = PenpalBSender::get();
	let amount_to_send = 1_000_000_000_000; // One DOT is 10 decimals but it's configured in Penpal as 12.
	let asset_owner = PenpalAssetOwner::get();
	let assets: Assets = (Parent, amount_to_send).into();
	let relay_native_asset_location = Location::parent();
	let sender_as_seen_by_ah = AssetHubPolkadot::sibling_location_of(PenpalB::para_id());
	let sov_of_sender_on_ah =
		AssetHubPolkadot::sovereign_account_id_of(sender_as_seen_by_ah.clone());

	// fund Parachain's sender account
	PenpalB::mint_foreign_asset(
		<PenpalB as Chain>::RuntimeOrigin::signed(asset_owner.clone()),
		relay_native_asset_location.clone(),
		sender.clone(),
		amount_to_send * 2,
	);

	// fund the Parachain Origin's SA on AssetHub with the native tokens held in reserve.
	AssetHubPolkadot::fund_accounts(vec![(sov_of_sender_on_ah.clone(), amount_to_send * 2)]);

	// Init values for Parachain Destination
	let beneficiary_id = PenpalAReceiver::get();

	let test_args = TestContext {
		sender: PenpalBSender::get(),     // Bob in PenpalB.
		receiver: PenpalAReceiver::get(), // Alice.
		args: TestArgs::new_para(
			destination,
			beneficiary_id.clone(),
			amount_to_send,
			assets,
			None,
			0,
		),
	};
	let mut test = ParaToParaThroughAHTest::new(test_args);

	// We get them from the PenpalB closure.
	let mut delivery_fees_amount = 0;
	let mut remote_message = VersionedXcm::from(Xcm(Vec::new()));
	<PenpalB as TestExt>::execute_with(|| {
		type Runtime = <PenpalB as Chain>::Runtime;
		type OriginCaller = <PenpalB as Chain>::OriginCaller;

		let call = transfer_assets_para_to_para_through_ah_call(test.clone());
		let origin = OriginCaller::system(RawOrigin::Signed(sender.clone()));
		let result = Runtime::dry_run_call(origin, call, xcm::prelude::XCM_VERSION).unwrap();
		// We filter the result to get only the messages we are interested in.
		let (destination_to_query, messages_to_query) = &result
			.forwarded_xcms
			.iter()
			.find(|(destination, _)| {
				*destination == VersionedLocation::from(Location::new(1, [Parachain(1000)]))
			})
			.unwrap();
		assert_eq!(messages_to_query.len(), 1);
		remote_message = messages_to_query[0].clone();
		let asset_id_for_delivery_fees = VersionedAssetId::from(Location::parent());
		let delivery_fees = Runtime::query_delivery_fees(
			destination_to_query.clone(),
			remote_message.clone(),
			asset_id_for_delivery_fees,
		)
		.unwrap();
		delivery_fees_amount = get_amount_from_versioned_assets(delivery_fees);
	});

	// These are set in the AssetHub closure.
	let mut intermediate_execution_fees = 0;
	let mut intermediate_delivery_fees_amount = 0;
	let mut intermediate_remote_message = VersionedXcm::from(Xcm::<()>(Vec::new()));
	<AssetHubPolkadot as TestExt>::execute_with(|| {
		type Runtime = <AssetHubPolkadot as Chain>::Runtime;
		type RuntimeCall = <AssetHubPolkadot as Chain>::RuntimeCall;

		// First we get the execution fees.
		let weight = Runtime::query_xcm_weight(remote_message.clone()).unwrap();
		intermediate_execution_fees = Runtime::query_weight_to_asset_fee(
			weight,
			VersionedAssetId::from(AssetId(Location::parent())),
		)
		.unwrap();

		// We have to do this to turn `VersionedXcm<()>` into `VersionedXcm<RuntimeCall>`.
		let xcm_program = VersionedXcm::from(Xcm::<RuntimeCall>::from(
			remote_message.clone().try_into().unwrap(),
		));

		// Now we get the delivery fees to the final destination.
		let result =
			Runtime::dry_run_xcm(sender_as_seen_by_ah.clone().into(), xcm_program).unwrap();
		let (destination_to_query, messages_to_query) = &result
			.forwarded_xcms
			.iter()
			.find(|(destination, _)| {
				*destination == VersionedLocation::from(Location::new(1, [Parachain(2000)]))
			})
			.unwrap();
		// There's actually two messages here.
		// One created when the message we sent from PenpalA arrived and was executed.
		// The second one when we dry-run the xcm.
		// We could've gotten the message from the queue without having to dry-run, but
		// offchain applications would have to dry-run, so we do it here as well.
		intermediate_remote_message = messages_to_query[0].clone();
		let asset_id_for_delivery_fees = VersionedAssetId::from(Location::parent());
		let delivery_fees = Runtime::query_delivery_fees(
			destination_to_query.clone(),
			intermediate_remote_message.clone(),
			asset_id_for_delivery_fees,
		)
		.unwrap();
		intermediate_delivery_fees_amount = get_amount_from_versioned_assets(delivery_fees);
	});

	// Get the final execution fees in the destination.
	let mut final_execution_fees = 0;
	<PenpalA as TestExt>::execute_with(|| {
		type Runtime = <PenpalA as Chain>::Runtime;

		let weight = Runtime::query_xcm_weight(intermediate_remote_message.clone()).unwrap();
		final_execution_fees = Runtime::query_weight_to_asset_fee(
			weight,
			VersionedAssetId::from(AssetId(Location::parent())),
		)
		.unwrap();
	});

	// Dry-running is done.
	PenpalB::reset_ext();
	AssetHubPolkadot::reset_ext();
	PenpalA::reset_ext();

	// Fund accounts again.
	PenpalB::mint_foreign_asset(
		<PenpalB as Chain>::RuntimeOrigin::signed(asset_owner),
		relay_native_asset_location.clone(),
		sender.clone(),
		amount_to_send * 2,
	);
	AssetHubPolkadot::fund_accounts(vec![(sov_of_sender_on_ah, amount_to_send * 2)]);

	// Actually run the extrinsic.
	let sender_assets_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location.clone(), &sender)
	});
	let receiver_assets_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location.clone(), &beneficiary_id)
	});

	test.set_assertion::<PenpalB>(sender_assertions);
	test.set_assertion::<AssetHubPolkadot>(hop_assertions);
	test.set_assertion::<PenpalA>(receiver_assertions);
	test.set_dispatchable::<PenpalB>(transfer_assets_para_to_para_through_ah_dispatchable);
	test.assert();

	let sender_assets_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location.clone(), &sender)
	});
	let receiver_assets_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location, &beneficiary_id)
	});

	// We know the exact fees on every hop.
	assert_eq!(
		sender_assets_after,
		sender_assets_before - amount_to_send - delivery_fees_amount /* This is charged directly
		                                                              * from the sender's
		                                                              * account. */
	);
	assert_eq!(
		receiver_assets_after,
		receiver_assets_before + amount_to_send -
			intermediate_execution_fees -
			intermediate_delivery_fees_amount -
			final_execution_fees
	);
}

fn sender_assertions(test: ParaToParaThroughAHTest) {
	type RuntimeEvent = <PenpalB as Chain>::RuntimeEvent;
	PenpalB::assert_xcm_pallet_attempted_complete(None);

	assert_expected_events!(
		PenpalB,
		vec![
			RuntimeEvent::ForeignAssets(
				pallet_assets::Event::Withdrawn { asset_id, who, amount }
			) => {
				asset_id: *asset_id == Location::parent(),
				who: *who == test.sender.account_id,
				amount: *amount == test.args.amount,
			},
		]
	);
}

fn hop_assertions(test: ParaToParaThroughAHTest) {
	type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
	AssetHubPolkadot::assert_xcmp_queue_success(None);

	assert_expected_events!(
		AssetHubPolkadot,
		vec![
			RuntimeEvent::Balances(
				pallet_balances::Event::Withdraw { amount, .. }
			) => {
				amount: *amount == test.args.amount,
			},
		]
	);
}

fn receiver_assertions(test: ParaToParaThroughAHTest) {
	type RuntimeEvent = <PenpalA as Chain>::RuntimeEvent;
	PenpalA::assert_xcmp_queue_success(None);

	assert_expected_events!(
		PenpalA,
		vec![
			RuntimeEvent::ForeignAssets(
				pallet_assets::Event::Deposited { asset_id, who, .. }
			) => {
				asset_id: *asset_id == Location::parent(),
				who: *who == test.receiver.account_id,
			},
		]
	);
}

fn transfer_assets_para_to_para_through_ah_dispatchable(
	test: ParaToParaThroughAHTest,
) -> DispatchResult {
	let call = transfer_assets_para_to_para_through_ah_call(test.clone());
	match call.dispatch(test.signed_origin) {
		Ok(_) => Ok(()),
		Err(error_with_post_info) => Err(error_with_post_info.error),
	}
}

fn transfer_assets_para_to_para_through_ah_call(
	test: ParaToParaThroughAHTest,
) -> <PenpalB as Chain>::RuntimeCall {
	type RuntimeCall = <PenpalB as Chain>::RuntimeCall;

	let asset_hub_location: Location = PenpalB::sibling_location_of(AssetHubPolkadot::para_id());
	let custom_xcm_on_dest = Xcm::<()>(vec![DepositAsset {
		assets: Wild(AllCounted(test.args.assets.len() as u32)),
		beneficiary: test.args.beneficiary,
	}]);
	RuntimeCall::PolkadotXcm(pallet_xcm::Call::transfer_assets_using_type_and_then {
		dest: bx!(test.args.dest.into()),
		assets: bx!(test.args.assets.clone().into()),
		assets_transfer_type: bx!(TransferType::RemoteReserve(asset_hub_location.clone().into())),
		remote_fees_id: bx!(VersionedAssetId::from(AssetId(Location::parent()))),
		fees_transfer_type: bx!(TransferType::RemoteReserve(asset_hub_location.into())),
		custom_xcm_on_dest: bx!(VersionedXcm::from(custom_xcm_on_dest)),
		weight_limit: test.args.weight_limit,
	})
}

fn pay_fees_sender_assertions(test: ParaToParaThroughAHTest) {
	type RuntimeEvent = <PenpalB as Chain>::RuntimeEvent;
	PenpalB::assert_xcm_pallet_attempted_complete(None);

	assert_expected_events!(
		PenpalB,
		vec![
			RuntimeEvent::ForeignAssets(
				pallet_assets::Event::Withdrawn { asset_id, who, .. }
			) => {
				asset_id: *asset_id == Location::parent(),
				who: *who == test.sender.account_id,
			},
		]
	);
}

fn pay_fees_hop_assertions(_test: ParaToParaThroughAHTest) {
	type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
	AssetHubPolkadot::assert_xcmp_queue_success(None);

	assert_expected_events!(
		AssetHubPolkadot,
		vec![
			RuntimeEvent::Balances(
				pallet_balances::Event::Withdraw { .. }
			) => {},
		]
	);
}

fn pay_fees_receiver_assertions(test: ParaToParaThroughAHTest) {
	type RuntimeEvent = <PenpalA as Chain>::RuntimeEvent;
	PenpalA::assert_xcmp_queue_success(None);

	assert_expected_events!(
		PenpalA,
		vec![
			RuntimeEvent::ForeignAssets(
				pallet_assets::Event::Deposited { asset_id, who, .. }
			) => {
				asset_id: *asset_id == Location::parent(),
				who: *who == test.receiver.account_id,
			},
		]
	);
}

/// We are able to dry-run and estimate exact fees for a multi-hop XCM journey using PayFees.
/// Scenario: Alice on PenpalB sends DOTs to PenpalA through AssetHub, paying exact fees at each
/// hop.
#[test]
fn multi_hop_pay_fees_works() {
	let destination = PenpalB::sibling_location_of(PenpalA::para_id());
	let sender = PenpalBSender::get();
	let amount_to_send = 1_000_000_000_000u128;
	let asset_owner = PenpalAssetOwner::get();
	let relay_native_asset_location = Location::parent();
	let sender_as_seen_by_ah = AssetHubPolkadot::sibling_location_of(PenpalB::para_id());
	let sov_of_sender_on_ah =
		AssetHubPolkadot::sovereign_account_id_of(sender_as_seen_by_ah.clone());

	fn get_call(
		estimated_local_fees: impl Into<Asset>,
		estimated_intermediate_fees: impl Into<Asset>,
		estimated_remote_fees: impl Into<Asset>,
	) -> <PenpalB as Chain>::RuntimeCall {
		type RuntimeCall = <PenpalB as Chain>::RuntimeCall;

		let beneficiary = PenpalAReceiver::get();
		let xcm_in_destination = Xcm::<()>::builder_unsafe()
			.pay_fees(estimated_remote_fees)
			.deposit_asset(AllCounted(1), beneficiary)
			.build();
		let ah_to_receiver = AssetHubPolkadot::sibling_location_of(PenpalA::para_id());
		let xcm_in_reserve = Xcm::<()>::builder_unsafe()
			.pay_fees(estimated_intermediate_fees)
			.deposit_reserve_asset(AllCounted(1), ah_to_receiver, xcm_in_destination)
			.build();
		let sender_to_ah = PenpalB::sibling_location_of(AssetHubPolkadot::para_id());
		let local_xcm = Xcm::<<PenpalB as Chain>::RuntimeCall>::builder()
			.withdraw_asset((Parent, 1_000_000_000_000u128))
			.pay_fees(estimated_local_fees)
			.initiate_reserve_withdraw(AllCounted(1), sender_to_ah, xcm_in_reserve)
			.build();

		RuntimeCall::PolkadotXcm(pallet_xcm::Call::execute {
			message: Box::new(VersionedXcm::from(local_xcm)),
			max_weight: Weight::from_parts(10_000_000_000, 500_000),
		})
	}

	// Fund parachain's sender account.
	PenpalB::mint_foreign_asset(
		<PenpalB as Chain>::RuntimeOrigin::signed(asset_owner.clone()),
		relay_native_asset_location.clone(),
		sender.clone(),
		amount_to_send * 2,
	);

	// Fund the parachain origin's SA on AssetHub with native tokens held in reserve.
	AssetHubPolkadot::fund_accounts(vec![(sov_of_sender_on_ah.clone(), amount_to_send * 2)]);

	let beneficiary_id = PenpalAReceiver::get();

	let test_args = TestContext {
		sender: PenpalBSender::get(),
		receiver: PenpalAReceiver::get(),
		args: TestArgs::new_para(
			destination,
			beneficiary_id.clone(),
			amount_to_send,
			(Parent, amount_to_send).into(),
			None,
			0,
		),
	};
	let mut test = ParaToParaThroughAHTest::new(test_args);

	// Dry-run phase: estimate fees at each hop.
	let mut local_execution_fees = 0;
	let mut local_delivery_fees = 0;
	let mut remote_message = VersionedXcm::from(Xcm::<()>(Vec::new()));
	<PenpalB as TestExt>::execute_with(|| {
		type Runtime = <PenpalB as Chain>::Runtime;
		type OriginCaller = <PenpalB as Chain>::OriginCaller;

		let call = get_call(
			(Parent, 100_000_000_000u128),
			(Parent, 100_000_000_000u128),
			(Parent, 100_000_000_000u128),
		);
		let origin = OriginCaller::system(RawOrigin::Signed(sender.clone()));
		let result = Runtime::dry_run_call(origin, call, xcm::prelude::XCM_VERSION).unwrap();
		let local_xcm = result.local_xcm.unwrap().clone();
		let local_xcm_weight = Runtime::query_xcm_weight(local_xcm).unwrap();
		local_execution_fees = Runtime::query_weight_to_asset_fee(
			local_xcm_weight,
			VersionedAssetId::from(AssetId(Location::parent())),
		)
		.unwrap();
		let (destination_to_query, messages_to_query) = &result
			.forwarded_xcms
			.iter()
			.find(|(destination, _)| {
				*destination == VersionedLocation::from(Location::new(1, [Parachain(1000)]))
			})
			.unwrap();
		assert_eq!(messages_to_query.len(), 1);
		remote_message = messages_to_query[0].clone();
		let asset_id_for_delivery_fees = VersionedAssetId::from(Location::parent());
		let delivery_fees = Runtime::query_delivery_fees(
			destination_to_query.clone(),
			remote_message.clone(),
			asset_id_for_delivery_fees,
		)
		.unwrap();
		local_delivery_fees = get_amount_from_versioned_assets(delivery_fees);
	});

	let mut intermediate_execution_fees = 0;
	let mut intermediate_delivery_fees = 0;
	let mut intermediate_remote_message = VersionedXcm::from(Xcm::<()>(Vec::new()));
	<AssetHubPolkadot as TestExt>::execute_with(|| {
		type Runtime = <AssetHubPolkadot as Chain>::Runtime;
		type RuntimeCall = <AssetHubPolkadot as Chain>::RuntimeCall;

		let weight = Runtime::query_xcm_weight(remote_message.clone()).unwrap();
		intermediate_execution_fees = Runtime::query_weight_to_asset_fee(
			weight,
			VersionedAssetId::from(AssetId(Location::new(1, []))),
		)
		.unwrap();

		let xcm_program = VersionedXcm::from(Xcm::<RuntimeCall>::from(
			remote_message.clone().try_into().unwrap(),
		));

		let result =
			Runtime::dry_run_xcm(sender_as_seen_by_ah.clone().into(), xcm_program).unwrap();
		let (destination_to_query, messages_to_query) = &result
			.forwarded_xcms
			.iter()
			.find(|(destination, _)| {
				*destination == VersionedLocation::from(Location::new(1, [Parachain(2000)]))
			})
			.unwrap();
		intermediate_remote_message = messages_to_query[0].clone();
		let asset_id_for_delivery_fees = VersionedAssetId::from(Location::parent());
		let delivery_fees = Runtime::query_delivery_fees(
			destination_to_query.clone(),
			intermediate_remote_message.clone(),
			asset_id_for_delivery_fees,
		)
		.unwrap();
		intermediate_delivery_fees = get_amount_from_versioned_assets(delivery_fees);
	});

	let mut final_execution_fees = 0;
	<PenpalA as TestExt>::execute_with(|| {
		type Runtime = <PenpalA as Chain>::Runtime;

		let weight = Runtime::query_xcm_weight(intermediate_remote_message.clone()).unwrap();
		final_execution_fees = Runtime::query_weight_to_asset_fee(
			weight,
			VersionedAssetId::from(AssetId(Location::parent())),
		)
		.unwrap();
	});

	// Dry-running is done. Reset and re-fund.
	PenpalB::reset_ext();
	AssetHubPolkadot::reset_ext();
	PenpalA::reset_ext();

	PenpalB::mint_foreign_asset(
		<PenpalB as Chain>::RuntimeOrigin::signed(asset_owner),
		relay_native_asset_location.clone(),
		sender.clone(),
		amount_to_send * 2,
	);
	AssetHubPolkadot::fund_accounts(vec![(sov_of_sender_on_ah, amount_to_send * 2)]);

	// Actually run the extrinsic with exact fees.
	let sender_assets_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location.clone(), &sender)
	});
	let receiver_assets_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location.clone(), &beneficiary_id)
	});

	test.set_assertion::<PenpalB>(pay_fees_sender_assertions);
	test.set_assertion::<AssetHubPolkadot>(pay_fees_hop_assertions);
	test.set_assertion::<PenpalA>(pay_fees_receiver_assertions);
	let call = get_call(
		(Parent, local_execution_fees + local_delivery_fees),
		(Parent, intermediate_execution_fees + intermediate_delivery_fees),
		(Parent, final_execution_fees),
	);
	test.set_call(call);
	test.assert();

	let sender_assets_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location.clone(), &sender)
	});
	let receiver_assets_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location, &beneficiary_id)
	});

	// We know the exact fees on every hop.
	assert_eq!(sender_assets_after, sender_assets_before - amount_to_send);
	assert_eq!(
		receiver_assets_after,
		receiver_assets_before + amount_to_send -
			local_execution_fees -
			local_delivery_fees -
			intermediate_execution_fees -
			intermediate_delivery_fees -
			final_execution_fees
	);
}

fn usdt_transfer_call(
	destination: Location,
	beneficiary: Location,
	amount_to_send: u128,
	usdt_location_on_penpal: Location,
	usdt_location_on_ah: Location,
) -> <PenpalB as Chain>::RuntimeCall {
	let asset_hub_location: Location = PenpalB::sibling_location_of(AssetHubPolkadot::para_id());
	let remote_xcm_on_penpal_a =
		Xcm::<()>(vec![DepositAsset { assets: Wild(AllCounted(1)), beneficiary }]);
	let fee_amount = 10_000_000u128; // 10 USDT per hop
	let xcm_on_asset_hub = Xcm::<()>(vec![InitiateTransfer {
		destination,
		remote_fees: Some(AssetTransferFilter::ReserveDeposit(Definite(
			(usdt_location_on_ah, fee_amount).into(),
		))),
		preserve_origin: false,
		assets: BoundedVec::truncate_from(vec![AssetTransferFilter::ReserveDeposit(Wild(All))]),
		remote_xcm: remote_xcm_on_penpal_a,
	}]);
	let xcm = Xcm::<<PenpalB as Chain>::RuntimeCall>(vec![
		WithdrawAsset((usdt_location_on_penpal.clone(), amount_to_send).into()),
		PayFees {
			asset: Asset {
				id: AssetId(usdt_location_on_penpal.clone()),
				fun: Fungible(fee_amount),
			},
		},
		InitiateTransfer {
			destination: asset_hub_location,
			remote_fees: Some(AssetTransferFilter::ReserveWithdraw(Definite(
				(usdt_location_on_penpal.clone(), fee_amount).into(),
			))),
			preserve_origin: false,
			assets: BoundedVec::truncate_from(vec![AssetTransferFilter::ReserveWithdraw(Wild(
				All,
			))]),
			remote_xcm: xcm_on_asset_hub,
		},
	]);
	<PenpalB as Chain>::RuntimeCall::PolkadotXcm(pallet_xcm::Call::execute {
		message: bx!(VersionedXcm::from(xcm)),
		max_weight: Weight::MAX,
	})
}

/// We are able to dry-run and estimate the fees for a multi-hop XCM journey paying in USDT.
/// Scenario: Alice on PenpalB sends USDT to PenpalA through AssetHub, paying fees in USDT.
#[test]
fn usdt_fee_estimation_in_usdt_works() {
	use emulated_integration_tests_common::USDT_ID;
	let destination = PenpalB::sibling_location_of(PenpalA::para_id());
	let sender = PenpalBSender::get();
	let amount_to_send = 100_000_000u128; // 100 USDT
	let usdt_location_on_penpal = PenpalUsdtFromAssetHub::get();
	let usdt_location_on_ah =
		Location::new(0, [PalletInstance(ASSETS_PALLET_ID), GeneralIndex(USDT_ID.into())]);
	let penpal_as_seen_by_ah = AssetHubPolkadot::sibling_location_of(PenpalB::para_id());
	let sov_of_penpal_on_ah =
		AssetHubPolkadot::sovereign_account_id_of(penpal_as_seen_by_ah.clone());
	PenpalB::mint_foreign_asset(
		<PenpalB as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		usdt_location_on_penpal.clone(),
		sender.clone(),
		amount_to_send * 2,
	);
	AssetHubPolkadot::mint_asset(
		<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotAssetOwner::get()),
		USDT_ID,
		sov_of_penpal_on_ah.clone(),
		amount_to_send * 2,
	);
	create_pool_with_dot_on!(
		AssetHubPolkadot,
		usdt_location_on_ah.clone(),
		false,
		AssetHubPolkadotSender::get(),
		1_000_000_000_000, // 100 DOT
		200_000_000        // 200 USDT
	);
	create_pool_with_dot_on!(
		PenpalB,
		usdt_location_on_penpal.clone(),
		true,
		PenpalAssetOwner::get(),
		1_000_000_000_000, // 100 DOT
		200_000_000        // 200 USDT
	);
	let beneficiary_id = PenpalAReceiver::get();
	let mut delivery_fees_amount = 0;
	let mut remote_message = VersionedXcm::from(Xcm(Vec::new()));
	<PenpalB as TestExt>::execute_with(|| {
		type Runtime = <PenpalB as Chain>::Runtime;
		type OriginCaller = <PenpalB as Chain>::OriginCaller;
		let call = usdt_transfer_call(
			destination.clone(),
			beneficiary_id.clone().into(),
			amount_to_send,
			usdt_location_on_penpal.clone(),
			usdt_location_on_ah.clone(),
		);
		let asset_hub_location: Location =
			PenpalB::sibling_location_of(AssetHubPolkadot::para_id());
		let origin = OriginCaller::system(RawOrigin::Signed(sender.clone()));
		let result = Runtime::dry_run_call(origin, call, xcm::prelude::XCM_VERSION).unwrap();
		let (destination_to_query, messages_to_query) = &result
			.forwarded_xcms
			.iter()
			.find(|(destination, _)| {
				*destination == VersionedLocation::from(asset_hub_location.clone())
			})
			.unwrap();
		assert_eq!(messages_to_query.len(), 1);
		remote_message = messages_to_query[0].clone();
		let usdt_asset_id = VersionedAssetId::from(AssetId(usdt_location_on_penpal.clone()));
		let delivery_fees = Runtime::query_delivery_fees(
			destination_to_query.clone(),
			remote_message.clone(),
			usdt_asset_id,
		)
		.unwrap();
		delivery_fees_amount = get_amount_from_versioned_assets(delivery_fees.clone());
		let fee_assets = match delivery_fees {
			VersionedAssets::V5(assets) => assets,
			_ => panic!("Expected V5 assets"),
		};
		assert_eq!(fee_assets.len(), 1);
		let fee_asset = fee_assets.get(0).unwrap();
		assert_eq!(fee_asset.id.0, usdt_location_on_penpal);
		if let Fungible(amount) = fee_asset.fun {
			assert!(amount > 0, "Delivery fees should be greater than 0");
		} else {
			panic!("Expected fungible delivery fees");
		}
	});
}
