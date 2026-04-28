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

#![allow(deprecated, missing_docs)]

use super::*;

/// Unreleased migrations. Add new ones here:
pub type Unreleased = (
	// Initialize TransactionStorage retention period on first upgrade.
	pallet_bulletin_transaction_storage::migrations::SetRetentionPeriodIfZero<
		Runtime,
		pallet_bulletin_transaction_storage::DefaultRetentionPeriod,
	>,
	// Migrate TransactionInfo from v0 to v1 (adds hashing and cid_codec fields).
	pallet_bulletin_transaction_storage::migrations::v1::MigrateV0ToV1<Runtime>,
);

/// Migrations/checks that do not need to be versioned and can run on every update.
pub type Permanent = (pallet_xcm::migration::MigrateToLatestXcmVersion<Runtime>,);

/// All single block migrations that will run on the next runtime upgrade.
pub type SingleBlockMigrations = (Unreleased, Permanent);

/// MBM migrations to apply on runtime upgrade.
pub type MbmMigrations = ();
