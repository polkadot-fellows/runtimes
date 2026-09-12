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
use frame_support::traits::{
	fungible::{Inspect as FungibleInspect, Mutate as FungibleMutate},
	Hooks,
};

/// Coretime revenue and dust accumulate together and are burnt on Asset Hub.
#[test]
fn coretime_revenue_is_burnt_on_asset_hub() {
	type CoretimeRuntime = <CoretimeKusama as Chain>::Runtime;
	type CoretimeEvent = <CoretimeKusama as Chain>::RuntimeEvent;
	type AssetHubRuntime = <AssetHubKusama as Chain>::Runtime;
	type AssetHubEvent = <AssetHubKusama as Chain>::RuntimeEvent;

	let accumulation_account: AccountId = CoretimeKusama::execute_with(|| {
		pallet_accumulate_and_forward::Pallet::<CoretimeRuntime>::accumulation_account()
	});
	let amount = 2 * CoretimeKusama::execute_with(|| {
		<CoretimeRuntime as pallet_accumulate_and_forward::Config>::MinTransferAmount::get()
	});
	CoretimeKusama::fund_accounts(vec![(accumulation_account.clone(), amount)]);

	// The emulated chain minted that KSM itself, so top up the checking account.
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

	CoretimeKusama::execute_with(|| {
		let issuance_before = pallet_balances::Pallet::<CoretimeRuntime>::total_issuance();
		let period: u32 =
			<CoretimeRuntime as pallet_accumulate_and_forward::Config>::TransferPeriod::get();

		frame_system::Pallet::<CoretimeRuntime>::set_block_number(period);
		pallet_accumulate_and_forward::Pallet::<CoretimeRuntime>::on_idle(period, Weight::MAX);

		// Emptied down to the ED; the KSM has left this chain.
		assert_expected_events!(
			CoretimeKusama,
			vec![CoretimeEvent::AccumulateForward(
				pallet_accumulate_and_forward::Event::ForwardSucceeded { .. }
			) => {},]
		);
		let forwarded = amount - CORETIME_KUSAMA_ED;
		assert_eq!(
			pallet_balances::Pallet::<CoretimeRuntime>::balance(&accumulation_account),
			CORETIME_KUSAMA_ED
		);
		assert_eq!(
			pallet_balances::Pallet::<CoretimeRuntime>::total_issuance(),
			issuance_before - forwarded
		);
	});

	// Asset Hub burns it: issuance and checking account drop alike.
	AssetHubKusama::execute_with(|| {
		assert_expected_events!(
			AssetHubKusama,
			vec![AssetHubEvent::MessageQueue(
				pallet_message_queue::Event::Processed { success: true, .. }
			) => {},]
		);
		let forwarded = amount - CORETIME_KUSAMA_ED;
		assert_eq!(
			pallet_balances::Pallet::<AssetHubRuntime>::total_issuance(),
			asset_hub_issuance_before - forwarded
		);
		assert_eq!(
			pallet_balances::Pallet::<AssetHubRuntime>::balance(&check_account),
			check_balance_before - forwarded
		);
	});
}
