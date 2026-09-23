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
use asset_hub_polkadot_runtime::xcm_config::LocationToAccountId;
use collectives_polkadot_runtime::fellowship::FellowshipSalaryPaymaster;
use emulated_integration_tests_common::accounts::BOB;
use frame_support::{
	assert_ok,
	traits::{fungibles::Mutate, tokens::Pay},
};
use parachains_common::AccountId;
use xcm_executor::traits::ConvertLocation;

const FELLOWSHIP_SALARY_PALLET_ID: u8 =
	collectives_polkadot_runtime_constants::FELLOWSHIP_SALARY_PALLET_INDEX;

#[test]
fn pay_salary_technical_fellowship() {
	const USDT_ID: u32 = 1984;
	let fellowship_salary = (
		Parent,
		Parachain(CollectivesPolkadot::para_id().into()),
		PalletInstance(FELLOWSHIP_SALARY_PALLET_ID),
	);
	let pay_from = LocationToAccountId::convert_location(&fellowship_salary.into()).unwrap();
	let pay_to = Polkadot::account_id_of(ALICE);
	let pay_amount = 9_000_000_000;

	AssetHubPolkadot::execute_with(|| {
		type AssetHubAssets = <AssetHubPolkadot as AssetHubPolkadotPallet>::Assets;
		// USDT registered in genesis, now mint some into the payer's account
		assert_ok!(<AssetHubAssets as Mutate<_>>::mint_into(USDT_ID, &pay_from, pay_amount * 2));
	});

	CollectivesPolkadot::execute_with(|| {
		type RuntimeEvent = <CollectivesPolkadot as Chain>::RuntimeEvent;

		assert_ok!(FellowshipSalaryPaymaster::pay(&pay_to, (), pay_amount));
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::Assets(pallet_assets::Event::Transferred { .. }) => {},
				RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true ,.. }) => {},
			]
		);
	});
}

#[test]
fn pay_salary_secretary() {
	const USDT_ID: u32 = 1984;
	// SecretarySalary uses FellowshipSalaryPaymaster, so the pay_from account is derived
	// from the fellowship salary pallet's interior location (pallet index 64).
	let fellowship_salary = (
		Parent,
		Parachain(CollectivesPolkadot::para_id().into()),
		PalletInstance(FELLOWSHIP_SALARY_PALLET_ID),
	);
	let pay_from = LocationToAccountId::convert_location(&fellowship_salary.into()).unwrap();
	let pay_to = Polkadot::account_id_of(ALICE);
	let pay_amount = 9_000_000_000;

	AssetHubPolkadot::execute_with(|| {
		type AssetHubAssets = <AssetHubPolkadot as AssetHubPolkadotPallet>::Assets;
		// USDT registered in genesis, now mint some into the payer's account
		assert_ok!(<AssetHubAssets as Mutate<_>>::mint_into(USDT_ID, &pay_from, pay_amount * 2));
	});

	CollectivesPolkadot::execute_with(|| {
		type RuntimeEvent = <CollectivesPolkadot as Chain>::RuntimeEvent;

		assert_ok!(FellowshipSalaryPaymaster::pay(&pay_to, (), pay_amount));
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::Assets(pallet_assets::Event::Transferred { .. }) => {},
				RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true ,.. }) => {},
			]
		);
	});
}

const SALARY: u128 = 1_000_000_000;

/// Makes `fellow` a rank 1 Fellow with `SALARY`, gives `delegate` a `NonTransfer` proxy for it and
/// moves the salary pallet into the payout window of a fresh cycle.
fn fellow_in_payout_window_with_non_transfer_delegate(fellow: &AccountId, delegate: &AccountId) {
	use collectives_polkadot_runtime::{
		fellowship::FellowshipCoreInstance, FellowshipCollective, FellowshipCore, FellowshipSalary,
		Proxy, ProxyType, Runtime, RuntimeOrigin,
	};
	use frame_support::traits::RankedMembers;
	use polkadot_runtime_constants::time::DAYS;

	assert_ok!(<FellowshipCollective as RankedMembers>::induct(fellow));
	assert_ok!(<FellowshipCollective as RankedMembers>::promote(fellow));
	pallet_core_fellowship::Params::<Runtime, FellowshipCoreInstance>::mutate(|p| {
		p.active_salary = vec![SALARY].try_into().unwrap();
		p.passive_salary = vec![SALARY].try_into().unwrap();
	});
	assert_ok!(FellowshipCore::import_member(
		RuntimeOrigin::signed(fellow.clone()),
		fellow.clone()
	));

	let start = frame_system::Pallet::<Runtime>::block_number();
	assert_ok!(FellowshipSalary::init(RuntimeOrigin::signed(fellow.clone())));
	assert_ok!(FellowshipSalary::induct(RuntimeOrigin::signed(fellow.clone())));
	// One cycle is 15 days of registration plus 15 days of payout.
	frame_system::Pallet::<Runtime>::set_block_number(start + 30 * DAYS);
	assert_ok!(FellowshipSalary::bump(RuntimeOrigin::signed(fellow.clone())));
	frame_system::Pallet::<Runtime>::set_block_number(start + 45 * DAYS);

	assert_ok!(Proxy::add_proxy(
		RuntimeOrigin::signed(fellow.clone()),
		delegate.clone().into(),
		ProxyType::NonTransfer,
		0
	));
}

#[test]
fn non_transfer_proxy_cannot_redirect_fellowship_salary() {
	use collectives_polkadot_runtime::{FellowshipSalary, Proxy, RuntimeCall, RuntimeOrigin};
	use frame_support::traits::fungibles::Inspect;

	const USDT_ID: u32 = 1984;
	let fellowship_salary = (
		Parent,
		Parachain(CollectivesPolkadot::para_id().into()),
		PalletInstance(FELLOWSHIP_SALARY_PALLET_ID),
	);
	let pay_from = LocationToAccountId::convert_location(&fellowship_salary.into()).unwrap();
	let fellow = CollectivesPolkadot::account_id_of(ALICE);
	let delegate = CollectivesPolkadot::account_id_of(BOB);

	AssetHubPolkadot::execute_with(|| {
		type AssetHubAssets = <AssetHubPolkadot as AssetHubPolkadotPallet>::Assets;
		assert_ok!(<AssetHubAssets as Mutate<_>>::mint_into(USDT_ID, &pay_from, SALARY * 2));
		assert_eq!(<AssetHubAssets as Inspect<_>>::balance(USDT_ID, &delegate), 0);
	});

	CollectivesPolkadot::execute_with(|| {
		type RuntimeEvent = <CollectivesPolkadot as Chain>::RuntimeEvent;
		fellow_in_payout_window_with_non_transfer_delegate(&fellow, &delegate);

		// The delegate tries to send the Fellow's salary to itself.
		assert_ok!(Proxy::proxy(
			RuntimeOrigin::signed(delegate.clone()),
			fellow.clone().into(),
			None,
			Box::new(RuntimeCall::FellowshipSalary(pallet_salary::Call::payout_other {
				beneficiary: delegate.clone(),
			})),
		));
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				RuntimeEvent::Proxy(pallet_proxy::Event::ProxyExecuted { result }) => {
					result: *result == Err(frame_system::Error::<
						collectives_polkadot_runtime::Runtime,
					>::CallFiltered
						.into()),
				},
			]
		);

		// The same call made by the Fellow itself does pay the delegate.
		assert_ok!(FellowshipSalary::payout_other(
			RuntimeOrigin::signed(fellow.clone()),
			delegate.clone()
		));
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				RuntimeEvent::FellowshipSalary(pallet_salary::Event::Paid { who, beneficiary, amount, .. }) => {
					who: *who == fellow,
					beneficiary: *beneficiary == delegate,
					amount: *amount == SALARY,
				},
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		type AssetHubAssets = <AssetHubPolkadot as AssetHubPolkadotPallet>::Assets;
		// Only the Fellow's own call paid out.
		assert_eq!(<AssetHubAssets as Inspect<_>>::balance(USDT_ID, &delegate), SALARY);
	});
}
