// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

use crate::*;
use asset_hub_polkadot_runtime::governance::pallet_custom_origins::Origin;
use cumulus_pallet_parachain_system::{
	relay_state_snapshot::MessagingStateSnapshot, RelevantMessagingState,
};
use frame_support::{assert_noop, sp_runtime::traits::Dispatchable, BoundedVec};
use indiv_pallet_members::{CurrentRingIndex, Root};
use indiv_pallet_members_notifier::{PendingInit, Subscribers, SubscriptionWhitelist};
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

		assert_eq!(
			asset_hub_polkadot_runtime::individuality::PgasAssetId::get(),
			asset_hub_polkadot_runtime::individuality::PGAS_ASSET_ID,
		);
	});
}

fn send_asset_hub_transact_to_people(
	asset_hub_origin: <AssetHubPolkadot as Chain>::RuntimeOrigin,
	people_origin_kind: OriginKind,
	people_call: &<PeoplePolkadot as Chain>::RuntimeCall,
) -> DispatchResult {
	type AssetHubRuntime = <AssetHubPolkadot as Chain>::Runtime;
	type AssetHubRuntimeCall = <AssetHubPolkadot as Chain>::RuntimeCall;

	AssetHubPolkadot::execute_with(|| {
		let send = AssetHubRuntimeCall::PolkadotXcm(pallet_xcm::Call::<AssetHubRuntime>::send {
			dest: bx!(VersionedLocation::from(AssetHubPolkadot::sibling_location_of(
				PeoplePolkadot::para_id()
			))),
			message: bx!(VersionedXcm::from(Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind: people_origin_kind,
					fallback_max_weight: None,
					call: people_call.encode().into(),
				},
			]))),
		});
		send.dispatch(asset_hub_origin).map(|_| ()).map_err(|e| e.error)
	})
}

#[test]
fn asset_hub_governance_creates_a_coinage_instance() {
	type PeopleRuntime = <PeoplePolkadot as Chain>::Runtime;
	type PeopleRuntimeCall = <PeoplePolkadot as Chain>::RuntimeCall;
	type PeopleRuntimeEvent = <PeoplePolkadot as Chain>::RuntimeEvent;
	type PeopleRuntimeOrigin = <PeoplePolkadot as Chain>::RuntimeOrigin;
	type AssetHubRuntimeOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;

	let asset = HollarLocation::get();
	let pallet_account = people_polkadot_runtime::Coinage::pallet_account();

	let create_asset =
		PeopleRuntimeCall::Assets(pallet_assets::Call::<PeopleRuntime>::force_create {
			id: asset.clone(),
			owner: pallet_account.clone().into(),
			is_sufficient: true,
			min_balance: 1,
		});
	let create_instance = PeopleRuntimeCall::Coinage(
		indiv_pallet_coinage::Call::<PeopleRuntime>::create_sufficient_instance {
			asset_id: asset.clone(),
			asset_unit: HOLLAR_UNITS / 100,
		},
	);

	assert_ok!(send_asset_hub_transact_to_people(
		AssetHubRuntimeOrigin::root(),
		OriginKind::Superuser,
		&create_asset,
	));

	PeoplePolkadot::execute_with(|| {
		PeoplePolkadot::assert_xcmp_queue_success(None);
		assert_expected_events!(
			PeoplePolkadot,
			vec![
				PeopleRuntimeEvent::Assets(pallet_assets::Event::ForceCreated { .. }) => {},
			]
		);

		// Emulate the reserve-transferred HOLLAR buffer required before instance creation.
		assert_ok!(people_polkadot_runtime::Assets::mint(
			PeopleRuntimeOrigin::signed(pallet_account.clone()),
			asset.clone(),
			pallet_account.clone().into(),
			1,
		));
	});

	assert_ok!(send_asset_hub_transact_to_people(
		Origin::StakingAdmin.into(),
		OriginKind::Xcm,
		&create_instance,
	));
	PeoplePolkadot::execute_with(|| {
		PeoplePolkadot::assert_xcmp_queue_success(None);
		assert!(indiv_pallet_coinage::Instances::<PeopleRuntime>::get(0).is_none());
	});

	assert_ok!(send_asset_hub_transact_to_people(
		Origin::TechnicalMaintenance.into(),
		OriginKind::Xcm,
		&create_instance,
	));

	PeoplePolkadot::execute_with(|| {
		PeoplePolkadot::assert_xcmp_queue_success(None);
		let instance = indiv_pallet_coinage::Instances::<PeopleRuntime>::get(0)
			.expect("technical_maintenance created the instance");
		assert_eq!(instance.asset_id, asset);
		assert_eq!(instance.asset_unit, HOLLAR_UNITS / 100);
		assert_eq!(instance.mode, indiv_pallet_coinage::InstanceMode::Sufficient);
	});
}

#[test]
fn asset_hub_subscribes_from_the_genesis_whitelist() {
	type PeopleRuntime = <PeoplePolkadot as Chain>::Runtime;
	type PeopleRuntimeOrigin = <PeoplePolkadot as Chain>::RuntimeOrigin;

	PeoplePolkadot::execute_with(|| {
		let para_id = AssetHubPolkadot::para_id();
		let authorized = PeopleRuntimeOrigin::from(frame_system::RawOrigin::Authorized);

		assert_ok!(people_polkadot_runtime::MembersNotifier::subscribe_whitelisted(
			authorized.clone(),
			para_id,
		));

		assert!(Subscribers::<PeopleRuntime>::contains_key(para_id));
		assert!(PendingInit::<PeopleRuntime>::contains_key(para_id));
		assert!(!SubscriptionWhitelist::<PeopleRuntime>::contains_key(para_id));
		assert_noop!(
			people_polkadot_runtime::MembersNotifier::subscribe_whitelisted(authorized, para_id),
			indiv_pallet_members_notifier::Error::<PeopleRuntime>::NotWhitelisted
		);
	});
}
