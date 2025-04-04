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

use asset_hub_polkadot_runtime::Runtime;
use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use pallet_ah_migrator::{benchmarking, benchmarking::*, Pallet};
use sp_runtime::BuildStorage;

pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();
	t.into()
}

//  #[test]
//  fn test_benchmarks() {
//    new_test_ext().execute_with(|| {
//      assert_ok!(pallet_ah_migrator::Pallet::<Runtime>::test_benchmark_receive_multisigs());
//    });
//  }

// impl_benchmark_test_suite!(
// 	Pallet,
// 	crate::bench::new_test_ext(),
// 	asset_hub_polkadot_runtime::Runtime,
// 	benchmarks_path = benchmarking,
// );
