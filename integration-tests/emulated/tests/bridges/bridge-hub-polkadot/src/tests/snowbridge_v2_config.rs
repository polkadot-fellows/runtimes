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

use asset_hub_polkadot_runtime::Runtime as AhRuntime;
use bridge_hub_polkadot_runtime::Runtime as BhRuntime;
use snowbridge_pallet_outbound_queue_v2::WeightInfo as OutboundQueueWeightInfo;
use snowbridge_pallet_system_frontend::BackendWeightInfo;
use snowbridge_pallet_system_v2::WeightInfo as SystemWeightInfo;

/// Verifies that the AssetHub backend weights are equal to or larger than the corresponding
/// BridgeHub extrinsic weight. If this test fails, please update
/// system-parachains/asset-hubs/asset-hub-polkadot/src/weights/snowbridge_pallet_system_backend.rs
/// with the corresponding weight value in the Polkadot Bridge Hub runtime.
#[test]
fn asset_hub_weights_should_be_equal_or_gte_bridge_hub_weights() {
	let bh_register_token =
		<BhRuntime as snowbridge_pallet_system_v2::Config>::WeightInfo::register_token();
	let bh_add_tip = <BhRuntime as snowbridge_pallet_system_v2::Config>::WeightInfo::add_tip();
	let bh_do_process_message =
		<BhRuntime as snowbridge_pallet_outbound_queue_v2::Config>::WeightInfo::do_process_message(
		);
	let bh_commit_single =
		<BhRuntime as snowbridge_pallet_outbound_queue_v2::Config>::WeightInfo::commit_single();
	let bh_submit_delivery_receipt = <BhRuntime as snowbridge_pallet_outbound_queue_v2::Config>::WeightInfo::submit_delivery_receipt();

	let ah_register_token = <AhRuntime as snowbridge_pallet_system_frontend::Config>::BackendWeightInfo::transact_register_token();
	let ah_add_tip = <AhRuntime as snowbridge_pallet_system_frontend::Config>::BackendWeightInfo::transact_add_tip();
	let ah_do_process_message = <AhRuntime as snowbridge_pallet_system_frontend::Config>::BackendWeightInfo::do_process_message();
	let ah_commit_single =
		<AhRuntime as snowbridge_pallet_system_frontend::Config>::BackendWeightInfo::commit_single(
		);
	let ah_submit_delivery_receipt = <AhRuntime as snowbridge_pallet_system_frontend::Config>::BackendWeightInfo::submit_delivery_receipt();

	assert!(
		ah_register_token.all_gte(bh_register_token),
		"Asset Hub register_token weight ({ah_register_token:?}) should be >= Bridge Hub weight ({bh_register_token:?})"
	);

	assert!(
		ah_add_tip.all_gte(bh_add_tip),
		"Asset Hub add_tip weight ({ah_add_tip:?}) should be >= Bridge Hub weight ({bh_add_tip:?})"
	);

	assert!(
		ah_do_process_message.all_gte(bh_do_process_message),
		"Asset Hub do_process_message weight ({ah_do_process_message:?}) should be >= Bridge Hub weight ({bh_do_process_message:?})"
	);

	assert!(
		ah_commit_single.all_gte(bh_commit_single),
		"Asset Hub commit_single weight ({ah_commit_single:?}) should be >= Bridge Hub weight ({bh_commit_single:?})"
	);

	assert!(
		ah_submit_delivery_receipt.all_gte(bh_submit_delivery_receipt),
		"Asset Hub submit_delivery_receipt weight ({ah_submit_delivery_receipt:?}) should be >= Bridge Hub weight ({bh_submit_delivery_receipt:?})"
	);
}
