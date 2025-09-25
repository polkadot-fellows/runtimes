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

//! Asset Hub Migration tests.

#[cfg(feature = "kusama-ahm")]
use crate::porting_prelude::*;

use asset_hub_polkadot_runtime::{BuildStorage, Runtime as T, RuntimeCall, RuntimeOrigin};
use cumulus_primitives_core::AggregateMessageOrigin;
use frame_support::{sp_runtime::traits::Dispatchable, traits::Contains};
use pallet_ah_migrator::*;
use sp_runtime::AccountId32;

/// Check that the call filtering mechanism works.
#[test]
fn call_filter_works() {
	let mut t: sp_io::TestExternalities =
		frame_system::GenesisConfig::<T>::default().build_storage().unwrap().into();

	// MQ calls are never filtered:
	let mq_call = RuntimeCall::MessageQueue(pallet_message_queue::Call::<T>::reap_page {
		message_origin: AggregateMessageOrigin::Here,
		page_index: 0,
	});
	// Balances calls are filtered during the migration:
	let balances_call = RuntimeCall::Balances(pallet_balances::Call::<T>::transfer_all {
		dest: AccountId32::from([0; 32]).into(),
		keep_alive: false,
	});
	// Indices calls are filtered during and after the migration:
	let indices_call = RuntimeCall::Indices(pallet_indices::Call::<T>::claim { index: 0 });
	// Staking calls are filtered before and during the migration:
	let staking_call =
		RuntimeCall::Staking(pallet_staking_async::Call::<T>::nominate { targets: vec![] });

	let is_allowed = |call: &RuntimeCall| Pallet::<T>::contains(call);

	// Try the BaseCallFilter
	t.execute_with(|| {
		// Before the migration starts
		{
			AhMigrationStage::<T>::put(MigrationStage::Pending);

			assert!(is_allowed(&mq_call));
			assert!(is_allowed(&balances_call));
			assert!(!is_allowed(&indices_call));
			assert!(!is_allowed(&staking_call));
		}

		// During the migration
		{
			AhMigrationStage::<T>::put(MigrationStage::DataMigrationOngoing);

			assert!(is_allowed(&mq_call));
			assert!(
				is_allowed(&balances_call),
				"Balance transfers are allowed on AH during the migration"
			);
			assert!(!is_allowed(&indices_call));
			assert!(!is_allowed(&staking_call));
		}

		// After the migration
		{
			AhMigrationStage::<T>::put(MigrationStage::MigrationDone);

			assert!(is_allowed(&mq_call));
			assert!(is_allowed(&balances_call));
			assert!(is_allowed(&indices_call));
			assert!(is_allowed(&staking_call));
		}
	});

	// Try to actually dispatch the calls
	t.execute_with(|| {
		let _ =
			<pallet_balances::Pallet<T> as frame_support::traits::Currency<_>>::deposit_creating(
				&AccountId32::from([0; 32]),
				u64::MAX.into(),
			);

		// Before the migration starts
		{
			AhMigrationStage::<T>::put(MigrationStage::Pending);

			assert!(!is_forbidden(&mq_call));
			assert!(!is_forbidden(&balances_call));
			assert!(is_forbidden(&indices_call));
			assert!(is_forbidden(&staking_call));
		}

		// During the migration
		{
			AhMigrationStage::<T>::put(MigrationStage::DataMigrationOngoing);

			assert!(!is_forbidden(&mq_call));
			assert!(
				!is_forbidden(&balances_call),
				"Balance transfers are allowed on AH during the migration"
			);
			assert!(is_forbidden(&indices_call));
			assert!(is_forbidden(&staking_call));
		}

		// After the migration
		{
			AhMigrationStage::<T>::put(MigrationStage::MigrationDone);

			assert!(!is_forbidden(&mq_call));
			assert!(!is_forbidden(&balances_call));
			assert!(!is_forbidden(&indices_call));
			assert!(!is_forbidden(&staking_call));
		}
	});
}

/// Whether a call is forbidden by the call filter.
fn is_forbidden(call: &RuntimeCall) -> bool {
	let Err(err) = call.clone().dispatch(RuntimeOrigin::signed(AccountId32::from([0; 32]))) else {
		return false;
	};

	let filtered_err: sp_runtime::DispatchError = frame_system::Error::<T>::CallFiltered.into();
	err.error == filtered_err
}
