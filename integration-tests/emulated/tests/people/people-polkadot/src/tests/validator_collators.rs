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
use asset_hub_polkadot_runtime::{
	staking::{AnnounceValidatorSet, ValidatorSetToSystemChains},
	ValidatorCollatorChains,
};
use pallet_staking_async::OnEraStart;
use pallet_validator_collators::{CollatorDraw, DrawnCollators, MaxCollators};
use polkadot_system_emulated_network::{
	bulletin_polkadot_emulated_chain::bulletin_polkadot_runtime,
	BulletinPolkadotPara as BulletinPolkadot,
};
use sp_keyring::Sr25519Keyring;
use sp_staking::ServiceKind;

type AssetHubRuntime = <AssetHubPolkadot as Chain>::Runtime;

/// Runs `$body` once per chain that receives Asset Hub's validator set and reports its blocks
/// there, with `$chain` aliasing the emulated chain and `$runtime` its runtime crate.
/// The pool is the list in the first rule.
macro_rules! for_each_collating_chain {
	(|$chain:ident, $runtime:ident| $body:block) => {
		for_each_collating_chain!(
			[PeoplePolkadot: people_polkadot_runtime, BulletinPolkadot: bulletin_polkadot_runtime]
			|$chain, $runtime| $body
		)
	};
	([$($para:ident: $para_runtime:ident),+] |$chain:ident, $runtime:ident| $body:block) => {
		$({
			let _on_panic = NameOnPanic(stringify!($para));
			#[allow(dead_code)]
			type $chain = $para;
			#[allow(unused_imports)]
			use $para_runtime as $runtime;
			$body
		})+
	};
}

/// Names the chain of the pool whose checks panic, as every chain runs them under one alias.
struct NameOnPanic(&'static str);

impl Drop for NameOnPanic {
	fn drop(&mut self) {
		if std::thread::panicking() {
			eprintln!("failed on {}", self.0);
		}
	}
}

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
fn era_start_on_asset_hub_reaches_each_collating_chain_without_drawing_collators_under_no_cap() {
	let validators: Vec<AccountId> = [Sr25519Keyring::Alice, Sr25519Keyring::Bob]
		.into_iter()
		.map(|keyring| keyring.to_account_id())
		.collect();

	// GIVEN the default cap on every collating chain and the collators each one has
	let mut before = std::collections::BTreeMap::new();
	for_each_collating_chain!(|CollatingChain, runtime| {
		let session = CollatingChain::execute_with(|| {
			assert_eq!(MaxCollators::<runtime::Runtime>::get(), CollatorDraw::None);
			(
				pallet_session::Pallet::<runtime::Runtime>::current_index(),
				pallet_session::Pallet::<runtime::Runtime>::validators(),
			)
		});
		before.insert(CollatingChain::para_id(), session);
	});
	// AND they are the destinations of Asset Hub
	let mut destinations = ValidatorCollatorChains::get();
	destinations.sort();
	assert_eq!(before.keys().copied().collect::<Vec<_>>(), destinations);

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
		// THEN it stores the set
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::ValidatorCollators(
					pallet_validator_collators::Event::ValidatorSetReceived { era: 1, count: 2 }
				) => {},
			]
		);
		// AND sends it to each collating chain
		for expected in &destinations {
			assert_expected_events!(
				AssetHubPolkadot,
				vec![
					RuntimeEvent::ValidatorSetAnnouncer(
						pallet_validator_set_announcer::Event::AnnouncementSent { destination, era: 1 }
					) => { destination: destination == expected, },
				]
			);
		}
	});

	for_each_collating_chain!(|CollatingChain, runtime| {
		type RuntimeEvent = <CollatingChain as Chain>::RuntimeEvent;
		let (session_index, collators) = &before[&CollatingChain::para_id()];
		// AND the chain stores it
		CollatingChain::execute_with(|| {
			assert_expected_events!(
				CollatingChain,
				vec![
					RuntimeEvent::ValidatorCollators(
						pallet_validator_collators::Event::ValidatorSetReceived { era: 1, count: 2 }
					) => {},
				]
			);
		});
		// THEN after the two forced rotations no validator is drawn and the same collators
		// collate. Every `execute_with` runs one block.
		CollatingChain::execute_with(|| {});
		CollatingChain::execute_with(|| {
			assert_eq!(
				DrawnCollators::<runtime::Runtime>::get()
					.map(|drawn| (drawn.era, drawn.collators.len())),
				Some((1, 0))
			);
			assert_eq!(
				pallet_session::Pallet::<runtime::Runtime>::current_index(),
				session_index + 2
			);
			assert_eq!(pallet_session::Pallet::<runtime::Runtime>::validators(), *collators);
		});
	});
}

#[test]
fn every_chain_caches_the_relay_epoch_randomness_from_the_next_block() {
	// GIVEN no chain has a cached relay chain epoch randomness
	AssetHubPolkadot::execute_with(|| {
		pallet_validator_collators::EpochRandomness::<AssetHubRuntime>::kill()
	});
	for_each_collating_chain!(|CollatingChain, runtime| {
		CollatingChain::execute_with(|| {
			pallet_validator_collators::EpochRandomness::<runtime::Runtime>::kill()
		});
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
	for_each_collating_chain!(|CollatingChain, runtime| {
		CollatingChain::execute_with(|| {
			assert_eq!(
				pallet_validator_collators::EpochRandomness::<runtime::Runtime>::get(),
				Some([0; 32])
			)
		});
	});
}

#[test]
fn blocks_of_each_collating_chain_earn_points_on_asset_hub_only_after_governance_sets_them() {
	let alice = Sr25519Keyring::Alice.to_account_id();
	// GIVEN era 1 is active on Asset Hub with Alice exposed
	activate_era_one_on_asset_hub_with(&alice);
	for_each_collating_chain!(|CollatingChain, runtime| {
		type ValidatorCollators = pallet_validator_collators::Pallet<runtime::Runtime>;
		let para_id = CollatingChain::para_id();
		let report_three_blocks_of_alice = || {
			CollatingChain::execute_with(|| {
				type RuntimeEvent = <CollatingChain as Chain>::RuntimeEvent;
				for _ in 0..3 {
					<ValidatorCollators as pallet_authorship::EventHandler<_, _>>::note_author(
						alice.clone(),
					);
				}
				// The emulator does not run `on_poll`.
				<ValidatorCollators as frame_support::traits::Hooks<_>>::on_poll(
					pallet_validator_collators::ReportPeriod::<runtime::Runtime>::get(),
					&mut frame_support::weights::WeightMeter::new(),
				);
				assert_expected_events!(
					CollatingChain,
					vec![
						RuntimeEvent::ValidatorCollators(
							pallet_validator_collators::Event::BlocksReported { reported: 1, .. }
						) => {},
					]
				);
			});
		};

		// AND Asset Hub holds no collation points of era 1, and the chain stores a set with Alice
		AssetHubPolkadot::execute_with(|| {
			pallet_staking_async::ErasRewardPointsByService::<AssetHubRuntime>::remove(
				1,
				ServiceKind::Collation,
			)
		});
		CollatingChain::execute_with(|| {
			assert_ok!(ValidatorCollators::set_validators(
				pallet_xcm::Origin::Xcm(runtime::xcm_config::AssetHubLocation::get()).into(),
				1,
				[alice.clone()]
					.into_iter()
					.collect::<std::collections::BTreeSet<_>>()
					.try_into()
					.unwrap(),
			));
		});

		// WHEN the chain reports Alice's blocks with the default points per service
		report_three_blocks_of_alice();
		// THEN Asset Hub executes the report and credits nothing
		AssetHubPolkadot::execute_with(|| {
			type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
			assert_expected_events!(
				AssetHubPolkadot,
				vec![
					RuntimeEvent::StakingPointsConverter(
						pallet_staking_async_points_converter::Event::BlocksIgnored { source, blocks: 3 }
					) => { source: *source == para_id, },
				]
			);
		});
		assert_eq!(collation_points_on_asset_hub(), vec![]);

		// WHEN governance sets 5 points per block of the chain and the chain reports again
		AssetHubPolkadot::execute_with(|| {
			assert_ok!(pallet_staking_async_points_converter::Pallet::<AssetHubRuntime>::set_points_per_service(
				<AssetHubPolkadot as Chain>::RuntimeOrigin::root(),
				para_id,
				ServiceKind::Collation,
				5,
			));
		});
		report_three_blocks_of_alice();
		// THEN Asset Hub credits Alice's blocks as collation points
		assert_eq!(collation_points_on_asset_hub(), vec![(alice.clone(), 15)]);
	});
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
fn reports_of_each_collating_chain_decode_on_asset_hub() {
	let blocks = full_report();
	for_each_collating_chain!(|CollatingChain, runtime| {
		// GIVEN the largest call the chain sends to report blocks
		let encoded = runtime::ReportBlocksToAssetHub::encoded_call(&blocks);
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
	});
}

#[test]
fn validator_set_of_asset_hub_decodes_on_each_collating_chain() {
	// GIVEN the largest validator set Asset Hub sends
	let validators: Vec<AccountId> = full_report().into_iter().map(|(who, _)| who).collect();
	let encoded = ValidatorSetToSystemChains::encoded_call(7, &validators);
	let mut sorted = validators.clone();
	sorted.sort();

	for_each_collating_chain!(|CollatingChain, runtime| {
		// WHEN the chain decodes it
		// THEN it is `set_validators` of `pallet-validator-collators` with the same set
		match <runtime::RuntimeCall as codec::Decode>::decode(&mut &encoded[..]) {
			Ok(runtime::RuntimeCall::ValidatorCollators(
				pallet_validator_collators::Call::set_validators { era: 7, validators: decoded },
			)) => assert_eq!(decoded.into_iter().collect::<Vec<_>>(), sorted),
			other => panic!("the set decodes as {other:?}"),
		}
	});
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
fn asset_hub_staking_admin_sets_the_cap_on_each_collating_chain() {
	use asset_hub_polkadot_runtime::governance::pallet_custom_origins::Origin;
	for_each_collating_chain!(|CollatingChain, runtime| {
		type RuntimeEvent = <CollatingChain as Chain>::RuntimeEvent;
		let set_cap = |max| {
			runtime::RuntimeCall::ValidatorCollators(
				pallet_validator_collators::Call::set_max_collators { max },
			)
			.encode()
		};
		let destination = AssetHubPolkadot::sibling_location_of(CollatingChain::para_id());
		// GIVEN the default cap
		CollatingChain::execute_with(|| {
			assert_eq!(MaxCollators::<runtime::Runtime>::get(), CollatorDraw::None)
		});

		// WHEN Asset Hub's GeneralAdmin sends a cap
		send_transact_from_asset_hub(
			Origin::GeneralAdmin.into(),
			destination.clone(),
			set_cap(CollatorDraw::UpTo(1)),
		);
		// THEN the chain executes the message, `Transact` fails without failing it, and the cap
		// stays
		CollatingChain::execute_with(|| {
			assert_expected_events!(
				CollatingChain,
				vec![
					RuntimeEvent::MessageQueue(
						pallet_message_queue::Event::Processed { success: true, .. }
					) => {},
				]
			);
			assert_eq!(MaxCollators::<runtime::Runtime>::get(), CollatorDraw::None);
		});

		// WHEN Asset Hub's StakingAdmin, the track of the enabling referenda, sends a cap
		send_transact_from_asset_hub(
			Origin::StakingAdmin.into(),
			destination,
			set_cap(CollatorDraw::UpTo(3)),
		);
		// THEN the chain stores it
		CollatingChain::execute_with(|| {
			assert_expected_events!(
				CollatingChain,
				vec![
					RuntimeEvent::ValidatorCollators(
						pallet_validator_collators::Event::MaxCollatorsSet { max: CollatorDraw::UpTo(3) }
					) => {},
				]
			);
			assert_eq!(MaxCollators::<runtime::Runtime>::get(), CollatorDraw::UpTo(3));
		});
	});
}
