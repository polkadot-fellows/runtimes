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

use frame_support::{
	assert_ok,
	traits::{fungible::Inspect, Get, Hooks},
	weights::Weight,
};
use parachains_common::{AccountId, Balance};
use xcm::latest::prelude::*;
use xcm_emulator::Parachain;

/// Asserts that funds teleported into a chain's accumulation account are forwarded back to Asset
/// Hub and burned there, leaving Asset Hub's checking account and the chain's issuance in step.
///
/// For Kusama-like chains, which burn. Polkadot chains forward to the DAP staging account instead
/// and are covered by the SDK's `dap_helpers::test_accumulate_forward_transfers_to_asset_hub`.
pub fn test_accumulated_funds_are_burnt_on_asset_hub<Sender, AH>(
	asset_hub_sender: AccountId,
	set_block_number: fn(u32),
) where
	Sender: Parachain,
	Sender::Runtime: pallet_accumulate_and_forward::Config
		+ pallet_balances::Config<Balance = Balance>
		+ frame_system::Config<AccountId = AccountId>,
	Sender::RuntimeEvent: TryInto<pallet_accumulate_and_forward::Event<Sender::Runtime>>,
	pallet_accumulate_and_forward::Pallet<Sender::Runtime>: Hooks<u32>,
	<Sender::Runtime as pallet_accumulate_and_forward::Config>::MinTransferAmount: Get<Balance>,
	<Sender::Runtime as pallet_accumulate_and_forward::Config>::TransferPeriod: Get<u32>,
	AH: Parachain,
	AH::Runtime: pallet_xcm::Config
		+ pallet_balances::Config<Balance = Balance>
		+ pallet_message_queue::Config
		+ pallet_collator_selection::Config
		+ frame_system::Config<AccountId = AccountId>,
	AH::RuntimeEvent: TryInto<pallet_message_queue::Event<AH::Runtime>>
		+ TryInto<pallet_balances::Event<AH::Runtime>>,
{
	type Balances<T> = pallet_balances::Pallet<T>;

	let sender_ed = <Sender::Runtime as pallet_balances::Config>::ExistentialDeposit::get();
	let accumulation_account = Sender::execute_with(|| {
		pallet_accumulate_and_forward::Pallet::<Sender::Runtime>::accumulation_account()
	});
	let check_account = AH::execute_with(pallet_xcm::Pallet::<AH::Runtime>::check_account);
	// Enough that the forward still clears `MinTransferAmount` after arrival fees.
	let teleported = 10 *
		Sender::execute_with(|| {
			<Sender::Runtime as pallet_accumulate_and_forward::Config>::MinTransferAmount::get()
		});

	let staking_pot =
		AH::execute_with(pallet_collator_selection::Pallet::<AH::Runtime>::account_id);
	let (asset_hub_issuance_before, check_balance_before) = AH::execute_with(|| {
		(
			Balances::<AH::Runtime>::total_issuance(),
			Balances::<AH::Runtime>::balance(&check_account),
		)
	});
	let chain_issuance_before = Sender::execute_with(Balances::<Sender::Runtime>::total_issuance);

	// GIVEN a real teleport out of Asset Hub into the accumulation account. The KSM moves into
	// Asset Hub's checking account, which is how it comes to sit on this chain at all.
	AH::execute_with(|| {
		let dest = AH::sibling_location_of(Sender::para_id());
		let beneficiary: Location =
			Junction::AccountId32 { network: None, id: accumulation_account.clone().into() }.into();
		let assets: Assets = (Location::parent(), teleported).into();

		assert_ok!(pallet_xcm::Pallet::<AH::Runtime>::limited_teleport_assets(
			frame_system::RawOrigin::Signed(asset_hub_sender).into(),
			Box::new(dest.into()),
			Box::new(beneficiary.into()),
			Box::new(assets.into()),
			0,
			WeightLimit::Unlimited,
		));
	});

	let accumulated = Sender::execute_with(|| {
		assert_eq!(
			Balances::<Sender::Runtime>::total_issuance(),
			chain_issuance_before + teleported,
			"the whole teleported amount should land on this chain"
		);
		Balances::<Sender::Runtime>::balance(&accumulation_account)
	});
	let forwarded = accumulated - sender_ed;

	// WHEN the forward runs.
	Sender::execute_with(|| {
		let period =
			<Sender::Runtime as pallet_accumulate_and_forward::Config>::TransferPeriod::get();
		set_block_number(period);
		let _ = <pallet_accumulate_and_forward::Pallet<Sender::Runtime> as Hooks<u32>>::on_idle(
			period,
			Weight::MAX,
		);

		let forwarded_out = Sender::events().into_iter().any(|event| {
			matches!(
				event.try_into(),
				Ok(pallet_accumulate_and_forward::Event::ForwardSucceeded { .. })
			)
		});
		assert!(forwarded_out, "expected an AccumulateForward::ForwardSucceeded event");

		// Emptied down to the ED; the KSM has left this chain.
		assert_eq!(Balances::<Sender::Runtime>::balance(&accumulation_account), sender_ed);
	});

	// THEN Asset Hub burns it.
	AH::execute_with(|| {
		let processed = AH::events().into_iter().any(|event| {
			matches!(
				event.try_into(),
				Ok(pallet_message_queue::Event::Processed { success: true, .. })
			)
		});
		assert!(processed, "expected a MessageQueue::Processed event on Asset Hub");

		// Everything that arrives is burned except Asset Hub's execution fee, which goes to the
		// collator pot and so stays in its issuance.
		let burned = asset_hub_issuance_before - Balances::<AH::Runtime>::total_issuance();
		// The fee is deposited to the collator pot, so read it from the event rather than the pot
		// balance, which collator payouts move in the same block.
		let fee = AH::events()
			.into_iter()
			.find_map(|event| match event.try_into() {
				Ok(pallet_balances::Event::Deposit { who, amount }) if who == staking_pot =>
					Some(amount),
				_ => None,
			})
			.expect("the execution fee is deposited to the collator pot");
		assert_eq!(burned + fee, forwarded, "all of it is either burned or paid as fee");
		assert!(burned > 0, "the burn should move Asset Hub's total issuance");
	});

	// AND the checking account still matches what this chain holds: both moved by the teleport in
	// minus the burn, which is the invariant the whole change exists for.
	let check_balance_after = AH::execute_with(|| Balances::<AH::Runtime>::balance(&check_account));
	let chain_issuance_after = Sender::execute_with(Balances::<Sender::Runtime>::total_issuance);
	assert_eq!(
		check_balance_after - check_balance_before,
		chain_issuance_after - chain_issuance_before,
		"Asset Hub's checking account must track this chain's issuance"
	);
	assert_eq!(check_balance_after - check_balance_before, teleported - forwarded);
}
