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
//!     -p polkadot-integration-tests-ahm \
//!     --features try-runtime \
//!     --features {{runtime}}-ahm \
//!     --release \
//!     post_migration_checks_only \
//!     -- --nocapture --ignored
//! ```

use crate::porting_prelude::*;

use super::{
	accounts_translation_works::AccountTranslationWorks,
	balances_test::BalancesCrossChecker,
	checks::{EntireStateDecodes, PalletsTryStateCheck, SanityChecks},
	mock::*,
	multisig_still_work::MultisigStillWork,
	multisig_test::MultisigsAccountIdStaysTheSame,
	proxy::ProxyBasicWorks,
};
use asset_hub_polkadot_runtime::{AhMigrator, Runtime as AssetHub, Runtime as PAH};
use cumulus_pallet_parachain_system::PendingUpwardMessages;
use cumulus_primitives_core::{InboundDownwardMessage, Junction, Location, UpwardMessageSender};
use frame_support::{
	assert_noop, hypothetically, hypothetically_ok,
	traits::{
		fungible::Inspect, schedule::DispatchTime, Currency, ExistenceRequirement, OnFinalize,
		OnInitialize, ReservableCurrency,
	},
};
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_ah_migrator::{
	sovereign_account_translation::{DERIVED_TRANSLATIONS, SOV_TRANSLATIONS},
	types::AhMigrationCheck,
	AhMigrationStage as AhMigrationStageStorage, MigrationEndBlock as AhMigrationEndBlock,
	MigrationStage as AhMigrationStage, MigrationStartBlock as AhMigrationStartBlock,
};
use pallet_rc_migrator::{
	staking::StakingMigratedCorrectly, types::RcMigrationCheck,
	MigrationEndBlock as RcMigrationEndBlock, MigrationStage as RcMigrationStage,
	MigrationStartBlock as RcMigrationStartBlock, RcMigrationStage as RcMigrationStageStorage,
};
use polkadot_primitives::UpwardMessage;
use polkadot_runtime::{RcMigrator, Runtime as Polkadot};
use rand::Rng;
use runtime_parachains::dmp::DownwardMessageQueues;
use sp_core::crypto::Ss58Codec;
use sp_io::TestExternalities;
use sp_runtime::{traits::Dispatchable, AccountId32, BuildStorage, DispatchError, TokenError};
use std::{collections::VecDeque, str::FromStr};
use xcm::latest::*;
use xcm_emulator::{assert_ok, WeightMeter};
#[cfg(all(feature = "polkadot-ahm", feature = "kusama-ahm"))]
use ::{
	cumuluse_primitives_core::ParaId,
	sp_core::ByteArray,
	std::collections::{BTreeMap, BTreeSet},
	xcm_emulator::ConvertLocation,
};

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
	EntireStateDecodes,
);

// Checks that are specific to Polkadot, and not available on other chains
#[cfg(feature = "polkadot-ahm")]
pub type RcRuntimeSpecificChecks = (
	MultisigsAccountIdStaysTheSame,
	pallet_rc_migrator::multisig::MultisigMigrationChecker<Polkadot>,
	pallet_rc_migrator::bounties::BountiesMigrator<Polkadot>,
	pallet_rc_migrator::treasury::TreasuryMigrator<Polkadot>,
	pallet_rc_migrator::claims::ClaimsMigrator<Polkadot>,
	pallet_rc_migrator::crowdloan::CrowdloanMigrator<Polkadot>,
	crate::proxy::ProxyWhaleWatching,
	StakingMigratedCorrectly<Polkadot>,
	pallet_rc_migrator::child_bounties::ChildBountiesMigratedCorrectly<Polkadot>,
);

// Checks that are specific to Kusama.
#[cfg(feature = "kusama-ahm")]
pub type RcRuntimeSpecificChecks = (
	MultisigsAccountIdStaysTheSame,
	pallet_rc_migrator::multisig::MultisigMigrationChecker<Polkadot>,
	pallet_rc_migrator::bounties::BountiesMigrator<Polkadot>,
	pallet_rc_migrator::treasury::TreasuryMigrator<Polkadot>,
	pallet_rc_migrator::claims::ClaimsMigrator<Polkadot>,
	pallet_rc_migrator::crowdloan::CrowdloanMigrator<Polkadot>,
	crate::account_whale_watching::BalanceWhaleWatching,
	crate::proxy::ProxyWhaleWatching,
	StakingMigratedCorrectly<Polkadot>,
	super::recovery_test::RecoveryDataMigrated,
	pallet_rc_migrator::society::tests::SocietyMigratorTest<Polkadot>,
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
	EntireStateDecodes,
);

#[cfg(feature = "polkadot-ahm")]
pub type AhRuntimeSpecificChecks = (
	MultisigsAccountIdStaysTheSame,
	pallet_rc_migrator::multisig::MultisigMigrationChecker<AssetHub>,
	pallet_rc_migrator::bounties::BountiesMigrator<AssetHub>,
	pallet_rc_migrator::treasury::TreasuryMigrator<AssetHub>,
	pallet_rc_migrator::claims::ClaimsMigrator<AssetHub>,
	pallet_rc_migrator::crowdloan::CrowdloanMigrator<AssetHub>,
	crate::proxy::ProxyWhaleWatching,
	StakingMigratedCorrectly<AssetHub>,
	pallet_rc_migrator::child_bounties::ChildBountiesMigratedCorrectly<AssetHub>,
);

#[cfg(feature = "kusama-ahm")]
pub type AhRuntimeSpecificChecks = (
	MultisigsAccountIdStaysTheSame,
	pallet_rc_migrator::multisig::MultisigMigrationChecker<AssetHub>,
	pallet_rc_migrator::bounties::BountiesMigrator<AssetHub>,
	pallet_rc_migrator::treasury::TreasuryMigrator<AssetHub>,
	pallet_rc_migrator::claims::ClaimsMigrator<AssetHub>,
	pallet_rc_migrator::crowdloan::CrowdloanMigrator<AssetHub>,
	crate::account_whale_watching::BalanceWhaleWatching,
	crate::proxy::ProxyWhaleWatching,
	StakingMigratedCorrectly<AssetHub>,
	super::recovery_test::RecoveryDataMigrated,
	pallet_rc_migrator::society::tests::SocietyMigratorTest<AssetHub>,
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

#[cfg(feature = "polkadot-ahm")]
#[tokio::test]
async fn num_leases_to_ending_block_works_simple() {
	use polkadot_runtime_common::slots as pallet_slots;

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
#[cfg(all(feature = "polkadot-ahm", feature = "kusama-ahm"))]
async fn find_translatable_accounts() {
	let mut dot_rc = load_externalities_uncached("POLKADOT_RC_SNAP").await.unwrap();
	let mut kusama_rc = load_externalities_uncached("KUSAMA_RC_SNAP").await.unwrap();

	// Extract all accounts from the RC
	let rc_accounts = dot_rc
		.execute_with(|| frame_system::Account::<Polkadot>::iter_keys().collect::<BTreeSet<_>>());
	let kusama_rc_accounts = kusama_rc.execute_with(|| {
		frame_system::Account::<kusama_runtime::Runtime>::iter_keys().collect::<BTreeSet<_>>()
	});
	let all_accounts = rc_accounts.union(&kusama_rc_accounts).collect::<BTreeSet<_>>();

	let (sov_translations, derived_translations) = account_translation_map(&all_accounts);
	write_account_translation_map(&sov_translations, &derived_translations);
}

#[cfg(all(feature = "polkadot-ahm", feature = "kusama-ahm"))]
#[allow(clippy::type_complexity)]
fn account_translation_map(
	rc_accounts: &BTreeSet<&AccountId32>,
) -> (Vec<(u32, (AccountId32, AccountId32))>, Vec<(ParaId, AccountId32, u16, AccountId32)>) {
	println!("Found {} RC accounts", rc_accounts.len());

	// Para ID -> (RC sovereign, AH sovereign)
	let mut sov_translations = BTreeMap::<u32, (AccountId32, AccountId32)>::new();
	// Para ID -> (RC derived, index, AH derived)
	let mut derived_translations = Vec::<(ParaId, AccountId32, u16, AccountId32)>::new();

	// Try to find Para sovereign and derived accounts.
	for para_id in 0..(u16::MAX as u32) {
		// The Parachain sovereign account ID on the relay chain
		let rc_para_sov =
			xcm_builder::ChildParachainConvertsVia::<ParaId, AccountId32>::convert_location(
				&Location::new(0, Junction::Parachain(para_id)),
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
				derived_translations.push((
					para_id.into(),
					rc_para_derived,
					derivation_index,
					ah_para_derived,
				));
			}
		}
	}
	derived_translations.sort_by(|(_, rc_acc, _, _), (_, rc_acc2, _, _)| rc_acc.cmp(rc_acc2));

	println!("Found {} RC sovereign account translations", sov_translations.len());
	println!("Found {} RC derived   account translations", derived_translations.len());

	let mut sov_translations = sov_translations.into_iter().collect::<Vec<_>>();
	sov_translations.sort_by(|(_, (rc_acc, _)), (_, (rc_acc2, _))| rc_acc.cmp(rc_acc2));

	(sov_translations, derived_translations)
}

#[cfg(all(feature = "polkadot-ahm", feature = "kusama-ahm"))]
fn write_account_translation_map(
	sov_translations: &[(u32, (AccountId32, AccountId32))],
	derived_translations: &[(ParaId, AccountId32, u16, AccountId32)],
) {
	let mut rust = String::new();

	rust.push_str(
		"/// List of RC para to AH sibl sovereign account translation sorted by RC account.
pub const SOV_TRANSLATIONS: &[((AccountId32, &str), (AccountId32, &str))] = &[\n",
	);

	for (para_id, (rc_acc, ah_acc)) in sov_translations.iter() {
		rust.push_str(&format!("\t// para {para_id}\n"));
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
pub const DERIVED_TRANSLATIONS: &[((AccountId32, &str), u16, (AccountId32, &str))] = &[\n",
	);

	for (para_id, rc_acc, derivation_index, ah_acc) in derived_translations.iter() {
		rust.push_str(&format!("\t// para {para_id} (derivation index {derivation_index})\n"));
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

	let path =
		std::path::Path::new("../../pallets/ah-migrator/src/sovereign_account_translation.rs");
	println!("Writing to {}", std::fs::canonicalize(path).unwrap().display());
	let mut file = std::fs::File::open(path).unwrap();
	let mut contents = String::new();
	std::io::Read::read_to_string(&mut file, &mut contents).unwrap();

	// Replace everything after the "AUTOGENERATED BELOW" comment with our Rust string
	let pos_auto_gen = contents.find("// AUTOGENERATED BELOW").unwrap() + 23;
	contents.truncate(pos_auto_gen);
	contents.insert_str(pos_auto_gen, &rust);

	// Write the result back to the file
	std::fs::write(path, contents).unwrap();
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

#[cfg(all(feature = "polkadot-ahm", feature = "kusama-ahm"))]
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
		#[cfg(not(feature = "kusama-ahm"))]
		let origin = pallet_xcm::Origin::Xcm(Location::new(
			0,
			[
				Junction::Parachain(1001),
				Junction::Plurality { id: BodyId::Technical, part: BodyPart::Voice },
			],
		));
		#[cfg(feature = "kusama-ahm")]
		let origin = polkadot_runtime::governance::Origin::Fellows;

		assert_ok!(RcMigrator::schedule_migration(
			origin.into(),
			DispatchTime::At(start),
			DispatchTime::At(warm_up_end),
			cool_off_end,
			true, // Ignore the staking era check
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
			RcMigrationStage::PureProxyCandidatesMigrationInit
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

		asset_hub_polkadot_runtime::ParachainSystem::ensure_successful_delivery();

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
		batch.push(pallet_rc_migrator::referenda::ReferendaMessage {
			referendum_count: None,
			deciding_count: vec![],
			track_queue: vec![],
		});
		// adding a second item; this will cause the dispatchable on AH to fail.
		batch.push(pallet_rc_migrator::referenda::ReferendaMessage {
			referendum_count: None,
			deciding_count: vec![],
			track_queue: vec![],
		});

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
		batch.push(pallet_rc_migrator::referenda::ReferendaMessage {
			referendum_count: None,
			deciding_count: vec![],
			track_queue: vec![],
		});

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

#[ignore] // Ignored since CI will not have the pre and post snapshots.
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

	let rc_pre_payload = rc_pre_ext.execute_with(RcChecks::pre_check);
	let ah_pre_payload = ah_pre_ext.execute_with(|| AhChecks::pre_check(rc_pre_payload.clone()));

	std::mem::drop(rc_pre_ext);
	std::mem::drop(ah_pre_ext);

	rc_post_ext.execute_with(|| RcChecks::post_check(rc_pre_payload.clone()));
	ah_post_ext.execute_with(|| AhChecks::post_check(rc_pre_payload, ah_pre_payload));
}

#[test]
fn schedule_migration() {
	new_test_rc_ext().execute_with(|| {
		let now = u16::MAX as u32 * 2;
		frame_system::Pallet::<RcRuntime>::set_block_number(now);
		let session_duration = polkadot_runtime::EpochDuration::get() as u32;
		let rng = rand::rng().random_range(1..=u16::MAX) as u32;

		// Scheduling two sessions into the future works
		hypothetically_ok!(pallet_rc_migrator::Pallet::<RcRuntime>::schedule_migration(
			RcRuntimeOrigin::root(),
			DispatchTime::At(now + session_duration * 2 + 1), // start
			DispatchTime::At(u32::MAX),                       // no-op
			DispatchTime::At(u32::MAX),                       // no-op
			Default::default(),
		));

		// Scheduling more than two sessions into the future works
		hypothetically_ok!(pallet_rc_migrator::Pallet::<RcRuntime>::schedule_migration(
			RcRuntimeOrigin::root(),
			DispatchTime::At(now + session_duration * 2 + rng), // start
			DispatchTime::At(u32::MAX),                         // no-op
			DispatchTime::At(u32::MAX),                         // no-op
			Default::default(),
		));

		// Scheduling less than two sessions into the future fails
		hypothetically!(pallet_rc_migrator::Pallet::<RcRuntime>::schedule_migration(
			RcRuntimeOrigin::root(),
			DispatchTime::At(now + session_duration * 2), // start
			DispatchTime::At(u32::MAX),                   // no-op
			DispatchTime::At(u32::MAX),                   // no-op
			Default::default(),
		)
		.unwrap_err());

		// Scheduling less than two sessions into the future fails
		hypothetically!(pallet_rc_migrator::Pallet::<RcRuntime>::schedule_migration(
			RcRuntimeOrigin::root(),
			DispatchTime::At(now + session_duration * 2 - rng), // start
			DispatchTime::At(u32::MAX),                         // no-op
			DispatchTime::At(u32::MAX),                         // no-op
			Default::default(),
		)
		.unwrap_err());

		// Disabling the check makes it always work
		hypothetically_ok!(pallet_rc_migrator::Pallet::<RcRuntime>::schedule_migration(
			RcRuntimeOrigin::root(),
			DispatchTime::At(now + session_duration * 2), // start
			DispatchTime::At(u32::MAX),                   // no-op
			DispatchTime::At(u32::MAX),                   // no-op
			true,
		));
	});
}

#[test]
fn schedule_migration_staking_pause_works() {
	new_test_rc_ext().execute_with(|| {
		let now = u16::MAX as u32 * 2;
		frame_system::Pallet::<RcRuntime>::set_block_number(now);
		let session_duration = polkadot_runtime::EpochDuration::get() as u32;
		let rng = rand::rng().random_range(1..=10) as u32;

		// Scheduling two sessions into the future works
		hypothetically!({
			pallet_rc_migrator::Pallet::<RcRuntime>::schedule_migration(
				RcRuntimeOrigin::root(),
				DispatchTime::At(now + session_duration * 2 + rng), // start
				DispatchTime::At(u32::MAX),                         // no-op
				DispatchTime::At(u32::MAX),                         // no-op
				Default::default(),
			)
			.unwrap();

			for _ in 0..rng {
				next_block_rc();
			}

			assert!(frame_system::Pallet::<RcRuntime>::events().iter().any(|record| {
				matches!(
					&record.event,
					RcRuntimeEvent::RcMigrator(pallet_rc_migrator::Event::StakingElectionsPaused,)
				)
			}));
		});

		// If we ignore the check and schedule too soon, then it will not be paused
		hypothetically!({
			pallet_rc_migrator::Pallet::<RcRuntime>::schedule_migration(
				RcRuntimeOrigin::root(),
				DispatchTime::At(now + session_duration * 2 - rng), // start
				DispatchTime::At(u32::MAX),                         // no-op
				DispatchTime::At(u32::MAX),                         // no-op
				true,
			)
			.unwrap();

			for _ in 0..session_duration * 2 {
				next_block_rc();
				assert_eq!(frame_system::Pallet::<RcRuntime>::events(), Vec::new());
			}
		});
	});
}

#[test]
fn bifrost_addresses_are_in_translation_map() {
	#[cfg(feature = "kusama-ahm")]
	use asset_hub_kusama_runtime::Runtime as KAH;

	TestExternalities::default().execute_with(|| {
		let sov_cases = [
			(
				// 2030 Polkadot
				"13YMK2eeopZtUNpeHnJ1Ws2HqMQG6Ts9PGCZYGyFbSYoZfcm",
				"13cKp89TtYknbyYnqnF6dWN75q5ZosvFSuqzoEVkUAaNR47A",
			),
			(
				// 2001 Kusama
				"5Ec4AhPV91i9yNuiWuNunPf6AQCYDhFTTA4G5QCbtqYApH9E",
				"5Eg2fntJDju46yds4uKzu2zuQssqw7JZWohhLMj6mZZjg2pK",
			),
		];

		for (from, to) in sov_cases {
			let from = AccountId32::from_str(from).unwrap();
			let to = AccountId32::from_str(to).unwrap();

			assert_eq!(
				pallet_ah_migrator::Pallet::<PAH>::translate_account_rc_to_ah(from.clone()),
				to
			);
			assert_eq!(
				pallet_ah_migrator::Pallet::<PAH>::maybe_sovereign_translate(&from),
				Some(to.clone())
			);
			assert_eq!(pallet_ah_migrator::Pallet::<PAH>::maybe_derived_translate(&from), None);

			// Translations work regardless of the runtime:
			#[cfg(feature = "kusama-ahm")]
			assert_eq!(
				pallet_ah_migrator::Pallet::<KAH>::translate_account_rc_to_ah(from.clone()),
				to
			);
			#[cfg(feature = "kusama-ahm")]
			assert_eq!(
				pallet_ah_migrator::Pallet::<KAH>::maybe_sovereign_translate(&from),
				Some(to)
			);
			#[cfg(feature = "kusama-ahm")]
			assert_eq!(pallet_ah_migrator::Pallet::<KAH>::maybe_derived_translate(&from), None);
		}

		let derived_cases = [
			(
				// 2030 / 0 Polkadot
				"14vtfeKAVKh1Jzb3s7e43SqZ3zB5MLsdCxZPoKDxeoCFKLu5",
				"5ETehspFKFNpBbe5DsfuziN6BWq5Qwp1J8qcTQQoAxwa7BsS",
			),
			(
				// 2030 / 1 Polkadot
				"14QkQ7wVVDRrhbC1UqHsFwKFUns1SRud94CXMWGHWB8Jhtro",
				"5DNWZkkAxLhqF8tevcbRGyARAVM7abukftmqvoDFUN5dDDDz",
			),
			(
				// 2030 / 2 Polkadot
				"13hLwqcVHqjiJMbZhR9LtfdhoxmTdssi7Kp8EJaW2yfk3knK",
				"5EmiwjDYiackJma1GW3aBbQ74rLfWh756UKDb7Cm83XDkUUZ",
			),
			(
				// 2001 / 0 Kusama
				"5E78xTBiaN3nAGYtcNnqTJQJqYAkSDGggKqaDfpNsKyPpbcb",
				"5CzXNqgBZT5yMpMETdfH55saYNKQoJBXsSfnu4d2s1ejYFir",
			),
			(
				// 2001 / 1 Kusama
				"5HXi9pzWnTQzk7VKzY6VQn92KfWCcA5NbSm53uKHrYU1VsjP",
				"5GcexD4YNqcKTbW1YWDRczQzpxic61byeNeLaHgqQHk8pxQJ",
			),
			(
				// 2001 / 2 Kusama
				"5CkKS3YMx64TguUYrMERc5Bn6Mn2aKMUkcozUFREQDgHS3Tv",
				"5FoYMVucmT552GDMWfYNxcF2XnuuvLbJHt7mU6DfDCpUAS2Y",
			),
			(
				// 2001 / 3 Kusama
				"5Crxhmiw5CQq3Mnfcu3dR3yJ3YpjbxjqaeDFtNNtqgmcnN4S",
				"5FP39fgPYhJw3vcLwSMqMnwBuEVGexUMG6JQLPR9yPVhq6Wy",
			),
			(
				// 2001 / 4 Kusama
				"5DAZP4gZKZafGv42uoWNTMau4tYuDd2XteJLGL4upermhQpn",
				"5ExtLdYnjHLJbngU1QpumjPieCGaCXwwkH1JrFBQ9GATuNGv",
			),
		];

		for (from, to) in derived_cases {
			let from = AccountId32::from_str(from).unwrap();
			let to = AccountId32::from_str(to).unwrap();

			assert_eq!(
				pallet_ah_migrator::Pallet::<PAH>::translate_account_rc_to_ah(from.clone()),
				to
			);
			assert_eq!(pallet_ah_migrator::Pallet::<PAH>::maybe_sovereign_translate(&from), None);
			assert_eq!(
				pallet_ah_migrator::Pallet::<PAH>::maybe_derived_translate(&from),
				Some(to.clone())
			);

			// Translations work regardless of the runtime:
			#[cfg(feature = "kusama-ahm")]
			assert_eq!(
				pallet_ah_migrator::Pallet::<KAH>::translate_account_rc_to_ah(from.clone()),
				to
			);
			#[cfg(feature = "kusama-ahm")]
			assert_eq!(pallet_ah_migrator::Pallet::<KAH>::maybe_sovereign_translate(&from), None);
			#[cfg(feature = "kusama-ahm")]
			assert_eq!(pallet_ah_migrator::Pallet::<KAH>::maybe_derived_translate(&from), Some(to));
		}
	});
}

#[cfg(feature = "polkadot-ahm")]
#[test]
fn map_known_governance_calls() {
	use codec::Decode;
	use frame_support::traits::{Bounded, BoundedInline, StorePreimage};
	use hex_literal::hex;

	let calls = vec![
		("referendum_1729", hex!("1a020c1a0300016d6f646c70792f74727372790000000000000000000000000000000000000000630804000100c91f040001010065feec15496516bcfde6b6b5df305163ed67e2be2703ccb69948a8d7a0b82e1f04040000000f000012fea8ca5d00000000000104020000000000630004000100c91f0414000401000002286bee1301000002286bee00060107005c4d1f053a900d00ad041d0065feec15496516bcfde6b6b5df305163ed67e2be2703ccb69948a8d7a0b82e1f000d020c430005000000e9030000001c06aaa6ca5d000000000000000000001c06aaa6ca5d00000000000000000000420065feec15496516bcfde6b6b5df305163ed67e2be2703ccb69948a8d7a0b82e1f2800000000000000000000000000000000000000010c01204e0000011027000001e90300000a00000000b4c40400000000000000000000000000806e877401000000000000000000000000420065feec15496516bcfde6b6b5df305163ed67e2be2703ccb69948a8d7a0b82e1f2800000000000000000000000000000000000000010c01204e0000011027000001e90300001600000000b4c40400000000000000000000000000806e877401000000000000000000000000140d01020400010100506172656e7400000000000000000000000000000000000000000000000000000104c409000001c4090000b838000000630004000100c91f0414000401000002286bee1301000002286bee00060107005c4d1f0502000400ed011d0065feec15496516bcfde6b6b5df305163ed67e2be2703ccb69948a8d7a0b82e1f008904080a00000000f2052a0100000000000000000000001600000000f2052a0100000000000000000000000000000004010200a10f0100af3e7da28608e13e4399cc7d14a57bdb154dde5f3d546f5f293994ef36ef7f1100140d01020400010100506172656e740000000000000000000000000000000000000000000000000000").to_vec()),
		("referendum_1728", hex!("1a02141a0300016d6f646c70792f74727372790000000000000000000000000000000000000000630804000100c91f040001010065feec15496516bcfde6b6b5df305163ed67e2be2703ccb69948a8d7a0b82e1f04040000000f000012fea8ca5d00000000000104020000000000630004000100c91f0414000401000002286bee1301000002286bee00060107005c4d1f053a900d00ad041d0065feec15496516bcfde6b6b5df305163ed67e2be2703ccb69948a8d7a0b82e1f000d020c430005000000e9030000001c06aaa6ca5d000000000000000000001c06aaa6ca5d00000000000000000000420065feec15496516bcfde6b6b5df305163ed67e2be2703ccb69948a8d7a0b82e1f2800000000000000000000000000000000000000010c01204e0000011027000001e90300000a00000000b4c40400000000000000000000000000806e877401000000000000000000000000420065feec15496516bcfde6b6b5df305163ed67e2be2703ccb69948a8d7a0b82e1f2800000000000000000000000000000000000000010c01204e0000011027000001e90300001600000000b4c40400000000000000000000000000806e877401000000000000000000000000140d01020400010100506172656e7400000000000000000000000000000000000000000000000000000104c409000001c4090000b838000000630004000100c91f0414000401000002286bee1301000002286bee00060107005c4d1f0502000400ed011d0065feec15496516bcfde6b6b5df305163ed67e2be2703ccb69948a8d7a0b82e1f008904080a00000000f2052a0100000000000000000000001600000000f2052a0100000000000000000000000000000004010200a10f0100af3e7da28608e13e4399cc7d14a57bdb154dde5f3d546f5f293994ef36ef7f1100140d01020400010100506172656e7400000000000000000000000000000000000000000000000000001a0300016d6f646c70792f74727372790000000000000000000000000000000000000000630804000100c91f0400010100506172656e74000000000000000000000000000000000000000000000000000004040000000f0000c16ff2862300000000000104010000000000630004000100c91f041400040100000700e40b5402130100000700e40b540200060107005c4d1f0502000400ac430005000000e90300000000c16ff286230000000000000000000000c16ff2862300000000000000000000140d01020400010100506172656e740000000000000000000000000000000000000000000000000000").to_vec()),
		("referendum_1501", hex!("1a040c630004000100c91f041400040100000700e40b5402130100000700e40b540200060107005c4d1f053a900d00a5041d00471f92361b617ad6346d8993c5eb601b15704cd07c816efda5f5c32cbb095cf9000d02144201bc1c0000004201bd1c000000430405000000e90300000000c16ff28623000000000000000000004200471f92361b617ad6346d8993c5eb601b15704cd07c816efda5f5c32cbb095cf91400000000000000000000000000000000000000010c01204e0000011027000000e90300000a00000000d0ed902e0000000000000000000000a086010000000000000000000000000000004200471f92361b617ad6346d8993c5eb601b15704cd07c816efda5f5c32cbb095cf91400000000000000000000000000000000000000010c01204e0000011027000000e90300001600000000d0ed902e0000000000000000000000a08601000000000000000000000000000000140d01020400010100471f92361b617ad6346d8993c5eb601b15704cd07c816efda5f5c32cbb095cf91a020c1a0300016d6f646c70792f747273727900000000000000000000000000000000000000001a0208630804000100c91f0400010100c5b7975df05c06a272ddc9d80cefafbe02d27c4303c04a8fc07df98000b48ab804040000000f0000c52ebca2b10000000000630804000100c91f0400010100506172656e74000000000000000000000000000000000000000000000000000004040000000b00a0724e180900000000000104020000000000630004000100c91f0414000401000002286bee1301000002286bee00060107005c4d1f053a900d00ad041d00c5b7975df05c06a272ddc9d80cefafbe02d27c4303c04a8fc07df98000b48ab8000d020c430005000000e9030000001cb9dab9a2b1000000000000000000001cb9dab9a2b1000000000000000000004200c5b7975df05c06a272ddc9d80cefafbe02d27c4303c04a8fc07df98000b48ab81400000000000000000000000000000000000000010c01204e0000011027000000e90300000a00000000d0ed902e00000000000000000000008096980000000000000000000000000000004200c5b7975df05c06a272ddc9d80cefafbe02d27c4303c04a8fc07df98000b48ab81400000000000000000000000000000000000000010c01204e0000011027000000e90300001600000000d0ed902e0000000000000000000000809698000000000000000000000000000000140d01020400010100506172656e7400000000000000000000000000000000000000000000000000000104e803000001e80300006c6b000000630004000100c91f0414000401000002286bee1301000002286bee00060107005c4d1f0502000400ed011d00c5b7975df05c06a272ddc9d80cefafbe02d27c4303c04a8fc07df98000b48ab8008904080a00000000f2052a0100000000000000000000001600000000f2052a0100000000000000000000000000000003010200a10f0100af3e7da28608e13e4399cc7d14a57bdb154dde5f3d546f5f293994ef36ef7f1100140d01020400010100506172656e740000000000000000000000000000000000000000000000000000630004000100c91f041400040100000700e40b5402130100000700e40b540200060107005c4d1f053a900d00ed011d00471f92361b617ad6346d8993c5eb601b15704cd07c816efda5f5c32cbb095cf9008904080a0000000088526a740000000000000000000000160000000088526a7400000000000000000000000000000004010200a10f0100af3e7da28608e13e4399cc7d14a57bdb154dde5f3d546f5f293994ef36ef7f1100140d01020400010100471f92361b617ad6346d8993c5eb601b15704cd07c816efda5f5c32cbb095cf9").to_vec()),
	];

	new_test_ah_ext().execute_with(|| {
		for (referendum_id, rc_call) in calls {
			log::info!("mapping referendum: {referendum_id:?}");

			let rc_call = match BoundedInline::try_from(rc_call) {
				Ok(bounded) => Bounded::Inline(bounded),
				Err(unbounded) => {
					let len = unbounded.len() as u32;
					Bounded::Lookup {
						hash: pallet_preimage::Pallet::<AssetHub>::note(unbounded.into())
							.expect("failed to note call"),
						len,
					}
				},
			};

			let ah_call = pallet_ah_migrator::Pallet::<AssetHub>::map_rc_ah_call(&rc_call)
				.expect("failed to map call");

			let ah_call_encoded = pallet_ah_migrator::Pallet::<AssetHub>::fetch_preimage(&ah_call)
				.expect("failed to fetch preimage");

			let ah_call_decoded = AhRuntimeCall::decode(&mut ah_call_encoded.as_slice())
				.expect("failed to decode call");

			log::info!("mapped call: {ah_call:?}");
			log::debug!("encoded call: 0x{}", hex::encode(ah_call_encoded.as_slice()));
			log::debug!("decoded call: {ah_call_decoded:?}");
		}
	});
}

#[test]
fn rc_calls_and_origins_work() {
	use frame_support::traits::schedule::DispatchTime;
	type PalletBalances = pallet_balances::Pallet<Polkadot>;

	let mut ext = new_test_rc_ext();

	let manager: AccountId32 = [1; 32].into();
	let canceller: AccountId32 = [2; 32].into();
	let user: AccountId32 = [3; 32].into();
	let manager_wo_balance: AccountId32 = [4; 32].into();
	#[cfg(feature = "polkadot-ahm")]
	let admin_origin = pallet_xcm::Origin::Xcm(Location::new(
		0,
		[
			Junction::Parachain(1001),
			Junction::Plurality { id: BodyId::Technical, part: BodyPart::Voice },
		],
	));
	#[cfg(feature = "kusama-ahm")]
	let admin_origin = polkadot_runtime::governance::Origin::Fellows;

	ext.execute_with(|| {
		let ed = polkadot_runtime::ExistentialDeposit::get();
		PalletBalances::force_set_balance(RcRuntimeOrigin::root(), manager.clone().into(), ed + ed)
			.expect("failed to set balance");
		PalletBalances::force_set_balance(
			RcRuntimeOrigin::root(),
			canceller.clone().into(),
			ed + ed,
		)
		.expect("failed to set balance");
		PalletBalances::force_set_balance(RcRuntimeOrigin::root(), user.clone().into(), ed + ed)
			.expect("failed to set balance");
		PalletBalances::reserve(&user, ed).expect("failed to reserve");
	});

	ext.execute_with(|| {
		RcMigrator::preserve_accounts(
			RcRuntimeOrigin::root(),
			vec![manager.clone(), canceller.clone()],
		)
		.expect("failed to preserve accounts");
		RcMigrator::set_manager(admin_origin.clone().into(), Some(manager_wo_balance))
			.expect("failed to set manager");
		RcMigrator::set_manager(RcRuntimeOrigin::root(), Some(manager.clone()))
			.expect("failed to set manager");
		RcMigrator::set_manager(admin_origin.clone().into(), Some(manager.clone()))
			.expect("failed to set manager");

		RcMigrator::set_canceller(RcRuntimeOrigin::root(), Some(canceller.clone()))
			.expect("failed to set canceller");
		RcMigrator::set_canceller(admin_origin.clone().into(), Some(canceller.clone()))
			.expect("failed to set canceller");

		assert_noop!(
			RcMigrator::preserve_accounts(RcRuntimeOrigin::root(), vec![user.clone()],),
			pallet_rc_migrator::Error::<Polkadot>::AccountReferenced
		);
		assert_noop!(
			RcMigrator::set_canceller(RcRuntimeOrigin::root(), Some(user.clone())),
			pallet_rc_migrator::Error::<Polkadot>::AccountReferenced
		);
	});

	ext.execute_with(|| {
		RcMigrator::force_set_stage(
			admin_origin.clone().into(),
			Box::new(RcMigrationStage::MigrationPaused),
		)
		.expect("failed to force set stage");

		RcMigrator::force_set_stage(
			RcRuntimeOrigin::signed(manager.clone()),
			Box::new(RcMigrationStage::Pending),
		)
		.expect("failed to force set stage");

		let now = frame_system::Pallet::<Polkadot>::block_number();

		assert_noop!(
			RcMigrator::schedule_migration(
				RcRuntimeOrigin::signed(manager.clone()),
				DispatchTime::At(now + 1),
				DispatchTime::At(now + 2),
				DispatchTime::At(now + 3),
				false,
			),
			pallet_rc_migrator::Error::<Polkadot>::EraEndsTooSoon
		);

		let session_duration = polkadot_runtime::EpochDuration::get() as u32;
		let start = now + session_duration * 2 + 1;

		RcMigrator::schedule_migration(
			RcRuntimeOrigin::signed(manager.clone()),
			DispatchTime::At(start),
			DispatchTime::At(start + 1),
			DispatchTime::At(start + 2),
			false,
		)
		.expect("failed to schedule migration");

		RcMigrator::pause_migration(RcRuntimeOrigin::signed(manager.clone()))
			.expect("failed to pause migration");

		let current_stage = RcMigrationStageStorage::<Polkadot>::get();
		assert_eq!(current_stage, RcMigrationStage::MigrationPaused);

		RcMigrator::schedule_migration(
			RcRuntimeOrigin::signed(manager.clone()),
			DispatchTime::At(start),
			DispatchTime::At(start + 1),
			DispatchTime::At(start + 2),
			false,
		)
		.expect("failed to schedule migration");

		RcMigrator::cancel_migration(RcRuntimeOrigin::signed(canceller.clone()))
			.expect("failed to cancel migration");

		let current_stage = RcMigrationStageStorage::<Polkadot>::get();
		assert_eq!(current_stage, RcMigrationStage::Pending);

		RcMigrator::schedule_migration(
			RcRuntimeOrigin::signed(manager.clone()),
			DispatchTime::At(start),
			DispatchTime::At(start + 1),
			DispatchTime::At(start + 2),
			false,
		)
		.expect("failed to schedule migration");

		RcMigrator::force_set_stage(
			RcRuntimeOrigin::signed(manager.clone()),
			Box::new(RcMigrationStage::WaitingForAh),
		)
		.expect("failed to force set stage");

		RcMigrator::start_data_migration(RcRuntimeOrigin::signed(manager.clone()))
			.expect("failed to schedule migration");

		let current_stage = RcMigrationStageStorage::<Polkadot>::get();
		assert_eq!(current_stage, RcMigrationStage::WarmUp { end_at: start + 1 });
	});
}

#[test]
fn ah_calls_and_origins_work() {
	let mut ext = new_test_ah_ext();

	let manager: AccountId32 = [1; 32].into();
	#[cfg(feature = "polkadot-ahm")]
	let admin_origin = pallet_xcm::Origin::Xcm(Location::new(
		1,
		[
			Junction::Parachain(1001),
			Junction::Plurality { id: BodyId::Technical, part: BodyPart::Voice },
		],
	));
	#[cfg(feature = "kusama-ahm")]
	let admin_origin = pallet_xcm::Origin::Xcm(Location::new(
		1,
		[Junction::Plurality { id: BodyId::Technical, part: BodyPart::Voice }],
	));

	ext.execute_with(|| {
		AhMigrator::set_manager(AhRuntimeOrigin::root(), Some(manager.clone()))
			.expect("failed to set manager");
		AhMigrator::set_manager(admin_origin.clone().into(), Some(manager.clone()))
			.expect("failed to set manager");
	});

	ext.execute_with(|| {
		AhMigrator::force_set_stage(
			admin_origin.clone().into(),
			AhMigrationStage::DataMigrationOngoing,
		)
		.expect("failed to force set stage");

		let current_stage = AhMigrationStageStorage::<AssetHub>::get();
		assert_eq!(current_stage, AhMigrationStage::DataMigrationOngoing);

		AhMigrator::force_set_stage(AhRuntimeOrigin::root(), AhMigrationStage::Pending)
			.expect("failed to force set stage");

		let current_stage = AhMigrationStageStorage::<AssetHub>::get();
		assert_eq!(current_stage, AhMigrationStage::Pending);

		AhMigrator::force_set_stage(
			AhRuntimeOrigin::signed(manager.clone()),
			AhMigrationStage::DataMigrationOngoing,
		)
		.expect("failed to force set stage");

		let current_stage = AhMigrationStageStorage::<AssetHub>::get();
		assert_eq!(current_stage, AhMigrationStage::DataMigrationOngoing);
	});
}

#[tokio::test]
async fn low_balance_accounts_migration_works() {
	use frame_system::Account as SystemAccount;
	use pallet_rc_migrator::accounts::AccountsMigrator;

	type PalletBalances = pallet_balances::Pallet<Polkadot>;

	let mut rc = new_test_rc_ext();
	let mut ah = new_test_ah_ext();

	let ed = polkadot_runtime::ExistentialDeposit::get();
	let ah_ed = asset_hub_polkadot_runtime::ExistentialDeposit::get();
	assert!(ed > ah_ed);

	// user with RC ED
	let user: AccountId32 = [0; 32].into();
	// user with AH ED
	let user1: AccountId32 = [1; 32].into();
	// user with AH ED and reserve
	let user2: AccountId32 = [2; 32].into();
	// user with AH ED and freeze
	let user3: AccountId32 = [3; 32].into();
	rc.execute_with(|| {
		PalletBalances::force_set_balance(RcRuntimeOrigin::root(), user.clone().into(), ed + 1)
			.expect("failed to set balance for `user`");

		PalletBalances::force_set_balance(RcRuntimeOrigin::root(), user1.clone().into(), ed + 1)
			.expect("failed to set balance for `user1`");
		frame_system::Account::<Polkadot>::mutate(&user1, |account| {
			account.data.free = ah_ed + 1;
		});

		PalletBalances::force_set_balance(RcRuntimeOrigin::root(), user2.clone().into(), ed + 1)
			.expect("failed to set balance for `user2`");
		frame_system::Account::<Polkadot>::mutate(&user2, |account| {
			account.data.free = ah_ed + 1;
			account.data.reserved = 1;
		});

		PalletBalances::force_set_balance(RcRuntimeOrigin::root(), user3.clone().into(), ed + 1)
			.expect("failed to set balance for `user3`");
		frame_system::Account::<Polkadot>::mutate(&user3, |account| {
			account.data.free = ah_ed + 1;
			account.data.frozen = 1;
		});

		pallet_rc_migrator::RcMigratedBalance::<Polkadot>::mutate(|tracker| {
			tracker.kept = ed * 10000;
			tracker.migrated = 0;
		});
	});

	let accounts: Vec<(&str, AccountId32, bool)> = vec![
		// (case name, account_id, should_be_migrated)
		("user_with_rc_ed", user, true),
		("user_with_ah_ed", user1, true),
		("user_with_ah_ed_and_reserve", user2, false),
		("user_with_ah_ed_and_freeze", user3, false),
	];

	for (case, account_id, should_be_migrated) in accounts {
		let (maybe_withdrawn_account, removed) = rc.execute_with(|| {
			let rc_account = SystemAccount::<Polkadot>::get(&account_id);
			log::info!("Case: {case:?}");
			log::info!("RC account info: {rc_account:?}");

			let maybe_withdrawn_account = AccountsMigrator::<Polkadot>::withdraw_account(
				account_id.clone(),
				rc_account,
				&mut WeightMeter::new(),
				0,
			)
			.unwrap_or_else(|err| {
				log::error!("Account withdrawal failed: {err:?}");
				None
			});

			(maybe_withdrawn_account, !SystemAccount::<Polkadot>::contains_key(&account_id))
		});

		let withdrawn_account = match maybe_withdrawn_account {
			Some(withdrawn_account) => {
				assert!(should_be_migrated);
				assert!(removed);
				withdrawn_account
			},
			None => {
				assert!(!should_be_migrated);
				assert!(!removed);
				log::warn!("Account is not withdrawable");
				continue;
			},
		};

		log::info!("Withdrawn account: {withdrawn_account:?}");

		ah.execute_with(|| {
			use codec::{Decode, Encode};

			let encoded_account = withdrawn_account.encode();
			let account = Decode::decode(&mut &encoded_account[..]).unwrap();
			let res = AhMigrator::do_receive_account(account);
			assert!(res.is_ok());
			log::info!("Account integration result: {res:?}");
		});
	}
}
