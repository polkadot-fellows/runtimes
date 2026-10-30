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
use crate::{mock::*, BlockLimits, CtMigratorCall, CtRuntimeCall, Event, Limits, RcMigrationStage};
use frame_support::assert_ok;
use migrator_types::{sibling_account, PortableProxyType};
use pallet_proxy::{Announcements, Proxies};

type Migrator = ProxyMigrator<Test>;

/// A set as the wire carries it: `(delegate, type, delay)` per definition.
fn portable(
	delegator: &AccountId,
	delegates: Vec<(AccountId, PortableProxyType, u32)>,
) -> PortableProxy<AccountId> {
	let delegates = delegates
		.into_iter()
		.map(|(delegate, proxy_type, delay)| PortableProxyDelegate { delegate, proxy_type, delay })
		.collect::<Vec<_>>();
	PortableProxy { delegator: delegator.clone(), delegates: delegates.try_into().unwrap() }
}

#[test]
fn portable_definitions_travel_and_every_entry_goes() {
	new_test_ext().execute_with(|| {
		// GIVEN Alice with one portable and one non-portable definition, the portable one with
		// a delay and an announcement pending on it, and Bob with a non-portable definition
		// only.
		fund(&ALICE, 500);
		fund(&BOB, 500);
		fund(&CHARLIE, 500);
		assert_ok!(Proxy::add_proxy(RuntimeOrigin::signed(ALICE), CHARLIE, ProxyType::Any, 4));
		add_proxy(&ALICE, &DAVE, ProxyType::Staking);
		add_proxy(&BOB, &CHARLIE, ProxyType::Staking);
		announce(&CHARLIE, &ALICE);

		// WHEN the stage runs. THEN Alice's portable definition travels with its delay, Bob
		// sends nothing, and both entries and the announcement are gone.
		assert_eq!(
			Migrator::migrate_many(10),
			BlockProxies {
				proxies: vec![portable(&ALICE, vec![(CHARLIE, PortableProxyType::Any, 4)])],
				removed: 3,
			}
		);
		assert_eq!(
			(Proxies::<Test>::iter().count(), Announcements::<Test>::iter().count()),
			(0, 0)
		);
	});
}

#[test]
fn sovereigns_are_named_by_their_sibling_address_on_the_wire() {
	new_test_ext().execute_with(|| {
		let delegator = child_sov(2000); // a para's sovereign delegating to another's
		let delegate_sov = child_sov(2001); // the sovereign it delegates to

		// GIVEN the delegation.
		fund(&delegator, 500);
		add_proxy(&delegator, &delegate_sov, ProxyType::NonTransfer);

		// WHEN the stage runs. THEN both ends carry the address that is the para on a parachain.
		assert_eq!(
			Migrator::migrate_many(10).proxies,
			vec![portable(
				&sibling_account(2000),
				vec![(sibling_account(2001), PortableProxyType::NonTransfer, 0)]
			)]
		);
	});
}

/// The `ProxiesBatchSent` counts in the block's events.
fn batches_sent() -> Vec<u32> {
	System::events()
		.into_iter()
		.filter_map(|r| match r.event {
			RuntimeEvent::Rc2Migrator(Event::ProxiesBatchSent { count }) => Some(count),
			_ => None,
		})
		.collect()
}

/// The proxy sets the `n`th sent message carries.
fn sent_proxies(n: usize) -> Vec<PortableProxy<AccountId>> {
	assert_eq!(sent()[n].0, ct_dest());
	let CtRuntimeCall::CtMigrator(CtMigratorCall::ReceiveProxies { proxies }) = sent_call(n) else {
		panic!("message {n} carries proxies for Coretime");
	};
	proxies
}

#[test]
fn proxy_stage_sends_in_chunks_and_resumes() {
	new_test_ext().execute_with(|| {
		// GIVEN 5 delegators, limits of 2 sets a message and 2 messages a block, and the stage
		// about to start.
		Limits::<Test>::put(BlockLimits { items_per_xcm: 2, xcm_per_block: 2 });
		for who in [&ALICE, &BOB, &CHARLIE, &DAVE, &EVE] {
			fund(who, 500);
			add_proxy(who, &FRANK, ProxyType::Any);
		}
		RcMigrationStage::<Test>::put(Stage::ProxyInit);
		run_blocks(1);
		assert_stage(Stage::ProxyOngoing);

		// WHEN the first block runs. THEN 4 sets leave in 2 full messages, their entries are
		// gone, and the stage goes on.
		run_blocks(1);
		assert_stage(Stage::ProxyOngoing);
		assert_eq!(Proxies::<Test>::iter().count(), 1);
		assert_eq!(sent().len(), 2);
		assert_eq!((sent_proxies(0).len(), sent_proxies(1).len()), (2, 2));
		assert_eq!(batches_sent(), vec![2, 2]);

		// WHEN the next block runs. THEN the last set leaves alone and the stage is done,
		// every delegator sent once and the map empty.
		run_blocks(1);
		assert_stage(Stage::ProxyDone);
		assert_eq!(sent().len(), 3);
		assert_eq!(batches_sent(), vec![2, 2, 1]);
		let mut delegators: Vec<_> = (0..3).flat_map(sent_proxies).map(|p| p.delegator).collect();
		delegators.sort();
		assert_eq!(delegators, vec![ALICE, BOB, CHARLIE, DAVE, EVE]);
		assert_eq!(Proxies::<Test>::iter().count(), 0);
	});
}

#[test]
fn an_entry_without_portable_definitions_is_removed_too() {
	new_test_ext().execute_with(|| {
		// GIVEN one delegator with only a non-portable definition, and blocks of one entry.
		Limits::<Test>::put(BlockLimits { items_per_xcm: 1, xcm_per_block: 1 });
		fund(&BOB, 500);
		add_proxy(&BOB, &CHARLIE, ProxyType::Staking);

		// WHEN a block visits it. THEN nothing travels and the entry is gone; the next block
		// finds the map empty.
		assert_eq!(Migrator::migrate_many(1), BlockProxies { removed: 1, ..Default::default() });
		assert!(!Proxies::<Test>::contains_key(&BOB));
		assert_eq!(Migrator::migrate_many(1), BlockProxies::default());

		// GIVEN the entry again. WHEN the stage runs over it. THEN it ends without sending
		// anything.
		add_proxy(&BOB, &CHARLIE, ProxyType::Staking);
		RcMigrationStage::<Test>::put(Stage::ProxyInit);
		run_blocks(3);
		assert_stage(Stage::ProxyDone);
		assert!(sent().is_empty());
	});
}

#[test]
fn proxy_stage_retries_a_block_whose_send_failed() {
	new_test_ext().execute_with(|| {
		// GIVEN the stage started with one delegator.
		fund(&ALICE, 500);
		add_proxy(&ALICE, &BOB, ProxyType::Any);
		RcMigrationStage::<Test>::put(Stage::ProxyInit);
		run_blocks(1);

		// WHEN a block runs and its send fails. THEN the stage stays where it was, the entry
		// too, and no event of the block survives.
		SendFails::set(true);
		run_blocks(1);
		assert_stage(Stage::ProxyOngoing);
		assert!(Proxies::<Test>::contains_key(&ALICE));
		assert!(sent().is_empty());
		assert!(batches_sent().is_empty());

		// WHEN the next block runs with a working router. THEN the set leaves and the entry
		// goes.
		SendFails::set(false);
		run_blocks(1);
		assert_stage(Stage::ProxyDone);
		assert_eq!(sent_proxies(0).len(), 1);
		assert!(!Proxies::<Test>::contains_key(&ALICE));
	});
}

#[test]
fn announcements_go_once_the_proxy_map_is_empty() {
	new_test_ext().execute_with(|| {
		// GIVEN Alice delegating to Bob, who has announced, and blocks of one entry.
		fund(&ALICE, 500);
		fund(&BOB, 500);
		add_proxy(&ALICE, &BOB, ProxyType::Any);
		announce(&BOB, &ALICE);

		// WHEN a block runs. THEN Alice's entry takes the block and the announcement waits.
		assert_eq!(
			Migrator::migrate_many(1),
			BlockProxies {
				proxies: vec![portable(&ALICE, vec![(BOB, PortableProxyType::Any, 0)])],
				removed: 1,
			}
		);
		assert!(Announcements::<Test>::contains_key(&BOB));

		// WHEN the next blocks run. THEN the announcement goes, and the block after finds
		// both maps empty.
		assert_eq!(Migrator::migrate_many(1), BlockProxies { removed: 1, ..Default::default() });
		assert!(!Announcements::<Test>::contains_key(&BOB));
		assert_eq!(Migrator::migrate_many(1), BlockProxies::default());
	});
}
