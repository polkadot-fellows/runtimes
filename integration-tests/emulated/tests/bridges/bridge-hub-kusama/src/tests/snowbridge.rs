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
use asset_hub_kusama_runtime::xcm_config::{
	bridging::to_ethereum::{BridgeHubEthereumBaseFee, EthereumNetwork},
	RelayTreasuryPalletAccount,
};
use bp_bridge_hub_kusama::snowbridge::CreateAssetCall;
use bridge_hub_kusama_runtime::{
	bridge_to_ethereum_config::EthereumGatewayAddress, EthereumBeaconClient, EthereumInboundQueue,
	Runtime, RuntimeOrigin,
};
use codec::{Decode, Encode};
use emulated_integration_tests_common::xcm_emulator::ConvertLocation;
use frame_support::pallet_prelude::TypeInfo;
use hex_literal::hex;
use kusama_system_emulated_network::{
	penpal_emulated_chain::CustomizableAssetFromSystemAssetHub,
	BridgeHubKusamaParaSender as BridgeHubKusamaSender,
};
use snowbridge_core::{
	gwei,
	inbound::{InboundQueueFixture, Log, Message, Proof},
	meth,
	outbound::OperatingMode,
	Rewards,
};
use snowbridge_pallet_system::PricingParametersOf;
use snowbridge_router_primitives::inbound::{
	Command, Destination, GlobalConsensusEthereumConvertsFor, MessageV1, VersionedMessage,
};
use sp_core::{H160, H256};
use sp_runtime::{DispatchError::Token, FixedU128, TokenError::FundsUnavailable};
use system_parachains_constants::kusama::currency::UNITS;

const INITIAL_FUND: u128 = 5_000_000_000 * KUSAMA_ED;
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
	EthereumBeaconClient::store_finalized_header(
		fixture.finalized_header,
		fixture.block_roots_root,
	)
	.unwrap();

	EthereumInboundQueue::submit(
		RuntimeOrigin::signed(BridgeHubKusamaSender::get()),
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
	BridgeHubKusama::fund_para_sovereign(origin_para.into(), INITIAL_FUND);

	let sudo_origin = <Kusama as Chain>::RuntimeOrigin::root();
	let destination = Kusama::child_location_of(BridgeHubKusama::para_id()).into();

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

	// Kusama Global Consensus
	// Send XCM message from Relay Chain to Bridge Hub source Parachain
	Kusama::execute_with(|| {
		assert_ok!(<Kusama as KusamaPallet>::XcmPallet::send(
			sudo_origin,
			bx!(destination),
			bx!(remote_xcm),
		));

		type RuntimeEvent = <Kusama as Chain>::RuntimeEvent;
		// Check that the Transact message was sent
		assert_expected_events!(
			Kusama,
			vec![
				RuntimeEvent::XcmPallet(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	BridgeHubKusama::execute_with(|| {
		type RuntimeEvent = <BridgeHubKusama as Chain>::RuntimeEvent;
		// Check that a message was sent to Ethereum to create the agent
		assert_expected_events!(
			BridgeHubKusama,
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
	BridgeHubKusama::fund_para_sovereign(origin_para.into(), INITIAL_FUND);

	let sudo_origin = <Kusama as Chain>::RuntimeOrigin::root();
	let destination: VersionedLocation =
		Kusama::child_location_of(BridgeHubKusama::para_id()).into();

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

	// Kusama Global Consensus
	// Send XCM message from Relay Chain to Bridge Hub source Parachain
	Kusama::execute_with(|| {
		assert_ok!(<Kusama as KusamaPallet>::XcmPallet::send(
			sudo_origin.clone(),
			bx!(destination.clone()),
			bx!(create_agent_xcm),
		));

		assert_ok!(<Kusama as KusamaPallet>::XcmPallet::send(
			sudo_origin,
			bx!(destination),
			bx!(create_channel_xcm),
		));

		type RuntimeEvent = <Kusama as Chain>::RuntimeEvent;

		assert_expected_events!(
			Kusama,
			vec![
				RuntimeEvent::XcmPallet(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	BridgeHubKusama::execute_with(|| {
		type RuntimeEvent = <BridgeHubKusama as Chain>::RuntimeEvent;

		// Check that the Channel was created
		assert_expected_events!(
			BridgeHubKusama,
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
	BridgeHubKusama::fund_para_sovereign(AssetHubKusama::para_id(), INITIAL_FUND);
	// Fund ethereum sovereign account on AssetHub.
	AssetHubKusama::fund_accounts(vec![(ethereum_sovereign_account(), INITIAL_FUND)]);

	BridgeHubKusama::execute_with(|| {
		type RuntimeEvent = <BridgeHubKusama as Chain>::RuntimeEvent;

		assert_ok!(<BridgeHubKusama as Chain>::System::set_storage(
			<BridgeHubKusama as Chain>::RuntimeOrigin::root(),
			vec![(EthereumGatewayAddress::key().to_vec(), H160(GATEWAY_ADDRESS).encode())],
		));
		// Construct RegisterToken message and sent to inbound queue
		let register_token_message = make_register_token_message();
		assert_ok!(send_inbound_message(register_token_message.clone()));

		assert_expected_events!(
			BridgeHubKusama,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;

		assert_expected_events!(
			AssetHubKusama,
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
	let asset_hub_sovereign = BridgeHubKusama::sovereign_account_id_of(Location::new(
		1,
		[Parachain(AssetHubKusama::para_id().into())],
	));

	// The Weth asset location, identified by the contract address on Ethereum
	let weth_asset_location: Location =
		(Parent, Parent, EthereumNetwork::get(), AccountKey20 { network: None, key: WETH }).into();
	// Converts the Weth asset location into an asset ID
	let weth_asset_id: v3::Location = weth_asset_location.clone().try_into().unwrap();

	// Fund ethereum sovereign on AssetHub
	AssetHubKusama::fund_accounts(vec![(ethereum_sovereign_account(), INITIAL_FUND)]);

	PenpalA::execute_with(|| {
		// Set the trusted asset location from AH, in this case, Ethereum.
		assert_ok!(<PenpalA as Chain>::System::set_storage(
			<PenpalA as Chain>::RuntimeOrigin::root(),
			vec![(
				CustomizableAssetFromSystemAssetHub::key().to_vec(),
				Location::new(2, [GlobalConsensus(Ethereum { chain_id: CHAIN_ID })]).encode(),
			)],
		));

		// Create asset on the Penpal parachain.
		assert_ok!(<PenpalA as PenpalAPallet>::ForeignAssets::create(
			<PenpalA as Chain>::RuntimeOrigin::signed(PenpalASender::get()),
			weth_asset_location.clone(),
			asset_hub_sovereign.clone().into(),
			1000,
		));

		assert!(<PenpalA as PenpalAPallet>::ForeignAssets::asset_exists(weth_asset_location));
	});

	AssetHubKusama::execute_with(|| {
		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets::force_create(
			<AssetHubKusama as Chain>::RuntimeOrigin::root(),
			weth_asset_id,
			asset_hub_sovereign.clone().into(),
			true,
			1000,
		));

		assert!(<AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets::asset_exists(
			weth_asset_id
		));
	});

	BridgeHubKusama::execute_with(|| {
		type RuntimeEvent = <BridgeHubKusama as Chain>::RuntimeEvent;

		type RuntimeOrigin = <BridgeHubKusama as Chain>::RuntimeOrigin;

		// Fund AssetHub sovereign account so it can pay execution fees for the asset transfer
		assert_ok!(<BridgeHubKusama as BridgeHubKusamaPallet>::Balances::force_set_balance(
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
					para_id: PenpalA::para_id().into(),
					id: PenpalAReceiver::get().into(),
					fee: 40_000_000_000,
				},
				amount: 1_000_000,
				fee: 40_000_000_000,
			},
		});
		// Convert the message to XCM
		let (xcm, _) = EthereumInboundQueue::do_convert(message_id, message).unwrap();
		// Send the XCM
		let _ = EthereumInboundQueue::send_xcm(xcm, AssetHubKusama::para_id()).unwrap();

		assert_expected_events!(
			BridgeHubKusama,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
		// Check that the assets were issued on AssetHub
		assert_expected_events!(
			AssetHubKusama,
			vec![
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { .. }) => {},
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	PenpalA::execute_with(|| {
		type RuntimeEvent = <PenpalA as Chain>::RuntimeEvent;
		// Check that the assets were issued on PenPal
		assert_expected_events!(
			PenpalA,
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
	BridgeHubKusama::fund_para_sovereign(AssetHubKusama::para_id(), INITIAL_FUND);
	// Fund ethereum sovereign account on AssetHub.
	AssetHubKusama::fund_accounts(vec![(ethereum_sovereign_account(), INITIAL_FUND)]);

	BridgeHubKusama::execute_with(|| {
		type RuntimeEvent = <BridgeHubKusama as Chain>::RuntimeEvent;

		assert_ok!(<BridgeHubKusama as Chain>::System::set_storage(
			<BridgeHubKusama as Chain>::RuntimeOrigin::root(),
			vec![(EthereumGatewayAddress::key().to_vec(), H160(GATEWAY_ADDRESS).encode())],
		));

		// Construct RegisterToken message and sent to inbound queue
		assert_ok!(send_inbound_message(make_register_token_message()));

		// Construct SendToken message and sent to inbound queue
		assert_ok!(send_inbound_message(make_send_token_message()));

		// Check that the message was sent
		assert_expected_events!(
			BridgeHubKusama,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;

		// Check that the token was received and issued as a foreign asset on AssetHub
		assert_expected_events!(
			AssetHubKusama,
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
	let assethub_sovereign = BridgeHubKusama::sovereign_account_id_of(Location::new(
		1,
		[Parachain(AssetHubKusama::para_id().into())],
	));

	AssetHubKusama::force_default_xcm_version(Some(XCM_VERSION));
	BridgeHubKusama::force_default_xcm_version(Some(XCM_VERSION));
	AssetHubKusama::force_xcm_version(
		Location::new(2, [GlobalConsensus(Ethereum { chain_id: CHAIN_ID })]),
		XCM_VERSION,
	);

	BridgeHubKusama::fund_accounts(vec![
		(assethub_sovereign.clone(), INITIAL_FUND),
		(RelayTreasuryPalletAccount::get(), INITIAL_FUND),
	]);
	AssetHubKusama::fund_accounts(vec![
		(AssetHubPolkadotReceiver::get(), INITIAL_FUND),
		(ethereum_sovereign_account(), INITIAL_FUND),
	]);

	const WETH_AMOUNT: u128 = 1_000_000_000;
	let base_fee = 2_750_872_500_000u128;

	AssetHubKusama::execute_with(|| {
		assert_ok!(<AssetHubKusama as Chain>::System::set_storage(
			<AssetHubKusama as Chain>::RuntimeOrigin::root(),
			vec![(BridgeHubEthereumBaseFee::key().to_vec(), base_fee.encode())],
		));
	});

	BridgeHubKusama::execute_with(|| {
		type RuntimeEvent = <BridgeHubKusama as Chain>::RuntimeEvent;

		assert_ok!(
			<BridgeHubKusama as BridgeHubKusamaPallet>::EthereumSystem::set_pricing_parameters(
				<BridgeHubKusama as Chain>::RuntimeOrigin::root(),
				PricingParametersOf::<Runtime> {
					exchange_rate: FixedU128::from_rational(1, 75),
					fee_per_gas: gwei(20),
					rewards: Rewards {
						local: (UNITS / 100), // 0.01 KSM
						remote: meth(1),
					},
					multiplier: FixedU128::from_rational(1, 1),
				}
			)
		);

		assert_ok!(<BridgeHubKusama as Chain>::System::set_storage(
			<BridgeHubKusama as Chain>::RuntimeOrigin::root(),
			vec![(EthereumGatewayAddress::key().to_vec(), H160(GATEWAY_ADDRESS).encode())],
		));

		// Construct RegisterToken message and sent to inbound queue
		assert_ok!(send_inbound_message(make_register_token_message()));

		// Check that the register token message was sent using xcm
		assert_expected_events!(
			BridgeHubKusama,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);

		// Construct SendToken message and sent to inbound queue
		assert_ok!(send_inbound_message(make_send_token_message()));

		// Check that the send token message was sent using xcm
		assert_expected_events!(
			BridgeHubKusama,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	// check treasury account balance on BH before
	let treasury_account_before = BridgeHubKusama::execute_with(|| {
		<<BridgeHubKusama as BridgeHubKusamaPallet>::Balances as frame_support::traits::fungible::Inspect<_>>::balance(&RelayTreasuryPalletAccount::get())
	});

	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
		type RuntimeOrigin = <AssetHubKusama as Chain>::RuntimeOrigin;

		// Check that AssetHub has issued the foreign asset
		assert_expected_events!(
			AssetHubKusama,
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

		let free_balance_before = <AssetHubKusama as AssetHubKusamaPallet>::Balances::free_balance(
			AssetHubKusamaReceiver::get(),
		);
		// Send the Weth back to Ethereum
		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::PolkadotXcm::reserve_transfer_assets(
			RuntimeOrigin::signed(AssetHubKusamaReceiver::get()),
			Box::new(destination),
			Box::new(beneficiary),
			Box::new(multi_assets),
			0,
		));

		let free_balance_after = <AssetHubKusama as AssetHubKusamaPallet>::Balances::free_balance(
			AssetHubKusamaReceiver::get(),
		);
		// Assert at least DefaultBridgeHubEthereumBaseFee charged from the sender
		let free_balance_diff = free_balance_before - free_balance_after;
		assert!(free_balance_diff > base_fee);
	});

	BridgeHubKusama::execute_with(|| {
		type RuntimeEvent = <BridgeHubKusama as Chain>::RuntimeEvent;
		// Check that the transfer token back to Ethereum message was queue in the Ethereum
		// Outbound Queue
		assert_expected_events!(
			BridgeHubKusama,
			vec![
				RuntimeEvent::EthereumOutboundQueue(snowbridge_pallet_outbound_queue::Event::MessageQueued {..}) => {},
			]
		);

		// check treasury account balance on BH after (should receive some fees)
		let treasury_account_after = <<BridgeHubKusama as BridgeHubKusamaPallet>::Balances as frame_support::traits::fungible::Inspect<_>>::balance(&RelayTreasuryPalletAccount::get());
		let local_fee = treasury_account_after - treasury_account_before;

		let events = BridgeHubKusama::events();
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
	BridgeHubKusama::fund_para_sovereign(AssetHubKusama::para_id(), INITIAL_FUND);

	BridgeHubKusama::execute_with(|| {
		type RuntimeEvent = <BridgeHubKusama as Chain>::RuntimeEvent;

		assert_ok!(<BridgeHubKusama as Chain>::System::set_storage(
			<BridgeHubKusama as Chain>::RuntimeOrigin::root(),
			vec![(EthereumGatewayAddress::key().to_vec(), H160(GATEWAY_ADDRESS).encode())],
		));

		// Construct RegisterToken message and sent to inbound queue
		let message = todo!(); // FAIL-CI @clara this does not exist anymore
					   // make_register_token_with_infufficient_fee_message();
		assert_ok!(send_inbound_message(message));

		assert_expected_events!(
			BridgeHubKusama,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;

		assert_expected_events!(
			AssetHubKusama,
			vec![
				RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success:false, .. }) => {},
			]
		);
	});
}

#[test]
fn send_token_from_ethereum_to_asset_hub_fail_for_insufficient_fund() {
	// Insufficient fund
	BridgeHubKusama::fund_para_sovereign(AssetHubKusama::para_id(), 1_000);

	BridgeHubKusama::execute_with(|| {
		assert_ok!(<BridgeHubKusama as Chain>::System::set_storage(
			<BridgeHubKusama as Chain>::RuntimeOrigin::root(),
			vec![(EthereumGatewayAddress::key().to_vec(), H160(GATEWAY_ADDRESS).encode())],
		));

		assert_err!(send_inbound_message(make_register_token_message()), Token(FundsUnavailable));
	});
}

/// Tests that the EthereumInboundQueue CreateAssetCall parameter on BridgeHub matches
/// the ForeignAssets::create call on AssetHub.
#[test]
fn asset_hub_foreign_assets_pallet_is_configured_correctly_in_bridge_hub() {
	let assethub_sovereign = BridgeHubKusama::sovereign_account_id_of(Location::new(
		1,
		[Parachain(AssetHubKusama::para_id().into())],
	));

	let call_create_foreign_assets =
		<AssetHubKusama as Chain>::RuntimeCall::ForeignAssets(pallet_assets::Call::<
			<AssetHubKusama as Chain>::Runtime,
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
	// FAIL-CI @clara
	todo!()
}

fn make_send_token_message() -> InboundQueueFixture {
	// FAIL-CI @clara
	todo!()
}
