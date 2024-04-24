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

/// The Kusama Coretime chain had some launch issues. These migrations clean up state and enable
/// immediate onboarding of system parachains.
///
/// None of these migrations affect storage structure, only values.
pub mod bootstrapping {
	use crate::{weights, Runtime, RuntimeOrigin};
	use frame_support::{pallet_prelude::*, traits::OnRuntimeUpgrade};
	#[cfg(feature = "try-runtime")]
	use pallet_broker::StatusRecord;
	use pallet_broker::{
		AllowedRenewals,
		CoreAssignment::{Pool, Task},
		CoreIndex, CoreMask, Leases, Reservations, SaleInfo, Schedule, ScheduleItem, Status,
		Timeslice, WeightInfo, Workplan,
	};
	#[cfg(feature = "try-runtime")]
	use sp_runtime::TryRuntimeError;
	#[cfg(feature = "try-runtime")]
	use sp_std::vec::Vec;

	/// The log target.
	const TARGET: &str = "runtime::bootstrapping::onboard-people";

	// The key in Workplan with the outdated assignment.
	const WORKPLAN_KEY: (Timeslice, CoreIndex) = (289960, 4);

	// Alias to the broker weights for this runtime.
	type BrokerWeights = weights::pallet_broker::WeightInfo<Runtime>;
	type RuntimeDbWeight = <Runtime as frame_system::Config>::DbWeight;

	/// This migration cleans up an outdated pool assignment in state from the update to Kusama
	/// Coretime 1002002.
	pub struct RemoveOutdatedPoolAssignment;

	impl OnRuntimeUpgrade for RemoveOutdatedPoolAssignment {
		fn on_runtime_upgrade() -> Weight {
			let schedule_pool = Schedule::truncate_from(Vec::from([ScheduleItem {
				mask: CoreMask::complete(),
				assignment: Pool,
			}]));
			if Workplan::<Runtime>::get(WORKPLAN_KEY) != Some(schedule_pool) {
				// Erroneous pool core assignment is not in state. Bailing.
				log::error!(target: TARGET, "This migration includes hardcoded values not relevant to this runtime. Bailing.");
				return RuntimeDbWeight::get().reads(1);
			}

			// Overwrite outdated pool core assignment to keep parachain 2000 on core.
			let schedule_2000 = Schedule::truncate_from(Vec::from([ScheduleItem {
				mask: CoreMask::complete(),
				assignment: Task(2000),
			}]));
			Workplan::<Runtime>::insert(WORKPLAN_KEY, schedule_2000);

			log::info!(target: TARGET, "Outdated Workplan entry has been overwritten.");

			RuntimeDbWeight::get().reads(1).saturating_add(RuntimeDbWeight::get().writes(1))
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
			let schedule_pool = Schedule::truncate_from(Vec::from([ScheduleItem {
				mask: CoreMask::complete(),
				assignment: Pool,
			}]));
			if Workplan::<Runtime>::get(WORKPLAN_KEY) != Some(schedule_pool) {
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
			log::info!(target: TARGET, "Checking migration.");

			// Check that cores 0-4 are now all reassigned to themselves at the end of the original
			// period 0 before sales were restarted.
			let expected_assignments = [Task(1000), Task(1001), Task(1002), Task(1005), Task(2000)];
			for (core, assignment) in expected_assignments.into_iter().enumerate() {
				assert_eq!(
					Workplan::<Runtime>::get((289960, core as u16)),
					Some(Schedule::truncate_from(Vec::from([ScheduleItem {
						mask: CoreMask::complete(),
						assignment
					}])))
				);
			}

			// There are no more surprise entries in the Workplan - the only cores which have
			// reassignments before start sales kicks in are the five checked above.
			assert_eq!(
				Workplan::<Runtime>::iter_keys()
					.filter(|(timeslice, _)| *timeslice != 290808)
					.count(),
				5
			);

			Ok(())
		}
	}

	/// The People Chain should be onboarded ASAP to Kusama, however the reserve extrinsic
	/// takes two sale period boundaries to actually put new reservations on core. This
	/// migration adds the People Chain immediately.
	///
	/// This is achieved in three steps:
	/// 1. Reserve a core for People (from period 2)
	/// 2. Add People Chain to the workplan for period 1
	/// 3. Add People Chain to the workplan for the remainder of period 0
	pub struct OnboardPeople;

	impl OnRuntimeUpgrade for OnboardPeople {
		fn on_runtime_upgrade() -> Weight {
			// Make sure People Chain is not already reserved.
			let schedule_people = Schedule::truncate_from(Vec::from([ScheduleItem {
				mask: CoreMask::complete(),
				assignment: Task(1004),
			}]));
			if Reservations::<Runtime>::get().iter().any(|res| *res == schedule_people) {
				log::error!(target: TARGET, "The people chain is already reserved. Bailing.");
				return RuntimeDbWeight::get().reads(1);
			}

			let next_period = SaleInfo::<Runtime>::get()
				.map(|sale_info| sale_info.region_begin)
				.expect("Sales have started on Kusama.");

			// Request an extra core for the People Chain.
			let core_count = Reservations::<Runtime>::decode_len().unwrap_or(0) as u16 +
				Leases::<Runtime>::decode_len().unwrap_or(0) as u16 +
				AllowedRenewals::<Runtime>::iter_keys()
					.filter(|renewal| renewal.when >= next_period)
					.count() as u16 + 4;

			match pallet_broker::Pallet::<Runtime>::request_core_count(
				RuntimeOrigin::root(),
				core_count,
			) {
				Ok(_) => log::info!(target: TARGET, "Request for 56 cores sent."),
				Err(_) => log::error!(target: TARGET, "Request for 56 cores failed to send."),
			}

			// People core should be assigned the new core to avoid clashes with the cores sold in
			// period 0.
			let people_core = core_count.saturating_sub(1);

			// 1. Schedule People Chain for period 2 and beyond.
			let schedule_people = Schedule::truncate_from(Vec::from([ScheduleItem {
				mask: CoreMask::complete(),
				assignment: Task(1004),
			}]));
			match pallet_broker::Pallet::<Runtime>::reserve(
				RuntimeOrigin::root(),
				schedule_people.clone(),
			) {
				Ok(_) => log::info!(target: TARGET, "People Chain reserved"),
				Err(_) => log::error!(target: TARGET, "People Chain reservation failed!"),
			}

			// 2. Schedule People Chain for period 1.
			Workplan::<Runtime>::insert((next_period, people_core), schedule_people.clone());

			// 3. Schedule People for the rest of period 0. Take the timeslice after the next tick
			//    so we the core definitely gets processed.
			let now_ish = Status::<Runtime>::get()
				.map(|status| status.last_committed_timeslice.saturating_add(2))
				.expect("Sales have started on Kusama.");
			Workplan::<Runtime>::insert((now_ish, people_core), schedule_people);

			BrokerWeights::reserve()
				.saturating_add(BrokerWeights::request_core_count(56))
				.saturating_add(RuntimeDbWeight::get().reads(6))
				.saturating_add(RuntimeDbWeight::get().writes(2))
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
			let schedule_people = Schedule::truncate_from(Vec::from([ScheduleItem {
				mask: CoreMask::complete(),
				assignment: Task(1004),
			}]));
			if Reservations::<Runtime>::get().iter().any(|res| *res == schedule_people) {
				return Ok(Vec::new())
			}
			let status = Status::<Runtime>::get().unwrap();
			Ok(status.encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
			if state.is_empty() {
				return Ok(())
			}
			log::info!(target: TARGET, "Checking migration.");

			let prev_status = <StatusRecord>::decode(&mut &state[..]).unwrap();

			// People Chain is reserved exactly once.
			let schedule_people = Schedule::truncate_from(Vec::from([ScheduleItem {
				mask: CoreMask::complete(),
				assignment: Task(1004),
			}]));
			assert_eq!(
				Reservations::<Runtime>::get()
					.iter()
					.filter(|&res| *res == schedule_people.clone())
					.count(),
				1
			);

			// And is in the Workplan for periods 0 and 1.
			assert_eq!(
				Workplan::<Runtime>::get((prev_status.last_committed_timeslice + 2, 55)),
				Some(schedule_people.clone())
			);

			let next_period =
				SaleInfo::<Runtime>::get().map(|sale_info| sale_info.region_begin).unwrap();

			assert_eq!(Workplan::<Runtime>::get((next_period, 55)), Some(schedule_people.clone()));

			// Ensure we have requested the correct number of cores.
			assert!(frame_system::Pallet::<Runtime>::read_events_no_consensus().any(|e| {
				match e.event {
					crate::RuntimeEvent::Broker(
						pallet_broker::Event::<Runtime>::CoreCountRequested { core_count },
					) => {
						log::info!(target: TARGET, "Reserved {core_count:?} cores.");

						// Ensure that both of these are correct as a sanity check since we hardcode
						// core 55 elsewhere.
						core_count == prev_status.core_count + 1 && core_count == 56
					},
					_ => false,
				}
			}));

			// And ensure this core isn't overwritten at any stage, it should only have the two
			// entries in the workload that we just checked.
			assert_eq!(Workplan::<Runtime>::iter_keys().filter(|(_, core)| *core == 55).count(), 2);

			Ok(())
		}
	}
}
