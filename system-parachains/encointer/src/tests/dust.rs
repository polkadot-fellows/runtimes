// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

use crate::{AccumulateForward, Balance, Balances, Runtime};
use frame_support::{
	assert_ok,
	traits::{
		fungible::{Inspect, Mutate},
		tokens::Preservation,
		OnRuntimeUpgrade,
	},
};
use parachains_common::AccountId;
use parachains_runtimes_test_utils::ExtBuilder;
use system_parachains_common::accumulate_and_forward::EnsureAccumulationAccountFunded;

const ALICE: [u8; 32] = [1u8; 32];
const BOB: [u8; 32] = [2u8; 32];

/// Dust accumulates for the forward to Asset Hub instead of being burned here, where the burn
/// would not show in the network total that Asset Hub tracks.
#[test]
fn dust_accumulates_instead_of_being_burned() {
	let existential_deposit: Balance =
		<Runtime as pallet_balances::Config>::ExistentialDeposit::get();
	let accumulation_account = AccumulateForward::accumulation_account();

	ExtBuilder::<Runtime>::default()
		// Funded out of band before the upgrade; without the ED, dust is rejected and burned.
		.with_balances(vec![(accumulation_account.clone(), existential_deposit)])
		.build()
		.execute_with(|| {
			EnsureAccumulationAccountFunded::<Runtime>::on_runtime_upgrade();

			let alice = AccountId::from(ALICE);
			let bob = AccountId::from(BOB);
			assert_ok!(Balances::mint_into(&alice, existential_deposit));
			assert_ok!(Balances::mint_into(&bob, existential_deposit));

			let issuance_before = Balances::total_issuance();
			let accumulated_before = Balances::balance(&accumulation_account);

			// Reap Alice, leaving dust behind.
			let dust = existential_deposit / 2;
			assert_ok!(<Balances as Mutate<_>>::transfer(
				&alice,
				&bob,
				existential_deposit - dust,
				Preservation::Expendable,
			));

			assert_eq!(Balances::balance(&alice), 0);
			assert_eq!(Balances::balance(&accumulation_account), accumulated_before + dust);
			assert_eq!(Balances::total_issuance(), issuance_before);
		});
}
