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
//! SNAP_RC="../../polkadot.snap" SNAP_AH="../../ah-polkadot.snap" RUST_LOG="info" ct polkadot-integration-tests-ahm -r on_initialize_works -- --nocapture
//! ```

use asset_hub_polkadot_runtime::Runtime as AssetHub;
use pallet_rc_migrator::types::RcPalletMigrationChecks;
use polkadot_runtime::Runtime as Polkadot;

use super::mock::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn account_migration_works() {
	let Some((rc, ah)) = load_externalities().await else { return };
	type RcPayload = <pallet_rc_migrator::preimage::PreimageChunkMigrator<Polkadot> as RcPalletMigrationChecks>::RcPayload;
	let (dmp_messages, rc_payload) =
		rc_migrate::<pallet_rc_migrator::preimage::PreimageChunkMigrator<Polkadot>>(rc);
	ah_migrate::<
		pallet_rc_migrator::preimage::PreimageChunkMigrator<Polkadot>,
		pallet_ah_migrator::preimage::PreimageMigrationCheck<AssetHub>,
	>(ah, rc_payload, dmp_messages);
}
