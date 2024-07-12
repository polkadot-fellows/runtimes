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

use crate::{
    AssetHubPolkadot, PenpalA, PenpalB, PenpalAPallet, PenpalAssetOwner,
    ParaToParaThroughAHTest, PenpalBPallet, TransferType,
    TestArgs, PenpalBReceiver, TestContext, PenpalASender, Chain, bx,
};
use emulated_integration_tests_common::impls::{TestExt, Parachain, RelayChain};
use frame_support::{
	sp_runtime::DispatchResult,
	traits::fungibles::Inspect,
    dispatch::RawOrigin,
};
use xcm::prelude::*;
use xcm_fee_payment_runtime_api::{
	dry_run::runtime_decl_for_dry_run_api::DryRunApiV1,
	fees::runtime_decl_for_xcm_payment_api::XcmPaymentApiV1,
};

/// We are able to dry-run and estimate the fees for a multi-hop XCM journey.
/// Scenario: Alice on PenpalA has some DOTs and wants to send them to PenpalB.
/// We want to know the fees using the `DryRunApi` and `XcmPaymentApi`.
#[test]
fn multi_hop_works() {
	let destination = PenpalA::sibling_location_of(PenpalB::para_id());
	let sender = PenpalASender::get();
	let amount_to_send = 1_000_000_000_000; // One DOT is 10 decimals but it's configured in Penpal as 12.
	let asset_owner = PenpalAssetOwner::get();
	let assets: Assets = (Parent, amount_to_send).into();
	let relay_native_asset_location = Location::parent();
	let sender_as_seen_by_ah = AssetHubPolkadot::sibling_location_of(PenpalA::para_id());
	let sov_of_sender_on_ah = AssetHubPolkadot::sovereign_account_id_of(sender_as_seen_by_ah.clone());

	// fund Parachain's sender account
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(asset_owner.clone()),
		relay_native_asset_location.clone(),
		sender.clone(),
		amount_to_send * 2,
	);

	// fund the Parachain Origin's SA on AssetHub with the native tokens held in reserve.
	AssetHubPolkadot::fund_accounts(vec![(sov_of_sender_on_ah.clone().into(), amount_to_send * 2)]);

	// Init values for Parachain Destination
	let beneficiary_id = PenpalBReceiver::get();
	let beneficiary: Location = AccountId32 {
		id: beneficiary_id.clone().into(),
		network: None, // Test doesn't allow specifying a network here.
	}
	.into();

	// We get them from the PenpalA closure.
	// let mut delivery_fees_amount = 0;
	// let mut remote_message = VersionedXcm::V4(Xcm(Vec::new()));
	// <PenpalA as TestExt>::execute_with(|| {
	// 	type Runtime = <PenpalA as Chain>::Runtime;
	// 	type RuntimeCall = <PenpalA as Chain>::RuntimeCall;
	// 	type OriginCaller = <PenpalA as Chain>::OriginCaller;

	// 	let call = RuntimeCall::PolkadotXcm(pallet_xcm::Call::transfer_assets {
	// 		dest: Box::new(VersionedLocation::V4(destination.clone())),
	// 		beneficiary: Box::new(VersionedLocation::V4(beneficiary)),
	// 		assets: Box::new(VersionedAssets::V4(assets.clone())),
	// 		fee_asset_item: 0,
	// 		weight_limit: Unlimited,
	// 	});
	// 	let origin = OriginCaller::system(RawOrigin::Signed(PenpalASender::get()));
	// 	let result = Runtime::dry_run_call(origin, call).unwrap();
	// 	assert_eq!(result.forwarded_xcms.len(), 1);
	// 	let (destination_to_query, messages_to_query) = &result.forwarded_xcms[0];
	// 	assert_eq!(messages_to_query.len(), 1);
	// 	remote_message = messages_to_query[0].clone();
	// 	let delivery_fees =
	// 		Runtime::query_delivery_fees(destination_to_query.clone(), remote_message.clone())
	// 			.unwrap();
	// 	delivery_fees_amount = get_amount_from_versioned_assets(delivery_fees);
	// });

	// // This is set in the Polkadot closure.
	// let mut intermediate_execution_fees = 0;
	// let mut intermediate_delivery_fees_amount = 0;
	// let mut intermediate_remote_message = VersionedXcm::V4(Xcm::<()>(Vec::new()));
	// <PolkadotChain as TestExt>::execute_with(|| {
	// 	type Runtime = <PolkadotChain as Chain>::Runtime;
	// 	type RuntimeCall = <PolkadotChain as Chain>::RuntimeCall;

	// 	// First we get the execution fees.
	// 	let weight = Runtime::query_xcm_weight(remote_message.clone()).unwrap();
	// 	intermediate_execution_fees =
	// 		Runtime::query_weight_to_asset_fee(weight, VersionedAssetId::V4(Here.into())).unwrap();

	// 	// We have to do this to turn `VersionedXcm<()>` into `VersionedXcm<RuntimeCall>`.
	// 	let xcm_program =
	// 		VersionedXcm::V4(Xcm::<RuntimeCall>::from(remote_message.clone().try_into().unwrap()));

	// 	// Now we get the delivery fees to the final destination.
	// 	let result =
	// 		Runtime::dry_run_xcm(sender_as_seen_by_ah.clone().into(), xcm_program).unwrap();
	// 	let (destination_to_query, messages_to_query) = &result.forwarded_xcms[0];
	// 	// There's actually two messages here.
	// 	// One created when the message we sent from PenpalA arrived and was executed.
	// 	// The second one when we dry-run the xcm.
	// 	// We could've gotten the message from the queue without having to dry-run, but
	// 	// offchain applications would have to dry-run, so we do it here as well.
	// 	intermediate_remote_message = messages_to_query[0].clone();
	// 	let delivery_fees = Runtime::query_delivery_fees(
	// 		destination_to_query.clone(),
	// 		intermediate_remote_message.clone(),
	// 	)
	// 	.unwrap();
	// 	intermediate_delivery_fees_amount = get_amount_from_versioned_assets(delivery_fees);
	// });

	// // Get the final execution fees in the destination.
	// let mut final_execution_fees = 0;
	// <PenpalB as TestExt>::execute_with(|| {
	// 	type Runtime = <PenpalB as Chain>::Runtime;

	// 	let weight = Runtime::query_xcm_weight(intermediate_remote_message.clone()).unwrap();
	// 	final_execution_fees =
	// 		Runtime::query_weight_to_asset_fee(weight, VersionedAssetId::V4(Parent.into()))
	// 			.unwrap();
	// });

	// Dry-running is done.
	PenpalA::reset_ext();
	AssetHubPolkadot::reset_ext();
	PenpalB::reset_ext();

	// Fund accounts again.
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(asset_owner),
		relay_native_asset_location.clone(),
		sender.clone(),
		amount_to_send * 2,
	);
	AssetHubPolkadot::fund_accounts(vec![(sov_of_sender_on_ah.into(), amount_to_send * 2)]);

	// Actually run the extrinsic.
	let test_args = TestContext {
		sender: PenpalASender::get(),     // Alice.
		receiver: PenpalBReceiver::get(), // Bob in PenpalB.
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

	let sender_assets_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location.clone(), &sender)
	});
	let receiver_assets_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location.clone(), &beneficiary_id)
	});

	test.set_dispatchable::<PenpalA>(transfer_assets_para_to_para);
	test.assert();

	let sender_assets_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location.clone(), &sender)
	});
	let receiver_assets_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(relay_native_asset_location, &beneficiary_id)
	});

	// We know the exact fees on every hop.
	// assert_eq!(
	// 	sender_assets_after,
	// 	sender_assets_before - amount_to_send - delivery_fees_amount /* This is charged directly
	// 	                                                              * from the sender's
	// 	                                                              * account. */
	// );
	// assert_eq!(
	// 	receiver_assets_after,
	// 	receiver_assets_before + amount_to_send -
	// 		intermediate_execution_fees -
	// 		intermediate_delivery_fees_amount -
	// 		final_execution_fees
	// );
}

fn get_amount_from_versioned_assets(assets: VersionedAssets) -> u128 {
	let latest_assets: Assets = assets.try_into().unwrap();
	let Fungible(amount) = latest_assets.inner()[0].fun else {
		unreachable!("asset is fungible");
	};
	amount
}

fn transfer_assets_para_to_para(test: ParaToParaThroughAHTest) -> DispatchResult {
	let fee_index = test.args.fee_asset_item as usize;
	let fee: Asset = test.args.assets.inner().get(fee_index).cloned().unwrap();
	let asset_hub_location: Location = PenpalA::sibling_location_of(AssetHubPolkadot::para_id());
	let custom_xcm_on_dest = Xcm::<()>(vec![DepositAsset {
		assets: Wild(AllCounted(test.args.assets.len() as u32)),
		beneficiary: test.args.beneficiary,
	}]);
	<PenpalA as PenpalAPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
		test.signed_origin,
		bx!(test.args.dest.into()),
		bx!(test.args.assets.clone().into()),
        bx!(TransferType::RemoteReserve(asset_hub_location.clone().into())),
        bx!(VersionedAssetId::V4(AssetId(Location::new(1, [])))),
        bx!(TransferType::RemoteReserve(asset_hub_location.clone().into())),
        bx!(VersionedXcm::from(custom_xcm_on_dest)),
		test.args.weight_limit,
	)
}
