// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Tests the People-to-Asset-Hub Individuality ring-root notification path.

use crate::*;
use cumulus_pallet_parachain_system::{
	relay_state_snapshot::MessagingStateSnapshot, RelevantMessagingState,
};
use frame_support::BoundedVec;
use indiv_pallet_members::{CurrentRingIndex, Root};
use indiv_pallet_members_notifier::{PendingInit, Subscribers};
use indiv_pallet_members_subscriber::{
	Pallet as MembersSubscriber, RingCollectionExponents, Subscription,
};
use indiv_support::crypto::{BandersnatchVrfVerifiable, GenerateVerifiable};
use polkadot_primitives::v9::AbridgedHrmpChannel;
use verifiable::ring::RingDomainSize;

/// The actual People -> Asset Hub initialization XCM activates the subscriber and transfers a
/// root that the Asset Hub proof consumers can subsequently use. The emulated environment does
/// not run offchain workers, so the notifier's authorized maintenance call is dispatched here.
#[test]
fn people_ring_root_notification_activates_asset_hub_subscriber() {
	type PeopleRuntime = <PeoplePolkadot as Chain>::Runtime;
	type AssetHubRuntime = <AssetHubPolkadot as Chain>::Runtime;

	let collection = *indiv_pallet_people::PEOPLE_MEMBER_IDENTIFIER;
	let ring_exponent = people_polkadot_runtime::individuality::MembersFlexibleRingExponent::get();
	let intermediate = BandersnatchVrfVerifiable::start_members(RingDomainSize::Domain11);
	let root = BandersnatchVrfVerifiable::finish_members(intermediate.clone());

	PeoplePolkadot::execute_with(|| {
		// The notifier uses the channel snapshot to bound a page. Seed the two-way HRMP
		// precondition represented by this sender-side channel before subscribing Asset Hub.
		RelevantMessagingState::<PeopleRuntime>::put(MessagingStateSnapshot {
			dmq_mqc_head: Default::default(),
			relay_dispatch_queue_remaining_capacity: Default::default(),
			ingress_channels: Vec::new(),
			egress_channels: vec![(
				AssetHubPolkadot::para_id(),
				AbridgedHrmpChannel {
					max_capacity: 1000,
					max_total_size: 1_000_000,
					max_message_size: 100_000,
					msg_count: 0,
					total_size: 0,
					mqc_head: None,
				},
			)],
		});

		// Seed an existing People collection/root so subscription initialization has real
		// state to transmit rather than merely exercising an empty init page.
		Root::<PeopleRuntime>::insert(
			collection,
			0,
			indiv_pallet_members::RingRoot::<PeopleRuntime> {
				root: root.clone(),
				revision: 7,
				intermediate,
			},
		);
		CurrentRingIndex::<PeopleRuntime>::insert(collection, 0);

		assert_ok!(people_polkadot_runtime::MembersNotifier::subscribe(
			<PeoplePolkadot as Chain>::RuntimeOrigin::root(),
			AssetHubPolkadot::para_id(),
			BoundedVec::try_from(vec![(collection, ring_exponent)]).unwrap(),
			97,
		));
		assert!(Subscribers::<PeopleRuntime>::contains_key(AssetHubPolkadot::para_id()));
		assert!(PendingInit::<PeopleRuntime>::contains_key(AssetHubPolkadot::para_id()));

		assert_ok!(people_polkadot_runtime::MembersNotifier::send_init_page(
			<PeoplePolkadot as Chain>::RuntimeOrigin::from(frame_system::RawOrigin::Authorized),
			AssetHubPolkadot::para_id(),
			0,
			None,
			0,
		));
	});

	AssetHubPolkadot::execute_with(|| {
		AssetHubPolkadot::assert_xcmp_queue_success(None);
		assert!(matches!(
			Subscription::<AssetHubRuntime>::get(),
			indiv_pallet_members_subscriber::types::SubscriptionStatus::Active { .. },
		));
		assert_eq!(
			RingCollectionExponents::<AssetHubRuntime>::get(collection),
			Some(ring_exponent)
		);

		let received = MembersSubscriber::<AssetHubRuntime>::current_ring_roots(&collection, 0)
			.expect("root was delivered");
		assert_eq!(received.len(), 1);
		assert_eq!(received[0].revision, 7);
		assert_eq!(received[0].root, root);

		// This is deliberately narrow: PGAS asset *creation* remains covered by the
		// migration/chopsticks finding, while this assertion protects the runtime wiring.
		assert_eq!(
			asset_hub_polkadot_runtime::individuality::PgasAssetId::get(),
			asset_hub_polkadot_runtime::individuality::PGAS_ASSET_ID,
		);
	});
}
