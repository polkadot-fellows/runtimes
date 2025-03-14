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

//! Rust integration for the Asset Hub Migration.
//!
//! This test calls `on_initialize` on the RC and on AH alternately and forwards DMP messages.
//!
//! Create snapshots in the root dir:
//!
//! ```
//! try-runtime create-snapshot --uri wss://sys.ibp.network:443/statemint ah-polkadot.snap
//! try-runtime create-snapshot --uri wss://try-runtime.polkadot.io:443 polkadot.snap
//! ```
//!
//! Run with:
//!
//! ```
//! SNAP_RC="../../polkadot.snap" SNAP_AH="../../ah-polkadot.snap" RUST_LOG="info" ct polkadot-integration-tests-ahm -r on_initialize_works -- --nocapture
//! ```

use super::mock::*;
use asset_hub_polkadot_runtime::Runtime as AssetHub;
use cumulus_pallet_parachain_system::PendingUpwardMessages;
use cumulus_primitives_core::{BlockT, Junction, Location, ParaId};
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_ah_migrator::types::AhMigrationCheck;
use pallet_rc_migrator::{
	types::RcMigrationCheck, MigrationStage as RcMigrationStage,
	RcMigrationStage as RcMigrationStageStorage,
};
use polkadot_runtime::{Block as PolkadotBlock, Runtime as Polkadot};
use polkadot_runtime_common::{paras_registrar, slots as pallet_slots};
use remote_externalities::RemoteExternalities;
use runtime_parachains::dmp::DownwardMessageQueues;
use sp_runtime::AccountId32;
use std::{collections::BTreeMap, str::FromStr};
use xcm_emulator::ConvertLocation;

type RcChecks = (
	pallet_rc_migrator::accounts::AccountsMigrator<Polkadot>,
	pallet_rc_migrator::preimage::PreimageChunkMigrator<Polkadot>,
	pallet_rc_migrator::indices::IndicesMigrator<Polkadot>,
	// other pallets go here
);

type AhChecks = (
	pallet_rc_migrator::accounts::AccountsMigrator<AssetHub>,
	pallet_rc_migrator::preimage::PreimageChunkMigrator<AssetHub>,
	pallet_rc_migrator::indices::IndicesMigrator<AssetHub>,
	// other pallets go here
);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pallet_migration_works() {
	let Some((mut rc, mut ah)) = load_externalities().await else { return };

	// Set the initial migration stage from env var if set.
	set_initial_migration_stage(&mut rc);

	// Pre-checks on the Relay
	let rc_pre = run_check(|| RcChecks::pre_check(), &mut rc);

	// Pre-checks on the Asset Hub
	let ah_pre = run_check(|| AhChecks::pre_check(rc_pre.clone().unwrap()), &mut ah);

	// Migrate the Relay Chain
	let dmp_messages = rc_migrate(&mut rc);

	// Post-checks on the Relay
	run_check(|| RcChecks::post_check(rc_pre.clone().unwrap()), &mut rc);

	// Migrate the Asset Hub
	ah_migrate(&mut ah, dmp_messages);

	// Post-checks on the Asset Hub
	run_check(|| AhChecks::post_check(rc_pre.unwrap(), ah_pre.unwrap()), &mut ah);
}

fn run_check<R, B: BlockT>(f: impl FnOnce() -> R, ext: &mut RemoteExternalities<B>) -> Option<R> {
	if std::env::var("START_STAGE").is_err() {
		Some(ext.execute_with(|| f()))
	} else {
		None
	}
}

#[tokio::test]
async fn num_leases_to_ending_block_works_simple() {
	let mut rc = remote_ext_test_setup::<PolkadotBlock>("SNAP_RC").await.unwrap();
	let f = |now: BlockNumberFor<Polkadot>, num_leases: u32| {
		frame_system::Pallet::<Polkadot>::set_block_number(now);
		pallet_rc_migrator::crowdloan::num_leases_to_ending_block::<Polkadot>(num_leases)
	};

	rc.execute_with(|| {
		let p = <Polkadot as pallet_slots::Config>::LeasePeriod::get();
		let o = <Polkadot as pallet_slots::Config>::LeaseOffset::get();

		// Sanity check:
		assert!(f(1000, 0).is_err());
		assert!(f(1000, 10).is_err());
		// Overflow check:
		assert!(f(o, u32::MAX).is_err());

		// In period 0:
		assert_eq!(f(o, 0), Ok(o));
		assert_eq!(f(o, 1), Ok(o + p));
		assert_eq!(f(o, 2), Ok(o + 2 * p));

		// In period 1:
		assert_eq!(f(o + p, 0), Ok(o + p));
		assert_eq!(f(o + p, 1), Ok(o + 2 * p));
		assert_eq!(f(o + p, 2), Ok(o + 3 * p));

		// In period 19 with 5 remaining:
		assert_eq!(f(o + 19 * p, 1), Ok(o + 20 * p));
		assert_eq!(f(o + 19 * p, 5), Ok(o + 24 * p));
	});
}

#[test]
fn sovereign_account_translation() {
	let good_cases = [
		(
			// para 2094 account https://polkadot.subscan.io/account/13YMK2dzLWfnGZXSLuAxgZbBiNMHLfnPZ8itzwXryJ9FcWsE
			"13YMK2dzLWfnGZXSLuAxgZbBiNMHLfnPZ8itzwXryJ9FcWsE",
			// on ah (different account id) https://assethub-polkadot.subscan.io/account/13cKp88oRErgQAFatu83oCvzxr2b45qVcnNLFu4Mr2ApU6ZC
			"13cKp88oRErgQAFatu83oCvzxr2b45qVcnNLFu4Mr2ApU6ZC",
		),
		(
			"13YMK2dsXbyC866w2tFM4vH52nRs3uTwac32jh1FNXZBXv18",
			"13cKp88gcLA6Fgq5atCSBZctHG7AmKX3eFgTzeXkFFakPWuo",
		),
	];

	for (rc_acc, ah_acc) in good_cases {
		let rc_acc = AccountId32::from_str(rc_acc).unwrap();
		let ah_acc = AccountId32::from_str(ah_acc).unwrap();

		let (translated, _para_id) = pallet_rc_migrator::accounts::AccountsMigrator::<Polkadot>::try_translate_rc_sovereign_to_ah(rc_acc).unwrap().unwrap();
		assert_eq!(translated, ah_acc);
	}

	let bad_cases = [
		"13yJaZUmhMDG91AftfdNeJm6hMVSL9Jq2gqiyFdhiJgXf6AY", // wrong prefix
		"13ddruDZgGbfVmbobzfNLV4momSgjkFnMXkfogizb4uEbHtQ", // "
		"13cF4T4kfi8VYw2nTZfkYkn9BjGpmRDsivYxFqGYUWkU8L2d", // "
		"13cKp88gcLA6Fgq5atCSBZctHG7AmKX3eFgTzeXkFFakPo6e", // last byte not 0
		"13cF4T4kfiJ39NqGh4DAZSMo6NuWT1fYfZzCo9f5HH8dUFBJ", // 7 byte not zero
		"13cKp88gcLA6Fgq5atCSBZctHGenFzUo3qmmReNVKzpnGvFg", // some center byte not zero
	];

	for rc_acc in bad_cases {
		let rc_acc = AccountId32::from_str(rc_acc).unwrap();

		let translated = pallet_rc_migrator::accounts::AccountsMigrator::<Polkadot>::try_translate_rc_sovereign_to_ah(rc_acc).unwrap();
		assert!(translated.is_none());
	}
}

/// For human consumption.
#[tokio::test]
async fn print_sovereign_account_translation() {
	let (mut rc, mut ah) = load_externalities().await.unwrap();

	let mut rc_to_ah = BTreeMap::new();

	rc.execute_with(|| {
		for para_id in paras_registrar::Paras::<Polkadot>::iter_keys().collect::<Vec<_>>() {
			let rc_acc = xcm_builder::ChildParachainConvertsVia::<ParaId, AccountId32>::convert_location(&Location::new(0, Junction::Parachain(para_id.into()))).unwrap();

			let (ah_acc, para_id) = pallet_rc_migrator::accounts::AccountsMigrator::<Polkadot>::try_translate_rc_sovereign_to_ah(rc_acc.clone()).unwrap().unwrap();
			rc_to_ah.insert(rc_acc, (ah_acc, para_id));
		}

		for account in frame_system::Account::<Polkadot>::iter_keys() {
			let translated = pallet_rc_migrator::accounts::AccountsMigrator::<Polkadot>::try_translate_rc_sovereign_to_ah(account.clone()).unwrap();

			if let Some((ah_acc, para_id)) = translated {
				if !rc_to_ah.contains_key(&account) {
					println!("Account belongs to an unregistered para {}: {}", para_id, account);
					rc_to_ah.insert(account, (ah_acc, para_id));
				}
			}
		}
	});

	let mut csv: String = "para,rc,ah\n".into();

	// Sanity check that they all exist. Note that they dont *have to*, but all do.
	println!("Translating {} RC accounts to AH", rc_to_ah.len());
	ah.execute_with(|| {
		for (rc_acc, (ah_acc, para_id)) in rc_to_ah.iter() {
			println!("[{}] {} -> {}", para_id, rc_acc, ah_acc);

			csv.push_str(&format!("{},{},{}\n", para_id, rc_acc, ah_acc));
		}
	});

	//std::fs::write("../../pallets/rc-migrator/src/sovereign_account_translation.csv",
	// csv).unwrap();
}

#[tokio::test]
async fn print_accounts_statistics() {
	use frame_system::Account as SystemAccount;

	let mut rc = remote_ext_test_setup::<PolkadotBlock>("SNAP_RC").await.unwrap();

	let mut total_counts = std::collections::HashMap::new();

	rc.execute_with(|| {
		for (who, account_info) in SystemAccount::<Polkadot>::iter() {
			total_counts.entry("total_count").and_modify(|count| *count += 1).or_insert(1);

			let freezes_count = pallet_balances::Freezes::<Polkadot>::get(&who).len();
			let lock_count = pallet_balances::Locks::<Polkadot>::get(&who).len();
			let holds_sum = pallet_balances::Holds::<Polkadot>::get(&who)
				.iter()
				.map(|h| h.amount)
				.sum::<u128>();
			let unnamed_reserve = account_info.data.reserved.saturating_sub(holds_sum);

			if freezes_count == 0 && lock_count == 0 && holds_sum == 0 && unnamed_reserve == 0 {
				total_counts
					.entry("total_liquid_count")
					.and_modify(|count| *count += 1)
					.or_insert(1);
			}
		}
	});

	/*
	RC Polkadot snapshot from 2025-01-24:
		total_count ~ 1_434_995
		total_liquid_count ~ 1_373_890
	 */
	println!("Total counts: {:?}", total_counts);
}

#[tokio::test(flavor = "current_thread")]
async fn migration_works() {
	let Some((mut rc, mut ah)) = load_externalities().await else { return };

	// Set the initial migration stage from env var if set.
	set_initial_migration_stage(&mut rc);

	// Pre-checks on the Relay
	let rc_pre = run_check(|| RcChecks::pre_check(), &mut rc);

	// Pre-checks on the Asset Hub
	let ah_pre = run_check(|| AhChecks::pre_check(rc_pre.clone().unwrap()), &mut ah);

	let mut rc_block_count = 0;
	// finish the loop when the migration is done.
	while rc.execute_with(|| RcMigrationStageStorage::<Polkadot>::get())
		!= RcMigrationStage::MigrationDone
	{
		// execute next RC block.
		let dmp_messages = rc.execute_with(|| {
			next_block_rc();

			DownwardMessageQueues::<Polkadot>::take(AH_PARA_ID)
		});
		rc.commit_all().unwrap();

		// enqueue DMP messages from RC to AH.
		ah.execute_with(|| {
			// TODO: bound the `dmp_messages` total size
			enqueue_dmp(dmp_messages);
		});
		ah.commit_all().unwrap();

		// execute next AH block on every second RC block.
		if rc_block_count % 2 == 0 {
			let ump_messages = ah.execute_with(|| {
				next_block_ah();

				PendingUpwardMessages::<AssetHub>::take()
			});
			ah.commit_all().unwrap();

			// enqueue UMP messages from AH to RC.
			rc.execute_with(|| {
				// TODO: bound the `ump_messages` total size
				enqueue_ump(ump_messages);
			});
			rc.commit_all().unwrap();
		}

		rc_block_count += 1;
	}

	// Post-checks on the Relay
	run_check(|| RcChecks::post_check(rc_pre.clone().unwrap()), &mut rc);

	// Post-checks on the Asset Hub
	run_check(|| AhChecks::post_check(rc_pre.unwrap(), ah_pre.unwrap()), &mut ah);

	println!("Migration done in {} RC blocks", rc_block_count);
}
