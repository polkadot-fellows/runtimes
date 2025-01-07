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

//! Asset Hub Migrator tests.

mod accounts;

// Runtime specific imports
// Use general aliases for the imports to make it easier to copy&paste the tests for other runtimes.
use polkadot_runtime::{Block, RcMigrator, Runtime as T, System, *};

// General imports
use remote_externalities::{Builder, Mode, OfflineConfig, RemoteExternalities};

/// Create externalities that have their state initialized from a snapshot.
///
/// The path to the snapshot must be provided through the environment variable `SNAP`. If if is not
/// set, this function will return `None`.
///
/// You can create such a snapshot with the [`try-runtime-cli`](https://github.com/paritytech/try-runtime-cli). For example:
/// `try-runtime create-snapshot --uri wss://rpc.polkadot.io:443 polkadot.snap`.
async fn remote_ext_test_setup() -> Option<RemoteExternalities<Block>> {
	sp_tracing::try_init_simple();
	let snap = std::env::var("SNAP").ok()?;
	//let snap = "/home/oliver/polkadot.snap".to_string(); // FIXME dont push
	let abs = std::path::absolute(snap.clone());

	let ext = Builder::<Block>::default()
		.mode(Mode::Offline(OfflineConfig { state_snapshot: snap.clone().into() }))
		.build()
		.await
		.map_err(|e| {
			eprintln!("Could not load from snapshot: {:?}: {:?}", abs, e);
		})
		.unwrap();

	Some(ext)
}
