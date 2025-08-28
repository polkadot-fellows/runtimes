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

use crate::porting_prelude::*;

use asset_hub_polkadot_runtime::{AhMigrator, BuildStorage};
use frame_support::{
	traits::{DefensiveTruncateFrom, OnFinalize},
	BoundedSlice,
};
use pallet_ah_migrator::{
	AhMigrationStage, DmpQueuePriority, DmpQueuePriorityConfig,
	Event::DmpQueuePrioritySet as DmpQueuePrioritySetEvent, MigrationStage,
};
use pallet_rc_migrator::{
	AhUmpQueuePriority, AhUmpQueuePriorityConfig,
	Event::AhUmpQueuePrioritySet as AhUmpQueuePrioritySetEvent, MigrationStage as RcMigrationStage,
	RcMigrationStage as RcMigrationStageStorage,
};
use polkadot_runtime::RcMigrator;
use xcm_emulator::EnqueueMessage;

#[test]
fn test_force_dmp_queue_priority() {
	let mut t: sp_io::TestExternalities = frame_system::GenesisConfig::<AhRuntime>::default()
		.build_storage()
		.unwrap()
		.into();

	// prioritization is not even attempted if the migration is not ongoing
	t.execute_with(|| {
		use asset_hub_polkadot_runtime::MessageQueue;
		use cumulus_primitives_core::AggregateMessageOrigin;

		MessageQueue::enqueue_message(
			BoundedSlice::defensive_truncate_from(&[1]),
			AggregateMessageOrigin::Parent,
		);
	});

	// prioritization is not even attempted if the migration is not ongoing
	t.execute_with(|| {
		let now = 1;
		frame_system::Pallet::<AhRuntime>::reset_events();
		frame_system::Pallet::<AhRuntime>::set_block_number(now);

		AhMigrationStage::<AhRuntime>::put(MigrationStage::Pending);
		<AhMigrator as OnFinalize<_>>::on_finalize(now);

		let events = frame_system::Pallet::<AhRuntime>::events();
		assert!(!events.iter().any(|record| {
			matches!(record.event, AhRuntimeEvent::AhMigrator(DmpQueuePrioritySetEvent { .. }))
		}));
	});

	// prioritization with default config setup is attempted if the migration is ongoing
	t.execute_with(|| {
		let now = 1;
		frame_system::Pallet::<AhRuntime>::reset_events();
		frame_system::Pallet::<AhRuntime>::set_block_number(now);

		AhMigrationStage::<AhRuntime>::put(MigrationStage::DataMigrationOngoing);
		<AhMigrator as OnFinalize<_>>::on_finalize(now);

		let events = frame_system::Pallet::<AhRuntime>::events();
		assert!(events.iter().any(|record| {
			matches!(record.event, AhRuntimeEvent::AhMigrator(DmpQueuePrioritySetEvent { .. }))
		}));
	});

	// prioritization is disabled
	t.execute_with(|| {
		let now = 1;
		frame_system::Pallet::<AhRuntime>::reset_events();
		frame_system::Pallet::<AhRuntime>::set_block_number(now);

		AhMigrationStage::<AhRuntime>::put(MigrationStage::DataMigrationOngoing);
		DmpQueuePriorityConfig::<AhRuntime>::put(DmpQueuePriority::Disabled);
		<AhMigrator as OnFinalize<_>>::on_finalize(now);

		let events = frame_system::Pallet::<AhRuntime>::events();
		assert!(!events.iter().any(|record| {
			matches!(record.event, AhRuntimeEvent::AhMigrator(DmpQueuePrioritySetEvent { .. }))
		}));
	});

	// prioritization with (10, 2) pattern

	t.execute_with(|| {
		let now = 11;
		frame_system::Pallet::<AhRuntime>::reset_events();
		frame_system::Pallet::<AhRuntime>::set_block_number(now);

		AhMigrationStage::<AhRuntime>::put(MigrationStage::DataMigrationOngoing);
		DmpQueuePriorityConfig::<AhRuntime>::put(DmpQueuePriority::OverrideConfig(10, 2));
		<AhMigrator as OnFinalize<_>>::on_finalize(now);

		frame_system::Pallet::<AhRuntime>::assert_has_event(AhRuntimeEvent::AhMigrator(
			DmpQueuePrioritySetEvent { prioritized: false, cycle_block: 12, cycle_period: 12 },
		));
	});

	t.execute_with(|| {
		let now = 12;
		frame_system::Pallet::<AhRuntime>::reset_events();
		frame_system::Pallet::<AhRuntime>::set_block_number(now);

		AhMigrationStage::<AhRuntime>::put(MigrationStage::DataMigrationOngoing);
		DmpQueuePriorityConfig::<AhRuntime>::put(DmpQueuePriority::OverrideConfig(10, 2));
		<AhMigrator as OnFinalize<_>>::on_finalize(now);

		frame_system::Pallet::<AhRuntime>::assert_has_event(AhRuntimeEvent::AhMigrator(
			DmpQueuePrioritySetEvent { prioritized: true, cycle_block: 1, cycle_period: 12 },
		));
	});

	t.execute_with(|| {
		let now = 13;
		frame_system::Pallet::<AhRuntime>::reset_events();
		frame_system::Pallet::<AhRuntime>::set_block_number(now);

		AhMigrationStage::<AhRuntime>::put(MigrationStage::DataMigrationOngoing);
		DmpQueuePriorityConfig::<AhRuntime>::put(DmpQueuePriority::OverrideConfig(10, 2));
		<AhMigrator as OnFinalize<_>>::on_finalize(now);

		frame_system::Pallet::<AhRuntime>::assert_has_event(AhRuntimeEvent::AhMigrator(
			DmpQueuePrioritySetEvent { prioritized: true, cycle_block: 2, cycle_period: 12 },
		));
	});

	t.execute_with(|| {
		let now = 21;
		frame_system::Pallet::<AhRuntime>::reset_events();
		frame_system::Pallet::<AhRuntime>::set_block_number(now);

		AhMigrationStage::<AhRuntime>::put(MigrationStage::DataMigrationOngoing);
		DmpQueuePriorityConfig::<AhRuntime>::put(DmpQueuePriority::OverrideConfig(10, 2));
		<AhMigrator as OnFinalize<_>>::on_finalize(now);

		frame_system::Pallet::<AhRuntime>::assert_has_event(AhRuntimeEvent::AhMigrator(
			DmpQueuePrioritySetEvent { prioritized: true, cycle_block: 10, cycle_period: 12 },
		));
	});

	t.execute_with(|| {
		let now = 22;
		frame_system::Pallet::<AhRuntime>::reset_events();
		frame_system::Pallet::<AhRuntime>::set_block_number(now);

		AhMigrationStage::<AhRuntime>::put(MigrationStage::DataMigrationOngoing);
		DmpQueuePriorityConfig::<AhRuntime>::put(DmpQueuePriority::OverrideConfig(10, 2));
		<AhMigrator as OnFinalize<_>>::on_finalize(now);

		frame_system::Pallet::<AhRuntime>::assert_has_event(AhRuntimeEvent::AhMigrator(
			DmpQueuePrioritySetEvent { prioritized: false, cycle_block: 11, cycle_period: 12 },
		));
	});

	t.execute_with(|| {
		let now = 23;
		frame_system::Pallet::<AhRuntime>::reset_events();
		frame_system::Pallet::<AhRuntime>::set_block_number(now);

		AhMigrationStage::<AhRuntime>::put(MigrationStage::DataMigrationOngoing);
		DmpQueuePriorityConfig::<AhRuntime>::put(DmpQueuePriority::OverrideConfig(10, 2));
		<AhMigrator as OnFinalize<_>>::on_finalize(now);

		frame_system::Pallet::<AhRuntime>::assert_has_event(AhRuntimeEvent::AhMigrator(
			DmpQueuePrioritySetEvent { prioritized: false, cycle_block: 12, cycle_period: 12 },
		));
	});

	t.execute_with(|| {
		let now = 24;
		frame_system::Pallet::<AhRuntime>::reset_events();
		frame_system::Pallet::<AhRuntime>::set_block_number(now);

		AhMigrationStage::<AhRuntime>::put(MigrationStage::DataMigrationOngoing);
		DmpQueuePriorityConfig::<AhRuntime>::put(DmpQueuePriority::OverrideConfig(10, 2));
		<AhMigrator as OnFinalize<_>>::on_finalize(now);

		frame_system::Pallet::<AhRuntime>::assert_has_event(AhRuntimeEvent::AhMigrator(
			DmpQueuePrioritySetEvent { prioritized: true, cycle_block: 1, cycle_period: 12 },
		));
	});
}

#[test]
fn test_force_ah_ump_queue_priority() {
	let mut t: sp_io::TestExternalities = frame_system::GenesisConfig::<RcRuntime>::default()
		.build_storage()
		.unwrap()
		.into();

	t.execute_with(|| {
		use polkadot_runtime::MessageQueue;
		use runtime_parachains::inclusion::{AggregateMessageOrigin, UmpQueueId};

		MessageQueue::enqueue_message(
			BoundedSlice::defensive_truncate_from(&[1]),
			AggregateMessageOrigin::Ump(UmpQueueId::Para(1000.into())),
		);
	});

	// prioritization is not even attempted if the migration is not ongoing
	t.execute_with(|| {
		let now = 1;
		frame_system::Pallet::<RcRuntime>::reset_events();
		frame_system::Pallet::<RcRuntime>::set_block_number(now);

		RcMigrationStageStorage::<RcRuntime>::put(RcMigrationStage::Pending);
		<RcMigrator as OnFinalize<_>>::on_finalize(now);

		let events = frame_system::Pallet::<RcRuntime>::events();
		assert!(!events.iter().any(|record| {
			matches!(record.event, RcRuntimeEvent::RcMigrator(AhUmpQueuePrioritySetEvent { .. }))
		}));
	});

	// prioritization with default config setup is attempted if the migration is ongoing
	t.execute_with(|| {
		let now = 1;
		frame_system::Pallet::<RcRuntime>::reset_events();
		frame_system::Pallet::<RcRuntime>::set_block_number(now);

		RcMigrationStageStorage::<RcRuntime>::put(RcMigrationStage::AccountsMigrationInit);
		<RcMigrator as OnFinalize<_>>::on_finalize(now);

		let events = frame_system::Pallet::<RcRuntime>::events();
		assert!(events.iter().any(|record| {
			matches!(record.event, RcRuntimeEvent::RcMigrator(AhUmpQueuePrioritySetEvent { .. }))
		}));
	});

	// prioritization is disabled
	t.execute_with(|| {
		let now = 1;
		frame_system::Pallet::<RcRuntime>::reset_events();
		frame_system::Pallet::<RcRuntime>::set_block_number(now);

		RcMigrationStageStorage::<RcRuntime>::put(RcMigrationStage::AccountsMigrationInit);
		AhUmpQueuePriorityConfig::<RcRuntime>::put(AhUmpQueuePriority::Disabled);
		<RcMigrator as OnFinalize<_>>::on_finalize(now);

		let events = frame_system::Pallet::<RcRuntime>::events();
		assert!(!events.iter().any(|record| {
			matches!(record.event, RcRuntimeEvent::RcMigrator(AhUmpQueuePrioritySetEvent { .. }))
		}));
	});

	// prioritization with (10, 2) pattern

	t.execute_with(|| {
		let now = 11;
		frame_system::Pallet::<RcRuntime>::reset_events();
		frame_system::Pallet::<RcRuntime>::set_block_number(now);

		RcMigrationStageStorage::<RcRuntime>::put(RcMigrationStage::AccountsMigrationInit);
		AhUmpQueuePriorityConfig::<RcRuntime>::put(AhUmpQueuePriority::OverrideConfig(10, 2));
		<RcMigrator as OnFinalize<_>>::on_finalize(now);

		frame_system::Pallet::<RcRuntime>::assert_has_event(RcRuntimeEvent::RcMigrator(
			AhUmpQueuePrioritySetEvent { prioritized: false, cycle_block: 12, cycle_period: 12 },
		));
	});

	t.execute_with(|| {
		let now = 12;
		frame_system::Pallet::<RcRuntime>::reset_events();
		frame_system::Pallet::<RcRuntime>::set_block_number(now);

		RcMigrationStageStorage::<RcRuntime>::put(RcMigrationStage::AccountsMigrationInit);
		AhUmpQueuePriorityConfig::<RcRuntime>::put(AhUmpQueuePriority::OverrideConfig(10, 2));
		<RcMigrator as OnFinalize<_>>::on_finalize(now);

		frame_system::Pallet::<RcRuntime>::assert_has_event(RcRuntimeEvent::RcMigrator(
			AhUmpQueuePrioritySetEvent { prioritized: true, cycle_block: 1, cycle_period: 12 },
		));
	});

	t.execute_with(|| {
		let now = 13;
		frame_system::Pallet::<RcRuntime>::reset_events();
		frame_system::Pallet::<RcRuntime>::set_block_number(now);

		RcMigrationStageStorage::<RcRuntime>::put(RcMigrationStage::AccountsMigrationInit);
		AhUmpQueuePriorityConfig::<RcRuntime>::put(AhUmpQueuePriority::OverrideConfig(10, 2));
		<RcMigrator as OnFinalize<_>>::on_finalize(now);

		frame_system::Pallet::<RcRuntime>::assert_has_event(RcRuntimeEvent::RcMigrator(
			AhUmpQueuePrioritySetEvent { prioritized: true, cycle_block: 2, cycle_period: 12 },
		));
	});

	t.execute_with(|| {
		let now = 21;
		frame_system::Pallet::<RcRuntime>::reset_events();
		frame_system::Pallet::<RcRuntime>::set_block_number(now);

		RcMigrationStageStorage::<RcRuntime>::put(RcMigrationStage::AccountsMigrationInit);
		AhUmpQueuePriorityConfig::<RcRuntime>::put(AhUmpQueuePriority::OverrideConfig(10, 2));
		<RcMigrator as OnFinalize<_>>::on_finalize(now);

		frame_system::Pallet::<RcRuntime>::assert_has_event(RcRuntimeEvent::RcMigrator(
			AhUmpQueuePrioritySetEvent { prioritized: true, cycle_block: 10, cycle_period: 12 },
		));
	});

	t.execute_with(|| {
		let now = 22;
		frame_system::Pallet::<RcRuntime>::reset_events();
		frame_system::Pallet::<RcRuntime>::set_block_number(now);

		RcMigrationStageStorage::<RcRuntime>::put(RcMigrationStage::AccountsMigrationInit);
		AhUmpQueuePriorityConfig::<RcRuntime>::put(AhUmpQueuePriority::OverrideConfig(10, 2));
		<RcMigrator as OnFinalize<_>>::on_finalize(now);

		frame_system::Pallet::<RcRuntime>::assert_has_event(RcRuntimeEvent::RcMigrator(
			AhUmpQueuePrioritySetEvent { prioritized: false, cycle_block: 11, cycle_period: 12 },
		));
	});

	t.execute_with(|| {
		let now = 23;
		frame_system::Pallet::<RcRuntime>::reset_events();
		frame_system::Pallet::<RcRuntime>::set_block_number(now);

		RcMigrationStageStorage::<RcRuntime>::put(RcMigrationStage::AccountsMigrationInit);
		AhUmpQueuePriorityConfig::<RcRuntime>::put(AhUmpQueuePriority::OverrideConfig(10, 2));
		<RcMigrator as OnFinalize<_>>::on_finalize(now);

		frame_system::Pallet::<RcRuntime>::assert_has_event(RcRuntimeEvent::RcMigrator(
			AhUmpQueuePrioritySetEvent { prioritized: false, cycle_block: 12, cycle_period: 12 },
		));
	});

	t.execute_with(|| {
		let now = 24;
		frame_system::Pallet::<RcRuntime>::reset_events();
		frame_system::Pallet::<RcRuntime>::set_block_number(now);

		RcMigrationStageStorage::<RcRuntime>::put(RcMigrationStage::AccountsMigrationInit);
		AhUmpQueuePriorityConfig::<RcRuntime>::put(AhUmpQueuePriority::OverrideConfig(10, 2));
		<RcMigrator as OnFinalize<_>>::on_finalize(now);

		frame_system::Pallet::<RcRuntime>::assert_has_event(RcRuntimeEvent::RcMigrator(
			AhUmpQueuePrioritySetEvent { prioritized: true, cycle_block: 1, cycle_period: 12 },
		));
	});
}
