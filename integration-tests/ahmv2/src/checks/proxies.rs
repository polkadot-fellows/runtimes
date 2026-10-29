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

//! Checks of the proxy stage over the Relay Chain and the Coretime chain.

use crate::{
	checks::{ParaMigrationCheck, RcMigrationCheck},
	mock::{network, AssetHubPara, CoretimePara},
};
use frame_support::traits::Get;
use migrator_types::PortableProxyDelegate;
use pallet_rc2_migrator::proxy::ProxyMigrator;
use sp_runtime::AccountId32;
use std::collections::{BTreeMap, BTreeSet};

type Rc = network::relay::Runtime;
type Ct = network::ct::Runtime;

/// A Relay Chain proxy entry as stored: the definitions and the recorded deposit.
type RcEntry = (
	Vec<
		pallet_proxy::ProxyDefinition<
			AccountId32,
			<Rc as pallet_proxy::Config>::ProxyType,
			pallet_proxy::BlockNumberFor<Rc>,
		>,
	>,
	u128,
);

/// A Coretime proxy entry: `(delegate, type, delay)` per definition, in storage order, and the
/// recorded deposit.
type CtEntry = (Vec<(AccountId32, network::ct::ProxyType, u32)>, u128);

fn rc_entry(who: &AccountId32) -> RcEntry {
	let (defs, deposit) = pallet_proxy::Proxies::<Rc>::get(who);
	(defs.into_inner(), deposit)
}

fn ct_entry(who: &AccountId32) -> CtEntry {
	let (defs, deposit) = pallet_proxy::Proxies::<Ct>::get(who);
	(defs.iter().map(|d| (d.delegate.clone(), d.proxy_type, d.delay)).collect(), deposit)
}

/// What the Relay Chain held before the migration.
#[derive(Clone, Debug)]
pub struct RcProxiesPre {
	/// Every entry of the proxy map, to assert the stage changed none of them.
	pub entries: BTreeMap<AccountId32, RcEntry>,
	/// The definitions that travel, by the account they land on. Two Relay Chain delegators
	/// can land on one account.
	pub expected: BTreeMap<AccountId32, Vec<PortableProxyDelegate<AccountId32>>>,
}

/// Every portable definition is recreated on the Coretime chain under the delegator's address
/// there, merged with what it already had, and the Relay Chain's map is left as it was.
pub struct ProxiesChecker;

impl RcMigrationCheck for ProxiesChecker {
	type RcPrePayload = RcProxiesPre;
	type RcPostPayload = ();

	fn pre_check() -> Self::RcPrePayload {
		let mut expected: BTreeMap<AccountId32, Vec<_>> = BTreeMap::new();
		let entries = pallet_proxy::Proxies::<Rc>::iter_keys()
			.map(|who| {
				let (defs, _) = pallet_proxy::Proxies::<Rc>::get(&who);
				if let Some(portable) = ProxyMigrator::<Rc>::migrate_single(&who, &defs) {
					expected
						.entry(portable.delegator)
						.or_default()
						.extend(portable.delegates.into_inner());
				}
				let entry = rc_entry(&who);
				(who, entry)
			})
			.collect();
		RcProxiesPre { entries, expected }
	}

	fn post_check(rc_pre: &Self::RcPrePayload) -> Self::RcPostPayload {
		let after: BTreeMap<_, _> = pallet_proxy::Proxies::<Rc>::iter_keys()
			.map(|who| (who.clone(), rc_entry(&who)))
			.collect();
		assert_eq!(after, rc_pre.entries, "the Relay Chain's proxy map must be left as it was");
	}
}

/// The Coretime chain's entries of every account a set lands on, before the migration.
#[derive(Clone, Debug)]
pub struct CtProxiesPre {
	pub entries: BTreeMap<AccountId32, CtEntry>,
}

impl ParaMigrationCheck<CoretimePara> for ProxiesChecker {
	type RcPrePayload = RcProxiesPre;
	type RcPostPayload = ();
	type PrePayload = CtProxiesPre;

	fn pre_check(rc_pre: &Self::RcPrePayload) -> Self::PrePayload {
		assert_eq!(pallet_ct_migrator::FailedProxies::<Ct>::iter().count(), 0);
		let entries = rc_pre.expected.keys().map(|dest| (dest.clone(), ct_entry(dest))).collect();
		CtProxiesPre { entries }
	}

	fn post_check(
		rc_pre: &Self::RcPrePayload,
		_: &Self::RcPostPayload,
		mut ct_pre: Self::PrePayload,
	) {
		let failed: Vec<_> = pallet_ct_migrator::FailedProxies::<Ct>::iter_keys().collect();
		assert!(failed.is_empty(), "proxy sets failed to integrate: {failed:?}");

		let ratio: u32 = <Ct as pallet_ct_migrator::Config>::RcBlockTimeRatio::get();
		let mut definitions = 0usize;
		for (dest, delegates) in &rc_pre.expected {
			// What was there, plus every migrated definition as this chain represents it, in
			// the pallet's order: the tuple orders as `ProxyDefinition` does.
			let (before, _) = ct_pre.entries.remove(dest).expect("every destination was read");
			let expected: BTreeSet<_> = before
				.into_iter()
				.chain(delegates.iter().map(|d| {
					let proxy_type = network::ct::ProxyType::from(d.proxy_type);
					(d.delegate.clone(), proxy_type, d.delay.div_ceil(ratio))
				}))
				.collect();
			definitions += delegates.len();

			let (defs, deposit) = ct_entry(dest);
			assert_eq!(
				defs,
				expected.into_iter().collect::<Vec<_>>(),
				"the proxy entry of {dest:?}"
			);
			let reserved = frame_system::Account::<Ct>::get(dest).data.reserved;
			assert!(
				deposit <= reserved,
				"{dest:?} records a proxy deposit of {deposit} but reserves {reserved}"
			);
		}

		// Every migrated proxy deposit was consumed by the stage.
		let reason = network::ct::RuntimeHoldReason::CtMigrator(
			pallet_ct_migrator::HoldReason::ProxyDeposit,
		);
		let left: Vec<_> = pallet_balances::Holds::<Ct>::iter()
			.filter(|(_, holds)| holds.iter().any(|h| h.id == reason))
			.map(|(who, _)| who)
			.collect();
		assert!(left.is_empty(), "proxy deposits still on hold: {left:?}");

		log::info!(
			target: "runtime::coretime",
			"Proxies recreated on Coretime: {} definition(s) for {} delegator(s)",
			definitions,
			rc_pre.expected.len()
		);
	}
}

impl ParaMigrationCheck<AssetHubPara> for ProxiesChecker {
	type RcPrePayload = RcProxiesPre;
	type RcPostPayload = ();
	type PrePayload = ();

	fn pre_check(_: &Self::RcPrePayload) -> Self::PrePayload {}

	fn post_check(_: &Self::RcPrePayload, _: &Self::RcPostPayload, _: Self::PrePayload) {}
}
