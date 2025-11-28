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

//! Runtime API for fetching info about the Asset Hub migration.

#![cfg_attr(not(feature = "std"), no_std)]

sp_api::decl_runtime_apis! {
	/// API to query information about the Asset Hub migration process.
	pub trait AssetHubMigrationApi<BlockNumber> where BlockNumber: sp_runtime::traits::BlockNumber {
		/// Returns the block number when the migration started.
		fn migration_start_block() -> BlockNumber;

		/// Returns the block number when the migration ended.
		fn migration_end_block() -> BlockNumber;
	}
}
