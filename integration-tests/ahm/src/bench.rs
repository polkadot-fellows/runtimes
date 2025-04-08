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

//! Test AH migrator pallet benchmark functions.

#![cfg(feature = "runtime-benchmarks")]

use asset_hub_polkadot_runtime::{Runtime as AssetHub, System as AssetHubSystem};
use pallet_ah_migrator::benchmarking::*;
use sp_runtime::BuildStorage;

pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::<AssetHub>::default().build_storage().unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| AssetHubSystem::set_block_number(1));
	ext
}

#[test]
fn test_benchmarks() {
	const BENCHMARK_N: u32 = 10;

	new_test_ext().execute_with(|| {
		test_receive_multisigs::<AssetHub>(BENCHMARK_N);
	});
	new_test_ext().execute_with(|| {
		test_on_finalize::<AssetHub>();
	});
	new_test_ext().execute_with(|| {
		test_receive_proxy_proxies::<AssetHub>(BENCHMARK_N);
	});
	new_test_ext().execute_with(|| {
		test_receive_proxy_announcements::<AssetHub>(BENCHMARK_N);
	});
	new_test_ext().execute_with(|| {
		test_receive_claims::<AssetHub>(BENCHMARK_N);
	});
	new_test_ext().execute_with(|| {
		test_receive_nom_pools_messages::<AssetHub>(BENCHMARK_N);
	});
	new_test_ext().execute_with(|| {
		test_receive_vesting_schedules::<AssetHub>(BENCHMARK_N);
	});
	new_test_ext().execute_with(|| {
		test_receive_fast_unstake_messages::<AssetHub>(BENCHMARK_N);
	});
	new_test_ext().execute_with(|| {
		test_receive_referenda_values::<AssetHub>();
	});
	new_test_ext().execute_with(|| {
		test_receive_active_referendums::<AssetHub>(BENCHMARK_N);
	});
	new_test_ext().execute_with(|| {
		test_receive_complete_referendums::<AssetHub>(BENCHMARK_N);
	});
	new_test_ext().execute_with(|| {
		test_receive_accounts::<AssetHub>(BENCHMARK_N);
	});
	new_test_ext().execute_with(|| {
		test_receive_liquid_accounts::<AssetHub>(BENCHMARK_N);
	});
	new_test_ext().execute_with(|| {
		test_receive_scheduler_agenda::<AssetHub>(BENCHMARK_N);
	});
	new_test_ext().execute_with(|| {
		test_receive_scheduler_lookup::<AssetHub>(BENCHMARK_N);
	});
	new_test_ext().execute_with(|| {
		test_receive_bags_list_messages::<AssetHub>(BENCHMARK_N);
	});
	new_test_ext().execute_with(|| {
		test_receive_indices::<AssetHub>(BENCHMARK_N);
	});
	new_test_ext().execute_with(|| {
		test_receive_conviction_voting_messages::<AssetHub>(BENCHMARK_N);
	});
}
