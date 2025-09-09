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

use crate::{common::collectives_send_whitelist, imports::*};
use asset_hub_kusama_runtime::governance::pallet_custom_origins::Origin;

#[test]
fn assethub_can_authorize_upgrade_for_itself() {
	let code_hash = [1u8; 32].into();
	type AssetHubRuntime = <AssetHubKusama as Chain>::Runtime;
	type AssetHubRuntimeCall = <AssetHubKusama as Chain>::RuntimeCall;
	type AssetHubRuntimeOrigin = <AssetHubKusama as Chain>::RuntimeOrigin;

	let authorize_upgrade =
		AssetHubRuntimeCall::Utility(pallet_utility::Call::<AssetHubRuntime>::force_batch {
			calls: vec![AssetHubRuntimeCall::System(frame_system::Call::authorize_upgrade {
				code_hash,
			})],
		});

	// bad origin
	let invalid_origin: AssetHubRuntimeOrigin = Origin::StakingAdmin.into();
	// ok origin
	let ok_origin: AssetHubRuntimeOrigin = Origin::WhitelistedCaller.into();

	let call_hash = call_hash_of::<AssetHubKusama>(&authorize_upgrade);

	// Err - when dispatch non-whitelisted
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(
			authorize_upgrade.clone(),
			ok_origin.clone()
		),
		DispatchError::Module(sp_runtime::ModuleError {
			index: 94,
			error: [3, 0, 0, 0],
			message: Some("CallIsNotWhitelisted")
		})
	);

	// whitelist from Kusama
	collectives_send_whitelist(
		Kusama::child_location_of(<AssetHubKusama as Parachain>::para_id()),
		|| {
			AssetHubRuntimeCall::Whitelist(
				pallet_whitelist::Call::<AssetHubRuntime>::whitelist_call { call_hash },
			)
			.encode()
		},
	);
	AssetHubKusama::execute_with(|| {
		assert_whitelisted!(AssetHubKusama, call_hash);
	});

	// Err - when dispatch wrong origin
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(
			authorize_upgrade.clone(),
			invalid_origin
		),
		DispatchError::BadOrigin
	);

	// check before
	AssetHubKusama::execute_with(|| {
		assert!(<AssetHubKusama as Chain>::System::authorized_upgrade().is_none())
	});

	// ok - authorized
	assert_ok!(dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(
		authorize_upgrade,
		ok_origin
	));

	// check after - authorized
	AssetHubKusama::execute_with(|| {
		assert_eq!(
			<AssetHubKusama as Chain>::System::authorized_upgrade().unwrap().code_hash(),
			&code_hash
		)
	});
}

#[test]
fn assethub_can_authorize_upgrade_for_relay_chain() {
	type AssetHubRuntime = <AssetHubKusama as Chain>::Runtime;
	type AssetHubRuntimeCall = <AssetHubKusama as Chain>::RuntimeCall;
	type AssetHubRuntimeOrigin = <AssetHubKusama as Chain>::RuntimeOrigin;

	let code_hash = [1u8; 32].into();
	let authorize_upgrade =
		AssetHubRuntimeCall::Utility(pallet_utility::Call::<AssetHubRuntime>::force_batch {
			calls: vec![build_xcm_send_authorize_upgrade_call::<AssetHubKusama, Kusama>(
				AssetHubKusama::parent_location(),
				&code_hash,
				None,
			)],
		});

	// bad origin
	let invalid_origin: AssetHubRuntimeOrigin = Origin::StakingAdmin.into();
	// ok origin
	let ok_origin: AssetHubRuntimeOrigin = Origin::WhitelistedCaller.into();

	let call_hash = call_hash_of::<AssetHubKusama>(&authorize_upgrade);

	// Err - when dispatch non-whitelisted
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(
			authorize_upgrade.clone(),
			ok_origin.clone()
		),
		DispatchError::Module(sp_runtime::ModuleError {
			index: 94,
			error: [3, 0, 0, 0],
			message: Some("CallIsNotWhitelisted")
		})
	);

	// whitelist from Kusama
	collectives_send_whitelist(
		Kusama::child_location_of(<AssetHubKusama as Parachain>::para_id()),
		|| {
			AssetHubRuntimeCall::Whitelist(
				pallet_whitelist::Call::<AssetHubRuntime>::whitelist_call { call_hash },
			)
			.encode()
		},
	);
	AssetHubKusama::execute_with(|| {
		assert_whitelisted!(AssetHubKusama, call_hash);
	});

	// Err - when dispatch wrong origin
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(
			authorize_upgrade.clone(),
			invalid_origin
		),
		DispatchError::BadOrigin
	);

	// check before
	Kusama::execute_with(|| assert!(<Kusama as Chain>::System::authorized_upgrade().is_none()));

	// ok - authorized
	assert_ok!(dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(
		authorize_upgrade,
		ok_origin
	));

	// check after - authorized
	Kusama::execute_with(|| {
		assert_eq!(<Kusama as Chain>::System::authorized_upgrade().unwrap().code_hash(), &code_hash)
	});
}

#[test]
fn assethub_can_authorize_upgrade_for_system_chains() {
	type AssetHubRuntime = <AssetHubKusama as Chain>::Runtime;
	type AssetHubRuntimeCall = <AssetHubKusama as Chain>::RuntimeCall;
	type AssetHubRuntimeOrigin = <AssetHubKusama as Chain>::RuntimeOrigin;

	let code_hash_bridge_hub = [2u8; 32].into();
	let code_hash_coretime = [4u8; 32].into();
	let code_hash_people = [5u8; 32].into();

	let authorize_upgrade =
		AssetHubRuntimeCall::Utility(pallet_utility::Call::<AssetHubRuntime>::force_batch {
			calls: vec![
				build_xcm_send_authorize_upgrade_call::<AssetHubKusama, BridgeHubKusama>(
					AssetHubKusama::sibling_location_of(BridgeHubKusama::para_id()),
					&code_hash_bridge_hub,
					None,
				),
				build_xcm_send_authorize_upgrade_call::<AssetHubKusama, CoretimeKusama>(
					AssetHubKusama::sibling_location_of(CoretimeKusama::para_id()),
					&code_hash_coretime,
					None,
				),
				build_xcm_send_authorize_upgrade_call::<AssetHubKusama, PeopleKusama>(
					AssetHubKusama::sibling_location_of(PeopleKusama::para_id()),
					&code_hash_people,
					None,
				),
			],
		});

	// bad origin
	let invalid_origin: AssetHubRuntimeOrigin = Origin::StakingAdmin.into();
	// ok origin
	let ok_origin: AssetHubRuntimeOrigin = Origin::WhitelistedCaller.into();

	let call_hash = call_hash_of::<AssetHubKusama>(&authorize_upgrade);

	// Err - when dispatch non-whitelisted
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(
			authorize_upgrade.clone(),
			ok_origin.clone()
		),
		DispatchError::Module(sp_runtime::ModuleError {
			index: 94,
			error: [3, 0, 0, 0],
			message: Some("CallIsNotWhitelisted")
		})
	);

	// whitelist from Kusama
	collectives_send_whitelist(
		Kusama::child_location_of(<AssetHubKusama as Parachain>::para_id()),
		|| {
			AssetHubRuntimeCall::Whitelist(
				pallet_whitelist::Call::<AssetHubRuntime>::whitelist_call { call_hash },
			)
			.encode()
		},
	);
	AssetHubKusama::execute_with(|| {
		assert_whitelisted!(AssetHubKusama, call_hash);
	});

	// Err - when dispatch wrong origin
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(
			authorize_upgrade.clone(),
			invalid_origin
		),
		DispatchError::BadOrigin
	);

	// check before
	BridgeHubKusama::execute_with(|| {
		assert!(<BridgeHubKusama as Chain>::System::authorized_upgrade().is_none())
	});
	CoretimeKusama::execute_with(|| {
		assert!(<CoretimeKusama as Chain>::System::authorized_upgrade().is_none())
	});
	PeopleKusama::execute_with(|| {
		assert!(<PeopleKusama as Chain>::System::authorized_upgrade().is_none())
	});

	// ok - authorized
	assert_ok!(dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(
		authorize_upgrade,
		ok_origin
	));

	// check after - authorized
	BridgeHubKusama::execute_with(|| {
		assert_eq!(
			<BridgeHubKusama as Chain>::System::authorized_upgrade().unwrap().code_hash(),
			&code_hash_bridge_hub
		)
	});
	CoretimeKusama::execute_with(|| {
		assert_eq!(
			<CoretimeKusama as Chain>::System::authorized_upgrade().unwrap().code_hash(),
			&code_hash_coretime
		)
	});
	PeopleKusama::execute_with(|| {
		assert_eq!(
			<PeopleKusama as Chain>::System::authorized_upgrade().unwrap().code_hash(),
			&code_hash_people
		)
	});
}

#[test]
fn assethub_fellowship_admin_can_manage_fellowship_on_relay() {
	type AssetHubOrigin = <AssetHubKusama as Chain>::RuntimeOrigin;

	type KusamaRuntime = <Kusama as Chain>::Runtime;
	type KusamaRuntimeEvent = <Kusama as Chain>::RuntimeEvent;

	let account: <KusamaRuntime as frame_system::Config>::AccountId =
		Charlie.to_account_id().into();
	let ok_origin: AssetHubOrigin = Origin::FellowshipAdmin.into();
	let bad_origin: AssetHubOrigin = Origin::StakingAdmin.into();

	let add_member_xcm = build_xcm_send_fellowship_add_member::<
		AssetHubKusama,
		Kusama,
		pallet_ranked_collective::Instance1,
	>(AssetHubKusama::parent_location(), account.clone(), None);
	let promote_member_xcm = build_xcm_send_fellowship_promote_member::<
		AssetHubKusama,
		Kusama,
		pallet_ranked_collective::Instance1,
	>(AssetHubKusama::parent_location(), account.clone(), None);
	let demote_member_xcm = build_xcm_send_fellowship_demote_member::<
		AssetHubKusama,
		Kusama,
		pallet_ranked_collective::Instance1,
	>(AssetHubKusama::parent_location(), account.clone(), None);
	let remove_member_xcm = build_xcm_send_fellowship_remove_member::<
		AssetHubKusama,
		Kusama,
		pallet_ranked_collective::Instance1,
	>(AssetHubKusama::parent_location(), account.clone(), 0, None);

	AssetHubKusama::execute_with(|| {
		assert_ok!(add_member_xcm.clone().dispatch(bad_origin.clone().into()));
	});
	Kusama::execute_with(|| {
		assert_expected_events!(
			Kusama,
			vec![
				KusamaRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false,.. }) => {},
			]
		);
	});
	AssetHubKusama::execute_with(|| {
		assert_ok!(add_member_xcm.dispatch(ok_origin.clone().into()));
	});
	Kusama::execute_with(|| {
		assert_expected_events!(
			Kusama,
			vec![
				KusamaRuntimeEvent::FellowshipCollective(pallet_ranked_collective::Event::MemberAdded { who }) => {
					who: *who == account,
				},
			]
		);
	});

	AssetHubKusama::execute_with(|| {
		assert_ok!(promote_member_xcm.clone().dispatch(bad_origin.clone().into()));
	});
	Kusama::execute_with(|| {
		assert_expected_events!(
			Kusama,
			vec![
				KusamaRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false,.. }) => {},
			]
		);
	});
	AssetHubKusama::execute_with(|| {
		assert_ok!(promote_member_xcm.dispatch(ok_origin.clone().into()));
	});
	Kusama::execute_with(|| {
		assert_expected_events!(
			Kusama,
			vec![
				KusamaRuntimeEvent::FellowshipCollective(pallet_ranked_collective::Event::RankChanged { who, rank: 1 }) => {
					who: *who == account,
				},
			]
		);
	});

	AssetHubKusama::execute_with(|| {
		assert_ok!(demote_member_xcm.clone().dispatch(bad_origin.clone().into()));
	});
	Kusama::execute_with(|| {
		assert_expected_events!(
			Kusama,
			vec![
				KusamaRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false,.. }) => {},
			]
		);
	});
	AssetHubKusama::execute_with(|| {
		assert_ok!(demote_member_xcm.dispatch(ok_origin.clone().into()));
	});
	Kusama::execute_with(|| {
		assert_expected_events!(
			Kusama,
			vec![
				KusamaRuntimeEvent::FellowshipCollective(pallet_ranked_collective::Event::RankChanged { who, rank: 0 }) => {
					who: *who == account,
				},
			]
		);
	});

	AssetHubKusama::execute_with(|| {
		assert_ok!(remove_member_xcm.clone().dispatch(bad_origin.clone().into()));
	});
	Kusama::execute_with(|| {
		assert_expected_events!(
			Kusama,
			vec![
				KusamaRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: false,.. }) => {},
			]
		);
	});
	AssetHubKusama::execute_with(|| {
		assert_ok!(remove_member_xcm.dispatch(ok_origin.clone().into()));
	});
	Kusama::execute_with(|| {
		assert_expected_events!(
			Kusama,
			vec![
				KusamaRuntimeEvent::FellowshipCollective(pallet_ranked_collective::Event::MemberRemoved { who, rank }) => {
					who: *who == account,
					rank: *rank == 0,
				},
			]
		);
	});
}
