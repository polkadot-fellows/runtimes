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
		AssetHubRuntimeCall::Utility(pallet_utility::Call::<AssetHubRuntime>::force_batch {
			calls: vec![AssetHubRuntimeCall::System(frame_system::Call::authorize_upgrade {
				code_hash,
			})],
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

	let authorize_upgrade =
		AssetHubRuntimeCall::Utility(pallet_utility::Call::<AssetHubRuntime>::force_batch {
			calls: vec![build_xcm_send_authorize_upgrade_call::<AssetHubPolkadot, Polkadot>(
				AssetHubPolkadot::parent_location(),
				&code_hash,
				None,
			)],
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
fn assethub_fellowsip_admin_can_induct_on_collectives() {
	type AssetHubOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;
	type CollectivesRuntime = <CollectivesPolkadot as Chain>::Runtime;
	type CollectivesRuntimeEvent = <CollectivesPolkadot as Chain>::RuntimeEvent;

	use asset_hub_polkadot_runtime::governance::pallet_custom_origins::Origin::FellowshipAdmin;
	use sp_keyring::Sr25519Keyring::Charlie;

	let account_to_induct: <CollectivesRuntime as frame_system::Config>::AccountId =
		Charlie.to_account_id().into();

	let send_induct_over_xcm = build_xcm_send_induct_member::<
		AssetHubPolkadot,
		CollectivesPolkadot,
		pallet_core_fellowship::Instance1,
	>(
		AssetHubPolkadot::sibling_location_of(<CollectivesPolkadot as Parachain>::para_id()),
		account_to_induct.clone(),
		None,
	);

	let fellows_origin: AssetHubOrigin = FellowshipAdmin.into();

	CollectivesPolkadot::execute_with(|| {
		assert!(!pallet_core_fellowship::Member::<
			CollectivesRuntime,
			pallet_core_fellowship::Instance1,
		>::contains_key(&account_to_induct));
		assert_eq!(pallet_core_fellowship::Member::<CollectivesRuntime, pallet_core_fellowship::Instance1>::get(&account_to_induct), None);
	});

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(send_induct_over_xcm.dispatch(fellows_origin.into()));
	});

	CollectivesPolkadot::execute_with(|| {
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				CollectivesRuntimeEvent::FellowshipCollective(pallet_ranked_collective::Event::MemberAdded { who }) => {
					who: *who == account_to_induct,
				},
				CollectivesRuntimeEvent::FellowshipCore(pallet_core_fellowship::Event::Inducted { who }) => {
					who: *who == account_to_induct,
				},
			]
		);
		assert!(pallet_core_fellowship::Member::<
			CollectivesRuntime,
			pallet_core_fellowship::Instance1,
		>::get(&account_to_induct)
		.is_some());
	});
}
