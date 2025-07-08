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
use emulated_integration_tests_common::PenpalBTeleportableAssetLocation;
use frame_support::traits::fungibles::Mutate;
use hex_literal::hex;
use integration_tests_helpers::create_pool_with_native_on;
use polkadot_system_emulated_network::penpal_emulated_chain::{
	penpal_runtime::xcm_config::{CheckingAccount, TELEPORTABLE_ASSET_ID},
	PenpalAssetOwner,
};
use snowbridge_core::AssetMetadata;
use sp_core::H160;
use xcm_builder::ExternalConsensusLocationsConverterFor;
use xcm_executor::traits::ConvertLocation;

pub const INITIAL_FUND: u128 = 50000_000_000_000_0000;
pub const ETHEREUM_DESTINATION_ADDRESS: [u8; 20] = hex!("44a57ee2f2FCcb85FDa2B0B18EBD0D8D2333700e");
pub const AGENT_ADDRESS: [u8; 20] = hex!("90A987B944Cb1dCcE5564e5FDeCD7a54D3de27Fe");
pub const TOKEN_AMOUNT: u128 = 10_000_000_000_000_000;
pub const REMOTE_FEE_AMOUNT_IN_ETHER: u128 = 6_000_000_000_000_000;
pub const LOCAL_FEE_AMOUNT_IN_DOT: u128 = 800_000_000_00000;
pub const EXECUTION_WEIGHT: u64 = 80_000_000_0000;
const AH_BASE_FEE_V2: u128 = 100_000_000_000;
/// An ERC-20 token to be registered and sent.
pub const TOKEN_ID: [u8; 20] = hex!("8daebade922df735c38c80c7ebd708af50815faa");

pub fn beneficiary() -> Location {
	Location::new(0, [AccountKey20 { network: None, key: ETHEREUM_DESTINATION_ADDRESS.into() }])
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
			GlobalConsensus(EthereumNetwork::get().into()),
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
		xcm_config::LocationToAccountId::convert_location(&xcm_config::RootLocation::get())
			.unwrap()
			.into()
	});
	penpal_root_sovereign
}

pub fn snowbridge_sovereign() -> sp_runtime::AccountId32 {
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
		String::from("roc"),
		String::from("ROC"),
		10,
	);
}

/// Registers DOT as a native Polkadot asset on Snowbridge.
pub fn register_relay_token_on_polkadot_bh() {
	register_asset_native_polkadot_asset_on_snowbridge(
		Location::parent(),
		String::from("dot"),
		String::from("DOT"),
		12,
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

		// Register KSM on BH
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
	let ethereum_sovereign: AccountId = snowbridge_sovereign();
	PenpalB::execute_with(|| {
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::force_create(
			<PenpalB as Chain>::RuntimeOrigin::root(),
			weth_location().try_into().unwrap(),
			ethereum_sovereign.clone().into(),
			true,
			1,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::force_create(
			<PenpalB as Chain>::RuntimeOrigin::root(),
			eth_location().try_into().unwrap(),
			ethereum_sovereign.into(),
			true,
			1,
		));
	});
}

pub fn register_foreign_asset(id: Location, owner: AccountId, sufficient: bool) {
	AssetHubPolkadot::force_create_foreign_asset(id, owner, sufficient, ASSET_MIN_BALANCE, vec![]);
}

pub fn register_pal_on_polkadot_ah() {
	// Create PAL(i.e. native asset for penpal) on AH.
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

pub fn fund_on_penpal() {
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
	});
	PenpalB::execute_with(|| {
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
	});
	PenpalB::execute_with(|| {
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			weth_location().try_into().unwrap(),
			&PenpalBReceiver::get(),
			INITIAL_FUND,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			weth_location().try_into().unwrap(),
			&PenpalBSender::get(),
			INITIAL_FUND,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			weth_location().try_into().unwrap(),
			&sudo_account,
			INITIAL_FUND,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			eth_location().try_into().unwrap(),
			&PenpalBReceiver::get(),
			INITIAL_FUND,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			eth_location().try_into().unwrap(),
			&PenpalBSender::get(),
			INITIAL_FUND,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			eth_location().try_into().unwrap(),
			&sudo_account,
			INITIAL_FUND,
		));
	});
}

pub fn fund_on_ah() {
	AssetHubPolkadot::fund_accounts(vec![(AssetHubPolkadotSender::get(), INITIAL_FUND)]);
	AssetHubPolkadot::fund_accounts(vec![(AssetHubPolkadotReceiver::get(), INITIAL_FUND)]);

	let penpal_sovereign = AssetHubPolkadot::sovereign_account_id_of(
		AssetHubPolkadot::sibling_location_of(PenpalB::para_id()),
	);
	let penpal_user_sovereign = LocationToAccountId::convert_location(&Location::new(
		1,
		[
			Parachain(PenpalB::para_id().into()),
			AccountId32 { network: Some(NetworkId::Polkadot), id: PenpalBSender::get().into() },
		],
	))
	.unwrap();

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint_into(
			weth_location().try_into().unwrap(),
			&penpal_sovereign,
			INITIAL_FUND,
		));
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint_into(
			weth_location().try_into().unwrap(),
			&penpal_user_sovereign,
			INITIAL_FUND,
		));
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint_into(
			weth_location().try_into().unwrap(),
			&AssetHubPolkadotReceiver::get(),
			INITIAL_FUND,
		));
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint_into(
			weth_location().try_into().unwrap(),
			&AssetHubPolkadotSender::get(),
			INITIAL_FUND,
		));

		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint_into(
			eth_location().try_into().unwrap(),
			&penpal_sovereign,
			INITIAL_FUND,
		));
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint_into(
			eth_location().try_into().unwrap(),
			&penpal_user_sovereign,
			INITIAL_FUND,
		));
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint_into(
			eth_location().try_into().unwrap(),
			&AssetHubPolkadotReceiver::get(),
			INITIAL_FUND,
		));
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint_into(
			eth_location().try_into().unwrap(),
			&AssetHubPolkadotSender::get(),
			INITIAL_FUND,
		));
	});

	AssetHubPolkadot::fund_accounts(vec![(snowbridge_sovereign(), INITIAL_FUND)]);
	AssetHubPolkadot::fund_accounts(vec![(penpal_sovereign.clone(), INITIAL_FUND)]);
	AssetHubPolkadot::fund_accounts(vec![(penpal_user_sovereign.clone(), INITIAL_FUND)]);
}

pub fn create_pools_on_ah() {
	// We create a pool between DOT and ETH in AssetHub to support paying for fees with ETH.
	let ethereum_sovereign = snowbridge_sovereign();
	AssetHubPolkadot::fund_accounts(vec![(ethereum_sovereign.clone(), INITIAL_FUND)]);
	PenpalB::fund_accounts(vec![(ethereum_sovereign.clone(), INITIAL_FUND)]);
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint_into(
			eth_location().try_into().unwrap(),
			&ethereum_sovereign.clone(),
			50000_000_000_000_0000,
		));
	});
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint_into(
			weth_location().try_into().unwrap(),
			&ethereum_sovereign.clone(),
			50000_000_000_000_0000,
		));
	});
	create_pool_with_native_on!(
		AssetHubPolkadot,
		weth_location(),
		true,
		ethereum_sovereign.clone(),
		900_000_000_000,
		100_000_000_000_0000
	);
	create_pool_with_native_on!(
		AssetHubPolkadot,
		eth_location(),
		true,
		ethereum_sovereign.clone(),
		900_000_000_000,
		100_000_000_000_0000
	);
}

pub(crate) fn set_up_eth_and_dot_pool() {
	// We create a pool between DOT and WETH in AssetHub to support paying for fees with WETH.
	let ethereum_sovereign = snowbridge_sovereign();
	AssetHubPolkadot::fund_accounts(vec![(ethereum_sovereign.clone(), INITIAL_FUND)]);
	PenpalB::fund_accounts(vec![(ethereum_sovereign.clone(), INITIAL_FUND)]);
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint_into(
			eth_location().try_into().unwrap(),
			&ethereum_sovereign.clone(),
			500_000_000_000_000,
		));
	});
	create_pool_with_native_on!(
		AssetHubPolkadot,
		eth_location(),
		true,
		ethereum_sovereign.clone(),
		100_000_000_000,
		100_000_000_000_000
	);
}

pub(crate) fn set_up_eth_and_dot_pool_on_penpal() {
	let ethereum_sovereign = snowbridge_sovereign();
	AssetHubPolkadot::fund_accounts(vec![(ethereum_sovereign.clone(), 100_000_000_000_000)]);
	PenpalB::fund_accounts(vec![(ethereum_sovereign.clone(), INITIAL_FUND)]);
	PenpalB::execute_with(|| {
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			eth_location().try_into().unwrap(),
			&ethereum_sovereign.clone(),
			500_000_000_000_000,
		));
	});
	create_pool_with_native_on!(
		PenpalB,
		eth_location(),
		true,
		ethereum_sovereign.clone(),
		100_000_000_000,
		100_000_000_000_000
	);
}

pub(crate) fn set_up_eth_and_dot_pool_on_kusama() {
	let sa_of_pah_on_kah = AssetHubKusama::sovereign_account_of_parachain_on_other_global_consensus(
		NetworkId::Polkadot,
		AssetHubPolkadot::para_id(),
	);
	AssetHubKusama::execute_with(|| {
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint_into(
			eth_location().try_into().unwrap(),
			&sa_of_pah_on_kah.clone(),
			500_000_000_000_000,
		));
	});
	AssetHubKusama::fund_accounts(vec![(sa_of_pah_on_kah.clone(), INITIAL_FUND)]);
	create_pool_with_native_on!(
		AssetHubKusama,
		eth_location(),
		true,
		sa_of_pah_on_kah.clone(),
		100_000_000_000,
		100_000_000_000_000
	);
}

// set up pool
pub(crate) fn set_up_pool_with_wnd_on_ah_polkadot(
	asset: Location,
	is_foreign: bool,
	initial_fund: u128,
	initial_liquidity: u128,
) {
	let wnd: Location = Parent.into();
	AssetHubPolkadot::fund_accounts(vec![(AssetHubPolkadotSender::get(), initial_fund)]);
	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		let owner = AssetHubPolkadotSender::get();
		let signed_owner = <AssetHubPolkadot as Chain>::RuntimeOrigin::signed(owner.clone());

		if is_foreign {
			assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint(
				signed_owner.clone(),
				asset.clone().into(),
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
			Box::new(wnd.clone()),
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
			Box::new(wnd),
			Box::new(asset),
			initial_liquidity,
			initial_liquidity,
			1,
			1,
			owner.into()
		));
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::LiquidityAdded {..}) => {},
			]
		);
	});
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
