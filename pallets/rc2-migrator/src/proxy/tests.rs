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
use crate::{mock::*, BlockLimits, Limits};
use frame_support::assert_ok;
use migrator_types::{sibling_account, PortableProxyType};

type Migrator = ProxyMigrator<Test>;

fn delegate(
	who: &AccountId,
	proxy_type: PortableProxyType,
	delay: u32,
) -> PortableProxyDelegate<AccountId> {
	PortableProxyDelegate { delegate: who.clone(), proxy_type, delay }
}

fn portable(
	delegator: &AccountId,
	delegates: Vec<PortableProxyDelegate<AccountId>>,
) -> PortableProxy<AccountId> {
	PortableProxy { delegator: delegator.clone(), delegates: delegates.try_into().unwrap() }
}

/// The relay chain's entry of `who`, as stored.
fn entry(who: &AccountId) -> (Vec<ProxyDefinitionOf<Test>>, u128) {
	let (defs, deposit) = pallet_proxy::Proxies::<Test>::get(who);
	(defs.into_inner(), deposit)
}

#[test]
fn portable_definitions_travel_and_the_map_is_untouched() {
	new_test_ext().execute_with(|| {
		// GIVEN Alice with one portable and one non-portable definition, the portable one with
		// a delay, and Bob with a non-portable definition only.
		fund(&ALICE, 500);
		fund(&BOB, 500);
		assert_ok!(Proxy::add_proxy(RuntimeOrigin::signed(ALICE), CHARLIE, ProxyType::Any, 4));
		add_proxy(&ALICE, &DAVE, ProxyType::Staking);
		add_proxy(&BOB, &CHARLIE, ProxyType::Staking);
		let before = (entry(&ALICE), entry(&BOB));

		// WHEN the stage runs. THEN Alice's portable definition travels with its delay, Bob
		// sends nothing, and both entries are as they were.
		assert_eq!(
			Migrator::migrate_many(None),
			BlockProxies {
				proxies: vec![portable(
					&ALICE,
					vec![delegate(&CHARLIE, PortableProxyType::Any, 4)]
				)],
				last_key: None,
			}
		);
		assert_eq!((entry(&ALICE), entry(&BOB)), before);
	});
}

#[test]
fn a_pure_proxy_travels_like_any_delegator() {
	new_test_ext().execute_with(|| {
		// GIVEN a pure proxy Alice created. Its `Any` definition points back at her.
		fund(&ALICE, 500);
		let pure = create_pure(&ALICE);

		// WHEN the stage runs. THEN the definition travels under the pure account.
		assert_eq!(
			Migrator::migrate_many(None).proxies,
			vec![portable(&pure, vec![delegate(&ALICE, PortableProxyType::Any, 0)])]
		);
	});
}

#[test]
fn sovereigns_are_named_by_their_sibling_address_on_the_wire() {
	new_test_ext().execute_with(|| {
		let delegator = child_sov(2000); // a para's sovereign delegating to another's
		let delegate_sov = child_sov(2001);

		// GIVEN the delegation.
		fund(&delegator, 500);
		add_proxy(&delegator, &delegate_sov, ProxyType::NonTransfer);

		// WHEN the stage runs. THEN both ends carry the address that is the para on a parachain.
		assert_eq!(
			Migrator::migrate_many(None).proxies,
			vec![portable(
				&sibling_account(2000),
				vec![delegate(&sibling_account(2001), PortableProxyType::NonTransfer, 0)]
			)]
		);
	});
}

#[test]
fn migrate_many_resumes_from_cursor() {
	new_test_ext().execute_with(|| {
		// GIVEN a limit of 2 entries a block, and 3 delegators.
		Limits::<Test>::put(BlockLimits { items_per_xcm: 2, xcm_per_block: 1 });
		for who in [&ALICE, &BOB, &CHARLIE] {
			fund(who, 500);
			add_proxy(who, &DAVE, ProxyType::Any);
		}

		// WHEN the first block runs. THEN it sends 2 and leaves a cursor.
		let first = Migrator::migrate_many(None);
		assert_eq!(first.proxies.len(), 2);
		let cursor = first.last_key.clone().expect("one entry is left");

		// WHEN the next block continues from it. THEN it sends the last one and reports the end.
		let second = Migrator::migrate_many(Some(cursor));
		assert_eq!(second.proxies.len(), 1);
		assert_eq!(second.last_key, None);

		// Every delegator was sent exactly once.
		let mut sent: Vec<_> = first
			.proxies
			.iter()
			.chain(&second.proxies)
			.map(|p| p.delegator.clone())
			.collect();
		sent.sort();
		assert_eq!(sent, vec![ALICE, BOB, CHARLIE]);
	});
}
