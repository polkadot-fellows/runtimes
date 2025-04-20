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
// along with Polkadot. If not, see <http://www.gnu.org/licenses/>.

//! TreasuryBurnHandler helper structure tests.
//!
//! Note: These tests emulate the effects of burning some amount on `pallet_treasury` via
//! [`OnUnbalanced`], not the behaviour itself.

use frame_support::{
	parameter_types,
	traits::{
		tokens::fungible::{Inspect, Mutate},
		Currency, ExistenceRequirement, Get, Imbalance, OnUnbalanced, WithdrawReasons,
	},
};
use kusama_runtime_constants::currency::UNITS;
use polkadot_primitives::{AccountId, Balance};
use sp_arithmetic::Permill;
use staging_kusama_runtime::{
	dynamic_params::treasury::{self, BurnDestination, BurnPortion},
	Balances, BurnDestinationAccount, Parameters, RuntimeOrigin, RuntimeParameters, Treasury,
	TreasuryBurnHandler,
};

parameter_types! {
	TreasuryAccount: AccountId = Treasury::account_id();
}

const BURN_DESTINATION_ACCOUNT: AccountId = AccountId::new([1u8; 32]);

const TREASURY_AMOUNT: Balance = 10 * UNITS;
const SURPLUS: Balance = UNITS;

fn test(pre: impl FnOnce(), test: impl FnOnce(Balance)) {
	sp_io::TestExternalities::default().execute_with(|| {
		pre();

		Balances::set_balance(&TreasuryAccount::get(), TREASURY_AMOUNT);

		let amount_to_handle = TreasuryBurnHandler::get() * SURPLUS;
		let burn = <Balances as Currency<_>>::withdraw(
			&TreasuryAccount::get(),
			SURPLUS,
			WithdrawReasons::RESERVE,
			ExistenceRequirement::KeepAlive,
		)
		.expect("withdrawing of `burn` is within balance limits; qed");

		// Withdrawn surplus to burn it.
		assert_eq!(Balances::balance(&TreasuryAccount::get()), TREASURY_AMOUNT - SURPLUS);

		let (credit, burn) = burn.split(amount_to_handle);

		// Burn amount that's not to handle.
		<() as OnUnbalanced<_>>::on_unbalanced(burn);

		assert_eq!(Balances::total_issuance(), TREASURY_AMOUNT - (SURPLUS - amount_to_handle));

		// Let's handle the `credit`
		TreasuryBurnHandler::on_unbalanced(credit);

		test(amount_to_handle);

		// Only the amount to handle was transferred to the burn destination account
		// let burn_destination_account = BurnDestination::get();
		let burn_destination_account = BURN_DESTINATION_ACCOUNT;
		let burn_destination_account_balance =
			<Balances as Inspect<_>>::total_balance(&burn_destination_account);

		assert_eq!(burn_destination_account_balance, amount_to_handle);
	})
}

#[test]
fn on_burn_parameters_not_set_does_not_handle_burn() {
	test(
		|| {},
		|amount_to_handle| {
			// Amount to burn should be zero by default
			assert_eq!(amount_to_handle, 0);
		},
	)
}

#[test]
fn on_burn_portion_not_set_does_not_handle_burn() {
	test(
		|| {
			Parameters::set_parameter(
				RuntimeOrigin::root(),
				RuntimeParameters::Treasury(treasury::Parameters::BurnDestination(
					BurnDestination,
					Some(BurnDestinationAccount(Some(BURN_DESTINATION_ACCOUNT))),
				)),
			)
			.expect("parameters are set accordingly; qed");
		},
		|amount_to_handle| {
			// Amount to burn should be zero by default
			assert_eq!(amount_to_handle, 0);
		},
	)
}

#[test]
fn on_burn_destination_not_set_does_not_handle_burn() {
	let one_percent = Permill::from_percent(1);
	test(
		|| {
			Parameters::set_parameter(
				RuntimeOrigin::root(),
				RuntimeParameters::Treasury(treasury::Parameters::BurnPortion(
					BurnPortion,
					Some(one_percent),
				)),
			)
			.expect("parameters are set accordingly; qed");
		},
		|amount_to_handle| {
			// Amount to burn should be zero by default
			assert_eq!(amount_to_handle, 0);
		},
	)
}

#[test]
fn on_burn_parameters_set_works() {
	let one_percent = Permill::from_percent(1);
	test(
		|| {
			Parameters::set_parameter(
				RuntimeOrigin::root(),
				RuntimeParameters::Treasury(treasury::Parameters::BurnDestination(
					BurnDestination,
					Some(BurnDestinationAccount(Some(BURN_DESTINATION_ACCOUNT))),
				)),
			)
			.expect("parameters are set accordingly; qed");
			Parameters::set_parameter(
				RuntimeOrigin::root(),
				RuntimeParameters::Treasury(treasury::Parameters::BurnPortion(
					BurnPortion,
					Some(one_percent),
				)),
			)
			.expect("parameters are set accordingly; qed");
		},
		|amount_to_handle| {
			// Amount to burn should be zero by default
			assert_eq!(amount_to_handle, one_percent * SURPLUS);
		},
	)
}
