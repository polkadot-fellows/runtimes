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
use frame_support::traits::{fungible::Inspect as FungibleInspect, Hooks};
use parachains_common::AccountId;

/// KSM teleported into the relay's accumulation account is forwarded back to Asset Hub and burned
/// there, leaving Asset Hub's checking account and the relay's issuance in step.
#[test]
fn relay_accumulated_funds_are_burnt_on_asset_hub() {
	type RelayRuntime = <Kusama as Chain>::Runtime;
	type RelayEvent = <Kusama as Chain>::RuntimeEvent;
	type RelayBalances = pallet_balances::Pallet<RelayRuntime>;
	type AssetHubRuntime = <AssetHubKusama as Chain>::Runtime;
	type AssetHubEvent = <AssetHubKusama as Chain>::RuntimeEvent;
	type AssetHubBalances = pallet_balances::Pallet<AssetHubRuntime>;

	let accumulation_account: AccountId = Kusama::execute_with(|| {
		pallet_accumulate_and_forward::Pallet::<RelayRuntime>::accumulation_account()
	});
	let check_account: AccountId =
		AssetHubKusama::execute_with(pallet_xcm::Pallet::<AssetHubRuntime>::check_account);
	// Enough that the forward still clears `MinTransferAmount` after arrival fees.
	let teleported = 10 *
		Kusama::execute_with(|| {
			<RelayRuntime as pallet_accumulate_and_forward::Config>::MinTransferAmount::get()
		});

	let (asset_hub_issuance_before, check_balance_before) = AssetHubKusama::execute_with(|| {
		(AssetHubBalances::total_issuance(), AssetHubBalances::balance(&check_account))
	});

	// GIVEN a real teleport from Asset Hub into the accumulation account. The KSM moves into Asset
	// Hub's checking account, which is how it comes to sit on the relay at all.
	AssetHubKusama::execute_with(|| {
		let beneficiary: Location =
			AccountId32Junction { network: None, id: accumulation_account.clone().into() }.into();
		let assets: Assets = (Location::parent(), teleported).into();
		assert_ok!(pallet_xcm::Pallet::<AssetHubRuntime>::limited_teleport_assets(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(AssetHubKusamaSender::get()),
			bx!(Location::parent().into()),
			bx!(beneficiary.into()),
			bx!(assets.into()),
			0,
			WeightLimit::Unlimited,
		));
	});
	let accumulated = Kusama::execute_with(|| RelayBalances::balance(&accumulation_account));
	assert!(accumulated > 0, "the teleport should reach the accumulation account");
	let forwarded = accumulated - KUSAMA_ED;

	// WHEN the forward runs.
	Kusama::execute_with(|| {
		let period: u32 =
			<RelayRuntime as pallet_accumulate_and_forward::Config>::TransferPeriod::get();
		Dmp::make_parachain_reachable(AssetHubKusama::para_id());
		frame_system::Pallet::<RelayRuntime>::set_block_number(period);
		pallet_accumulate_and_forward::Pallet::<RelayRuntime>::on_idle(period, Weight::MAX);

		assert_expected_events!(
			Kusama,
			vec![RelayEvent::AccumulateForward(
				pallet_accumulate_and_forward::Event::ForwardSucceeded { .. }
			) => {},]
		);
		// Emptied down to the ED; the KSM has left the relay.
		assert_eq!(RelayBalances::balance(&accumulation_account), KUSAMA_ED);
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

	// AND the checking account keeps what stayed on the relay. Relay issuance isn't compared: the
	// relay pays its arrival fee via `ToAuthor`, which burns it when the emulator has no author.
	let check_balance_after =
		AssetHubKusama::execute_with(|| AssetHubBalances::balance(&check_account));
	assert_eq!(check_balance_after - check_balance_before, teleported - forwarded);
}
