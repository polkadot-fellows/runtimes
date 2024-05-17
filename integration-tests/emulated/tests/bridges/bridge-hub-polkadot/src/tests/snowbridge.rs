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
use crate::*;
use asset_hub_polkadot_runtime::xcm_config::{
	bridging::to_ethereum::{BridgeHubEthereumBaseFee, EthereumNetwork},
	RelayTreasuryPalletAccount,
};
use bp_bridge_hub_polkadot::snowbridge::CreateAssetCall;
use bridge_hub_polkadot_runtime::{
	bridge_to_ethereum_config::EthereumGatewayAddress, EthereumBeaconClient, EthereumInboundQueue,
	Runtime, RuntimeOrigin,
};
use codec::{Decode, Encode};
use emulated_integration_tests_common::xcm_emulator::ConvertLocation;
use frame_support::pallet_prelude::TypeInfo;
use hex_literal::hex;
use polkadot_system_emulated_network::{
	penpal_emulated_chain::CustomizableAssetFromSystemAssetHub,
	BridgeHubPolkadotParaSender as BridgeHubPolkadotSender,
};
use snowbridge_beacon_primitives::CompactExecutionHeader;
use snowbridge_core::{
	gwei,
	inbound::{Log, Message, Proof},
	meth,
	outbound::OperatingMode,
	Rewards,
};
use snowbridge_pallet_inbound_queue_fixtures::{
	register_token_with_insufficient_fee::make_register_token_with_infufficient_fee_message,
	InboundQueueFixture,
};
use snowbridge_pallet_system::PricingParametersOf;
use snowbridge_router_primitives::inbound::{
	Command, Destination, GlobalConsensusEthereumConvertsFor, MessageV1, VersionedMessage,
};
use sp_core::{H160, H256};
use sp_runtime::{DispatchError::Token, FixedU128, TokenError::FundsUnavailable};
use system_parachains_constants::polkadot::currency::UNITS;

const INITIAL_FUND: u128 = 5_000_000_000 * POLKADOT_ED;
const CHAIN_ID: u64 = 1;
const WETH: [u8; 20] = hex!("87d1f7fdfEe7f651FaBc8bFCB6E086C278b77A7d");
const ETHEREUM_DESTINATION_ADDRESS: [u8; 20] = hex!("44a57ee2f2FCcb85FDa2B0B18EBD0D8D2333700e");
const GATEWAY_ADDRESS: [u8; 20] = hex!("EDa338E4dC46038493b885327842fD3E301CaB39");

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, TypeInfo)]
pub enum ControlCall {
	#[codec(index = 3)]
	CreateAgent,
	#[codec(index = 4)]
	CreateChannel { mode: OperatingMode },
}

#[allow(clippy::large_enum_variant)]
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, TypeInfo)]
pub enum SnowbridgeControl {
	#[codec(index = 83)]
	Control(ControlCall),
}

pub fn send_inbound_message(fixture: InboundQueueFixture) -> DispatchResult {
	EthereumBeaconClient::store_execution_header(
		fixture.message.proof.block_hash,
		fixture.execution_header,
		0,
		H256::default(),
	);

	EthereumInboundQueue::submit(
		RuntimeOrigin::signed(BridgeHubPolkadotSender::get()),
		fixture.message,
	)
}

/// Create an agent on Ethereum. An agent is a representation of an entity in the Polkadot
/// ecosystem (like a parachain) on Ethereum.
#[test]
#[ignore]
fn create_agent() {
	let origin_para: u32 = 1001;
	// Fund the origin parachain sovereign account so that it can pay execution fees.
	BridgeHubPolkadot::fund_para_sovereign(origin_para.into(), INITIAL_FUND);

	let sudo_origin = <Polkadot as Chain>::RuntimeOrigin::root();
	let destination = Polkadot::child_location_of(BridgeHubPolkadot::para_id()).into();

	let create_agent_call = SnowbridgeControl::Control(ControlCall::CreateAgent {});
	// Construct XCM to create an agent for para 1001
	let remote_xcm = VersionedXcm::from(Xcm(vec![
		UnpaidExecution { weight_limit: Unlimited, check_origin: None },
		DescendOrigin(Parachain(origin_para).into()),
		Transact {
			require_weight_at_most: 3000000000.into(),
			origin_kind: OriginKind::Xcm,
			call: create_agent_call.encode().into(),
		},
	]));

	// Polkadot Global Consensus
	// Send XCM message from Relay Chain to Bridge Hub source Parachain
	Polkadot::execute_with(|| {
		assert_ok!(<Polkadot as PolkadotPallet>::XcmPallet::send(
			sudo_origin,
			bx!(destination),
			bx!(remote_xcm),
		));

		type RuntimeEvent = <Polkadot as Chain>::RuntimeEvent;
		// Check that the Transact message was sent
		assert_expected_events!(
			Polkadot,
			vec![
				RuntimeEvent::XcmPallet(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;
		// Check that a message was sent to Ethereum to create the agent
		assert_expected_events!(
			BridgeHubPolkadot,
			vec![
				RuntimeEvent::EthereumSystem(snowbridge_pallet_system::Event::CreateAgent {
					..
				}) => {},
			]
		);
	});
}

/// Create a channel for a consensus system. A channel is a bidirectional messaging channel
/// between BridgeHub and Ethereum.
#[test]
#[ignore]
fn create_channel() {
	let origin_para: u32 = 1001;
	// Fund AssetHub sovereign account so that it can pay execution fees.
	BridgeHubPolkadot::fund_para_sovereign(origin_para.into(), INITIAL_FUND);

	let sudo_origin = <Polkadot as Chain>::RuntimeOrigin::root();
	let destination: VersionedLocation =
		Polkadot::child_location_of(BridgeHubPolkadot::para_id()).into();

	let create_agent_call = SnowbridgeControl::Control(ControlCall::CreateAgent {});
	// Construct XCM to create an agent for para 1001
	let create_agent_xcm = VersionedXcm::from(Xcm(vec![
		UnpaidExecution { weight_limit: Unlimited, check_origin: None },
		DescendOrigin(Parachain(origin_para).into()),
		Transact {
			require_weight_at_most: 3000000000.into(),
			origin_kind: OriginKind::Xcm,
			call: create_agent_call.encode().into(),
		},
	]));

	let create_channel_call =
		SnowbridgeControl::Control(ControlCall::CreateChannel { mode: OperatingMode::Normal });
	// Construct XCM to create a channel for para 1001
	let create_channel_xcm = VersionedXcm::from(Xcm(vec![
		UnpaidExecution { weight_limit: Unlimited, check_origin: None },
		DescendOrigin(Parachain(origin_para).into()),
		Transact {
			require_weight_at_most: 3000000000.into(),
			origin_kind: OriginKind::Xcm,
			call: create_channel_call.encode().into(),
		},
	]));

	// Polkadot Global Consensus
	// Send XCM message from Relay Chain to Bridge Hub source Parachain
	Polkadot::execute_with(|| {
		assert_ok!(<Polkadot as PolkadotPallet>::XcmPallet::send(
			sudo_origin.clone(),
			bx!(destination.clone()),
			bx!(create_agent_xcm),
		));

		assert_ok!(<Polkadot as PolkadotPallet>::XcmPallet::send(
			sudo_origin,
			bx!(destination),
			bx!(create_channel_xcm),
		));

		type RuntimeEvent = <Polkadot as Chain>::RuntimeEvent;

		assert_expected_events!(
			Polkadot,
			vec![
				RuntimeEvent::XcmPallet(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;

		// Check that the Channel was created
		assert_expected_events!(
			BridgeHubPolkadot,
			vec![
				RuntimeEvent::EthereumSystem(snowbridge_pallet_system::Event::CreateChannel {
					..
				}) => {},
			]
		);
	});
}

/// Tests the registering of a token as an asset on AssetHub.
#[test]
fn register_weth_token_from_ethereum_to_asset_hub() {
	// Fund AH sovereign account on BH so that it can pay execution fees.
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
		let register_token_message = make_register_token_message();
		assert_ok!(send_inbound_message(register_token_message.clone()));

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
	let weth_asset_id: v3::Location = weth_asset_location.try_into().unwrap();

	// Fund ethereum sovereign on AssetHub
	AssetHubPolkadot::fund_accounts(vec![(ethereum_sovereign_account(), INITIAL_FUND)]);

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

		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::create(
			<PenpalB as Chain>::RuntimeOrigin::signed(PenpalBSender::get()),
			weth_asset_id,
			asset_hub_sovereign.clone().into(),
			1000,
		));

		assert!(<PenpalB as PenpalBPallet>::ForeignAssets::asset_exists(weth_asset_id));
	});

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::force_create(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::root(),
			weth_asset_id,
			asset_hub_sovereign.clone().into(),
			true,
			1000,
		));

		assert!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::asset_exists(
			weth_asset_id
		));
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
				amount: 1_000_000,
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
fn send_token_from_ethereum_to_asset_hub() {
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
		assert_ok!(send_inbound_message(make_register_token_message()));

		// Construct SendToken message and sent to inbound queue
		assert_ok!(send_inbound_message(make_send_token_message()));

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

/// Tests the full cycle of token transfers:
/// - registering a token on AssetHub
/// - sending a token to AssetHub
/// - returning the token to Ethereum
#[test]
fn send_weth_asset_from_asset_hub_to_ethereum() {
	let assethub_sovereign = BridgeHubPolkadot::sovereign_account_id_of(Location::new(
		1,
		[Parachain(AssetHubPolkadot::para_id().into())],
	));

	AssetHubPolkadot::force_default_xcm_version(Some(XCM_VERSION));
	BridgeHubPolkadot::force_default_xcm_version(Some(XCM_VERSION));
	AssetHubPolkadot::force_xcm_version(
		Location::new(2, [GlobalConsensus(Ethereum { chain_id: CHAIN_ID })]),
		XCM_VERSION,
	);

	BridgeHubPolkadot::fund_accounts(vec![
		(assethub_sovereign.clone(), INITIAL_FUND),
		(RelayTreasuryPalletAccount::get(), INITIAL_FUND),
	]);
	AssetHubPolkadot::fund_accounts(vec![
		(AssetHubPolkadotReceiver::get(), INITIAL_FUND),
		(ethereum_sovereign_account(), INITIAL_FUND),
	]);

	const WETH_AMOUNT: u128 = 1_000_000_000;
	let base_fee = 2_750_872_500_000u128;

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(<AssetHubPolkadot as Chain>::System::set_storage(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::root(),
			vec![(BridgeHubEthereumBaseFee::key().to_vec(), base_fee.encode())],
		));
	});

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

		// Construct RegisterToken message and sent to inbound queue
		assert_ok!(send_inbound_message(make_register_token_message()));

		// Check that the register token message was sent using xcm
		assert_expected_events!(
			BridgeHubPolkadot,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);

		// Construct SendToken message and sent to inbound queue
		assert_ok!(send_inbound_message(make_send_token_message()));

		// Check that the send token message was sent using xcm
		assert_expected_events!(
			BridgeHubPolkadot,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	// check treasury account balance on BH before
	let treasury_account_before = BridgeHubPolkadot::execute_with(|| {
		<<BridgeHubPolkadot as BridgeHubPolkadotPallet>::Balances as frame_support::traits::fungible::Inspect<_>>::balance(&RelayTreasuryPalletAccount::get())
	});

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		type RuntimeOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;

		// Check that AssetHub has issued the foreign asset
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { .. }) => {},
			]
		);
		let assets = vec![Asset {
			id: AssetId(Location::new(
				2,
				[
					GlobalConsensus(Ethereum { chain_id: CHAIN_ID }),
					AccountKey20 { network: None, key: WETH },
				],
			)),
			fun: Fungible(WETH_AMOUNT),
		}];
		let multi_assets = VersionedAssets::V4(Assets::from(assets));

		let destination = VersionedLocation::V4(Location::new(
			2,
			[GlobalConsensus(Ethereum { chain_id: CHAIN_ID })],
		));

		let beneficiary = VersionedLocation::V4(Location::new(
			0,
			[AccountKey20 { network: None, key: ETHEREUM_DESTINATION_ADDRESS }],
		));

		let free_balance_before =
			<AssetHubPolkadot as AssetHubPolkadotPallet>::Balances::free_balance(
				AssetHubPolkadotReceiver::get(),
			);
		// Send the Weth back to Ethereum
		assert_ok!(
			<AssetHubPolkadot as AssetHubPolkadotPallet>::PolkadotXcm::reserve_transfer_assets(
				RuntimeOrigin::signed(AssetHubPolkadotReceiver::get()),
				Box::new(destination),
				Box::new(beneficiary),
				Box::new(multi_assets),
				0,
			)
		);

		let free_balance_after =
			<AssetHubPolkadot as AssetHubPolkadotPallet>::Balances::free_balance(
				AssetHubPolkadotReceiver::get(),
			);
		// Assert at least DefaultBridgeHubEthereumBaseFee charged from the sender
		let free_balance_diff = free_balance_before - free_balance_after;
		assert!(free_balance_diff > base_fee);
	});

	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;
		// Check that the transfer token back to Ethereum message was queue in the Ethereum
		// Outbound Queue
		assert_expected_events!(
			BridgeHubPolkadot,
			vec![
				RuntimeEvent::EthereumOutboundQueue(snowbridge_pallet_outbound_queue::Event::MessageQueued {..}) => {},
			]
		);

		// check treasury account balance on BH after (should receive some fees)
		let treasury_account_after = <<BridgeHubPolkadot as BridgeHubPolkadotPallet>::Balances as frame_support::traits::fungible::Inspect<_>>::balance(&RelayTreasuryPalletAccount::get());
		let local_fee = treasury_account_after - treasury_account_before;

		let events = BridgeHubPolkadot::events();
		// Check that the local fee was credited to the Snowbridge sovereign account
		assert!(
			events.iter().any(|event| matches!(
				event,
				RuntimeEvent::Balances(pallet_balances::Event::Minted { who, amount })
					if *who == RelayTreasuryPalletAccount::get() && *amount == local_fee
			)),
			"Snowbridge sovereign takes local fee."
		);
		// Check that the remote delivery fee was credited to the AssetHub sovereign account
		assert!(
			events.iter().any(|event| matches!(
				event,
				RuntimeEvent::Balances(pallet_balances::Event::Minted { who, .. })
					if *who == assethub_sovereign,
			)),
			"AssetHub sovereign takes remote fee."
		);
	});
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

		// Construct RegisterToken message and sent to inbound queue
		let message = make_register_token_with_infufficient_fee_message();
		assert_ok!(send_inbound_message(message));

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
			id: v3::Location::default(),
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
	GlobalConsensusEthereumConvertsFor::<AccountId>::convert_location(&origin_location).unwrap()
}

fn make_register_token_message() -> InboundQueueFixture {
	InboundQueueFixture {
		execution_header: CompactExecutionHeader{
			parent_hash: hex!("d5de3dd02c96dbdc8aaa4db70a1e9fdab5ded5f4d52f18798acd56a3d37d1ad6").into(),
			block_number: 772,
			state_root: hex!("49cba2a79b23ad74cefe80c3a96699825d1cda0f75bfceb587c5549211c86245").into(),
			receipts_root: hex!("ac9cf067acc72a958a0d7c572c7b66ba6e232f65bbbd09078d7c7123f87ede64").into(),
		},
		message: Message {
			event_log: 	Log {
				address: hex!("eda338e4dc46038493b885327842fd3e301cab39").into(),
				topics: vec![
					hex!("7153f9357c8ea496bba60bf82e67143e27b64462b49041f8e689e1b05728f84f").into(),
					hex!("c173fac324158e77fb5840738a1a541f633cbec8884c6a601c567d2b376a0539").into(),
					hex!("5f7060e971b0dc81e63f0aa41831091847d97c1a4693ac450cc128c7214e65e0").into(),
				],
				data: hex!("00000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000002e0001000000000000000087d1f7fdfee7f651fabc8bfcb6e086c278b77a7d00e40b54020000000000000000000000000000000000000000000000000000000000").into(),
			},
			proof: Proof {
				block_hash: hex!("392182a385b3a417e8ddea8b252953ee81e6ec0fb09d9056c96c89fbeb703a3f").into(),
				tx_index: 0,
				data: (vec![
					hex!("7b1f61b9714c080ef0be014e01657a15f45f0304b477beebc7ca5596c8033095").to_vec(),
				], vec![
					hex!("f9028e822080b9028802f90284018301d205b9010000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000080000000000000000000000000000004000000000080000000000000000000000000000000000010100000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000040004000000000000002000002000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000100000000000000000200000000000010f90179f85894eda338e4dc46038493b885327842fd3e301cab39e1a0f78bb28d4b1d7da699e5c0bc2be29c2b04b5aab6aacf6298fe5304f9db9c6d7ea000000000000000000000000087d1f7fdfee7f651fabc8bfcb6e086c278b77a7df9011c94eda338e4dc46038493b885327842fd3e301cab39f863a07153f9357c8ea496bba60bf82e67143e27b64462b49041f8e689e1b05728f84fa0c173fac324158e77fb5840738a1a541f633cbec8884c6a601c567d2b376a0539a05f7060e971b0dc81e63f0aa41831091847d97c1a4693ac450cc128c7214e65e0b8a000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000002e0001000000000000000087d1f7fdfee7f651fabc8bfcb6e086c278b77a7d00e40b54020000000000000000000000000000000000000000000000000000000000").to_vec(),
				]),
			},
		},
	}
}

fn make_send_token_message() -> InboundQueueFixture {
	InboundQueueFixture {
		execution_header: CompactExecutionHeader{
			parent_hash: hex!("920cecde45d428e3a77590b70f8533cf4c2c36917b8a7b74c915e7fa3dae7075").into(),
			block_number: 1148,
			state_root: hex!("bbc6ba0e9940d641afecbbaf3f97abd2b9ffaf2f6bd4879c4a71e659eca89978").into(),
			receipts_root: hex!("717d6f476c17511fe96543b914cf08f19352567e10188f7f6c6c2f4528806c9c").into(),
		},
		message: Message {
			event_log: 	Log {
				address: hex!("eda338e4dc46038493b885327842fd3e301cab39").into(),
				topics: vec![
					hex!("7153f9357c8ea496bba60bf82e67143e27b64462b49041f8e689e1b05728f84f").into(),
					hex!("c173fac324158e77fb5840738a1a541f633cbec8884c6a601c567d2b376a0539").into(),
					hex!("c8eaf22f2cb07bac4679df0a660e7115ed87fcfd4e32ac269f6540265bbbd26f").into(),
				],
				data: hex!("00000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000005f0001000000000000000187d1f7fdfee7f651fabc8bfcb6e086c278b77a7d008eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48000064a7b3b6e00d000000000000000000e40b5402000000000000000000000000").into(),
			},
			proof: Proof {
				block_hash: hex!("d3c155f123c3cbff22f3d7869283e02179edea9ffa7a5e9a4d8414c2a6b8991f").into(),
				tx_index: 0,
				data: (vec![
					hex!("9f3340b57eddc1f86de30776db57faeca80269a3dd459031741988dec240ce34").to_vec(),
				], vec![
					hex!("f90451822080b9044b02f90447018301bcb9b9010000800000000000000000000020000000000000000000004000000000000000000400000000000000000000001000000010000000000000000000000008000000200000000000000001000008000000000000000000000000000000008000080000000000200000000000000000000000000100000000000000000011000000000000020200000000000000000000000000003000000040080008000000000000000000040044000021000000002000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000200800000000000f9033cf89b9487d1f7fdfee7f651fabc8bfcb6e086c278b77a7df863a0ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3efa000000000000000000000000090a987b944cb1dcce5564e5fdecd7a54d3de27fea000000000000000000000000057a2d4ff0c3866d96556884bf09fecdd7ccd530ca00000000000000000000000000000000000000000000000000de0b6b3a7640000f9015d94eda338e4dc46038493b885327842fd3e301cab39f884a024c5d2de620c6e25186ae16f6919eba93b6e2c1a33857cc419d9f3a00d6967e9a000000000000000000000000090a987b944cb1dcce5564e5fdecd7a54d3de27fea000000000000000000000000087d1f7fdfee7f651fabc8bfcb6e086c278b77a7da000000000000000000000000000000000000000000000000000000000000003e8b8c000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000de0b6b3a76400000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000208eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48f9013c94eda338e4dc46038493b885327842fd3e301cab39f863a07153f9357c8ea496bba60bf82e67143e27b64462b49041f8e689e1b05728f84fa0c173fac324158e77fb5840738a1a541f633cbec8884c6a601c567d2b376a0539a0c8eaf22f2cb07bac4679df0a660e7115ed87fcfd4e32ac269f6540265bbbd26fb8c000000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000005f0001000000000000000187d1f7fdfee7f651fabc8bfcb6e086c278b77a7d008eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48000064a7b3b6e00d000000000000000000e40b5402000000000000000000000000").to_vec(),
				]),
			},
		},
	}
}
