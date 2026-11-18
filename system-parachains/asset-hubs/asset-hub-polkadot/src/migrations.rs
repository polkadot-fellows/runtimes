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

//! The runtime migrations per release.
use crate::Runtime;

/// Unreleased migrations. Add new ones here:
pub type Unreleased = (
	// EPMB signed: store the base deposit per submission, fixed at registration
	// (polkadot-sdk#13722).
	pallet_election_provider_multi_block::signed::migrations::v1::MigrateV0ToV1<Runtime>,
);

/// All single block migrations that will run on the next runtime upgrade.
pub type SingleBlockMigrations = Unreleased;

/// MBM migrations to apply on runtime upgrade.
pub type MbmMigrations = ();
