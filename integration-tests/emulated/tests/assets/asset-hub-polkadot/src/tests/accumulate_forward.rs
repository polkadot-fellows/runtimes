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
use emulated_integration_tests_common::dap_helpers::test_accumulate_forward_transfers_to_asset_hub;
use integration_tests_helpers::{
	frame_support::traits::fungible::Inspect as _, pallet_accumulate_and_forward, pallet_balances,
	pallet_xcm,
};
use polkadot_system_emulated_network::{
	BridgeHubPolkadotParaSender as BridgeHubPolkadotSender,
	BulletinPolkadotPara as BulletinPolkadot,
	CollectivesPolkadotParaSender as CollectivesPolkadotSender,
	CoretimePolkadotParaSender as CoretimePolkadotSender,
	PeoplePolkadotParaSender as PeoplePolkadotSender,
};

/// Every Polkadot chain with `pallet-accumulate-and-forward` forwards its accumulated funds to the
/// DAP staging account on Asset Hub.
#[test]
fn accumulated_funds_are_forwarded_to_the_dap() {
	Polkadot::execute_with(|| Dmp::make_parachain_reachable(AssetHubPolkadot::para_id()));
	test_accumulate_forward_transfers_to_asset_hub::<Polkadot, AssetHubPolkadot>();
	test_accumulate_forward_transfers_to_asset_hub::<BridgeHubPolkadot, AssetHubPolkadot>();
	test_accumulate_forward_transfers_to_asset_hub::<CollectivesPolkadot, AssetHubPolkadot>();
	test_accumulate_forward_transfers_to_asset_hub::<PeoplePolkadot, AssetHubPolkadot>();
	test_accumulate_forward_transfers_to_asset_hub::<CoretimePolkadot, AssetHubPolkadot>();
	test_accumulate_forward_transfers_to_asset_hub::<BulletinPolkadot, AssetHubPolkadot>();
}

/// A signed teleport from `$chain` pays a delivery fee into its accumulation account.
macro_rules! assert_delivery_fee_reaches_the_accumulation_account {
	( $chain:ident, $sender:ident, $dest:expr, $native:expr $(,)? ) => {{
		type ChainRuntime = <$chain as Chain>::Runtime;
		type ChainBalances = pallet_balances::Pallet<ChainRuntime>;

		let accumulation_account = $chain::execute_with(
			pallet_accumulate_and_forward::Pallet::<ChainRuntime>::accumulation_account,
		);
		let accumulated_before =
			$chain::execute_with(|| ChainBalances::balance(&accumulation_account));

		$chain::execute_with(|| {
			let beneficiary: Location =
				AccountId32Junction { network: None, id: AssetHubPolkadotReceiver::get().into() }
					.into();
			let amount =
				1000 * <ChainRuntime as pallet_balances::Config>::ExistentialDeposit::get();
			let assets: Assets = ($native, amount).into();
			assert_ok!(pallet_xcm::Pallet::<ChainRuntime>::limited_teleport_assets(
				<$chain as Chain>::RuntimeOrigin::signed($sender::get()),
				bx!($dest.into()),
				bx!(beneficiary.into()),
				bx!(assets.into()),
				0,
				WeightLimit::Unlimited,
			));
		});

		let accumulated_after =
			$chain::execute_with(|| ChainBalances::balance(&accumulation_account));
		assert!(
			accumulated_after > accumulated_before,
			"{}: the delivery fee should reach the accumulation account",
			stringify!($chain)
		);
	}};
}

/// XCM delivery fees on the relay and system chains go to the DAP.
#[test]
fn xcm_delivery_fees_reach_the_accumulation_account() {
	Polkadot::execute_with(|| Dmp::make_parachain_reachable(AssetHubPolkadot::para_id()));
	assert_delivery_fee_reaches_the_accumulation_account!(
		Polkadot,
		PolkadotSender,
		Polkadot::child_location_of(AssetHubPolkadot::para_id()),
		Here
	);
	let asset_hub = || Location::new(1, Parachain(AssetHubPolkadot::para_id().into()));
	assert_delivery_fee_reaches_the_accumulation_account!(
		BridgeHubPolkadot,
		BridgeHubPolkadotSender,
		asset_hub(),
		Parent
	);
	assert_delivery_fee_reaches_the_accumulation_account!(
		CollectivesPolkadot,
		CollectivesPolkadotSender,
		asset_hub(),
		Parent
	);
	assert_delivery_fee_reaches_the_accumulation_account!(
		CoretimePolkadot,
		CoretimePolkadotSender,
		asset_hub(),
		Parent
	);
	assert_delivery_fee_reaches_the_accumulation_account!(
		PeoplePolkadot,
		PeoplePolkadotSender,
		asset_hub(),
		Parent
	);
}
