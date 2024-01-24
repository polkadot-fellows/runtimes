// This file is part of Cumulus.

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

//! Tests for the Kusama Asset Hub (previously known as Statemine) chain.

use asset_hub_kusama_runtime::{
	xcm_config::{
		bridging::{self, XcmBridgeHubRouterFeeAssetId},
		AssetFeeAsExistentialDepositMultiplierFeeCharger, CheckingAccount,
		ForeignCreatorsSovereignAccountOf, KsmLocation, LocationToAccountId, TreasuryAccount,
		TrustBackedAssetsPalletLocation, XcmConfig,
	},
	AllPalletsWithoutSystem, AssetDeposit, Assets, Balances, ExistentialDeposit, ForeignAssets,
	ForeignAssetsInstance, MetadataDepositBase, MetadataDepositPerByte, ParachainSystem, Runtime,
	RuntimeCall, RuntimeEvent, SessionKeys, ToPolkadotXcmRouterInstance, TrustBackedAssetsInstance,
	XcmpQueue,
};
use asset_test_utils::{
	test_cases_over_bridge::TestBridgingConfig, CollatorSessionKey, CollatorSessionKeys, ExtBuilder,
};
use codec::{Decode, Encode};
use cumulus_primitives_utility::ChargeWeightInFungibles;
use frame_support::{
	assert_noop, assert_ok,
	traits::fungibles::InspectEnumerable,
	weights::{Weight, WeightToFee as WeightToFeeT},
};
use parachains_common::{AccountId, AssetIdForTrustBackedAssets, AuraId, Balance};
use sp_runtime::traits::MaybeEquivalence;
use system_parachains_constants::kusama::fee::WeightToFee;
use xcm::latest::prelude::*;
use xcm_executor::traits::{Identity, JustTry, WeightTrader};

const ALICE: [u8; 32] = [1u8; 32];
const SOME_ASSET_ADMIN: [u8; 32] = [5u8; 32];

type AssetIdForTrustBackedAssetsConvert =
	assets_common::AssetIdForTrustBackedAssetsConvert<TrustBackedAssetsPalletLocation>;

type RuntimeHelper = asset_test_utils::RuntimeHelper<Runtime, AllPalletsWithoutSystem>;

fn collator_session_key(account: [u8; 32]) -> CollatorSessionKey<Runtime> {
	CollatorSessionKey::new(
		AccountId::from(account),
		AccountId::from(account),
		SessionKeys { aura: AuraId::from(sp_core::sr25519::Public::from_raw(account)) },
	)
}

fn collator_session_keys() -> CollatorSessionKeys<Runtime> {
	CollatorSessionKeys::new(
		AccountId::from(ALICE),
		AccountId::from(ALICE),
		SessionKeys { aura: AuraId::from(sp_core::sr25519::Public::from_raw(ALICE)) },
	)
}

#[test]
fn test_ed_is_one_hundredth_of_relay() {
	ExtBuilder::<Runtime>::default()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			let relay_ed = kusama_runtime_constants::currency::EXISTENTIAL_DEPOSIT;
			let ah_ed = ExistentialDeposit::get();
			assert_eq!(relay_ed / 100, ah_ed);
		});
}

#[test]
fn test_asset_xcm_trader() {
	ExtBuilder::<Runtime>::default()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			// We need root origin to create a sufficient asset
			let minimum_asset_balance = 3333333_u128;
			let local_asset_id = 1;
			assert_ok!(Assets::force_create(
				RuntimeHelper::root_origin(),
				local_asset_id.into(),
				AccountId::from(ALICE).into(),
				true,
				minimum_asset_balance
			));

			// We first mint enough asset for the account to exist for assets
			assert_ok!(Assets::mint(
				RuntimeHelper::origin_of(AccountId::from(ALICE)),
				local_asset_id.into(),
				AccountId::from(ALICE).into(),
				minimum_asset_balance
			));

			// get asset id as multilocation
			let asset_multilocation =
				AssetIdForTrustBackedAssetsConvert::convert_back(&local_asset_id).unwrap();

			// Set Alice as block author, who will receive fees
			RuntimeHelper::run_to_block(2, AccountId::from(ALICE));

			// We are going to buy 4e9 weight
			let bought = Weight::from_parts(4_000_000_000u64, 0);

			// Lets calculate amount needed
			let asset_amount_needed =
				AssetFeeAsExistentialDepositMultiplierFeeCharger::charge_weight_in_fungibles(
					local_asset_id,
					bought,
				)
				.expect("failed to compute");

			// Lets pay with: asset_amount_needed + asset_amount_extra
			let asset_amount_extra = 100_u128;
			let asset: MultiAsset =
				(asset_multilocation, asset_amount_needed + asset_amount_extra).into();

			let mut trader = <XcmConfig as xcm_executor::Config>::Trader::new();
			let ctx = XcmContext { origin: None, message_id: XcmHash::default(), topic: None };

			// Lets buy_weight and make sure buy_weight does not return an error
			let unused_assets = trader.buy_weight(bought, asset.into(), &ctx).expect("Expected Ok");
			// Check whether a correct amount of unused assets is returned
			assert_ok!(
				unused_assets.ensure_contains(&(asset_multilocation, asset_amount_extra).into())
			);

			// Drop trader
			drop(trader);

			// Make sure author(Alice) has received the amount
			assert_eq!(
				Assets::balance(local_asset_id, AccountId::from(ALICE)),
				minimum_asset_balance + asset_amount_needed
			);

			// We also need to ensure the total supply increased
			assert_eq!(
				Assets::total_supply(local_asset_id),
				minimum_asset_balance + asset_amount_needed
			);
		});
}

#[test]
fn test_asset_xcm_trader_with_refund() {
	ExtBuilder::<Runtime>::default()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			// We need root origin to create a sufficient asset
			// We set existential deposit to be identical to the one for Balances first
			assert_ok!(Assets::force_create(
				RuntimeHelper::root_origin(),
				1.into(),
				AccountId::from(ALICE).into(),
				true,
				ExistentialDeposit::get()
			));

			// We first mint enough asset for the account to exist for assets
			assert_ok!(Assets::mint(
				RuntimeHelper::origin_of(AccountId::from(ALICE)),
				1.into(),
				AccountId::from(ALICE).into(),
				ExistentialDeposit::get()
			));

			let mut trader = <XcmConfig as xcm_executor::Config>::Trader::new();
			let ctx = XcmContext { origin: None, message_id: XcmHash::default(), topic: None };

			// Set Alice as block author, who will receive fees
			RuntimeHelper::run_to_block(2, AccountId::from(ALICE));

			// We are going to buy 4e9 weight
			let bought = Weight::from_parts(4_000_000_000u64, 0);

			let asset_multilocation = AssetIdForTrustBackedAssetsConvert::convert_back(&1).unwrap();

			// lets calculate amount needed
			let amount_bought = WeightToFee::weight_to_fee(&bought);

			let asset: MultiAsset = (asset_multilocation, amount_bought).into();

			// Make sure buy_weight does not return an error
			assert_ok!(trader.buy_weight(bought, asset.clone().into(), &ctx));

			// Make sure again buy_weight does return an error
			// This assert relies on the fact, that we use `TakeFirstAssetTrader` in `WeightTrader`
			// tuple chain, which cannot be called twice
			assert_noop!(trader.buy_weight(bought, asset.into(), &ctx), XcmError::TooExpensive);

			// We actually use half of the weight
			let weight_used = bought / 2;

			// Make sure refurnd works.
			let amount_refunded = WeightToFee::weight_to_fee(&(bought - weight_used));

			assert_eq!(
				trader.refund_weight(bought - weight_used, &ctx),
				Some((asset_multilocation, amount_refunded).into())
			);

			// Drop trader
			drop(trader);

			// We only should have paid for half of the bought weight
			let fees_paid = WeightToFee::weight_to_fee(&weight_used);

			assert_eq!(
				Assets::balance(1, AccountId::from(ALICE)),
				ExistentialDeposit::get() + fees_paid
			);

			// We also need to ensure the total supply increased
			assert_eq!(Assets::total_supply(1), ExistentialDeposit::get() + fees_paid);
		});
}

#[test]
fn test_asset_xcm_trader_refund_not_possible_since_amount_less_than_ed() {
	ExtBuilder::<Runtime>::default()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			// We need root origin to create a sufficient asset
			// We set existential deposit to be identical to the one for Balances first
			assert_ok!(Assets::force_create(
				RuntimeHelper::root_origin(),
				1.into(),
				AccountId::from(ALICE).into(),
				true,
				ExistentialDeposit::get()
			));

			let mut trader = <XcmConfig as xcm_executor::Config>::Trader::new();
			let ctx = XcmContext { origin: None, message_id: XcmHash::default(), topic: None };

			// Set Alice as block author, who will receive fees
			RuntimeHelper::run_to_block(2, AccountId::from(ALICE));

			// We are going to buy small amount
			let bought = Weight::from_parts(50_000_000u64, 0);

			let asset_multilocation = AssetIdForTrustBackedAssetsConvert::convert_back(&1).unwrap();

			let amount_bought = WeightToFee::weight_to_fee(&bought);

			assert!(
				amount_bought < ExistentialDeposit::get(),
				"we are testing what happens when the amount does not exceed ED"
			);

			let asset: MultiAsset = (asset_multilocation, amount_bought).into();

			// Buy weight should return an error
			assert_noop!(trader.buy_weight(bought, asset.into(), &ctx), XcmError::TooExpensive);

			// not credited since the ED is higher than this value
			assert_eq!(Assets::balance(1, AccountId::from(ALICE)), 0);

			// We also need to ensure the total supply did not increase
			assert_eq!(Assets::total_supply(1), 0);
		});
}

#[test]
fn test_that_buying_ed_refund_does_not_refund() {
	ExtBuilder::<Runtime>::default()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			// We need root origin to create a sufficient asset
			// We set existential deposit to be identical to the one for Balances first
			assert_ok!(Assets::force_create(
				RuntimeHelper::root_origin(),
				1.into(),
				AccountId::from(ALICE).into(),
				true,
				ExistentialDeposit::get()
			));

			let mut trader = <XcmConfig as xcm_executor::Config>::Trader::new();
			let ctx = XcmContext { origin: None, message_id: XcmHash::default(), topic: None };

			// Set Alice as block author, who will receive fees
			RuntimeHelper::run_to_block(2, AccountId::from(ALICE));

			// We are gonna buy ED
			let bought = Weight::from_parts(ExistentialDeposit::get().try_into().unwrap(), 0);

			let asset_multilocation = AssetIdForTrustBackedAssetsConvert::convert_back(&1).unwrap();

			let amount_bought = WeightToFee::weight_to_fee(&bought);

			assert!(
				amount_bought < ExistentialDeposit::get(),
				"we are testing what happens when the amount does not exceed ED"
			);

			// We know we will have to buy at least ED, so lets make sure first it will
			// fail with a payment of less than ED
			let asset: MultiAsset = (asset_multilocation, amount_bought).into();
			assert_noop!(trader.buy_weight(bought, asset.into(), &ctx), XcmError::TooExpensive);

			// Now lets buy ED at least
			let asset: MultiAsset = (asset_multilocation, ExistentialDeposit::get()).into();

			// Buy weight should work
			assert_ok!(trader.buy_weight(bought, asset.into(), &ctx));

			// Should return None. We have a specific check making sure we dont go below ED for
			// drop payment
			assert_eq!(trader.refund_weight(bought, &ctx), None);

			// Drop trader
			drop(trader);

			// Make sure author(Alice) has received the amount
			assert_eq!(Assets::balance(1, AccountId::from(ALICE)), ExistentialDeposit::get());

			// We also need to ensure the total supply increased
			assert_eq!(Assets::total_supply(1), ExistentialDeposit::get());
		});
}

#[test]
fn test_asset_xcm_trader_not_possible_for_non_sufficient_assets() {
	ExtBuilder::<Runtime>::default()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			// Create a non-sufficient asset with specific existential deposit
			let minimum_asset_balance = 1_000_000_u128;
			assert_ok!(Assets::force_create(
				RuntimeHelper::root_origin(),
				1.into(),
				AccountId::from(ALICE).into(),
				false,
				minimum_asset_balance
			));

			// We first mint enough asset for the account to exist for assets
			assert_ok!(Assets::mint(
				RuntimeHelper::origin_of(AccountId::from(ALICE)),
				1.into(),
				AccountId::from(ALICE).into(),
				minimum_asset_balance
			));

			let mut trader = <XcmConfig as xcm_executor::Config>::Trader::new();
			let ctx = XcmContext { origin: None, message_id: XcmHash::default(), topic: None };

			// Set Alice as block author, who will receive fees
			RuntimeHelper::run_to_block(2, AccountId::from(ALICE));

			// We are going to buy 4e9 weight
			let bought = Weight::from_parts(4_000_000_000u64, 0);

			// lets calculate amount needed
			let asset_amount_needed = WeightToFee::weight_to_fee(&bought);

			let asset_multilocation = AssetIdForTrustBackedAssetsConvert::convert_back(&1).unwrap();

			let asset: MultiAsset = (asset_multilocation, asset_amount_needed).into();

			// Make sure again buy_weight does return an error
			assert_noop!(trader.buy_weight(bought, asset.into(), &ctx), XcmError::TooExpensive);

			// Drop trader
			drop(trader);

			// Make sure author(Alice) has NOT received the amount
			assert_eq!(Assets::balance(1, AccountId::from(ALICE)), minimum_asset_balance);

			// We also need to ensure the total supply NOT increased
			assert_eq!(Assets::total_supply(1), minimum_asset_balance);
		});
}

#[test]
fn test_assets_balances_api_works() {
	use assets_common::runtime_api::runtime_decl_for_fungibles_api::FungiblesApi;

	ExtBuilder::<Runtime>::default()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			let local_asset_id = 1;
			let foreign_asset_id_multilocation =
				MultiLocation { parents: 1, interior: X2(Parachain(1234), GeneralIndex(12345)) };

			// check before
			assert_eq!(Assets::balance(local_asset_id, AccountId::from(ALICE)), 0);
			assert_eq!(
				ForeignAssets::balance(foreign_asset_id_multilocation, AccountId::from(ALICE)),
				0
			);
			assert_eq!(Balances::free_balance(AccountId::from(ALICE)), 0);
			assert!(Runtime::query_account_balances(AccountId::from(ALICE))
				.unwrap()
				.try_as::<MultiAssets>()
				.unwrap()
				.is_none());

			// Drip some balance
			use frame_support::traits::fungible::Mutate;
			let some_currency = ExistentialDeposit::get();
			Balances::mint_into(&AccountId::from(ALICE), some_currency).unwrap();

			// We need root origin to create a sufficient asset
			let minimum_asset_balance = 3333333_u128;
			assert_ok!(Assets::force_create(
				RuntimeHelper::root_origin(),
				local_asset_id.into(),
				AccountId::from(ALICE).into(),
				true,
				minimum_asset_balance
			));

			// We first mint enough asset for the account to exist for assets
			assert_ok!(Assets::mint(
				RuntimeHelper::origin_of(AccountId::from(ALICE)),
				local_asset_id.into(),
				AccountId::from(ALICE).into(),
				minimum_asset_balance
			));

			// create foreign asset
			let foreign_asset_minimum_asset_balance = 3333333_u128;
			assert_ok!(ForeignAssets::force_create(
				RuntimeHelper::root_origin(),
				foreign_asset_id_multilocation,
				AccountId::from(SOME_ASSET_ADMIN).into(),
				false,
				foreign_asset_minimum_asset_balance
			));

			// We first mint enough asset for the account to exist for assets
			assert_ok!(ForeignAssets::mint(
				RuntimeHelper::origin_of(AccountId::from(SOME_ASSET_ADMIN)),
				foreign_asset_id_multilocation,
				AccountId::from(ALICE).into(),
				6 * foreign_asset_minimum_asset_balance
			));

			// check after
			assert_eq!(
				Assets::balance(local_asset_id, AccountId::from(ALICE)),
				minimum_asset_balance
			);
			assert_eq!(
				ForeignAssets::balance(foreign_asset_id_multilocation, AccountId::from(ALICE)),
				6 * minimum_asset_balance
			);
			assert_eq!(Balances::free_balance(AccountId::from(ALICE)), some_currency);

			let result: MultiAssets = Runtime::query_account_balances(AccountId::from(ALICE))
				.unwrap()
				.try_into()
				.unwrap();
			assert_eq!(result.len(), 3);

			// check currency
			assert!(result.inner().iter().any(|asset| asset.eq(
				&assets_common::fungible_conversion::convert_balance::<KsmLocation, Balance>(
					some_currency
				)
				.unwrap()
			)));
			// check trusted asset
			assert!(result.inner().iter().any(|asset| asset.eq(&(
				AssetIdForTrustBackedAssetsConvert::convert_back(&local_asset_id).unwrap(),
				minimum_asset_balance
			)
				.into())));
			// check foreign asset
			assert!(result.inner().iter().any(|asset| asset.eq(&(
				Identity::convert_back(&foreign_asset_id_multilocation).unwrap(),
				6 * foreign_asset_minimum_asset_balance
			)
				.into())));
		});
}

asset_test_utils::include_teleports_for_native_asset_works!(
	Runtime,
	AllPalletsWithoutSystem,
	XcmConfig,
	CheckingAccount,
	WeightToFee,
	ParachainSystem,
	collator_session_keys(),
	ExistentialDeposit::get(),
	Box::new(|runtime_event_encoded: Vec<u8>| {
		match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
			Ok(RuntimeEvent::PolkadotXcm(event)) => Some(event),
			_ => None,
		}
	}),
	Box::new(|runtime_event_encoded: Vec<u8>| {
		match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
			Ok(RuntimeEvent::XcmpQueue(event)) => Some(event),
			_ => None,
		}
	}),
	1000
);

asset_test_utils::include_teleports_for_foreign_assets_works!(
	Runtime,
	AllPalletsWithoutSystem,
	XcmConfig,
	CheckingAccount,
	WeightToFee,
	ParachainSystem,
	ForeignCreatorsSovereignAccountOf,
	ForeignAssetsInstance,
	collator_session_keys(),
	ExistentialDeposit::get(),
	Box::new(|runtime_event_encoded: Vec<u8>| {
		match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
			Ok(RuntimeEvent::PolkadotXcm(event)) => Some(event),
			_ => None,
		}
	}),
	Box::new(|runtime_event_encoded: Vec<u8>| {
		match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
			Ok(RuntimeEvent::XcmpQueue(event)) => Some(event),
			_ => None,
		}
	})
);

asset_test_utils::include_asset_transactor_transfer_with_local_consensus_currency_works!(
	Runtime,
	XcmConfig,
	collator_session_keys(),
	ExistentialDeposit::get(),
	Box::new(|| {
		assert!(Assets::asset_ids().collect::<Vec<_>>().is_empty());
		assert!(ForeignAssets::asset_ids().collect::<Vec<_>>().is_empty());
	}),
	Box::new(|| {
		assert!(Assets::asset_ids().collect::<Vec<_>>().is_empty());
		assert!(ForeignAssets::asset_ids().collect::<Vec<_>>().is_empty());
	})
);

asset_test_utils::include_asset_transactor_transfer_with_pallet_assets_instance_works!(
	asset_transactor_transfer_with_trust_backed_assets_works,
	Runtime,
	XcmConfig,
	TrustBackedAssetsInstance,
	AssetIdForTrustBackedAssets,
	AssetIdForTrustBackedAssetsConvert,
	collator_session_keys(),
	ExistentialDeposit::get(),
	12345,
	Box::new(|| {
		assert!(ForeignAssets::asset_ids().collect::<Vec<_>>().is_empty());
	}),
	Box::new(|| {
		assert!(ForeignAssets::asset_ids().collect::<Vec<_>>().is_empty());
	})
);

asset_test_utils::include_asset_transactor_transfer_with_pallet_assets_instance_works!(
	asset_transactor_transfer_with_foreign_assets_works,
	Runtime,
	XcmConfig,
	ForeignAssetsInstance,
	MultiLocation,
	JustTry,
	collator_session_keys(),
	ExistentialDeposit::get(),
	MultiLocation { parents: 1, interior: X2(Parachain(1313), GeneralIndex(12345)) },
	Box::new(|| {
		assert!(Assets::asset_ids().collect::<Vec<_>>().is_empty());
	}),
	Box::new(|| {
		assert!(Assets::asset_ids().collect::<Vec<_>>().is_empty());
	})
);

asset_test_utils::include_create_and_manage_foreign_assets_for_local_consensus_parachain_assets_works!(
	Runtime,
	XcmConfig,
	WeightToFee,
	ForeignCreatorsSovereignAccountOf,
	ForeignAssetsInstance,
	MultiLocation,
	JustTry,
	collator_session_keys(),
	ExistentialDeposit::get(),
	AssetDeposit::get(),
	MetadataDepositBase::get(),
	MetadataDepositPerByte::get(),
	Box::new(|pallet_asset_call| RuntimeCall::ForeignAssets(pallet_asset_call).encode()),
	Box::new(|runtime_event_encoded: Vec<u8>| {
		match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
			Ok(RuntimeEvent::ForeignAssets(pallet_asset_event)) => Some(pallet_asset_event),
			_ => None,
		}
	}),
	Box::new(|| {
		assert!(Assets::asset_ids().collect::<Vec<_>>().is_empty());
		assert!(ForeignAssets::asset_ids().collect::<Vec<_>>().is_empty());
	}),
	Box::new(|| {
		assert!(Assets::asset_ids().collect::<Vec<_>>().is_empty());
		assert_eq!(ForeignAssets::asset_ids().collect::<Vec<_>>().len(), 1);
	})
);

fn bridging_to_asset_hub_polkadot() -> TestBridgingConfig {
	TestBridgingConfig {
		bridged_network: bridging::to_polkadot::PolkadotNetwork::get(),
		local_bridge_hub_para_id: bridging::SiblingBridgeHubParaId::get(),
		local_bridge_hub_location: bridging::SiblingBridgeHub::get(),
		bridged_target_location: bridging::to_polkadot::AssetHubPolkadot::get(),
	}
}

#[test]
fn limited_reserve_transfer_assets_for_native_asset_to_asset_hub_polkadot_works() {
	missing_asset_test_utils_test_cases_over_bridge::limited_reserve_transfer_assets_for_native_asset_works::<
		Runtime,
		AllPalletsWithoutSystem,
		XcmConfig,
		ParachainSystem,
		XcmpQueue,
		LocationToAccountId,
	>(
		collator_session_keys(),
		ExistentialDeposit::get(),
		AccountId::from(ALICE),
		Box::new(|runtime_event_encoded: Vec<u8>| {
			match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
				Ok(RuntimeEvent::PolkadotXcm(event)) => Some(event),
				_ => None,
			}
		}),
		Box::new(|runtime_event_encoded: Vec<u8>| {
			match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
				Ok(RuntimeEvent::XcmpQueue(event)) => Some(event),
				_ => None,
			}
		}),
		bridging_to_asset_hub_polkadot,
		WeightLimit::Unlimited,
		Some(XcmBridgeHubRouterFeeAssetId::get()),
		TreasuryAccount::get(),
	)
}

#[test]
fn receive_reserve_asset_deposited_roc_from_asset_hub_polkadot_works() {
	const BLOCK_AUTHOR_ACCOUNT: [u8; 32] = [13; 32];
	asset_test_utils::test_cases_over_bridge::receive_reserve_asset_deposited_from_different_consensus_works::<
			Runtime,
			AllPalletsWithoutSystem,
			XcmConfig,
			LocationToAccountId,
			ForeignAssetsInstance,
		>(
			collator_session_keys().add(collator_session_key(BLOCK_AUTHOR_ACCOUNT)),
			ExistentialDeposit::get(),
			AccountId::from([73; 32]),
			AccountId::from(BLOCK_AUTHOR_ACCOUNT),
			// receiving ROCs
			(MultiLocation { parents: 2, interior: X1(GlobalConsensus(Polkadot)) }, 1000000000000, 1_000_000_000),
			bridging_to_asset_hub_polkadot,
			(
				X1(PalletInstance(bp_bridge_hub_kusama::WITH_BRIDGE_KUSAMA_TO_POLKADOT_MESSAGES_PALLET_INDEX)),
				GlobalConsensus(Polkadot),
				X1(Parachain(1000))
			)
		)
}

#[test]
fn report_bridge_status_from_xcm_bridge_router_for_polkadot_works() {
	missing_asset_test_utils_test_cases_over_bridge::report_bridge_status_from_xcm_bridge_router_works::<
		Runtime,
		AllPalletsWithoutSystem,
		XcmConfig,
		LocationToAccountId,
		ToPolkadotXcmRouterInstance,
	>(
		collator_session_keys(),
		bridging_to_asset_hub_polkadot,
		|| Decode::decode(&mut &bp_asset_hub_kusama::CongestedMessage::get().encode()[..]).unwrap(),
		|| {
			Decode::decode(&mut &bp_asset_hub_kusama::UncongestedMessage::get().encode()[..])
				.unwrap()
		},
	)
}

#[test]
fn test_report_bridge_status_call_compatibility() {
	// if this test fails, make sure `bp_asset_hub_polkadot` has valid encoding
	assert_eq!(
		RuntimeCall::ToPolkadotXcmRouter(
			pallet_xcm_bridge_hub_router::Call::report_bridge_status {
				bridge_id: Default::default(),
				is_congested: true,
			}
		)
		.encode(),
		bp_asset_hub_kusama::Call::ToPolkadotXcmRouter(
			bp_asset_hub_kusama::XcmBridgeHubRouterCall::report_bridge_status {
				bridge_id: Default::default(),
				is_congested: true,
			}
		)
		.encode()
	)
}

#[test]
fn check_sane_weight_report_bridge_status() {
	use pallet_xcm_bridge_hub_router::WeightInfo;
	let actual = <Runtime as pallet_xcm_bridge_hub_router::Config<
			ToPolkadotXcmRouterInstance,
		>>::WeightInfo::report_bridge_status();
	let max_weight = bp_asset_hub_kusama::XcmBridgeHubRouterTransactCallMaxWeight::get();
	assert!(
		actual.all_lte(max_weight),
		"max_weight: {:?} should be adjusted to actual {:?}",
		max_weight,
		actual
	);
}

#[test]
fn change_xcm_bridge_hub_router_byte_fee_by_governance_works() {
	asset_test_utils::test_cases::change_storage_constant_by_governance_works::<
		Runtime,
		bridging::XcmBridgeHubRouterByteFee,
		Balance,
	>(
		collator_session_keys(),
		1000,
		Box::new(|call| RuntimeCall::System(call).encode()),
		|| {
			(
				bridging::XcmBridgeHubRouterByteFee::key().to_vec(),
				bridging::XcmBridgeHubRouterByteFee::get(),
			)
		},
		|old_value| {
			if let Some(new_value) = old_value.checked_add(1) {
				new_value
			} else {
				old_value.checked_sub(1).unwrap()
			}
		},
	)
}

// missing stuff from asset_test_utils::test_cases_over_bridge
// TODO: replace me with direct usages of `asset_test_utils` after deps are bumped to (at least) 1.4
mod missing_asset_test_utils_test_cases_over_bridge {
	use asset_test_utils::test_cases_over_bridge::TestBridgingConfig;
	use codec::Encode;
	use cumulus_primitives_core::XcmpMessageSource;
	use frame_support::{
		assert_ok,
		traits::{Currency, Get, OnFinalize, OnInitialize, OriginTrait, ProcessMessageError},
	};
	use frame_system::pallet_prelude::BlockNumberFor;
	use parachains_common::{AccountId, Balance};
	use parachains_runtimes_test_utils::{
		mock_open_hrmp_channel, AccountIdOf, BalanceOf, CollatorSessionKeys, ExtBuilder,
		RuntimeHelper, ValidatorIdOf, XcmReceivedFrom,
	};
	use sp_runtime::{traits::StaticLookup, Saturating};
	use xcm::{latest::prelude::*, VersionedMultiAssets};
	use xcm_builder::{CreateMatcher, MatchXcm};
	use xcm_executor::{
		traits::{ConvertLocation, TransactAsset},
		XcmExecutor,
	};

	/// Helper function to verify `xcm` contains all relevant instructions expected on destination
	/// chain as part of a reserve-asset-transfer.
	fn assert_matches_reserve_asset_deposited_instructions<RuntimeCall: sp_std::fmt::Debug>(
		xcm: &mut Xcm<RuntimeCall>,
		expected_reserve_assets_deposited: &MultiAssets,
		expected_beneficiary: &MultiLocation,
	) {
		let _ = xcm
			.0
			.matcher()
			.skip_inst_while(|inst| !matches!(inst, ReserveAssetDeposited(..)))
			.expect("no instruction ReserveAssetDeposited?")
			.match_next_inst(|instr| match instr {
				ReserveAssetDeposited(reserve_assets) => {
					assert_eq!(reserve_assets, expected_reserve_assets_deposited);
					Ok(())
				},
				_ => Err(ProcessMessageError::BadFormat),
			})
			.expect("expected instruction ReserveAssetDeposited")
			.match_next_inst(|instr| match instr {
				ClearOrigin => Ok(()),
				_ => Err(ProcessMessageError::BadFormat),
			})
			.expect("expected instruction ClearOrigin")
			.match_next_inst(|instr| match instr {
				BuyExecution { .. } => Ok(()),
				_ => Err(ProcessMessageError::BadFormat),
			})
			.expect("expected instruction BuyExecution")
			.match_next_inst(|instr| match instr {
				DepositAsset { assets: _, beneficiary } if beneficiary == expected_beneficiary =>
					Ok(()),
				_ => Err(ProcessMessageError::BadFormat),
			})
			.expect("expected instruction DepositAsset");
	}

	pub fn limited_reserve_transfer_assets_for_native_asset_works<
		Runtime,
		AllPalletsWithoutSystem,
		XcmConfig,
		HrmpChannelOpener,
		HrmpChannelSource,
		LocationToAccountId,
	>(
		collator_session_keys: CollatorSessionKeys<Runtime>,
		existential_deposit: BalanceOf<Runtime>,
		alice_account: AccountIdOf<Runtime>,
		unwrap_pallet_xcm_event: Box<dyn Fn(Vec<u8>) -> Option<pallet_xcm::Event<Runtime>>>,
		unwrap_xcmp_queue_event: Box<
			dyn Fn(Vec<u8>) -> Option<cumulus_pallet_xcmp_queue::Event<Runtime>>,
		>,
		prepare_configuration: fn() -> TestBridgingConfig,
		weight_limit: WeightLimit,
		maybe_paid_export_message: Option<AssetId>,
		delivery_fees_account: Option<AccountIdOf<Runtime>>,
	) where
		Runtime: frame_system::Config
			+ pallet_balances::Config
			+ pallet_session::Config
			+ pallet_xcm::Config
			+ parachain_info::Config
			+ pallet_collator_selection::Config
			+ cumulus_pallet_parachain_system::Config
			+ cumulus_pallet_xcmp_queue::Config,
		AllPalletsWithoutSystem:
			OnInitialize<BlockNumberFor<Runtime>> + OnFinalize<BlockNumberFor<Runtime>>,
		AccountIdOf<Runtime>: Into<[u8; 32]>,
		ValidatorIdOf<Runtime>: From<AccountIdOf<Runtime>>,
		BalanceOf<Runtime>: From<Balance>,
		<Runtime as pallet_balances::Config>::Balance: From<Balance> + Into<u128>,
		XcmConfig: xcm_executor::Config,
		LocationToAccountId: ConvertLocation<AccountIdOf<Runtime>>,
		<Runtime as frame_system::Config>::AccountId:
			Into<<<Runtime as frame_system::Config>::RuntimeOrigin as OriginTrait>::AccountId>,
		<<Runtime as frame_system::Config>::Lookup as StaticLookup>::Source:
			From<<Runtime as frame_system::Config>::AccountId>,
		<Runtime as frame_system::Config>::AccountId: From<AccountId>,
		HrmpChannelOpener: frame_support::inherent::ProvideInherent<
			Call = cumulus_pallet_parachain_system::Call<Runtime>,
		>,
		HrmpChannelSource: XcmpMessageSource,
	{
		let runtime_para_id = 1000;
		ExtBuilder::<Runtime>::default()
			.with_collators(collator_session_keys.collators())
			.with_session_keys(collator_session_keys.session_keys())
			.with_tracing()
			.with_safe_xcm_version(3)
			.with_para_id(runtime_para_id.into())
			.build()
			.execute_with(|| {
				let mut alice = [0u8; 32];
				alice[0] = 1;
				let included_head = RuntimeHelper::<Runtime, AllPalletsWithoutSystem>::run_to_block(
					2,
					AccountId::from(alice).into(),
				);

				// prepare bridge config
				let TestBridgingConfig {
					bridged_network,
					local_bridge_hub_para_id,
					bridged_target_location: target_location_from_different_consensus,
					..
				} = prepare_configuration();

				let reserve_account = LocationToAccountId::convert_location(
					&target_location_from_different_consensus,
				)
				.expect("Sovereign account for reserves");
				let balance_to_transfer = 1_000_000_000_000_u128;
				let native_asset = MultiLocation::parent();

				// open HRMP to bridge hub
				mock_open_hrmp_channel::<Runtime, HrmpChannelOpener>(
					runtime_para_id.into(),
					local_bridge_hub_para_id.into(),
					included_head,
					&alice,
				);

				// drip ED to account
				let alice_account_init_balance = existential_deposit + balance_to_transfer.into();
				let _ = <pallet_balances::Pallet<Runtime>>::deposit_creating(
					&alice_account,
					alice_account_init_balance,
				);
				// SA of target location needs to have at least ED, otherwise making reserve fails
				let _ = <pallet_balances::Pallet<Runtime>>::deposit_creating(
					&reserve_account,
					existential_deposit,
				);

				// we just check here, that user retains enough balance after withdrawal
				// and also we check if `balance_to_transfer` is more than `existential_deposit`,
				assert!(
					(<pallet_balances::Pallet<Runtime>>::free_balance(&alice_account) -
						balance_to_transfer.into()) >=
						existential_deposit
				);
				// SA has just ED
				assert_eq!(
					<pallet_balances::Pallet<Runtime>>::free_balance(&reserve_account),
					existential_deposit
				);

				let delivery_fees_account_balance_before = delivery_fees_account
					.as_ref()
					.map(|dfa| <pallet_balances::Pallet<Runtime>>::free_balance(dfa))
					.unwrap_or(0.into());

				// local native asset (pallet_balances)
				let asset_to_transfer = MultiAsset {
					fun: Fungible(balance_to_transfer.into()),
					id: Concrete(native_asset),
				};

				// destination is (some) account relative to the destination different consensus
				let target_destination_account = MultiLocation {
					parents: 0,
					interior: X1(AccountId32 {
						network: Some(bridged_network),
						id: sp_runtime::AccountId32::new([3; 32]).into(),
					}),
				};

				let assets_to_transfer = MultiAssets::from(asset_to_transfer);
				let mut expected_assets = assets_to_transfer.clone();
				let context = XcmConfig::UniversalLocation::get();
				expected_assets
					.reanchor(&target_location_from_different_consensus, context)
					.unwrap();

				let expected_beneficiary = target_destination_account;

				// Make sure sender has enough funds for paying delivery fees
				let handling_delivery_fees = {
					// Probable XCM with `ReserveAssetDeposited`.
					let mut expected_reserve_asset_deposited_message = Xcm(vec![
						ReserveAssetDeposited(MultiAssets::from(expected_assets.clone())),
						ClearOrigin,
						BuyExecution {
							fees: MultiAsset {
								id: Concrete(Default::default()),
								fun: Fungible(balance_to_transfer),
							},
							weight_limit: Unlimited,
						},
						DepositAsset {
							assets: Wild(AllCounted(1)),
							beneficiary: expected_beneficiary,
						},
						SetTopic([
							220, 188, 144, 32, 213, 83, 111, 175, 44, 210, 111, 19, 90, 165, 191,
							112, 140, 247, 192, 124, 42, 17, 153, 141, 114, 34, 189, 20, 83, 69,
							237, 173,
						]),
					]);
					assert_matches_reserve_asset_deposited_instructions(
						&mut expected_reserve_asset_deposited_message,
						&expected_assets,
						&expected_beneficiary,
					);

					// Call `SendXcm::validate` to get delivery fees.
					let (_, delivery_fees): (_, MultiAssets) = XcmConfig::XcmSender::validate(
						&mut Some(target_location_from_different_consensus),
						&mut Some(expected_reserve_asset_deposited_message),
					)
					.expect("validate passes");
					// Drip delivery fee to Alice account.
					let mut delivery_fees_added = false;
					for delivery_fee in delivery_fees.inner() {
						assert_ok!(<XcmConfig::AssetTransactor as TransactAsset>::deposit_asset(
							&delivery_fee,
							&MultiLocation {
								parents: 0,
								interior: X1(AccountId32 {
									network: None,
									id: alice_account.clone().into(),
								}),
							},
							None,
						));
						delivery_fees_added = true;
					}
					delivery_fees_added
				};

				// do pallet_xcm call reserve transfer
				assert_ok!(<pallet_xcm::Pallet<Runtime>>::limited_reserve_transfer_assets(
					RuntimeHelper::<Runtime, AllPalletsWithoutSystem>::origin_of(
						alice_account.clone()
					),
					Box::new(target_location_from_different_consensus.into_versioned()),
					Box::new(target_destination_account.into_versioned()),
					Box::new(VersionedMultiAssets::from(assets_to_transfer)),
					0,
					weight_limit,
				));

				// check events
				// check pallet_xcm attempted
				RuntimeHelper::<Runtime, AllPalletsWithoutSystem>::assert_pallet_xcm_event_outcome(
					&unwrap_pallet_xcm_event,
					|outcome| {
						assert_ok!(outcome.ensure_complete());
					},
				);

				// check that xcm was sent
				let xcm_sent_message_hash = <frame_system::Pallet<Runtime>>::events()
					.into_iter()
					.filter_map(|e| unwrap_xcmp_queue_event(e.event.encode()))
					.find_map(|e| match e {
						cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { message_hash } =>
							Some(message_hash),
						_ => None,
					});

				// read xcm
				let xcm_sent =
					RuntimeHelper::<HrmpChannelSource, AllPalletsWithoutSystem>::take_xcm(
						local_bridge_hub_para_id.into(),
					)
					.unwrap();
				assert_eq!(
					xcm_sent_message_hash,
					Some(xcm_sent.using_encoded(sp_io::hashing::blake2_256))
				);
				let mut xcm_sent: Xcm<()> = xcm_sent.try_into().expect("versioned xcm");

				// check sent XCM ExportMessage to BridgeHub

				// 1. check paid or unpaid
				if let Some(expected_fee_asset_id) = maybe_paid_export_message {
					xcm_sent
						.0
						.matcher()
						.match_next_inst(|instr| match instr {
							WithdrawAsset(_) => Ok(()),
							_ => Err(ProcessMessageError::BadFormat),
						})
						.expect("contains WithdrawAsset")
						.match_next_inst(|instr| match instr {
							BuyExecution { fees, .. } if fees.id.eq(&expected_fee_asset_id) =>
								Ok(()),
							_ => Err(ProcessMessageError::BadFormat),
						})
						.expect("contains BuyExecution")
				} else {
					xcm_sent
						.0
						.matcher()
						.match_next_inst(|instr| match instr {
							// first instruction could be UnpaidExecution (because we could have
							// explicit unpaid execution on BridgeHub)
							UnpaidExecution { weight_limit, check_origin }
								if weight_limit == &Unlimited && check_origin.is_none() =>
								Ok(()),
							_ => Err(ProcessMessageError::BadFormat),
						})
						.expect("contains UnpaidExecution")
				}
				// 2. check ExportMessage
				.match_next_inst(|instr| match instr {
					// next instruction is ExportMessage
					ExportMessage { network, destination, xcm: inner_xcm } => {
						assert_eq!(network, &bridged_network);
						let (_, target_location_junctions_without_global_consensus) =
							target_location_from_different_consensus
								.interior
								.split_global()
								.expect("split works");
						assert_eq!(
							destination,
							&target_location_junctions_without_global_consensus
						);
						assert_matches_reserve_asset_deposited_instructions(
							inner_xcm,
							&expected_assets,
							&expected_beneficiary,
						);
						Ok(())
					},
					_ => Err(ProcessMessageError::BadFormat),
				})
				.expect("contains ExportMessage");

				// check alice account decreased by balance_to_transfer
				assert_eq!(
					<pallet_balances::Pallet<Runtime>>::free_balance(&alice_account),
					alice_account_init_balance
						.saturating_sub(existential_deposit)
						.saturating_sub(balance_to_transfer.into())
				);

				// check reserve account increased by balance_to_transfer
				assert_eq!(
					<pallet_balances::Pallet<Runtime>>::free_balance(&reserve_account),
					existential_deposit + balance_to_transfer.into()
				);

				// check dedicated account increased by delivery fees (if configured)
				if handling_delivery_fees {
					if let Some(delivery_fees_account) = delivery_fees_account {
						let delivery_fees_account_balance_after =
							<pallet_balances::Pallet<Runtime>>::free_balance(
								&delivery_fees_account,
							);
						assert!(
							delivery_fees_account_balance_after >
								delivery_fees_account_balance_before
						);
					}
				}
			})
	}

	pub fn report_bridge_status_from_xcm_bridge_router_works<
		Runtime,
		AllPalletsWithoutSystem,
		XcmConfig,
		LocationToAccountId,
		XcmBridgeHubRouterInstance,
	>(
		collator_session_keys: CollatorSessionKeys<Runtime>,
		prepare_configuration: fn() -> TestBridgingConfig,
		congested_message: fn() -> Xcm<XcmConfig::RuntimeCall>,
		uncongested_message: fn() -> Xcm<XcmConfig::RuntimeCall>,
	) where
		Runtime: frame_system::Config
			+ pallet_balances::Config
			+ pallet_session::Config
			+ pallet_xcm::Config
			+ parachain_info::Config
			+ pallet_collator_selection::Config
			+ cumulus_pallet_parachain_system::Config
			+ cumulus_pallet_xcmp_queue::Config
			+ pallet_xcm_bridge_hub_router::Config<XcmBridgeHubRouterInstance>,
		AllPalletsWithoutSystem:
			OnInitialize<BlockNumberFor<Runtime>> + OnFinalize<BlockNumberFor<Runtime>>,
		AccountIdOf<Runtime>: Into<[u8; 32]>,
		ValidatorIdOf<Runtime>: From<AccountIdOf<Runtime>>,
		BalanceOf<Runtime>: From<Balance>,
		<Runtime as pallet_balances::Config>::Balance: From<Balance> + Into<u128>,
		XcmConfig: xcm_executor::Config,
		LocationToAccountId: ConvertLocation<AccountIdOf<Runtime>>,
		<Runtime as frame_system::Config>::AccountId:
			Into<<<Runtime as frame_system::Config>::RuntimeOrigin as OriginTrait>::AccountId>,
		<<Runtime as frame_system::Config>::Lookup as StaticLookup>::Source:
			From<<Runtime as frame_system::Config>::AccountId>,
		<Runtime as frame_system::Config>::AccountId: From<AccountId>,
		XcmBridgeHubRouterInstance: 'static,
	{
		ExtBuilder::<Runtime>::default()
		.with_collators(collator_session_keys.collators())
		.with_session_keys(collator_session_keys.session_keys())
		.with_tracing()
		.build()
		.execute_with(|| {
			let report_bridge_status = |is_congested: bool| {
				// prepare bridge config
				let TestBridgingConfig { local_bridge_hub_location, .. } = prepare_configuration();

				// Call received XCM execution
				let xcm = if is_congested { congested_message() } else { uncongested_message() };
				let hash = xcm.using_encoded(sp_io::hashing::blake2_256);

				// execute xcm as XcmpQueue would do
				let outcome = XcmExecutor::<XcmConfig>::execute_xcm(
					local_bridge_hub_location,
					xcm,
					hash,
					RuntimeHelper::<Runtime, AllPalletsWithoutSystem>::xcm_max_weight(XcmReceivedFrom::Sibling),
				);
				assert_eq!(outcome.ensure_complete(), Ok(()));
				assert_eq!(is_congested, pallet_xcm_bridge_hub_router::Pallet::<Runtime, XcmBridgeHubRouterInstance>::bridge().is_congested);
			};

			report_bridge_status(true);
			report_bridge_status(false);
		})
	}
}
