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
use integration_tests_helpers::{
	frame_support::{
		traits::{
			fungible::{Balanced, Inspect as _, Mutate as _},
			Hooks, OnUnbalanced,
		},
		PalletId,
	},
	frame_system, pallet_accumulate_and_forward, pallet_balances, pallet_collator_selection,
	pallet_message_queue, pallet_xcm,
};
use sp_runtime::traits::AccountIdConversion;

/// Revenue paid through the broker's `OnRevenue` hook reaches the accumulation account and is
/// burnt on Asset Hub, with nothing left in the retired `py/ctbrn` account.
#[test]
fn coretime_revenue_is_burnt_on_asset_hub() {
	type CoretimeRuntime = <CoretimeKusama as Chain>::Runtime;
	type CoretimeEvent = <CoretimeKusama as Chain>::RuntimeEvent;
	type CoretimeBalances = pallet_balances::Pallet<CoretimeRuntime>;
	type AssetHubRuntime = <AssetHubKusama as Chain>::Runtime;
	type AssetHubEvent = <AssetHubKusama as Chain>::RuntimeEvent;
	type AssetHubBalances = pallet_balances::Pallet<AssetHubRuntime>;

	let accumulation_account: AccountId = CoretimeKusama::execute_with(
		pallet_accumulate_and_forward::Pallet::<CoretimeRuntime>::accumulation_account,
	);
	let revenue = 2 * CoretimeKusama::execute_with(|| {
		<CoretimeRuntime as pallet_accumulate_and_forward::Config>::MinTransferAmount::get()
	});

	// The emulated chain mints the revenue itself, so top up the checking account as the
	// teleport that brought it here would have.
	let check_account: AccountId =
		AssetHubKusama::execute_with(pallet_xcm::Pallet::<AssetHubRuntime>::check_account);
	AssetHubKusama::execute_with(|| {
		assert_ok!(AssetHubBalances::mint_into(&check_account, revenue + ASSET_HUB_KUSAMA_ED));
	});
	let (asset_hub_issuance_before, check_balance_before) = AssetHubKusama::execute_with(|| {
		(AssetHubBalances::total_issuance(), AssetHubBalances::balance(&check_account))
	});

	let forwarded = CoretimeKusama::execute_with(|| {
		// GIVEN revenue paid through the broker's hook.
		<CoretimeRuntime as pallet_broker::Config>::OnRevenue::on_unbalanced(
			CoretimeBalances::issue(revenue),
		);
		let accumulated = CoretimeBalances::balance(&accumulation_account);
		assert!(accumulated >= revenue, "revenue reaches the accumulation account");
		let burn_account: AccountId = PalletId(*b"py/ctbrn").into_account_truncating();
		assert_eq!(CoretimeBalances::total_balance(&burn_account), 0, "nothing uses py/ctbrn");

		// WHEN the forward runs.
		let issuance_before = CoretimeBalances::total_issuance();
		let period =
			<CoretimeRuntime as pallet_accumulate_and_forward::Config>::TransferPeriod::get();
		frame_system::Pallet::<CoretimeRuntime>::set_block_number(period);
		pallet_accumulate_and_forward::Pallet::<CoretimeRuntime>::on_idle(period, Weight::MAX);

		assert_expected_events!(
			CoretimeKusama,
			vec![CoretimeEvent::AccumulateForward(
				pallet_accumulate_and_forward::Event::ForwardSucceeded { .. }
			) => {},]
		);
		let forwarded = accumulated - CORETIME_KUSAMA_ED;
		assert_eq!(CoretimeBalances::balance(&accumulation_account), CORETIME_KUSAMA_ED);
		assert_eq!(CoretimeBalances::total_issuance(), issuance_before - forwarded);
		forwarded
	});

	// THEN Asset Hub burns it, less the execution fee paid to the collator pot. Read the fee
	// from its event, since collator payouts move the pot balance in the same block.
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
		assert_eq!(
			AssetHubBalances::balance(&check_account),
			check_balance_before - forwarded,
			"the checking account releases what came in"
		);
	});
}

integration_tests_helpers::test_accumulated_funds_are_burnt_on_asset_hub!(
	CoretimeKusama,
	AssetHubKusama,
	CORETIME_KUSAMA_ED,
	ASSET_HUB_KUSAMA_ED,
);
