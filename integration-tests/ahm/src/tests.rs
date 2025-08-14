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
//! SNAP_RC="../../polkadot.snap" SNAP_AH="../../ah-polkadot.snap" RUST_LOG="info" ct polkadot-integration-tests-ahm -r pallet_migration_works -- --nocapture
//! add `--features try-runtime` if you want to run the `try-runtime` tests for all pallets too.
//! ```
//!
//! To run the pre+post migration checks against a set of snapshots from pre/post migration (created
//! in `ahm-drynrun's CI`):
//!
//! ```
//! SNAP_RC_PRE="rc-pre.snap" \
//! SNAP_AH_PRE="ah-pre.snap" \
//! SNAP_RC_POST="rc-post.snap" \
//! SNAP_AH_POST="ah-post.snap" \
//! cargo test \
//! 	-p polkadot-integration-tests-ahm \
//! 	--features try-runtime \
//! 	--features paseo \
//! 	--release \
//! 	-- post_migration_checks_only --nocapture
//! ```

use crate::porting_prelude::*;

#[cfg(not(feature = "paseo"))]
use super::proxy::ProxyWhaleWatching;
use super::{
	accounts_translation_works::AccountTranslationWorks,
	balances_test::BalancesCrossChecker,
	checks::{PalletsTryStateCheck, SanityChecks},
	mock::*,
	multisig_still_work::MultisigStillWork,
	multisig_test::MultisigsAccountIdStaysTheSame,
	proxy::ProxyBasicWorks,
};
use asset_hub_polkadot_runtime::Runtime as AssetHub;
use cumulus_pallet_parachain_system::PendingUpwardMessages;
use cumulus_primitives_core::{InboundDownwardMessage, Junction, Location, ParaId};
use frame_support::traits::{
	fungible::Inspect, schedule::DispatchTime, Currency, ExistenceRequirement, OnFinalize,
	OnInitialize, ReservableCurrency,
};
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_ah_migrator::{
	sovereign_account_translation::{DERIVED_TRANSLATIONS, SOV_TRANSLATIONS},
	types::AhMigrationCheck,
	AhMigrationStage as AhMigrationStageStorage, MigrationEndBlock as AhMigrationEndBlock,
	MigrationStage as AhMigrationStage, MigrationStartBlock as AhMigrationStartBlock,
};
use pallet_rc_migrator::{
	child_bounties::ChildBountiesMigratedCorrectly, staking::StakingMigratedCorrectly,
	types::RcMigrationCheck, MigrationEndBlock as RcMigrationEndBlock,
	MigrationStage as RcMigrationStage, MigrationStartBlock as RcMigrationStartBlock,
	RcMigrationStage as RcMigrationStageStorage,
};
use polkadot_primitives::UpwardMessage;
use polkadot_runtime::{RcMigrator, Runtime as Polkadot};
use polkadot_runtime_common::slots as pallet_slots;
use runtime_parachains::dmp::DownwardMessageQueues;
use sp_core::{crypto::Ss58Codec, ByteArray};
use sp_io::TestExternalities;
use sp_runtime::{traits::Dispatchable, AccountId32, BuildStorage, DispatchError, TokenError};
use std::{
	collections::{BTreeMap, BTreeSet, VecDeque},
	str::FromStr,
};
use xcm::latest::*;
use xcm_emulator::{assert_ok, ConvertLocation, WeightMeter};

type RcChecks = (
	SanityChecks,
	pallet_rc_migrator::accounts::tests::AccountsMigrationChecker<Polkadot>,
	pallet_rc_migrator::preimage::PreimageChunkMigrator<Polkadot>,
	pallet_rc_migrator::preimage::PreimageRequestStatusMigrator<Polkadot>,
	pallet_rc_migrator::preimage::PreimageLegacyRequestStatusMigrator<Polkadot>,
	pallet_rc_migrator::indices::IndicesMigrator<Polkadot>,
	pallet_rc_migrator::vesting::VestingMigrator<Polkadot>,
	pallet_rc_migrator::proxy::ProxyProxiesMigrator<Polkadot>,
	pallet_rc_migrator::staking::bags_list::BagsListMigrator<Polkadot>,
	pallet_rc_migrator::conviction_voting::ConvictionVotingMigrator<Polkadot>,
	pallet_rc_migrator::asset_rate::AssetRateMigrator<Polkadot>,
	pallet_rc_migrator::scheduler::SchedulerMigrator<Polkadot>,
	pallet_rc_migrator::staking::nom_pools::NomPoolsMigrator<Polkadot>,
	pallet_rc_migrator::staking::delegated_staking::DelegatedStakingMigrator<Polkadot>,
	pallet_rc_migrator::referenda::ReferendaMigrator<Polkadot>,
	BalancesCrossChecker,
	RcRuntimeSpecificChecks,
	// other checks go here (if available on Polkadot, Kusama and Westend)
	ProxyBasicWorks,
	MultisigStillWork,
	AccountTranslationWorks,
	PalletsTryStateCheck,
);

// Checks that are specific to Polkadot, and not available on other chains (like Paseo)
#[cfg(not(feature = "paseo"))]
pub type RcRuntimeSpecificChecks = (
	MultisigsAccountIdStaysTheSame,
	pallet_rc_migrator::multisig::MultisigMigrationChecker<Polkadot>,
	pallet_rc_migrator::bounties::BountiesMigrator<Polkadot>,
	pallet_rc_migrator::treasury::TreasuryMigrator<Polkadot>,
	pallet_rc_migrator::claims::ClaimsMigrator<Polkadot>,
	pallet_rc_migrator::crowdloan::CrowdloanMigrator<Polkadot>,
	ProxyWhaleWatching,
	StakingMigratedCorrectly<Polkadot>,
	ChildBountiesMigratedCorrectly<Polkadot>,
);

// Checks that are specific to Paseo.
#[cfg(feature = "paseo")]
pub type RcRuntimeSpecificChecks = (
	MultisigsAccountIdStaysTheSame,
	pallet_rc_migrator::multisig::MultisigMigrationChecker<Polkadot>,
	pallet_rc_migrator::bounties::BountiesMigrator<Polkadot>,
	pallet_rc_migrator::treasury::TreasuryMigrator<Polkadot>,
	pallet_rc_migrator::claims::ClaimsMigrator<Polkadot>,
	pallet_rc_migrator::crowdloan::CrowdloanMigrator<Polkadot>,
);

type AhChecks = (
	SanityChecks,
	pallet_rc_migrator::accounts::tests::AccountsMigrationChecker<AssetHub>,
	pallet_rc_migrator::preimage::PreimageChunkMigrator<AssetHub>,
	pallet_rc_migrator::preimage::PreimageRequestStatusMigrator<AssetHub>,
	pallet_rc_migrator::preimage::PreimageLegacyRequestStatusMigrator<AssetHub>,
	pallet_rc_migrator::indices::IndicesMigrator<AssetHub>,
	pallet_rc_migrator::vesting::VestingMigrator<AssetHub>,
	pallet_ah_migrator::proxy::ProxyBasicChecks<
		AssetHub,
		<Polkadot as pallet_proxy::Config>::ProxyType,
	>,
	pallet_rc_migrator::staking::bags_list::BagsListMigrator<AssetHub>,
	pallet_rc_migrator::conviction_voting::ConvictionVotingMigrator<AssetHub>,
	pallet_rc_migrator::asset_rate::AssetRateMigrator<AssetHub>,
	pallet_rc_migrator::scheduler::SchedulerMigrator<AssetHub>,
	pallet_rc_migrator::staking::nom_pools::NomPoolsMigrator<AssetHub>,
	pallet_rc_migrator::staking::delegated_staking::DelegatedStakingMigrator<AssetHub>,
	pallet_rc_migrator::referenda::ReferendaMigrator<AssetHub>,
	BalancesCrossChecker,
	AhRuntimeSpecificChecks,
	// other checks go here (if available on Polkadot, Kusama and Westend)
	ProxyBasicWorks,
	MultisigStillWork,
	AccountTranslationWorks,
	PalletsTryStateCheck,
);

#[cfg(not(feature = "paseo"))]
pub type AhRuntimeSpecificChecks = (
	MultisigsAccountIdStaysTheSame,
	pallet_rc_migrator::multisig::MultisigMigrationChecker<AssetHub>,
	pallet_rc_migrator::bounties::BountiesMigrator<AssetHub>,
	pallet_rc_migrator::treasury::TreasuryMigrator<AssetHub>,
	pallet_rc_migrator::claims::ClaimsMigrator<AssetHub>,
	pallet_rc_migrator::crowdloan::CrowdloanMigrator<AssetHub>,
	ProxyWhaleWatching,
	StakingMigratedCorrectly<AssetHub>,
	ChildBountiesMigratedCorrectly<AssetHub>,
);

// TODO: staking missing for Paseo

#[cfg(feature = "paseo")]
pub type AhRuntimeSpecificChecks = (
	MultisigsAccountIdStaysTheSame,
	pallet_rc_migrator::multisig::MultisigMigrationChecker<AssetHub>,
	pallet_rc_migrator::bounties::BountiesMigrator<AssetHub>,
	pallet_rc_migrator::treasury::TreasuryMigrator<AssetHub>,
	pallet_rc_migrator::claims::ClaimsMigrator<AssetHub>,
	pallet_rc_migrator::crowdloan::CrowdloanMigrator<AssetHub>,
);

#[ignore] // we use the equivalent [migration_works_time] test instead
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pallet_migration_works() {
	let (mut rc, mut ah) = load_externalities().await.unwrap();

	// Set the initial migration stage from env var if set.
	set_initial_migration_stage(&mut rc, None);

	// Pre-checks on the Relay
	let rc_pre = run_check(RcChecks::pre_check, &mut rc);

	// Pre-checks on the Asset Hub
	let ah_pre = run_check(|| AhChecks::pre_check(rc_pre.clone().unwrap()), &mut ah);

	// Run relay chain, sends start signal to AH
	let dmp_messages = rc_migrate(&mut rc);
	// AH process start signal, send back ack
	ah_migrate(&mut ah, dmp_messages);
	// no upward messaging support in this test yet, just manually advance the stage
	rc.execute_with(|| {
		RcMigrationStageStorage::<Polkadot>::put(RcMigrationStage::Starting);
	});
	rc.commit_all().unwrap();

	// Migrate the Relay Chain
	let dmp_messages = rc_migrate(&mut rc);

	// Post-checks on the Relay
	run_check(|| RcChecks::post_check(rc_pre.clone().unwrap()), &mut rc);

	// Migrate the Asset Hub
	ah_migrate(&mut ah, dmp_messages);

	ah.execute_with(|| {
		assert_eq!(
			pallet_ah_migrator::AhMigrationStage::<AssetHub>::get(),
			pallet_ah_migrator::MigrationStage::MigrationDone
		);
	});

	// Post-checks on the Asset Hub
	run_check(|| AhChecks::post_check(rc_pre.unwrap(), ah_pre.unwrap()), &mut ah);
}

fn run_check<R>(f: impl FnOnce() -> R, ext: &mut TestExternalities) -> Option<R> {
	if std::env::var("START_STAGE").is_err() {
		Some(ext.execute_with(f))
	} else {
		None
	}
}

#[tokio::test]
async fn num_leases_to_ending_block_works_simple() {
	let mut rc = remote_ext_test_setup(Chain::Relay).await.unwrap();
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

		let (translated, _para_id) =
			pallet_ah_ops::Pallet::<AssetHub>::try_translate_rc_sovereign_to_ah(&rc_acc).unwrap();
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

		let translated =
			pallet_ah_ops::Pallet::<AssetHub>::try_translate_rc_sovereign_to_ah(&rc_acc);
		assert!(translated.is_err());
	}
}

/// This test updates the `pallets/ah-migrator/src/sovereign_account_translation.rs` file.
///
/// It iterates through all possible Para-IDs (alive or dead) and their sovereign accounts and their
/// first 100 derived accounts. If an account is found, it is added to the translation map. The
/// value of 100 is arbitrary but nobody seems to use more than 10 in practice. The theoretical
/// limit is 2^16, but that would make the test ~655 times slower.
#[ignore]
#[tokio::test]
async fn find_translatable_accounts() {
	let mut rc = remote_ext_test_setup(Chain::Relay).await.unwrap();

	// Extract all accounts from the RC
	let rc_accounts =
		rc.execute_with(|| frame_system::Account::<Polkadot>::iter_keys().collect::<BTreeSet<_>>());
	println!("Found {} RC accounts", rc_accounts.len());

	// Para ID -> (RC sovereign, AH sovereign)
	let mut sov_translations = BTreeMap::<u32, (AccountId32, AccountId32)>::new();
	// Para ID -> (RC derived, index, AH derived)
	let mut derived_translations = BTreeMap::<u32, (AccountId32, u16, AccountId32)>::new();

	// Try to find Para sovereign and derived accounts.
	for para_id in 0..(u16::MAX as u32) {
		// The Parachain sovereign account ID on the relay chain
		let rc_para_sov =
			xcm_builder::ChildParachainConvertsVia::<ParaId, AccountId32>::convert_location(
				&Location::new(0, Junction::Parachain(para_id.into())),
			)
			.unwrap();

		let (ah_para_sibl, found_para_id) =
			pallet_ah_ops::Pallet::<AssetHub>::try_translate_rc_sovereign_to_ah(&rc_para_sov)
				.unwrap();

		// Check if we need to translate this to the sovereign sibl account
		if rc_accounts.contains(&rc_para_sov) {
			assert_eq!(found_para_id, para_id.into()); // sanity check
			println!(
				"Found RC sovereign for para {}: {} -> {}",
				&para_id, &rc_para_sov, &ah_para_sibl
			);
			sov_translations.insert(para_id, (rc_para_sov.clone(), ah_para_sibl.clone()));
		} else {
			// NOTE we do not have a `continue` here, meaning that we also check derived accounts
			// of non-existent para sovs, just in case they get revived in the future.
		}

		// Now we check the first 100 derived accounts
		for derivation_index in 0..100 {
			let rc_para_derived =
				pallet_ah_ops::derivative_account_id(rc_para_sov.clone(), derivation_index);
			let expected_ah_para_derived =
				pallet_ah_ops::derivative_account_id(ah_para_sibl.clone(), derivation_index);

			if rc_accounts.contains(&rc_para_derived) {
				let (ah_para_derived, found_para_id) =
					pallet_ah_ops::Pallet::<AssetHub>::try_rc_sovereign_derived_to_ah(
						&rc_para_derived,
						&rc_para_sov,
						derivation_index,
					)
					.unwrap();

				assert_eq!(ah_para_derived, expected_ah_para_derived); // sanity check
				assert_eq!(found_para_id, para_id.into()); // sanity check

				println!(
					"Found RC derived   for para {}: {} -> {} (index {})",
					&para_id, &rc_para_derived, &ah_para_derived, &derivation_index
				);
				derived_translations
					.insert(para_id, (rc_para_derived, derivation_index, ah_para_derived));
			}
		}
	}

	println!("Found {} RC sovereign account translations", sov_translations.len());
	println!("Found {} RC derived   account translations", derived_translations.len());

	// Rust code with the translation maps
	let mut rust: String = format!("// RC snap path: {}\n", std::env::var("SNAP_RC").unwrap());

	rust.push_str(
		"/// List of RC para to AH sibl sovereign account translation sorted by RC account.
pub const SOV_TRANSLATIONS: &[((AccountId32, &'static str), (AccountId32, &'static str))] = &[\n",
	);

	let mut sov_translations = sov_translations.into_iter().collect::<Vec<_>>();
	sov_translations.sort_by(|(_, (rc_acc, _)), (_, (rc_acc2, _))| rc_acc.cmp(rc_acc2));

	for (para_id, (rc_acc, ah_acc)) in sov_translations.iter() {
		rust.push_str(&format!("\t// para {}\n", para_id));
		rust.push_str(&format!(
			"\t(({}, \"{}\"), ({}, \"{}\")),\n",
			format_account_id(rc_acc),
			rc_acc.to_ss58check(),
			format_account_id(ah_acc),
			ah_acc.to_ss58check(),
		));
	}

	rust.push_str("];");

	rust.push_str(
		"\n\n/// List of RC para to AH sibl derived account translation sorted by RC account.
pub const DERIVED_TRANSLATIONS: &[((AccountId32, &'static str), u16, (AccountId32, &'static str))] = &[\n",
	);

	let mut derived_translations = derived_translations.into_iter().collect::<Vec<_>>();
	derived_translations.sort_by(|(_, (rc_acc, _, _)), (_, (rc_acc2, _, _))| rc_acc.cmp(rc_acc2));

	for (para_id, (rc_acc, derivation_index, ah_acc)) in derived_translations.iter() {
		rust.push_str(&format!("\t// para {} (derivation index {})\n", para_id, derivation_index));
		rust.push_str(&format!(
			"\t(({}, \"{}\"), {}, ({}, \"{}\")),\n",
			format_account_id(rc_acc),
			rc_acc.to_ss58check(),
			derivation_index,
			format_account_id(ah_acc),
			ah_acc.to_ss58check(),
		));
	}

	rust.push_str("];");

	// Replace everything after the "AUTOGENERATED BELOW" comment with our Rust string
	let path =
		std::path::Path::new("../../pallets/ah-migrator/src/sovereign_account_translation.rs");
	let mut file = std::fs::File::open(path).unwrap();
	let mut contents = String::new();
	std::io::Read::read_to_string(&mut file, &mut contents).unwrap();

	// Replace everything after the "AUTOGENERATED BELOW" comment with our Rust string
	let pos_auto_gen = contents.find("// AUTOGENERATED BELOW").unwrap() + 23;
	contents.truncate(pos_auto_gen);
	contents.insert_str(pos_auto_gen, &rust);

	// Write the result back to the file
	std::fs::write(path, contents).unwrap();

	println!("Wrote to {}", std::fs::canonicalize(path).unwrap().display());
}

/// Check the SS58 IDs in `pallets/ah-migrator/src/sovereign_account_translation.rs` are correct.
#[test]
fn translation_integrity_check() {
	for ((rc_acc, rc_id), (ah_acc, ah_id)) in SOV_TRANSLATIONS.iter() {
		assert_eq!(&rc_acc.to_ss58check(), rc_id);
		assert_eq!(&ah_acc.to_ss58check(), ah_id);
	}

	for ((rc_acc, rc_id), _, (ah_acc, ah_id)) in DERIVED_TRANSLATIONS.iter() {
		assert_eq!(&rc_acc.to_ss58check(), rc_id);
		assert_eq!(&ah_acc.to_ss58check(), ah_id);
	}
}

fn format_account_id(acc: &AccountId32) -> String {
	format!("AccountId32::new(hex!(\"{}\"))", hex::encode(acc.as_slice()))
}

#[tokio::test]
async fn print_accounts_statistics() {
	use frame_system::Account as SystemAccount;

	let mut rc = remote_ext_test_setup(Chain::Relay).await.unwrap();

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
	println!("Total counts: {total_counts:?}");
}

#[test]
fn ah_account_migration_weight() {
	use frame_support::weights::constants::WEIGHT_REF_TIME_PER_MILLIS;
	use pallet_rc_migrator::weights_ah::WeightInfo;

	let ms_for_accs = |num_accs: u32| {
		let weight =
			pallet_rc_migrator::weights_ah::SubstrateWeight::<AssetHub>::receive_liquid_accounts(
				num_accs,
			);
		weight.ref_time() as f64 / WEIGHT_REF_TIME_PER_MILLIS as f64
	};
	let mb_for_accs = |num_accs: u32| {
		let weight =
			pallet_rc_migrator::weights_ah::SubstrateWeight::<AssetHub>::receive_liquid_accounts(
				num_accs,
			);
		weight.proof_size() as f64 / 1_000_000.0
	};

	// Print for 10, 100 and 1000 accounts in ms
	for i in [10, 100, 486, 1000] {
		let (ms, mb) = (ms_for_accs(i), mb_for_accs(i));
		println!("Weight for {i} accounts: {ms: >4.2} ms, {mb: >4.2} MB");

		assert!(ms < 200.0, "Ref time weight for Accounts migration is insane");
		assert!(mb < 4.0, "Proof size for Accounts migration is insane");
	}
}

#[tokio::test]
async fn migration_works_time() {
	let Some((mut rc, mut ah)) = load_externalities().await else { return };

	let migrate = |ah_block_start: u32, rc: &mut TestExternalities, ah: &mut TestExternalities| {
		// we push first message to be popped for the first RC block and the second one to delay the
		// ump messages from the first AH block, since with async backing and full blocks we
		// generally expect the AH+0 block to be backed at RC+2 block, where RC+0 is its parent RC
		// block. Hence the only RC+2 block will receive and process the messages from the AH+0
		// block.
		let mut ump_messages: VecDeque<(Vec<UpwardMessage>, BlockNumberFor<AssetHub>)> =
			vec![(vec![], ah_block_start - 1), (vec![], ah_block_start)].into();
		// AH generally builds the blocks on every new RC block, therefore every DMP message
		// received and processed immediately without delay.
		let mut dmp_messages: VecDeque<(Vec<InboundDownwardMessage>, BlockNumberFor<Polkadot>)> =
			vec![].into();

		// finish the loop when the migration is done.
		while ah.execute_with(AhMigrationStageStorage::<AssetHub>::get) !=
			AhMigrationStage::MigrationDone
		{
			// with async backing having three unincluded segments, we expect the Asset Hub block
			// to typically be backed not in the immediate next block, but in the block after that.
			// therefore, the queue should always contain at least two messages: one from the most
			// recent Asset Hub block and one from the previous block.
			assert!(
				ump_messages.len() > 1,
				"ump_messages queue should contain at least two messages"
			);

			// enqueue UMP messages from AH to RC.
			rc.execute_with(|| {
				enqueue_ump(
					ump_messages
						.pop_front()
						.expect("should contain at least empty message package"),
				);
			});

			// execute next RC block.
			rc.execute_with(|| {
				next_block_rc();
			});

			// read dmp messages sent to AH.
			dmp_messages.push_back(rc.execute_with(|| {
				(
					DownwardMessageQueues::<Polkadot>::take(AH_PARA_ID),
					frame_system::Pallet::<Polkadot>::block_number(),
				)
			}));

			// end of RC cycle.
			rc.commit_all().unwrap();

			// enqueue DMP messages from RC to AH.
			ah.execute_with(|| {
				enqueue_dmp(
					dmp_messages
						.pop_front()
						.expect("should contain at least empty message package"),
				);
			});

			// execute next AH block.
			ah.execute_with(|| {
				next_block_ah();
			});

			// collect UMP messages from AH generated by the current block execution.
			ump_messages.push_back(ah.execute_with(|| {
				(
					PendingUpwardMessages::<AssetHub>::take(),
					frame_system::Pallet::<AssetHub>::block_number(),
				)
			}));

			// end of AH cycle.
			ah.commit_all().unwrap();
		}
	};

	// Set the initial migration stage from env var if set.
	set_initial_migration_stage(&mut rc, None);

	// Pre-checks on the Relay
	let rc_pre = run_check(RcChecks::pre_check, &mut rc);

	// Pre-checks on the Asset Hub
	let ah_pre = run_check(|| AhChecks::pre_check(rc_pre.clone().unwrap()), &mut ah);

	let rc_block_start = rc.execute_with(frame_system::Pallet::<Polkadot>::block_number);
	let ah_block_start = ah.execute_with(frame_system::Pallet::<AssetHub>::block_number);

	log::info!("Running the migration first time");

	migrate(ah_block_start, &mut rc, &mut ah);

	let rc_block_end = rc.execute_with(frame_system::Pallet::<Polkadot>::block_number);
	let ah_block_end = ah.execute_with(frame_system::Pallet::<AssetHub>::block_number);

	rc.execute_with(|| {
		assert_eq!(RcMigrationStartBlock::<Polkadot>::get(), Some(rc_block_start + 1));
		assert_eq!(RcMigrationEndBlock::<Polkadot>::get(), Some(rc_block_end));
	});

	ah.execute_with(|| {
		assert_eq!(AhMigrationStartBlock::<AssetHub>::get(), Some(ah_block_start + 1));
		assert_eq!(AhMigrationEndBlock::<AssetHub>::get(), Some(ah_block_end));
	});

	// Post-checks on the Relay
	run_check(|| RcChecks::post_check(rc_pre.clone().unwrap()), &mut rc);

	// Post-checks on the Asset Hub
	run_check(|| AhChecks::post_check(rc_pre.clone().unwrap(), ah_pre.clone().unwrap()), &mut ah);

	println!(
		"Migration done in {} RC blocks, {} AH blocks",
		rc_block_end - rc_block_start,
		ah_block_end - ah_block_start
	);

	// run the migration again to check its idempotent

	// Set the initial migration stage from env var if set.
	set_initial_migration_stage(
		&mut rc,
		Some(RcMigrationStage::AccountsMigrationOngoing { last_key: None }),
	);
	ah.execute_with(|| {
		AhMigrationStageStorage::<AssetHub>::put(AhMigrationStage::DataMigrationOngoing);
	});

	let new_ah_block_start = ah.execute_with(frame_system::Pallet::<AssetHub>::block_number);

	log::info!("Running the migration second time");

	migrate(new_ah_block_start, &mut rc, &mut ah);

	let new_rc_block_end = rc.execute_with(frame_system::Pallet::<Polkadot>::block_number);
	let new_ah_block_end = ah.execute_with(frame_system::Pallet::<AssetHub>::block_number);

	rc.execute_with(|| {
		assert_eq!(RcMigrationStartBlock::<Polkadot>::get(), Some(rc_block_start + 1));
		assert_eq!(RcMigrationEndBlock::<Polkadot>::get(), Some(new_rc_block_end));
	});

	ah.execute_with(|| {
		assert_eq!(AhMigrationStartBlock::<AssetHub>::get(), Some(ah_block_start + 1));
		assert_eq!(AhMigrationEndBlock::<AssetHub>::get(), Some(new_ah_block_end));
	});

	// run post checks with the pre checks data from the first migration

	// Post-checks on the Relay
	run_check(|| RcChecks::post_check(rc_pre.clone().unwrap()), &mut rc);

	// Post-checks on the Asset Hub
	run_check(|| AhChecks::post_check(rc_pre.unwrap(), ah_pre.unwrap()), &mut ah);
}

#[tokio::test]
async fn scheduled_migration_works() {
	let Some((mut rc, mut ah)) = load_externalities().await else { return };

	// Check that the migration is pending on the RC.
	rc.execute_with(|| {
		log::info!("Asserting the initial state on RC");
		next_block_rc();

		assert_eq!(RcMigrationStageStorage::<Polkadot>::get(), RcMigrationStage::Pending);

		// clear the DMP queue.
		let _ = DownwardMessageQueues::<Polkadot>::take(AH_PARA_ID);
	});
	rc.commit_all().unwrap();

	// Check that the migration is pending on the AH.
	ah.execute_with(|| {
		log::info!("Asserting the initial state on AH");
		next_block_ah();

		assert_eq!(AhMigrationStageStorage::<AssetHub>::get(), AhMigrationStage::Pending);

		// clear the UMP queue.
		let _ = PendingUpwardMessages::<AssetHub>::take();
	});
	ah.commit_all().unwrap();

	let mut start = 0u32;
	let mut warm_up_end = 0u32;
	// 2 blocks after the end of the data migration.
	let cool_off_end = DispatchTime::After(2u32);

	// Schedule the migration on RC.
	let dmp_messages = rc.execute_with(|| {
		log::info!("Scheduling the migration on RC");
		next_block_rc();

		let now = frame_system::Pallet::<Polkadot>::block_number();
		start = now + 2;
		warm_up_end = start + 3;

		// Fellowship Origin
		let origin = pallet_xcm::Origin::Xcm(Location::new(
			0,
			[
				Junction::Parachain(1001),
				Junction::Plurality { id: BodyId::Technical, part: BodyPart::Voice },
			],
		));
		assert_ok!(RcMigrator::schedule_migration(
			origin.into(),
			DispatchTime::At(start),
			DispatchTime::At(warm_up_end),
			cool_off_end,
		));
		assert_eq!(
			RcMigrationStageStorage::<Polkadot>::get(),
			RcMigrationStage::Scheduled { start }
		);

		next_block_rc();
		// migrating not yet started
		assert_eq!(
			RcMigrationStageStorage::<Polkadot>::get(),
			RcMigrationStage::Scheduled { start }
		);
		assert_eq!(DownwardMessageQueues::<Polkadot>::take(AH_PARA_ID).len(), 0);

		next_block_rc();

		// migration is waiting for AH to acknowledge the start
		assert_eq!(RcMigrationStageStorage::<Polkadot>::get(), RcMigrationStage::WaitingForAh);
		let dmp_messages = DownwardMessageQueues::<Polkadot>::take(AH_PARA_ID);
		assert!(!dmp_messages.is_empty());

		dmp_messages
	});

	// enqueue DMP messages from RC to AH.
	ah.execute_with(|| {
		enqueue_dmp((dmp_messages, 0u32));
	});
	ah.commit_all().unwrap();

	// Asset Hub receives the message from the Relay Chain to start the migration and the
	// acknowledges it by sending the message back to the Relay Chain.
	let ump_messages = ah.execute_with(|| {
		log::info!("Acknowledging the start of the migration on AH");
		assert_eq!(AhMigrationStageStorage::<AssetHub>::get(), AhMigrationStage::Pending);

		next_block_ah();

		assert_eq!(
			AhMigrationStageStorage::<AssetHub>::get(),
			AhMigrationStage::DataMigrationOngoing
		);

		PendingUpwardMessages::<AssetHub>::take()
	});
	ah.commit_all().unwrap();

	// enqueue UMP messages from AH to RC.
	rc.execute_with(|| {
		enqueue_ump((ump_messages, 0u32));
	});
	rc.commit_all().unwrap();

	// Relay Chain receives the acknowledgement from the Asset Hub and starts sending the data.
	rc.execute_with(|| {
		log::info!("Receiving the acknowledgement from AH on RC");

		assert_eq!(RcMigrationStageStorage::<Polkadot>::get(), RcMigrationStage::WaitingForAh);

		next_block_rc();

		let end_at = warm_up_end;

		// cooling off
		assert_eq!(RcMigrationStageStorage::<Polkadot>::get(), RcMigrationStage::WarmUp { end_at });

		next_block_rc();

		// still cooling off
		assert_eq!(RcMigrationStageStorage::<Polkadot>::get(), RcMigrationStage::WarmUp { end_at });

		next_block_rc();

		// starting
		assert_eq!(RcMigrationStageStorage::<Polkadot>::get(), RcMigrationStage::Starting);

		next_block_rc();

		// accounts migration init
		assert_eq!(
			RcMigrationStageStorage::<Polkadot>::get(),
			RcMigrationStage::AccountsMigrationInit
		);
	});
	rc.commit_all().unwrap();

	// Relay Chain receives the acknowledgement from the Asset Hub and starts sending the data.
	rc.execute_with(|| {
		log::info!("Fast forward to the data migrating finish");

		RcMigrationStageStorage::<Polkadot>::set(RcMigrationStage::StakingMigrationDone);

		let now = frame_system::Pallet::<Polkadot>::block_number();

		next_block_rc();

		let now = now + 1;
		let end_at = cool_off_end.evaluate(now);

		// cooling off
		assert_eq!(
			RcMigrationStageStorage::<Polkadot>::get(),
			RcMigrationStage::CoolOff { end_at }
		);

		next_block_rc();

		// still cooling off
		assert_eq!(
			RcMigrationStageStorage::<Polkadot>::get(),
			RcMigrationStage::CoolOff { end_at }
		);

		next_block_rc();

		// cool-off end
		assert_eq!(
			RcMigrationStageStorage::<Polkadot>::get(),
			RcMigrationStage::SignalMigrationFinish
		);
	});
	rc.commit_all().unwrap();
}

#[tokio::test]
async fn some_account_migration_works() {
	use frame_system::Account as SystemAccount;
	use pallet_rc_migrator::accounts::AccountsMigrator;

	let Some((mut rc, mut ah)) = load_externalities().await else { return };

	let accounts: Vec<AccountId32> = vec![
		// 18.03.2025 - account with reserve above ED, but no free balance
		"5HB5nWBF2JfqogQYTcVkP1BfrgfadBizGmLBhmoAbGm5C7ir".parse().unwrap(),
		// 18.03.2025 - account with zero free balance, and reserve below ED
		"5GTtcseuBoAVLbxQ32XRnqkBmxxDaHqdpPs8ktUnH1zE4Cg3".parse().unwrap(),
		// 18.03.2025 - account with free balance below ED, and reserve above ED
		"5HMehBKuxRq7AqdxwQcaM7ff5e8Snchse9cNNGT9wsr4CqBK".parse().unwrap(),
		// 19.06.2025 - account with free balance below ED, some reserved balance to keep on the
		// RC, and a staking hold to migrate to AH.
		"5CfWkGnSnG89mGm3HjSUYHfJrNKmeyWgix5NKAMqePu4csgP".parse().unwrap(),
		// 04.07.2025 - account with free balance below ED, a delegated-staking hold to migrate to
		// AH. When migrating the hold on AH, the free balance is dusted.
		"5HBpFvUckfYEevbMnGXgGidcCRBygFww1FyksaJXYxjagPCK".parse().unwrap(),
	];

	for account_id in accounts {
		let maybe_withdrawn_account = rc.execute_with(|| {
			let rc_account = SystemAccount::<Polkadot>::get(&account_id);
			log::info!("Migrating account id: {:?}", account_id.to_ss58check());
			log::info!("RC account info: {rc_account:?}");

			let maybe_withdrawn_account = AccountsMigrator::<Polkadot>::withdraw_account(
				account_id,
				rc_account,
				&mut WeightMeter::new(),
				0,
			)
			.unwrap_or_else(|err| {
				log::error!("Account withdrawal failed: {err:?}");
				None
			});

			maybe_withdrawn_account
		});

		let withdrawn_account = match maybe_withdrawn_account {
			Some(withdrawn_account) => withdrawn_account,
			None => {
				log::warn!("Account is not withdrawable");
				continue;
			},
		};

		log::info!("Withdrawn account: {withdrawn_account:?}");

		ah.execute_with(|| {
			use asset_hub_polkadot_runtime::AhMigrator;
			use codec::{Decode, Encode};

			let encoded_account = withdrawn_account.encode();
			let account = Decode::decode(&mut &encoded_account[..]).unwrap();
			let res = AhMigrator::do_receive_account(account);
			log::info!("Account integration result: {res:?}");
		});
	}
}

#[test]
fn test_account_references() {
	type PalletBalances = pallet_balances::Pallet<Polkadot>;
	type PalletSystem = frame_system::Pallet<Polkadot>;

	new_test_rc_ext().execute_with(|| {
		// create new account.
		let who: AccountId32 = [0; 32].into();
		let ed = <PalletBalances as Currency<_>>::minimum_balance();
		let _ = PalletBalances::deposit_creating(&who, ed + ed + ed);

		// account is create with right balance and references.
		assert_eq!(PalletBalances::balance(&who), ed + ed + ed);
		assert_eq!(PalletSystem::consumers(&who), 0);
		assert_eq!(PalletSystem::providers(&who), 1);

		// decrement consumer reference from `0`.
		PalletSystem::dec_consumers(&who);

		// account is still alive.
		assert_eq!(PalletBalances::balance(&who), ed + ed + ed);
		assert_eq!(PalletSystem::consumers(&who), 0);
		assert_eq!(PalletSystem::providers(&who), 1);

		// reserve some balance which results `+1` consumer reference.
		PalletBalances::reserve(&who, ed).expect("reserve failed");

		// account data is valid.
		assert_eq!(PalletBalances::balance(&who), ed + ed);
		assert_eq!(PalletBalances::reserved_balance(&who), ed);
		assert_eq!(PalletSystem::consumers(&who), 1);
		assert_eq!(PalletSystem::providers(&who), 1);

		// force decrement consumer reference from `1`.
		PalletSystem::dec_consumers(&who);

		// account is still alive.
		assert_eq!(PalletBalances::balance(&who), ed + ed);
		assert_eq!(PalletBalances::reserved_balance(&who), ed);
		assert_eq!(PalletSystem::consumers(&who), 0);
		assert_eq!(PalletSystem::providers(&who), 1);

		// transfer some balance (or perform any update on account) to new account which results
		// consumer reference to automatically correct the consumer reference since the reserve
		// is still there.
		let who2: AccountId32 = [1; 32].into();
		PalletBalances::transfer(&who, &who2, ed, ExistenceRequirement::AllowDeath)
			.expect("transfer failed");

		// account is still alive, and consumer reference is corrected.
		assert_eq!(PalletBalances::balance(&who), ed);
		assert_eq!(PalletBalances::reserved_balance(&who), ed);
		assert_eq!(PalletSystem::consumers(&who), 1);
		assert_eq!(PalletSystem::providers(&who), 1);

		// force decrement consumer reference from `1`.
		PalletSystem::dec_consumers(&who);

		// account is still alive, and consumer reference is force decremented.
		assert_eq!(PalletBalances::balance(&who), ed);
		assert_eq!(PalletBalances::reserved_balance(&who), ed);
		assert_eq!(PalletSystem::consumers(&who), 0);
		assert_eq!(PalletSystem::providers(&who), 1);

		// try to kill the account by transfer all.
		assert_eq!(
			PalletBalances::transfer(&who, &who2, ed + ed, ExistenceRequirement::AllowDeath),
			Err(TokenError::FundsUnavailable.into())
		);

		// account is still alive.
		assert_eq!(PalletBalances::balance(&who), ed);
		assert_eq!(PalletBalances::reserved_balance(&who), ed);
		assert_eq!(PalletSystem::consumers(&who), 0);
		assert_eq!(PalletSystem::providers(&who), 1);

		// try to transfer all free balance, leaving only reserve.
		assert_eq!(
			PalletBalances::transfer(&who, &who2, ed, ExistenceRequirement::AllowDeath),
			Err(DispatchError::ConsumerRemaining)
		);

		// account is still alive. in this case consumer reference even gets corrected.
		assert_eq!(PalletBalances::balance(&who), ed);
		assert_eq!(PalletBalances::reserved_balance(&who), ed);
		assert_eq!(PalletSystem::consumers(&who), 1);
		assert_eq!(PalletSystem::providers(&who), 1);
	});
}

#[test]
fn test_control_flow() {
	let mut rc: sp_io::TestExternalities = frame_system::GenesisConfig::<RcRuntime>::default()
		.build_storage()
		.unwrap()
		.into();
	let mut ah: sp_io::TestExternalities = frame_system::GenesisConfig::<AhRuntime>::default()
		.build_storage()
		.unwrap()
		.into();

	let mut rc_now = 0;
	let mut ah_now = 0;

	// prepare the RC to send XCM messages to AH and Collectives.
	rc.execute_with(|| {
		rc_now += 1;
		frame_system::Pallet::<RcRuntime>::reset_events();
		frame_system::Pallet::<RcRuntime>::set_block_number(rc_now);

		// prepare the RC to send XCM messages to AH.
		let result =
			RcRuntimeCall::XcmPallet(pallet_xcm::Call::<RcRuntime>::force_default_xcm_version {
				maybe_xcm_version: Some(xcm::prelude::XCM_VERSION),
			})
			.dispatch(RcRuntimeOrigin::root());

		// set the max downward message size to 51200.
		runtime_parachains::configuration::ActiveConfig::<RcRuntime>::mutate(|config| {
			config.max_downward_message_size = 51200;
		});

		// make the Asset Hub from RC reachable.
		polkadot_runtime::Dmp::make_parachain_reachable(1000);

		assert!(result.is_ok(), "fails with error: {:?}", result.err());
	});

	// prepare the AH to send XCM messages to RC and Collectives.
	ah.execute_with(|| {
		ah_now += 1;
		frame_system::Pallet::<AhRuntime>::reset_events();
		frame_system::Pallet::<AhRuntime>::set_block_number(ah_now);

		// setup default XCM version
		let result =
			AhRuntimeCall::PolkadotXcm(pallet_xcm::Call::<AhRuntime>::force_default_xcm_version {
				maybe_xcm_version: Some(xcm::prelude::XCM_VERSION),
			})
			.dispatch(AhRuntimeOrigin::root());
		assert!(result.is_ok(), "fails with error: {:?}", result.err());
	});

	let (first_query_id, second_query_id, third_query_id) = rc.execute_with(|| {
		assert_eq!(pallet_rc_migrator::PendingXcmMessages::<RcRuntime>::count(), 0);
		// the query ids are incremented by 1 for each message.
		(0, 1, 2)
	});

	// send invalid XCM message from RC to AH via rc-migrator.
	let dmp_messages = rc.execute_with(|| {
		rc_now += 1;
		frame_system::Pallet::<RcRuntime>::reset_events();
		frame_system::Pallet::<RcRuntime>::set_block_number(rc_now);

		let mut batch = pallet_rc_migrator::types::XcmBatchAndMeter::new_from_config::<RcRuntime>();
		batch.push((None, vec![], vec![]));
		// adding a second item; this will cause the dispatchable on AH to fail.
		batch.push((None, vec![], vec![]));

		pallet_rc_migrator::Pallet::<RcRuntime>::send_chunked_xcm_and_track(batch, |batch| {
			pallet_rc_migrator::types::AhMigratorCall::<RcRuntime>::ReceiveReferendaValues {
				values: batch,
			}
		})
		.expect("failed to send XCM messages");

		// make sure the message buffered in the rc migrator.
		let message_hash = pallet_rc_migrator::PendingXcmQueries::<RcRuntime>::get(first_query_id)
			.expect("query id not found");
		assert!(pallet_rc_migrator::PendingXcmMessages::<RcRuntime>::get(message_hash).is_some());

		// take the message from the queue to feed it later to the AH message processor.
		let dmp_messages = DownwardMessageQueues::<RcRuntime>::take(AH_PARA_ID);
		assert_eq!(dmp_messages.len(), 1);

		dmp_messages
	});

	// process the message in the AH.
	let ump_messages = ah.execute_with(|| {
		ah_now += 1;
		frame_system::Pallet::<AhRuntime>::reset_events();
		frame_system::Pallet::<AhRuntime>::set_block_number(ah_now);

		enqueue_dmp((dmp_messages, 0u32));

		<asset_hub_polkadot_runtime::MessageQueue as OnInitialize<_>>::on_initialize(ah_now);
		<asset_hub_polkadot_runtime::MessageQueue as OnFinalize<_>>::on_finalize(ah_now);

		// take the acknowledgement message from the AH.
		let ump_messages = PendingUpwardMessages::<AhRuntime>::take();
		assert_eq!(ump_messages.len(), 1);

		ump_messages
	});

	// process the acknowledgement message from AH in the RC.
	rc.execute_with(|| {
		rc_now += 1;
		frame_system::Pallet::<RcRuntime>::reset_events();
		frame_system::Pallet::<RcRuntime>::set_block_number(rc_now);

		enqueue_ump((ump_messages, 0u32));

		<polkadot_runtime::MessageQueue as OnInitialize<_>>::on_initialize(rc_now);
		<polkadot_runtime::MessageQueue as OnFinalize<_>>::on_finalize(rc_now);

		// make sure the message is still buffered since the message failed to be processed on AH.
		let message_hash = pallet_rc_migrator::PendingXcmQueries::<RcRuntime>::get(first_query_id)
			.expect("query id not found");
		assert!(pallet_rc_migrator::PendingXcmMessages::<RcRuntime>::get(message_hash).is_some());

		// RC migrator has received the response from the AH indicating that the message failed to
		// be processed.
		assert!(frame_system::Pallet::<Polkadot>::events().first().is_some_and(|record| {
			match &record.event {
				RcRuntimeEvent::RcMigrator(pallet_rc_migrator::Event::QueryResponseReceived {
					query_id,
					response: MaybeErrorCode::Error(..),
				}) => *query_id == first_query_id,
				_ => {
					println!("actual event: {:?}", &record.event);
					false
				},
			}
		}));
	});

	// send valid XCM message from RC to AH via rc-migrator.
	let dmp_messages = rc.execute_with(|| {
		rc_now += 1;
		frame_system::Pallet::<RcRuntime>::reset_events();
		frame_system::Pallet::<RcRuntime>::set_block_number(rc_now);

		let mut batch = pallet_rc_migrator::types::XcmBatchAndMeter::new_from_config::<RcRuntime>();
		batch.push((None, vec![], vec![]));

		pallet_rc_migrator::Pallet::<RcRuntime>::send_chunked_xcm_and_track(batch, |batch| {
			pallet_rc_migrator::types::AhMigratorCall::<RcRuntime>::ReceiveReferendaValues {
				values: batch,
			}
		})
		.expect("failed to send XCM messages");

		// make sure the second message buffered in the rc migrator.
		let second_message_hash =
			pallet_rc_migrator::PendingXcmQueries::<RcRuntime>::get(second_query_id)
				.expect("query id not found");
		assert!(
			pallet_rc_migrator::PendingXcmMessages::<RcRuntime>::get(second_message_hash).is_some()
		);

		// take the message from the queue and drop it to make sure AH will not process and
		// acknowledge it.
		let dmp_messages = DownwardMessageQueues::<RcRuntime>::take(AH_PARA_ID);
		assert_eq!(dmp_messages.len(), 1);

		// resend the buffered message via rc-migrator.
		let result = RcRuntimeCall::RcMigrator(pallet_rc_migrator::Call::<RcRuntime>::resend_xcm {
			query_id: second_query_id,
		})
		.dispatch(RcRuntimeOrigin::root());

		assert!(result.is_ok(), "fails with error: {:?}", result.err());

		// make sure rc-migrator created a new query response request and buffered the message
		// again with the new query id.
		assert!(pallet_rc_migrator::PendingXcmQueries::<RcRuntime>::get(third_query_id).is_some());
		assert!(
			pallet_rc_migrator::PendingXcmMessages::<RcRuntime>::get(second_message_hash).is_some()
		);

		// take the message from the queue to feed it later to the AH message processor.
		let dmp_messages = DownwardMessageQueues::<RcRuntime>::take(AH_PARA_ID);
		assert_eq!(dmp_messages.len(), 1);

		dmp_messages
	});

	// process the message in the AH.
	let ump_messages = ah.execute_with(|| {
		ah_now += 1;
		frame_system::Pallet::<AhRuntime>::reset_events();
		frame_system::Pallet::<AhRuntime>::set_block_number(ah_now);

		enqueue_dmp((dmp_messages, 0u32));

		<asset_hub_polkadot_runtime::MessageQueue as OnInitialize<_>>::on_initialize(ah_now);
		<asset_hub_polkadot_runtime::MessageQueue as OnFinalize<_>>::on_finalize(ah_now);

		// take the acknowledgement message from the AH.
		let ump_messages = PendingUpwardMessages::<AhRuntime>::take();
		assert_eq!(ump_messages.len(), 1);

		ump_messages
	});

	// process the acknowledgement message from AH in the RC.
	rc.execute_with(|| {
		rc_now += 1;
		frame_system::Pallet::<RcRuntime>::reset_events();
		frame_system::Pallet::<RcRuntime>::set_block_number(rc_now);

		enqueue_ump((ump_messages, 0u32));

		<polkadot_runtime::MessageQueue as OnInitialize<_>>::on_initialize(rc_now);
		<polkadot_runtime::MessageQueue as OnFinalize<_>>::on_finalize(rc_now);

		// RC migrator has received the response from the AH indicating that the message was
		// successfully processed.
		assert!(frame_system::Pallet::<Polkadot>::events().first().is_some_and(|record| {
			match &record.event {
				RcRuntimeEvent::RcMigrator(pallet_rc_migrator::Event::QueryResponseReceived {
					query_id,
					response: MaybeErrorCode::Success,
				}) => *query_id == third_query_id,
				_ => {
					println!("actual event: {:?}", &record.event);
					false
				},
			}
		}));

		// make sure the message is not buffered since the acknowledgement of successful processing
		// received from AH.
		let second_message_hash =
			pallet_rc_migrator::PendingXcmQueries::<RcRuntime>::get(second_query_id)
				.expect("query id not found");
		assert!(
			pallet_rc_migrator::PendingXcmMessages::<RcRuntime>::get(second_message_hash).is_none()
		);
		assert!(pallet_rc_migrator::PendingXcmQueries::<RcRuntime>::get(third_query_id).is_none());

		// make sure the first message is still buffered.
		let first_message_hash =
			pallet_rc_migrator::PendingXcmQueries::<RcRuntime>::get(first_query_id)
				.expect("query id not found");
		assert!(
			pallet_rc_migrator::PendingXcmMessages::<RcRuntime>::get(first_message_hash).is_some()
		);
	});
}

#[tokio::test]
async fn post_migration_checks_only() {
	//! Migration invariant checks across distinct pre/post snapshots.
	//! Env vars (must all be set):
	//!   SNAP_RC_PRE  - Relay Chain (pre-migration) snapshot
	//!   SNAP_AH_PRE  - Asset Hub  (pre-migration) snapshot
	//!   SNAP_RC_POST - Relay Chain (post-migration) snapshot
	//!   SNAP_AH_POST - Asset Hub  (post-migration) snapshot

	use polkadot_runtime::Block as PolkadotBlock;
	use remote_externalities::{Builder, Mode, OfflineConfig};

	sp_tracing::try_init_simple();

	// Helper to load a snapshot from env var name (panic if missing / fails).
	async fn load_ext(var: &str) -> TestExternalities {
		let snap = std::env::var(var).unwrap_or_else(|_| panic!("Missing env var {var}"));
		let abs = std::path::absolute(&snap).expect("abs path");
		let remote = Builder::<PolkadotBlock>::default()
			.mode(Mode::Offline(OfflineConfig { state_snapshot: snap.clone().into() }))
			.build()
			.await
			.unwrap_or_else(|e| panic!("Failed to load snapshot {abs:?}: {e:?}"));
		let (kv, root) = remote.inner_ext.into_raw_snapshot();
		TestExternalities::from_raw_snapshot(kv, root, sp_storage::StateVersion::V1)
	}

	let mut rc_pre_ext = load_ext("SNAP_RC_PRE").await;
	let mut ah_pre_ext = load_ext("SNAP_AH_PRE").await;

	let mut rc_post_ext = load_ext("SNAP_RC_POST").await;
	let mut ah_post_ext = load_ext("SNAP_AH_POST").await;

	rc_pre_ext.execute_with(|| {
		log::info!(target: "ahm", "PRE: RC migration stage: {:?}", RcMigrationStageStorage::<Polkadot>::get());
	});
	ah_pre_ext.execute_with(|| {
		log::info!(target: "ahm", "PRE: AH migration stage: {:?}", AhMigrationStageStorage::<AssetHub>::get());
	});
	rc_post_ext.execute_with(|| {
		log::info!(target: "ahm", "POST: RC migration stage: {:?}", RcMigrationStageStorage::<Polkadot>::get());
	});
	ah_post_ext.execute_with(|| {
		log::info!(target: "ahm", "POST: AH migration stage: {:?}", AhMigrationStageStorage::<AssetHub>::get());
	});

	let rc_pre_payload = rc_pre_ext.execute_with(RcChecks::pre_check);
	let ah_pre_payload = ah_pre_ext.execute_with(|| AhChecks::pre_check(rc_pre_payload.clone()));

	std::mem::drop(rc_pre_ext);
	std::mem::drop(ah_pre_ext);


	rc_post_ext.execute_with(|| RcChecks::post_check(rc_pre_payload.clone()));
	ah_post_ext.execute_with(|| AhChecks::post_check(rc_pre_payload, ah_pre_payload));
}
