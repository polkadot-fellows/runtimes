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

//! Migration to fix the coretime migration lease offset-related issues
//!
//! This fixes a problem with the leases where the relay migration did not take the lease offset
//! into consideration, so the end of leases is 64 days short, in some cases leading to them being
//! dropped completely.

extern crate alloc;

use crate::{weights, Runtime, RuntimeOrigin};
use frame_support::{pallet_prelude::*, traits::OnRuntimeUpgrade};
use pallet_broker::{
	CompletionStatus, CoreAssignment::{self, Pool}, CoreIndex, CoreMask, LeaseRecordItem, Leases, LeasesRecordOf, PotentialRenewalId, PotentialRenewals, SalePerformance, Schedule, ScheduleItem, TaskId, Timeslice, WeightInfo, Workplan, AdaptPrice,
};

use sp_std::vec::Vec;
use alloc::collections::btree_map::BTreeMap;
use sp_arithmetic::traits::Saturating;

#[cfg(feature = "try-runtime")]
use pallet_broker::{CoreAssignment::Task, PotentialRenewalRecord, SaleInfo, SaleInfoRecordOf};
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;
use system_parachains_constants::polkadot::currency::UNITS;

/// The log target.
const TARGET: &str = "runtime::bootstrapping::fix-migration";

// Alias into the broker weights for this runtime.
type BrokerWeights = weights::pallet_broker::WeightInfo<Runtime>;

pub struct FixMigration;

impl FixMigration {

    fn get_first_usable_core() -> Option<CoreIndex> {
        let sale_info = SaleInfo::<Runtime>::get()?;
        // Cores between first_core and cores_offered are for sale and my not be used anymore:
        Some(sale_info.first_core + sale_info.cores_offered)
    }

    fn add_pool_core(first_core: &mut CoreIndex) {
        let Some(sale_info) = SaleInfo::<Runtime>::get() else {
            log::error!(target: TARGET, "Retrieving `SaleInfo` failed!");
            return
        };

        let schedule = BoundedVec::truncate_from(Vec::from([ScheduleItem { mask: CoreMask::complete(), assignment: CoreAssignment::Pool}]));
        let result = pallet_broker::Pallet::<Runtime>::reserve(RuntimeOrigin::root(), schedule.clone());
        debug_assert!(result.is_ok());
        // But we want it now(ish): add it to the workplan.
        Workplan::<Runtime>::insert((sale_info.region_begin, *first_core), schedule);
        first_core.saturating_inc();
    }

    fn extend_short_leases() {
        let short_leases: BTreeMap<TaskId, Timeslice> = [
            (2035, 359280),
            (3344, 344160),
            (3370, 389520),
            (3367, 374400),
            (2086, 389520),
            (2032, 359280),
            (2051, 389520),
            (3369, 313920),
            (2008, 389520),
            (2025, 359280),
            (2000, 313920),
            (2092, 404640),
            (2002, 359280),
            (3359, 389520),
            (2030, 374400),
            (3378, 404640),
            (2104, 404640),
            (2046, 374400),
            (3345, 344160),
            (3340, 344160),
            (3338, 329040),
            (2004, 329040),
            (3377, 389520),
            (3373, 389520),
            (2031, 344160),
            (3389, 404640),
            (3366, 374400),
            (2037, 374400),
            (2034, 359280),
            (2090, 404640),
            (3346, 344160),
            (2012, 359280),
            (3397, 404640),
            (2043, 374400),
            (2091, 404640),
            (2026, 344160),
            (3388, 404640),
            (3354, 359280),
            (2006, 344160),
            (2013, 374400),
        ].into();

        let leases = Leases::<Runtime>::get();
        let leases = BoundedVec::truncate_from(leases.into_iter().map(|mut l| {
            let Some(new_until) = short_leases.get(&l.task) else {
                return l
            };
            debug_assert!(*new_until > l.until);
            l.until = *new_until;
            l
        }).collect());
        Leases::<Runtime>::put(leases);
    }

    // Undo rotate_sale effect on leases that wrongly expired.
    //
    // Needs to be called before `give_dropped_leases_renewal_rights_and_workplan_entry`.
    fn remove_premature_renewals_add_back_leases() {
        let premature_renewals = [
            (2094, 298800),
            (2040, 298800),
            (3333, 298800),
            (2106, 298800),
            (2093, 298800),
            (2101, 298800),
        ];

        debug_assert_eq!(PotentialRenewals::<Runtime>::iter().count(), premature_renewals.len());
        let result = PotentialRenewals::<Runtime>::clear(premature_renewals.len() as u32, None);
        debug_assert!(result.maybe_cursor.is_none());

        for (task, until) in premature_renewals {
            let result = pallet_broker::Pallet::<Runtime>::set_lease(RuntimeOrigin::root(), task, until);
            debug_assert!(result.is_ok());
        }
    }

    fn give_dropped_leases_renewal_rights_and_workplan_entry(first_core: &mut CoreIndex) {
        let Some(sale_info) = SaleInfo::<Runtime>::get() else {
            log::error!(target: TARGET, "Retrieving `SaleInfo` failed!");
            return
        };

        let dropped_leases = [
            (2048, 283680),
            (3375, 283680),
            (3358, 283680),
            (2053, 283680),
            (2056, 283680),
        ];

        // Leases should have been added, but removed again by rotate_sale - replaces with work
        // plan items + renewal rights.
        // https://github.com/paritytech/polkadot-sdk/blob/f170af615c0dc413482100892758b236d1fda93b/substrate/frame/broker/src/tick_impls.rs#L212
        // So we need to:
        // - Don't change leases, already correct.
        // - Add work plan entry.
        // - add to potential renewals

        for (task, _) in dropped_leases  {
            // Workplan
            let mask = CoreMask::complete();
            let assignment = CoreAssignment::Task(task);
            // DONE: enough to start at next rotation?
            // - Yes all good, assignments are all for next rotation already. Hence relay chain
            // state is persisted for now.
            let schedule = BoundedVec::truncate_from(Vec::from([ScheduleItem { mask, assignment }]));
            Workplan::<Runtime>::insert((sale_info.region_begin, *first_core), schedule.clone());

            // Renewal:
            let new_prices = <Runtime as pallet_broker::Config>::PriceAdapter::adapt_price(SalePerformance::from_sale(&sale_info));
            debug_assert_eq!(new_prices.target_price, 100*UNITS);
            let renewal_id = PotentialRenewalId { core: *first_core, when: sale_info.region_end };
            let record = PotentialRenewalRecord {
                price: new_prices.target_price,
                completion: CompletionStatus::Complete(schedule),
            };
            PotentialRenewals::<Runtime>::insert(renewal_id, &record);

            first_core.saturating_inc()
        }

    }

	fn on_runtime_upgrade_donal() -> Weight {
		if PotentialRenewals::<Runtime>::get(PotentialRenewalId { core: 6, when: 292605 }).is_none()
		{
			// Idempotency check - this core will never be renewable at this timeslice ever again.
			log::error!(target: TARGET, "This migration includes hardcoded values not relevant to this runtime. Bailing.");
			return <Runtime as frame_system::Config>::DbWeight::get().reads(1);
		}

		// Add leases for 2040, 2094, 3333, 2106, 2101, 2093 with a properly calculated end
		// timeslice. Add 11520 for all other leases.
		let leases: LeasesRecordOf<Runtime> = LEASES
			.iter()
			.map(|(until, task)| LeaseRecordItem { until: *until, task: *task })
			.collect::<Vec<_>>()
			.try_into()
			.expect("Within range of bounded vec");
		Leases::<Runtime>::put(leases);

		// This reorders the cores, so the existing entries to the workplan need to be overwritten.
		for (&(para_id, _), core_id) in LEASES.iter().zip(51u16..56u16) {
			// Add to the workplan at timeslice 287565 using the new cores.
			let workplan_entry = Schedule::truncate_from(Vec::from([ScheduleItem {
				mask: CoreMask::complete(),
				assignment: CoreAssignment::Task(para_id),
			}]));

			Workplan::<Runtime>::insert((287565, core_id), workplan_entry);
		}

		// Remove the existing 6 PotentialRenewals.
		for &(core, when) in INCORRECT_RENEWAL_IDS.iter() {
			PotentialRenewals::<Runtime>::remove(PotentialRenewalId { core, when });
		}

		// Sort the parachains who can renew. They are currently missing from the broker
		// state entirely.
		// TODO double check the core ids, these should be on top of the last available in the sale.
		for (&(para_id, _), core_id) in POTENTIAL_RENEWALS.iter().zip(56u16..61u16) {
			// Add to the workplan at timeslice 287565 using the new cores.
			let workplan_entry = Schedule::truncate_from(Vec::from([ScheduleItem {
				mask: CoreMask::complete(),
				assignment: CoreAssignment::Task(para_id),
			}]));

			Workplan::<Runtime>::insert((287565, core_id), workplan_entry);
		}

		// Add cores to the system - this will take 2 sessions to kick in.
		// 1 core for the system on-demand pool, 5 for open market, 5 for reservations and 51
		// parachains, of which 46 have leases and 5 are up for renewal.
		let core_count = pallet_broker::Reservations::<Runtime>::decode_len().unwrap_or(0) as u16 +
			pallet_broker::Leases::<Runtime>::decode_len().unwrap_or(0) as u16 +
			5 + 6;

		match pallet_broker::Pallet::<Runtime>::request_core_count(
			RuntimeOrigin::root(),
			core_count,
		) {
			Ok(_) => log::info!(target: TARGET, "Request for 62 cores sent."),
			Err(_) => log::error!(target: TARGET, "Request for 62 cores failed to send."),
		}

		let pool_assignment = Schedule::truncate_from(Vec::from([ScheduleItem {
			mask: CoreMask::complete(),
			assignment: Pool,
		}]));

		// Reserve the system pool core - this kicks in after two sale period boundaries.
		match pallet_broker::Pallet::<Runtime>::reserve(
			RuntimeOrigin::root(),
			pool_assignment.clone(),
		) {
			Ok(_) => log::info!(target: TARGET, "Pool core reserved."),
			Err(_) => log::error!(target: TARGET, "Pool core reservation failed."),
		}

		// Add the system pool core to the workplan for the next cycle (287565) on the last new core
		// (core 61)
		Workplan::<Runtime>::insert((292605, 61), pool_assignment.clone());

		// Add the system pool core to the workplan starting now.
		let now = 287000; // TODO - this needs to be after the cores have just added have been processed, which takes
				  // two sessions. Currently just a placeholder. This part is probably better as a call to
				  // assign_core on the relay as part of the referendum instead of here.
		Workplan::<Runtime>::insert((now, 61), pool_assignment);

		// TODO finalise the weights here.
		<Runtime as frame_system::Config>::DbWeight::get()
			.writes(1)
			.saturating_mul(LEASES.len() as u64)
			.saturating_add(BrokerWeights::request_core_count(62))
			.saturating_add(<Runtime as frame_system::Config>::DbWeight::get().reads(1))
	}
}

impl OnRuntimeUpgrade for FixMigration {
	fn on_runtime_upgrade() -> Weight {
		if PotentialRenewals::<Runtime>::get(PotentialRenewalId { core: 6, when: 292605 }).is_none()
		{
			// Idempotency check - this core will never be renewable at this timeslice ever again.
			log::error!(target: TARGET, "This migration includes hardcoded values not relevant to this runtime. Bailing.");
			return <Runtime as frame_system::Config>::DbWeight::get().reads(1);
		}

        let Some(mut first_core) = Self::get_first_usable_core() else {
            log::error!(target: TARGET, "Retrieving `SaleInfo` (first_core) failed!");
            // Return dummy weight. This should really not happen and if it does we have bigger
            // problems than wrong weights.
            return <Runtime as frame_system::Config>::DbWeight::get()
			.writes(50)
        };

        Self::add_pool_core(&mut first_core);
        Self::extend_short_leases();
        Self::remove_premature_renewals_add_back_leases();
        Self::give_dropped_leases_renewal_rights_and_workplan_entry(&mut first_core);


		// TODO finalise the weights here.
		<Runtime as frame_system::Config>::DbWeight::get()
			.writes(1)
			.saturating_mul(LEASES.len() as u64)
			.saturating_add(BrokerWeights::request_core_count(62))
			.saturating_add(<Runtime as frame_system::Config>::DbWeight::get().reads(1))
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		// Idempotency check - this core will never be renewable at this timeslice ever again.
		if PotentialRenewals::<Runtime>::get(PotentialRenewalId { core: 6, when: 292605 }).is_none()
		{
			return Ok(Vec::new())
		}
		let sale_info = SaleInfo::<Runtime>::get().unwrap();
		let leases = Leases::<Runtime>::get();
		let pre_upgrade_state = (sale_info, leases);
		Ok(pre_upgrade_state.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		if state.is_empty() {
			return Ok(())
		}
		let (prev_sale_info, prev_leases): (SaleInfoRecordOf<Runtime>, LeasesRecordOf<Runtime>) =
			Decode::decode(&mut &state[..]).unwrap();

		log::info!(target: TARGET, "Checking migration.");

		let sale_info = SaleInfo::<Runtime>::get().unwrap();

		// Check the sale start has not changed.
		assert_eq!(sale_info, prev_sale_info);

		// The workplan entries start from the region begin reported by the new SaleInfo.
		let workplan_start = sale_info.region_begin;

		let system_chains = [Task(1001), Task(1002), Task(1000), Task(1004), Task(1005)];

		// Check the reservations are still in the workplan out of an abundance of caution.
		for (core_id, task) in system_chains.iter().enumerate() {
			assert_eq!(
				Workplan::<Runtime>::get((workplan_start, core_id as u16)),
				Some(Schedule::truncate_from(Vec::from([ScheduleItem {
					mask: CoreMask::complete(),
					assignment: task.clone(),
				}])))
			);
		}

		// Make sure we've got all the leases.
		let leases = Leases::<Runtime>::get();
		assert_eq!(leases.len(), LEASES.iter().filter(|(_, l)| sale_info.region_end <= *l).count());

		// Make something out of the leases that is easier to check against.
		let leases_vec: Vec<(u32, u32)> =
			leases.iter().map(|LeaseRecordItem { until, task }| (*task, *until)).collect();

		// Iterate through hardcoded leases and check they're all correctly in state and scheduled
		// in the workplan.
		for (i, (para_id, until)) in LEASES.iter().enumerate() {
			// Add the system parachains as an offset - these should come before the leases.
			let core_id = i as u16 + 5;

			assert!(leases_vec.contains(&(*until, *para_id)));

			// This is the entry found in Workplan
			let workload = Schedule::truncate_from(Vec::from([ScheduleItem {
				mask: CoreMask::complete(),
				assignment: Task(*para_id),
			}]));

			// They should all be in the workplan for next region.
			assert_eq!(Workplan::<Runtime>::get((workplan_start, core_id)), Some(workload));
		}

		// For the leases we had before their lease should extend for an additional 11520
		// timeslices (64 days).
		for LeaseRecordItem { task, until } in prev_leases.iter() {
			log::error!("{task}, {until}");
			assert!(leases_vec.contains(&(*until + 11520, *task)))
		}

		// Iterate through hardcoded potential renewals and check they're all correctly in state
		// and scheduled in the workplan.
		for (i, (para_id, until)) in POTENTIAL_RENEWALS.iter().enumerate() {
			// Add the system parachains and leases as an offset.
			// system chains + leases + new cores = 5 + 46 + 5 = 56.
			let core_id = i as u16 + 56;

			// This is the entry found in Workplan and PotentialRenewals.
			let workload = Schedule::truncate_from(Vec::from([ScheduleItem {
				mask: CoreMask::complete(),
				assignment: Task(*para_id),
			}]));

			// Make sure they're not in the leases.
			assert!(!leases.contains(&LeaseRecordItem { until: *until, task: *para_id }));

			// Ensure they can renew in the next region.
			assert_eq!(
				PotentialRenewals::<Runtime>::get(PotentialRenewalId {
					core: core_id,
					when: sale_info.region_end + 5040,
				}),
				Some(PotentialRenewalRecord {
					price: 1_000_000_000_000,
					completion: pallet_broker::CompletionStatus::Complete(workload.clone())
				})
			);

			// They should all be in the workplan for next sale.
			assert_eq!(Workplan::<Runtime>::get((workplan_start, core_id)), Some(workload));
		}

		// Walk the workplan at timeslice 287565 and make sure there is an entry for every 62 cores.
		for i in 0..61 {
			let entry = Workplan::<Runtime>::get((287565, i)).expect("Entry should exist");
			assert_eq!(entry.len(), 1);
			assert_eq!(entry.get(0).unwrap().mask, CoreMask::complete());
			if i < 5 {
				// system chains
				assert_eq!(entry.get(0).unwrap().assignment, system_chains[i as usize]);
			} else if i < 51 {
				// leases
				assert_eq!(
					entry.get(0).unwrap().assignment,
					Task(LEASES.get(i as usize - 5).unwrap().0)
				);
			} else if i < 56 {
				// 5 new cores
				assert_eq!(
					entry.get(0).unwrap().assignment,
					Task(LEASES.get(i as usize - 51).unwrap().0)
				);
			} else {
				// 5 potential renewals
				assert_eq!(
					entry.get(0).unwrap().assignment,
					Task(POTENTIAL_RENEWALS.get(i as usize - 56).unwrap().0)
				);
			}
		}

		// Ensure we have requested the correct number of cores.
		assert!(frame_system::Pallet::<Runtime>::read_events_no_consensus().any(|e| {
			match e.event {
				crate::RuntimeEvent::Broker(
					pallet_broker::Event::<Runtime>::CoreCountRequested { core_count },
				) => {
					log::info!("{core_count:?}");

					core_count == 62
				},
				_ => false,
			}
		}));

		Ok(())
	}
}

// Incorrect potential renewals in state
const INCORRECT_RENEWAL_IDS: [(u16, u32); 6] =
	[(6, 292605), (5, 292605), (13, 292605), (15, 292605), (47, 292605), (44, 292605)];

// Hardcoded para ids and their end timeslice.
// Taken from https://github.com/SBalaguer/coretime-migration/blob/master/polkadot-output-200924.json
const POTENTIAL_RENEWALS: [(u32, u32); 5] =
	[(2048, 283680), (3375, 283680), (3358, 283680), (2053, 283680), (2056, 283680)];

const LEASES: [(u32, u32); 46] = [
	(2094, 298800),
	(2040, 298800),
	(2035, 359280),
	(3344, 344160),
	(3370, 389520),
	(3367, 374400),
	(2086, 389520),
	(2032, 359280),
	(3333, 298800),
	(2051, 389520),
	(2106, 298800),
	(3369, 313920),
	(2008, 389520),
	(2025, 359280),
	(2000, 313920),
	(2092, 404640),
	(2002, 359280),
	(3359, 389520),
	(2030, 374400),
	(3378, 404640),
	(2104, 404640),
	(2046, 374400),
	(3345, 344160),
	(3340, 344160),
	(3338, 329040),
	(2004, 329040),
	(3377, 389520),
	(3373, 389520),
	(2031, 344160),
	(3389, 404640),
	(3366, 374400),
	(2037, 374400),
	(2034, 359280),
	(2090, 404640),
	(3346, 344160),
	(2012, 359280),
	(3397, 404640),
	(2043, 374400),
	(2091, 404640),
	(2093, 298800),
	(2026, 344160),
	(3388, 404640),
	(2101, 298800),
	(3354, 359280),
	(2006, 344160),
	(2013, 374400),
];
