// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

use std::i8::MIN;
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
	tests::snowbridge_common::{
		erc20_token_location, eth_location,
		register_foreign_asset, set_up_eth_and_dot_pool, set_up_eth_and_dot_pool_on_penpal,
		snowbridge_sovereign, weth_location,
	},
	*,
};
use asset_hub_polkadot_runtime::ForeignAssets;
use bp_asset_hub_polkadot;
use bp_bridge_hub_polkadot::snowbridge::CreateAssetCall;
use bridge_hub_polkadot_runtime::{
	bridge_common_config::BridgeReward, bridge_to_ethereum_config::EthereumGatewayAddress,
	EthereumInboundQueueV2,
};
use codec::Encode;
use frame_support::assert_ok;
use pallet_bridge_relayers;
// Polkadot genesis hash
const POLKADOT_GENESIS_HASH: [u8; 32] =
	hex!("91b171bb158e2d3848fa23a9f1c25182fb8e20313b2c1eb49219da7a70ce90c3");
use emulated_integration_tests_common::RESERVABLE_ASSET_ID;
use frame_support::{traits::fungibles::Mutate, BoundedVec};
use hex_literal::hex;
use polkadot_system_emulated_network::penpal_emulated_chain::PARA_ID_B;
use snowbridge_core::{AssetMetadata, TokenIdOf};
use snowbridge_inbound_queue_primitives::v2::{
	EthereumAsset::{ForeignTokenERC20, NativeTokenERC20},
	Message, Network, XcmPayload,
};
use sp_core::{H160, H256};
use sp_io::hashing::blake2_256;
use sp_runtime::MultiAddress;
use xcm::opaque::latest::AssetTransferFilter::ReserveDeposit;
use xcm_executor::traits::ConvertLocation;
use crate::tests::snowbridge_common::TOKEN_AMOUNT;

/// Calculates the XCM prologue fee for sending an XCM to AH.
const INITIAL_FUND: u128 = 5_000_000_000_000;

/// An ERC-20 token to be registered and sent.
const TOKEN_ID: [u8; 20] = hex!("8daebade922df735c38c80c7ebd708af50815faa");

#[test]
fn register_token_v2() {
	let relayer_account = BridgeHubPolkadotSender::get();
	let relayer_reward = 1_500_000_000_000u128;
	let receiver = AssetHubPolkadotReceiver::get();
	let bridge_owner = snowbridge_sovereign();
	BridgeHubPolkadot::fund_accounts(vec![(relayer_account.clone(), INITIAL_FUND)]);
	AssetHubPolkadot::fund_accounts(vec![(bridge_owner.clone(), INITIAL_FUND)]);

	set_up_eth_and_dot_pool();

	let claimer = Location::new(0, AccountId32 { network: None, id: receiver.clone().into() });
	let claimer_bytes = claimer.encode();

	let token: H160 = TOKEN_ID.into();

	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;
		let origin = EthereumGatewayAddress::get();

		let message = Message {
			gateway: origin,
			nonce: 1,
			origin,
			assets: vec![],
			xcm: XcmPayload::CreateAsset { token, network: Network::Polkadot },
			claimer: Some(claimer_bytes),
			// Used to pay the asset creation deposit.
			value: 9_000_000_000_000u128,
			execution_fee: 1_500_000_000_000u128,
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
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
				// Check that the token was created as a foreign asset on AssetHub
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Created { asset_id, owner, .. }) => {
					asset_id: *asset_id == erc20_token_location(token),
					owner: *owner == bridge_owner,
				},
				// Check that excess fees were paid to the claimer
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == eth_location(),
					owner: *owner == receiver.clone().into(),
				},
			]
		);

		let events = AssetHubPolkadot::events();
		// Check that no assets were trapped
		assert!(
			!events.iter().any(|event| matches!(
				event,
				RuntimeEvent::PolkadotXcm(pallet_xcm::Event::AssetsTrapped { .. })
			)),
			"Assets were trapped, should not happen."
		);
	});
}

#[test]
fn send_token_v2() {
	let relayer_account = BridgeHubPolkadotSender::get();
	let relayer_reward = 1_500_000_000_000u128;

	let token: H160 = TOKEN_ID.into();
	let token_location = erc20_token_location(token);

	let receiver = AssetHubPolkadotReceiver::get();
	let claimer = Location::new(0, AccountId32 { network: None, id: receiver.clone().into() });
	let claimer_bytes = claimer.encode();

	let beneficiary_acc_id: H256 = H256::random();
	let beneficiary_acc_bytes: [u8; 32] = beneficiary_acc_id.into();
	let beneficiary =
		Location::new(0, AccountId32 { network: None, id: beneficiary_acc_id.into() });

	let token_transfer_value = 2_000_000_000_000u128;

	register_foreign_asset(token_location.clone());

	let snowbridge_sovereign = snowbridge_sovereign();

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint_into(
			eth_location().try_into().unwrap(),
			&snowbridge_sovereign,
			500_000_000_000_000,
		));
	});

	let assets = vec![
		// the token being transferred
		NativeTokenERC20 { token_id: token.into(), value: token_transfer_value },
	];

	set_up_eth_and_dot_pool();
	let topic_id = BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;
		let instructions = vec![
			RefundSurplus,
			DepositAsset {
				assets: Wild(AllOf {
					id: AssetId(token_location.clone()),
					fun: WildFungibility::Fungible,
				}),
				beneficiary,
			},
			DepositAsset {
				assets: Wild(AllOf { id: AssetId(eth_location()), fun: WildFungibility::Fungible }),
				beneficiary: claimer,
			},
		];
		let xcm: Xcm<()> = instructions.into();
		let versioned_message_xcm = VersionedXcm::V5(xcm);
		let origin = H160::random();

		let message = Message {
			gateway: EthereumGatewayAddress::get(),
			nonce: 1,
			origin,
			assets,
			xcm: XcmPayload::Raw(versioned_message_xcm.encode()),
			claimer: Some(claimer_bytes),
			value: TOKEN_AMOUNT.into(),
			execution_fee: 1_500_000_000_000u128,
			relayer_fee: relayer_reward,
		};

		EthereumInboundQueueV2::process_message(relayer_account.clone(), message.clone()).unwrap();

		let topic_id = blake2_256(&("SnowbridgeInboundQueueV2", message.nonce).encode());
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
				RuntimeEvent::EthereumInboundQueueV2(snowbridge_pallet_inbound_queue_v2::Event::MessageReceived { message_id, .. }) => {
					message_id: *message_id == topic_id,
				},
			]
		);
		topic_id
	});

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, id, .. }
				) => {
					id: *id == topic_id.into(),
				},
				// Check that the token was received and issued as a foreign asset on AssetHub
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == token_location,
					owner: *owner == beneficiary_acc_bytes.into(),
				},
				// Check that excess fees were paid to the claimer, which was set by the UX
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == eth_location(),
					owner: *owner == receiver.clone().into(),
				},
			]
		);

		// Beneficiary received the token transfer value
		assert_eq!(
			ForeignAssets::balance(token_location, AccountId::from(beneficiary_acc_bytes)),
			token_transfer_value
		);
		// Claimer received eth refund for fees paid
		assert!(ForeignAssets::balance(eth_location(), receiver) > 0);

		let events = AssetHubPolkadot::events();
		// Check that no assets were trapped
		assert!(
			!events.iter().any(|event| matches!(
				event,
				RuntimeEvent::PolkadotXcm(pallet_xcm::Event::AssetsTrapped { .. })
			)),
			"Assets were trapped, should not happen."
		);
	});
}

#[test]
fn send_weth_v2() {
	let relayer_account = BridgeHubPolkadotSender::get();
	let relayer_reward = 1_500_000_000_000u128;

	let beneficiary_acc_id: H256 = H256::random();
	let beneficiary_acc_bytes: [u8; 32] = beneficiary_acc_id.into();
	let beneficiary =
		Location::new(0, AccountId32 { network: None, id: beneficiary_acc_id.into() });

	let claimer_acc_id = H256::random();
	let claimer = Location::new(0, AccountId32 { network: None, id: claimer_acc_id.into() });
	let claimer_bytes = claimer.encode();

	let token_transfer_value = TOKEN_AMOUNT;

	let assets = vec![
		// the token being transferred
		NativeTokenERC20 { token_id: WETH.into(), value: token_transfer_value },
	];

	set_up_eth_and_dot_pool();
	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;
		let instructions = vec![
			RefundSurplus,
			DepositAsset { assets: Wild(AllCounted(2)), beneficiary: beneficiary.clone() },
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
			execution_fee: 1_500_000_000_000u128,
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
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
				// Check that the token was received and issued as a foreign asset on AssetHub
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == weth_location(),
					owner: *owner == beneficiary_acc_bytes.into(),
				},
				// Check that excess fees were paid to the beneficiary
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == eth_location(),
					owner: *owner == beneficiary_acc_bytes.into(),
				},
			]
		);

		// Beneficiary received the token transfer value
		assert_eq!(
			ForeignAssets::balance(weth_location(), AccountId::from(beneficiary_acc_bytes)),
			token_transfer_value
		);

		// Claimer received eth refund for fees paid
		assert!(ForeignAssets::balance(eth_location(), AccountId::from(beneficiary_acc_bytes)) > 0);

		let events = AssetHubPolkadot::events();
		// Check that no assets were trapped
		assert!(
			!events.iter().any(|event| matches!(
				event,
				RuntimeEvent::PolkadotXcm(pallet_xcm::Event::AssetsTrapped { .. })
			)),
			"Assets were trapped, should not happen."
		);
	});
}

#[test]
fn register_and_send_multiple_tokens_v2() {
	let relayer_account = BridgeHubPolkadotSender::get();
	let relayer_reward = 1_500_000_000_000u128;

	let token: H160 = TOKEN_ID.into();
	let token_location = erc20_token_location(token);

	let bridge_owner = snowbridge_sovereign();

	let beneficiary_acc_id: H256 = H256::random();
	let beneficiary_acc_bytes: [u8; 32] = beneficiary_acc_id.into();
	let beneficiary =
		Location::new(0, AccountId32 { network: None, id: beneficiary_acc_id.into() });

	// To satisfy ED
	AssetHubPolkadot::fund_accounts(vec![(
		sp_runtime::AccountId32::from(beneficiary_acc_bytes),
		3_000_000_000_000,
	)]);

	let claimer_acc_id = H256::random();
	let claimer = Location::new(0, AccountId32 { network: None, id: claimer_acc_id.into() });
	let claimer_bytes = claimer.encode();

	set_up_eth_and_dot_pool();

	let token_transfer_value = TOKEN_AMOUNT;
	let weth_transfer_value = TOKEN_AMOUNT;
	let eth_token_deposit = 9_000_000_000_000u128;

	let dot_asset = Location::new(1, Here);
	let dot_fee: xcm::prelude::Asset =
		(dot_asset, bp_asset_hub_polkadot::CreateForeignAssetDeposit::get()).into();

	// Used to pay the asset creation deposit.
	let eth_asset_value = eth_token_deposit + (MIN_ETHER_BALANCE * 100);
	let asset_deposit: xcm::prelude::Asset = (eth_location(), eth_token_deposit).into();

	let assets = vec![
		NativeTokenERC20 { token_id: WETH.into(), value: weth_transfer_value },
		NativeTokenERC20 { token_id: token.into(), value: token_transfer_value },
	];

	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;
		let instructions = vec![
			ExchangeAsset {
				give: asset_deposit.clone().into(),
				want: dot_fee.clone().into(),
				maximal: false,
			},
			DepositAsset { assets: dot_fee.into(), beneficiary: bridge_owner.clone().into() },
			// register new token
			Transact {
				origin_kind: OriginKind::Xcm,
				fallback_max_weight: None,
				call: (
					CreateAssetCall::get(),
					token_location.clone(),
					MultiAddress::<[u8; 32], ()>::Id(bridge_owner.clone().into()),
					1u128,
				)
					.encode()
					.into(),
			},
			ExpectTransactStatus(MaybeErrorCode::Success),
			// deposit new token, weth and leftover ether fees to beneficiary.
			RefundSurplus,
			DepositAsset { assets: Wild(AllCounted(3)), beneficiary: beneficiary.clone() },
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
			value: eth_asset_value,
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
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
				// Check that the token was created as a foreign asset on AssetHub
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Created { asset_id, owner, .. }) => {
					asset_id: *asset_id == token_location.clone(),
					owner: *owner == bridge_owner.clone().into(),
				},
				// Check that the token was received and issued as a foreign asset on AssetHub
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == token_location,
					owner: *owner == beneficiary_acc_bytes.into(),
				},
				// Check that excess fees were paid to the beneficiary
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


		// Beneficiary received the weth transfer value
		assert!(
			ForeignAssets::balance(weth_location(), AccountId::from(beneficiary_acc_bytes)) >=
				weth_transfer_value
		);

		let events = AssetHubPolkadot::events();
		// Check that no assets were trapped
		assert!(
			!events.iter().any(|event| matches!(
				event,
				RuntimeEvent::PolkadotXcm(pallet_xcm::Event::AssetsTrapped { .. })
			)),
			"Assets were trapped, should not happen."
		);

		// Beneficiary received eth refund for fees paid
		assert!(ForeignAssets::balance(eth_location(), AccountId::from(beneficiary_acc_bytes)) > 0);
	});
}

#[test]
fn send_token_to_penpal_v2() {
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

	// To pay fees on Penpal.
	let eth_fee_penpal_ah: xcm::prelude::Asset = (eth_location(), 3_000_000_000_000u128).into();

	let snowbridge_sovereign = snowbridge_sovereign();

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint_into(
			eth_location().try_into().unwrap(),
			&snowbridge_sovereign,
			500_000_000_000_000,
		));
	});

	// To satisfy ED
	PenpalB::fund_accounts(vec![(
		sp_runtime::AccountId32::from(beneficiary_acc_bytes),
		3_000_000_000_000,
	)]);

	PenpalB::execute_with(|| {
		type RuntimeOrigin = <PenpalB as Chain>::RuntimeOrigin;

		// Register token on Penpal
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::force_create(
			RuntimeOrigin::root(),
			token_location.clone().try_into().unwrap(),
			snowbridge_sovereign.clone().into(),
			true,
			1000,
		));

		assert!(<PenpalB as PenpalBPallet>::ForeignAssets::asset_exists(
			token_location.clone().try_into().unwrap(),
		));

		// Register eth on Penpal
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::force_create(
			RuntimeOrigin::root(),
			eth_location().try_into().unwrap(),
			snowbridge_sovereign.clone().into(),
			true,
			1000,
		));

		assert!(<PenpalB as PenpalBPallet>::ForeignAssets::asset_exists(
			eth_location().try_into().unwrap(),
		));

		assert_ok!(<PenpalB as Chain>::System::set_storage(
			<PenpalB as Chain>::RuntimeOrigin::root(),
			vec![(
				PenpalCustomizableAssetFromSystemAssetHub::key().to_vec(),
				Location::new(
					2,
					[GlobalConsensus(Ethereum { chain_id: crate::tests::snowbridge::CHAIN_ID })]
				)
				.encode(),
			)],
		));

		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			eth_location().try_into().unwrap(),
			&snowbridge_sovereign,
			500_000_000_000_000,
		));
	});

	set_up_eth_and_dot_pool();
	set_up_eth_and_dot_pool_on_penpal();

	let token_transfer_value = 2_000_000_000_000u128;

	let assets = vec![
		// the token being transferred
		NativeTokenERC20 { token_id: token.into(), value: token_transfer_value },
	];

	let token_asset_ah: xcm::prelude::Asset = (token_location.clone(), token_transfer_value).into();
	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;
		let instructions = vec![
			// Send message to Penpal
			InitiateTransfer {
				// Penpal
				destination: Location::new(1, [Parachain(PARA_ID_B)]),
				remote_fees: Some(ReserveDeposit(Definite(vec![eth_fee_penpal_ah.clone()].into()))),
				preserve_origin: true,
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
			value: 3_500_000_000_000u128,
			execution_fee: 1_500_000_000_000u128,
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

	let penpal_sov_on_ah = AssetHubPolkadot::sovereign_account_id_of(Location::new(
		1,
		[Parachain(PenpalB::para_id().into())],
	));

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
				// Ether was issued to beneficiary
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == eth_location(),
					owner: *owner == penpal_sov_on_ah,
				},
				// Token was issued to beneficiary
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == token_location,
					owner: *owner == penpal_sov_on_ah,
				},
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);

		let events = AssetHubPolkadot::events();
		// Check that no assets were trapped
		assert!(
			!events.iter().any(|event| matches!(
				event,
				RuntimeEvent::PolkadotXcm(pallet_xcm::Event::AssetsTrapped { .. })
			)),
			"Assets were trapped, should not happen."
		);
	});

	PenpalB::execute_with(|| {
		type RuntimeEvent = <PenpalB as Chain>::RuntimeEvent;

		assert_expected_events!(
			PenpalB,
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

		let events = PenpalB::events();
		// Check that no assets were trapped
		assert!(
			!events.iter().any(|event| matches!(
				event,
				RuntimeEvent::PolkadotXcm(pallet_xcm::Event::AssetsTrapped { .. })
			)),
			"Assets were trapped on Penpal, should not happen."
		);
	});
}

#[test]
fn send_foreign_erc20_token_back_to_polkadot() {
	let relayer_account = BridgeHubPolkadotSender::get();
	let relayer_reward = 1_500_000_000_000u128;

	let claimer = AccountId32 { network: None, id: H256::random().into() };
	let claimer_bytes = claimer.encode();
	let beneficiary =
		Location::new(0, AccountId32 { network: None, id: AssetHubPolkadotReceiver::get().into() });

	let asset_id: Location =
		[PalletInstance(ASSETS_PALLET_ID), GeneralIndex(RESERVABLE_ASSET_ID.into())].into();

	let asset_id_in_bh: Location = Location::new(
		1,
		[
			Parachain(AssetHubPolkadot::para_id().into()),
			PalletInstance(ASSETS_PALLET_ID),
			GeneralIndex(RESERVABLE_ASSET_ID.into()),
		],
	);

	let asset_id_after_reanchored = Location::new(
		1,
		[
			GlobalConsensus(ByGenesis(POLKADOT_GENESIS_HASH)),
			Parachain(AssetHubPolkadot::para_id().into()),
		],
	)
	.appended_with(asset_id.clone().interior)
	.unwrap();

	set_up_eth_and_dot_pool();
	// Register token
	BridgeHubPolkadot::execute_with(|| {
		type RuntimeOrigin = <BridgeHubPolkadot as Chain>::RuntimeOrigin;

		assert_ok!(<BridgeHubPolkadot as BridgeHubPolkadotPallet>::EthereumSystem::register_token(
			RuntimeOrigin::root(),
			Box::new(VersionedLocation::from(asset_id_in_bh.clone())),
			AssetMetadata {
				name: "ah_asset".as_bytes().to_vec().try_into().unwrap(),
				symbol: "ah_asset".as_bytes().to_vec().try_into().unwrap(),
				decimals: 12,
			},
		));
	});

	let ethereum_sovereign: AccountId = snowbridge_sovereign();

	AssetHubPolkadot::fund_accounts(vec![(ethereum_sovereign.clone(), INITIAL_FUND)]);

	// Mint the asset into the bridge sovereign account, to mimic locked funds
	AssetHubPolkadot::mint_asset(
		<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotAssetOwner::get()),
		RESERVABLE_ASSET_ID,
		ethereum_sovereign.clone(),
		TOKEN_AMOUNT,
	);

	let token_id = TokenIdOf::convert_location(&asset_id_after_reanchored).unwrap();

	let assets = vec![
		// the token being transferred
		ForeignTokenERC20 { token_id: token_id.into(), value: TOKEN_AMOUNT },
	];

	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;
		let instructions =
			vec![RefundSurplus, DepositAsset { assets: Wild(AllCounted(2)), beneficiary }];
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
			value: 1_500_000_000_000u128,
			execution_fee: 3_500_000_000_000u128,
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
			vec![RuntimeEvent::Assets(pallet_assets::Event::Burned{..}) => {},]
		);

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				// Message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
				// Check that the native token burnt from some reserved account
				RuntimeEvent::Assets(pallet_assets::Event::Burned { owner, .. }) => {
					owner: *owner == ethereum_sovereign.clone().into(),
				},
				// Check that the token was minted to beneficiary
				RuntimeEvent::Assets(pallet_assets::Event::Issued { owner, .. }) => {
					owner: *owner == AssetHubPolkadotReceiver::get(),
				},
			]
		);

		let events = AssetHubPolkadot::events();
		// Check that no assets were trapped
		assert!(
			!events.iter().any(|event| matches!(
				event,
				RuntimeEvent::PolkadotXcm(pallet_xcm::Event::AssetsTrapped { .. })
			)),
			"Assets were trapped, should not happen."
		);
	});
}

#[test]
fn invalid_xcm_traps_funds_on_ah() {
	let relayer_account = BridgeHubPolkadotSender::get();
	let relayer_reward = 1_500_000_000_000u128;

	let token: H160 = TOKEN_ID.into();
	let claimer = AccountId32 { network: None, id: H256::random().into() };
	let claimer_bytes = claimer.encode();
	let beneficiary_acc_bytes: [u8; 32] = H256::random().into();

	AssetHubPolkadot::fund_accounts(vec![(
		sp_runtime::AccountId32::from(beneficiary_acc_bytes),
		3_000_000_000_000,
	)]);

	set_up_eth_and_dot_pool();

	let assets = vec![
		// to transfer assets
		NativeTokenERC20 { token_id: WETH.into(), value: 2_800_000_000_000u128 },
		// the token being transferred
		NativeTokenERC20 { token_id: token.into(), value: 2_000_000_000_000u128 },
	];

	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;
		// invalid xcm
		let instructions = hex!("02806c072d50e2c7cd6821d1f084cbb4");
		let origin = EthereumGatewayAddress::get();

		let message = Message {
			gateway: origin,
			nonce: 1,
			origin,
			assets,
			xcm: XcmPayload::Raw(instructions.to_vec()),
			claimer: Some(claimer_bytes),
			value: 1_500_000_000_000u128,
			execution_fee: 1_500_000_000_000u128,
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

		// Assets are trapped
		assert_expected_events!(
			AssetHubPolkadot,
			vec![RuntimeEvent::PolkadotXcm(pallet_xcm::Event::AssetsTrapped { .. }) => {},]
		);
	});
}

#[test]
fn invalid_claimer_does_not_fail_the_message() {
	let relayer_account = BridgeHubPolkadotSender::get();
	let relayer_reward = 1_500_000_000_000u128;

	let beneficiary_acc: [u8; 32] = H256::random().into();
	let beneficiary = Location::new(0, AccountId32 { network: None, id: beneficiary_acc.into() });

	let token_transfer_value = 2_000_000_000_000u128;

	let assets = vec![
		// the token being transferred
		NativeTokenERC20 { token_id: WETH.into(), value: token_transfer_value },
	];

	let origin = H160::random();

	set_up_eth_and_dot_pool();
	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;
		let instructions = vec![
			RefundSurplus,
			// Deposit weth and leftover ether fees to beneficiary.
			DepositAsset { assets: Wild(AllCounted(2)), beneficiary: beneficiary.clone() },
		];
		let xcm: Xcm<()> = instructions.into();
		let versioned_message_xcm = VersionedXcm::V5(xcm);

		let message = Message {
			gateway: EthereumGatewayAddress::get(),
			nonce: 1,
			origin,
			assets,
			xcm: XcmPayload::Raw(versioned_message_xcm.encode()),
			// Set an invalid claimer
			claimer: Some(hex!("2b7ce7bc7e87e4d6619da21487c7a53f").to_vec()),
			value: 1_500_000_000_000u128,
			execution_fee: 1_500_000_000_000u128,
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

	// Message still processes successfully
	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				// Token was issued to beneficiary
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == weth_location(),
					owner: *owner == beneficiary_acc.into(),
				},
				// Leftover fees deposited to beneficiary
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == eth_location(),
					owner: *owner == beneficiary_acc.into(),
				},
			]
		);

		// Beneficiary received the token transfer value
		assert_eq!(
			ForeignAssets::balance(weth_location(), AccountId::from(beneficiary_acc)),
			token_transfer_value
		);

		let events = AssetHubPolkadot::events();
		// Check that no assets were trapped
		assert!(
			!events.iter().any(|event| matches!(
				event,
				RuntimeEvent::PolkadotXcm(pallet_xcm::Event::AssetsTrapped { .. })
			)),
			"Assets were trapped, should not happen."
		);
	});
}

#[test]
fn create_foreign_asset_deposit_is_equal_to_asset_hub_foreign_asset_pallet_deposit() {
	let asset_hub_deposit = asset_hub_polkadot_runtime::ForeignAssetsAssetDeposit::get();
	let bridge_hub_deposit = bp_asset_hub_polkadot::CreateForeignAssetDeposit::get();
	assert!(
		bridge_hub_deposit >=
		asset_hub_deposit,
		"The BridgeHub asset creation deposit must be equal to or larger than the asset creation deposit configured on BridgeHub"
	);
}
