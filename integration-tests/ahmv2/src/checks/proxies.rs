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
use frame_support::{
	assert_noop, assert_ok, dispatch::PostDispatchInfo, hypothetically, traits::Get,
};
use migrator_types::{translate_destination, PortableProxyDelegate};
use pallet_rc2_migrator::{accounts::AccountsMigrator, proxy::ProxyMigrator};
use sp_runtime::{traits::Dispatchable, AccountId32, DispatchError};
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

/// What travels to one Coretime account.
#[derive(Clone, Debug, Default)]
pub struct Expected {
	pub delegates: Vec<PortableProxyDelegate<AccountId32>>,
	/// The most the accounts stage can place on hold for these definitions: what each
	/// delegator that is not a pure proxy recorded, up to its reserve.
	pub hold_at_most: u128,
}

/// What the Relay Chain held before the migration.
#[derive(Clone, Debug)]
pub struct RcProxiesPre {
	/// Every entry of the proxy map, to assert the stage changed none of them.
	pub entries: BTreeMap<AccountId32, RcEntry>,
	/// By the account the definitions land on. Two Relay Chain delegators can land on one
	/// account.
	pub expected: BTreeMap<AccountId32, Expected>,
	/// A delegator and a delegate, both plain accounts, to dispatch a proxied call with: on
	/// the Relay Chain before the migration, and on both chains after it.
	pub probe: Option<(AccountId32, AccountId32)>,
}

/// Every portable definition is recreated on the Coretime chain under the delegator's address
/// there, merged with what it already had, and the Relay Chain's map is left as it was.
pub struct ProxiesChecker;

fn dispatch_signed<C>(who: &AccountId32, call: C) -> Result<(), DispatchError>
where
	C: Dispatchable<PostInfo = PostDispatchInfo>,
	C::RuntimeOrigin: From<frame_system::RawOrigin<AccountId32>>,
{
	call.dispatch(frame_system::RawOrigin::Signed(who.clone()).into())
		.map(|_| ())
		.map_err(|e| e.error)
}

/// `delegate`'s proxied `remark` as `real`, through the Relay Chain's call filter.
fn rc_proxied_remark(delegate: &AccountId32, real: &AccountId32) -> Result<(), DispatchError> {
	let remark = network::relay::RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
	let call = network::relay::RuntimeCall::Proxy(pallet_proxy::Call::proxy {
		real: real.clone().into(),
		force_proxy_type: None,
		call: Box::new(remark),
	});
	dispatch_signed(delegate, call)
}

/// `delegate` dispatches a `remark` as `real` on the Coretime chain, and the proxy pallet
/// reports that it ran.
fn ct_proxied_remark_works(delegate: &AccountId32, real: &AccountId32) {
	let remark = network::ct::RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
	let call = network::ct::RuntimeCall::Proxy(pallet_proxy::Call::proxy {
		real: real.clone().into(),
		force_proxy_type: None,
		call: Box::new(remark),
	});
	hypothetically!({
		frame_system::Pallet::<Ct>::reset_events();
		assert_ok!(dispatch_signed(delegate, call));
		let executed = pallet_proxy::Event::<Ct>::ProxyExecuted { result: Ok(()) };
		frame_system::Pallet::<Ct>::assert_has_event(executed.into());
	});
}

impl RcMigrationCheck for ProxiesChecker {
	type RcPrePayload = RcProxiesPre;
	type RcPostPayload = ();

	fn pre_check() -> Self::RcPrePayload {
		let mut expected: BTreeMap<AccountId32, Expected> = BTreeMap::new();
		let mut probe = None;
		let entries = pallet_proxy::Proxies::<Rc>::iter_keys()
			.map(|who| {
				let (defs, deposit) = pallet_proxy::Proxies::<Rc>::get(&who);
				let info = frame_system::Account::<Rc>::get(&who);
				let pure = AccountsMigrator::<Rc>::is_pure_proxy(&who, &info);
				if let Some(portable) = ProxyMigrator::<Rc>::migrate_single(&who, &defs) {
					// a plain delegator with a plain delegate that needs no announcement
					if probe.is_none() && !pure && translate_destination(&who) == who {
						probe = portable
							.delegates
							.iter()
							.find(|d| {
								d.delay == 0 && translate_destination(&d.delegate) == d.delegate
							})
							.map(|d| (who.clone(), d.delegate.clone()));
					}
					let to = expected.entry(portable.delegator).or_default();
					to.delegates.extend(portable.delegates.into_inner());
					if !pure {
						to.hold_at_most += deposit.min(info.data.reserved);
					}
				}
				(who, (defs.into_inner(), deposit))
			})
			.collect();

		// the probe is sound: the call goes through before the migration
		let (delegator, delegate) = probe.clone().expect("a plain delegator with a plain delegate");
		hypothetically!({
			assert_ok!(rc_proxied_remark(&delegate, &delegator));
		});
		RcProxiesPre { entries, expected, probe }
	}

	fn post_check(rc_pre: &Self::RcPrePayload) -> Self::RcPostPayload {
		let after: BTreeMap<_, _> = pallet_proxy::Proxies::<Rc>::iter_keys()
			.map(|who| (who.clone(), rc_entry(&who)))
			.collect();
		assert_eq!(after, rc_pre.entries, "the Relay Chain's proxy map must be left as it was");

		// Proxies are closed here for good.
		let (delegator, delegate) = rc_pre.probe.clone().expect("picked before the migration");
		let filtered: DispatchError = frame_system::Error::<Rc>::CallFiltered.into();
		hypothetically!({
			assert_noop!(rc_proxied_remark(&delegate, &delegator), filtered);
		});
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
		let (mut definitions, mut deposit_free) = (0usize, 0usize);
		for (dest, expected) in &rc_pre.expected {
			// What was there, plus every migrated definition as this chain represents it, in
			// the pallet's order: the tuple orders as `ProxyDefinition` does.
			let (before, before_deposit) =
				ct_pre.entries.remove(dest).expect("every destination was read");
			let defs_expected: BTreeSet<_> = before
				.into_iter()
				.chain(expected.delegates.iter().map(|d| {
					let proxy_type = network::ct::ProxyType::from(d.proxy_type);
					(d.delegate.clone(), proxy_type, d.delay.div_ceil(ratio))
				}))
				.collect();
			definitions += expected.delegates.len();

			let (defs, deposit) = ct_entry(dest);
			assert_eq!(
				defs,
				defs_expected.into_iter().collect::<Vec<_>>(),
				"the proxy entry of {dest:?}"
			);
			// Priced at this chain's rates out of the migrated hold, and never more.
			let required = pallet_proxy::Pallet::<Ct>::deposit(defs.len() as u32);
			assert_eq!(
				deposit,
				required.min(before_deposit + expected.hold_at_most),
				"the proxy deposit of {dest:?}"
			);
			let reserved = frame_system::Account::<Ct>::get(dest).data.reserved;
			assert!(deposit <= reserved, "{dest:?} records {deposit} but reserves {reserved}");
			if deposit == 0 {
				deposit_free += 1;
			}
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

		// A migrated definition works here: for a plain delegator, and for a pure proxy
		// through its `Any` delegate.
		let (delegator, delegate) = rc_pre.probe.clone().expect("picked before the migration");
		ct_proxied_remark_works(&delegate, &delegator);
		let pure = rc_pre
			.expected
			.iter()
			.find(|(dest, e)| {
				e.hold_at_most == 0 && e.delegates.iter().all(|d| d.delegate != **dest)
			})
			.map(|(dest, e)| (dest.clone(), e.delegates[0].delegate.clone()))
			.expect("a pure proxy");
		ct_proxied_remark_works(&pure.1, &pure.0);

		log::info!(
			target: "runtime::coretime",
			"Proxies recreated on Coretime: {} definition(s) for {} delegator(s), {} of them deposit-free",
			definitions,
			rc_pre.expected.len(),
			deposit_free
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
