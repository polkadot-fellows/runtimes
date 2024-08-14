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
use pallet_broker::{ConfigRecord, Configuration, CoreAssignment, CoreMask, ScheduleItem};
use polkadot_runtime_constants::system_parachain::coretime::TIMESLICE_PERIOD;
use sp_runtime::Perbill;

#[test]
fn transact_hardcoded_weights_are_sane() {
	// There are three transacts with hardcoded weights sent from the Coretime Chain to the Relay
	// Chain across the CoretimeInterface which are triggered at various points in the sales cycle.
	// - Request core count - triggered directly by `start_sales` or `request_core_count`
	//   extrinsics.
	// - Request revenue info - triggered when each timeslice is committed.
	// - Assign core - triggered when an entry is encountered in the workplan for the next
	//   timeslice.

	// RuntimeEvent aliases to avoid warning from usage of qualified paths in assertions due to
	// <https://github.com/rust-lang/rust/issues/86935>
	type CoretimeEvent = <CoretimePolkadot as Chain>::RuntimeEvent;
	type RelayEvent = <Polkadot as Chain>::RuntimeEvent;

	// Reserve a workload, configure broker and start sales.
	CoretimePolkadot::execute_with(|| {
		// Hooks don't run in emulated tests - workaround as we need `on_initialize` to tick things
		// along and have no concept of time passing otherwise.
		<CoretimePolkadot as CoretimePolkadotPallet>::Broker::on_initialize(
			<CoretimePolkadot as Chain>::System::block_number(),
		);

		let coretime_root_origin = <CoretimePolkadot as Chain>::RuntimeOrigin::root();

		// Create and populate schedule with some assignments on this core.
		let mut schedule = Vec::new();
		for i in 0..10 {
			schedule.push(ScheduleItem {
				mask: CoreMask::void().set(i),
				assignment: CoreAssignment::Task(2000 + i),
			})
		}

		assert_ok!(<CoretimePolkadot as CoretimePolkadotPallet>::Broker::reserve(
			coretime_root_origin.clone(),
			schedule.try_into().expect("Vector is within bounds."),
		));

		// Configure broker and start sales.
		let config = ConfigRecord {
			advance_notice: 1,
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

	// Check that the request_core_count message was processed successfully. This will fail if the
	// weights are misconfigured.
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

	// Keep track of the relay chain block number so we can fast forward while still checking the
	// right block.
	let mut block_number_cursor = Polkadot::ext_wrapper(<Polkadot as Chain>::System::block_number);

	let config = CoretimePolkadot::ext_wrapper(|| {
		Configuration::<<CoretimePolkadot as Chain>::Runtime>::get()
			.expect("Pallet was configured earlier.")
	});

	// Now run up to the block before the sale is rotated.
	while block_number_cursor < TIMESLICE_PERIOD - config.advance_notice - 1 {
		CoretimePolkadot::execute_with(|| {
			// Hooks don't run in emulated tests - workaround.
			<CoretimePolkadot as CoretimePolkadotPallet>::Broker::on_initialize(
				<CoretimePolkadot as Chain>::System::block_number(),
			);
		});

		Polkadot::ext_wrapper(|| {
			block_number_cursor = <Polkadot as Chain>::System::block_number();
		});

		dbg!(&block_number_cursor);
	}

	// In this block we trigger assign core.
	CoretimePolkadot::execute_with(|| {
		// Hooks don't run in emulated tests - workaround.
		<CoretimePolkadot as CoretimePolkadotPallet>::Broker::on_initialize(
			<CoretimePolkadot as Chain>::System::block_number(),
		);

		assert_expected_events!(
			CoretimePolkadot,
			vec![
				CoretimeEvent::Broker(
					pallet_broker::Event::SaleInitialized { .. }
				) => {},
				CoretimeEvent::Broker(
					pallet_broker::Event::CoreAssigned { .. }
				) => {},
				CoretimeEvent::ParachainSystem(
					cumulus_pallet_parachain_system::Event::UpwardMessageSent { .. }
				) => {},
			]
		);
	});

	// In this block we trigger request revenue.
	CoretimePolkadot::execute_with(|| {
		// Hooks don't run in emulated tests - workaround.
		<CoretimePolkadot as CoretimePolkadotPallet>::Broker::on_initialize(
			<CoretimePolkadot as Chain>::System::block_number(),
		);

		assert_expected_events!(
			CoretimePolkadot,
			vec![
				CoretimeEvent::ParachainSystem(
					cumulus_pallet_parachain_system::Event::UpwardMessageSent { .. }
				) => {},
			]
		);
	});

	// Check that the assign_core and request_revenue_info_at messages were processed successfully.
	// This will fail if the weights are misconfigured.
	Polkadot::execute_with(|| {
		Polkadot::assert_ump_queue_processed(true, Some(CoretimePolkadot::para_id()), None);

		assert_expected_events!(
			Polkadot,
			vec![
				RelayEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
				RelayEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
				RelayEvent::Coretime(
					runtime_parachains::coretime::Event::CoreAssigned { .. }
				) => {},
			]
		);
	});
}
