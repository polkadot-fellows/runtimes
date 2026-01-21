// This file is part of Cumulus.

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

extern crate alloc;

use alloc::{vec, vec::Vec};
use asset_hub_polkadot_runtime::xcm_config::bridging::{
	to_ethereum::{
		BridgeHubEthereumBaseFee, EthereumNetwork, EthereumNetworkExportTableV1,
		EthereumNetworkExportTableV2,
	},
	SiblingBridgeHub, XcmBridgeHubRouterFeeAssetId,
};
use sp_core::H160;
use xcm::latest::prelude::*;
use xcm_builder::ExporterFor;

#[test]
fn network_export_table_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		#[allow(clippy::type_complexity)]
		let test_data: Vec<(NetworkId, InteriorLocation, Option<(Location, Option<Asset>)>)> = vec![
			// From Ethereum (from GlobalConsensus(Ethereum) is routed to BridgeHub, with a fee,
			// matched.
			(
				EthereumNetwork::get(),
				Here,
				Some((
					SiblingBridgeHub::get(),
					Some(Asset {
						id: XcmBridgeHubRouterFeeAssetId::get(),
						fun: Fungible(BridgeHubEthereumBaseFee::get()),
					}),
				)),
			),
			// From Ethereum with a random parachain ID filter, not matched.
			(EthereumNetwork::get(), [Parachain(4321)].into(), None),
			// From Ethereum with a account ID added to the Ethereum Network, not matched.
			(
				EthereumNetwork::get(),
				[
					GlobalConsensus(EthereumNetwork::get()),
					AccountKey20 { network: None, key: H160::random().into() },
				]
				.into(),
				None,
			),
			// From Ethereum with the Sepolia chain ID instead of Mainnet, not matched.
			(Ethereum { chain_id: 11155111 }, Here, None),
		];

		for (network, remote_location, expected_result) in test_data {
			assert_eq!(
				EthereumNetworkExportTableV1::exporter_for(
					&network,
					&remote_location,
					&Xcm::default()
				),
				expected_result,
				"EthereumBridgeTableV1: expected_result: {expected_result:?} not matched for network: {network:?} and remote_location: {remote_location:?}",
			);

			assert_eq!(
				EthereumNetworkExportTableV2::exporter_for(
					&network,
					&remote_location,
					&Xcm::default()
				),
				expected_result,
				"EthereumBridgeTableV2: expected_result: {expected_result:?} not matched for network: {network:?} and remote_location: {remote_location:?}",
			);
		}
	});
}
