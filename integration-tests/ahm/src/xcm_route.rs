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

use asset_hub_polkadot_runtime::{
	xcm_config::XcmRouter as AhXcmRouter, BuildStorage, ParachainSystem as AhParachainSystem,
};
use codec::Encode;
use cumulus_pallet_parachain_system::PendingUpwardMessages;
use cumulus_primitives_core::{send_xcm, UpwardMessageSender};
use pallet_ah_migrator::{
	AhMigrationStage as AhMigrationStageStorage, MigrationStage as AhMigrationStage,
};
use pallet_rc_migrator::{
	MigrationStage as RcMigrationStage, RcMigrationStage as RcMigrationStageStorage,
};
use polkadot_runtime::xcm_config::XcmRouter as RcXcmRouter;
use runtime_parachains::dmp::DownwardMessageQueues;
use sp_runtime::{traits::Dispatchable, AccountId32};
use xcm::prelude::*;

#[test]
fn test_send_to_rc_from_ah() {
	let mut t: sp_io::TestExternalities = frame_system::GenesisConfig::<AhRuntime>::default()
		.build_storage()
		.unwrap()
		.into();

	// our universal xcm message to send to the RC
	let xcm_message = Xcm(vec![
		Instruction::UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
		Instruction::Transact {
			origin_kind: OriginKind::Xcm,
			fallback_max_weight: None,
			call: AhRuntimeCall::System(frame_system::Call::remark { remark: vec![1] })
				.encode()
				.into(),
		},
	]);

	// prepare the AH to send XCM messages to RC and Collectives.
	t.execute_with(|| {
		let now = 1;
		frame_system::Pallet::<AhRuntime>::reset_events();
		frame_system::Pallet::<AhRuntime>::set_block_number(now);

		// setup default XCM version
		let result =
			AhRuntimeCall::PolkadotXcm(pallet_xcm::Call::<AhRuntime>::force_default_xcm_version {
				maybe_xcm_version: Some(xcm::prelude::XCM_VERSION),
			})
			.dispatch(AhRuntimeOrigin::root());

		asset_hub_polkadot_runtime::ParachainSystem::ensure_successful_delivery();

		assert!(result.is_ok(), "fails with error: {:?}", result.err());

		// open the channel between AH and Collectives (1001)
		AhParachainSystem::open_outbound_hrmp_channel_for_benchmarks_or_tests(1001.into());
	});

	// sending XCM messages via main `XcmRouter` from AH to RC and AH to Collectives succeeds
	// while migration is pending.
	t.execute_with(|| {
		let now = 2;
		frame_system::Pallet::<AhRuntime>::reset_events();
		frame_system::Pallet::<AhRuntime>::set_block_number(now);

		AhMigrationStageStorage::<AhRuntime>::put(AhMigrationStage::Pending);

		let dest = Location::parent();
		let result = send_xcm::<AhXcmRouter>(dest, xcm_message.clone());

		assert!(result.is_ok());

		let dest = Location::new(1, Parachain(1001));
		let result = send_xcm::<AhXcmRouter>(dest, xcm_message.clone());

		assert!(result.is_ok(), "fails with error: {:?}", result.err());
	});

	// sending XCM messages via main `XcmRouter` fails from AH to RC but succeeds from AH to
	// Collectives while migration is ongoing.
	t.execute_with(|| {
		let now = 2;
		frame_system::Pallet::<AhRuntime>::reset_events();
		frame_system::Pallet::<AhRuntime>::set_block_number(now);

		AhMigrationStageStorage::<AhRuntime>::put(AhMigrationStage::DataMigrationOngoing);

		let dest = Location::parent();
		let err = send_xcm::<AhXcmRouter>(dest, xcm_message.clone()).unwrap_err();

		assert_eq!(err, SendError::Transport("Migration ongoing - routing is temporary blocked!"));

		let dest = Location::new(1, Parachain(1001));
		let result = send_xcm::<AhXcmRouter>(dest, xcm_message.clone());

		assert!(result.is_ok(), "fails with error: {:?}", result.err());
	});

	// sending XCM messages via main `XcmRouter` from AH to RC and AH to Collectives succeeds
	// while migration is done.
	t.execute_with(|| {
		let now = 2;
		frame_system::Pallet::<AhRuntime>::reset_events();
		frame_system::Pallet::<AhRuntime>::set_block_number(now);

		AhMigrationStageStorage::<AhRuntime>::put(AhMigrationStage::MigrationDone);

		let dest = Location::parent();
		let result = send_xcm::<AhXcmRouter>(dest, xcm_message.clone());

		assert!(result.is_ok(), "fails with error: {:?}", result.err());

		let dest = Location::new(1, Parachain(1001));
		let result = send_xcm::<AhXcmRouter>(dest, xcm_message.clone());

		assert!(result.is_ok(), "fails with error: {:?}", result.err());
	});
}

#[test]
fn test_send_to_ah_from_rc() {
	let mut t: sp_io::TestExternalities = frame_system::GenesisConfig::<RcRuntime>::default()
		.build_storage()
		.unwrap()
		.into();

	// our universal xcm message to send to the RC
	let xcm_message = Xcm(vec![
		Instruction::UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
		Instruction::Transact {
			origin_kind: OriginKind::Xcm,
			fallback_max_weight: None,
			call: RcRuntimeCall::System(frame_system::Call::remark { remark: vec![1] })
				.encode()
				.into(),
		},
	]);

	// prepare the RC to send XCM messages to AH and Collectives.
	t.execute_with(|| {
		let now = 1;
		frame_system::Pallet::<RcRuntime>::reset_events();
		frame_system::Pallet::<RcRuntime>::set_block_number(now);

		// setup default XCM version
		let result =
			RcRuntimeCall::XcmPallet(pallet_xcm::Call::<RcRuntime>::force_default_xcm_version {
				maybe_xcm_version: Some(xcm::prelude::XCM_VERSION),
			})
			.dispatch(RcRuntimeOrigin::root());

		runtime_parachains::configuration::ActiveConfig::<RcRuntime>::mutate(|config| {
			config.max_downward_message_size = 51200;
		});

		polkadot_runtime::Dmp::make_parachain_reachable(1000);
		polkadot_runtime::Dmp::make_parachain_reachable(1001);

		assert!(result.is_ok(), "fails with error: {:?}", result.err());
	});

	// sending XCM messages via main `XcmRouter` from RC to AH and RC to Collectives succeeds
	// while migration is pending.
	t.execute_with(|| {
		let now = 2;
		frame_system::Pallet::<RcRuntime>::reset_events();
		frame_system::Pallet::<RcRuntime>::set_block_number(now);

		RcMigrationStageStorage::<RcRuntime>::put(RcMigrationStage::Pending);

		let dest = Location::new(0, Parachain(1000));
		let result = send_xcm::<RcXcmRouter>(dest, xcm_message.clone());

		assert!(result.is_ok(), "fails with error: {:?}", result.err());

		let dest = Location::new(0, Parachain(1001));
		let result = send_xcm::<RcXcmRouter>(dest, xcm_message.clone());

		assert!(result.is_ok(), "fails with error: {:?}", result.err());
	});

	// sending XCM messages via main `XcmRouter` fails from RC to AH but succeeds from RC to
	// Collectives while migration is ongoing.
	t.execute_with(|| {
		let now = 2;
		frame_system::Pallet::<RcRuntime>::reset_events();
		frame_system::Pallet::<RcRuntime>::set_block_number(now);

		RcMigrationStageStorage::<RcRuntime>::put(RcMigrationStage::AccountsMigrationInit);

		let dest = Location::new(0, Parachain(1000));
		let err = send_xcm::<RcXcmRouter>(dest, xcm_message.clone()).unwrap_err();

		assert_eq!(err, SendError::Transport("Migration ongoing - routing is temporary blocked!"));

		let dest = Location::new(0, Parachain(1001));
		let result = send_xcm::<RcXcmRouter>(dest, xcm_message.clone());

		assert!(result.is_ok(), "fails with error: {:?}", result.err());
	});

	// sending XCM messages via main `XcmRouter` from RC to AH and RC to Collectives succeeds
	// while migration is done.
	t.execute_with(|| {
		let now = 2;
		frame_system::Pallet::<RcRuntime>::reset_events();
		frame_system::Pallet::<RcRuntime>::set_block_number(now);

		RcMigrationStageStorage::<RcRuntime>::put(RcMigrationStage::MigrationDone);

		let dest = Location::new(0, Parachain(1000));
		let result = send_xcm::<RcXcmRouter>(dest, xcm_message.clone());

		assert!(result.is_ok(), "fails with error: {:?}", result.err());

		let dest = Location::new(0, Parachain(1001));
		let result = send_xcm::<RcXcmRouter>(dest, xcm_message.clone());

		assert!(result.is_ok(), "fails with error: {:?}", result.err());
	});
}

#[test]
fn test_send_to_rc_from_ah_via_extrinsic() {
	let mut t: sp_io::TestExternalities = frame_system::GenesisConfig::<AhRuntime>::default()
		.build_storage()
		.unwrap()
		.into();

	let migration_admin: AccountId32 = [1u8; 32].into();

	// our xcm message with send with `pallet_rc_migrator::Pallet::send_xcm_message` extrinsic.
	let xcm_message: VersionedXcm<()> = VersionedXcm::V5(Xcm(vec![
		Instruction::UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
		Instruction::Transact {
			origin_kind: OriginKind::Xcm,
			fallback_max_weight: None,
			call: AhRuntimeCall::System(frame_system::Call::remark_with_event { remark: vec![1] })
				.encode()
				.into(),
		},
	]));

	// prepare the AH to send XCM messages to RC and Collectives.
	t.execute_with(|| {
		let now = 1;
		frame_system::Pallet::<AhRuntime>::reset_events();
		frame_system::Pallet::<AhRuntime>::set_block_number(now);

		// setup default XCM version
		let result =
			AhRuntimeCall::PolkadotXcm(pallet_xcm::Call::<AhRuntime>::force_default_xcm_version {
				maybe_xcm_version: Some(xcm::prelude::XCM_VERSION),
			})
			.dispatch(AhRuntimeOrigin::root());
		assert!(result.is_ok(), "fails with error: {:?}", result.err());

		// open the channel between AH and Collectives (1001)
		AhParachainSystem::open_outbound_hrmp_channel_for_benchmarks_or_tests(1001.into());

		asset_hub_polkadot_runtime::ParachainSystem::ensure_successful_delivery();

		let result =
			AhRuntimeCall::AhMigrator(pallet_ah_migrator::Call::<AhRuntime>::set_manager {
				new: Some(migration_admin.clone()),
			})
			.dispatch(AhRuntimeOrigin::root());

		assert!(result.is_ok(), "fails with error: {:?}", result.err());
	});

	// sending XCM messages via main `XcmRouter` with `pallet_rc_migrator::Pallet::send_xcm_message`
	// extrinsic from AH to RC succeeds while migration is pending.
	t.execute_with(|| {
		let now = 2;
		frame_system::Pallet::<AhRuntime>::reset_events();
		frame_system::Pallet::<AhRuntime>::set_block_number(now);

		AhMigrationStageStorage::<AhRuntime>::put(AhMigrationStage::Pending);

		let origin: AhRuntimeOrigin = AhRuntimeOrigin::signed(migration_admin.clone());

		let result =
			AhRuntimeCall::AhMigrator(pallet_ah_migrator::Call::<AhRuntime>::send_xcm_message {
				dest: Box::new(Location::parent().into()),
				message: Box::new(xcm_message.clone()),
			})
			.dispatch(origin);

		assert!(result.is_ok(), "fails with error: {:?}", result.err());

		let ump_messages = PendingUpwardMessages::<AhRuntime>::take();
		assert_eq!(ump_messages.len(), 1);
	});

	// sending XCM messages succeeds when migration is ongoing.
	t.execute_with(|| {
		let now = 2;
		frame_system::Pallet::<AhRuntime>::reset_events();
		frame_system::Pallet::<AhRuntime>::set_block_number(now);

		AhMigrationStageStorage::<AhRuntime>::put(AhMigrationStage::DataMigrationOngoing);

		let origin: AhRuntimeOrigin = AhRuntimeOrigin::signed(migration_admin.clone());

		let result =
			AhRuntimeCall::AhMigrator(pallet_ah_migrator::Call::<AhRuntime>::send_xcm_message {
				dest: Box::new(Location::parent().into()),
				message: Box::new(xcm_message.clone()),
			})
			.dispatch(origin);

		assert!(result.is_ok(), "fails with error: {:?}", result.err());

		let ump_messages = PendingUpwardMessages::<AhRuntime>::take();
		assert_eq!(ump_messages.len(), 1);
	});
}

#[test]
fn test_send_to_ah_from_rc_via_extrinsic() {
	let mut t: sp_io::TestExternalities = frame_system::GenesisConfig::<RcRuntime>::default()
		.build_storage()
		.unwrap()
		.into();

	let migration_admin: AccountId32 = [1u8; 32].into();

	// our xcm message with send with `pallet_rc_migrator::Pallet::send_xcm_message` extrinsic.
	let xcm_message: VersionedXcm<()> = VersionedXcm::V5(Xcm(vec![
		Instruction::UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
		Instruction::Transact {
			origin_kind: OriginKind::Xcm,
			fallback_max_weight: None,
			call: AhRuntimeCall::System(frame_system::Call::remark_with_event { remark: vec![1] })
				.encode()
				.into(),
		},
	]));

	// prepare the RC to send XCM messages to AH and Collectives.
	t.execute_with(|| {
		let now = 1;
		frame_system::Pallet::<RcRuntime>::reset_events();
		frame_system::Pallet::<RcRuntime>::set_block_number(now);

		// setup default XCM version
		let result =
			RcRuntimeCall::XcmPallet(pallet_xcm::Call::<RcRuntime>::force_default_xcm_version {
				maybe_xcm_version: Some(xcm::prelude::XCM_VERSION),
			})
			.dispatch(RcRuntimeOrigin::root());

		runtime_parachains::configuration::ActiveConfig::<RcRuntime>::mutate(|config| {
			config.max_downward_message_size = 51200;
		});

		polkadot_runtime::Dmp::make_parachain_reachable(1000);
		polkadot_runtime::Dmp::make_parachain_reachable(1001);

		assert!(result.is_ok(), "fails with error: {:?}", result.err());

		let result =
			RcRuntimeCall::RcMigrator(pallet_rc_migrator::Call::<RcRuntime>::set_manager {
				new: Some(migration_admin.clone()),
			})
			.dispatch(RcRuntimeOrigin::root());

		assert!(result.is_ok(), "fails with error: {:?}", result.err());
	});

	// sending XCM messages via main `XcmRouter` with `pallet_rc_migrator::Pallet::send_xcm_message`
	// extrinsic from RC to AH and RC to Collectives succeeds while migration is pending.
	t.execute_with(|| {
		let now = 2;
		frame_system::Pallet::<RcRuntime>::reset_events();
		frame_system::Pallet::<RcRuntime>::set_block_number(now);

		RcMigrationStageStorage::<RcRuntime>::put(RcMigrationStage::Pending);

		let origin: RcRuntimeOrigin = RcRuntimeOrigin::signed(migration_admin.clone());

		let result =
			RcRuntimeCall::RcMigrator(pallet_rc_migrator::Call::<RcRuntime>::send_xcm_message {
				dest: Box::new(Location::new(0, Parachain(1000)).into()),
				message: Box::new(xcm_message.clone()),
			})
			.dispatch(origin);

		assert!(result.is_ok(), "fails with error: {:?}", result.err());

		let dmp_messages = DownwardMessageQueues::<RcRuntime>::take(1000);
		assert_eq!(dmp_messages.len(), 1);
	});

	// sending XCM messages succeeds when migration is ongoing.
	t.execute_with(|| {
		let now = 3;
		frame_system::Pallet::<RcRuntime>::reset_events();
		frame_system::Pallet::<RcRuntime>::set_block_number(now);

		RcMigrationStageStorage::<RcRuntime>::put(RcMigrationStage::AccountsMigrationInit);

		let origin: RcRuntimeOrigin = RcRuntimeOrigin::signed(migration_admin.clone());

		let result =
			RcRuntimeCall::RcMigrator(pallet_rc_migrator::Call::<RcRuntime>::send_xcm_message {
				dest: Box::new(Location::new(0, Parachain(1000)).into()),
				message: Box::new(xcm_message.clone()),
			})
			.dispatch(origin);

		assert!(result.is_ok(), "fails with error: {:?}", result.err());

		let dmp_messages = DownwardMessageQueues::<RcRuntime>::take(1000);
		assert_eq!(dmp_messages.len(), 1);
	});
}
