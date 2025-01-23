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

use frame_support::{sp_runtime::traits::Dispatchable, traits::Contains};
use pallet_rc_migrator::*;
use polkadot_primitives::Id as ParaId;
use polkadot_runtime::{Block, BuildStorage, RcMigrator, Runtime as T, RuntimeOrigin, System, *};
use remote_externalities::{Builder, Mode, OfflineConfig, RemoteExternalities};
use runtime_parachains::inclusion::AggregateMessageOrigin;
use sp_runtime::AccountId32;

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

#[test]
fn call_filter_works() {
	let mut t: sp_io::TestExternalities =
		frame_system::GenesisConfig::<T>::default().build_storage().unwrap().into();

	// MQ calls are never filtered:
	let mq_call =
		polkadot_runtime::RuntimeCall::MessageQueue(pallet_message_queue::Call::<T>::reap_page {
			message_origin: AggregateMessageOrigin::Ump(
				runtime_parachains::inclusion::UmpQueueId::Para(ParaId::from(1000)),
			),
			page_index: 0,
		});
	// System calls are filtered during the migration:
	let system_call =
		polkadot_runtime::RuntimeCall::System(frame_system::Call::<T>::remark { remark: vec![] });
	// Indices calls are filtered during and after the migration:
	let indices_call =
		polkadot_runtime::RuntimeCall::Indices(pallet_indices::Call::<T>::claim { index: 0 });

	let is_allowed = |call: &polkadot_runtime::RuntimeCall| Pallet::<T>::contains(call);

	// Try the BaseCallFilter
	t.execute_with(|| {
		// Before the migration starts
		{
			RcMigrationStage::<T>::put(MigrationStage::Pending);

			assert!(is_allowed(&mq_call));
			assert!(is_allowed(&system_call));
			assert!(is_allowed(&indices_call));
		}

		// During the migration
		{
			RcMigrationStage::<T>::put(MigrationStage::ProxyMigrationInit);

			assert!(is_allowed(&mq_call));
			assert!(!is_allowed(&system_call));
			assert!(!is_allowed(&indices_call));
		}

		// After the migration
		{
			RcMigrationStage::<T>::put(MigrationStage::MigrationDone);

			assert!(is_allowed(&mq_call));
			assert!(is_allowed(&system_call));
			assert!(!is_allowed(&indices_call));
		}
	});

	// Try to actually dispatch the calls
	t.execute_with(|| {
		let alice = AccountId32::from([0; 32]);
		<pallet_balances::Pallet<T> as frame_support::traits::Currency<_>>::deposit_creating(
			&alice,
			u64::MAX.into(),
		);

		// Before the migration starts
		{
			RcMigrationStage::<T>::put(MigrationStage::Pending);

			assert!(!is_forbidden(&mq_call));
			assert!(!is_forbidden(&system_call));
			assert!(!is_forbidden(&indices_call));
		}

		// During the migration
		{
			RcMigrationStage::<T>::put(MigrationStage::ProxyMigrationInit);

			assert!(!is_forbidden(&mq_call));
			assert!(is_forbidden(&system_call));
			assert!(is_forbidden(&indices_call));
		}

		// After the migration
		{
			RcMigrationStage::<T>::put(MigrationStage::MigrationDone);

			assert!(!is_forbidden(&mq_call));
			assert!(!is_forbidden(&system_call));
			assert!(is_forbidden(&indices_call));
		}
	});
}

fn is_forbidden(call: &polkadot_runtime::RuntimeCall) -> bool {
	let Err(err) = call.clone().dispatch(RuntimeOrigin::signed(AccountId32::from([0; 32]))) else {
		return false;
	};

	let runtime_err: sp_runtime::DispatchError = frame_system::Error::<T>::CallFiltered.into();
	err.error == runtime_err
}
