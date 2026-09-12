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
use parachains_common::AccountId;

/// KSM accumulated on the relay is teleported to Asset Hub and burned where issuance is
/// tracked.
#[test]
fn relay_accumulated_funds_are_burnt_on_asset_hub() {
	type RelayRuntime = <Kusama as Chain>::Runtime;
	type RelayEvent = <Kusama as Chain>::RuntimeEvent;
	type AssetHubRuntime = <AssetHubKusama as Chain>::Runtime;
	type AssetHubEvent = <AssetHubKusama as Chain>::RuntimeEvent;

	let accumulation_account: AccountId = Kusama::execute_with(|| {
		pallet_accumulate_and_forward::Pallet::<RelayRuntime>::accumulation_account()
	});
	let amount = 2 * Kusama::execute_with(|| {
		<RelayRuntime as pallet_accumulate_and_forward::Config>::MinTransferAmount::get()
	});
	Kusama::fund_accounts(vec![(accumulation_account.clone(), amount)]);

	// The emulated relay minted that KSM itself, so top up the checking account.
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

	Kusama::execute_with(|| {
		let issuance_before = pallet_balances::Pallet::<RelayRuntime>::total_issuance();
		let period: u32 =
			<RelayRuntime as pallet_accumulate_and_forward::Config>::TransferPeriod::get();

		Dmp::make_parachain_reachable(AssetHubKusama::para_id());
		frame_system::Pallet::<RelayRuntime>::set_block_number(period);
		pallet_accumulate_and_forward::Pallet::<RelayRuntime>::on_idle(period, Weight::MAX);

		// Emptied down to the ED; the KSM has left the relay.
		assert_expected_events!(
			Kusama,
			vec![RelayEvent::AccumulateForward(
				pallet_accumulate_and_forward::Event::ForwardSucceeded { .. }
			) => {},]
		);
		let forwarded = amount - KUSAMA_ED;
		assert_eq!(
			pallet_balances::Pallet::<RelayRuntime>::balance(&accumulation_account),
			KUSAMA_ED
		);
		assert_eq!(
			pallet_balances::Pallet::<RelayRuntime>::total_issuance(),
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
		let forwarded = amount - KUSAMA_ED;
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
