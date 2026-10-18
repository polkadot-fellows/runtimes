// Copyright (C) Parity Technologies and the various Polkadot contributors, see Contributions.md
// for a list of specific contributors.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::*;
use cumulus_pallet_parachain_system::ValidationData;
use integration_tests_helpers::{
	frame_support::{
		traits::{fungible::Inspect as _, Hooks, OnInitialize},
		PalletId,
	},
	frame_system, pallet_accumulate_and_forward, pallet_balances, pallet_collator_selection,
	pallet_message_queue, pallet_xcm,
};
use kusama_runtime_constants::currency::UNITS;
use pallet_broker::{ConfigRecordOf, SaleInfo};
use sp_runtime::{traits::AccountIdConversion, Perbill};

/// A real coretime purchase pays revenue through `OnRevenue` into the accumulation account, which
/// is forwarded to Asset Hub and burned, leaving the checking account and this chain's issuance
/// in step.
#[test]
fn coretime_revenue_is_burnt_on_asset_hub() {
	type CoretimeRuntime = <CoretimeKusama as Chain>::Runtime;
	type CoretimeEvent = <CoretimeKusama as Chain>::RuntimeEvent;
	type CoretimeBalances = pallet_balances::Pallet<CoretimeRuntime>;
	type Broker = pallet_broker::Pallet<CoretimeRuntime>;
	type CoretimeSystem = frame_system::Pallet<CoretimeRuntime>;
	type AssetHubRuntime = <AssetHubKusama as Chain>::Runtime;
	type AssetHubEvent = <AssetHubKusama as Chain>::RuntimeEvent;
	type AssetHubBalances = pallet_balances::Pallet<AssetHubRuntime>;

	let buyer = CoretimeKusamaReceiver::get();
	let accumulation_account: AccountId = CoretimeKusama::execute_with(
		pallet_accumulate_and_forward::Pallet::<CoretimeRuntime>::accumulation_account,
	);
	let check_account: AccountId =
		AssetHubKusama::execute_with(pallet_xcm::Pallet::<AssetHubRuntime>::check_account);
	let teleported = 1_000 * UNITS;

	let (asset_hub_issuance_before, check_balance_before) = AssetHubKusama::execute_with(|| {
		(AssetHubBalances::total_issuance(), AssetHubBalances::balance(&check_account))
	});
	let coretime_issuance_before = CoretimeKusama::execute_with(CoretimeBalances::total_issuance);

	// GIVEN a buyer funded by a real teleport from Asset Hub.
	AssetHubKusama::execute_with(|| {
		let dest = AssetHubKusama::sibling_location_of(CoretimeKusama::para_id());
		let beneficiary: Location =
			AccountId32Junction { network: None, id: buyer.clone().into() }.into();
		let assets: Assets = (Location::parent(), teleported).into();
		assert_ok!(pallet_xcm::Pallet::<AssetHubRuntime>::limited_teleport_assets(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(AssetHubKusamaSender::get()),
			bx!(dest.into()),
			bx!(beneficiary.into()),
			bx!(assets.into()),
			0,
			WeightLimit::Unlimited,
		));
	});

	let forwarded = CoretimeKusama::execute_with(|| {
		// Hooks don't run in emulated tests, so drive the broker and the relay clock by hand.
		fn advance_to(block: &mut u32, target: u32) {
			while *block < target {
				*block += 1;
				CoretimeSystem::set_block_number(*block);
				let mut data =
					ValidationData::<CoretimeRuntime>::get().expect("set by the emulator");
				data.relay_parent_number = *block;
				ValidationData::<CoretimeRuntime>::put(data);
				<Broker as OnInitialize<_>>::on_initialize(*block);
			}
		}
		let mut block = CoretimeSystem::block_number()
			.max(ValidationData::<CoretimeRuntime>::get().map_or(0, |v| v.relay_parent_number));

		// WHEN the buyer purchases a region in a real sale.
		let config = ConfigRecordOf::<CoretimeRuntime> {
			advance_notice: 1,
			interlude_length: 1,
			leadin_length: 2,
			region_length: 1,
			ideal_bulk_proportion: Perbill::from_percent(100),
			limit_cores_offered: None,
			renewal_bump: Perbill::from_percent(3),
			contribution_timeout: 1,
		};
		let root = <CoretimeKusama as Chain>::RuntimeOrigin::root();
		assert_ok!(Broker::configure(root.clone(), config.clone()));
		assert_ok!(Broker::start_sales(root, 10 * UNITS, 1));
		let sale_start = SaleInfo::<CoretimeRuntime>::get().unwrap().sale_start;
		advance_to(&mut block, sale_start + config.interlude_length);

		let accumulated_before = CoretimeBalances::balance(&accumulation_account);
		let buyer_before = CoretimeBalances::balance(&buyer);
		assert_ok!(Broker::purchase(
			<CoretimeKusama as Chain>::RuntimeOrigin::signed(buyer.clone()),
			500 * UNITS,
		));
		let price = buyer_before - CoretimeBalances::balance(&buyer);
		assert!(price > 0, "the purchase pays revenue");
		assert_eq!(
			CoretimeBalances::balance(&accumulation_account),
			accumulated_before + price,
			"the revenue reaches the accumulation account"
		);
		let burn_account: AccountId = PalletId(*b"py/ctbrn").into_account_truncating();
		assert_eq!(CoretimeBalances::total_balance(&burn_account), 0, "nothing uses py/ctbrn");

		// AND the forward runs.
		let accumulated = CoretimeBalances::balance(&accumulation_account);
		let period =
			<CoretimeRuntime as pallet_accumulate_and_forward::Config>::TransferPeriod::get();
		let at = (block / period + 1) * period;
		CoretimeSystem::set_block_number(at);
		pallet_accumulate_and_forward::Pallet::<CoretimeRuntime>::on_idle(at, Weight::MAX);
		assert_expected_events!(
			CoretimeKusama,
			vec![CoretimeEvent::AccumulateForward(
				pallet_accumulate_and_forward::Event::ForwardSucceeded { .. }
			) => {},]
		);
		accumulated - CORETIME_KUSAMA_ED
	});

	// THEN Asset Hub burns it, less the execution fee paid to the collator pot. Read the fee from
	// its event, since collator payouts move the pot balance in the same block.
	AssetHubKusama::execute_with(|| {
		assert_expected_events!(
			AssetHubKusama,
			vec![AssetHubEvent::MessageQueue(
				pallet_message_queue::Event::Processed { success: true, .. }
			) => {},]
		);
		let staking_pot = pallet_collator_selection::Pallet::<AssetHubRuntime>::account_id();
		let fee = frame_system::Pallet::<AssetHubRuntime>::events()
			.iter()
			.find_map(|record| match &record.event {
				AssetHubEvent::Balances(pallet_balances::Event::Deposit { who, amount })
					if *who == staking_pot =>
					Some(*amount),
				_ => None,
			})
			.expect("the execution fee is deposited to the collator pot");
		let burned = asset_hub_issuance_before - AssetHubBalances::total_issuance();
		assert_eq!(burned + fee, forwarded, "all of it is either burned or paid as fee");
	});

	// AND the checking account still matches what this chain holds.
	let check_delta = AssetHubKusama::execute_with(|| AssetHubBalances::balance(&check_account)) -
		check_balance_before;
	let coretime_delta =
		CoretimeKusama::execute_with(CoretimeBalances::total_issuance) - coretime_issuance_before;
	assert_eq!(check_delta, coretime_delta, "the checking account tracks this chain's issuance");
	assert_eq!(check_delta, teleported - forwarded);
}

integration_tests_helpers::test_accumulated_funds_are_burnt_on_asset_hub!(
	CoretimeKusama,
	AssetHubKusama,
	CORETIME_KUSAMA_ED,
	ASSET_HUB_KUSAMA_ED,
);
