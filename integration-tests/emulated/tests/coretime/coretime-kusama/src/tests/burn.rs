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
use frame_support::{
	traits::fungible::{Inspect as FungibleInspect, Mutate as FungibleMutate},
	PalletId,
};
use kusama_runtime_constants::{system_parachain::coretime::TIMESLICE_PERIOD, time::DAYS};
use pallet_broker::CoretimeInterface;
use sp_runtime::traits::AccountIdConversion;

/// Coretime sweeps its revenue holding account and burns it on Asset Hub.
#[test]
fn coretime_revenue_is_burnt_on_asset_hub() {
	type CoretimeRuntime = <CoretimeKusama as Chain>::Runtime;
	type CoretimeEvent = <CoretimeKusama as Chain>::RuntimeEvent;
	type AssetHubRuntime = <AssetHubKusama as Chain>::Runtime;
	type AssetHubEvent = <AssetHubKusama as Chain>::RuntimeEvent;

	// GIVEN a day of revenue in the holding account.
	let amount = 1_000 * CORETIME_KUSAMA_ED;
	let burn_account: AccountId = PalletId(*b"py/ctbrn").into_account_truncating();
	CoretimeKusama::fund_accounts(vec![(burn_account.clone(), amount)]);

	// The emulated Coretime chain minted that KSM itself, so Asset Hub's checking account never saw
	// it leave. Fund it as a real teleport out would have.
	let check_account: AccountId =
		AssetHubKusama::execute_with(pallet_xcm::Pallet::<AssetHubRuntime>::check_account);
	AssetHubKusama::execute_with(|| {
		assert_ok!(<pallet_balances::Pallet<AssetHubRuntime> as FungibleMutate<_>>::mint_into(
			&check_account,
			amount + ASSET_HUB_KUSAMA_ED,
		));
	});
	let (asset_hub_issuance_before, check_balance_before) = AssetHubKusama::execute_with(|| {
		(
			pallet_balances::Pallet::<AssetHubRuntime>::total_issuance(),
			pallet_balances::Pallet::<AssetHubRuntime>::balance(&check_account),
		)
	});

	// WHEN the daily sweep runs.
	CoretimeKusama::execute_with(|| {
		let issuance_before = pallet_balances::Pallet::<CoretimeRuntime>::total_issuance();

		<<CoretimeRuntime as pallet_broker::Config>::Coretime as CoretimeInterface>::on_new_timeslice(
			DAYS / TIMESLICE_PERIOD,
		);

		// THEN the holding account is emptied and the KSM has left this chain.
		assert_expected_events!(
			CoretimeKusama,
			vec![CoretimeEvent::Balances(pallet_balances::Event::Withdraw { who, amount: withdrawn }) => {
				who: *who == burn_account,
				withdrawn: *withdrawn == amount,
			},]
		);
		assert_eq!(pallet_balances::Pallet::<CoretimeRuntime>::balance(&burn_account), 0);
		assert_eq!(
			pallet_balances::Pallet::<CoretimeRuntime>::total_issuance(),
			issuance_before - amount
		);
	});

	// AND Asset Hub burns it: its issuance and its checking account drop by the same amount.
	AssetHubKusama::execute_with(|| {
		assert_expected_events!(
			AssetHubKusama,
			vec![AssetHubEvent::MessageQueue(
				pallet_message_queue::Event::Processed { success: true, .. }
			) => {},]
		);
		assert_eq!(
			pallet_balances::Pallet::<AssetHubRuntime>::total_issuance(),
			asset_hub_issuance_before - amount
		);
		assert_eq!(
			pallet_balances::Pallet::<AssetHubRuntime>::balance(&check_account),
			check_balance_before - amount
		);
	});
}
