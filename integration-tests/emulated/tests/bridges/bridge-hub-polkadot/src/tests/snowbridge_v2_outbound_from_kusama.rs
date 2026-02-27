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
	tests::{
		asset_hub_kusama_location, create_foreign_on_ah_polkadot,
		snowbridge_common::*,
		snowbridge_v2_outbound::{EthereumSystemFrontend, EthereumSystemFrontendCall},
	},
	*,
};
use frame_support::{traits::fungibles::Mutate, BoundedVec};
use xcm::latest::AssetTransferFilter;

// set up pool
pub(crate) fn set_up_pool_with_dot_on_ah_polkadot(
	asset: Location,
	is_foreign: bool,
	initial_fund: u128,
	initial_liquidity: u128,
) {
	let dot: Location = Parent.into();
	AssetHubPolkadot::fund_accounts(vec![(AssetHubPolkadotSender::get(), initial_fund)]);
	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		let owner = AssetHubPolkadotSender::get();
		let signed_owner = <AssetHubPolkadot as Chain>::RuntimeOrigin::signed(owner.clone());

		if is_foreign {
			assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint(
				signed_owner.clone(),
				asset.clone(),
				owner.clone().into(),
				initial_fund,
			));
		} else {
			let asset_id = match asset.interior.last() {
				Some(GeneralIndex(id)) => *id as u32,
				_ => unreachable!(),
			};
			assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::Assets::mint(
				signed_owner.clone(),
				asset_id.into(),
				owner.clone().into(),
				initial_fund,
			));
		}
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::AssetConversion::create_pool(
			signed_owner.clone(),
			Box::new(dot.clone()),
			Box::new(asset.clone()),
		));
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::PoolCreated { .. }) => {},
			]
		);
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::AssetConversion::add_liquidity(
			signed_owner.clone(),
			Box::new(dot),
			Box::new(asset),
			initial_liquidity,
			initial_liquidity,
			1,
			1,
			owner
		));
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::LiquidityAdded {..}) => {},
			]
		);
	});
}

pub(crate) fn assert_bridge_hub_kusama_message_accepted(expected_processed: bool) {
	BridgeHubKusama::execute_with(|| {
		type RuntimeEvent = <BridgeHubKusama as Chain>::RuntimeEvent;

		if expected_processed {
			assert_expected_events!(
				BridgeHubKusama,
				vec![
					// pay for bridge fees
					RuntimeEvent::Balances(pallet_balances::Event::Burned { .. }) => {},
					// message exported
					RuntimeEvent::BridgePolkadotMessages(
						pallet_bridge_messages::Event::MessageAccepted { .. }
					) => {},
					// message processed successfully
					RuntimeEvent::MessageQueue(
						pallet_message_queue::Event::Processed { success: true, .. }
					) => {},
				]
			);
		} else {
			assert_expected_events!(
				BridgeHubKusama,
				vec![
					RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed {
						success: false,
						..
					}) => {},
				]
			);
		}
	});
}

pub(crate) fn assert_bridge_hub_polkadot_message_received() {
	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			BridgeHubPolkadot,
			vec![
				// message sent to destination
				RuntimeEvent::XcmpQueue(
					cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }
				) => {},
			]
		);
	})
}

#[test]
fn send_ksm_from_asset_hub_kusama_to_ethereum() {
	let initial_fund: u128 = 20_000_000_000_000_000;
	let initial_liquidity: u128 = initial_fund / 2;
	let amount: u128 = initial_fund;
	let ksm_fee_amount: u128 = initial_liquidity / 2;
	let dot_amount_to_swap: u128 = initial_liquidity / 10;
	let dot_fee_amount: u128 = dot_amount_to_swap / 10;

	let ether_fee_amount: u128 = MIN_ETHER_BALANCE * 2;

	let sender = AssetHubKusamaSender::get();
	let ksm_at_asset_hub_kusama = Location::parent();
	let bridged_ksm_at_asset_hub_polkadot = bridged_ksm_at_ah_polkadot();

	set_bridge_hub_ethereum_base_fee();
	create_foreign_on_ah_polkadot(
		bridged_ksm_at_asset_hub_polkadot.clone(),
		true,
		vec![(asset_hub_kusama_location(), false).into()],
		vec![],
	);
	set_up_pool_with_dot_on_ah_polkadot(
		bridged_ksm_at_asset_hub_polkadot.clone(),
		true,
		initial_fund,
		initial_liquidity,
	);
	let previous_owner = ethereum_sovereign();
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::start_destroy(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(previous_owner),
			eth_location()
		));
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::finish_destroy(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadot::account_id_of(
				ALICE
			)),
			eth_location()
		));
	});
	create_foreign_on_ah_polkadot(
		eth_location(),
		true,
		vec![(eth_location(), false).into()],
		vec![],
	);
	set_up_pool_with_dot_on_ah_polkadot(eth_location(), true, initial_fund, initial_liquidity);
	BridgeHubKusama::fund_para_sovereign(AssetHubKusama::para_id(), initial_fund);
	AssetHubKusama::fund_accounts(vec![(AssetHubKusamaSender::get(), initial_fund)]);
	fund_on_bh();
	register_ksm_as_native_polkadot_asset_on_snowbridge();

	// set XCM versions
	AssetHubKusama::force_xcm_version(asset_hub_polkadot_location(), XCM_VERSION);
	BridgeHubKusama::force_xcm_version(bridge_hub_polkadot_location(), XCM_VERSION);

	// send ROCs, use them for fees
	let local_fee_asset: Asset = (ksm_at_asset_hub_kusama.clone(), ksm_fee_amount).into();
	let remote_fee_on_polkadot: Asset = (ksm_at_asset_hub_kusama.clone(), ksm_fee_amount).into();
	let assets: Assets = (ksm_at_asset_hub_kusama.clone(), amount).into();
	let reserved_asset_on_polkadot: Asset =
		(ksm_at_asset_hub_kusama.clone(), amount - ksm_fee_amount * 2).into();
	let reserved_asset_on_polkadot_reanchored: Asset =
		(bridged_ksm_at_asset_hub_polkadot.clone(), (amount - ksm_fee_amount * 2) / 2).into();

	let xcm = VersionedXcm::from(Xcm(vec![
		WithdrawAsset(assets.clone()),
		PayFees { asset: local_fee_asset.clone() },
		InitiateTransfer {
			destination: asset_hub_polkadot_location(),
			remote_fees: Some(AssetTransferFilter::ReserveDeposit(Definite(
				remote_fee_on_polkadot.clone().into(),
			))),
			preserve_origin: true,
			assets: BoundedVec::truncate_from(vec![AssetTransferFilter::ReserveDeposit(Definite(
				reserved_asset_on_polkadot.clone().into(),
			))]),
			remote_xcm: Xcm(vec![
				// swap from ksm to dot
				ExchangeAsset {
					give: Definite(reserved_asset_on_polkadot_reanchored.clone().into()),
					want: (Parent, dot_amount_to_swap).into(),
					maximal: true,
				},
				// swap some dot to ether
				ExchangeAsset {
					give: Definite((Parent, dot_amount_to_swap).into()),
					want: (eth_location(), ether_fee_amount).into(),
					maximal: true,
				},
				PayFees { asset: (Parent, dot_fee_amount).into() },
				InitiateTransfer {
					destination: eth_location(),
					remote_fees: Some(AssetTransferFilter::ReserveWithdraw(Definite(
						Asset { id: AssetId(eth_location()), fun: Fungible(ether_fee_amount) }
							.into(),
					))),
					preserve_origin: true,
					assets: BoundedVec::truncate_from(vec![AssetTransferFilter::ReserveDeposit(
						Definite(reserved_asset_on_polkadot_reanchored.clone().into()),
					)]),
					remote_xcm: Xcm(vec![DepositAsset {
						assets: Wild(All),
						beneficiary: beneficiary(),
					}]),
				},
			]),
		},
	]));

	AssetHubKusama::execute_with(|| {
		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::PolkadotXcm::execute(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(sender),
			bx!(xcm),
			Weight::from(EXECUTION_WEIGHT),
		));
	});

	assert_bridge_hub_kusama_message_accepted(true);
	assert_bridge_hub_polkadot_message_received();

	// verify expected events on final destination
	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;

		// Check that the Ethereum message was queue in the Outbound Queue
		assert_expected_events!(
			BridgeHubPolkadot,
			vec![RuntimeEvent::EthereumOutboundQueueV2(snowbridge_pallet_outbound_queue_v2::Event::MessageQueued{ .. }) => {},]
		);
	});
}

#[test]
fn register_kusama_asset_on_ethereum_from_rah() {
	const XCM_FEE: u128 = 4_000_000_000_000;
	let sa_of_kah_on_pah =
		AssetHubPolkadot::sovereign_account_of_parachain_on_other_global_consensus(
			Kusama,
			AssetHubKusama::para_id(),
		);

	// Kusama Asset Hub asset when bridged to Polkadot Asset Hub.
	let bridged_asset_at_pah = Location::new(
		2,
		[
			GlobalConsensus(Kusama),
			Parachain(AssetHubKusama::para_id().into()),
			PalletInstance(ASSETS_PALLET_ID),
			GeneralIndex(ASSET_ID.into()),
		],
	);

	AssetHubPolkadot::force_create_foreign_asset(
		bridged_asset_at_pah.clone(),
		sa_of_kah_on_pah.clone(),
		true,
		ASSET_MIN_BALANCE,
		vec![],
	);

	let fee_asset =
		Asset { id: AssetId(eth_location()), fun: Fungible(REMOTE_FEE_AMOUNT_IN_ETHER) };

	let call =
		EthereumSystemFrontend::EthereumSystemFrontend(EthereumSystemFrontendCall::RegisterToken {
			asset_id: Box::new(VersionedLocation::from(bridged_asset_at_pah.clone())),
			metadata: Default::default(),
			fee_asset,
		})
		.encode();

	let origin_kind = OriginKind::Xcm;
	let fee_amount = XCM_FEE;
	let fees = (Parent, fee_amount).into();

	let xcm = xcm_transact_paid_execution(call.into(), origin_kind, fees, sa_of_kah_on_pah.clone());

	// SA-of-RAH-on-WAH needs to have balance to pay for fees and asset creation deposit
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint_into(
			eth_location(),
			&sa_of_kah_on_pah,
			INITIAL_FUND,
		));
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::Balances::force_set_balance(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::root(),
			sa_of_kah_on_pah.into(),
			INITIAL_FUND
		));
	});

	let destination = asset_hub_polkadot_location();

	// fund the RAH's SA on RBH for paying bridge delivery fees
	BridgeHubKusama::fund_para_sovereign(AssetHubKusama::para_id(), 10_000_000_000_000u128);

	// set XCM versions
	AssetHubKusama::force_xcm_version(destination.clone(), XCM_VERSION);
	BridgeHubKusama::force_xcm_version(bridge_hub_polkadot_location(), XCM_VERSION);

	let root_origin = <AssetHubKusama as Chain>::RuntimeOrigin::root();
	AssetHubKusama::execute_with(|| {
		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::PolkadotXcm::send(
			root_origin,
			bx!(destination.into()),
			bx!(xcm),
		));

		AssetHubKusama::assert_xcm_pallet_sent();
	});

	assert_bridge_hub_kusama_message_accepted(true);
	assert_bridge_hub_polkadot_message_received();
	AssetHubPolkadot::execute_with(|| {
		AssetHubPolkadot::assert_xcmp_queue_success(None);
	});
	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;

		// Check that the Ethereum message was queue in the Outbound Queue
		assert_expected_events!(
			BridgeHubPolkadot,
			vec![RuntimeEvent::EthereumOutboundQueueV2(snowbridge_pallet_outbound_queue_v2::Event::MessageQueued{ .. }) => {},]
		);
	});
}
