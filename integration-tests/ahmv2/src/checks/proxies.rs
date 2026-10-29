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
use codec::Encode;
use frame_support::traits::{fungible::InspectHold, Get};
use migrator_types::PortableProxyDelegate;
use pallet_rc2_migrator::proxy::ProxyMigrator;
use sp_runtime::AccountId32;
use std::collections::{BTreeMap, BTreeSet};

type Rc = network::relay::Runtime;
type Ct = network::ct::Runtime;

/// A proxy definition as the Coretime chain stores it: `(delegate, type, delay)`.
type CtDef = (AccountId32, network::ct::ProxyType, u32);

/// What the Relay Chain held before the migration.
#[derive(Clone, Debug)]
pub struct RcProxiesPre {
	/// Every entry of the proxy map, encoded, to assert the stage changed none of them.
	pub entries: Vec<(AccountId32, Vec<u8>)>,
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
		let entries = pallet_proxy::Proxies::<Rc>::iter()
			.map(|(who, (defs, deposit))| {
				let portable = ProxyMigrator::<Rc>::migrate_single(&who, &defs)
					.unwrap_or_else(|e| panic!("proxies of {who:?} cannot travel: {e:?}"));
				if let Some(portable) = portable {
					expected
						.entry(portable.delegator)
						.or_default()
						.extend(portable.delegates.into_inner());
				}
				(who, (defs, deposit).encode())
			})
			.collect();
		RcProxiesPre { entries, expected }
	}

	fn post_check(rc_pre: &Self::RcPrePayload) -> Self::RcPostPayload {
		assert_eq!(pallet_proxy::Proxies::<Rc>::iter().count(), rc_pre.entries.len());
		for (who, before) in &rc_pre.entries {
			assert_eq!(
				pallet_proxy::Proxies::<Rc>::get(who).encode(),
				*before,
				"the proxy entry of {who:?} must be left as it was"
			);
		}
	}
}

/// The Coretime chain's definitions of every account a set lands on, before the migration.
#[derive(Clone, Debug)]
pub struct CtProxiesPre {
	pub entries: BTreeMap<AccountId32, BTreeSet<CtDef>>,
}

fn ct_defs(who: &AccountId32) -> (BTreeSet<CtDef>, u128, bool) {
	let (defs, deposit) = pallet_proxy::Proxies::<Ct>::get(who);
	let sorted = defs.windows(2).all(|w| w[0] <= w[1]);
	let defs = defs.iter().map(|d| (d.delegate.clone(), d.proxy_type, d.delay)).collect();
	(defs, deposit, sorted)
}

fn proxy_deposit_hold(who: &AccountId32) -> u128 {
	let reason =
		network::ct::RuntimeHoldReason::CtMigrator(pallet_ct_migrator::HoldReason::ProxyDeposit);
	pallet_balances::Pallet::<Ct>::balance_on_hold(&reason, who)
}

impl ParaMigrationCheck<CoretimePara> for ProxiesChecker {
	type RcPrePayload = RcProxiesPre;
	type RcPostPayload = ();
	type PrePayload = CtProxiesPre;

	fn pre_check(rc_pre: &Self::RcPrePayload) -> Self::PrePayload {
		assert_eq!(pallet_ct_migrator::FailedProxies::<Ct>::iter().count(), 0);
		let entries = rc_pre.expected.keys().map(|dest| (dest.clone(), ct_defs(dest).0)).collect();
		CtProxiesPre { entries }
	}

	fn post_check(rc_pre: &Self::RcPrePayload, _: &Self::RcPostPayload, ct_pre: Self::PrePayload) {
		let failed: Vec<_> = pallet_ct_migrator::FailedProxies::<Ct>::iter_keys().collect();
		assert!(failed.is_empty(), "proxy sets failed to integrate: {failed:?}");

		let ratio: u32 = <Ct as pallet_ct_migrator::Config>::RcBlockTimeRatio::get();
		let mut definitions = 0usize;
		for (dest, delegates) in &rc_pre.expected {
			// What was there, plus every migrated definition as this chain represents it.
			let mut expected = ct_pre.entries[dest].clone();
			expected.extend(delegates.iter().map(|d| {
				(d.delegate.clone(), network::ct::ProxyType::from(d.proxy_type), d.delay / ratio)
			}));
			definitions += delegates.len();

			let (defs, deposit, sorted) = ct_defs(dest);
			assert_eq!(defs, expected, "the proxy entry of {dest:?} on Coretime");
			assert!(sorted, "the proxy entry of {dest:?} is not in the pallet's order");
			let reserved = frame_system::Account::<Ct>::get(dest).data.reserved;
			assert!(
				deposit <= reserved,
				"{dest:?} records a proxy deposit of {deposit} but reserves {reserved}"
			);
			assert_eq!(proxy_deposit_hold(dest), 0, "{dest:?} still holds its migrated deposit");
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
