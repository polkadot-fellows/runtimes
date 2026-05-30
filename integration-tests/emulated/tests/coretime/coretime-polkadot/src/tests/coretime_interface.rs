// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::*;
use frame_support::traits::OnInitialize;
use pallet_broker::{ConfigRecord, CoreAssignment, CoreMask, ScheduleItem};
use polkadot_runtime::Dmp;
use polkadot_runtime_constants::system_parachain::coretime::TIMESLICE_PERIOD;
use sp_runtime::Perbill;

#[test]
fn broker_transacts_are_processed_by_relay() {
	type CoretimeEvent = <CoretimePolkadot as Chain>::RuntimeEvent;
	type RelayEvent = <Polkadot as Chain>::RuntimeEvent;

	Polkadot::execute_with(|| {
		Dmp::make_parachain_reachable(CoretimePolkadot::para_id());
	});

	CoretimePolkadot::execute_with(|| {
		// Hooks don't run in emulated tests; tick the broker manually here and inside the loop
		// below so `do_tick` runs with the current relay block.
		<CoretimePolkadot as CoretimePolkadotPallet>::Broker::on_initialize(
			<CoretimePolkadot as Chain>::System::block_number(),
		);

		let coretime_root_origin = <CoretimePolkadot as Chain>::RuntimeOrigin::root();

		let mut schedule = Vec::new();
		for i in 0..80 {
			schedule.push(ScheduleItem {
				mask: CoreMask::void().set(i),
				assignment: CoreAssignment::Task(2000 + i),
			})
		}

		assert_ok!(<CoretimePolkadot as CoretimePolkadotPallet>::Broker::reserve(
			coretime_root_origin.clone(),
			schedule.try_into().expect("Vector is within bounds."),
		));

		let config = ConfigRecord {
			advance_notice: 2,
			interlude_length: 1,
			leadin_length: 2,
			region_length: 1,
			ideal_bulk_proportion: Perbill::from_percent(40),
			limit_cores_offered: None,
			renewal_bump: Perbill::from_percent(2),
			contribution_timeout: 1,
		};
		assert_ok!(<CoretimePolkadot as CoretimePolkadotPallet>::Broker::configure(
			coretime_root_origin.clone(),
			config
		));
		assert_ok!(<CoretimePolkadot as CoretimePolkadotPallet>::Broker::start_sales(
			coretime_root_origin,
			100,
			0
		));
		assert_eq!(
			pallet_broker::Status::<<CoretimePolkadot as Chain>::Runtime>::get()
				.unwrap()
				.core_count,
			1
		);

		assert_expected_events!(
			CoretimePolkadot,
			vec![
				CoretimeEvent::Broker(
					pallet_broker::Event::ReservationMade { .. }
				) => {},
				CoretimeEvent::Broker(
					pallet_broker::Event::CoreCountRequested { core_count: 1 }
				) => {},
				CoretimeEvent::ParachainSystem(
					cumulus_pallet_parachain_system::Event::UpwardMessageSent { .. }
				) => {},
			]
		);
	});

	Polkadot::execute_with(|| {
		Polkadot::assert_ump_queue_processed(true, Some(CoretimePolkadot::para_id()), None);

		assert_expected_events!(
			Polkadot,
			vec![
				RelayEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let mut block_number_cursor = Polkadot::ext_wrapper(<Polkadot as Chain>::System::block_number);

	let mut found_sale_initialized = false;
	let mut found_core_assigned = false;
	let mut found_history_dropped = false;
	let mut found_relay_core_assigned = false;
	let mut relay_ump_processed = 0u32;
	// `HistoryDropped` is the terminal event of the round-trip, so it implies all earlier
	// broker/relay steps have already fired in prior iterations.
	while !found_history_dropped && block_number_cursor < TIMESLICE_PERIOD * 100 {
		CoretimePolkadot::execute_with(|| {
			<CoretimePolkadot as CoretimePolkadotPallet>::Broker::on_initialize(
				<CoretimePolkadot as Chain>::System::block_number(),
			);

			for event in &<CoretimePolkadot as Chain>::System::events() {
				match &event.event {
					CoretimeEvent::Broker(pallet_broker::Event::SaleInitialized { .. }) =>
						found_sale_initialized = true,
					CoretimeEvent::Broker(pallet_broker::Event::CoreAssigned { .. }) =>
						found_core_assigned = true,
					CoretimeEvent::Broker(pallet_broker::Event::HistoryDropped {
						when: 0,
						revenue: 0,
					}) => found_history_dropped = true,
					_ => {},
				}
			}
		});

		// `Polkadot::execute_with` (not `ext_wrapper`) is required: the relay's outgoing DMPs
		// only get flushed into the emulator's downward queue from within a relay `execute_with`,
		// and that's the path by which `notify_revenue` reaches the broker.
		Polkadot::execute_with(|| {
			for event in &<Polkadot as Chain>::System::events() {
				match &event.event {
					RelayEvent::MessageQueue(pallet_message_queue::Event::Processed {
						success: true,
						..
					}) => relay_ump_processed += 1,
					RelayEvent::Coretime(runtime_parachains::coretime::Event::CoreAssigned {
						..
					}) => found_relay_core_assigned = true,
					_ => {},
				}
			}

			block_number_cursor = <Polkadot as Chain>::System::block_number();
		});
	}
	assert!(found_sale_initialized, "broker never emitted `SaleInitialized`");
	assert!(found_core_assigned, "broker never emitted `CoreAssigned`");
	assert!(
		found_history_dropped,
		"broker never emitted `HistoryDropped` (revenue round-trip did not complete)",
	);
	assert!(
		relay_ump_processed >= 2,
		"relay processed fewer UMPs than expected: got {relay_ump_processed}",
	);
	assert!(
		found_relay_core_assigned,
		"relay never emitted `coretime::CoreAssigned` (assign_core dispatch failed)",
	);
}
