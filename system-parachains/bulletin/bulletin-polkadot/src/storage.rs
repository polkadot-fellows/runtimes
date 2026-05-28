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

use crate::DAYS;
use super::{xcm_config::PeopleLocation, Runtime, RuntimeCall, RuntimeEvent, RuntimeHoldReason};
use alloc::vec::Vec;
use bulletin_pallets_common::inspect_utility_wrapper;
use frame_support::{
	parameter_types,
	traits::{Contains, EitherOf, Equals},
};
use pallet_bulletin_transaction_storage::{
	AsAuthorizer, CallInspector, EnsureAllowedAuthorizers, DEFAULT_MAX_BLOCK_TRANSACTIONS,
	DEFAULT_MAX_TRANSACTION_SIZE,
};
use pallet_xcm::EnsureXcm;
use sp_runtime::transaction_validity::{TransactionLongevity, TransactionPriority};

parameter_types! {
	/// Cap on the total bytes committed to permanent storage (via `renew`) across all
	/// authorizations on this chain. Seeded at 1.7 TiB; storage-backed so governance
	/// (root) can raise/lower it via `system.set_storage` without a runtime upgrade.
	pub storage MaxPermanentStorageSize: u64 = 17 * 1024 * 1024 * 1024 * 1024 / 10;
}

parameter_types! {
	// TODO: @bkontur @franciscoaguirre @karolk91 confirm
	pub const AuthorizationPeriod: crate::BlockNumber = 14 * DAYS;
	// Priorities and longevities used by the transaction storage pallet extrinsics.
	//
	// `RemoveExpiredAuthorization` (permissionless cleanup) sits at the top so it always
	// runs before stores compete for blockspace.
	pub const RemoveExpiredAuthorizationPriority: TransactionPriority = TransactionPriority::MAX;
	pub const RemoveExpiredAuthorizationLongevity: TransactionLongevity = crate::DAYS as TransactionLongevity;
	// Base priority for `store` / `renew`. Picked well below `TransactionPriority::MAX` so
	// `AllowanceBasedPriority` can add its boost without saturating `u64`, while still
	// leaving plenty of headroom above generic transactions.
	pub const StoreRenewPriority: TransactionPriority = TransactionPriority::MAX / 4;
	pub const StoreRenewLongevity: TransactionLongevity = DAYS as TransactionLongevity;
}

/// Tells [`pallet_bulletin_transaction_storage::extension::ValidateStorageCalls`] how to find
/// storage calls inside wrapper extrinsics so it can recursively validate and consume
/// authorization.
///
/// Also implements [`Contains<RuntimeCall>`] returning `true` for storage-mutating calls
/// (store, store_with_cid_config, renew). Used with `EverythingBut` as the XCM
/// `SafeCallFilter` to block these calls from XCM dispatch — they require on-chain
/// authorization that XCM cannot provide.
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

/// Returns `true` for storage-mutating TransactionStorage calls (store, store_with_cid_config,
/// renew). Recursively inspects wrapper calls (Utility) to prevent bypass via nesting.
/// Used with `EverythingBut` as the XCM `SafeCallFilter`.
impl Contains<RuntimeCall> for StorageCallInspector {
	fn contains(call: &RuntimeCall) -> bool {
		Self::is_storage_mutating_call(call, 0)
	}
}

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
	type MaxPermanentStorageSize = MaxPermanentStorageSize;
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
			AsAuthorizer<
				EnsureXcm<Equals<PeopleLocation>>,
				Self::AccountId,
				crate::BlockNumber,
			>,
		>,
		// Accounts registered in `AllowedAuthorizers` storage (managed via
		// `add_authorizer` / `remove_authorizer`).
		EnsureAllowedAuthorizers<Runtime>,
	>;
	type StoreRenewPriority = StoreRenewPriority;
	type StoreRenewLongevity = StoreRenewLongevity;
	type RemoveExpiredAuthorizationPriority = RemoveExpiredAuthorizationPriority;
	type RemoveExpiredAuthorizationLongevity = RemoveExpiredAuthorizationLongevity;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = pallet_bulletin_transaction_storage::benchmarking::DefaultCheckProofHelper;
}

parameter_types! {
	/// Maximum allowable skew between the user's submit timestamp and the on-chain
	/// time when validating a HOP promotion: 48 hours, in milliseconds.
	pub const SubmitTimestampTolerance: u64 = 48 * 60 * 60 * 1000;
}

impl pallet_bulletin_hop_promotion::Config for Runtime {
	type SubmitTimestampTolerance = SubmitTimestampTolerance;
	type WeightInfo = crate::weights::pallet_bulletin_hop_promotion::WeightInfo<Runtime>;
}
