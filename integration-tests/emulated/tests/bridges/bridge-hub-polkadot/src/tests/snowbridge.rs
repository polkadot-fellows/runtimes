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
		asset_hub_kusama_location, bridged_dot_at_ah_kusama, create_foreign_on_ah_kusama,
		dot_at_ah_polkadot, snowbridge_common::*,
	},
	*,
};
use asset_hub_polkadot_runtime::xcm_config::bridging::to_ethereum::{
	BridgeHubEthereumBaseFee, EthereumNetwork,
};
use bp_bridge_hub_polkadot::snowbridge::CreateAssetCall;
use bridge_hub_polkadot_runtime::{
	bridge_to_ethereum_config::EthereumGatewayAddress, xcm_config::RelayTreasuryPalletAccount,
	EthereumBeaconClient, EthereumInboundQueue, Runtime, RuntimeOrigin,
};
use codec::Encode;
use emulated_integration_tests_common::{xcm_emulator::ConvertLocation, RESERVABLE_ASSET_ID};
use hex_literal::hex;
use integration_tests_helpers::common::snowbridge::{MIN_ETHER_BALANCE, WETH};
use polkadot_system_emulated_network::{
	asset_hub_polkadot_emulated_chain::genesis::AssetHubPolkadotAssetOwner,
	penpal_emulated_chain::CustomizableAssetFromSystemAssetHub,
	BridgeHubPolkadotParaSender as BridgeHubPolkadotSender,
};
use snowbridge_beacon_primitives::{
	types::deneb, AncestryProof, BeaconHeader, ExecutionProof, VersionedExecutionPayloadHeader,
};
use snowbridge_core::{gwei, meth, AssetMetadata, Rewards, TokenIdOf};
use snowbridge_inbound_queue_primitives::{
	v1::{Command, Destination, MessageV1, VersionedMessage},
	EthereumLocationsConverterFor, EventFixture, EventProof, Log, Proof,
};
use snowbridge_pallet_system::PricingParametersOf;
use sp_core::{H160, H256, U256};
use sp_runtime::{DispatchError::Token, FixedU128, TokenError::FundsUnavailable};
use system_parachains_constants::polkadot::currency::UNITS;

pub const CHAIN_ID: u64 = 1;
pub const ETHEREUM_DESTINATION_ADDRESS: [u8; 20] = hex!("44a57ee2f2FCcb85FDa2B0B18EBD0D8D2333700e");
pub const GATEWAY_ADDRESS: [u8; 20] = hex!("EDa338E4dC46038493b885327842fD3E301CaB39");

const INITIAL_FUND: u128 = 1_000_000_000_000 * POLKADOT_ED;
const INSUFFICIENT_XCM_FEE: u128 = 1000;
const XCM_FEE: u128 = 4_000_000_000;
const TOKEN_AMOUNT: u128 = 20_000_000_000_000;
const AH_BASE_FEE: u128 = 2_750_872_500_000u128;
const ETHER_TOKEN_ADDRESS: [u8; 20] = [0; 20];

pub fn send_inbound_message(fixture: EventFixture) -> DispatchResult {
	EthereumBeaconClient::store_finalized_header(
		fixture.finalized_header,
		fixture.block_roots_root,
	)
	.unwrap();

	EthereumInboundQueue::submit(
		RuntimeOrigin::signed(BridgeHubPolkadotSender::get()),
		fixture.event,
	)
}

/// Tests the registering of a token as an asset on AssetHub.
#[test]
fn register_token_from_ethereum_to_asset_hub() {
	// Fund AH sovereign account on BH so that it can pay execution fees.
	BridgeHubPolkadot::fund_para_sovereign(AssetHubPolkadot::para_id(), INITIAL_FUND);
	// Fund ethereum sovereign account on AssetHub.
	AssetHubPolkadot::fund_accounts(vec![(ethereum_sovereign_account(), INITIAL_FUND)]);

	let token_id = H160::random();

	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;

		assert_ok!(<BridgeHubPolkadot as Chain>::System::set_storage(
			<BridgeHubPolkadot as Chain>::RuntimeOrigin::root(),
			vec![(EthereumGatewayAddress::key().to_vec(), H160(GATEWAY_ADDRESS).encode())],
		));
		// Construct RegisterToken message and sent to inbound queue
		let message = VersionedMessage::V1(MessageV1 {
			chain_id: CHAIN_ID,
			command: Command::RegisterToken { token: token_id, fee: XCM_FEE },
		});
		// Convert the message to XCM
		let (xcm, _) = EthereumInboundQueue::do_convert([0; 32].into(), message).unwrap();
		let _ = EthereumInboundQueue::send_xcm(xcm, AssetHubPolkadot::para_id()).unwrap();

		assert_expected_events!(
			BridgeHubPolkadot,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Created { .. }) => {},
			]
		);
	});
}

/// Tests sending a token to a 3rd party parachain, called PenPal. The token reserve is
/// still located on AssetHub.
#[test]
fn send_token_from_ethereum_to_penpal() {
	let asset_hub_sovereign = BridgeHubPolkadot::sovereign_account_id_of(Location::new(
		1,
		[Parachain(AssetHubPolkadot::para_id().into())],
	));

	// The Weth asset location, identified by the contract address on Ethereum
	let weth_asset_location: Location =
		(Parent, Parent, EthereumNetwork::get(), AccountKey20 { network: None, key: WETH }).into();
	// Converts the Weth asset location into an asset ID

	// Fund ethereum sovereign on AssetHub
	AssetHubPolkadot::fund_accounts(vec![(ethereum_sovereign_account(), INITIAL_FUND)]);

	set_trust_reserve_on_penpal();

	// Create asset on the Penpal parachain.
	PenpalB::execute_with(|| {
		// Set the trusted asset location from AH, in this case, Ethereum.
		assert_ok!(<PenpalB as Chain>::System::set_storage(
			<PenpalB as Chain>::RuntimeOrigin::root(),
			vec![(
				CustomizableAssetFromSystemAssetHub::key().to_vec(),
				Location::new(2, [GlobalConsensus(Ethereum { chain_id: CHAIN_ID })]).encode(),
			)],
		));

		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::force_create(
			<PenpalB as Chain>::RuntimeOrigin::root(),
			weth_asset_location.clone(),
			asset_hub_sovereign.clone().into(),
			true,
			1000
		));

		assert!(<PenpalB as PenpalBPallet>::ForeignAssets::asset_exists(weth_asset_location));
	});

	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;

		type RuntimeOrigin = <BridgeHubPolkadot as Chain>::RuntimeOrigin;

		// Fund AssetHub sovereign account so it can pay execution fees for the asset transfer
		assert_ok!(<BridgeHubPolkadot as BridgeHubPolkadotPallet>::Balances::force_set_balance(
			RuntimeOrigin::root(),
			asset_hub_sovereign.clone().into(),
			INITIAL_FUND,
		));

		let message_id: H256 = [1; 32].into();
		let message = VersionedMessage::V1(MessageV1 {
			chain_id: CHAIN_ID,
			command: Command::SendToken {
				token: WETH.into(),
				destination: Destination::ForeignAccountId32 {
					para_id: PenpalB::para_id().into(),
					id: PenpalBReceiver::get().into(),
					fee: 40_000_000_000,
				},
				amount: MIN_ETHER_BALANCE,
				fee: 40_000_000_000,
			},
		});
		// Convert the message to XCM
		let (xcm, _) = EthereumInboundQueue::do_convert(message_id, message).unwrap();
		// Send the XCM
		let _ = EthereumInboundQueue::send_xcm(xcm, AssetHubPolkadot::para_id()).unwrap();

		assert_expected_events!(
			BridgeHubPolkadot,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		// Check that the assets were issued on AssetHub
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { .. }) => {},
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	PenpalB::execute_with(|| {
		type RuntimeEvent = <PenpalB as Chain>::RuntimeEvent;
		// Check that the assets were issued on PenPal
		assert_expected_events!(
			PenpalB,
			vec![
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { .. }) => {},
			]
		);
	});
}

/// Tests the registering of a token as an asset on AssetHub, and then subsequently sending
/// a token from Ethereum to AssetHub.
#[test]
fn send_weth_from_ethereum_to_asset_hub() {
	BridgeHubPolkadot::fund_para_sovereign(AssetHubPolkadot::para_id(), INITIAL_FUND);
	// Fund ethereum sovereign account on AssetHub.
	AssetHubPolkadot::fund_accounts(vec![(ethereum_sovereign_account(), INITIAL_FUND)]);

	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;

		assert_ok!(<BridgeHubPolkadot as Chain>::System::set_storage(
			<BridgeHubPolkadot as Chain>::RuntimeOrigin::root(),
			vec![(EthereumGatewayAddress::key().to_vec(), H160(GATEWAY_ADDRESS).encode())],
		));

		// Construct RegisterToken message and sent to inbound queue
		let message_id: H256 = [1; 32].into();
		let message = VersionedMessage::V1(MessageV1 {
			chain_id: CHAIN_ID,
			command: Command::RegisterToken { token: WETH.into(), fee: XCM_FEE },
		});
		// Convert the message to XCM
		let (xcm, _) = EthereumInboundQueue::do_convert(message_id, message).unwrap();
		// Send the XCM
		let _ = EthereumInboundQueue::send_xcm(xcm, AssetHubPolkadot::para_id()).unwrap();

		assert_expected_events!(
			BridgeHubPolkadot,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);

		// Construct SendToken message and sent to inbound queue
		let message = VersionedMessage::V1(MessageV1 {
			chain_id: CHAIN_ID,
			command: Command::SendToken {
				token: WETH.into(),
				destination: Destination::AccountId32 {
					id: AssetHubPolkadotReceiver::get().into(),
				},
				amount: TOKEN_AMOUNT,
				fee: XCM_FEE,
			},
		});
		// Convert the message to XCM
		let (xcm, _) = EthereumInboundQueue::do_convert(message_id, message).unwrap();
		// Send the XCM
		let _ = EthereumInboundQueue::send_xcm(xcm, AssetHubPolkadot::para_id()).unwrap();

		// Check that the message was sent
		assert_expected_events!(
			BridgeHubPolkadot,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

		// Check that the token was received and issued as a foreign asset on AssetHub
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { .. }) => {},
			]
		);
	});
}

// Performs a round trip transfer of a token, asserting success.
fn send_token_from_ethereum_to_asset_hub_and_back_works(
	token_address: H160,
	amount: u128,
	asset_location: Location,
) {
	let assethub_sovereign = BridgeHubPolkadot::sovereign_account_id_of(
		BridgeHubPolkadot::sibling_location_of(AssetHubPolkadot::para_id()),
	);

	BridgeHubPolkadot::fund_accounts(vec![
		(assethub_sovereign.clone(), INITIAL_FUND),
		(RelayTreasuryPalletAccount::get(), INITIAL_FUND),
	]);
	AssetHubPolkadot::fund_accounts(vec![
		(AssetHubPolkadotReceiver::get(), INITIAL_FUND),
		(ethereum_sovereign_account(), INITIAL_FUND),
	]);

	// Set base transfer fee to Ethereum on AH.
	AssetHubPolkadot::execute_with(|| {
		type RuntimeOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;

		assert_ok!(<AssetHubPolkadot as Chain>::System::set_storage(
			RuntimeOrigin::root(),
			vec![(BridgeHubEthereumBaseFee::key().to_vec(), AH_BASE_FEE.encode())],
		));
	});

	// Send Token from Bridge Hub (simulates received Command from Ethereum)
	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;

		assert_ok!(
			<BridgeHubPolkadot as BridgeHubPolkadotPallet>::EthereumSystem::set_pricing_parameters(
				<BridgeHubPolkadot as Chain>::RuntimeOrigin::root(),
				PricingParametersOf::<Runtime> {
					exchange_rate: FixedU128::from_rational(1, 75),
					fee_per_gas: gwei(20),
					rewards: Rewards {
						local: (UNITS / 100), // 0.01 DOT
						remote: meth(1),
					},
					multiplier: FixedU128::from_rational(1, 1),
				}
			)
		);

		assert_ok!(<BridgeHubPolkadot as Chain>::System::set_storage(
			<BridgeHubPolkadot as Chain>::RuntimeOrigin::root(),
			vec![(EthereumGatewayAddress::key().to_vec(), H160(GATEWAY_ADDRESS).encode())],
		));

		let message_id: H256 = [1; 32].into();
		// Construct SendToken message and sent to inbound queue
		let message = VersionedMessage::V1(MessageV1 {
			chain_id: CHAIN_ID,
			command: Command::SendToken {
				token: token_address,
				destination: Destination::AccountId32 {
					id: AssetHubPolkadotReceiver::get().into(),
				},
				amount,
				fee: XCM_FEE,
			},
		});
		// Convert the message to XCM
		let (xcm, _) = EthereumInboundQueue::do_convert(message_id, message).unwrap();
		// Send the XCM
		let _ = EthereumInboundQueue::send_xcm(xcm, AssetHubPolkadot::para_id()).unwrap();

		// Check that the message was sent
		assert_expected_events!(
			BridgeHubPolkadot,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	// Receive Token on Asset Hub.
	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

		// Check that the token was received and issued as a foreign asset on AssetHub
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, .. }) => {
					asset_id: *asset_id == asset_location,
				},
			]
		);
	});

	send_token_back_to_ethereum(asset_location, amount);
}

fn send_token_back_to_ethereum(asset_location: Location, amount: u128) {
	// Send Token from Asset Hub back to Ethereum.
	AssetHubPolkadot::execute_with(|| {
		type RuntimeOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;

		let assets = vec![Asset { id: AssetId(asset_location), fun: Fungible(amount) }];
		let versioned_assets = VersionedAssets::from(Assets::from(assets));

		let destination = VersionedLocation::from(Location::new(
			2,
			[GlobalConsensus(Ethereum { chain_id: CHAIN_ID })],
		));

		let beneficiary = VersionedLocation::from(Location::new(
			0,
			[AccountKey20 { network: None, key: ETHEREUM_DESTINATION_ADDRESS }],
		));

		let free_balance_before =
			<AssetHubPolkadot as AssetHubPolkadotPallet>::Balances::free_balance(
				AssetHubPolkadotReceiver::get(),
			);
		// Send the Token back to Ethereum
		assert_ok!(
 			<AssetHubPolkadot as AssetHubPolkadotPallet>::PolkadotXcm::limited_reserve_transfer_assets(
 				RuntimeOrigin::signed(AssetHubPolkadotReceiver::get()),
 				Box::new(destination),
 				Box::new(beneficiary),
 				Box::new(versioned_assets),
 				0,
 				Unlimited,
 			)
 		);

		let free_balance_after =
			<AssetHubPolkadot as AssetHubPolkadotPallet>::Balances::free_balance(
				AssetHubPolkadotReceiver::get(),
			);
		// Assert at least DefaultBridgeHubEthereumBaseFee charged from the sender
		let free_balance_diff = free_balance_before - free_balance_after;
		assert!(free_balance_diff > AH_BASE_FEE);
	});

	// Check that message with Token was queued on the BridgeHub
	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;
		// check the outbound queue
		assert_expected_events!(
			BridgeHubPolkadot,
			vec![
 				RuntimeEvent::EthereumOutboundQueue(snowbridge_pallet_outbound_queue::Event::MessageQueued
 {..}) => {}, 			]
		);
	});
}

/// Tests sending Ether from Ethereum to Asset Hub and back to Ethereum
#[test]
fn send_eth_asset_from_asset_hub_to_ethereum() {
	let ether_location: Location = (Parent, Parent, EthereumNetwork::get()).into();

	// Perform a roundtrip transfer of Ether
	send_token_from_ethereum_to_asset_hub_and_back_works(
		ETHER_TOKEN_ADDRESS.into(),
		MIN_ETHER_BALANCE + TOKEN_AMOUNT,
		ether_location,
	);
}

/// Tests the full cycle of token transfers:
/// - registering a token on AssetHub
/// - sending a token to AssetHub
/// - returning the token to Ethereum
#[test]
fn send_weth_asset_from_asset_hub_to_ethereum() {
	let weth_location: Location =
		(Parent, Parent, EthereumNetwork::get(), AccountKey20 { network: None, key: WETH }).into();
	// Perform a roundtrip transfer of WETH
	send_token_from_ethereum_to_asset_hub_and_back_works(WETH.into(), TOKEN_AMOUNT, weth_location);
}

#[test]
fn register_weth_token_in_asset_hub_fail_for_insufficient_fee() {
	BridgeHubPolkadot::fund_para_sovereign(AssetHubPolkadot::para_id(), INITIAL_FUND);

	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;

		assert_ok!(<BridgeHubPolkadot as Chain>::System::set_storage(
			<BridgeHubPolkadot as Chain>::RuntimeOrigin::root(),
			vec![(EthereumGatewayAddress::key().to_vec(), H160(GATEWAY_ADDRESS).encode())],
		));

		let message_id: H256 = [1; 32].into();
		let message = VersionedMessage::V1(MessageV1 {
			chain_id: CHAIN_ID,
			command: Command::RegisterToken { token: WETH.into(), fee: INSUFFICIENT_XCM_FEE },
		});
		// Convert the message to XCM
		let (xcm, _) = EthereumInboundQueue::do_convert(message_id, message).unwrap();
		// Send the XCM
		let _ = EthereumInboundQueue::send_xcm(xcm, AssetHubPolkadot::para_id()).unwrap();

		assert_expected_events!(
			BridgeHubPolkadot,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success:false, .. }) => {},
			]
		);
	});
}

#[test]
fn send_token_from_ethereum_to_asset_hub_fail_for_insufficient_fund() {
	// Insufficient fund
	BridgeHubPolkadot::fund_para_sovereign(AssetHubPolkadot::para_id(), 1_000);

	BridgeHubPolkadot::execute_with(|| {
		assert_ok!(<BridgeHubPolkadot as Chain>::System::set_storage(
			<BridgeHubPolkadot as Chain>::RuntimeOrigin::root(),
			vec![(EthereumGatewayAddress::key().to_vec(), H160(GATEWAY_ADDRESS).encode())],
		));

		assert_err!(send_inbound_message(make_register_token_message()), Token(FundsUnavailable));
	});
}

/// Tests that the EthereumInboundQueue CreateAssetCall parameter on BridgeHub matches
/// the ForeignAssets::create call on AssetHub.
#[test]
fn asset_hub_foreign_assets_pallet_is_configured_correctly_in_bridge_hub() {
	let assethub_sovereign = BridgeHubPolkadot::sovereign_account_id_of(Location::new(
		1,
		[Parachain(AssetHubPolkadot::para_id().into())],
	));

	let call_create_foreign_assets =
		<AssetHubPolkadot as Chain>::RuntimeCall::ForeignAssets(pallet_assets::Call::<
			<AssetHubPolkadot as Chain>::Runtime,
			pallet_assets::Instance2,
		>::create {
			id: Location::default(),
			min_balance: ASSET_MIN_BALANCE,
			admin: assethub_sovereign.into(),
		})
		.encode();

	let bridge_hub_inbound_queue_assets_pallet_call_index = CreateAssetCall::get();

	assert!(
		call_create_foreign_assets.starts_with(&bridge_hub_inbound_queue_assets_pallet_call_index)
	);
}

fn ethereum_sovereign_account() -> AccountId {
	let origin_location = (Parent, Parent, EthereumNetwork::get()).into();
	EthereumLocationsConverterFor::<AccountId>::convert_location(&origin_location).unwrap()
}

fn make_register_token_message() -> EventFixture {
	EventFixture {
		event: EventProof {
			event_log: Log {
				address: hex!("eda338e4dc46038493b885327842fd3e301cab39").into(),
				topics: vec![
					hex!("7153f9357c8ea496bba60bf82e67143e27b64462b49041f8e689e1b05728f84f").into(),
					hex!("c173fac324158e77fb5840738a1a541f633cbec8884c6a601c567d2b376a0539").into(),
					hex!("5f7060e971b0dc81e63f0aa41831091847d97c1a4693ac450cc128c7214e65e0").into(),
				],
				data: hex!("00000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000002e00a736aa00000000000087d1f7fdfee7f651fabc8bfcb6e086c278b77a7d00e40b54020000000000000000000000000000000000000000000000000000000000").into(),
			},
			proof: Proof {
				receipt_proof: vec![
					hex!("f851a09c01dd6d2d8de951c45af23d3ad00829ce021c04d6c8acbe1612d456ee320d4980808080808080a04a98e45a319168b0fc6005ce6b744ee9bf54338e2c0784b976a8578d241ced0f8080808080808080").to_vec(),
					hex!("f9028c30b9028802f90284018301d205b9010000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000080000000000000000000000000000004000000000080000000000000000000000000000000000010100000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000040004000000000000002000002000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000100000000000000000200000000000010f90179f85894eda338e4dc46038493b885327842fd3e301cab39e1a0f78bb28d4b1d7da699e5c0bc2be29c2b04b5aab6aacf6298fe5304f9db9c6d7ea000000000000000000000000087d1f7fdfee7f651fabc8bfcb6e086c278b77a7df9011c94eda338e4dc46038493b885327842fd3e301cab39f863a07153f9357c8ea496bba60bf82e67143e27b64462b49041f8e689e1b05728f84fa0c173fac324158e77fb5840738a1a541f633cbec8884c6a601c567d2b376a0539a05f7060e971b0dc81e63f0aa41831091847d97c1a4693ac450cc128c7214e65e0b8a000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000002e00a736aa00000000000087d1f7fdfee7f651fabc8bfcb6e086c278b77a7d00e40b54020000000000000000000000000000000000000000000000000000000000").to_vec(),
				],
				execution_proof: ExecutionProof {
					header: BeaconHeader {
						slot: 393,
						proposer_index: 4,
						parent_root: hex!("6545b47a614a1dd4cad042a0cdbbf5be347e8ffcdc02c6c64540d5153acebeef").into(),
						state_root: hex!("b62ac34a8cb82497be9542fe2114410c9f6021855b766015406101a1f3d86434").into(),
						body_root: hex!("04005fe231e11a5b7b1580cb73b177ae8b338bedd745497e6bb7122126a806db").into(),
					},
					ancestry_proof: Some(AncestryProof {
						header_branch: vec![
							hex!("6545b47a614a1dd4cad042a0cdbbf5be347e8ffcdc02c6c64540d5153acebeef").into(),
							hex!("fa84cc88ca53a72181599ff4eb07d8b444bce023fe2347c3b4f51004c43439d3").into(),
							hex!("cadc8ae211c6f2221c9138e829249adf902419c78eb4727a150baa4d9a02cc9d").into(),
							hex!("33a89962df08a35c52bd7e1d887cd71fa7803e68787d05c714036f6edf75947c").into(),
							hex!("2c9760fce5c2829ef3f25595a703c21eb22d0186ce223295556ed5da663a82cf").into(),
							hex!("e1aa87654db79c8a0ecd6c89726bb662fcb1684badaef5cd5256f479e3c622e1").into(),
							hex!("aa70d5f314e4a1fbb9c362f3db79b21bf68b328887248651fbd29fc501d0ca97").into(),
							hex!("160b6c235b3a1ed4ef5f80b03ee1c76f7bf3f591c92fca9d8663e9221b9f9f0f").into(),
							hex!("f68d7dcd6a07a18e9de7b5d2aa1980eb962e11d7dcb584c96e81a7635c8d2535").into(),
							hex!("1d5f912dfd6697110dd1ecb5cb8e77952eef57d85deb373572572df62bb157fc").into(),
							hex!("ffff0ad7e659772f9534c195c815efc4014ef1e1daed4404c06385d11192e92b").into(),
							hex!("6cf04127db05441cd833107a52be852868890e4317e6a02ab47683aa75964220").into(),
							hex!("b7d05f875f140027ef5118a2247bbb84ce8f2f0f1123623085daf7960c329f5f").into(),
						],
						finalized_block_root: hex!("751414cd97c0624f922b3e80285e9f776b08fa22fd5f87391f2ed7ef571a8d46").into(),
					}),
					execution_header: VersionedExecutionPayloadHeader::Deneb(deneb::ExecutionPayloadHeader {
						parent_hash: hex!("8092290aa21b7751576440f77edd02a94058429ce50e63a92d620951fb25eda2").into(),
						fee_recipient: hex!("0000000000000000000000000000000000000000").into(),
						state_root: hex!("96a83e9ddf745346fafcb0b03d57314623df669ed543c110662b21302a0fae8b").into(),
						receipts_root: hex!("dccdfceea05036f7b61dcdabadc937945d31e68a8d3dfd4dc85684457988c284").into(),
						logs_bloom: hex!("00000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000080000000400000000000000000000004000000000080000000000000000000000000000000000010100000000000000000000000000000000020000000000000000000000000000000000080000000000000000000000000000040004000000000000002002002000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000080000000000000000000000000000000000100000000000000000200000200000010").into(),
						prev_randao: hex!("62e309d4f5119d1f5c783abc20fc1a549efbab546d8d0b25ff1cfd58be524e67").into(),
						block_number: 393,
						gas_limit: 54492273,
						gas_used: 199644,
						timestamp: 1710552813,
						extra_data: hex!("d983010d0b846765746888676f312e32312e368664617277696e").into(),
						base_fee_per_gas: U256::from(7u64),
						block_hash: hex!("6a9810efb9581d30c1a5c9074f27c68ea779a8c1ae31c213241df16225f4e131").into(),
						transactions_root: hex!("2cfa6ed7327e8807c7973516c5c32a68ef2459e586e8067e113d081c3bd8c07d").into(),
						withdrawals_root: hex!("792930bbd5baac43bcc798ee49aa8185ef76bb3b44ba62b91d86ae569e4bb535").into(),
						blob_gas_used: 0,
						excess_blob_gas: 0,
					}),
					execution_branch: vec![
						hex!("a6833fa629f3286b6916c6e50b8bf089fc9126bee6f64d0413b4e59c1265834d").into(),
						hex!("b46f0c01805fe212e15907981b757e6c496b0cb06664224655613dcec82505bb").into(),
						hex!("db56114e00fdd4c1f85c892bf35ac9a89289aaecb1ebd0a96cde606a748b5d71").into(),
						hex!("d3af7c05c516726be7505239e0b9c7cb53d24abce6b91cdb3b3995f0164a75da").into(),
					],
				},
			},
		},
		finalized_header: BeaconHeader {
			slot: 864,
			proposer_index: 4,
			parent_root: hex!("614e7672f991ac268cd841055973f55e1e42228831a211adef207bb7329be614").into(),
			state_root: hex!("5fa8dfca3d760e4242ab46d529144627aa85348a19173b6e081172c701197a4a").into(),
			body_root: hex!("0f34c083b1803666bb1ac5e73fa71582731a2cf37d279ff0a3b0cad5a2ff371e").into(),
		},
		block_roots_root: hex!("b9aab9c388c4e4fcd899b71f62c498fc73406e38e8eb14aa440e9affa06f2a10").into(),
	}
}

fn send_token_from_ethereum_to_asset_hub_with_fee(account_id: [u8; 32], amount: u128, fee: u128) {
	// Fund asset hub sovereign on bridge hub
	let asset_hub_sovereign = BridgeHubPolkadot::sovereign_account_id_of(Location::new(
		1,
		[Parachain(AssetHubPolkadot::para_id().into())],
	));
	BridgeHubPolkadot::fund_accounts(vec![(asset_hub_sovereign.clone(), INITIAL_FUND)]);

	// Send WETH to an existent account on asset hub
	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;

		type EthereumInboundQueue =
			<BridgeHubPolkadot as BridgeHubPolkadotPallet>::EthereumInboundQueue;
		let message_id: H256 = [0; 32].into();
		let message = VersionedMessage::V1(MessageV1 {
			chain_id: CHAIN_ID,
			command: Command::SendToken {
				token: WETH.into(),
				destination: Destination::AccountId32 { id: account_id },
				amount,
				fee,
			},
		});
		let (xcm, _) = EthereumInboundQueue::do_convert(message_id, message).unwrap();
		assert_ok!(EthereumInboundQueue::send_xcm(xcm, AssetHubPolkadot::para_id()));

		// Check that the message was sent
		assert_expected_events!(
			BridgeHubPolkadot,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});
}

#[test]
fn send_token_from_ethereum_to_existent_account_on_asset_hub() {
	send_token_from_ethereum_to_asset_hub_with_fee(
		AssetHubPolkadotSender::get().into(),
		MIN_ETHER_BALANCE,
		XCM_FEE,
	);

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

		// Check that the token was received and issued as a foreign asset on AssetHub
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { .. }) => {},
			]
		);
	});
}

#[test]
fn send_token_from_ethereum_to_non_existent_account_on_asset_hub() {
	send_token_from_ethereum_to_asset_hub_with_fee([1; 32], MIN_ETHER_BALANCE, XCM_FEE);

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

		// Check that the token was received and issued as a foreign asset on AssetHub
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { .. }) => {},
			]
		);
	});
}

#[test]
fn send_token_from_ethereum_to_non_existent_account_on_asset_hub_with_insufficient_fee() {
	send_token_from_ethereum_to_asset_hub_with_fee(
		[1; 32],
		MIN_ETHER_BALANCE,
		INSUFFICIENT_XCM_FEE,
	);

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

		// Check that the message was not processed successfully due to insufficient fee

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success:false, .. }) => {},
			]
		);
	});
}

#[test]
fn send_token_from_ethereum_to_non_existent_account_on_asset_hub_with_sufficient_fee_but_do_not_satisfy_ed(
) {
	// On AH, ED is 0.1 DOT. Make both the transfer amount (in WETH) and the XCM fee below the ED.
	let insufficient_token_amount_in_weth_below_ed = MIN_ETHER_BALANCE - 1;
	let sufficient_fee_in_dot_below_ed = ASSET_HUB_POLKADOT_ED - 1;
	// let sufficient_fee_above_ed = XCM_FEE;
	send_token_from_ethereum_to_asset_hub_with_fee(
		[1; 32],
		insufficient_token_amount_in_weth_below_ed,
		sufficient_fee_in_dot_below_ed,
	);

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		// Since the XCM fee is sufficient, the message is processed successfully.
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success:true, .. }) => {},
			]
		);
		let events = AssetHubPolkadot::events();
		//Check that no foreign assets were issued
		assert!(
			!events.iter().any(|event| matches!(
				event,
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { .. })
			)),
			"Assets issued, should not happen."
		);
		//Check that no new account created
		assert!(
			!events.iter().any(|event| matches!(
				event,
				RuntimeEvent::System(frame_system::Event::NewAccount { .. })
			)),
			"Account created, should not happen."
		);
	});
}

#[test]
fn transfer_relay_token() {
	let assethub_sovereign = BridgeHubPolkadot::sovereign_account_id_of(
		BridgeHubPolkadot::sibling_location_of(AssetHubPolkadot::para_id()),
	);
	BridgeHubPolkadot::fund_accounts(vec![(assethub_sovereign.clone(), INITIAL_FUND)]);

	let asset_id: Location = Location { parents: 1, interior: [].into() };
	let expected_asset_id: Location =
		Location { parents: 1, interior: [GlobalConsensus(Polkadot)].into() };

	let expected_token_id = TokenIdOf::convert_location(&expected_asset_id).unwrap();

	let ethereum_sovereign: AccountId =
		EthereumLocationsConverterFor::<[u8; 32]>::convert_location(&Location::new(
			2,
			[GlobalConsensus(EthereumNetwork::get())],
		))
		.unwrap()
		.into();

	// Register token
	BridgeHubPolkadot::execute_with(|| {
		type RuntimeOrigin = <BridgeHubPolkadot as Chain>::RuntimeOrigin;
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;

		assert_ok!(<BridgeHubPolkadot as BridgeHubPolkadotPallet>::Balances::force_set_balance(
			RuntimeOrigin::root(),
			sp_runtime::MultiAddress::Id(BridgeHubPolkadotSender::get()),
			INITIAL_FUND * 10,
		));

		assert_ok!(<BridgeHubPolkadot as BridgeHubPolkadotPallet>::EthereumSystem::register_token(
			RuntimeOrigin::root(),
			Box::new(VersionedLocation::V5(asset_id.clone())),
			AssetMetadata {
				name: "wnd".as_bytes().to_vec().try_into().unwrap(),
				symbol: "wnd".as_bytes().to_vec().try_into().unwrap(),
				decimals: 12,
			},
		));
		// Check that a message was sent to Ethereum to create the agent
		assert_expected_events!(
			BridgeHubPolkadot,
			vec![RuntimeEvent::EthereumSystem(snowbridge_pallet_system::Event::RegisterToken { .. }) => {},]
		);
	});

	// Send token to Ethereum
	AssetHubPolkadot::execute_with(|| {
		type RuntimeOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

		// Set base transfer fee to Ethereum on AH.
		assert_ok!(<AssetHubPolkadot as Chain>::System::set_storage(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::root(),
			vec![(BridgeHubEthereumBaseFee::key().to_vec(), AH_BASE_FEE.encode())],
		));

		let assets = vec![Asset { id: AssetId(Location::parent()), fun: Fungible(TOKEN_AMOUNT) }];
		let versioned_assets = VersionedAssets::V5(Assets::from(assets));

		let destination = VersionedLocation::V5(Location::new(
			2,
			[GlobalConsensus(Ethereum { chain_id: CHAIN_ID })],
		));

		let beneficiary =
			Location::new(0, [AccountKey20 { network: None, key: ETHEREUM_DESTINATION_ADDRESS }]);

		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
			RuntimeOrigin::signed(AssetHubPolkadotSender::get()),
			Box::new(destination),
			Box::new(versioned_assets),
			Box::new(TransferType::LocalReserve),
			Box::new(VersionedAssetId::from(AssetId(Location::parent()))),
			Box::new(TransferType::LocalReserve),
			Box::new(VersionedXcm::from(
				Xcm::<()>::builder_unsafe()
					.deposit_asset(AllCounted(1), beneficiary)
					.build()
			)),
			Unlimited,
		));

		let events = AssetHubPolkadot::events();
		// Check that the native asset transferred to some reserved account(sovereign of Ethereum)
		assert!(
			events.iter().any(|event| matches!(
				event,
				RuntimeEvent::Balances(pallet_balances::Event::Transfer { amount, to, ..})
					if *amount == TOKEN_AMOUNT && *to == ethereum_sovereign.clone(),
			)),
			"native token reserved to Ethereum sovereign account."
		);
	});

	// Send token back from ethereum
	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;

		// Check that the transfer token back to Ethereum message was queue in the Ethereum
		// Outbound Queue
		assert_expected_events!(
			BridgeHubPolkadot,
			vec![RuntimeEvent::EthereumOutboundQueue(snowbridge_pallet_outbound_queue::Event::MessageQueued{ .. }) => {},]
		);

		// Send relay token back to AH
		let message_id: H256 = [0; 32].into();
		let message = VersionedMessage::V1(MessageV1 {
			chain_id: CHAIN_ID,
			command: Command::SendNativeToken {
				token_id: expected_token_id,
				destination: Destination::AccountId32 {
					id: AssetHubPolkadotReceiver::get().into(),
				},
				amount: TOKEN_AMOUNT,
				fee: XCM_FEE,
			},
		});
		// Convert the message to XCM
		let (xcm, _) = EthereumInboundQueue::do_convert(message_id, message).unwrap();
		// Send the XCM
		let _ = EthereumInboundQueue::send_xcm(xcm, AssetHubPolkadot::para_id()).unwrap();

		assert_expected_events!(
			BridgeHubPolkadot,
			vec![RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

		assert_expected_events!(
			AssetHubPolkadot,
			vec![RuntimeEvent::Balances(pallet_balances::Event::Burned{ .. }) => {},]
		);

		let events = AssetHubPolkadot::events();

		// Check that the native token burnt from some reserved account
		assert!(
			events.iter().any(|event| matches!(
				event,
				RuntimeEvent::Balances(pallet_balances::Event::Burned { who, ..})
					if *who == ethereum_sovereign.clone(),
			)),
			"native token burnt from Ethereum sovereign account."
		);

		// Check that the token was minted to beneficiary
		assert!(
			events.iter().any(|event| matches!(
				event,
				RuntimeEvent::Balances(pallet_balances::Event::Minted { who, amount })
					if *amount >= TOKEN_AMOUNT && *who == AssetHubPolkadotReceiver::get()
			)),
			"Token minted to beneficiary."
		);
	});
}

#[test]
fn transfer_ah_token() {
	let assethub_sovereign = BridgeHubPolkadot::sovereign_account_id_of(
		BridgeHubPolkadot::sibling_location_of(AssetHubPolkadot::para_id()),
	);
	BridgeHubPolkadot::fund_accounts(vec![(assethub_sovereign.clone(), INITIAL_FUND)]);

	let ethereum_destination = Location::new(2, [GlobalConsensus(Ethereum { chain_id: CHAIN_ID })]);

	let ethereum_sovereign: AccountId =
		EthereumLocationsConverterFor::<[u8; 32]>::convert_location(&ethereum_destination)
			.unwrap()
			.into();
	AssetHubPolkadot::fund_accounts(vec![(ethereum_sovereign.clone(), INITIAL_FUND)]);

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
		[GlobalConsensus(Polkadot), Parachain(AssetHubPolkadot::para_id().into())],
	)
	.appended_with(asset_id.clone().interior)
	.unwrap();

	let token_id = TokenIdOf::convert_location(&asset_id_after_reanchored).unwrap();

	// Register token
	BridgeHubPolkadot::execute_with(|| {
		type RuntimeOrigin = <BridgeHubPolkadot as Chain>::RuntimeOrigin;

		assert_ok!(<BridgeHubPolkadot as BridgeHubPolkadotPallet>::EthereumSystem::register_token(
			RuntimeOrigin::root(),
			Box::new(VersionedLocation::V5(asset_id_in_bh.clone())),
			AssetMetadata {
				name: "ah_asset".as_bytes().to_vec().try_into().unwrap(),
				symbol: "ah_asset".as_bytes().to_vec().try_into().unwrap(),
				decimals: 12,
			},
		));
	});

	// Mint some token
	AssetHubPolkadot::mint_asset(
		<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotAssetOwner::get()),
		RESERVABLE_ASSET_ID,
		AssetHubPolkadotSender::get(),
		TOKEN_AMOUNT,
	);

	// Send token to Ethereum
	AssetHubPolkadot::execute_with(|| {
		type RuntimeOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

		// Set base transfer fee to Ethereum on AH.
		assert_ok!(<AssetHubPolkadot as Chain>::System::set_storage(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::root(),
			vec![(BridgeHubEthereumBaseFee::key().to_vec(), AH_BASE_FEE.encode())],
		));

		// Send partial of the token, will fail if send all
		let assets = vec![Asset { id: AssetId(asset_id.clone()), fun: Fungible(TOKEN_AMOUNT / 2) }];
		let versioned_assets = VersionedAssets::V5(Assets::from(assets));

		let beneficiary = VersionedLocation::V5(Location::new(
			0,
			[AccountKey20 { network: None, key: ETHEREUM_DESTINATION_ADDRESS }],
		));

		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::PolkadotXcm::limited_reserve_transfer_assets(
			RuntimeOrigin::signed(AssetHubPolkadotSender::get()),
			Box::new(VersionedLocation::from(ethereum_destination)),
			Box::new(beneficiary),
			Box::new(versioned_assets),
			0,
			Unlimited,
		));

		assert_expected_events!(
			AssetHubPolkadot,
			vec![RuntimeEvent::Assets(pallet_assets::Event::Transferred{ .. }) => {},]
		);

		let events = AssetHubPolkadot::events();
		// Check that the native asset transferred to some reserved account(sovereign of Ethereum)
		assert!(
			events.iter().any(|event| matches!(
				event,
				RuntimeEvent::Assets(pallet_assets::Event::Transferred { asset_id, to, ..})
					if *asset_id == RESERVABLE_ASSET_ID && *to == ethereum_sovereign.clone()
			)),
			"native token reserved to Ethereum sovereign account."
		);
	});

	// Send token back from Ethereum
	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;

		// Check that the transfer token back to Ethereum message was queue in the Ethereum
		// Outbound Queue
		assert_expected_events!(
			BridgeHubPolkadot,
			vec![RuntimeEvent::EthereumOutboundQueue(snowbridge_pallet_outbound_queue::Event::MessageQueued{ .. }) => {},]
		);

		let message = VersionedMessage::V1(MessageV1 {
			chain_id: CHAIN_ID,
			command: Command::SendNativeToken {
				token_id,
				destination: Destination::AccountId32 {
					id: AssetHubPolkadotReceiver::get().into(),
				},
				amount: TOKEN_AMOUNT / 10,
				fee: XCM_FEE,
			},
		});
		// Convert the message to XCM
		let (xcm, _) = EthereumInboundQueue::do_convert([0; 32].into(), message).unwrap();
		// Send the XCM
		let _ = EthereumInboundQueue::send_xcm(xcm, AssetHubPolkadot::para_id()).unwrap();

		assert_expected_events!(
			BridgeHubPolkadot,
			vec![RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

		assert_expected_events!(
			AssetHubPolkadot,
			vec![RuntimeEvent::Assets(pallet_assets::Event::Burned{..}) => {},]
		);

		let events = AssetHubPolkadot::events();

		// Check that the native token burnt from some reserved account
		assert!(
			events.iter().any(|event| matches!(
				event,
				RuntimeEvent::Assets(pallet_assets::Event::Burned { owner, .. })
					if *owner == ethereum_sovereign.clone(),
			)),
			"token burnt from Ethereum sovereign account."
		);

		// Check that the token was minted to beneficiary
		assert!(
			events.iter().any(|event| matches!(
				event,
				RuntimeEvent::Assets(pallet_assets::Event::Issued { owner, .. })
					if *owner == AssetHubPolkadotReceiver::get()
			)),
			"Token minted to beneficiary."
		);
	});
}

#[test]
fn send_weth_from_ethereum_to_ahp_to_ahk_and_back() {
	let sender = AssetHubPolkadotSender::get();
	let assethub_location = BridgeHubPolkadot::sibling_location_of(AssetHubPolkadot::para_id());
	let assethub_sovereign = BridgeHubPolkadot::sovereign_account_id_of(assethub_location);

	BridgeHubPolkadot::fund_accounts(vec![
		(assethub_sovereign.clone(), INITIAL_FUND),
		(RelayTreasuryPalletAccount::get(), INITIAL_FUND),
	]);
	AssetHubPolkadot::fund_accounts(vec![
		(AssetHubPolkadotReceiver::get(), INITIAL_FUND),
		(ethereum_sovereign_account(), INITIAL_FUND),
		(sender.clone(), INITIAL_FUND),
	]);
	BridgeHubKusama::fund_para_sovereign(AssetHubKusama::para_id(), INITIAL_FUND);
	BridgeHubPolkadot::fund_para_sovereign(AssetHubPolkadot::para_id(), INITIAL_FUND);

	let asset_hub_polkadot_location = Location::new(
		2,
		[GlobalConsensus(Polkadot), Parachain(AssetHubPolkadot::para_id().into())],
	);
	// set XCM versions
	BridgeHubPolkadot::force_xcm_version(asset_hub_polkadot_location.clone(), XCM_VERSION);
	BridgeHubPolkadot::force_xcm_version(asset_hub_kusama_location(), XCM_VERSION);
	AssetHubPolkadot::force_xcm_version(asset_hub_kusama_location(), XCM_VERSION);
	AssetHubKusama::force_xcm_version(asset_hub_polkadot_location.clone(), XCM_VERSION);
	BridgeHubKusama::force_xcm_version(asset_hub_polkadot_location.clone(), XCM_VERSION);
	BridgeHubKusama::force_xcm_version(asset_hub_kusama_location(), XCM_VERSION);

	let bridged_dot_at_asset_hub_kusama = bridged_dot_at_ah_kusama();

	// Create foreign asset using the V4 location
	create_foreign_on_ah_kusama(bridged_dot_at_asset_hub_kusama.clone(), true);

	// We'll need this later in the code, so clone it before it's moved into the closure
	let bridged_dot_at_asset_hub_kusama_for_later = bridged_dot_at_asset_hub_kusama.clone();

	// Create the pool directly instead of using the macro to avoid version mismatch issues
	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
		let owner = sender.clone();
		let signed_owner = <AssetHubKusama as Chain>::RuntimeOrigin::signed(owner.clone());

		// Native KSM asset (Parent)
		let native_asset: Location = Parent.into();

		// Mint foreign asset
		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets::mint(
			signed_owner.clone(),
			bridged_dot_at_asset_hub_kusama.clone(),
			owner.clone().into(),
			10_000_000_000_000, // For it to have more than enough.
		));

		// Create the pool
		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::AssetConversion::create_pool(
			signed_owner.clone(),
			Box::new(native_asset.clone()),
			Box::new(bridged_dot_at_asset_hub_kusama.clone()),
		));

		assert_expected_events!(
			AssetHubKusama,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::PoolCreated { .. }) => {},
			]
		);

		// Add liquidity
		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::AssetConversion::add_liquidity(
			signed_owner,
			Box::new(native_asset),
			Box::new(bridged_dot_at_asset_hub_kusama),
			1_000_000_000_000,
			2_000_000_000_000, // $asset is worth half of native_asset
			0,
			0,
			owner
		));

		assert_expected_events!(
			AssetHubKusama,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::LiquidityAdded { .. }) => {},
			]
		);
	});

	// Set base transfer fee to Ethereum on AH.
	AssetHubPolkadot::execute_with(|| {
		type RuntimeOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;

		assert_ok!(<AssetHubPolkadot as Chain>::System::set_storage(
			RuntimeOrigin::root(),
			vec![(BridgeHubEthereumBaseFee::key().to_vec(), AH_BASE_FEE.encode())],
		));
	});

	// Bridge token from Ethereum to AHP
	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;

		assert_ok!(
			<BridgeHubPolkadot as BridgeHubPolkadotPallet>::EthereumSystem::set_pricing_parameters(
				<BridgeHubPolkadot as Chain>::RuntimeOrigin::root(),
				PricingParametersOf::<Runtime> {
					exchange_rate: FixedU128::from_rational(1, 75),
					fee_per_gas: gwei(20),
					rewards: Rewards {
						local: (UNITS / 100), // 0.01 DOT
						remote: meth(1),
					},
					multiplier: FixedU128::from_rational(1, 1),
				}
			)
		);

		// Construct SendToken message and sent to inbound queue
		let message = VersionedMessage::V1(MessageV1 {
			chain_id: CHAIN_ID,
			command: Command::SendToken {
				token: WETH.into(),
				destination: Destination::AccountId32 { id: sender.clone().into() },
				amount: MIN_ETHER_BALANCE * 4,
				fee: XCM_FEE,
			},
		});
		// Convert the message to XCM
		let message_id: H256 = [1; 32].into();
		let (xcm, _) = EthereumInboundQueue::do_convert(message_id, message).unwrap();
		// Send the XCM
		let _ = EthereumInboundQueue::send_xcm(xcm, AssetHubPolkadot::para_id()).unwrap();

		// Check that the message was sent
		assert_expected_events!(
			BridgeHubPolkadot,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

		// Check that the token was received and issued as a foreign asset on AssetHub
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { .. }) => {},
			]
		);
	});

	let beneficiary =
		Location::new(0, [AccountId32 { network: None, id: AssetHubKusamaReceiver::get().into() }]);
	let weth_location = Location::new(
		2,
		[GlobalConsensus(EthereumNetwork::get()), AccountKey20 { network: None, key: WETH }],
	);

	let fee = dot_at_ah_polkadot();
	let fees_asset: AssetId = fee.clone().into();
	let custom_xcm_on_dest =
		Xcm::<()>(vec![DepositAsset { assets: Wild(AllCounted(2)), beneficiary }]);

	AssetHubPolkadot::fund_accounts(vec![
		// to pay fees to AHK
		(sender.clone(), INITIAL_FUND),
	]);

	let assets: Assets =
		vec![(weth_location.clone(), MIN_ETHER_BALANCE).into(), (fee.clone(), XCM_FEE * 3).into()]
			.into();

	assert_ok!(AssetHubPolkadot::execute_with(|| {
		<AssetHubPolkadot as AssetHubPolkadotPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
 			<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(sender),
 			bx!(asset_hub_kusama_location().into()),
 			bx!(assets.into()),
 			bx!(TransferType::LocalReserve),
 			bx!(fees_asset.into()),
 			bx!(TransferType::LocalReserve),
 			bx!(VersionedXcm::from(custom_xcm_on_dest)),
 			WeightLimit::Unlimited,
 		)
	}));

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

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

	// process and verify intermediary hops
	assert_bridge_hub_polkadot_message_accepted(true);
	assert_bridge_hub_kusama_message_received();

	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;

		// Check that the token was received and issued as a foreign asset on AssetHub
		assert_expected_events!(
			AssetHubKusama,
			vec![
				// Token was issued to beneficiary
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == weth_location,
					owner: *owner == AssetHubKusamaReceiver::get(),
				},
			]
		);

		let events = AssetHubKusama::events();
		// Check that no assets were trapped
		assert!(
			!events.iter().any(|event| matches!(
				event,
				RuntimeEvent::PolkadotXcm(pallet_xcm::Event::AssetsTrapped { .. })
			)),
			"Assets were trapped, should not happen."
		);
	});

	let beneficiary = Location::new(
		0,
		[AccountId32 { network: None, id: AssetHubPolkadotReceiver::get().into() }],
	);
	let fee = bridged_dot_at_asset_hub_kusama_for_later.clone();
	let fees_asset: AssetId = fee.clone().into();
	let custom_xcm_on_dest =
		Xcm::<()>(vec![DepositAsset { assets: Wild(AllCounted(2)), beneficiary }]);

	let assets: Assets =
		vec![(weth_location.clone(), MIN_ETHER_BALANCE).into(), (fee.clone(), XCM_FEE).into()]
			.into();

	// Transfer the token back to Polkadot.
	assert_ok!(AssetHubKusama::execute_with(|| {
		<AssetHubKusama as AssetHubKusamaPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(AssetHubKusamaReceiver::get()),
			bx!(asset_hub_polkadot_location.into()),
			bx!(assets.into()),
			bx!(TransferType::DestinationReserve),
			bx!(fees_asset.into()),
			bx!(TransferType::DestinationReserve),
			bx!(VersionedXcm::from(custom_xcm_on_dest)),
			WeightLimit::Unlimited,
		)
	}));

	BridgeHubKusama::execute_with(|| {
		type RuntimeEvent = <BridgeHubKusama as Chain>::RuntimeEvent;
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
	});

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
	});

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

		// Check that the token was received and issued as a foreign asset on AssetHub
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				// Token was issued to beneficiary
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == weth_location,
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

	send_token_back_to_ethereum(weth_location, MIN_ETHER_BALANCE);
}

#[test]
fn export_from_non_system_parachain_will_fail() {
	let penpal_sovereign = BridgeHubPolkadot::sovereign_account_id_of(Location::new(
		1,
		[Parachain(PenpalB::para_id().into())],
	));
	BridgeHubPolkadot::fund_accounts(vec![(penpal_sovereign.clone(), INITIAL_FUND)]);

	PenpalB::execute_with(|| {
		type RuntimeEvent = <PenpalB as Chain>::RuntimeEvent;
		type RuntimeOrigin = <PenpalB as Chain>::RuntimeOrigin;

		let local_fee_asset =
			Asset { id: AssetId(Location::here()), fun: Fungible(1_000_000_000_000) };

		let weth_location_reanchored =
			Location::new(0, [AccountKey20 { network: None, key: WETH }]);

		let weth_asset =
			Asset { id: AssetId(weth_location_reanchored.clone()), fun: Fungible(TOKEN_AMOUNT) };

		assert_ok!(<PenpalB as PenpalBPallet>::PolkadotXcm::send(
			RuntimeOrigin::root(),
			bx!(VersionedLocation::from(Location::new(
				1,
				Parachain(BridgeHubPolkadot::para_id().into())
			))),
			bx!(VersionedXcm::from(Xcm(vec![
				WithdrawAsset(local_fee_asset.clone().into()),
				BuyExecution { fees: local_fee_asset.clone(), weight_limit: Unlimited },
				ExportMessage {
					network: Ethereum { chain_id: CHAIN_ID },
					destination: Here,
					xcm: Xcm(vec![
						WithdrawAsset(weth_asset.clone().into()),
						DepositAsset {
							assets: Wild(All),
							beneficiary: Location::new(
								0,
								[AccountKey20 { network: None, key: ETHEREUM_DESTINATION_ADDRESS }]
							)
						},
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
			vec![RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed{ success:false, origin,
.. }) => { 				origin: *origin ==
bridge_hub_common::AggregateMessageOrigin::Sibling(PenpalB::para_id()), 			},]
		);
	});
}
