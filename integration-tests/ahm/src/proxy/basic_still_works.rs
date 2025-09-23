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

//! Test that proxies can still be used correctly after migration.
//!
//! This is additional to the tests in the AH and RC migrator pallets. Those tests just check that
//! the data was moved over - not that the functionality is retained.
//!
//! NOTE: These tests should be written in the E2E chopsticks framework, but since that is not up
//! yet, they are here. This test is also very simple, it is not generic and just uses the Runtime
//! types directly.

#[cfg(feature = "kusama-ahm")]
use crate::porting_prelude::*;

use super::Permission;
use frame_support::{
	pallet_prelude::*,
	traits::{schedule::DispatchTime, Currency, StorePreimage},
};
use frame_system::{pallet_prelude::*, RawOrigin};
use pallet_ah_migrator::types::AhMigrationCheck;
use pallet_rc_migrator::types::{RcMigrationCheck, ToPolkadotSs58};
use pallet_staking_async::RewardDestination;
use sp_runtime::{
	traits::{Dispatchable, TryConvert},
	AccountId32,
	DispatchError::Module,
	ModuleError,
};
use std::{
	collections::{BTreeMap, BTreeSet},
	str::FromStr,
};

type RelayRuntime = polkadot_runtime::Runtime;
type AssetHubRuntime = asset_hub_polkadot_runtime::Runtime;

/// Proxy accounts can still be controlled by their delegates with the correct permissions.
///
/// This tests the actual functionality, not the raw data. It does so by dispatching calls from the
/// delegatee account on behalf of the delegator. It then checks for whether or not the correct
/// events were emitted.
pub struct ProxyBasicWorks;

/// An account that has some delegatees set to control it.
///
/// Can be pure or impure.
#[derive(Clone, PartialEq, Eq, RuntimeDebug)]
pub struct Proxy {
	pub who: AccountId32,
	/// The original proxy type as set on the Relay Chain.
	///
	/// We will use this to check that the intention of the proxy is still the same. This should
	/// catch issues with translation and index collision.
	pub permissions: Vec<Permission>,
	/// Can control `who`.
	pub delegatee: AccountId32,
}

/// Map of (Delegatee, Delegator) to Vec<Permissions>
type PureProxies = BTreeMap<(AccountId32, AccountId32), Vec<Permission>>;
type ZeroNonceAccounts = BTreeSet<AccountId32>;

impl RcMigrationCheck for ProxyBasicWorks {
	type RcPrePayload = (PureProxies, ZeroNonceAccounts);

	fn pre_check() -> Self::RcPrePayload {
		let mut pre_payload = BTreeMap::new();

		for (delegator, (proxies, _deposit)) in pallet_proxy::Proxies::<RelayRuntime>::iter() {
			for proxy in proxies.into_iter() {
				let inner = proxy.proxy_type.0;

				let permission = Permission::try_convert(inner)
					.expect("Must translate proxy kind to permission");
				pre_payload
					.entry((proxy.delegate, delegator.clone()))
					.or_insert_with(Vec::new)
					.push(permission);
			}
		}

		let mut zero_nonce_accounts = BTreeSet::new();
		for delegator in pallet_proxy::Proxies::<RelayRuntime>::iter_keys() {
			let nonce = frame_system::Pallet::<RelayRuntime>::account_nonce(&delegator);
			if nonce == 0 {
				zero_nonce_accounts.insert(delegator);
			}
		}

		(pre_payload, zero_nonce_accounts)
	}

	fn post_check((_, zero_nonce_accounts): Self::RcPrePayload) {
		use pallet_rc_migrator::PureProxyCandidatesMigrated;
		// All items in PureProxyCandidatesMigrated are true
		for (_, b) in PureProxyCandidatesMigrated::<RelayRuntime>::iter() {
			assert!(b, "All items in PureProxyCandidatesMigrated are true");
		}

		log::error!(
			"There are {} zero nonce accounts and {} proxies",
			zero_nonce_accounts.len(),
			pallet_proxy::Proxies::<RelayRuntime>::iter().count()
		);
		// All Remaining ones are 'Any' proxies
		for (delegator, (proxies, _deposit)) in pallet_proxy::Proxies::<RelayRuntime>::iter() {
			for proxy in proxies.into_iter() {
				let inner = proxy.proxy_type.0;

				let permission = Permission::try_convert(inner)
					.expect("Must translate proxy kind to permission");
				assert_eq!(permission, Permission::Any, "All remaining proxies are 'Any'");
				let nonce = frame_system::Pallet::<RelayRuntime>::account_nonce(&delegator);
				assert!(zero_nonce_accounts.contains(&delegator), "All remaining proxies are from zero nonce accounts but account {:?} is not, current nonce: {}", delegator.to_polkadot_ss58(), nonce);
			}
		}
	}
}

impl AhMigrationCheck for ProxyBasicWorks {
	type RcPrePayload = (PureProxies, ZeroNonceAccounts);
	type AhPrePayload = PureProxies;

	fn pre_check(_: Self::RcPrePayload) -> Self::AhPrePayload {
		// Not empty in this case
		assert!(
			pallet_proxy::Proxies::<AssetHubRuntime>::iter().next().is_some(),
			"Assert storage 'Proxy::Proxies::ah_pre::empty'"
		);

		let mut pre_payload = BTreeMap::new();

		for (delegator, (proxies, _deposit)) in pallet_proxy::Proxies::<AssetHubRuntime>::iter() {
			for proxy in proxies.into_iter() {
				let inner = proxy.proxy_type;

				let permission = match Permission::try_convert(inner) {
					Ok(permission) => permission,
					Err(e) => {
						defensive!("Proxy could not be converted: {:?}", e);
						continue;
					},
				};
				pre_payload
					.entry((proxy.delegate, delegator.clone()))
					.or_insert_with(Vec::new)
					.push(permission);
			}
		}

		pre_payload
	}

	fn post_check(
		(rc_pre_payload, _rc_zero_nonce_accounts): Self::RcPrePayload,
		ah_pre_payload: Self::AhPrePayload,
	) {
		let mut pre_and_post = rc_pre_payload;
		for ((delegatee, delegator), permissions) in ah_pre_payload.iter() {
			pre_and_post
				.entry((delegatee.clone(), delegator.clone()))
				.or_insert_with(Vec::new)
				.extend(permissions.clone());
		}

		for ((delegatee, delegator), permissions) in pre_and_post.iter() {
			// Assert storage "Proxy::Proxies::ah_post::correct"
			let (entry, _) = pallet_proxy::Proxies::<AssetHubRuntime>::get(delegator);
			if entry.is_empty() {
				defensive!("Storage entry must exist");
			}

			let maybe_delay =
				entry.iter().find(|proxy| proxy.delegate == *delegatee).map(|proxy| proxy.delay);

			let delegatee =
				pallet_ah_migrator::Pallet::<AssetHubRuntime>::translate_account_rc_to_ah(
					delegatee.clone(),
				);
			let delegator =
				pallet_ah_migrator::Pallet::<AssetHubRuntime>::translate_account_rc_to_ah(
					delegator.clone(),
				);
			Self::check_proxy(&delegatee, &delegator, permissions, maybe_delay.unwrap_or(0));
		}
	}
}

impl ProxyBasicWorks {
	pub fn check_proxy(
		delegatee: &AccountId32,
		delegator: &AccountId32,
		permissions: &[Permission],
		delay: BlockNumberFor<AssetHubRuntime>,
	) {
		if delay > 0 {
			log::debug!(
				"Not testing proxy delegatee {:?} -> {:?} because of delay: {:?}",
				delegator.to_polkadot_ss58(),
				delegatee.to_polkadot_ss58(),
				delay
			);
			return;
		}

		frame_system::Pallet::<AssetHubRuntime>::reset_events();
		let alice =
			AccountId32::from_str("5FA9nQDVg267DEd8m1ZypXLBnvN7SFxYwV7ndqSYGiN9TTpu").unwrap();

		log::debug!(
			"Checking that proxy relation {:?} -> {:?} still works with permissions {:?}",
			delegator.to_polkadot_ss58(),
			delegatee.to_polkadot_ss58(),
			permissions
		);
		if delegatee == delegator {
			return;
		}

		let allowed_transfer = permissions.contains(&Permission::Any);
		if allowed_transfer {
			assert!(
				Self::can_transfer(delegatee, delegator, permissions, true),
				"`Any` can transfer"
			);
		} else {
			assert!(
				!Self::can_transfer(delegatee, delegator, permissions, false),
				"Only `Any` can transfer"
			);
		}

		let allowed_governance = permissions.contains(&Permission::Any) ||
			permissions.contains(&Permission::NonTransfer) ||
			permissions.contains(&Permission::Governance);
		if allowed_governance {
			assert!(
				Self::can_governance(delegatee, delegator, permissions, true),
				"`Any`, `NonTransfer`, or `Governance` can do governance"
			);
		} else {
			assert!(
				!Self::can_governance(delegatee, delegator, permissions, false),
				"Only `Any`, `NonTransfer`, or `Governance` can do governance, permissions: {permissions:?}"
			);
		}

		let allowed_staking = permissions.contains(&Permission::Any) ||
			permissions.contains(&Permission::NonTransfer) ||
			permissions.contains(&Permission::Staking);
		if allowed_staking {
			assert!(
				Self::can_stake(delegatee, delegator, permissions, true),
				"`Any` or `Staking` can stake"
			);
		} else {
			assert!(
				!Self::can_stake(delegatee, delegator, permissions, false),
				"Only `Any` or `Staking` can stake"
			);
		}

		// Alice cannot transfer
		assert!(!Self::can_transfer_impl(&alice, delegator, None, false), "Alice cannot transfer");
		// Alice cannot do governance
		assert!(
			!Self::can_governance_impl(&alice, delegator, None, false),
			"Alice cannot do governance"
		);
		// Alice cannot stake
		assert!(!Self::can_stake_impl(&alice, delegator, None, false), "Alice cannot stake");
	}

	/// Check that the `delegatee` can transfer balances on behalf of the `delegator`.
	fn can_transfer(
		delegatee: &AccountId32,
		delegator: &AccountId32,
		permissions: &[Permission],
		hint: bool,
	) -> bool {
		let mut force_types = permissions
			.iter()
			.map(|p| Permission::try_convert(p.clone()).ok())
			.collect::<Vec<_>>();
		// Also always check without a force type
		force_types.push(None);

		force_types
			.into_iter()
			.any(|p| Self::can_transfer_impl(delegatee, delegator, p, hint))
	}

	fn can_transfer_impl(
		delegatee: &AccountId32,
		delegator: &AccountId32,
		force_proxy_type: Option<asset_hub_polkadot_runtime::ProxyType>,
		hint: bool,
	) -> bool {
		frame_support::hypothetically!({
			let ed = Self::fund_accounts(delegatee, delegator);

			let transfer: asset_hub_polkadot_runtime::RuntimeCall =
				pallet_balances::Call::transfer_keep_alive {
					dest: delegatee.clone().into(), // Transfer to self (does not matter).
					value: ed * 10,                 // Does not matter.
				}
				.into();

			let proxy_call: asset_hub_polkadot_runtime::RuntimeCall = pallet_proxy::Call::proxy {
				real: delegator.clone().into(),
				force_proxy_type,
				call: Box::new(transfer),
			}
			.into();
			let hint = if hint { " (it should)" } else { " (it should not)" };

			log::debug!(
				"Checking whether {:?} can transfer on behalf of {:?}{}",
				delegatee.to_polkadot_ss58(),
				delegator.to_polkadot_ss58(),
				hint
			);

			frame_system::Pallet::<AssetHubRuntime>::reset_events();
			let _ = proxy_call
				.dispatch(asset_hub_polkadot_runtime::RuntimeOrigin::signed(delegatee.clone()));

			Self::find_transfer_event(delegatee, delegator)
		})
	}

	/// Check that the `delegatee` can do governance on behalf of the `delegator`.
	///
	/// Currently only checks the `bounties::propose_bounty` call.
	fn can_governance(
		delegatee: &AccountId32,
		delegator: &AccountId32,
		permissions: &[Permission],
		hint: bool,
	) -> bool {
		let mut force_types = permissions
			.iter()
			.map(|p| Permission::try_convert(p.clone()).ok())
			.collect::<Vec<_>>();
		// Also always check without a force type
		force_types.push(None);

		force_types
			.into_iter()
			.any(|p| Self::can_governance_impl(delegatee, delegator, p, hint))
	}

	fn can_governance_impl(
		delegatee: &AccountId32,
		delegator: &AccountId32,
		force_proxy_type: Option<asset_hub_polkadot_runtime::ProxyType>,
		hint: bool,
	) -> bool {
		frame_support::hypothetically!({
			Self::fund_accounts(delegatee, delegator);

			let value = <AssetHubRuntime as pallet_bounties::Config>::BountyValueMinimum::get() * 2;
			let proposal_call =
				pallet_bounties::Call::propose_bounty { value, description: vec![] }.into();
			let proposal =
				<pallet_preimage::Pallet<AssetHubRuntime> as StorePreimage>::bound(proposal_call)
					.unwrap();
			let call: asset_hub_polkadot_runtime::RuntimeCall = pallet_referenda::Call::submit {
				proposal_origin: Box::new(RawOrigin::Root.into()),
				proposal,
				enactment_moment: DispatchTime::At(0),
			}
			.into();

			let proxy_call: asset_hub_polkadot_runtime::RuntimeCall = pallet_proxy::Call::proxy {
				real: delegator.clone().into(),
				force_proxy_type,
				call: Box::new(call),
			}
			.into();

			let hint = if hint { " (it should)" } else { " (it should not)" };

			log::debug!(
				"Checking whether {:?} can do governance on behalf of {:?}{}",
				delegatee.to_polkadot_ss58(),
				delegator.to_polkadot_ss58(),
				hint
			);

			frame_system::Pallet::<AssetHubRuntime>::reset_events();
			let _ = proxy_call
				.dispatch(asset_hub_polkadot_runtime::RuntimeOrigin::signed(delegatee.clone()));

			Self::find_referenda_submitted_event()
		})
	}

	/// Check that the `delegatee` can do staking on behalf of the `delegator`.
	///
	/// Uses the `bond` call
	fn can_stake(
		delegatee: &AccountId32,
		delegator: &AccountId32,
		permissions: &[Permission],
		hint: bool,
	) -> bool {
		let mut force_types = permissions
			.iter()
			.map(|p| Permission::try_convert(p.clone()).ok())
			.collect::<Vec<_>>();
		// Also always check without a force type
		force_types.push(None);

		force_types
			.into_iter()
			.any(|p| Self::can_stake_impl(delegatee, delegator, p, hint))
	}

	fn can_stake_impl(
		delegatee: &AccountId32,
		delegator: &AccountId32,
		force_proxy_type: Option<asset_hub_polkadot_runtime::ProxyType>,
		hint: bool,
	) -> bool {
		frame_support::hypothetically!({
			// Migration should have finished
			assert!(
				pallet_ah_migrator::AhMigrationStage::<AssetHubRuntime>::get() ==
					pallet_ah_migrator::MigrationStage::MigrationDone,
				"Migration should have finished"
			);
			Self::fund_accounts(delegatee, delegator);

			let hint = if hint { " (it should)" } else { " (it should not)" };
			log::debug!(
				"Checking whether {:?} can stake on behalf of {:?}{}",
				delegatee.to_polkadot_ss58(),
				delegator.to_polkadot_ss58(),
				hint
			);

			let call: asset_hub_polkadot_runtime::RuntimeCall =
				pallet_staking_async::Call::set_payee { payee: RewardDestination::Staked }.into();

			let proxy_call: asset_hub_polkadot_runtime::RuntimeCall = pallet_proxy::Call::proxy {
				real: delegator.clone().into(),
				force_proxy_type,
				call: Box::new(call.clone()),
			}
			.into();

			let _ = proxy_call
				.dispatch(asset_hub_polkadot_runtime::RuntimeOrigin::signed(delegatee.clone()));

			Self::find_proxy_executed_event()
		})
	}

	/// Fund the `delegatee` and `delegator` with some balance.
	fn fund_accounts(
		delegatee: &AccountId32,
		delegator: &AccountId32,
	) -> <AssetHubRuntime as pallet_balances::Config>::Balance {
		let ed = <AssetHubRuntime as pallet_balances::Config>::ExistentialDeposit::get();
		let _ = pallet_balances::Pallet::<AssetHubRuntime>::deposit_creating(
			&delegatee.clone(),
			ed * 100_000_000_000,
		);
		let _ = pallet_balances::Pallet::<AssetHubRuntime>::deposit_creating(
			&delegator.clone(),
			ed * 100_000_000_000,
		);
		ed
	}

	/// Check if there is a `Transfer` event from the `delegator` to the `delegatee`.
	fn find_transfer_event(delegatee: &AccountId32, delegator: &AccountId32) -> bool {
		for event in frame_system::Pallet::<AssetHubRuntime>::events() {
			if let asset_hub_polkadot_runtime::RuntimeEvent::Balances(
				pallet_balances::Event::Transfer { from, to, .. },
			) = event.event
			{
				if from == *delegator && to == *delegatee {
					return true
				}
			}
		}

		false
	}

	/// Check if there is a `ReferendaSubmitted` event.
	fn find_referenda_submitted_event() -> bool {
		for event in frame_system::Pallet::<AssetHubRuntime>::events() {
			if let asset_hub_polkadot_runtime::RuntimeEvent::Referenda(
				pallet_referenda::Event::Submitted { .. },
			) = event.event
			{
				return true
			}
		}

		false
	}

	/// Check that the proxy call was executed.
	///
	/// Some operations of a pallet do not emit an event themselves so we rely on the Proxy pallet.
	fn find_proxy_executed_event() -> bool {
		for event in frame_system::Pallet::<AssetHubRuntime>::events() {
			match event.event {
				asset_hub_polkadot_runtime::RuntimeEvent::Proxy(
					pallet_proxy::Event::ProxyExecuted { result: Ok(()) },
				) => return true,
				// Pallet 89 is the staking pallet, if it fails in there then that means that the
				// proxy already succeeded.
				asset_hub_polkadot_runtime::RuntimeEvent::Proxy(
					pallet_proxy::Event::ProxyExecuted {
						result: Err(Module(ModuleError { index: 89, .. })),
					},
				) => return true,
				_ => (),
			}
		}

		false
	}
}
