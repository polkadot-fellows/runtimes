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

//! Tests for the runtime's bootstrap migrations.
//!
//! People Polkadot bootstraps its Individuality state through runtime migrations rather than
//! genesis flags (the shipped genesis presets carry no collection-creation switches), so the
//! original genesis-preset tests have no equivalent here. The chunk-page-hash specific tests
//! live next to the migration, in `crate::migrations::chunk_page_hashes`.

use crate::{Runtime, System};
use cumulus_primitives_core::ParaId;
use frame_support::traits::OnRuntimeUpgrade;
use indiv_pallet_chunks_manager::ChunkPageHashes;
use indiv_support::traits::RingExponent;
use polkadot_runtime_constants::system_parachain::ASSET_HUB_ID;
use sp_io::TestExternalities;
use sp_runtime::BuildStorage;

/// Externalities from default genesis: no chunk page hashes, no collections. The shared
/// `new_test_ext` cannot be used here since it pre-creates both collections.
fn new_empty_ext() -> TestExternalities {
	let mut ext: TestExternalities = frame_system::GenesisConfig::<Runtime>::default()
		.build_storage()
		.expect("frame system genesis storage builds")
		.into();
	ext.execute_with(|| System::set_block_number(1));
	ext
}

#[test]
fn migrations_tuple_initializes_bootstrap_state() {
	new_empty_ext().execute_with(|| {
		<crate::migrations::SingleBlockMigrations as OnRuntimeUpgrade>::on_runtime_upgrade();

		// Both member collections exist.
		assert!(indiv_pallet_people::PeopleCollectionCreated::<Runtime>::get());
		assert!(indiv_pallet_people_lite::LitePeopleCollectionCreated::<Runtime>::get());

		// The SRS chunk page hashes are committed for both ring sizes this runtime uses.
		assert!(ChunkPageHashes::<Runtime>::contains_key(RingExponent::R2e9, 0));
		assert!(ChunkPageHashes::<Runtime>::contains_key(RingExponent::R2e10, 0));

		// Asset Hub is whitelisted for a permissionless one-shot subscription.
		assert!(indiv_pallet_members_notifier::SubscriptionWhitelist::<Runtime>::contains_key(
			ParaId::from(ASSET_HUB_ID)
		));
	});
}

#[test]
fn migrations_tuple_rerun_is_a_no_op() {
	new_empty_ext().execute_with(|| {
		<crate::migrations::SingleBlockMigrations as OnRuntimeUpgrade>::on_runtime_upgrade();
		let root_after_first = sp_io::storage::root(sp_runtime::StateVersion::V1);

		<crate::migrations::SingleBlockMigrations as OnRuntimeUpgrade>::on_runtime_upgrade();
		let root_after_second = sp_io::storage::root(sp_runtime::StateVersion::V1);

		assert_eq!(root_after_first, root_after_second);
	});
}
