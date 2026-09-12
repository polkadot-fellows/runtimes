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
use frame_support::traits::{
	fungible::{Inspect as FungibleInspect, Mutate as FungibleMutate},
	Hooks,
};

/// Accumulated KSM is teleported to Asset Hub and burned there, where the network's
/// `TotalIssuance` is tracked.
#[test]
fn accumulated_funds_are_burnt_on_asset_hub() {
	type PeopleRuntime = <PeopleKusama as Chain>::Runtime;
	type PeopleEvent = <PeopleKusama as Chain>::RuntimeEvent;
	type AssetHubRuntime = <AssetHubKusama as Chain>::Runtime;
	type AssetHubEvent = <AssetHubKusama as Chain>::RuntimeEvent;

	// GIVEN more than `MinTransferAmount` sitting in the accumulation account.
	let accumulation_account: AccountId = PeopleKusama::execute_with(|| {
		pallet_accumulate_and_forward::Pallet::<PeopleRuntime>::accumulation_account()
	});
	let amount = 2 * PeopleKusama::execute_with(|| {
		<PeopleRuntime as pallet_accumulate_and_forward::Config>::MinTransferAmount::get()
	});
	PeopleKusama::fund_accounts(vec![(accumulation_account.clone(), amount)]);

	// The emulated chain minted that KSM itself, so top up the checking account as a real
	// teleport out would have.
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

	// WHEN the forward hook runs on a block that is a multiple of the transfer period.
	PeopleKusama::execute_with(|| {
		let issuance_before = pallet_balances::Pallet::<PeopleRuntime>::total_issuance();
		let period: u32 =
			<PeopleRuntime as pallet_accumulate_and_forward::Config>::TransferPeriod::get();

		frame_system::Pallet::<PeopleRuntime>::set_block_number(period);
		pallet_accumulate_and_forward::Pallet::<PeopleRuntime>::on_idle(period, Weight::MAX);

		// THEN the account is emptied down to the ED and the KSM has left this chain.
		assert_expected_events!(
			PeopleKusama,
			vec![PeopleEvent::AccumulateForward(
				pallet_accumulate_and_forward::Event::ForwardSucceeded { .. }
			) => {},]
		);
		let forwarded = amount - PEOPLE_KUSAMA_ED;
		assert_eq!(
			pallet_balances::Pallet::<PeopleRuntime>::balance(&accumulation_account),
			PEOPLE_KUSAMA_ED
		);
		assert_eq!(
			pallet_balances::Pallet::<PeopleRuntime>::total_issuance(),
			issuance_before - forwarded
		);
	});

	// AND Asset Hub burns it: issuance and checking account drop by the same amount.
	AssetHubKusama::execute_with(|| {
		assert_expected_events!(
			AssetHubKusama,
			vec![AssetHubEvent::MessageQueue(
				pallet_message_queue::Event::Processed { success: true, .. }
			) => {},]
		);
		let forwarded = amount - PEOPLE_KUSAMA_ED;
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
