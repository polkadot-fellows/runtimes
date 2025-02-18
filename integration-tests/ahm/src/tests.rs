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

use asset_hub_polkadot_runtime::Runtime as AssetHub;
use cumulus_primitives_core::{AggregateMessageOrigin, Junction, Location, ParaId};
use frame_support::traits::*;
use pallet_rc_migrator::{types::PalletMigrationChecks, MigrationStage, RcMigrationStage};
use polkadot_runtime::Runtime as Polkadot;
use polkadot_runtime_common::paras_registrar;
use sp_runtime::AccountId32;
use std::str::FromStr;
use xcm_emulator::ConvertLocation;

use super::mock::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn account_migration_works() {
	let Some((mut rc, mut ah)) = load_externalities().await else { return };
	let para_id = ParaId::from(1000);

	// Simulate relay blocks and grab the DMP messages
	let (dmp_messages, pre_check_payload) = rc.execute_with(|| {
		let mut dmps = Vec::new();

		if let Ok(stage) = std::env::var("START_STAGE") {
			let stage = MigrationStage::from_str(&stage).expect("Invalid start stage");
			RcMigrationStage::<Polkadot>::put(stage);
		}

		let pre_check_payload =
			pallet_rc_migrator::preimage::PreimageChunkMigrator::<Polkadot>::pre_check();

		// Loop until no more DMPs are added and we had at least 1
		loop {
			next_block_rc();

			let new_dmps =
				runtime_parachains::dmp::DownwardMessageQueues::<Polkadot>::take(para_id);
			dmps.extend(new_dmps);

			if RcMigrationStage::<Polkadot>::get() ==
				pallet_rc_migrator::MigrationStage::MigrationDone
			{
				log::info!("Migration done");
				break (dmps, pre_check_payload);
			}
		}
	});
	rc.commit_all().unwrap();
	// TODO: for some reason this prints some small value (2947), but logs on XCM send and receive
	// show more iteration.
	log::info!("Num of RC->AH DMP messages: {}", dmp_messages.len());

	// Inject the DMP messages into the Asset Hub
	ah.execute_with(|| {
		pallet_ah_migrator::preimage::PreimageMigrationCheck::<AssetHub>::pre_check();
		let mut fp =
			asset_hub_polkadot_runtime::MessageQueue::footprint(AggregateMessageOrigin::Parent);
		enqueue_dmp(dmp_messages);

		// Loop until no more DMPs are queued
		loop {
			let new_fp =
				asset_hub_polkadot_runtime::MessageQueue::footprint(AggregateMessageOrigin::Parent);
			if fp == new_fp {
				log::info!("AH DMP messages left: {}", fp.storage.count);
				break;
			}
			fp = new_fp;

			log::debug!("AH DMP messages left: {}", fp.storage.count);
			next_block_ah();

			if RcMigrationStage::<Polkadot>::get() ==
				pallet_rc_migrator::MigrationStage::PreimageMigrationDone
			{
				pallet_rc_migrator::preimage::PreimageChunkMigrator::<Polkadot>::post_check(
					pre_check_payload.clone(),
				);
			}
		}

		pallet_ah_migrator::preimage::PreimageMigrationCheck::<AssetHub>::post_check(());
		// NOTE that the DMP queue is probably not empty because the snapshot that we use contains
		// some overweight ones.
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

		let translated = pallet_rc_migrator::accounts::AccountsMigrator::<Polkadot>::try_translate_rc_sovereign_to_ah(rc_acc).unwrap().unwrap();
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

	let mut rc_accounts = Vec::new();
	let mut ah_accounts = Vec::new();

	let paras = rc.execute_with(|| {
		let paras = paras_registrar::Paras::<Polkadot>::iter_keys().collect::<Vec<_>>();

		for para_id in paras.clone() {
			let rc_acc = xcm_builder::ChildParachainConvertsVia::<ParaId, AccountId32>::convert_location(&Location::new(0, Junction::Parachain(para_id.into()))).unwrap();
			rc_accounts.push(rc_acc.clone());

			let ah_acc = pallet_rc_migrator::accounts::AccountsMigrator::<Polkadot>::try_translate_rc_sovereign_to_ah(rc_acc).unwrap().unwrap();
			ah_accounts.push(ah_acc);
		}

		paras
	});

	let mut csv: String = "para,rc,ah,exists_on_ah\n".into();

	// Sanity check that they all exist. Note that they dont *have to*, but all do.
	println!("Translating {} RC accounts to AH", rc_accounts.len());
	ah.execute_with(|| {
		for (i, (rc_acc, ah_acc)) in rc_accounts.iter().zip(ah_accounts.iter()).enumerate() {
			println!("[{}] {} -> {}", paras[i], rc_acc, ah_acc);

			csv.push_str(&format!("{},{},{}\n", paras[i], rc_acc, ah_acc));
		}
	});

	//std::fs::write("/tmp/sovereign_account_translation.csv", csv).unwrap();
}
