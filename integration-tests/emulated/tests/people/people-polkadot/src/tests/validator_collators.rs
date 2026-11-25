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

use crate::*;
use asset_hub_polkadot_runtime::staking::{AnnounceValidatorSet, ValidatorSetToSystemChains};
use pallet_staking_async::OnEraStart;
use pallet_validator_collators::{CollatorDraw, DrawnCollators, MaxCollators};
use people_polkadot_runtime::xcm_config::AssetHubLocation;
use polkadot_system_emulated_network::{
	bulletin_polkadot_emulated_chain::bulletin_polkadot_runtime,
	BulletinPolkadotPara as BulletinPolkadot,
};
use sp_keyring::Sr25519Keyring;
use sp_staking::ServiceKind;

type AssetHubRuntime = <AssetHubPolkadot as Chain>::Runtime;
type PeopleRuntime = <PeoplePolkadot as Chain>::Runtime;
type BulletinRuntime = <BulletinPolkadot as Chain>::Runtime;

/// Era 1 is active on Asset Hub with `who` exposed, so credited points reach `who`.
fn activate_era_one_on_asset_hub_with(who: &AccountId) {
	AssetHubPolkadot::execute_with(|| {
		pallet_staking_async::ActiveEra::<AssetHubRuntime>::put(
			pallet_staking_async::ActiveEraInfo { index: 1, start: Some(0) },
		);
		pallet_staking_async::session_rotation::Eras::<AssetHubRuntime>::upsert_exposure(
			1,
			who,
			pallet_staking_async::Exposure { total: 1, own: 1, others: vec![] },
		);
	});
}

fn collation_points_on_asset_hub() -> Vec<(AccountId, u32)> {
	AssetHubPolkadot::execute_with(|| {
		pallet_staking_async::ErasRewardPointsByService::<AssetHubRuntime>::get(
			1,
			ServiceKind::Collation,
		)
		.individual
		.into_iter()
		.collect()
	})
}

#[test]
fn era_start_on_asset_hub_reaches_people_and_bulletin_without_drawing_collators_under_no_cap() {
	let validators: Vec<AccountId> = [Sr25519Keyring::Alice, Sr25519Keyring::Bob]
		.into_iter()
		.map(|keyring| keyring.to_account_id())
		.collect();
	let people = PeoplePolkadot::para_id();
	let bulletin = BulletinPolkadot::para_id();

	// GIVEN the default cap on every chain and the collators each chain has
	let people_before = PeoplePolkadot::execute_with(|| {
		assert_eq!(MaxCollators::<PeopleRuntime>::get(), CollatorDraw::None);
		(
			pallet_session::Pallet::<PeopleRuntime>::current_index(),
			pallet_session::Pallet::<PeopleRuntime>::validators(),
		)
	});
	let bulletin_before = BulletinPolkadot::execute_with(|| {
		assert_eq!(MaxCollators::<BulletinRuntime>::get(), CollatorDraw::None);
		(
			pallet_session::Pallet::<BulletinRuntime>::current_index(),
			pallet_session::Pallet::<BulletinRuntime>::validators(),
		)
	});

	// WHEN governance has turned validator collation on and Asset Hub starts era 1
	AssetHubPolkadot::execute_with(|| {
		use asset_hub_polkadot_runtime::dynamic_params::validator_collation;
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		assert_ok!(asset_hub_polkadot_runtime::Parameters::set_parameter(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::root(),
			asset_hub_polkadot_runtime::RuntimeParameters::ValidatorCollation(
				validator_collation::Parameters::Enabled(validator_collation::Enabled, Some(true)),
			),
		));
		AnnounceValidatorSet::on_era_start(1, &validators);
		// THEN it stores the set and sends it to People and Bulletin
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::ValidatorCollators(
					pallet_validator_collators::Event::ValidatorSetReceived { era: 1, count: 2 }
				) => {},
				RuntimeEvent::ValidatorSetAnnouncer(
					pallet_validator_set_announcer::Event::AnnouncementSent { destination, era: 1 }
				) => { destination: *destination == people, },
				RuntimeEvent::ValidatorSetAnnouncer(
					pallet_validator_set_announcer::Event::AnnouncementSent { destination, era: 1 }
				) => { destination: *destination == bulletin, },
			]
		);
	});
	// AND both chains store it
	PeoplePolkadot::execute_with(|| {
		type RuntimeEvent = <PeoplePolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			PeoplePolkadot,
			vec![
				RuntimeEvent::ValidatorCollators(
					pallet_validator_collators::Event::ValidatorSetReceived { era: 1, count: 2 }
				) => {},
			]
		);
	});
	BulletinPolkadot::execute_with(|| {
		type RuntimeEvent = <BulletinPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			BulletinPolkadot,
			vec![
				RuntimeEvent::ValidatorCollators(
					pallet_validator_collators::Event::ValidatorSetReceived { era: 1, count: 2 }
				) => {},
			]
		);
	});

	// THEN after the two forced rotations no validator is drawn and the same collators collate.
	// Every `execute_with` runs one block.
	PeoplePolkadot::execute_with(|| {});
	PeoplePolkadot::execute_with(|| {
		assert_eq!(
			DrawnCollators::<PeopleRuntime>::get().map(|drawn| (drawn.era, drawn.collators.len())),
			Some((1, 0))
		);
		assert_eq!(pallet_session::Pallet::<PeopleRuntime>::current_index(), people_before.0 + 2);
		assert_eq!(pallet_session::Pallet::<PeopleRuntime>::validators(), people_before.1);
	});
	BulletinPolkadot::execute_with(|| {});
	BulletinPolkadot::execute_with(|| {
		assert_eq!(
			DrawnCollators::<BulletinRuntime>::get()
				.map(|drawn| (drawn.era, drawn.collators.len())),
			Some((1, 0))
		);
		assert_eq!(
			pallet_session::Pallet::<BulletinRuntime>::current_index(),
			bulletin_before.0 + 2
		);
		assert_eq!(pallet_session::Pallet::<BulletinRuntime>::validators(), bulletin_before.1);
	});
}

#[test]
fn every_chain_caches_the_relay_epoch_randomness_from_the_next_block() {
	// GIVEN no chain has a cached relay chain epoch randomness
	AssetHubPolkadot::execute_with(|| {
		pallet_validator_collators::EpochRandomness::<AssetHubRuntime>::kill()
	});
	PeoplePolkadot::execute_with(|| {
		pallet_validator_collators::EpochRandomness::<PeopleRuntime>::kill()
	});
	BulletinPolkadot::execute_with(|| {
		pallet_validator_collators::EpochRandomness::<BulletinRuntime>::kill()
	});

	// WHEN the next block processes its relay chain state proof
	// THEN the pallet, wired as `OnSystemEvent`, caches the randomness, which is a zero value in
	// the emulator's relay state proof
	AssetHubPolkadot::execute_with(|| {
		assert_eq!(
			pallet_validator_collators::EpochRandomness::<AssetHubRuntime>::get(),
			Some([0; 32])
		)
	});
	PeoplePolkadot::execute_with(|| {
		assert_eq!(
			pallet_validator_collators::EpochRandomness::<PeopleRuntime>::get(),
			Some([0; 32])
		)
	});
	BulletinPolkadot::execute_with(|| {
		assert_eq!(
			pallet_validator_collators::EpochRandomness::<BulletinRuntime>::get(),
			Some([0; 32])
		)
	});
}

#[test]
fn people_blocks_earn_points_on_asset_hub_only_after_governance_sets_them() {
	type PeopleValidatorCollators = pallet_validator_collators::Pallet<PeopleRuntime>;
	let alice = Sr25519Keyring::Alice.to_account_id();
	let people = PeoplePolkadot::para_id();
	let report_three_blocks_of_alice = || {
		PeoplePolkadot::execute_with(|| {
			type RuntimeEvent = <PeoplePolkadot as Chain>::RuntimeEvent;
			for _ in 0..3 {
				<PeopleValidatorCollators as pallet_authorship::EventHandler<_, _>>::note_author(
					alice.clone(),
				);
			}
			// The emulator does not run `on_poll`.
			<PeopleValidatorCollators as frame_support::traits::Hooks<_>>::on_poll(
				pallet_validator_collators::ReportPeriod::<PeopleRuntime>::get(),
				&mut frame_support::weights::WeightMeter::new(),
			);
			assert_expected_events!(
				PeoplePolkadot,
				vec![
					RuntimeEvent::ValidatorCollators(
						pallet_validator_collators::Event::BlocksReported { reported: 1, .. }
					) => {},
				]
			);
		});
	};

	// GIVEN era 1 is active on Asset Hub with Alice exposed, and People stores a set with Alice
	activate_era_one_on_asset_hub_with(&alice);
	PeoplePolkadot::execute_with(|| {
		assert_ok!(PeopleValidatorCollators::set_validators(
			pallet_xcm::Origin::Xcm(AssetHubLocation::get()).into(),
			1,
			[alice.clone()]
				.into_iter()
				.collect::<std::collections::BTreeSet<_>>()
				.try_into()
				.unwrap(),
		));
	});

	// WHEN People reports Alice's blocks with the default points per service
	report_three_blocks_of_alice();
	// THEN Asset Hub executes the report and credits nothing
	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::StakingPointsConverter(
					pallet_staking_async_points_converter::Event::BlocksIgnored { source, blocks: 3 }
				) => { source: *source == people, },
			]
		);
	});
	assert_eq!(collation_points_on_asset_hub(), vec![]);

	// WHEN governance sets 5 points per People block and People reports again
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(pallet_staking_async_points_converter::Pallet::<AssetHubRuntime>::set_points_per_service(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::root(),
			people,
			ServiceKind::Collation,
			5,
		));
	});
	report_three_blocks_of_alice();
	// THEN Asset Hub credits Alice's blocks as collation points
	assert_eq!(collation_points_on_asset_hub(), vec![(alice, 15)]);
}

#[test]
fn bulletin_blocks_earn_points_on_asset_hub_only_after_governance_sets_them() {
	type BulletinValidatorCollators = pallet_validator_collators::Pallet<BulletinRuntime>;
	let alice = Sr25519Keyring::Alice.to_account_id();
	let bulletin = BulletinPolkadot::para_id();
	let report_three_blocks_of_alice = || {
		BulletinPolkadot::execute_with(|| {
			for _ in 0..3 {
				<BulletinValidatorCollators as pallet_authorship::EventHandler<_, _>>::note_author(
					alice.clone(),
				);
			}
			// The emulator does not run `on_poll`.
			<BulletinValidatorCollators as frame_support::traits::Hooks<_>>::on_poll(
				pallet_validator_collators::ReportPeriod::<BulletinRuntime>::get(),
				&mut frame_support::weights::WeightMeter::new(),
			);
		});
	};

	// GIVEN era 1 is active on Asset Hub with Alice exposed, and Bulletin stores a set with Alice
	activate_era_one_on_asset_hub_with(&alice);
	BulletinPolkadot::execute_with(|| {
		assert_ok!(BulletinValidatorCollators::set_validators(
			pallet_xcm::Origin::Xcm(AssetHubLocation::get()).into(),
			1,
			[alice.clone()]
				.into_iter()
				.collect::<std::collections::BTreeSet<_>>()
				.try_into()
				.unwrap(),
		));
	});

	// WHEN Bulletin reports Alice's blocks with the default points per service
	report_three_blocks_of_alice();
	// THEN Asset Hub executes the report and credits nothing
	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::StakingPointsConverter(
					pallet_staking_async_points_converter::Event::BlocksIgnored { source, blocks: 3 }
				) => { source: *source == bulletin, },
			]
		);
	});
	assert_eq!(collation_points_on_asset_hub(), vec![]);

	// WHEN governance sets 5 points per Bulletin block and Bulletin reports again
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(pallet_staking_async_points_converter::Pallet::<AssetHubRuntime>::set_points_per_service(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::root(),
			bulletin,
			ServiceKind::Collation,
			5,
		));
	});
	report_three_blocks_of_alice();
	// THEN Asset Hub credits Alice's blocks as collation points
	assert_eq!(collation_points_on_asset_hub(), vec![(alice, 15)]);
}

/// The largest list of blocks a chain reports, one entry per validator of a full set.
fn full_report() -> Vec<(AccountId, u32)> {
	(0..<asset_hub_polkadot_runtime::staking::MaxValidatorSet as frame_support::traits::Get<u32>>::get())
		.map(|i| {
			let mut account = [0u8; 32];
			account[..4].copy_from_slice(&i.to_le_bytes());
			(AccountId::from(account), i + 1)
		})
		.collect()
}

#[test]
fn reports_of_people_and_bulletin_decode_on_asset_hub() {
	// GIVEN the largest call People and Bulletin send to report blocks
	let blocks = full_report();
	for encoded in [
		people_polkadot_runtime::ReportBlocksToAssetHub::encoded_call(&blocks),
		bulletin_polkadot_runtime::ReportBlocksToAssetHub::encoded_call(&blocks),
	] {
		// WHEN Asset Hub decodes it
		let call =
			<asset_hub_polkadot_runtime::RuntimeCall as codec::Decode>::decode(&mut &encoded[..]);
		// THEN it is `report_authored_blocks` of `pallet-staking-async-points-converter` with
		// the same entries
		match call {
			Ok(asset_hub_polkadot_runtime::RuntimeCall::StakingPointsConverter(
				pallet_staking_async_points_converter::Call::report_authored_blocks {
					blocks: decoded,
				},
			)) => assert_eq!(decoded, blocks),
			other => panic!("the report decodes on Asset Hub as {other:?}"),
		}
	}
}

#[test]
fn validator_set_of_asset_hub_decodes_on_people_and_bulletin() {
	// GIVEN the largest validator set Asset Hub sends
	let validators: Vec<AccountId> = full_report().into_iter().map(|(who, _)| who).collect();
	let encoded = ValidatorSetToSystemChains::encoded_call(7, &validators);
	let mut sorted = validators.clone();
	sorted.sort();

	// WHEN People decodes it
	// THEN it is `set_validators` of `pallet-validator-collators` with the same set
	match <people_polkadot_runtime::RuntimeCall as codec::Decode>::decode(&mut &encoded[..]) {
		Ok(people_polkadot_runtime::RuntimeCall::ValidatorCollators(
			pallet_validator_collators::Call::set_validators { era: 7, validators: decoded },
		)) => assert_eq!(decoded.into_iter().collect::<Vec<_>>(), sorted),
		other => panic!("the set decodes on People as {other:?}"),
	}
	// WHEN Bulletin decodes it
	// THEN it is the same call
	match <bulletin_polkadot_runtime::RuntimeCall as codec::Decode>::decode(&mut &encoded[..]) {
		Ok(bulletin_polkadot_runtime::RuntimeCall::ValidatorCollators(
			pallet_validator_collators::Call::set_validators { era: 7, validators: decoded },
		)) => assert_eq!(decoded.into_iter().collect::<Vec<_>>(), sorted),
		other => panic!("the set decodes on Bulletin as {other:?}"),
	}
}

/// Sends `call` from Asset Hub to `destination` for `origin` to execute there, as a
/// referendum's `pallet_xcm::send` does.
fn send_transact_from_asset_hub(
	origin: <AssetHubPolkadot as Chain>::RuntimeOrigin,
	destination: Location,
	call: Vec<u8>,
) {
	use frame_support::sp_runtime::traits::Dispatchable;
	AssetHubPolkadot::execute_with(|| {
		let send = <AssetHubPolkadot as Chain>::RuntimeCall::PolkadotXcm(pallet_xcm::Call::send {
			dest: bx!(VersionedLocation::from(destination)),
			message: bx!(VersionedXcm::from(Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::Xcm,
					fallback_max_weight: None,
					call: call.into()
				},
			]))),
		});
		assert_ok!(send.dispatch(origin));
	});
}

#[test]
fn asset_hub_staking_admin_sets_the_cap_on_people() {
	use asset_hub_polkadot_runtime::governance::pallet_custom_origins::Origin;
	type RuntimeEvent = <PeoplePolkadot as Chain>::RuntimeEvent;
	let set_cap = |max| {
		people_polkadot_runtime::RuntimeCall::ValidatorCollators(
			pallet_validator_collators::Call::set_max_collators { max },
		)
		.encode()
	};
	// GIVEN the default cap
	PeoplePolkadot::execute_with(|| {
		assert_eq!(MaxCollators::<PeopleRuntime>::get(), CollatorDraw::None)
	});

	// WHEN Asset Hub's GeneralAdmin sends a cap
	send_transact_from_asset_hub(
		Origin::GeneralAdmin.into(),
		AssetHubPolkadot::sibling_location_of(PeoplePolkadot::para_id()),
		set_cap(CollatorDraw::UpTo(1)),
	);
	// THEN People executes the message, `Transact` fails without failing it, and the cap stays
	PeoplePolkadot::execute_with(|| {
		assert_expected_events!(
			PeoplePolkadot,
			vec![
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
		assert_eq!(MaxCollators::<PeopleRuntime>::get(), CollatorDraw::None);
	});

	// WHEN Asset Hub's StakingAdmin, the track of the enabling referenda, sends a cap
	send_transact_from_asset_hub(
		Origin::StakingAdmin.into(),
		AssetHubPolkadot::sibling_location_of(PeoplePolkadot::para_id()),
		set_cap(CollatorDraw::UpTo(3)),
	);
	// THEN People stores it
	PeoplePolkadot::execute_with(|| {
		assert_expected_events!(
			PeoplePolkadot,
			vec![
				RuntimeEvent::ValidatorCollators(
					pallet_validator_collators::Event::MaxCollatorsSet { max: CollatorDraw::UpTo(3) }
				) => {},
			]
		);
		assert_eq!(MaxCollators::<PeopleRuntime>::get(), CollatorDraw::UpTo(3));
	});
}

#[test]
fn asset_hub_staking_admin_sets_the_cap_on_bulletin() {
	use asset_hub_polkadot_runtime::governance::pallet_custom_origins::Origin;
	type RuntimeEvent = <BulletinPolkadot as Chain>::RuntimeEvent;
	let set_cap = |max| {
		bulletin_polkadot_runtime::RuntimeCall::ValidatorCollators(
			pallet_validator_collators::Call::set_max_collators { max },
		)
		.encode()
	};
	// GIVEN the default cap
	BulletinPolkadot::execute_with(|| {
		assert_eq!(MaxCollators::<BulletinRuntime>::get(), CollatorDraw::None)
	});

	// WHEN Asset Hub's GeneralAdmin sends a cap
	send_transact_from_asset_hub(
		Origin::GeneralAdmin.into(),
		AssetHubPolkadot::sibling_location_of(BulletinPolkadot::para_id()),
		set_cap(CollatorDraw::UpTo(1)),
	);
	// THEN Bulletin executes the message, `Transact` fails without failing it, and the cap stays
	BulletinPolkadot::execute_with(|| {
		assert_expected_events!(
			BulletinPolkadot,
			vec![
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
		assert_eq!(MaxCollators::<BulletinRuntime>::get(), CollatorDraw::None);
	});

	// WHEN Asset Hub's StakingAdmin, the track of the enabling referenda, sends a cap
	send_transact_from_asset_hub(
		Origin::StakingAdmin.into(),
		AssetHubPolkadot::sibling_location_of(BulletinPolkadot::para_id()),
		set_cap(CollatorDraw::UpTo(3)),
	);
	// THEN Bulletin stores it
	BulletinPolkadot::execute_with(|| {
		assert_expected_events!(
			BulletinPolkadot,
			vec![
				RuntimeEvent::ValidatorCollators(
					pallet_validator_collators::Event::MaxCollatorsSet { max: CollatorDraw::UpTo(3) }
				) => {},
			]
		);
		assert_eq!(MaxCollators::<BulletinRuntime>::get(), CollatorDraw::UpTo(3));
	});
}
