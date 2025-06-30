// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Tests for `WeighTrader` type of XCM Executor.

use asset_hub_polkadot_runtime::{
	xcm_config::{DotLocation, StakingPot, TrustBackedAssetsPalletLocation, XcmConfig},
	AllPalletsWithoutSystem, AssetConversion, Assets, Balances, ForeignAssets, Runtime,
	SessionKeys,
};
use asset_test_utils::ExtBuilder;
use assets_common::AssetIdForTrustBackedAssetsConvert;
use frame_support::{
	assert_ok,
	traits::{
		fungible::{Inspect, Mutate},
		fungibles::{Create, Inspect as FungiblesInspect, Mutate as FungiblesMutate},
	},
	weights::WeightToFee as WeightToFeeT,
};
use parachains_common::{AccountId, AssetHubPolkadotAuraId as AuraId};
use sp_runtime::traits::MaybeEquivalence;
use system_parachains_constants::polkadot::{currency::*, fee::WeightToFee};
use xcm::latest::prelude::*;
use xcm_executor::traits::WeightTrader;

const ALICE: [u8; 32] = [1u8; 32];
const SOME_ASSET_ADMIN: [u8; 32] = [5u8; 32];

type RuntimeHelper = asset_test_utils::RuntimeHelper<Runtime, AllPalletsWithoutSystem>;

#[test]
fn test_buy_and_refund_weight_with_native() {
	ExtBuilder::<Runtime>::default()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::ed25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			let bob: AccountId = SOME_ASSET_ADMIN.into();
			let staking_pot = StakingPot::get();
			let native_location = DotLocation::get();
			let initial_balance = 200 * UNITS;

			assert_ok!(Balances::mint_into(&bob, initial_balance));
			assert_ok!(Balances::mint_into(&staking_pot, initial_balance));

			// keep initial total issuance to assert later.
			let total_issuance = Balances::total_issuance();

			// prepare input to buy weight.
			let weight = Weight::from_parts(4_000_000_000, 0);
			let fee = WeightToFee::weight_to_fee(&weight);
			let extra_amount = 100;
			let ctx = XcmContext { origin: None, message_id: XcmHash::default(), topic: None };
			let payment: Asset = (native_location.clone(), fee + extra_amount).into();

			// init trader and buy weight.
			let mut trader = <XcmConfig as xcm_executor::Config>::Trader::new();
			let unused_asset =
				trader.buy_weight(weight, payment.into(), &ctx).expect("Expected Ok");

			// assert.
			let unused_amount =
				unused_asset.fungible.get(&native_location.clone().into()).map_or(0, |a| *a);
			assert_eq!(unused_amount, extra_amount);
			assert_eq!(Balances::total_issuance(), total_issuance);

			// prepare input to refund weight.
			let refund_weight = Weight::from_parts(1_000_000_000, 0);
			let refund = WeightToFee::weight_to_fee(&refund_weight);

			// refund.
			let actual_refund = trader.refund_weight(refund_weight, &ctx).unwrap();
			assert_eq!(actual_refund, (native_location, refund).into());

			// assert.
			assert_eq!(Balances::balance(&staking_pot), initial_balance);
			// only after `trader` is dropped we expect the fee to be resolved into the treasury
			// account.
			drop(trader);
			assert_eq!(Balances::balance(&staking_pot), initial_balance + fee - refund);
			assert_eq!(Balances::total_issuance(), total_issuance + fee - refund);
		})
}

#[test]
fn test_buy_and_refund_weight_with_swap_local_asset_xcm_trader() {
	ExtBuilder::<Runtime>::default()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::ed25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			let bob: AccountId = SOME_ASSET_ADMIN.into();
			let staking_pot = StakingPot::get();
			let asset_1: u32 = 1;
			let native_location = DotLocation::get();
			let asset_1_location = AssetIdForTrustBackedAssetsConvert::<
				TrustBackedAssetsPalletLocation,
				Location,
			>::convert_back(&asset_1)
			.unwrap();
			// bob's initial balance for native and `asset1` assets.
			let initial_balance = 200 * UNITS;
			// liquidity for both arms of (native, asset1) pool.
			let pool_liquidity = 100 * UNITS;

			// init asset, balances and pool.
			assert_ok!(<Assets as Create<_>>::create(asset_1, bob.clone(), true, 10));

			assert_ok!(Assets::mint_into(asset_1, &bob, initial_balance));
			assert_ok!(Balances::mint_into(&bob, initial_balance));
			assert_ok!(Balances::mint_into(&staking_pot, initial_balance));

			assert_ok!(AssetConversion::create_pool(
				RuntimeHelper::origin_of(bob.clone()),
				Box::new(native_location.clone()),
				Box::new(asset_1_location.clone())
			));

			assert_ok!(AssetConversion::add_liquidity(
				RuntimeHelper::origin_of(bob.clone()),
				Box::new(native_location.clone()),
				Box::new(asset_1_location.clone()),
				pool_liquidity,
				pool_liquidity,
				1,
				1,
				bob,
			));

			// keep initial total issuance to assert later.
			let asset_total_issuance = Assets::total_issuance(asset_1);
			let native_total_issuance = Balances::total_issuance();

			// prepare input to buy weight.
			let weight = Weight::from_parts(4_000_000_000, 0);
			let fee = WeightToFee::weight_to_fee(&weight);
			let asset_fee =
				AssetConversion::get_amount_in(&fee, &pool_liquidity, &pool_liquidity).unwrap();
			let extra_amount = 100;
			let ctx = XcmContext { origin: None, message_id: XcmHash::default(), topic: None };
			let asset_1_location_latest: Location = asset_1_location.clone();
			let payment: Asset = (asset_1_location_latest.clone(), asset_fee + extra_amount).into();

			// init trader and buy weight.
			let mut trader = <XcmConfig as xcm_executor::Config>::Trader::new();
			let unused_asset =
				trader.buy_weight(weight, payment.into(), &ctx).expect("Expected Ok");

			// assert.
			let unused_amount = unused_asset
				.fungible
				.get(&asset_1_location_latest.clone().into())
				.map_or(0, |a| *a);
			assert_eq!(unused_amount, extra_amount);
			assert_eq!(Assets::total_issuance(asset_1), asset_total_issuance + asset_fee);

			// prepare input to refund weight.
			let refund_weight = Weight::from_parts(1_000_000_000, 0);
			let refund = WeightToFee::weight_to_fee(&refund_weight);
			let (reserve1, reserve2) =
				AssetConversion::get_reserves(native_location, asset_1_location.clone()).unwrap();
			let asset_refund =
				AssetConversion::get_amount_out(&refund, &reserve1, &reserve2).unwrap();

			// refund.
			let actual_refund = trader.refund_weight(refund_weight, &ctx).unwrap();
			assert_eq!(actual_refund, (asset_1_location_latest, asset_refund).into());

			// assert.
			assert_eq!(Balances::balance(&staking_pot), initial_balance);
			// only after `trader` is dropped we expect the fee to be resolved into the treasury
			// account.
			drop(trader);
			assert_eq!(Balances::balance(&staking_pot), initial_balance + fee - refund);
			assert_eq!(
				Assets::total_issuance(asset_1),
				asset_total_issuance + asset_fee - asset_refund
			);
			assert_eq!(Balances::total_issuance(), native_total_issuance);
		})
}

#[test]
fn test_buy_and_refund_weight_with_swap_foreign_asset_xcm_trader() {
	ExtBuilder::<Runtime>::default()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::ed25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			let bob: AccountId = SOME_ASSET_ADMIN.into();
			let staking_pot = StakingPot::get();
			let native_location = DotLocation::get();
			let foreign_location =
				Location { parents: 1, interior: (Parachain(1234), GeneralIndex(12345)).into() };
			// bob's initial balance for native and `asset1` assets.
			let initial_balance = 200 * UNITS;
			// liquidity for both arms of (native, asset1) pool.
			let pool_liquidity = 100 * UNITS;

			// init asset, balances and pool.
			assert_ok!(<ForeignAssets as Create<_>>::create(
				foreign_location.clone(),
				bob.clone(),
				true,
				10
			));

			assert_ok!(ForeignAssets::mint_into(foreign_location.clone(), &bob, initial_balance));
			assert_ok!(Balances::mint_into(&bob, initial_balance));
			assert_ok!(Balances::mint_into(&staking_pot, initial_balance));

			assert_ok!(AssetConversion::create_pool(
				RuntimeHelper::origin_of(bob.clone()),
				Box::new(native_location.clone()),
				Box::new(foreign_location.clone())
			));

			assert_ok!(AssetConversion::add_liquidity(
				RuntimeHelper::origin_of(bob.clone()),
				Box::new(native_location.clone()),
				Box::new(foreign_location.clone()),
				pool_liquidity,
				pool_liquidity,
				1,
				1,
				bob,
			));

			// keep initial total issuance to assert later.
			let asset_total_issuance = ForeignAssets::total_issuance(foreign_location.clone());
			let native_total_issuance = Balances::total_issuance();

			// prepare input to buy weight.
			let weight = Weight::from_parts(4_000_000_000, 0);
			let fee = WeightToFee::weight_to_fee(&weight);
			let asset_fee =
				AssetConversion::get_amount_in(&fee, &pool_liquidity, &pool_liquidity).unwrap();
			let extra_amount = 100;
			let ctx = XcmContext { origin: None, message_id: XcmHash::default(), topic: None };
			let payment: Asset = (foreign_location.clone(), asset_fee + extra_amount).into();

			// init trader and buy weight.
			let mut trader = <XcmConfig as xcm_executor::Config>::Trader::new();
			let unused_asset =
				trader.buy_weight(weight, payment.into(), &ctx).expect("Expected Ok");

			// assert.
			let unused_amount =
				unused_asset.fungible.get(&foreign_location.clone().into()).map_or(0, |a| *a);
			assert_eq!(unused_amount, extra_amount);
			assert_eq!(
				ForeignAssets::total_issuance(foreign_location.clone()),
				asset_total_issuance + asset_fee
			);

			// prepare input to refund weight.
			let refund_weight = Weight::from_parts(1_000_000_000, 0);
			let refund = WeightToFee::weight_to_fee(&refund_weight);
			let (reserve1, reserve2) =
				AssetConversion::get_reserves(native_location, foreign_location.clone()).unwrap();
			let asset_refund =
				AssetConversion::get_amount_out(&refund, &reserve1, &reserve2).unwrap();

			// refund.
			let actual_refund = trader.refund_weight(refund_weight, &ctx).unwrap();
			let asset: Asset = (foreign_location.clone(), asset_refund).into();
			assert_eq!(actual_refund, asset);

			// assert.
			assert_eq!(Balances::balance(&staking_pot), initial_balance);
			// only after `trader` is dropped we expect the fee to be resolved into the treasury
			// account.
			drop(trader);
			assert_eq!(Balances::balance(&staking_pot), initial_balance + fee - refund);
			assert_eq!(
				ForeignAssets::total_issuance(foreign_location),
				asset_total_issuance + asset_fee - asset_refund
			);
			assert_eq!(Balances::total_issuance(), native_total_issuance);
		})
}
