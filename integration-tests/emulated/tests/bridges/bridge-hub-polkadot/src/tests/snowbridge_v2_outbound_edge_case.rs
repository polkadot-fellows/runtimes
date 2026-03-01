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
//

use crate::{
	tests::{
		snowbridge::CHAIN_ID,
		snowbridge_common::*,
		snowbridge_v2_outbound::{EthereumSystemFrontend, EthereumSystemFrontendCall},
		usdt_at_ah_polkadot,
	},
	*,
};
use asset_hub_polkadot_runtime::xcm_config::bridging::to_ethereum::BridgeHubEthereumBaseFee;
use emulated_integration_tests_common::PenpalATeleportableAssetLocation;
use frame_support::{assert_noop, BoundedVec};
use snowbridge_core::AssetMetadata;
use sp_runtime::DispatchError::BadOrigin;
use xcm::v5::AssetTransferFilter;

#[test]
fn register_penpal_a_asset_from_penpal_b_will_fail() {
	fund_on_bh();
	prefund_accounts_on_polkadot_asset_hub();
	set_up_eth_and_dot_pool_on_polkadot_asset_hub();
	set_trust_reserve_on_penpal();
	register_ethereum_assets_on_penpal();
	prefund_accounts_on_penpal_b();
	let penpal_user_location = Location::new(
		1,
		[
			Parachain(PenpalB::para_id().into()),
			AccountId32 { network: Some(Polkadot), id: PenpalBSender::get().into() },
		],
	);
	let asset_location_on_penpal = PenpalATeleportableAssetLocation::get();
	let penpal_a_asset_at_asset_hub = Location::new(1, [Parachain(PenpalA::para_id().into())])
		.appended_with(asset_location_on_penpal)
		.unwrap();
	PenpalB::execute_with(|| {
		type RuntimeOrigin = <PenpalB as Chain>::RuntimeOrigin;

		let local_fee_asset_on_penpal =
			Asset { id: AssetId(Location::parent()), fun: Fungible(LOCAL_FEE_AMOUNT_IN_DOT) };

		let remote_fee_asset_on_ah =
			Asset { id: AssetId(eth_location()), fun: Fungible(REMOTE_FEE_AMOUNT_IN_ETHER) };

		let remote_fee_asset_on_ethereum =
			Asset { id: AssetId(eth_location()), fun: Fungible(REMOTE_FEE_AMOUNT_IN_ETHER) };

		let call = EthereumSystemFrontend::EthereumSystemFrontend(
			EthereumSystemFrontendCall::RegisterToken {
				asset_id: Box::new(VersionedLocation::from(penpal_a_asset_at_asset_hub)),
				metadata: Default::default(),
				fee_asset: remote_fee_asset_on_ethereum.clone(),
			},
		);

		let assets = vec![
			local_fee_asset_on_penpal.clone(),
			remote_fee_asset_on_ah.clone(),
			remote_fee_asset_on_ethereum.clone(),
		];

		let xcm = VersionedXcm::from(Xcm(vec![
			WithdrawAsset(assets.clone().into()),
			PayFees { asset: local_fee_asset_on_penpal.clone() },
			InitiateTransfer {
				destination: asset_hub(),
				remote_fees: Some(AssetTransferFilter::ReserveWithdraw(Definite(
					remote_fee_asset_on_ah.clone().into(),
				))),
				preserve_origin: true,
				assets: BoundedVec::truncate_from(vec![AssetTransferFilter::ReserveWithdraw(
					Definite(remote_fee_asset_on_ethereum.clone().into()),
				)]),
				remote_xcm: Xcm(vec![
					DepositAsset { assets: Wild(All), beneficiary: penpal_user_location },
					Transact {
						origin_kind: OriginKind::Xcm,
						call: call.encode().into(),
						fallback_max_weight: None,
					},
				]),
			},
		]));

		assert_ok!(<PenpalB as PenpalBPallet>::PolkadotXcm::execute(
			RuntimeOrigin::root(),
			bx!(xcm.clone()),
			Weight::from(EXECUTION_WEIGHT),
		));
	});

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPolkadot,
			vec![RuntimeEvent::ForeignAssets(pallet_assets::Event::Burned { .. }) => {},]
		);
	});

	// No events should be emitted on the bridge hub
	BridgeHubPolkadot::execute_with(|| {
		assert_expected_events!(BridgeHubPolkadot, vec![]);
	});
}

#[test]
fn export_from_non_system_parachain_will_fail() {
	let penpal_location = Location::new(1, [Parachain(PenpalB::para_id().into())]);
	let penpal_sovereign = BridgeHubPolkadot::sovereign_account_id_of(penpal_location.clone());
	BridgeHubPolkadot::fund_accounts(vec![(penpal_sovereign.clone(), INITIAL_FUND)]);

	PenpalB::execute_with(|| {
		type RuntimeEvent = <PenpalB as Chain>::RuntimeEvent;
		type RuntimeOrigin = <PenpalB as Chain>::RuntimeOrigin;

		let relay_fee_asset =
			Asset { id: AssetId(Location::parent()), fun: Fungible(1_000_000_000_000) };

		let weth_location_reanchored =
			Location::new(0, [AccountKey20 { network: None, key: WETH }]);

		let weth_asset =
			Asset { id: AssetId(weth_location_reanchored.clone()), fun: Fungible(TOKEN_AMOUNT) };

		assert_ok!(<PenpalB as PenpalBPallet>::PolkadotXcm::send(
			RuntimeOrigin::root(),
			bx!(VersionedLocation::from(bridge_hub())),
			bx!(VersionedXcm::from(Xcm(vec![
				WithdrawAsset(relay_fee_asset.clone().into()),
				BuyExecution { fees: relay_fee_asset.clone(), weight_limit: Unlimited },
				ExportMessage {
					network: Ethereum { chain_id: CHAIN_ID },
					destination: Here,
					xcm: Xcm(vec![
						AliasOrigin(penpal_location),
						WithdrawAsset(weth_asset.clone().into()),
						DepositAsset { assets: Wild(All), beneficiary: beneficiary() },
						SetTopic([0; 32]),
					]),
				},
			]))),
		));

		assert_expected_events!(
			PenpalB,
			vec![RuntimeEvent::PolkadotXcm(pallet_xcm::Event::Sent{ .. }) => {},]
		);
	});

	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			BridgeHubPolkadot,
			vec![RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed{ success: false, .. }) => {},]
		);
	});
}

#[test]
pub fn register_usdt_not_from_owner_on_asset_hub_will_fail() {
	fund_on_bh();
	prefund_accounts_on_polkadot_asset_hub();
	AssetHubPolkadot::execute_with(|| {
		type RuntimeOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;

		let fees_asset =
			Asset { id: AssetId(eth_location()), fun: Fungible(REMOTE_FEE_AMOUNT_IN_ETHER) };

		assert_noop!(
			<AssetHubPolkadot as AssetHubPolkadotPallet>::SnowbridgeSystemFrontend::register_token(
				// The owner is Alice, while AssetHubPolkadotReceiver is Bob, so it should fail
				RuntimeOrigin::signed(AssetHubPolkadotReceiver::get()),
				bx!(VersionedLocation::from(usdt_at_ah_polkadot())),
				AssetMetadata {
					name: "usdt".as_bytes().to_vec().try_into().unwrap(),
					symbol: "usdt".as_bytes().to_vec().try_into().unwrap(),
					decimals: 6,
				},
				fees_asset
			),
			BadOrigin
		);
	});
}

#[test]
pub fn register_relay_token_from_asset_hub_user_origin_will_fail() {
	fund_on_bh();
	prefund_accounts_on_polkadot_asset_hub();
	AssetHubPolkadot::execute_with(|| {
		type RuntimeOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;

		let fees_asset =
			Asset { id: AssetId(eth_location()), fun: Fungible(REMOTE_FEE_AMOUNT_IN_ETHER) };

		assert_noop!(
			<AssetHubPolkadot as AssetHubPolkadotPallet>::SnowbridgeSystemFrontend::register_token(
				RuntimeOrigin::signed(AssetHubPolkadotSender::get()),
				bx!(VersionedLocation::from(Location { parents: 1, interior: [].into() })),
				AssetMetadata {
					name: "dot".as_bytes().to_vec().try_into().unwrap(),
					symbol: "DOT".as_bytes().to_vec().try_into().unwrap(),
					decimals: 10,
				},
				fees_asset,
			),
			BadOrigin
		);
	});
}

// A malicious user attempted to exploit the bridge by manually adding an AliasOrigin in the
// remoteXcm, successfully routing to the V2 path, but ultimately failing at the BH Exporter.
#[test]
pub fn exploit_v2_route_with_legacy_v1_transfer_will_fail() {
	prefund_accounts_on_polkadot_asset_hub();
	set_bridge_hub_ethereum_base_fee();

	// Set base transfer fee to Ethereum on AH.
	AssetHubPolkadot::execute_with(|| {
		type RuntimeOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;

		assert_ok!(<AssetHubPolkadot as Chain>::System::set_storage(
			RuntimeOrigin::root(),
			vec![(BridgeHubEthereumBaseFee::key().to_vec(), 1_000_000_000_u128.encode())],
		));
	});

	let remote_fee_asset =
		Asset { id: AssetId(eth_location()), fun: Fungible(REMOTE_FEE_AMOUNT_IN_ETHER) };

	let reserve_asset = Asset { id: AssetId(eth_location()), fun: Fungible(TOKEN_AMOUNT) };

	let assets = vec![reserve_asset.clone(), remote_fee_asset.clone()];

	let custom_xcm_on_dest = Xcm::<()>(vec![
		AliasOrigin(Location::parent()),
		DepositAsset { assets: Wild(AllCounted(2)), beneficiary: beneficiary() },
	]);

	assert_ok!(AssetHubPolkadot::execute_with(|| {
		<AssetHubPolkadot as AssetHubPolkadotPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
 			<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotSender::get()),
 			bx!(eth_location().into()),
 			bx!(assets.into()),
 			bx!(TransferType::DestinationReserve),
 			bx!(AssetId(eth_location()).into()),
 			bx!(TransferType::DestinationReserve),
 			bx!(VersionedXcm::from(custom_xcm_on_dest)),
 			Unlimited,
 		)
	}));

	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;
		// Check that the Ethereum message was queue in the Outbound Queue
		assert_expected_events!(
			BridgeHubPolkadot,
			vec![
				RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed{ success: false, .. }) => {},
			]
		);
	})
}
