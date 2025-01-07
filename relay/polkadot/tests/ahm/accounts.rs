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

use frame_support::{pallet_prelude::*, traits::*, weights::WeightMeter};
use sp_core::H256;
use sp_io::TestExternalities;
use sp_storage::StateVersion;

use std::cell::OnceCell;

use super::*;
use pallet_rc_migrator::RcMigrationStage;

#[tokio::test]
async fn account_test() {
	remote_ext_test_setup().await.map(|mut e| {
		e.execute_with(|| {
			let _ = RcMigrator::obtain_rc_accounts();
			let _ = RcMigrator::migrate_accounts(None, &mut WeightMeter::new()).unwrap();
		})
	});
}

#[tokio::test]
async fn on_initialize_works() {
	remote_ext_test_setup().await.map(|mut e| {
		e.execute_with(|| {
			for _ in 0..10 {
				log::debug!(target: LOG_TARGET, "Stage: {:?}", RcMigrationStage::<T>::get());
				next_block();
			}

			// DMP:
			let para_id = polkadot_parachain_primitives::primitives::Id::from(1000);
			let msg_count = runtime_parachains::dmp::DownwardMessageQueues::<T>::get(para_id).len();
			log::debug!(target: LOG_TARGET, "DMP message count for para 1000: {msg_count}");
		})
	});
}

fn next_block() {
	let now = System::block_number();
	log::debug!(target: LOG_TARGET, "Next block: {:?}", now + 1);
	<RcMigrator as frame_support::traits::OnFinalize<_>>::on_finalize(now);
	System::set_block_number(now + 1);
	<RcMigrator as frame_support::traits::OnInitialize<_>>::on_initialize(now + 1);
}
