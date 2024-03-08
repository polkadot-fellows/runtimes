use asset_hub_kusama_runtime::xcm_config::bridging::{
	to_ethereum::{BridgeHubEthereumBaseFee, BridgeTable},
	SiblingBridgeHub, XcmBridgeHubRouterFeeAssetId,
};
use frame_support::ensure;
use parachains_common::AccountId;
use sp_core::H160;
use sp_std::prelude::*;
use system_parachains_constants::kusama::snowbridge::EthereumNetwork;
use xcm::prelude::*;
use xcm_builder::{ExporterFor, NetworkExportTable, NetworkExportTableItem};

#[test]
fn network_export_table_works() {
	sp_io::TestExternalities::default().execute_with(|| {
		let test_data: Vec<(NetworkId, InteriorLocation, Option<(Location, Option<Asset>)>)> = vec![
			// From Ethereum (from GlobalConsensus(Ethereum) is routed to BridgeHub, with a fee,
			// matched.
			(
				EthereumNetwork::get(),
				Junctions::Here,
				Some((
					SiblingBridgeHub::get().into(),
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
			(NetworkId::Ethereum { chain_id: 11155111 }, Junctions::Here, None),
		];

		for (network, remote_location, expected_result) in test_data {
			assert_eq!(
				NetworkExportTable::<BridgeTable>::exporter_for(
					&network,
					&remote_location,
					&Xcm::default()
				),
				expected_result,
				"expected_result: {:?} not matched for network: {:?} and remote_location: {:?}",
				expected_result,
				network,
				remote_location,
			)
		}
	});
}
