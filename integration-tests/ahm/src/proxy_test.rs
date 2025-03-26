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

use crate::porting_prelude::*;

use frame_support::{pallet_prelude::*, traits::Currency};
use frame_system::pallet_prelude::*;
use pallet_ah_migrator::types::AhMigrationCheck;
use pallet_rc_migrator::types::{RcMigrationCheck, ToPolkadotSs58};
use sp_runtime::{
	traits::{TryConvert, Dispatchable},
	AccountId32,
};
use std::{collections::BTreeMap, str::FromStr};

type RelayRuntime = polkadot_runtime::Runtime;
type AssetHubRuntime = asset_hub_polkadot_runtime::Runtime;

/// Intent based permission.
///
/// Should be a superset of all possible proxy types.
#[derive(Clone, PartialEq, Eq, RuntimeDebug)]
pub enum Permission {
	Any,
	NonTransfer,
	Governance,
	Staking,
	CancelProxy,
	Auction,
	NominationPools,
	ParaRegistration,
}

// Implementation for the Polkadot runtime. Will need more for Kusama and Westend in the future.
impl TryConvert<rc_proxy_definition::ProxyType, Permission> for Permission {
	fn try_convert(proxy: rc_proxy_definition::ProxyType) -> Result<Self, rc_proxy_definition::ProxyType> {
		use rc_proxy_definition::ProxyType::*;

		Ok(match proxy {
			Any => Permission::Any,
			NonTransfer => Permission::NonTransfer,
			Governance => Permission::Governance,
			Staking => Permission::Staking,
			CancelProxy => Permission::CancelProxy,
			Auction => Permission::Auction,
			NominationPools => Permission::NominationPools,
			ParaRegistration => Permission::ParaRegistration,

			#[cfg(feature = "ahm-test-westend")]
			SudoBalances | IdentityJudgement => return Err(proxy),
		})
	}
}

/// Proxy accounts can still be controlled by their delegates with the correct permissions.
///
/// This tests the actual functionality, not the raw data. It does so by dispatching calls from the
/// delegatee account on behalf of the delegator. It then checks for whether or not the correct
/// events were emitted.
pub struct ProxiesStillWork;

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
	// TODO also check the delay
}

/// Map of (Delegatee, Delegator) to Vec<Permissions>
type PureProxies = BTreeMap<(AccountId32, AccountId32), Vec<Permission>>;

impl RcMigrationCheck for ProxiesStillWork {
	type RcPrePayload = PureProxies;

	fn pre_check() -> Self::RcPrePayload {
		let mut pre_payload = BTreeMap::new();

		for (delegator, (proxies, _deposit)) in pallet_proxy::Proxies::<RelayRuntime>::iter() {
			for proxy in proxies.into_iter() {
				let permission = Permission::try_convert(proxy.proxy_type.0).expect("Proxy could not be converted");
				pre_payload
					.entry((proxy.delegate, delegator.clone()))
					.or_insert_with(Vec::new)
					.push(permission);
			}
		}

		pre_payload
	}

	fn post_check(_: Self::RcPrePayload) {
		()
	}
}

impl AhMigrationCheck for ProxiesStillWork {
	type RcPrePayload = PureProxies;
	type AhPrePayload = ();

	fn pre_check(_: Self::RcPrePayload) -> Self::AhPrePayload {
		// Not empty in this case
		assert!(
			!pallet_proxy::Proxies::<AssetHubRuntime>::iter().next().is_none(),
			"Assert storage 'Proxy::Proxies::ah_pre::empty'"
		);
	}

	fn post_check(rc_pre_payload: Self::RcPrePayload, _: Self::AhPrePayload) {
		for ((delegatee, delegator), permissions) in rc_pre_payload.iter() {
			// Assert storage "Proxy::Proxies::ah_post::correct"
			let (entry, _) = pallet_proxy::Proxies::<AssetHubRuntime>::get(&delegator);
			if entry.is_empty() {
				// FIXME possibly bug
				log::error!(
					"Storage entry must exist for {:?} -> {:?}",
					delegator.to_polkadot_ss58(),
					delegatee.to_polkadot_ss58()
				);
				continue
			}

			let maybe_delay =
				entry.iter().find(|proxy| proxy.delegate == *delegatee).map(|proxy| proxy.delay);

			Self::check_proxy(delegatee, delegator, permissions, maybe_delay.unwrap_or(0));
		}
	}
}

impl ProxiesStillWork {
	fn check_proxy(
		delegatee: &AccountId32,
		delegator: &AccountId32,
		permissions: &Vec<Permission>,
		delay: BlockNumberFor<AssetHubRuntime>,
	) {
		if delay > 0 {
			log::warn!(
				"Skipping proxy delegatee {:?} -> {:?} because of delay: {:?}",
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
			assert!(Self::can_transfer(delegatee, delegator), "`Any` can transfer");
		} else {
			assert!(!Self::can_transfer(delegatee, delegator), "Only `Any` can transfer");
		}

		#[cfg(not(feature = "ahm-test-westend"))] // Westend has no Governance
		{
			let allowed_governance = permissions.contains(&Permission::Any) ||
				permissions.contains(&Permission::NonTransfer) ||
				permissions.contains(&Permission::Governance);
			if allowed_governance {
				assert!(
					Self::can_governance(delegatee, delegator),
					"`Any`, `NonTransfer`, or `Governance` can do governance"
				);
			} else {
				assert!(
					!Self::can_governance(delegatee, delegator),
					"Only `Any`, `NonTransfer`, or `Governance` can do governance"
				);
			}
		}

		// TODO add staking etc

		// Alice cannot transfer
		assert!(!Self::can_transfer(&alice, &delegator), "Alice cannot transfer");
		// Alice cannot do governance
		#[cfg(not(feature = "ahm-test-westend"))]
		assert!(!Self::can_governance(&alice, &delegator), "Alice cannot do governance");
	}

	/// Check that the `delegatee` can transfer balances on behalf of the `delegator`.
	fn can_transfer(delegatee: &AccountId32, delegator: &AccountId32) -> bool {
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
				force_proxy_type: None,
				call: Box::new(transfer),
			}
			.into();

			log::debug!(
				"Checking whether {:?} can transfer on behalf of {:?}",
				delegatee.to_polkadot_ss58(),
				delegator.to_polkadot_ss58()
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
	#[cfg(not(feature = "ahm-test-westend"))] // Westend has no Governance
	fn can_governance(delegatee: &AccountId32, delegator: &AccountId32) -> bool {
		frame_support::hypothetically!({
			Self::fund_accounts(delegatee, delegator);

			let value = <AssetHubRuntime as pallet_bounties::Config>::BountyValueMinimum::get() * 2;
			let call: asset_hub_polkadot_runtime::RuntimeCall =
				pallet_bounties::Call::propose_bounty { value, description: vec![] }.into();

			let proxy_call: asset_hub_polkadot_runtime::RuntimeCall = pallet_proxy::Call::proxy {
				real: delegator.clone().into(),
				force_proxy_type: None,
				call: Box::new(call),
			}
			.into();

			log::debug!(
				"Checking whether {:?} can do governance on behalf of {:?}",
				delegatee.to_polkadot_ss58(),
				delegator.to_polkadot_ss58()
			);

			frame_system::Pallet::<AssetHubRuntime>::reset_events();
			let _ = proxy_call
				.dispatch(asset_hub_polkadot_runtime::RuntimeOrigin::signed(delegatee.clone()));

			Self::find_bounty_event()
		})
	}

	/// Fund the `delegatee` and `delegator` with some balance.
	fn fund_accounts(
		delegatee: &AccountId32,
		delegator: &AccountId32,
	) -> <AssetHubRuntime as pallet_balances::Config>::Balance {
		let ed = <AssetHubRuntime as pallet_balances::Config>::ExistentialDeposit::get();
		let _ = pallet_balances::Pallet::<AssetHubRuntime>::deposit_creating(
			&delegatee.clone().into(),
			ed * 10000000,
		);
		let _ = pallet_balances::Pallet::<AssetHubRuntime>::deposit_creating(
			&delegator.clone().into(),
			ed * 10000000,
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

	/// Check if there is a `BountyProposed` event.
	#[cfg(not(feature = "ahm-test-westend"))]
	fn find_bounty_event() -> bool {
		for event in frame_system::Pallet::<AssetHubRuntime>::events() {
			if let asset_hub_polkadot_runtime::RuntimeEvent::Bounties(
				pallet_bounties::Event::BountyProposed { .. },
			) = event.event
			{
				return true
			}
		}

		false
	}
}
