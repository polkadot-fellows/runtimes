// Copyright (C) Parity Technologies and the various Polkadot contributors, see Contributions.md
// for a list of specific contributors.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Asset Hub's subscription whitelist on `MembersNotifier`.

use super::*;
use crate::individuality::ASSET_HUB_MEMBERS_SUBSCRIBER_INDEX;
use cumulus_primitives_core::ParaId;
use frame_support::{assert_noop, assert_ok};
use indiv_pallet_members_notifier::{
	PendingInit, SubscriberInfo, Subscribers, SubscriptionWhitelist,
};
use indiv_support::traits::{RingExponent, PEOPLE_IDENTIFIER, PEOPLE_LITE_IDENTIFIER};
use polkadot_runtime_constants::system_parachain::ASSET_HUB_ID;

fn asset_hub() -> ParaId {
	ParaId::from(ASSET_HUB_ID)
}

/// The call is feeless and authorized, so it carries no signature.
fn authorized_origin() -> RuntimeOrigin {
	RuntimeOrigin::from(frame_system::RawOrigin::Authorized)
}

/// Collections asset hub is whitelisted for, with the exponents it mirrors them at.
fn expected_collections() -> Vec<(indiv_support::traits::Identifier, RingExponent)> {
	vec![(*PEOPLE_IDENTIFIER, RingExponent::R2e9), (*PEOPLE_LITE_IDENTIFIER, RingExponent::R2e9)]
}

#[test]
fn every_configured_whitelist_entry_is_well_formed() {
	let entries = asset_hub_subscription_whitelist();
	assert!(!entries.is_empty(), "the runtime is expected to whitelist asset hub");

	let max_subscribers =
		<<Runtime as indiv_pallet_members_notifier::Config>::MaxSubscribers as Get<u32>>::get()
			as usize;
	assert!(entries.len() <= max_subscribers, "more whitelisted parachains than MaxSubscribers",);

	let mut seen = Vec::new();
	for entry in &entries {
		assert!(!seen.contains(&entry.para_id), "{:?} is whitelisted twice", entry.para_id);
		seen.push(entry.para_id);

		MembersNotifier::resolve_whitelist_entry(entry).unwrap_or_else(|e| {
			panic!("whitelist entry for {:?} is malformed: {e:?}", entry.para_id)
		});
	}
}

#[test]
fn genesis_whitelists_asset_hub() {
	new_test_ext().execute_with(|| {
		let entry = SubscriptionWhitelist::<Runtime>::get(asset_hub())
			.expect("asset hub is whitelisted at genesis");

		assert_eq!(entry.collections.to_vec(), expected_collections());
		assert_eq!(entry.pallet_index, ASSET_HUB_MEMBERS_SUBSCRIBER_INDEX);

		// Whitelisting alone does not subscribe anyone.
		assert!(!Subscribers::<Runtime>::contains_key(asset_hub()));
		assert_eq!(Subscribers::<Runtime>::count(), 0);
	});
}

#[test]
fn anyone_can_subscribe_asset_hub_without_governance() {
	new_test_ext().execute_with(|| {
		// No account and no fee involved: the origin comes from on-chain authorization.
		assert_ok!(MembersNotifier::subscribe_whitelisted(authorized_origin(), asset_hub()));

		let info: SubscriberInfo<Runtime> =
			Subscribers::<Runtime>::get(asset_hub()).expect("asset hub is subscribed");
		assert_eq!(info.collections.to_vec(), expected_collections());
		assert_eq!(info.pallet_index, ASSET_HUB_MEMBERS_SUBSCRIBER_INDEX);

		// Initialization is queued, so the offchain worker will ship the initial ring roots.
		let pending = PendingInit::<Runtime>::get(asset_hub()).expect("init is pending");
		assert_eq!(pending.collections.to_vec(), expected_collections());
		assert_eq!(pending.pallet_index, ASSET_HUB_MEMBERS_SUBSCRIBER_INDEX);
		assert_eq!(pending.current_collection_index, 0);

		// The entry is consumed.
		assert!(!SubscriptionWhitelist::<Runtime>::contains_key(asset_hub()));
	});
}

#[test]
fn root_unsubscribing_asset_hub_is_final() {
	new_test_ext().execute_with(|| {
		assert_ok!(MembersNotifier::subscribe_whitelisted(authorized_origin(), asset_hub()));

		assert_ok!(MembersNotifier::unsubscribe(RuntimeOrigin::root(), Some(asset_hub())));
		assert!(!Subscribers::<Runtime>::contains_key(asset_hub()));

		assert_noop!(
			MembersNotifier::subscribe_whitelisted(authorized_origin(), asset_hub()),
			indiv_pallet_members_notifier::Error::<Runtime>::NotWhitelisted,
		);

		// Only root can bring asset hub back.
		assert_ok!(MembersNotifier::subscribe(
			RuntimeOrigin::root(),
			asset_hub(),
			expected_collections().try_into().expect("collections fit the subscriber bound"),
			ASSET_HUB_MEMBERS_SUBSCRIBER_INDEX,
		));
		assert!(Subscribers::<Runtime>::contains_key(asset_hub()));
	});
}
