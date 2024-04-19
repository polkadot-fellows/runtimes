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

/// The XCM Transact which was meant to set the leases as part of the Kusama relay runtime upgrade
/// did not have enough weight. Therefore the leases were not migrated.
///
/// This migration populates the leases and restarts the sale from whichever timeslice it runs.
///
/// This does not affect storage structure, only values.
pub mod bootstrapping {
	use crate::{weights, Runtime, RuntimeOrigin};
	use frame_support::{pallet_prelude::*, traits::OnRuntimeUpgrade};
	#[cfg(feature = "try-runtime")]
	use pallet_broker::{
		AllowedRenewalId, AllowedRenewalRecord, AllowedRenewals, Configuration,
		CoreAssignment::{Pool, Task},
		CoreMask, LeaseRecordItem, SaleInfo, SaleInfoRecordOf, Schedule, ScheduleItem, Workplan,
	};
	use pallet_broker::{Leases, WeightInfo};
	#[cfg(feature = "try-runtime")]
	use sp_runtime::TryRuntimeError;
	#[cfg(feature = "try-runtime")]
	use sp_std::vec::Vec;

	/// The log target.
	const TARGET: &str = "runtime::bootstrapping::import-leases";

	// Alias into the broker weights for this runtime.
	type BrokerWeights = weights::pallet_broker::WeightInfo<Runtime>;

	pub struct ImportLeases;

	impl OnRuntimeUpgrade for ImportLeases {
		fn on_runtime_upgrade() -> Weight {
			// This migration contains hardcoded values only relevant to Kusama Coretime
			// 1002000 before it has any leases. These checks could be tightened.
			if Leases::<Runtime>::decode_len().unwrap_or(0) > 0 {
				// Already has leases, bail
				log::error!(target: TARGET, "This migration includes hardcoded values not relevant to this runtime. Bailing.");
				return <Runtime as frame_system::Config>::DbWeight::get().reads(1);
			}

			for (para_id, end) in LEASES {
				match pallet_broker::Pallet::<Runtime>::set_lease(
					RuntimeOrigin::root(),
					para_id,
					end,
				) {
					Ok(_) =>
						log::info!(target: TARGET, "Importing lease for parachain {}", &para_id),
					Err(_) =>
						log::error!(target: TARGET, "Importing lease for parachain {} failed!", &para_id),
				}
			}

			// The values used in referendum 375 included 52 cores. Replaying this here shifts the
			// start of the sale, while crucially populating the workplan with the leases and
			// recalculating the number of cores to be offered. However, there are 4 system
			// parachains + 1 pool core + 47 leases + 3 cores for the open market, therefore we need
			// to start sales with 55 cores.
			match pallet_broker::Pallet::<Runtime>::request_core_count(
				RuntimeOrigin::root(),
				pallet_broker::Reservations::<Runtime>::decode_len().unwrap_or(0) as u16 +
					pallet_broker::Leases::<Runtime>::decode_len().unwrap_or(0) as u16 +
					1 + 3,
			) {
				Ok(_) => log::info!(target: TARGET, "Request for 55 cores sent."),
				Err(_) => log::error!(target: TARGET, "Request for 55 cores failed to send."),
			}
			match pallet_broker::Pallet::<Runtime>::start_sales(
				RuntimeOrigin::root(),
				5_000_000_000_000,
				55,
			) {
				Ok(_) => log::info!(target: TARGET, "Sales started"),
				Err(_) => log::error!(target: TARGET, "Start sales failed!"),
			}

			// Weight for setting every lease and starting the sales, plus one read for leases
			// check.
			BrokerWeights::set_lease()
				.saturating_mul(LEASES.len() as u64)
				.saturating_add(BrokerWeights::request_core_count(55))
				.saturating_add(BrokerWeights::start_sales(55))
				.saturating_add(<Runtime as frame_system::Config>::DbWeight::get().reads(1))
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
			if Leases::<Runtime>::decode_len().unwrap_or(0) > 0 {
				return Ok(Vec::new())
			}
			let sale_info = SaleInfo::<Runtime>::get().unwrap();
			Ok(sale_info.encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
			if state.is_empty() {
				return Ok(())
			}
			let prev_sale_info = <SaleInfoRecordOf<Runtime>>::decode(&mut &state[..]).unwrap();

			log::info!(target: TARGET, "Checking migration.");

			let sale_info = SaleInfo::<Runtime>::get().unwrap();
			let now = frame_system::Pallet::<Runtime>::block_number();
			let config = Configuration::<Runtime>::get().unwrap();

			// Check the sale start has changed as expected and the cores_offered is the correct
			// number.
			assert_eq!(sale_info.sale_start, now + config.interlude_length);
			assert!(sale_info.region_begin > prev_sale_info.region_begin);
			assert_eq!(sale_info.cores_offered, 3);

			// The workplan entries start from the region begin reported by the new SaleInfo.
			let workplan_start = sale_info.region_begin;

			// Check the reservations are still in the workplan out of an abundance of caution.
			for (core_id, task) in
				[Task(1000), Task(1001), Task(1002), Task(1005), Pool].into_iter().enumerate()
			{
				assert_eq!(
					Workplan::<Runtime>::get((workplan_start, core_id as u16)),
					Some(Schedule::truncate_from(Vec::from([ScheduleItem {
						mask: CoreMask::complete(),
						assignment: task,
					}])))
				);
			}

			// Because we also run start_sales, 12 expiring leases are removed from the original 47,
			// leaving 35.
			let leases = Leases::<Runtime>::get();
			assert_eq!(
				leases.len(),
				LEASES.iter().filter(|(_, l)| sale_info.region_end * 80 <= l).count()
			);

			// Iterate through hardcoded leases and check they're all correctly in state (leases or
			// allowedrenewals) and scheduled in the workplan.
			for (i, (para_id, until)) in LEASES.iter().enumerate() {
				// Add the system parachains and pool core as an offset - these should come before
				// the leases.
				let core_id = i as u16 + 5;
				// This is the entry found in Workplan and AllowedRenewal
				let workload = Schedule::truncate_from(Vec::from([ScheduleItem {
					mask: CoreMask::complete(),
					assignment: Task(*para_id),
				}]));

				// Check that the 12 who no longer have a lease can renew.
				if !leases.contains(&LeaseRecordItem { until: *until, task: *para_id }) {
					assert_eq!(
						AllowedRenewals::<Runtime>::get(AllowedRenewalId {
							core: core_id,
							when: sale_info.region_end,
						}),
						Some(AllowedRenewalRecord {
							price: 5_000_000_000_000,
							completion: pallet_broker::CompletionStatus::Complete(workload.clone())
						})
					);
				}
				// They should all be in the workplan for next sale.
				assert_eq!(Workplan::<Runtime>::get((workplan_start, core_id)), Some(workload));
			}

			// Ensure we have requested the correct number of events.
			assert!(frame_system::Pallet::<Runtime>::read_events_no_consensus()
				.any(|e| pallet_broker::Event::CoreCountRequested { core_count: 55 }.into() == e));

			Ok(())
		}
	}

	// Hardcoded para ids and their end timeslice.
	// Calculated using https://github.com/seadanda/coretime-scripts/blob/main/get_leases.py
	const LEASES: [(u32, u32); 47] = [
		(2000, 340200),
		(2001, 302400),
		(2004, 332640),
		(2007, 317520),
		(2011, 325080),
		(2012, 309960),
		(2015, 287280),
		(2023, 309960),
		(2024, 309960),
		(2048, 302400),
		(2084, 340200),
		(2085, 294840),
		(2087, 340200),
		(2088, 287280),
		(2090, 340200),
		(2092, 287280),
		(2095, 332640),
		(2096, 332640),
		(2105, 325080),
		(2106, 325080),
		(2110, 317520),
		(2113, 332640),
		(2114, 317520),
		(2119, 340200),
		(2121, 332640),
		(2123, 294840),
		(2124, 287280),
		(2125, 294840),
		(2222, 302400),
		(2233, 294840),
		(2236, 317520),
		(2239, 332640),
		(2241, 325080),
		(2274, 294840),
		(2275, 294840),
		(2281, 302400),
		(3334, 309960),
		(3336, 317520),
		(3338, 317520),
		(3339, 325080),
		(3340, 325080),
		(3343, 317520),
		(3344, 340200),
		(3345, 325080),
		(3347, 287280),
		(3348, 287280),
		(3350, 340200),
	];
}
