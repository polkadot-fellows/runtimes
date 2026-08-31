// Copyright (C) Parity Technologies and the various Polkadot contributors, see Contributions.md
// for a list of specific contributors.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Governance-mutable parameter tests.
//!
//! People Polkadot keeps the notification period, the long-term-storage period and its grace
//! window as compile-time constants (they are dynamic parameters on the Paseo testnet runtime),
//! and it deploys no people-airdrops pallet, so those parts of the original test have no
//! equivalent here.

use super::*;
use crate::parameters::{
	dynamic_params::{bulletin_storage, statement_storage},
	RuntimeParameters, LONG_TERM_STORAGE_CLEANUP_LIMIT_CAP, STMT_STORE_CLEANUP_LIMIT_CAP,
};
use frame_support::{assert_noop, assert_ok};
use indiv_pallet_resources::WeightInfo as ResourcesWeightInfo;
use indiv_support::{parameters::StatementAllowanceParameter, weight_budget::OcwWeightBudget};
use sp_runtime::DispatchError;
use sp_statement_store::StatementAllowance;

type ResourcesWeights = <Runtime as indiv_pallet_resources::Config>::WeightInfo;

fn set_parameter(parameter: RuntimeParameters) {
	assert_ok!(Parameters::set_parameter(RuntimeOrigin::root(), parameter));
}

/// Stores `value` for a statement-storage parameter.
fn set_statement_parameter<Key, Value>(key: Key, value: Value)
where
	statement_storage::Parameters: From<(Key, Value)>,
{
	set_parameter(RuntimeParameters::StatementStorage((key, value).into()));
}

/// Stores `value` for a long-term storage parameter.
fn set_bulletin_parameter<Key, Value>(key: Key, value: Value)
where
	bulletin_storage::Parameters: From<(Key, Value)>,
{
	set_parameter(RuntimeParameters::BulletinStorage((key, value).into()));
}

#[test]
fn parameters_default_to_the_initial_configuration() {
	new_test_ext().execute_with(|| {
		assert_eq!(
			<Runtime as indiv_pallet_resources::Config>::StmtStoreGraceWindow::get(),
			24 * 60 * 60
		);
		assert_eq!(
			<Runtime as indiv_pallet_resources::Config>::PersonStatementLimit::get(),
			StatementAllowance { max_size: 100 * 1024, max_count: 30 }
		);
		assert_eq!(
			<<Runtime as indiv_pallet_resources::Config>::NotificationPeriodDuration as Get<u32>>::get(),
			24 * 60 * 60
		);
	});
}

#[test]
fn signed_origins_cannot_set_parameters() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Parameters::set_parameter(
				RuntimeOrigin::signed(Sr25519Keyring::Alice.to_account_id()),
				RuntimeParameters::StatementStorage(
					(statement_storage::StmtStoreGraceWindow, 60 * 60u32).into()
				)
			),
			DispatchError::BadOrigin
		);
	});
}

#[test]
fn root_parameter_updates_change_what_the_resources_pallet_reads() {
	new_test_ext().execute_with(|| {
		set_statement_parameter(statement_storage::StmtStoreGraceWindow, 60 * 60);
		assert_eq!(
			<Runtime as indiv_pallet_resources::Config>::StmtStoreGraceWindow::get(),
			60 * 60
		);

		set_statement_parameter(
			statement_storage::PersonStatementLimit,
			StatementAllowanceParameter { max_size: 42, max_count: 3 },
		);
		assert_eq!(
			<Runtime as indiv_pallet_resources::Config>::PersonStatementLimit::get(),
			StatementAllowance { max_size: 42, max_count: 3 }
		);
	});
}

#[test]
fn governance_cannot_break_statement_storage_invariants() {
	new_test_ext().execute_with(|| {
		// Slot limits stay non-zero.
		set_statement_parameter(statement_storage::StmtStoreSlotsPerPeriod, 0);
		assert_eq!(<Runtime as indiv_pallet_resources::Config>::StmtStoreSlotsPerPeriod::get(), 1);
		set_statement_parameter(statement_storage::StmtStoreSlotsPerPeriod, 20);
		assert_eq!(<Runtime as indiv_pallet_resources::Config>::StmtStoreSlotsPerPeriod::get(), 20);

		// The lite slot limit stays non-zero and never exceeds the full limit.
		set_statement_parameter(statement_storage::LiteStmtStoreSlotsPerPeriod, 0);
		assert_eq!(
			<Runtime as indiv_pallet_resources::Config>::LiteStmtStoreSlotsPerPeriod::get(),
			1
		);
		set_statement_parameter(statement_storage::LiteStmtStoreSlotsPerPeriod, u32::MAX);
		assert_eq!(
			<Runtime as indiv_pallet_resources::Config>::LiteStmtStoreSlotsPerPeriod::get(),
			20
		);
		set_statement_parameter(statement_storage::LiteStmtStoreSlotsPerPeriod, 10);
		assert_eq!(
			<Runtime as indiv_pallet_resources::Config>::LiteStmtStoreSlotsPerPeriod::get(),
			10
		);

		// The replacement cooldown stays non-zero and never exceeds the one day period.
		set_statement_parameter(statement_storage::StmtStoreReplacementCooldown, 0);
		assert_eq!(
			<Runtime as indiv_pallet_resources::Config>::StmtStoreReplacementCooldown::get(),
			1
		);
		set_statement_parameter(statement_storage::StmtStoreReplacementCooldown, u32::MAX);
		assert_eq!(
			<Runtime as indiv_pallet_resources::Config>::StmtStoreReplacementCooldown::get(),
			24 * 60 * 60
		);
		set_statement_parameter(statement_storage::StmtStoreReplacementCooldown, 60);
		assert_eq!(
			<Runtime as indiv_pallet_resources::Config>::StmtStoreReplacementCooldown::get(),
			60
		);

		// The grace window stays non-zero.
		set_statement_parameter(statement_storage::StmtStoreGraceWindow, 0);
		assert_eq!(<Runtime as indiv_pallet_resources::Config>::StmtStoreGraceWindow::get(), 1);
		set_statement_parameter(statement_storage::StmtStoreGraceWindow, 60 * 60);
		assert_eq!(
			<Runtime as indiv_pallet_resources::Config>::StmtStoreGraceWindow::get(),
			60 * 60
		);

		// Zero is valid, leaving the single slot `0`, and the lite limit never exceeds the full
		// one.
		set_statement_parameter(statement_storage::NotificationSlotsPerPeriod, 0);
		assert_eq!(
			<Runtime as indiv_pallet_resources::Config>::NotificationSlotsPerPeriod::get(),
			0
		);
		set_statement_parameter(statement_storage::LiteNotificationSlotsPerPeriod, u8::MAX);
		assert_eq!(
			<Runtime as indiv_pallet_resources::Config>::LiteNotificationSlotsPerPeriod::get(),
			0
		);
		set_statement_parameter(statement_storage::NotificationSlotsPerPeriod, 8);
		assert_eq!(
			<Runtime as indiv_pallet_resources::Config>::NotificationSlotsPerPeriod::get(),
			8
		);
		set_statement_parameter(statement_storage::LiteNotificationSlotsPerPeriod, 0);
		assert_eq!(
			<Runtime as indiv_pallet_resources::Config>::LiteNotificationSlotsPerPeriod::get(),
			0
		);
		set_statement_parameter(statement_storage::LiteNotificationSlotsPerPeriod, u8::MAX);
		assert_eq!(
			<Runtime as indiv_pallet_resources::Config>::LiteNotificationSlotsPerPeriod::get(),
			8
		);
		set_statement_parameter(statement_storage::LiteNotificationSlotsPerPeriod, 4);
		assert_eq!(
			<Runtime as indiv_pallet_resources::Config>::LiteNotificationSlotsPerPeriod::get(),
			4
		);
	});
}

#[test]
fn governance_cannot_break_bulletin_storage_invariants() {
	new_test_ext().execute_with(|| {
		// The claim limit stays non-zero.
		set_bulletin_parameter(bulletin_storage::LongTermStorageClaimsPerPeriod, 0u8);
		assert_eq!(
			<Runtime as indiv_pallet_resources::Config>::LongTermStorageClaimsPerPeriod::get(),
			1
		);
		set_bulletin_parameter(bulletin_storage::LongTermStorageClaimsPerPeriod, 10u8);
		assert_eq!(
			<Runtime as indiv_pallet_resources::Config>::LongTermStorageClaimsPerPeriod::get(),
			10
		);
	});
}

/// Benchmark builds pin the cleanup limits to their caps (see
/// [`cleanup_limits_pin_to_the_benchmarked_caps_in_benchmark_builds`]), so the clamp behaviour is
/// only observable in production builds.
#[cfg(not(feature = "runtime-benchmarks"))]
#[test]
fn governance_cannot_move_cleanup_limits_outside_the_benchmarked_range() {
	new_test_ext().execute_with(|| {
		set_statement_parameter(statement_storage::StmtStoreCleanupLimit, 0);
		assert_eq!(<Runtime as indiv_pallet_resources::Config>::StmtStoreCleanupLimit::get(), 1);
		set_statement_parameter(statement_storage::StmtStoreCleanupLimit, u32::MAX);
		assert_eq!(
			<Runtime as indiv_pallet_resources::Config>::StmtStoreCleanupLimit::get(),
			STMT_STORE_CLEANUP_LIMIT_CAP
		);
		set_statement_parameter(statement_storage::StmtStoreCleanupLimit, 25);
		assert_eq!(<Runtime as indiv_pallet_resources::Config>::StmtStoreCleanupLimit::get(), 25);

		set_bulletin_parameter(bulletin_storage::LongTermStorageCleanupLimit, 0);
		assert_eq!(
			<Runtime as indiv_pallet_resources::Config>::LongTermStorageCleanupLimit::get(),
			1
		);
		set_bulletin_parameter(bulletin_storage::LongTermStorageCleanupLimit, u32::MAX);
		assert_eq!(
			<Runtime as indiv_pallet_resources::Config>::LongTermStorageCleanupLimit::get(),
			LONG_TERM_STORAGE_CLEANUP_LIMIT_CAP
		);
		set_bulletin_parameter(bulletin_storage::LongTermStorageCleanupLimit, 10);
		assert_eq!(
			<Runtime as indiv_pallet_resources::Config>::LongTermStorageCleanupLimit::get(),
			10
		);
	});
}

/// Benchmarks must sweep and budget-check the largest value reachable in production, so benchmark
/// builds read the cleanup limits as their caps regardless of the stored value.
#[cfg(feature = "runtime-benchmarks")]
#[test]
fn cleanup_limits_pin_to_the_benchmarked_caps_in_benchmark_builds() {
	new_test_ext().execute_with(|| {
		set_statement_parameter(statement_storage::StmtStoreCleanupLimit, 1);
		assert_eq!(
			<Runtime as indiv_pallet_resources::Config>::StmtStoreCleanupLimit::get(),
			STMT_STORE_CLEANUP_LIMIT_CAP
		);

		set_bulletin_parameter(bulletin_storage::LongTermStorageCleanupLimit, 1);
		assert_eq!(
			<Runtime as indiv_pallet_resources::Config>::LongTermStorageCleanupLimit::get(),
			LONG_TERM_STORAGE_CLEANUP_LIMIT_CAP
		);
	});
}

#[test]
fn cleanup_caps_fit_the_ocw_weight_budget() {
	new_test_ext().execute_with(|| {
		let budget = OcwWeightBudget::from_normal_max::<Runtime>();
		budget.assert_fits(
			"clear_expired_stmt_store_allowances",
			ResourcesWeights::clear_expired_stmt_store_allowances(STMT_STORE_CLEANUP_LIMIT_CAP)
				.saturating_add(ResourcesWeights::authorize_clear_expired_stmt_store_allowances()),
		);
		budget.assert_fits(
			"clear_expired_long_term_storage_aliases",
			ResourcesWeights::clear_expired_long_term_storage_aliases(
				LONG_TERM_STORAGE_CLEANUP_LIMIT_CAP,
			)
			.saturating_add(ResourcesWeights::authorize_clear_expired_long_term_storage_aliases()),
		);
	});
}

#[test]
fn subscriber_sent_calls_fit_the_xcm_message_budget() {
	use indiv_pallet_members_notifier::WeightInfo as NotifierWeightInfo;

	type NotifierWeights = <Runtime as indiv_pallet_members_notifier::Config>::WeightInfo;

	new_test_ext().execute_with(|| {
		let budget = crate::MessageQueueServiceWeight::get();

		let unsubscribe = NotifierWeights::unsubscribe();
		assert!(
			unsubscribe.all_lte(budget),
			"`unsubscribe` worst-case weight {unsubscribe:?} exceeds the XCM message budget {budget:?}",
		);

		let max_indices =
			<Runtime as indiv_pallet_members_notifier::Config>::MaxUpdatesPerBatch::get();
		let request_replay = NotifierWeights::request_replay(max_indices).saturating_add(
			<Runtime as indiv_pallet_members_notifier::Config>::RequestReplayRemoteWeight::get()
				.saturating_mul(max_indices.into()),
		);
		assert!(
			request_replay.all_lte(budget),
			"`request_replay` worst-case weight {request_replay:?} exceeds the XCM message budget {budget:?}",
		);
	});
}
