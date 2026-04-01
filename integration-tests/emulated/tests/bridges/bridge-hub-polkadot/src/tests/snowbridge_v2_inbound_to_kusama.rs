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
		assert_bridge_hub_kusama_message_received, assert_bridge_hub_polkadot_message_accepted,
		asset_hub_kusama_location, asset_hub_polkadot_global_location,
		create_foreign_on_ah_polkadot, snowbridge_common::*,
	},
	*,
};
use asset_hub_polkadot_runtime::ForeignAssets;
use bridge_hub_polkadot_runtime::{
	bridge_common_config::BridgeReward, bridge_to_ethereum_config::EthereumGatewayAddress,
	EthereumInboundQueueV2,
};
use codec::Encode;
use frame_support::BoundedVec;
use snowbridge_core::TokenIdOf;
use snowbridge_inbound_queue_primitives::v2::{
	EthereumAsset::{ForeignTokenERC20, NativeTokenERC20},
	Message, XcmPayload,
};
use sp_core::{H160, H256};
use xcm::opaque::latest::AssetTransferFilter::{ReserveDeposit, ReserveWithdraw};
use xcm_executor::traits::ConvertLocation;

#[test]
fn send_token_to_kusama_v2() {
	let relayer_account = BridgeHubPolkadotSender::get();
	let relayer_reward = 1_500_000_000_000u128;

	let token: H160 = TOKEN_ID.into();
	let token_location = erc20_token_location(token);

	let beneficiary_acc_id: H256 = H256::random();
	let beneficiary_acc_bytes: [u8; 32] = beneficiary_acc_id.into();
	let beneficiary =
		Location::new(0, AccountId32 { network: None, id: beneficiary_acc_id.into() });

	let claimer_acc_id = H256::random();
	let claimer = AccountId32 { network: None, id: claimer_acc_id.into() };
	let claimer_bytes = claimer.encode();

	// set XCM versions
	BridgeHubPolkadot::force_xcm_version(asset_hub_polkadot_location(), XCM_VERSION);
	BridgeHubPolkadot::force_xcm_version(asset_hub_kusama_location(), XCM_VERSION);
	AssetHubPolkadot::force_xcm_version(asset_hub_kusama_location(), XCM_VERSION);

	// To pay fees on Kusama.
	let eth_fee_kusama_ah: Asset = (eth_location(), MIN_ETHER_BALANCE * 2).into();

	// To satisfy ED
	AssetHubKusama::fund_accounts(vec![(
		sp_runtime::AccountId32::from(beneficiary_acc_bytes),
		3_000_000_000_000,
	)]);
	BridgeHubPolkadot::fund_para_sovereign(AssetHubPolkadot::para_id(), INITIAL_FUND);

	// Register the token on AH Polkadot and Kusama
	let ethereum_sovereign = ethereum_sovereign();
	AssetHubKusama::execute_with(|| {
		type RuntimeOrigin = <AssetHubKusama as Chain>::RuntimeOrigin;

		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets::force_create(
			RuntimeOrigin::root(),
			token_location.clone(),
			ethereum_sovereign.clone().into(),
			true,
			1000,
		));

		assert!(<AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets::asset_exists(
			token_location.clone(),
		));
	});
	AssetHubKusama::set_foreign_asset_reserves(
		token_location.clone(),
		ethereum_sovereign.clone(),
		vec![(asset_hub_polkadot_global_location(), false).into()],
	);

	// To satisfy ED
	let sov_ahw_on_ahr = AssetHubPolkadot::sovereign_account_of_parachain_on_other_global_consensus(
		Kusama,
		AssetHubKusama::para_id(),
	);
	AssetHubPolkadot::fund_accounts(vec![(sov_ahw_on_ahr.clone(), INITIAL_FUND)]);

	register_foreign_asset(token_location.clone(), ethereum_sovereign, false);
	set_up_eth_and_dot_pool_on_polkadot_asset_hub();
	set_up_eth_and_ksm_pool_on_kusama_asset_hub();

	let token_transfer_value = TOKEN_AMOUNT;

	let assets = vec![
		// the token being transferred
		NativeTokenERC20 { token_id: token, value: token_transfer_value },
	];

	let token_asset_ah: Asset = (token_location.clone(), token_transfer_value).into();
	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;
		let instructions = vec![
			// Send message to Kusama AH
			InitiateTransfer {
				// Kusama
				destination: Location::new(2, [GlobalConsensus(Kusama), Parachain(1000u32)]),
				remote_fees: Some(ReserveDeposit(Definite(vec![eth_fee_kusama_ah.clone()].into()))),
				preserve_origin: false,
				assets: BoundedVec::truncate_from(vec![ReserveDeposit(Definite(
					vec![token_asset_ah.clone()].into(),
				))]),
				remote_xcm: vec![
					// Refund unspent fees
					RefundSurplus,
					// Deposit assets to beneficiary.
					DepositAsset { assets: Wild(AllCounted(3)), beneficiary: beneficiary.clone() },
					SetTopic(H256::random().into()),
				]
				.into(),
			},
			RefundSurplus,
			DepositAsset {
				assets: Wild(AllOf { id: AssetId(eth_location()), fun: WildFungibility::Fungible }),
				beneficiary,
			},
		];
		let xcm: Xcm<()> = instructions.into();
		let versioned_message_xcm = VersionedXcm::V5(xcm);
		let origin = EthereumGatewayAddress::get();

		let message = Message {
			gateway: origin,
			nonce: 1,
			origin,
			assets,
			xcm: XcmPayload::Raw(versioned_message_xcm.encode()),
			claimer: Some(claimer_bytes),
			value: TOKEN_AMOUNT,
			execution_fee: MIN_ETHER_BALANCE * 3,
			relayer_fee: relayer_reward,
		};

		EthereumInboundQueueV2::process_message(relayer_account.clone(), message).unwrap();

		assert_expected_events!(
			BridgeHubPolkadot,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
				// Check that the relayer reward was registered.
				RuntimeEvent::BridgeRelayers(pallet_bridge_relayers::Event::RewardRegistered { relayer, reward_kind, reward_balance }) => {
					relayer: *relayer == relayer_account,
					reward_kind: *reward_kind == BridgeReward::Snowbridge,
					reward_balance: *reward_balance == relayer_reward,
				},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		// Check that the assets were issued on AssetHub
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				// Message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	ensure_no_assets_trapped_on_pah();
	assert_bridge_hub_polkadot_message_accepted(true);
	assert_bridge_hub_kusama_message_received();

	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;

		assert_expected_events!(
			AssetHubKusama,
			vec![
				// Message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
				// Token was issued to beneficiary
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == token_location,
					owner: *owner == beneficiary_acc_bytes.into(),
				},
				// Leftover fees was deposited to beneficiary
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == eth_location(),
					owner: *owner == beneficiary_acc_bytes.into(),
				},
			]
		);

		// Beneficiary received the token transfer value
		assert_eq!(
			ForeignAssets::balance(token_location, AccountId::from(beneficiary_acc_bytes)),
			token_transfer_value
		);
	});

	ensure_no_assets_trapped_on_pah();
}

#[test]
fn send_ether_to_kusama_v2() {
	let relayer_account = BridgeHubPolkadotSender::get();
	let relayer_reward = 1_500_000_000_000u128;

	let beneficiary_acc_id: H256 = H256::random();
	let beneficiary_acc_bytes: [u8; 32] = beneficiary_acc_id.into();
	let beneficiary =
		Location::new(0, AccountId32 { network: None, id: beneficiary_acc_id.into() });

	let claimer_acc_id = H256::random();
	let claimer = AccountId32 { network: None, id: claimer_acc_id.into() };
	let claimer_bytes = claimer.encode();

	// set XCM versions
	BridgeHubPolkadot::force_xcm_version(asset_hub_polkadot_location(), XCM_VERSION);
	BridgeHubPolkadot::force_xcm_version(asset_hub_kusama_location(), XCM_VERSION);
	AssetHubPolkadot::force_xcm_version(asset_hub_kusama_location(), XCM_VERSION);

	// To pay fees on Kusama.
	let eth_fee_kusama_ah: Asset = (eth_location(), MIN_ETHER_BALANCE).into();
	let ether_asset_ah: Asset = (eth_location(), TOKEN_AMOUNT).into();

	BridgeHubPolkadot::fund_para_sovereign(AssetHubPolkadot::para_id(), INITIAL_FUND);

	set_up_eth_and_dot_pool_on_polkadot_asset_hub();
	set_up_eth_and_ksm_pool_on_kusama_asset_hub();
	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;
		let instructions = vec![
			// Send message to Kusama AH
			InitiateTransfer {
				// Kusama
				destination: Location::new(2, [GlobalConsensus(Kusama), Parachain(1000u32)]),
				remote_fees: Some(ReserveDeposit(Definite(vec![eth_fee_kusama_ah.clone()].into()))),
				preserve_origin: false,
				assets: BoundedVec::truncate_from(vec![ReserveDeposit(Definite(
					vec![ether_asset_ah.clone()].into(),
				))]),
				remote_xcm: vec![
					// Refund unspent fees
					RefundSurplus,
					// Deposit assets to beneficiary.
					DepositAsset { assets: Wild(AllCounted(3)), beneficiary: beneficiary.clone() },
					SetTopic(H256::random().into()),
				]
				.into(),
			},
			RefundSurplus,
			DepositAsset {
				assets: Wild(AllOf { id: AssetId(eth_location()), fun: WildFungibility::Fungible }),
				beneficiary,
			},
		];
		let xcm: Xcm<()> = instructions.into();
		let versioned_message_xcm = VersionedXcm::V5(xcm);
		let origin = EthereumGatewayAddress::get();

		let message = Message {
			gateway: origin,
			nonce: 1,
			origin,
			assets: vec![],
			xcm: XcmPayload::Raw(versioned_message_xcm.encode()),
			claimer: Some(claimer_bytes),
			value: TOKEN_AMOUNT,
			execution_fee: MIN_ETHER_BALANCE * 2,
			relayer_fee: relayer_reward,
		};

		EthereumInboundQueueV2::process_message(relayer_account.clone(), message).unwrap();

		assert_expected_events!(
			BridgeHubPolkadot,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
				// Check that the relayer reward was registered.
				RuntimeEvent::BridgeRelayers(pallet_bridge_relayers::Event::RewardRegistered { relayer, reward_kind, reward_balance }) => {
					relayer: *relayer == relayer_account,
					reward_kind: *reward_kind == BridgeReward::Snowbridge,
					reward_balance: *reward_balance == relayer_reward,
				},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		// Check that the assets were issued on AssetHub
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				// Message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	ensure_no_assets_trapped_on_pah();
	assert_bridge_hub_polkadot_message_accepted(true);
	assert_bridge_hub_kusama_message_received();

	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;

		assert_expected_events!(
			AssetHubKusama,
			vec![
				// Message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
				// Ether was deposited to beneficiary
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == eth_location(),
					owner: *owner == beneficiary_acc_bytes.into(),
				},
			]
		);
	});

	ensure_no_assets_trapped_on_kah();
}

#[test]
fn send_ksm_from_ethereum_to_kusama() {
	let initial_fund: u128 = 200_000_000_000_000;
	let relayer_account = BridgeHubPolkadotSender::get();
	let relayer_reward = 1_500_000_000_000u128;

	let claimer = AccountId32 { network: None, id: H256::random().into() };
	let claimer_bytes = claimer.encode();
	let beneficiary =
		Location::new(0, AccountId32 { network: None, id: AssetHubKusamaReceiver::get().into() });

	BridgeHubPolkadot::fund_para_sovereign(AssetHubPolkadot::para_id(), INITIAL_FUND);

	let ethereum_sovereign: AccountId = ethereum_sovereign();
	let bridged_roc_at_asset_hub_polkadot = bridged_ksm_at_ah_polkadot();
	create_foreign_on_ah_polkadot(
		bridged_roc_at_asset_hub_polkadot.clone(),
		true,
		vec![(asset_hub_kusama_location(), false).into()],
		vec![],
	);

	BridgeHubKusama::fund_para_sovereign(AssetHubKusama::para_id(), initial_fund);
	AssetHubKusama::fund_accounts(vec![(AssetHubKusamaSender::get(), initial_fund)]);
	register_ksm_as_native_polkadot_asset_on_snowbridge();

	set_up_eth_and_dot_pool_on_polkadot_asset_hub();
	set_up_eth_and_ksm_pool_on_kusama_asset_hub();

	// set XCM versions
	BridgeHubPolkadot::force_xcm_version(asset_hub_polkadot_location(), XCM_VERSION);
	BridgeHubPolkadot::force_xcm_version(asset_hub_kusama_location(), XCM_VERSION);
	AssetHubPolkadot::force_xcm_version(asset_hub_kusama_location(), XCM_VERSION);

	let eth_fee_kusama_ah: Asset = (eth_location(), MIN_ETHER_BALANCE).into();

	let ksm = Location::new(1, [GlobalConsensus(Kusama)]);
	let token_id = TokenIdOf::convert_location(&ksm).unwrap();

	let ksm_reachored: Asset =
		(Location::new(2, [GlobalConsensus(NetworkId::Kusama)]), TOKEN_AMOUNT).into();

	let assets = vec![
		// the token being transferred
		ForeignTokenERC20 { token_id, value: TOKEN_AMOUNT },
	];

	AssetHubPolkadot::execute_with(|| {
		// Mint the asset into the bridge sovereign account, to mimic locked funds
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotAssetOwner::get()),
			bridged_roc_at_asset_hub_polkadot.clone(),
			ethereum_sovereign.clone().into(),
			TOKEN_AMOUNT,
		));
	});

	// fund the AHP's SA on AHK with the KSM tokens held in reserve
	let sov_ahw_on_ahr = AssetHubKusama::sovereign_account_of_parachain_on_other_global_consensus(
		NetworkId::Polkadot,
		AssetHubPolkadot::para_id(),
	);
	AssetHubKusama::fund_accounts(vec![(sov_ahw_on_ahr.clone(), INITIAL_FUND)]);

	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;
		let instructions = vec![
			// Send message to Kusama AH
			InitiateTransfer {
				// Kusama
				destination: Location::new(2, [GlobalConsensus(Kusama), Parachain(1000u32)]),
				remote_fees: Some(ReserveDeposit(Definite(vec![eth_fee_kusama_ah.clone()].into()))),
				preserve_origin: false,
				assets: BoundedVec::truncate_from(vec![ReserveWithdraw(Definite(
					vec![ksm_reachored.clone()].into(),
				))]),
				remote_xcm: vec![
					// Refund unspent fees
					RefundSurplus,
					// Deposit assets and leftover fees to beneficiary.
					DepositAsset { assets: Wild(AllCounted(2)), beneficiary: beneficiary.clone() },
					SetTopic(H256::random().into()),
				]
				.into(),
			},
			RefundSurplus,
			DepositAsset {
				assets: Wild(AllOf { id: AssetId(eth_location()), fun: WildFungibility::Fungible }),
				beneficiary,
			},
		];
		let xcm: Xcm<()> = instructions.into();
		let versioned_message_xcm = VersionedXcm::V5(xcm);
		let origin = EthereumGatewayAddress::get();

		let message = Message {
			gateway: origin,
			nonce: 1,
			origin,
			assets,
			xcm: XcmPayload::Raw(versioned_message_xcm.encode()),
			claimer: Some(claimer_bytes),
			value: TOKEN_AMOUNT,
			execution_fee: MIN_ETHER_BALANCE * 2,
			relayer_fee: relayer_reward,
		};

		EthereumInboundQueueV2::process_message(relayer_account.clone(), message).unwrap();

		assert_expected_events!(
			BridgeHubPolkadot,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
				// Check that the relayer reward was registered.
				RuntimeEvent::BridgeRelayers(pallet_bridge_relayers::Event::RewardRegistered { relayer, reward_kind, reward_balance }) => {
					relayer: *relayer == relayer_account,
					reward_kind: *reward_kind == BridgeReward::Snowbridge,
					reward_balance: *reward_balance == relayer_reward,
				},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				// Message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	ensure_no_assets_trapped_on_pah();
	assert_bridge_hub_polkadot_message_accepted(true);
	assert_bridge_hub_kusama_message_received();

	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;

		assert_expected_events!(
			AssetHubKusama,
			vec![
				// ROC is withdrawn from AHW's SA on AHR
				RuntimeEvent::Balances(
					pallet_balances::Event::Burned { who, amount }
				) => {
					who: *who == sov_ahw_on_ahr,
					amount: *amount == TOKEN_AMOUNT,
				},
				// ROCs deposited to beneficiary
				RuntimeEvent::Balances(pallet_balances::Event::Minted { who, .. }) => {
					who: *who == AssetHubKusamaReceiver::get(),
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	ensure_no_assets_trapped_on_pah();
}
