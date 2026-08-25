// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Cumulus.
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

//! Storage-specific configurations.

use super::{xcm_config::PeopleLocation, Runtime, RuntimeCall, RuntimeEvent, RuntimeHoldReason};
use crate::DAYS;
use alloc::vec::Vec;
use bulletin_pallets_common::inspect_utility_wrapper;
use frame_support::{
	parameter_types,
	traits::{Contains, EitherOf, Equals},
};
use pallet_bulletin_transaction_storage::{
	AsAuthorizer, CallInspector, EnsureAllowedAuthorizers, ValidTransactionParams,
	DEFAULT_MAX_BLOCK_TRANSACTIONS, DEFAULT_MAX_TRANSACTION_SIZE, MAX_WRAPPER_DEPTH,
};
use pallet_xcm::EnsureXcm;
use sp_runtime::transaction_validity::{TransactionLongevity, TransactionPriority};

parameter_types! {
	/// Cap on the total bytes committed to permanent storage (via `renew`) across all
	/// authorizations on this chain. Seeded at 1.7 TiB; storage-backed so governance
	/// (root) can raise/lower it via `system.set_storage` without a runtime upgrade.
	pub storage MaxPermanentStorageSize: u64 = 17 * 1024 * 1024 * 1024 * 1024 / 10;
}

// Permissionless cleanup sits at the top so it always runs before stores compete for
// blockspace.
const CLEANUP_PRIORITY: TransactionPriority = TransactionPriority::MAX;
// Base priority for `store`. Picked well below `TransactionPriority::MAX` so
// `AllowanceBasedPriority` can add its boost without saturating `u64`, while still
// leaving plenty of headroom above generic transactions.
const STORE_PRIORITY: TransactionPriority = TransactionPriority::MAX / 4;
const TX_LONGEVITY: TransactionLongevity = DAYS as TransactionLongevity;

parameter_types! {
	pub const AuthorizationPeriod: crate::BlockNumber = 14 * DAYS;
	// Pool params per call family. The tag prefixes must stay pairwise distinct — the
	// pallet's `integrity_test` asserts it — so families never dedup each other out of
	// the pool.
	pub const StoreTxParams: ValidTransactionParams =
		ValidTransactionParams::new("TransactionStorageStore", STORE_PRIORITY, TX_LONGEVITY);
	pub const RenewTxParams: ValidTransactionParams =
		ValidTransactionParams::new("TransactionStorageRenew", STORE_PRIORITY, TX_LONGEVITY);
	pub const AuthorizeTxParams: ValidTransactionParams =
		ValidTransactionParams::new("TransactionStorageAuthorize", STORE_PRIORITY, TX_LONGEVITY);
	pub const RemoveExpiredAccountAuthorizationTxParams: ValidTransactionParams =
		ValidTransactionParams::new(
			"TransactionStorageRemoveExpiredAccountAuthorization",
			CLEANUP_PRIORITY,
			TX_LONGEVITY,
		);
	pub const RemoveExpiredPreimageAuthorizationTxParams: ValidTransactionParams =
		ValidTransactionParams::new(
			"TransactionStorageRemoveExpiredPreimageAuthorization",
			CLEANUP_PRIORITY,
			TX_LONGEVITY,
		);
	pub const RemoveExhaustedAuthorizerTxParams: ValidTransactionParams =
		ValidTransactionParams::new(
			"TransactionStorageRemoveExhaustedAuthorizer",
			CLEANUP_PRIORITY,
			TX_LONGEVITY,
		);
}

/// Tells [`pallet_bulletin_transaction_storage::extension::ValidateAuthorizedCalls`] how to find
/// storage calls inside wrapper extrinsics so it can recursively validate and consume
/// authorization.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct StorageCallInspector;

impl pallet_bulletin_transaction_storage::CallInspector<Runtime> for StorageCallInspector {
	fn inspect_wrapper(call: &RuntimeCall) -> Option<Vec<&RuntimeCall>> {
		match call {
			RuntimeCall::Utility(c) => inspect_utility_wrapper(c),
			// Root origin (e.g., relayed governance) can store data via the
			// underlying pallet without authorization, as Root is accepted by
			// `ensure_authorized`. Sudo is not present on the Polkadot Bulletin chain.
			_ => None,
		}
	}
}

/// XCM `SafeCallFilter` (via `EverythingBut`): `true` for calls that commit data — stores plus
/// renewals — including inside `Utility` wrappers.
impl Contains<RuntimeCall> for StorageCallInspector {
	fn contains(call: &RuntimeCall) -> bool {
		Self::is_storage_mutating_call(call, 0) || Self::is_renewal_committing_call(call, 0)
	}
}

impl StorageCallInspector {
	/// Renewal counterpart to [`CallInspector::is_storage_mutating_call`], which only knows the
	/// storage pallet's calls. `ensure_authorized` accepts Root and `LocationAsSuperuser` hands
	/// Root to Relay/Asset Hub `Transact`, so an XCM `force_renew` would otherwise commit
	/// permanent bytes for free. An allowlist, so a call added by a future bump is blocked.
	// TODO(upstream): drop once the pallets expose a composable committing-call predicate.
	fn is_renewal_committing_call(call: &RuntimeCall, depth: u32) -> bool {
		use pallet_bulletin_data_renewal::Call as RenewalCall;
		if let RuntimeCall::DataRenewal(inner) = call {
			return !matches!(
				inner,
				// Releases a registration rather than committing one; Root needs it for cleanup.
				RenewalCall::disable_auto_renew { .. } |
					// Mandatory inherent — `ensure_none` rejects any `Transact` origin anyway.
					RenewalCall::process_pending_renewals { .. }
			);
		}
		<Self as CallInspector<Runtime>>::inspect_wrapper(call).is_some_and(|inner_calls| {
			// Same fail-safe as the storage-call walk: a wrapper too deep to inspect counts as
			// committing.
			depth >= MAX_WRAPPER_DEPTH ||
				inner_calls
					.into_iter()
					.any(|inner| Self::is_renewal_committing_call(inner, depth + 1))
		})
	}
}

/// Both pallets' authorization-gated calls: each leaf is offered to `StorageLeaves`, then
/// `RenewalLeaves`.
pub type ValidateBulletinCalls =
	pallet_bulletin_transaction_storage::extension::ValidateAuthorizedCalls<
		Runtime,
		StorageCallInspector,
		(
			pallet_bulletin_transaction_storage::extension::StorageLeaves<Runtime>,
			pallet_bulletin_data_renewal::extension::RenewalLeaves<Runtime>,
		),
	>;

/// Priority boost for in-allowance stores.
pub type StoragePriorityBoost =
	pallet_bulletin_transaction_storage::extension::AllowanceBasedPriority<
		Runtime,
		pallet_bulletin_transaction_storage::extension::FlatBoost,
	>;

/// The main business of the Bulletin chain.
impl pallet_bulletin_transaction_storage::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = bulletin_pallets_common::NoCurrency<Self::AccountId, RuntimeHoldReason>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type FeeDestination = ();
	type WeightInfo = crate::weights::pallet_bulletin_transaction_storage::WeightInfo<Runtime>;
	type MaxBlockTransactions = crate::ConstU32<{ DEFAULT_MAX_BLOCK_TRANSACTIONS }>;
	type MaxTransactionSize = crate::ConstU32<{ DEFAULT_MAX_TRANSACTION_SIZE }>;
	type AuthorizationPeriod = AuthorizationPeriod;
	type AuthorizerRegistrarOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type Authorizer = EitherOf<
		EitherOf<
			// Root can do whatever.
			AsAuthorizer<
				frame_system::EnsureRoot<Self::AccountId>,
				Self::AccountId,
				crate::BlockNumber,
			>,
			// The People Chain can authorize for storage allowances.
			AsAuthorizer<EnsureXcm<Equals<PeopleLocation>>, Self::AccountId, crate::BlockNumber>,
		>,
		// Accounts registered in `AllowedAuthorizers` storage (managed via
		// `add_authorizer` / `remove_authorizer`).
		EnsureAllowedAuthorizers<Runtime>,
	>;
	type StoreTxParams = StoreTxParams;
	type AuthorizeTxParams = AuthorizeTxParams;
	type RemoveExpiredAccountAuthorizationTxParams = RemoveExpiredAccountAuthorizationTxParams;
	type RemoveExpiredPreimageAuthorizationTxParams = RemoveExpiredPreimageAuthorizationTxParams;
	type RemoveExhaustedAuthorizerTxParams = RemoveExhaustedAuthorizerTxParams;
	type EntryMeta = bulletin_transaction_storage_primitives::EntryKind;
	type AuthorizationExtra = pallet_bulletin_data_renewal::PermanentExtent;
	type OnObsoleteTransactions = crate::DataRenewal;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = pallet_bulletin_data_renewal::RenewalBenchmarkHelper;
}

impl pallet_bulletin_data_renewal::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = crate::weights::pallet_bulletin_data_renewal::WeightInfo<Runtime>;
	type MaxPermanentStorageSize = MaxPermanentStorageSize;
	type RenewTxParams = RenewTxParams;
}

parameter_types! {
	/// Maximum allowable skew between the user's submit timestamp and the on-chain
	/// time when validating a HOP promotion: 48 hours, in milliseconds.
	pub const SubmitTimestampTolerance: u64 = 48 * 60 * 60 * 1000;
	// Lowest priority: promotion only fills blockspace stores would not have used.
	pub const PromoteTxParams: ValidTransactionParams =
		ValidTransactionParams::new("HopPromotion", 0, 5);
}

impl pallet_bulletin_hop_promotion::Config for Runtime {
	type SubmitTimestampTolerance = SubmitTimestampTolerance;
	type PromoteTxParams = PromoteTxParams;
	type WeightInfo = crate::weights::pallet_bulletin_hop_promotion::WeightInfo<Runtime>;
}
