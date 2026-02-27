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

use crate::{tests::snowbridge::CHAIN_ID, *};
use asset_hub_polkadot_runtime::xcm_config::{
	bridging::to_ethereum::BridgeHubEthereumBaseFeeV2, LocationToAccountId,
};
use bp_bridge_hub_polkadot::snowbridge::EthereumNetwork;
use emulated_integration_tests_common::{
	create_pool_with_native_on, PenpalBTeleportableAssetLocation,
};
use frame_support::traits::fungibles::Mutate;
use hex_literal::hex;
use polkadot_system_emulated_network::penpal_emulated_chain::{
	penpal_runtime::xcm_config::{CheckingAccount, TELEPORTABLE_ASSET_ID},
	PenpalAssetOwner,
};
use snowbridge_core::AssetMetadata;
use sp_core::H160;
use xcm_builder::ExternalConsensusLocationsConverterFor;
use xcm_executor::traits::ConvertLocation;

/// Initial fund in DOT to be used to prefund test and sovereign accounts.
pub const INITIAL_FUND: u128 = 50_000_000_000_000_000;
/// A beneficiary address on Ethereum.
pub const ETHEREUM_DESTINATION_ADDRESS: [u8; 20] = hex!("44a57ee2f2FCcb85FDa2B0B18EBD0D8D2333700e");
/// Agent on Ethereum address.
pub const AGENT_ADDRESS: [u8; 20] = hex!("90A987B944Cb1dCcE5564e5FDeCD7a54D3de27Fe");
/// A test ERC-20 token to be registered and sent.
pub const TOKEN_ID: [u8; 20] = hex!("8daebade922df735c38c80c7ebd708af50815faa");
/// ERC-20 token amount to be transferred.
pub const TOKEN_AMOUNT: u128 = 10_000_000_000_000_000;
/// The fee in ether to be sent
pub const REMOTE_FEE_AMOUNT_IN_ETHER: u128 = 6_000_000_000_000_000;
/// Local execution fee in DOT.
pub const LOCAL_FEE_AMOUNT_IN_DOT: u128 = 80_000_000_000_000;
/// Execution weight provided as limited for XCM execute.
pub const EXECUTION_WEIGHT: u64 = 800_000_000_000;
/// The execution fee (in Ether) for execution on AssetHub.
pub const EXECUTION_IN_ETHER: u128 = 1_500_000_000_000;
/// The reward allocated to the relayer for relaying the message.
pub const RELAYER_REWARD_IN_ETHER: u128 = 1_500_000_000_000;
/// The base cost for transfers to Ethereum, for Snowbridge V2.
const AH_BASE_FEE_V2: u128 = 100_000_000_000;
/// Amount of native to be provided for pool creation.
const DOT_POOL_AMOUNT: u128 = 900_000_000_000;
/// Amount of ether to be provided for pool creation.
const ETH_POOL_AMOUNT: u128 = 100_000_000_000_000;

pub fn beneficiary() -> Location {
	Location::new(0, [AccountKey20 { network: None, key: ETHEREUM_DESTINATION_ADDRESS }])
}

pub fn asset_hub() -> Location {
	Location::new(1, Parachain(AssetHubPolkadot::para_id().into()))
}

pub fn bridge_hub() -> Location {
	Location::new(1, Parachain(BridgeHubPolkadot::para_id().into()))
}

pub(crate) fn asset_hub_polkadot_location() -> Location {
	Location::new(
		2,
		[GlobalConsensus(NetworkId::Polkadot), Parachain(AssetHubPolkadot::para_id().into())],
	)
}
pub(crate) fn bridge_hub_polkadot_location() -> Location {
	Location::new(
		2,
		[GlobalConsensus(NetworkId::Polkadot), Parachain(BridgeHubPolkadot::para_id().into())],
	)
}

pub fn fund_on_bh() {
	let assethub_sovereign = BridgeHubPolkadot::sovereign_account_id_of(asset_hub());
	BridgeHubPolkadot::fund_accounts(vec![(assethub_sovereign.clone(), INITIAL_FUND)]);
}

pub fn weth_location() -> Location {
	erc20_token_location(WETH.into())
}

pub fn eth_location() -> Location {
	Location::new(2, [GlobalConsensus(Ethereum { chain_id: CHAIN_ID })])
}

pub fn erc20_token_location(token_id: H160) -> Location {
	Location::new(
		2,
		[
			GlobalConsensus(EthereumNetwork::get()),
			AccountKey20 { network: None, key: token_id.into() },
		],
	)
}

pub(crate) fn bridged_ksm_at_ah_polkadot() -> Location {
	Location::new(2, [GlobalConsensus(Kusama)])
}

pub fn penpal_root_sovereign() -> sp_runtime::AccountId32 {
	let penpal_root_sovereign: AccountId = PenpalB::execute_with(|| {
		use polkadot_system_emulated_network::penpal_emulated_chain::penpal_runtime::xcm_config;
		xcm_config::LocationToAccountId::convert_location(&xcm_config::RootLocation::get()).unwrap()
	});
	penpal_root_sovereign
}

pub fn ethereum_sovereign() -> sp_runtime::AccountId32 {
	use asset_hub_polkadot_runtime::xcm_config::UniversalLocation as AssetHubPolkadotUniversalLocation;
	AssetHubPolkadot::execute_with(|| {
		ExternalConsensusLocationsConverterFor::<
			AssetHubPolkadotUniversalLocation,
			[u8; 32],
		>::convert_location(&Location::new(
			2,
			[GlobalConsensus(EthereumNetwork::get())],
		))
			.unwrap()
			.into()
	})
}

/// Registers KSM as a native Polkadot asset on Snowbridge.
pub fn register_ksm_as_native_polkadot_asset_on_snowbridge() {
	register_asset_native_polkadot_asset_on_snowbridge(
		bridged_ksm_at_ah_polkadot(),
		String::from("ksm"),
		String::from("KSM"),
		12,
	);
}

/// Registers DOT as a native Polkadot asset on Snowbridge.
pub fn register_relay_token_on_polkadot_bh() {
	register_asset_native_polkadot_asset_on_snowbridge(
		Location::parent(),
		String::from("dot"),
		String::from("DOT"),
		10,
	);
}

/// Method to register a native asset on Snowbridge.
pub fn register_asset_native_polkadot_asset_on_snowbridge(
	asset_location: Location,
	name: String,
	symbol: String,
	decimals: u8,
) {
	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;
		type RuntimeOrigin = <BridgeHubPolkadot as Chain>::RuntimeOrigin;

		assert_ok!(<BridgeHubPolkadot as BridgeHubPolkadotPallet>::EthereumSystem::register_token(
			RuntimeOrigin::root(),
			Box::new(VersionedLocation::from(asset_location)),
			AssetMetadata {
				name: name.as_bytes().to_vec().try_into().unwrap(),
				symbol: symbol.as_bytes().to_vec().try_into().unwrap(),
				decimals,
			},
		));
		assert_expected_events!(
			BridgeHubPolkadot,
			vec![RuntimeEvent::EthereumSystem(snowbridge_pallet_system::Event::RegisterToken { .. }) => {},]
		);
	});
}

/// Register Ether and Weth on Penpal B.
pub fn register_ethereum_assets_on_penpal() {
	let ethereum_sovereign: AccountId = ethereum_sovereign();
	register_foreign_asset_on_penpal(weth_location(), ethereum_sovereign.clone(), true);
	register_foreign_asset_on_penpal(eth_location(), ethereum_sovereign, true);
}

/// Register a foreign asset on PenpalB.
pub fn register_foreign_asset_on_penpal(id: Location, owner: AccountId, sufficient: bool) {
	PenpalB::force_create_foreign_asset(id, owner, sufficient, ASSET_MIN_BALANCE, vec![]);
}

/// Registers a foreign asset on Polkadot AssetHub.
pub fn register_foreign_asset(id: Location, owner: AccountId, sufficient: bool) {
	AssetHubPolkadot::force_create_foreign_asset(
		id.clone(),
		owner.clone(),
		sufficient,
		ASSET_MIN_BALANCE,
		vec![],
	);
	AssetHubPolkadot::set_foreign_asset_reserves(id, owner, vec![(eth_location(), false).into()]);
}

/// Create PAL (native asset for penpal) on AH.
pub fn register_pal_on_polkadot_asset_hub() {
	AssetHubPolkadot::execute_with(|| {
		type RuntimeOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;
		let penpal_asset_id = Location::new(1, Parachain(PenpalB::para_id().into()));

		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::force_create(
			RuntimeOrigin::root(),
			penpal_asset_id.clone(),
			PenpalAssetOwner::get().into(),
			false,
			1_000_000,
		));

		assert!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::asset_exists(
			penpal_asset_id.clone(),
		));

		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint_into(
			penpal_asset_id.clone(),
			&AssetHubPolkadotReceiver::get(),
			TOKEN_AMOUNT,
		));

		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint_into(
			penpal_asset_id.clone(),
			&AssetHubPolkadotSender::get(),
			TOKEN_AMOUNT,
		));
	});
}

pub fn register_pal_on_polkadot_bh() {
	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;
		type RuntimeOrigin = <BridgeHubPolkadot as Chain>::RuntimeOrigin;

		assert_ok!(<BridgeHubPolkadot as BridgeHubPolkadotPallet>::EthereumSystem::register_token(
			RuntimeOrigin::root(),
			Box::new(VersionedLocation::from(PenpalBTeleportableAssetLocation::get())),
			AssetMetadata {
				name: "pal".as_bytes().to_vec().try_into().unwrap(),
				symbol: "pal".as_bytes().to_vec().try_into().unwrap(),
				decimals: 12,
			},
		));
		assert_expected_events!(
			BridgeHubPolkadot,
			vec![RuntimeEvent::EthereumSystem(snowbridge_pallet_system::Event::RegisterToken { .. }) => {},]
		);
	});
}

/// Fund all the accounts that need to funded for tests, on Penpal B.
pub fn prefund_accounts_on_penpal_b() {
	let sudo_account = penpal_root_sovereign();
	PenpalB::fund_accounts(vec![
		(PenpalBReceiver::get(), INITIAL_FUND),
		(PenpalBSender::get(), INITIAL_FUND),
		(CheckingAccount::get(), INITIAL_FUND),
		(sudo_account.clone(), INITIAL_FUND),
	]);
	PenpalB::execute_with(|| {
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			Location::parent(),
			&PenpalBReceiver::get(),
			INITIAL_FUND,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			Location::parent(),
			&PenpalBSender::get(),
			INITIAL_FUND,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			Location::parent(),
			&sudo_account,
			INITIAL_FUND,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::Assets::mint_into(
			TELEPORTABLE_ASSET_ID,
			&PenpalBReceiver::get(),
			INITIAL_FUND,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::Assets::mint_into(
			TELEPORTABLE_ASSET_ID,
			&PenpalBSender::get(),
			INITIAL_FUND,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::Assets::mint_into(
			TELEPORTABLE_ASSET_ID,
			&sudo_account,
			INITIAL_FUND,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			weth_location(),
			&PenpalBReceiver::get(),
			INITIAL_FUND,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			weth_location(),
			&PenpalBSender::get(),
			INITIAL_FUND,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			weth_location(),
			&sudo_account,
			INITIAL_FUND,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			eth_location(),
			&PenpalBReceiver::get(),
			INITIAL_FUND,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			eth_location(),
			&PenpalBSender::get(),
			INITIAL_FUND,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			eth_location(),
			&sudo_account,
			INITIAL_FUND,
		));
	});
}

/// Fund all the accounts that need to funded for tests, on Polkadot AssetHub.
pub fn prefund_accounts_on_polkadot_asset_hub() {
	AssetHubPolkadot::fund_accounts(vec![(AssetHubPolkadotSender::get(), INITIAL_FUND)]);
	AssetHubPolkadot::fund_accounts(vec![(AssetHubPolkadotReceiver::get(), INITIAL_FUND)]);

	let penpal_sovereign_on_pah = AssetHubPolkadot::sovereign_account_id_of(
		AssetHubPolkadot::sibling_location_of(PenpalB::para_id()),
	);
	let penpal_user_sovereign_on_pah = LocationToAccountId::convert_location(&Location::new(
		1,
		[
			Parachain(PenpalB::para_id().into()),
			AccountId32 { network: Some(NetworkId::Polkadot), id: PenpalBSender::get().into() },
		],
	))
	.unwrap();

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint_into(
			weth_location(),
			&penpal_sovereign_on_pah,
			INITIAL_FUND,
		));
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint_into(
			weth_location(),
			&penpal_user_sovereign_on_pah,
			INITIAL_FUND,
		));
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint_into(
			weth_location(),
			&AssetHubPolkadotReceiver::get(),
			INITIAL_FUND,
		));
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint_into(
			weth_location(),
			&AssetHubPolkadotSender::get(),
			INITIAL_FUND,
		));
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint_into(
			eth_location(),
			&penpal_sovereign_on_pah,
			INITIAL_FUND,
		));
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint_into(
			eth_location(),
			&penpal_user_sovereign_on_pah,
			INITIAL_FUND,
		));
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint_into(
			eth_location(),
			&AssetHubPolkadotReceiver::get(),
			INITIAL_FUND,
		));
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint_into(
			eth_location(),
			&AssetHubPolkadotSender::get(),
			INITIAL_FUND,
		));
	});

	AssetHubPolkadot::fund_accounts(vec![(ethereum_sovereign(), INITIAL_FUND)]);
	AssetHubPolkadot::fund_accounts(vec![(penpal_sovereign_on_pah.clone(), INITIAL_FUND)]);
	AssetHubPolkadot::fund_accounts(vec![(penpal_user_sovereign_on_pah.clone(), INITIAL_FUND)]);
}

/// Create a pool between DOT and ETH on Polkadot AssetHub to support paying for fees with ETH.
pub(crate) fn set_up_eth_and_dot_pool_on_polkadot_asset_hub() {
	set_up_foreign_asset_and_dot_pool_on_polkadot_asset_hub(eth_location());
}

/// Create a pool between DOT and a foreign asset on Polkadot AssetHub.
pub(crate) fn set_up_foreign_asset_and_dot_pool_on_polkadot_asset_hub(asset: Location) {
	let ethereum_sovereign = ethereum_sovereign();
	AssetHubPolkadot::fund_accounts(vec![(ethereum_sovereign.clone(), INITIAL_FUND)]);
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint_into(
			asset.clone(),
			&ethereum_sovereign.clone(),
			INITIAL_FUND,
		));
	});
	create_pool_with_native_on!(
		AssetHubPolkadot,
		asset,
		true,
		ethereum_sovereign.clone(),
		DOT_POOL_AMOUNT,
		ETH_POOL_AMOUNT
	);
}

/// Create a pool between DOT and ETH on Penpal to support paying for fees with ETH.
pub(crate) fn set_up_eth_and_dot_pool_on_penpal() {
	let ethereum_sovereign = ethereum_sovereign();
	PenpalB::fund_accounts(vec![(ethereum_sovereign.clone(), INITIAL_FUND)]);
	PenpalB::execute_with(|| {
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			eth_location(),
			&ethereum_sovereign.clone(),
			INITIAL_FUND,
		));
	});
	create_pool_with_native_on!(
		PenpalB,
		eth_location(),
		true,
		ethereum_sovereign.clone(),
		DOT_POOL_AMOUNT,
		ETH_POOL_AMOUNT
	);
}

/// Setup KSM and Ether pool on Kusama to pay for fees.
pub(crate) fn set_up_eth_and_ksm_pool_on_kusama_asset_hub() {
	let sa_of_pah_on_kah = AssetHubKusama::sovereign_account_of_parachain_on_other_global_consensus(
		NetworkId::Polkadot,
		AssetHubPolkadot::para_id(),
	);
	AssetHubKusama::execute_with(|| {
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint_into(
			eth_location(),
			&sa_of_pah_on_kah.clone(),
			INITIAL_FUND,
		));
	});
	AssetHubKusama::fund_accounts(vec![(sa_of_pah_on_kah.clone(), INITIAL_FUND)]);
	create_pool_with_native_on!(
		AssetHubKusama,
		eth_location(),
		true,
		sa_of_pah_on_kah.clone(),
		DOT_POOL_AMOUNT,
		ETH_POOL_AMOUNT
	);
}

/// Set the BridgeHubEthereumBaseFeeV2 storage item in the Polkadot AssetHub xcm config.
/// This is the minimum fee to send transactions from Polkadot AH to Ethereum.
pub fn set_bridge_hub_ethereum_base_fee() {
	AssetHubPolkadot::execute_with(|| {
		type RuntimeOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;

		assert_ok!(<AssetHubPolkadot as Chain>::System::set_storage(
			RuntimeOrigin::root(),
			vec![(BridgeHubEthereumBaseFeeV2::key().to_vec(), AH_BASE_FEE_V2.encode())],
		));
	});
}

/// Set the PenpalCustomizableAssetFromSystemAssetHub storage item to trust assets from
/// Ethereum.
pub fn set_trust_reserve_on_penpal() {
	PenpalB::execute_with(|| {
		assert_ok!(<PenpalB as Chain>::System::set_storage(
			<PenpalB as Chain>::RuntimeOrigin::root(),
			vec![(
				PenpalCustomizableAssetFromSystemAssetHub::key().to_vec(),
				Location::new(2, [GlobalConsensus(Ethereum { chain_id: CHAIN_ID })]).encode(),
			)],
		));
	});
}

/// Check that no assets were trapped on Polkadot AssetHub.
pub fn ensure_no_assets_trapped_on_pah() {
	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

		let events = AssetHubPolkadot::events();
		assert!(
			!events.iter().any(|event| matches!(
				event,
				RuntimeEvent::PolkadotXcm(pallet_xcm::Event::AssetsTrapped { .. })
			)),
			"Assets were trapped on Polkadot AssetHub, should not happen."
		);
	});
}

/// Check that no assets were trapped on Kusama AssetHub.
pub fn ensure_no_assets_trapped_on_kah() {
	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;

		let events = AssetHubKusama::events();
		assert!(
			!events.iter().any(|event| matches!(
				event,
				RuntimeEvent::PolkadotXcm(pallet_xcm::Event::AssetsTrapped { .. })
			)),
			"Assets were trapped on Kusama AssetHub, should not happen."
		);
	});
}

/// Check that no assets were trapped on Penpal B.
pub fn ensure_no_assets_trapped_on_penpal_b() {
	PenpalB::execute_with(|| {
		type RuntimeEvent = <PenpalB as Chain>::RuntimeEvent;

		let events = PenpalB::events();
		assert!(
			!events.iter().any(|event| matches!(
				event,
				RuntimeEvent::PolkadotXcm(pallet_xcm::Event::AssetsTrapped { .. })
			)),
			"Assets were trapped on PenpalB, should not happen."
		);
	});
}
