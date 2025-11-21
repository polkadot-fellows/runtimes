// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
use crate::{common::*, imports::*};

use asset_hub_polkadot_runtime::governance::pallet_custom_origins::Origin;

#[test]
fn assethub_can_authorize_upgrade_for_itself() {
	let code_hash = [1u8; 32].into();
	type AssetHubRuntime = <AssetHubPolkadot as Chain>::Runtime;
	type AssetHubRuntimeCall = <AssetHubPolkadot as Chain>::RuntimeCall;
	type AssetHubRuntimeOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;

	let authorize_upgrade =
		AssetHubRuntimeCall::System(frame_system::Call::authorize_upgrade { code_hash });

	// bad origin
	let invalid_origin: AssetHubRuntimeOrigin = Origin::StakingAdmin.into();
	// ok origin
	let ok_origin: AssetHubRuntimeOrigin = Origin::WhitelistedCaller.into();

	let call_hash = call_hash_of::<AssetHubPolkadot>(&authorize_upgrade);

	// Err - when dispatch non-whitelisted
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubPolkadot>(
			authorize_upgrade.clone(),
			ok_origin.clone()
		),
		DispatchError::Module(sp_runtime::ModuleError {
			index: 64,
			error: [3, 0, 0, 0],
			message: Some("CallIsNotWhitelisted")
		})
	);

	// whitelist
	collectives_send_whitelist(
		CollectivesPolkadot::sibling_location_of(<AssetHubPolkadot as Parachain>::para_id()),
		|| {
			AssetHubRuntimeCall::Whitelist(
				pallet_whitelist::Call::<AssetHubRuntime>::whitelist_call { call_hash },
			)
			.encode()
		},
	);
	AssetHubPolkadot::execute_with(|| {
		assert_whitelisted!(AssetHubPolkadot, call_hash);
	});

	// Err - when dispatch wrong origin
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubPolkadot>(
			authorize_upgrade.clone(),
			invalid_origin
		),
		DispatchError::BadOrigin
	);

	// check before
	AssetHubPolkadot::execute_with(|| {
		assert!(<AssetHubPolkadot as Chain>::System::authorized_upgrade().is_none())
	});

	// ok - authorized
	assert_ok!(dispatch_whitelisted_call_with_preimage::<AssetHubPolkadot>(
		authorize_upgrade,
		ok_origin
	));

	// check after - authorized
	AssetHubPolkadot::execute_with(|| {
		assert_eq!(
			<AssetHubPolkadot as Chain>::System::authorized_upgrade().unwrap().code_hash(),
			&code_hash
		)
	});
}

#[test]
fn assethub_can_authorize_upgrade_for_relay_chain() {
	type AssetHubRuntime = <AssetHubPolkadot as Chain>::Runtime;
	type AssetHubRuntimeCall = <AssetHubPolkadot as Chain>::RuntimeCall;
	type AssetHubRuntimeOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;

	let code_hash = [1u8; 32].into();

	let authorize_upgrade = build_xcm_send_authorize_upgrade_call::<AssetHubPolkadot, Polkadot>(
		AssetHubPolkadot::parent_location(),
		&code_hash,
		None,
	);

	// bad origin
	let invalid_origin: AssetHubRuntimeOrigin = Origin::StakingAdmin.into();
	// ok origin
	let ok_origin: AssetHubRuntimeOrigin = Origin::WhitelistedCaller.into();

	let call_hash = call_hash_of::<AssetHubPolkadot>(&authorize_upgrade);

	// Err - when dispatch non-whitelisted
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubPolkadot>(
			authorize_upgrade.clone(),
			ok_origin.clone()
		),
		DispatchError::Module(sp_runtime::ModuleError {
			index: 64,
			error: [3, 0, 0, 0],
			message: Some("CallIsNotWhitelisted")
		})
	);

	// whitelist
	collectives_send_whitelist(
		CollectivesPolkadot::sibling_location_of(<AssetHubPolkadot as Parachain>::para_id()),
		|| {
			AssetHubRuntimeCall::Whitelist(
				pallet_whitelist::Call::<AssetHubRuntime>::whitelist_call { call_hash },
			)
			.encode()
		},
	);
	AssetHubPolkadot::execute_with(|| {
		assert_whitelisted!(AssetHubPolkadot, call_hash);
	});

	// Err - when dispatch wrong origin
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubPolkadot>(
			authorize_upgrade.clone(),
			invalid_origin
		),
		DispatchError::BadOrigin
	);

	// check before
	Polkadot::execute_with(|| assert!(<Polkadot as Chain>::System::authorized_upgrade().is_none()));

	// ok - authorized
	assert_ok!(dispatch_whitelisted_call_with_preimage::<AssetHubPolkadot>(
		authorize_upgrade,
		ok_origin
	));

	// check after - authorized
	Polkadot::execute_with(|| {
		assert_eq!(
			<Polkadot as Chain>::System::authorized_upgrade().unwrap().code_hash(),
			&code_hash
		)
	});
}

#[test]
fn assethub_can_authorize_upgrade_for_system_chains() {
	type AssetHubRuntime = <AssetHubPolkadot as Chain>::Runtime;
	type AssetHubRuntimeCall = <AssetHubPolkadot as Chain>::RuntimeCall;
	type AssetHubRuntimeOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;

	let code_hash_bridge_hub = [2u8; 32].into();
	let code_hash_collectives = [3u8; 32].into();
	let code_hash_coretime = [4u8; 32].into();
	let code_hash_people = [5u8; 32].into();

	let authorize_upgrade =
		AssetHubRuntimeCall::Utility(pallet_utility::Call::<AssetHubRuntime>::force_batch {
			calls: vec![
				build_xcm_send_authorize_upgrade_call::<AssetHubPolkadot, BridgeHubPolkadot>(
					AssetHubPolkadot::sibling_location_of(BridgeHubPolkadot::para_id()),
					&code_hash_bridge_hub,
					None,
				),
				build_xcm_send_authorize_upgrade_call::<AssetHubPolkadot, CollectivesPolkadot>(
					AssetHubPolkadot::sibling_location_of(CollectivesPolkadot::para_id()),
					&code_hash_collectives,
					None,
				),
				build_xcm_send_authorize_upgrade_call::<AssetHubPolkadot, CoretimePolkadot>(
					AssetHubPolkadot::sibling_location_of(CoretimePolkadot::para_id()),
					&code_hash_coretime,
					None,
				),
				build_xcm_send_authorize_upgrade_call::<AssetHubPolkadot, PeoplePolkadot>(
					AssetHubPolkadot::sibling_location_of(PeoplePolkadot::para_id()),
					&code_hash_people,
					None,
				),
			],
		});

	// bad origin
	let invalid_origin: AssetHubRuntimeOrigin = Origin::StakingAdmin.into();
	// ok origin
	let ok_origin: AssetHubRuntimeOrigin = Origin::WhitelistedCaller.into();

	let call_hash = call_hash_of::<AssetHubPolkadot>(&authorize_upgrade);

	// Err - when dispatch non-whitelisted
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubPolkadot>(
			authorize_upgrade.clone(),
			ok_origin.clone()
		),
		DispatchError::Module(sp_runtime::ModuleError {
			index: 64,
			error: [3, 0, 0, 0],
			message: Some("CallIsNotWhitelisted")
		})
	);

	// whitelist
	collectives_send_whitelist(
		CollectivesPolkadot::sibling_location_of(<AssetHubPolkadot as Parachain>::para_id()),
		|| {
			AssetHubRuntimeCall::Whitelist(
				pallet_whitelist::Call::<AssetHubRuntime>::whitelist_call { call_hash },
			)
			.encode()
		},
	);
	AssetHubPolkadot::execute_with(|| {
		assert_whitelisted!(AssetHubPolkadot, call_hash);
	});

	// Err - when dispatch wrong origin
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubPolkadot>(
			authorize_upgrade.clone(),
			invalid_origin
		),
		DispatchError::BadOrigin
	);

	// check before
	BridgeHubPolkadot::execute_with(|| {
		assert!(<BridgeHubPolkadot as Chain>::System::authorized_upgrade().is_none())
	});
	CollectivesPolkadot::execute_with(|| {
		assert!(<CollectivesPolkadot as Chain>::System::authorized_upgrade().is_none())
	});
	CoretimePolkadot::execute_with(|| {
		assert!(<CoretimePolkadot as Chain>::System::authorized_upgrade().is_none())
	});
	PeoplePolkadot::execute_with(|| {
		assert!(<PeoplePolkadot as Chain>::System::authorized_upgrade().is_none())
	});

	// ok - authorized
	assert_ok!(dispatch_whitelisted_call_with_preimage::<AssetHubPolkadot>(
		authorize_upgrade,
		ok_origin
	));

	// check after - authorized
	BridgeHubPolkadot::execute_with(|| {
		assert_eq!(
			<BridgeHubPolkadot as Chain>::System::authorized_upgrade().unwrap().code_hash(),
			&code_hash_bridge_hub
		)
	});
	CollectivesPolkadot::execute_with(|| {
		assert_eq!(
			<CollectivesPolkadot as Chain>::System::authorized_upgrade()
				.unwrap()
				.code_hash(),
			&code_hash_collectives
		)
	});
	CoretimePolkadot::execute_with(|| {
		assert_eq!(
			<CoretimePolkadot as Chain>::System::authorized_upgrade().unwrap().code_hash(),
			&code_hash_coretime
		)
	});
	PeoplePolkadot::execute_with(|| {
		assert_eq!(
			<PeoplePolkadot as Chain>::System::authorized_upgrade().unwrap().code_hash(),
			&code_hash_people
		)
	});
}

#[test]
fn assethub_fellowship_admin_can_manage_fellowship_on_collectives() {
	type AssetHubOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;
	type CollectivesRuntime = <CollectivesPolkadot as Chain>::Runtime;
	type CollectivesRuntimeEvent = <CollectivesPolkadot as Chain>::RuntimeEvent;

	let account: <CollectivesRuntime as frame_system::Config>::AccountId = Charlie.to_account_id();
	let ok_origin: AssetHubOrigin = Origin::FellowshipAdmin.into();
	let bad_origin: AssetHubOrigin = Origin::StakingAdmin.into();

	let set_params_xcm = build_xcm_send_fellowship_core_set_rank1_min_promotion_period::<
		AssetHubPolkadot,
		CollectivesPolkadot,
		pallet_core_fellowship::Instance1,
	>(
		AssetHubPolkadot::sibling_location_of(<CollectivesPolkadot as Parachain>::para_id()),
		1,
		None,
	);
	let induct_member_xcm = build_xcm_send_induct_member::<
		AssetHubPolkadot,
		CollectivesPolkadot,
		pallet_core_fellowship::Instance1,
	>(
		AssetHubPolkadot::sibling_location_of(<CollectivesPolkadot as Parachain>::para_id()),
		account.clone(),
		None,
	);
	let promote_member_xcm = build_xcm_send_fellowship_core_promote_member::<
		AssetHubPolkadot,
		CollectivesPolkadot,
		pallet_core_fellowship::Instance1,
	>(
		AssetHubPolkadot::sibling_location_of(<CollectivesPolkadot as Parachain>::para_id()),
		account.clone(),
		collectives_polkadot_runtime::fellowship::ranks::DAN_1,
		None,
	);
	let demote_member_xcm = build_xcm_send_fellowship_demote_member::<
		AssetHubPolkadot,
		CollectivesPolkadot,
		pallet_ranked_collective::Instance1,
	>(
		AssetHubPolkadot::sibling_location_of(<CollectivesPolkadot as Parachain>::para_id()),
		account.clone(),
		None,
	);
	let remove_member_xcm = build_xcm_send_fellowship_remove_member::<
		AssetHubPolkadot,
		CollectivesPolkadot,
		pallet_ranked_collective::Instance1,
	>(
		AssetHubPolkadot::sibling_location_of(<CollectivesPolkadot as Parachain>::para_id()),
		account.clone(),
		0,
		None,
	);

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(set_params_xcm.clone().dispatch(bad_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false,.. }) => {},
			]
		);
	});
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(set_params_xcm.dispatch(ok_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::FellowshipCore(pallet_core_fellowship::Event::ParamsChanged { .. }) => {},
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(induct_member_xcm.clone().dispatch(bad_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false,.. }) => {},
			]
		);
	});
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(induct_member_xcm.dispatch(ok_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::FellowshipCollective(pallet_ranked_collective::Event::MemberAdded { who }) => {
					who: *who == account,
				},
				CollectivesRuntimeEvent::FellowshipCore(pallet_core_fellowship::Event::Inducted { who }) => {
					who: *who == account,
				},
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(promote_member_xcm.clone().dispatch(bad_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false,.. }) => {},
			]
		);
	});
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(promote_member_xcm.dispatch(ok_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::FellowshipCollective(pallet_ranked_collective::Event::RankChanged { who, rank: 1 }) => {
					who: *who == account,
				},
				CollectivesRuntimeEvent::FellowshipCore(pallet_core_fellowship::Event::Promoted { who, to_rank: 1 }) => {
					who: *who == account,
				},
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(demote_member_xcm.clone().dispatch(bad_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false,.. }) => {},
			]
		);
	});
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(demote_member_xcm.dispatch(ok_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::FellowshipCollective(pallet_ranked_collective::Event::RankChanged { who, rank: 0 }) => {
					who: *who == account,
				},
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(remove_member_xcm.clone().dispatch(bad_origin));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false,.. }) => {},
			]
		);
	});
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(remove_member_xcm.dispatch(ok_origin));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::FellowshipCollective(pallet_ranked_collective::Event::MemberRemoved { who, rank }) => {
					who: *who == account,
					rank: *rank == 0,
				},
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});
}

#[test]
fn assethub_fellowship_admin_can_manage_ambassadors_on_collectives() {
	type AssetHubOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;
	type CollectivesRuntime = <CollectivesPolkadot as Chain>::Runtime;
	type CollectivesRuntimeEvent = <CollectivesPolkadot as Chain>::RuntimeEvent;

	let account: <CollectivesRuntime as frame_system::Config>::AccountId = Charlie.to_account_id();
	let ok_origin: AssetHubOrigin = Origin::FellowshipAdmin.into();
	let bad_origin: AssetHubOrigin = Origin::StakingAdmin.into();

	let set_params_xcm = build_xcm_send_fellowship_core_set_rank1_min_promotion_period::<
		AssetHubPolkadot,
		CollectivesPolkadot,
		pallet_core_fellowship::Instance2,
	>(
		AssetHubPolkadot::sibling_location_of(<CollectivesPolkadot as Parachain>::para_id()),
		1,
		None,
	);
	let induct_member_xcm = build_xcm_send_induct_member::<
		AssetHubPolkadot,
		CollectivesPolkadot,
		pallet_core_fellowship::Instance2,
	>(
		AssetHubPolkadot::sibling_location_of(<CollectivesPolkadot as Parachain>::para_id()),
		account.clone(),
		None,
	);
	let promote_member_xcm = build_xcm_send_fellowship_core_promote_member::<
		AssetHubPolkadot,
		CollectivesPolkadot,
		pallet_core_fellowship::Instance2,
	>(
		AssetHubPolkadot::sibling_location_of(<CollectivesPolkadot as Parachain>::para_id()),
		account.clone(),
		collectives_polkadot_runtime::fellowship::ranks::DAN_1,
		None,
	);
	let demote_member_xcm = build_xcm_send_fellowship_demote_member::<
		AssetHubPolkadot,
		CollectivesPolkadot,
		pallet_ranked_collective::Instance2,
	>(
		AssetHubPolkadot::sibling_location_of(<CollectivesPolkadot as Parachain>::para_id()),
		account.clone(),
		None,
	);
	let remove_member_xcm = build_xcm_send_fellowship_remove_member::<
		AssetHubPolkadot,
		CollectivesPolkadot,
		pallet_ranked_collective::Instance2,
	>(
		AssetHubPolkadot::sibling_location_of(<CollectivesPolkadot as Parachain>::para_id()),
		account.clone(),
		0,
		None,
	);

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(set_params_xcm.clone().dispatch(bad_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false,.. }) => {},
			]
		);
	});
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(set_params_xcm.dispatch(ok_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::AmbassadorCore(pallet_core_fellowship::Event::ParamsChanged { .. }) => {},
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(induct_member_xcm.clone().dispatch(bad_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false,.. }) => {},
			]
		);
	});
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(induct_member_xcm.dispatch(ok_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::AmbassadorCollective(pallet_ranked_collective::Event::MemberAdded { who }) => {
					who: *who == account,
				},
				CollectivesRuntimeEvent::AmbassadorCore(pallet_core_fellowship::Event::Inducted { who }) => {
					who: *who == account,
				},
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(promote_member_xcm.clone().dispatch(bad_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false,.. }) => {},
			]
		);
	});
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(promote_member_xcm.dispatch(ok_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::AmbassadorCollective(pallet_ranked_collective::Event::RankChanged { who, rank: 1 }) => {
					who: *who == account,
				},
				CollectivesRuntimeEvent::AmbassadorCore(pallet_core_fellowship::Event::Promoted { who, to_rank: 1 }) => {
					who: *who == account,
				},
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(demote_member_xcm.clone().dispatch(bad_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false,.. }) => {},
			]
		);
	});
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(demote_member_xcm.dispatch(ok_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::AmbassadorCollective(pallet_ranked_collective::Event::RankChanged { who, rank: 0 }) => {
					who: *who == account,
				},
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(remove_member_xcm.clone().dispatch(bad_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false,.. }) => {},
			]
		);
	});
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(remove_member_xcm.dispatch(ok_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::AmbassadorCollective(pallet_ranked_collective::Event::MemberRemoved { who, rank }) => {
					who: *who == account,
					rank: *rank == 0,
				},
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});
}

#[test]
fn assethub_staking_admin_can_manage_collator_config_on_other_chains() {
	type AssetHubOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;
	type CoretimeRuntimeEvent = <CoretimePolkadot as Chain>::RuntimeEvent;
	type BridgeHubRuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;
	type PeopleRuntimeEvent = <PeoplePolkadot as Chain>::RuntimeEvent;
	type CollectivesRuntimeEvent = <CollectivesPolkadot as Chain>::RuntimeEvent;

	let ok_origin: AssetHubOrigin = Origin::StakingAdmin.into();
	let bad_origin: AssetHubOrigin = Origin::FellowshipAdmin.into();

	let new_desired_candidates = 23; // random number

	let set_candidates_xcm_coretime =
		build_xcm_send_set_desired_candidates::<AssetHubPolkadot, CoretimePolkadot>(
			AssetHubPolkadot::sibling_location_of(CoretimePolkadot::para_id()),
			new_desired_candidates,
			None,
		);
	let set_candidates_xcm_bridge_hub =
		build_xcm_send_set_desired_candidates::<AssetHubPolkadot, BridgeHubPolkadot>(
			AssetHubPolkadot::sibling_location_of(BridgeHubPolkadot::para_id()),
			new_desired_candidates,
			None,
		);
	let set_candidates_xcm_people =
		build_xcm_send_set_desired_candidates::<AssetHubPolkadot, PeoplePolkadot>(
			AssetHubPolkadot::sibling_location_of(PeoplePolkadot::para_id()),
			new_desired_candidates,
			None,
		);
	let set_candidates_xcm_collectives =
		build_xcm_send_set_desired_candidates::<AssetHubPolkadot, CollectivesPolkadot>(
			AssetHubPolkadot::sibling_location_of(CollectivesPolkadot::para_id()),
			new_desired_candidates,
			None,
		);

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(set_candidates_xcm_coretime.clone().dispatch(bad_origin.clone()));
	});
	CoretimePolkadot::execute_with(|| {
		assert_expected_events!(
			CoretimePolkadot,
			vec![
				CoretimeRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false,.. }) => {},
			]
		);
	});
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(set_candidates_xcm_coretime.dispatch(ok_origin.clone()));
	});
	CoretimePolkadot::execute_with(|| {
		assert_expected_events!(
			CoretimePolkadot,
			vec![
				CoretimeRuntimeEvent::CollatorSelection(pallet_collator_selection::Event::NewDesiredCandidates { desired_candidates }) => {
					desired_candidates: *desired_candidates == new_desired_candidates,
				},
				CoretimeRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(set_candidates_xcm_bridge_hub.clone().dispatch(bad_origin.clone()));
	});
	BridgeHubPolkadot::execute_with(|| {
		assert_expected_events!(
			BridgeHubPolkadot,
			vec![
				BridgeHubRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false,.. }) => {},
			]
		);
	});
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(set_candidates_xcm_bridge_hub.dispatch(ok_origin.clone()));
	});
	BridgeHubPolkadot::execute_with(|| {
		assert_expected_events!(
			BridgeHubPolkadot,
			vec![
				BridgeHubRuntimeEvent::CollatorSelection(pallet_collator_selection::Event::NewDesiredCandidates { desired_candidates }) => {
					desired_candidates: *desired_candidates == new_desired_candidates,
				},
				BridgeHubRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(set_candidates_xcm_people.clone().dispatch(bad_origin.clone()));
	});
	PeoplePolkadot::execute_with(|| {
		assert_expected_events!(
			PeoplePolkadot,
			vec![
				PeopleRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false,.. }) => {},
			]
		);
	});
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(set_candidates_xcm_people.dispatch(ok_origin.clone()));
	});
	PeoplePolkadot::execute_with(|| {
		assert_expected_events!(
			PeoplePolkadot,
			vec![
				PeopleRuntimeEvent::CollatorSelection(pallet_collator_selection::Event::NewDesiredCandidates { desired_candidates }) => {
					desired_candidates: *desired_candidates == new_desired_candidates,
				},
				PeopleRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(set_candidates_xcm_collectives.clone().dispatch(bad_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false,.. }) => {},
			]
		);
	});
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(set_candidates_xcm_collectives.dispatch(ok_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::CollatorSelection(pallet_collator_selection::Event::NewDesiredCandidates { desired_candidates }) => {
					desired_candidates: *desired_candidates == new_desired_candidates,
				},
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});
}

#[test]
fn assethub_general_admin_can_manage_hrmp_on_relay() {
	type AssetHubOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;
	type PolkadotRuntimeEvent = <Polkadot as Chain>::RuntimeEvent;

	let ok_origin: AssetHubOrigin = Origin::GeneralAdmin.into();
	let bad_origin: AssetHubOrigin = Origin::StakingAdmin.into();

	let force_clean_hrmp_xcm = build_xcm_send_force_clean_hrmp::<AssetHubPolkadot, Polkadot>(
		AssetHubPolkadot::parent_location(),
		PeoplePolkadot::para_id(),
		0,
		0,
		None,
	);

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(force_clean_hrmp_xcm.clone().dispatch(bad_origin.clone()));
	});
	Polkadot::execute_with(|| {
		assert_expected_events!(
			Polkadot,
			vec![
				PolkadotRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false,.. }) => {},
			]
		);
	});
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(force_clean_hrmp_xcm.dispatch(ok_origin.clone()));
	});
	Polkadot::execute_with(|| {
		assert_expected_events!(
			Polkadot,
			vec![
				PolkadotRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});
}

#[test]
fn assethub_staking_admin_can_manage_staking_on_relay() {
	type AssetHubOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;
	type PolkadotRuntimeEvent = <Polkadot as Chain>::RuntimeEvent;

	let ok_origin: AssetHubOrigin = Origin::StakingAdmin.into();
	let bad_origin: AssetHubOrigin = Origin::GeneralAdmin.into();

	let set_min_commisions_xcm = build_xcm_send_set_min_commissions::<AssetHubPolkadot, Polkadot>(
		AssetHubPolkadot::parent_location(),
		sp_runtime::Perbill::from_percent(80),
		None,
	);

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(set_min_commisions_xcm.clone().dispatch(bad_origin.clone()));
	});
	Polkadot::execute_with(|| {
		assert_expected_events!(
			Polkadot,
			vec![
				PolkadotRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false,.. }) => {},
			]
		);
	});
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(set_min_commisions_xcm.dispatch(ok_origin.clone()));
	});
	Polkadot::execute_with(|| {
		assert_expected_events!(
			Polkadot,
			vec![
				PolkadotRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});
}

#[test]
fn assethub_staking_admin_can_manage_elections_on_relay() {
	type AssetHubOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;
	type PolkadotRuntimeEvent = <Polkadot as Chain>::RuntimeEvent;

	let ok_origin: AssetHubOrigin = Origin::StakingAdmin.into();
	let bad_origin: AssetHubOrigin = Origin::GeneralAdmin.into();

	let new_score =
		sp_npos_elections::ElectionScore { minimal_stake: 0, sum_stake: 0, sum_stake_squared: 0 };

	let set_minimum_untrusted_score_xcm =
		build_xcm_send_set_minimum_untrusted_score::<AssetHubPolkadot, Polkadot>(
			AssetHubPolkadot::parent_location(),
			Some(new_score),
			None,
		);

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(set_minimum_untrusted_score_xcm.clone().dispatch(bad_origin.clone()));
	});
	Polkadot::execute_with(|| {
		assert_expected_events!(
			Polkadot,
			vec![
				PolkadotRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false,.. }) => {},
			]
		);
	});
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(set_minimum_untrusted_score_xcm.dispatch(ok_origin.clone()));
	});
	Polkadot::execute_with(|| {
		assert_expected_events!(
			Polkadot,
			vec![
				PolkadotRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});
}

#[test]
fn assethub_treasurer_can_manage_asset_rate_on_collectives() {
	type AssetHubOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;
	type CollectivesRuntimeEvent = <CollectivesPolkadot as Chain>::RuntimeEvent;

	let ok_origin: AssetHubOrigin = Origin::Treasurer.into();
	let bad_origin: AssetHubOrigin = Origin::GeneralAdmin.into();

	let test_asset_kind = polkadot_runtime_common::impls::VersionedLocatableAsset::V5 {
		location: Location::new(0, [Parachain(1004)]),
		asset_id: Location::parent().into(),
	};
	let test_rate = 100.into();
	let updated_test_rate = test_rate * 2.into();

	let asset_rate_create_xcm =
		build_xcm_send_asset_rate_create::<AssetHubPolkadot, CollectivesPolkadot>(
			AssetHubPolkadot::sibling_location_of(CollectivesPolkadot::para_id()),
			test_asset_kind.clone(),
			test_rate,
			None,
		);

	let asset_rate_update_xcm =
		build_xcm_send_asset_rate_update::<AssetHubPolkadot, CollectivesPolkadot>(
			AssetHubPolkadot::sibling_location_of(CollectivesPolkadot::para_id()),
			test_asset_kind.clone(),
			updated_test_rate,
			None,
		);

	let asset_rate_remove_xcm =
		build_xcm_send_asset_rate_remove::<AssetHubPolkadot, CollectivesPolkadot>(
			AssetHubPolkadot::sibling_location_of(CollectivesPolkadot::para_id()),
			test_asset_kind.clone(),
			None,
		);

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(asset_rate_create_xcm.clone().dispatch(bad_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false,.. }) => {},
			]
		);
	});
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(asset_rate_create_xcm.dispatch(ok_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
				CollectivesRuntimeEvent::AssetRate(pallet_asset_rate::Event::AssetRateCreated { asset_kind, rate }) => {
					asset_kind: *asset_kind == test_asset_kind,
					rate: *rate == test_rate,
				},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(asset_rate_update_xcm.clone().dispatch(bad_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false,.. }) => {},
			]
		);
	});
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(asset_rate_update_xcm.dispatch(ok_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
				CollectivesRuntimeEvent::AssetRate(pallet_asset_rate::Event::AssetRateUpdated { asset_kind, old, new }) => {
					asset_kind: *asset_kind == test_asset_kind,
					old: *old == test_rate,
					new: *new == updated_test_rate,
				},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(asset_rate_remove_xcm.clone().dispatch(bad_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false,.. }) => {},
			]
		);
	});
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(asset_rate_remove_xcm.dispatch(ok_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
				CollectivesRuntimeEvent::AssetRate(pallet_asset_rate::Event::AssetRateRemoved { asset_kind }) => {
					asset_kind: *asset_kind == test_asset_kind,
				},
			]
		);
	});
}

#[test]
fn assethub_treasurer_can_manage_spend_from_fellowship_treasury_on_collectives() {
	type AssetHubOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;
	type CollectivesRuntimeEvent = <CollectivesPolkadot as Chain>::RuntimeEvent;

	let ok_origin: AssetHubOrigin = Origin::Treasurer.into();
	let bad_origin: AssetHubOrigin = Origin::GeneralAdmin.into();

	let test_asset_kind = polkadot_runtime_common::impls::VersionedLocatableAsset::V5 {
		location: Location::new(1, [Parachain(1004)]),
		asset_id: Location::parent().into(),
	};
	let test_amount = 1_000u128;
	let test_beneficiary: VersionedLocation = VersionedLocation::V5(Location::new(
		0,
		[AccountId32 { network: None::<NetworkId>, id: Charlie.into() }],
	));
	let test_valid_from = 0u32;
	let expected_index = 0u32;

	let treasury_spend_xcm: asset_hub_polkadot_runtime::RuntimeCall = build_xcm_send_treasury_spend::<
		AssetHubPolkadot,
		CollectivesPolkadot,
		pallet_treasury::Instance1,
		_,
	>(
		AssetHubPolkadot::sibling_location_of(CollectivesPolkadot::para_id()),
		test_asset_kind.clone(),
		test_amount,
		test_beneficiary.clone(),
		Some(test_valid_from),
		None,
	);

	let treasury_void_spend_xcm: asset_hub_polkadot_runtime::RuntimeCall =
		build_xcm_send_treasury_void_spend::<
			AssetHubPolkadot,
			CollectivesPolkadot,
			pallet_treasury::Instance1,
		>(
			AssetHubPolkadot::sibling_location_of(CollectivesPolkadot::para_id()),
			expected_index,
			None,
		);

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(treasury_spend_xcm.clone().dispatch(bad_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false, .. }) => {},
			]
		);
	});
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(treasury_spend_xcm.dispatch(ok_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
				CollectivesRuntimeEvent::FellowshipTreasury(pallet_treasury::Event::AssetSpendApproved { index, asset_kind, amount, beneficiary, valid_from, .. }) => {
					index: *index == expected_index,
					asset_kind: *asset_kind == test_asset_kind,
					amount: *amount == test_amount,
					beneficiary: *beneficiary == test_beneficiary,
					valid_from: *valid_from == test_valid_from,
				},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(treasury_void_spend_xcm.clone().dispatch(bad_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false, .. }) => {},
			]
		);
	});
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(treasury_void_spend_xcm.dispatch(ok_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
				CollectivesRuntimeEvent::FellowshipTreasury(pallet_treasury::Event::AssetSpendVoided { index }) => {
					index: *index == expected_index,
				},
			]
		);
	});
}

#[test]
fn assethub_treasurer_can_manage_spend_from_ambassador_treasury_on_collectives() {
	type AssetHubOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;
	type CollectivesRuntimeEvent = <CollectivesPolkadot as Chain>::RuntimeEvent;

	let ok_origin_spend: AssetHubOrigin = Origin::Treasurer.into();
	let ok_origin_void_spend: AssetHubOrigin = Origin::FellowshipAdmin.into();
	let bad_origin: AssetHubOrigin = Origin::GeneralAdmin.into();

	let test_asset_kind = polkadot_runtime_common::impls::VersionedLocatableAsset::V5 {
		location: Location::new(1, [Parachain(1004)]),
		asset_id: Location::parent().into(),
	};
	let test_amount = 1_000u128;
	let test_beneficiary: VersionedLocation = VersionedLocation::V5(Location::new(
		0,
		[AccountId32 { network: None::<NetworkId>, id: Charlie.into() }],
	));
	let test_valid_from = 0u32;
	let expected_index = 0u32;

	let treasury_spend_xcm: asset_hub_polkadot_runtime::RuntimeCall = build_xcm_send_treasury_spend::<
		AssetHubPolkadot,
		CollectivesPolkadot,
		pallet_treasury::Instance2,
		_,
	>(
		AssetHubPolkadot::sibling_location_of(CollectivesPolkadot::para_id()),
		test_asset_kind.clone(),
		test_amount,
		test_beneficiary.clone(),
		Some(test_valid_from),
		None,
	);

	let treasury_void_spend_xcm: asset_hub_polkadot_runtime::RuntimeCall =
		build_xcm_send_treasury_void_spend::<
			AssetHubPolkadot,
			CollectivesPolkadot,
			pallet_treasury::Instance2,
		>(
			AssetHubPolkadot::sibling_location_of(CollectivesPolkadot::para_id()),
			expected_index,
			None,
		);

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(treasury_spend_xcm.clone().dispatch(bad_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false, .. }) => {},
			]
		);
	});
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(treasury_spend_xcm.dispatch(ok_origin_spend.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
				CollectivesRuntimeEvent::AmbassadorTreasury(pallet_treasury::Event::AssetSpendApproved { index, asset_kind, amount, beneficiary, valid_from, .. }) => {
					index: *index == expected_index,
					asset_kind: *asset_kind == test_asset_kind,
					amount: *amount == test_amount,
					beneficiary: *beneficiary == test_beneficiary,
					valid_from: *valid_from == test_valid_from,
				},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(treasury_void_spend_xcm.clone().dispatch(bad_origin.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false, .. }) => {},
			]
		);
	});
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(treasury_void_spend_xcm.dispatch(ok_origin_void_spend.clone()));
	});
	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
				CollectivesRuntimeEvent::AmbassadorTreasury(pallet_treasury::Event::AssetSpendVoided { index }) => {
					index: *index == expected_index,
				},
			]
		);
	});
}
