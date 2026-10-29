// Copyright (C) Polkadot Fellows.
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

use super::*;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok, hypothetically};
use migrator_types::{PortableHoldReason, PortableProxyType};

/// The entry of `who` in the proxy pallet: `(delegate, type, delay)` per definition, and the
/// recorded deposit.
fn entry(who: &AccountId) -> (Vec<(AccountId, ProxyType, u64)>, u128) {
	let (defs, deposit) = pallet_proxy::Proxies::<Test>::get(who);
	(defs.into_iter().map(|d| (d.delegate, d.proxy_type, d.delay)).collect(), deposit)
}

#[test]
fn receive_writes_the_definitions_and_reprices_the_deposit() {
	new_test_ext().execute_with(|| {
		// GIVEN Alice as the accounts stage left her: a free buffer and her relay chain proxy
		// deposit on hold.
		CtMigrator::do_receive_accounts(vec![portable_account(
			&ALICE,
			10,
			vec![(PortableHoldReason::ProxyDeposit, 400)],
		)]);
		let issuance = total_issuance();

		// WHEN her set arrives: Bob with a 4-block delay, Charlie with a 1-block one.
		CtMigrator::do_receive_proxies(vec![portable_proxy(
			&ALICE,
			vec![(BOB, PortableProxyType::Any, 4), (CHARLIE, PortableProxyType::NonTransfer, 1)],
		)]);

		// THEN both definitions exist with the delays halved, rounding up.
		assert_eq!(
			entry(&ALICE),
			(
				vec![(BOB, ProxyType::Any, 2), (CHARLIE, ProxyType::NonTransfer, 1)],
				proxy_deposit(2)
			)
		);
		// AND the hold is gone, the entry is reserved at this chain's rates, the rest is free,
		// and issuance did not move.
		assert_eq!(held(HoldReason::ProxyDeposit, &ALICE), 0);
		assert_eq!(reserved(&ALICE), proxy_deposit(2));
		assert_eq!(free(&ALICE), 10 + 400 - proxy_deposit(2));
		assert_eq!(total_issuance(), issuance);
		assert_eq!(
			migrator_events(),
			vec![
				Event::AccountsReceived { count_good: 1, count_bad: 0 },
				Event::ProxiesReceived { count_good: 1, count_bad: 0 }
			]
		);
	});
}

#[test]
fn receive_merges_with_the_definitions_already_here() {
	new_test_ext().execute_with(|| {
		// GIVEN Alice already delegates to Bob here, priced at this chain's rates, and her relay
		// chain proxy deposit on hold.
		fund(&ALICE, 1_000);
		assert_ok!(Proxy::add_proxy(RuntimeOrigin::signed(ALICE), BOB, ProxyType::Any, 0));
		assert_eq!(reserved(&ALICE), proxy_deposit(1));
		CtMigrator::do_receive_accounts(vec![portable_account(
			&ALICE,
			0,
			vec![(PortableHoldReason::ProxyDeposit, 400)],
		)]);

		// WHEN a set arrives with Bob again and Charlie.
		CtMigrator::do_receive_proxies(vec![portable_proxy(
			&ALICE,
			vec![(BOB, PortableProxyType::Any, 0), (CHARLIE, PortableProxyType::Any, 0)],
		)]);

		// THEN Bob is not added twice, the deposit tops up to two definitions out of the
		// migrated one, and the rest of it is free.
		assert_eq!(
			entry(&ALICE),
			(vec![(BOB, ProxyType::Any, 0), (CHARLIE, ProxyType::Any, 0)], proxy_deposit(2))
		);
		assert_eq!(reserved(&ALICE), proxy_deposit(2));
		assert_eq!(held(HoldReason::ProxyDeposit, &ALICE), 0);
		assert_eq!(free(&ALICE), 1_000 + 400 - proxy_deposit(2));
	});
}

#[test]
fn receive_reserves_no_more_than_the_migrated_deposit() {
	new_test_ext().execute_with(|| {
		// GIVEN Alice with a balance of her own and nothing on hold, as a pure proxy arrives.
		fund(&ALICE, 1_000);
		let set = vec![portable_proxy(&ALICE, vec![(BOB, PortableProxyType::Any, 0)])];

		// WHEN her set arrives. THEN the definition is written with no deposit recorded, her
		// balance stays free, and the set counts as good.
		hypothetically!({
			CtMigrator::do_receive_proxies(set.clone());
			assert_eq!(entry(&ALICE), (vec![(BOB, ProxyType::Any, 0)], 0));
			assert_eq!(reserved(&ALICE), 0);
			assert_eq!(free(&ALICE), 1_000);
			assert_eq!(
				migrator_events(),
				vec![Event::ProxiesReceived { count_good: 1, count_bad: 0 }]
			);
		});

		// WHEN a deposit short of this chain's price arrives on hold with her instead. THEN
		// that much is reserved and recorded, and her own balance stays free.
		let short = proxy_deposit(1) - 1;
		CtMigrator::do_receive_accounts(vec![portable_account(
			&ALICE,
			0,
			vec![(PortableHoldReason::ProxyDeposit, short)],
		)]);
		CtMigrator::do_receive_proxies(set);
		assert_eq!(entry(&ALICE), (vec![(BOB, ProxyType::Any, 0)], short));
		assert_eq!(reserved(&ALICE), short);
		assert_eq!(held(HoldReason::ProxyDeposit, &ALICE), 0);
		assert_eq!(free(&ALICE), 1_000);
	});
}

/// The relay chain sends no empty set.
#[test]
#[should_panic(expected = "Defensive failure")]
fn receive_defends_an_empty_set() {
	new_test_ext().execute_with(|| {
		CtMigrator::do_receive_proxies(vec![portable_proxy(&ALICE, vec![])]);
	});
}

/// The relay chain's proxy pallet refuses a self proxy, so none arrives.
#[test]
#[should_panic(expected = "Defensive failure")]
fn receive_defends_a_self_proxy() {
	new_test_ext().execute_with(|| {
		fund(&ALICE, 1_000);
		CtMigrator::do_receive_proxies(vec![portable_proxy(
			&ALICE,
			vec![(ALICE, PortableProxyType::Any, 0)],
		)]);
	});
}

#[test]
fn receive_parks_a_set_that_does_not_fit_and_keeps_the_batch() {
	new_test_ext().execute_with(|| {
		// GIVEN Alice with a full entry here and a migrated deposit on hold, and Dave with
		// nothing.
		fund(&ALICE, 1_000);
		for delegate in [BOB, CHARLIE, DAVE, ADMIN] {
			assert_ok!(Proxy::add_proxy(RuntimeOrigin::signed(ALICE), delegate, ProxyType::Any, 0));
		}
		CtMigrator::do_receive_accounts(vec![portable_account(
			&ALICE,
			0,
			vec![(PortableHoldReason::ProxyDeposit, 400)],
		)]);
		let before = (entry(&ALICE), reserved(&ALICE), held(HoldReason::ProxyDeposit, &ALICE));
		System::reset_events();

		// WHEN a batch arrives with one more delegate for Alice and a set for Dave.
		let alices = portable_proxy(&ALICE, vec![(ALICE + 100, PortableProxyType::Any, 0)]);
		CtMigrator::do_receive_proxies(vec![
			alices.clone(),
			portable_proxy(&DAVE, vec![(BOB, PortableProxyType::Any, 0)]),
		]);

		// THEN Alice's set is parked whole and nothing of hers moved, hold included; Dave's
		// went through.
		assert_eq!(FailedProxies::<Test>::get(&ALICE), Some(alices));
		assert_eq!(
			(entry(&ALICE), reserved(&ALICE), held(HoldReason::ProxyDeposit, &ALICE)),
			before
		);
		assert_eq!(entry(&DAVE).0, vec![(BOB, ProxyType::Any, 0)]);
		assert_eq!(migrator_events(), vec![Event::ProxiesReceived { count_good: 1, count_bad: 1 }]);
	});
}

#[test]
fn the_proxy_pallet_can_work_with_what_was_written() {
	new_test_ext().execute_with(|| {
		// GIVEN Alice with a set that arrives out of the pallet's order, and the funds to back
		// it.
		fund(&ALICE, 1_000);
		CtMigrator::do_receive_proxies(vec![portable_proxy(
			&ALICE,
			vec![
				(DAVE, PortableProxyType::Any, 0),
				(BOB, PortableProxyType::NonTransfer, 2),
				(CHARLIE, PortableProxyType::Any, 0),
			],
		)]);

		// THEN the entry is in the order the pallet keeps.
		assert_eq!(
			entry(&ALICE).0,
			vec![
				(BOB, ProxyType::NonTransfer, 1),
				(CHARLIE, ProxyType::Any, 0),
				(DAVE, ProxyType::Any, 0)
			]
		);

		// WHEN the delegates use them. THEN Charlie dispatches as Alice, and Bob is refused
		// until he announces, since his delay came along.
		let call = Box::new(tagged_remark(PRE_MIGRATION));
		assert_ok!(Proxy::proxy(RuntimeOrigin::signed(CHARLIE), ALICE, None, call.clone()));
		System::assert_has_event(pallet_proxy::Event::ProxyExecuted { result: Ok(()) }.into());
		assert_noop!(
			Proxy::proxy(RuntimeOrigin::signed(BOB), ALICE, None, call),
			pallet_proxy::Error::<Test>::Unannounced
		);

		// WHEN the pallet's own calls run over it. THEN a duplicate is refused, and every
		// migrated definition can be removed, the deposit following each removal.
		assert_noop!(
			Proxy::add_proxy(RuntimeOrigin::signed(ALICE), CHARLIE, ProxyType::Any, 0),
			pallet_proxy::Error::<Test>::Duplicate
		);
		for (delegate, proxy_type, delay) in entry(&ALICE).0 {
			assert_ok!(Proxy::remove_proxy(
				RuntimeOrigin::signed(ALICE),
				delegate,
				proxy_type,
				delay
			));
		}
		assert_eq!(entry(&ALICE), (vec![], 0));
		assert_eq!(reserved(&ALICE), 0);
	});
}
