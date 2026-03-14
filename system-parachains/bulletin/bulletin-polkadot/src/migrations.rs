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
	pallet_collator_selection::migration::v2::MigrationToV2<Runtime>,
	cumulus_pallet_xcmp_queue::migration::v4::MigrationToV4<Runtime>,
	cumulus_pallet_xcmp_queue::migration::v5::MigrateV4ToV5<Runtime>,
	pallet_session::migrations::v1::MigrateV0ToV1<
		Runtime,
		pallet_session::migrations::v1::InitOffenceSeverity<Runtime>,
	>,
	cumulus_pallet_aura_ext::migration::MigrateV0ToV1<Runtime>,
);

/// Migrations/checks that do not need to be versioned and can run on every update.
pub type Permanent = (pallet_xcm::migration::MigrateToLatestXcmVersion<Runtime>,);

/// All single block migrations that will run on the next runtime upgrade.
pub type SingleBlockMigrations = (Unreleased, Permanent);

/// MBM migrations to apply on runtime upgrade.
pub type MbmMigrations = ();
