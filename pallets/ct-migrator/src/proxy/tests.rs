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
use sp_runtime::traits::Hash;

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
		let set = vec![portable_proxy(
			&ALICE,
			vec![(BOB, PortableProxyType::Any, 0), (CHARLIE, PortableProxyType::Any, 0)],
		)];
		CtMigrator::do_receive_proxies(set.clone());

		// THEN Bob is not added twice, the deposit tops up to two definitions out of the
		// migrated one, and the rest of it is free.
		assert_eq!(
			entry(&ALICE),
			(vec![(BOB, ProxyType::Any, 0), (CHARLIE, ProxyType::Any, 0)], proxy_deposit(2))
		);
		assert_eq!(reserved(&ALICE), proxy_deposit(2));
		assert_eq!(held(HoldReason::ProxyDeposit, &ALICE), 0);
		assert_eq!(free(&ALICE), 1_000 + 400 - proxy_deposit(2));
		assert_eq!(
			migrator_events(),
			vec![
				Event::AccountsReceived { count_good: 1, count_bad: 0 },
				Event::ProxiesReceived { count_good: 1, count_bad: 0 }
			]
		);

		// WHEN the same set arrives again, as a relay chain rewind resends it. THEN nothing
		// changes and it still counts as good.
		CtMigrator::do_receive_proxies(set);
		assert_eq!(
			entry(&ALICE),
			(vec![(BOB, ProxyType::Any, 0), (CHARLIE, ProxyType::Any, 0)], proxy_deposit(2))
		);
		assert_eq!(
			(reserved(&ALICE), free(&ALICE)),
			(proxy_deposit(2), 1_000 + 400 - proxy_deposit(2))
		);
		assert_eq!(migrator_events(), vec![Event::ProxiesReceived { count_good: 1, count_bad: 0 }]);
	});
}

#[test]
fn receive_takes_the_deposit_out_of_the_migrated_hold_only() {
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

		// WHEN a set arrives in which every definition is already here. THEN it is good, the
		// entry is as it was, and the hold is released whole.
		hypothetically!({
			let all: Vec<_> =
				[BOB, CHARLIE, DAVE, ADMIN].map(|d| (d, PortableProxyType::Any, 0)).to_vec();
			CtMigrator::do_receive_proxies(vec![portable_proxy(&ALICE, all)]);
			assert_eq!(
				migrator_events(),
				vec![Event::ProxiesReceived { count_good: 1, count_bad: 0 }]
			);
			assert_eq!(entry(&ALICE), before.0);
			assert_eq!(
				(reserved(&ALICE), held(HoldReason::ProxyDeposit, &ALICE)),
				(proxy_deposit(4), 0)
			);
		});

		// WHEN a batch arrives with one more delegate for Alice and a set for Dave.
		let alices = portable_proxy(&ALICE, vec![(EVE, PortableProxyType::Any, 0)]);
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
		// GIVEN Alice with her relay chain deposit on hold and a set that arrives out of the
		// pallet's order, Bob's definition with a 2-block delay, which is one block here.
		fund(&ALICE, 1_000);
		CtMigrator::do_receive_accounts(vec![portable_account(
			&ALICE,
			0,
			vec![(PortableHoldReason::ProxyDeposit, 400)],
		)]);
		CtMigrator::do_receive_proxies(vec![portable_proxy(
			&ALICE,
			vec![
				(DAVE, PortableProxyType::Any, 0),
				(BOB, PortableProxyType::Any, 2),
				(CHARLIE, PortableProxyType::Any, 0),
			],
		)]);
		assert_eq!(
			migrator_events(),
			vec![
				Event::AccountsReceived { count_good: 1, count_bad: 0 },
				Event::ProxiesReceived { count_good: 1, count_bad: 0 }
			]
		);

		// THEN the entry is in the order the pallet keeps, priced for three.
		assert_eq!(
			entry(&ALICE),
			(
				vec![
					(BOB, ProxyType::Any, 1),
					(CHARLIE, ProxyType::Any, 0),
					(DAVE, ProxyType::Any, 0)
				],
				proxy_deposit(3)
			)
		);

		// WHEN the delegates use them. THEN Charlie dispatches as Alice at once, and Bob only
		// after announcing and waiting out his delay.
		let call = Box::new(tagged_remark(PRE_MIGRATION));
		assert_ok!(Proxy::proxy(RuntimeOrigin::signed(CHARLIE), ALICE, None, call.clone()));
		System::assert_has_event(pallet_proxy::Event::ProxyExecuted { result: Ok(()) }.into());
		assert_noop!(
			Proxy::proxy(RuntimeOrigin::signed(BOB), ALICE, None, call.clone()),
			pallet_proxy::Error::<Test>::Unannounced
		);
		let hash = <Test as pallet_proxy::Config>::CallHasher::hash_of(&call);
		assert_ok!(Proxy::announce(RuntimeOrigin::signed(BOB), ALICE, hash));
		assert_noop!(
			Proxy::proxy_announced(RuntimeOrigin::signed(BOB), BOB, ALICE, None, call.clone()),
			pallet_proxy::Error::<Test>::Unannounced
		);
		System::set_block_number(2);
		System::reset_events();
		assert_ok!(Proxy::proxy_announced(RuntimeOrigin::signed(BOB), BOB, ALICE, None, call));
		System::assert_has_event(pallet_proxy::Event::ProxyExecuted { result: Ok(()) }.into());

		// WHEN the pallet's own calls run over it. THEN a duplicate is refused, and every
		// migrated definition can be removed, the deposit following each removal.
		assert_noop!(
			Proxy::add_proxy(RuntimeOrigin::signed(ALICE), CHARLIE, ProxyType::Any, 0),
			pallet_proxy::Error::<Test>::Duplicate
		);
		for (n, (delegate, proxy_type, delay)) in entry(&ALICE).0.into_iter().enumerate() {
			assert_ok!(Proxy::remove_proxy(
				RuntimeOrigin::signed(ALICE),
				delegate,
				proxy_type,
				delay
			));
			let left = 2 - n as u128;
			assert_eq!(reserved(&ALICE), if left == 0 { 0 } else { proxy_deposit(left) });
		}
		assert_eq!(entry(&ALICE), (vec![], 0));
		assert_eq!(free(&ALICE), 1_000 + 400);
	});
}

/// Most migrated entries record no deposit: a pure's creator was refunded on the relay chain.
/// The proxy pallet prices such an entry on its next change.
#[test]
fn a_deposit_free_entry_is_priced_by_the_next_change() {
	new_test_ext().execute_with(|| {
		// GIVEN a pure, Alice, with two migrated definitions, no deposit recorded, and no
		// funds.
		let set = vec![portable_proxy(
			&ALICE,
			vec![(BOB, PortableProxyType::Any, 0), (CHARLIE, PortableProxyType::Any, 0)],
		)];
		CtMigrator::do_receive_proxies(set);
		assert_eq!(entry(&ALICE).1, 0);
		assert_eq!(free(&ALICE), 0);

		// WHEN she removes one definition. THEN the price of the one that stays has to be
		// reserved, and she cannot: a pure without funds cannot shrink its set one by one.
		assert_noop!(
			Proxy::remove_proxy(RuntimeOrigin::signed(ALICE), BOB, ProxyType::Any, 0),
			pallet_balances::Error::<Test>::InsufficientBalance
		);
		// WHEN she removes them all. THEN nothing is reserved, and the entry is gone.
		hypothetically!({
			assert_ok!(Proxy::remove_proxies(RuntimeOrigin::signed(ALICE)));
			assert!(!pallet_proxy::Proxies::<Test>::contains_key(ALICE));
			assert_eq!(reserved(&ALICE), 0);
		});

		// GIVEN she has funds.
		fund(&ALICE, 1_000);
		// WHEN she pokes. THEN the full price of two is reserved and recorded.
		hypothetically!({
			assert_ok!(Proxy::poke_deposit(RuntimeOrigin::signed(ALICE)));
			assert_eq!(entry(&ALICE).1, proxy_deposit(2));
			assert_eq!(reserved(&ALICE), proxy_deposit(2));
			System::assert_has_event(
				pallet_proxy::Event::DepositPoked {
					who: ALICE,
					kind: pallet_proxy::DepositKind::Proxies,
					old_deposit: 0,
					new_deposit: proxy_deposit(2),
				}
				.into(),
			);
		});
		// WHEN she removes one. THEN the price of the one that stays is reserved.
		hypothetically!({
			assert_ok!(Proxy::remove_proxy(RuntimeOrigin::signed(ALICE), BOB, ProxyType::Any, 0));
			assert_eq!(entry(&ALICE), (vec![(CHARLIE, ProxyType::Any, 0)], proxy_deposit(1)));
			assert_eq!(reserved(&ALICE), proxy_deposit(1));
		});
		// WHEN she adds one. THEN the price of all three is reserved, not the increment.
		assert_ok!(Proxy::add_proxy(RuntimeOrigin::signed(ALICE), DAVE, ProxyType::Any, 0));
		assert_eq!(entry(&ALICE).1, proxy_deposit(3));
		assert_eq!(reserved(&ALICE), proxy_deposit(3));
	});
}

/// A pure's address derives from its creation on the relay chain, the same way here.
#[test]
fn kill_pure_works_with_the_relay_chain_coordinates() {
	new_test_ext().execute_with(|| {
		// GIVEN a pure spawned by Bob on the relay chain at block 5, extrinsic 3, whose entry
		// and balance the migration brought here.
		let (height, ext_index) = (5u64, 3u32);
		let pure = pallet_proxy::Pallet::<Test>::pure_account(
			&BOB,
			&ProxyType::Any,
			0,
			Some((height, ext_index)),
		);
		fund(&pure, 50);
		CtMigrator::do_receive_proxies(vec![portable_proxy(
			&pure,
			vec![(BOB, PortableProxyType::Any, 0)],
		)]);
		assert_eq!(entry(&pure), (vec![(BOB, ProxyType::Any, 0)], 0));

		// WHEN the wrong coordinates are given. THEN refused.
		assert_noop!(
			Proxy::kill_pure(
				RuntimeOrigin::signed(pure),
				BOB,
				ProxyType::Any,
				0,
				height + 1,
				ext_index
			),
			pallet_proxy::Error::<Test>::NoPermission
		);

		// WHEN the pure kills itself with the relay chain's coordinates. THEN the entry goes,
		// Bob has nothing to unreserve, and the pure's own balance stays. Dispatched directly:
		// the mock's base call filter admits no `kill_pure` through `proxy`.
		assert_ok!(Proxy::kill_pure(
			RuntimeOrigin::signed(pure),
			BOB,
			ProxyType::Any,
			0,
			height,
			ext_index
		));
		assert!(!pallet_proxy::Proxies::<Test>::contains_key(pure));
		assert_eq!((reserved(&BOB), free(&pure)), (0, 50));
		System::assert_has_event(
			pallet_proxy::Event::PureKilled {
				pure,
				spawner: BOB,
				proxy_type: ProxyType::Any,
				disambiguation_index: 0,
			}
			.into(),
		);
	});
}
