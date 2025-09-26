// This file is part of Substrate.

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
use frame_benchmarking::v2::*;
use frame_support::traits::Currency;
use frame_system::{Account as SystemAccount, RawOrigin};
use runtime_parachains::dmp as parachains_dmp;

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

#[benchmarks]
pub mod benchmarks {
	use super::*;

	#[benchmark]
	fn withdraw_account() {
		let create_liquid_account = |n: u8| {
			let who: AccountId32 = [n; 32].into();
			let ed = <pallet_balances::Pallet<T> as Currency<_>>::minimum_balance();
			let _ = <pallet_balances::Pallet<T> as Currency<_>>::deposit_creating(&who, ed);
		};

		let n = 50;
		(0..n).for_each(create_liquid_account);
		let last_key: AccountId32 = [n / 2; 32].into();

		RcMigratedBalance::<T>::mutate(|tracker| {
			tracker.kept = <<T as Config>::Currency as Currency<_>>::total_issuance();
		});

		#[block]
		{
			let (who, account_info) = SystemAccount::<T>::iter_from_key(last_key).next().unwrap();
			let mut ah_weight = WeightMeter::new();
			let batch_len = 0;
			let res = AccountsMigrator::<T>::withdraw_account(
				who,
				account_info,
				&mut ah_weight,
				batch_len,
			);
			assert!(res.unwrap().is_some());
		}
	}

	#[benchmark]
	fn force_set_stage() {
		let stage = MigrationStageOf::<T>::Scheduled { start: 1u32.into() };

		#[extrinsic_call]
		_(RawOrigin::Root, Box::new(stage.clone()));

		assert_last_event::<T>(
			Event::StageTransition { old: MigrationStageOf::<T>::Pending, new: stage }.into(),
		);
	}

	#[benchmark]
	fn schedule_migration() {
		let start = DispatchTime::<BlockNumberFor<T>>::At(10u32.into());
		let warm_up = DispatchTime::<BlockNumberFor<T>>::At(20u32.into());
		let cool_off = DispatchTime::<BlockNumberFor<T>>::After(20u32.into());

		#[extrinsic_call]
		_(RawOrigin::Root, start, warm_up, cool_off, true);

		assert_last_event::<T>(
			Event::StageTransition {
				old: MigrationStageOf::<T>::Pending,
				new: MigrationStageOf::<T>::Scheduled { start: 10u32.into() },
			}
			.into(),
		);
	}

	#[benchmark]
	fn start_data_migration() {
		let now = frame_system::Pallet::<T>::block_number();
		let warm_up = DispatchTime::<BlockNumberFor<T>>::At(200u32.into());
		WarmUpPeriod::<T>::put(warm_up);
		let initial_stage = MigrationStageOf::<T>::WaitingForAh;
		RcMigrationStage::<T>::put(&initial_stage);

		#[extrinsic_call]
		_(RawOrigin::Root);

		assert_last_event::<T>(
			Event::StageTransition {
				old: initial_stage,
				new: MigrationStageOf::<T>::WarmUp { end_at: warm_up.evaluate(now) },
			}
			.into(),
		);
	}

	#[benchmark]
	fn send_chunked_xcm_and_track() {
		let mut batches = XcmBatch::new();
		batches.push(vec![0u8; (MAX_XCM_SIZE / 2 - 10) as usize]);
		batches.push(vec![1u8; (MAX_XCM_SIZE / 2 - 10) as usize]);
		parachains_dmp::Pallet::<T>::make_parachain_reachable(1000);

		#[block]
		{
			let res =
				Pallet::<T>::send_chunked_xcm_and_track(batches, |batch| types::AhMigratorCall::<
					T,
				>::TestCall {
					data: batch,
				});
			assert_eq!(res.unwrap(), 1);
		}
	}

	#[benchmark]
	fn receive_query_response() {
		let query_id = 1;
		let xcm = Xcm(vec![Instruction::UnpaidExecution {
			weight_limit: WeightLimit::Unlimited,
			check_origin: None,
		}]);
		let message_hash = T::Hashing::hash_of(&xcm);
		PendingXcmMessages::<T>::insert(message_hash, xcm);
		PendingXcmQueries::<T>::insert(query_id, message_hash);

		let maybe_error = MaybeErrorCode::Success;
		let response = Response::DispatchResult(maybe_error.clone());

		#[extrinsic_call]
		_(RawOrigin::Root, query_id, response);

		assert!(PendingXcmMessages::<T>::get(message_hash).is_none());
		assert_last_event::<T>(
			Event::QueryResponseReceived { query_id, response: maybe_error }.into(),
		);
	}

	#[benchmark]
	fn resend_xcm() {
		let query_id = 10;
		let next_query_id = 0;
		let xcm = Xcm(vec![Instruction::UnpaidExecution {
			weight_limit: WeightLimit::Unlimited,
			check_origin: None,
		}]);
		let message_hash = T::Hashing::hash_of(&xcm);
		PendingXcmMessages::<T>::insert(message_hash, xcm);
		PendingXcmQueries::<T>::insert(query_id, message_hash);
		parachains_dmp::Pallet::<T>::make_parachain_reachable(1000);

		#[extrinsic_call]
		_(RawOrigin::Root, query_id);

		assert!(PendingXcmMessages::<T>::get(message_hash).is_some());
		assert!(PendingXcmQueries::<T>::get(query_id).is_some());
		assert!(PendingXcmQueries::<T>::get(next_query_id).is_some());
		assert_last_event::<T>(
			Event::XcmResendAttempt { query_id: next_query_id, send_error: None }.into(),
		);
	}

	#[benchmark]
	fn set_unprocessed_msg_buffer() {
		let old = Pallet::<T>::get_unprocessed_msg_buffer_size();
		let size = 111u32;
		#[extrinsic_call]
		_(RawOrigin::Root, Some(size));

		let new = Pallet::<T>::get_unprocessed_msg_buffer_size();
		assert_eq!(new, size);
		assert_last_event::<T>(Event::UnprocessedMsgBufferSet { new: size, old }.into());
	}

	#[benchmark]
	fn force_ah_ump_queue_priority() {
		use frame_support::BoundedSlice;

		T::MessageQueue::enqueue_message(
			BoundedSlice::defensive_truncate_from(&[1]),
			AggregateMessageOrigin::Ump(UmpQueueId::Para(1000.into())),
		);
		let now = BlockNumberFor::<T>::from(1u32);
		let priority_blocks = BlockNumberFor::<T>::from(10u32);
		let round_robin_blocks = BlockNumberFor::<T>::from(1u32);
		AhUmpQueuePriorityConfig::<T>::put(AhUmpQueuePriority::OverrideConfig(
			priority_blocks,
			round_robin_blocks,
		));

		#[block]
		{
			Pallet::<T>::force_ah_ump_queue_priority(now)
		}

		assert_last_event::<T>(
			Event::AhUmpQueuePrioritySet {
				prioritized: true,
				cycle_block: now + BlockNumberFor::<T>::from(1u32),
				cycle_period: priority_blocks + round_robin_blocks,
			}
			.into(),
		);
	}

	#[benchmark]
	fn set_ah_ump_queue_priority() {
		let old = AhUmpQueuePriorityConfig::<T>::get();
		let new = AhUmpQueuePriority::OverrideConfig(
			BlockNumberFor::<T>::from(10u32),
			BlockNumberFor::<T>::from(1u32),
		);
		#[extrinsic_call]
		_(RawOrigin::Root, new.clone());

		assert_last_event::<T>(Event::AhUmpQueuePriorityConfigSet { old, new }.into());
	}

	#[benchmark]
	fn set_manager() {
		let old = Manager::<T>::get();
		let new = Some([0; 32].into());
		#[extrinsic_call]
		_(RawOrigin::Root, new.clone());

		assert_last_event::<T>(Event::ManagerSet { old, new }.into());
	}

	#[cfg(feature = "std")]
	pub fn test_withdraw_account<T: Config>() {
		_withdraw_account::<T>(true /* enable checks */)
	}

	#[cfg(feature = "std")]
	pub fn test_force_set_stage<T: Config>() {
		_force_set_stage::<T>(true /* enable checks */);
	}

	#[cfg(feature = "std")]
	pub fn test_schedule_migration<T: Config>() {
		_schedule_migration::<T>(true /* enable checks */);
	}

	#[cfg(feature = "std")]
	pub fn test_start_data_migration<T: Config>() {
		_start_data_migration::<T>(true /* enable checks */);
	}

	#[cfg(feature = "std")]
	pub fn test_send_chunked_xcm_and_track<T: Config>() {
		_send_chunked_xcm_and_track::<T>(true /* enable checks */);
	}

	#[cfg(feature = "std")]
	pub fn test_receive_query_response<T: Config>() {
		_receive_query_response::<T>(true /* enable checks */);
	}

	#[cfg(feature = "std")]
	pub fn test_resend_xcm<T: Config>() {
		_resend_xcm::<T>(true /* enable checks */);
	}

	#[cfg(feature = "std")]
	pub fn test_set_unprocessed_msg_buffer<T: Config>() {
		_set_unprocessed_msg_buffer::<T>(true /* enable checks */);
	}

	pub fn test_force_ah_ump_queue_priority<T: Config>() {
		_force_ah_ump_queue_priority::<T>(true /* enable checks */);
	}

	#[cfg(feature = "std")]
	pub fn test_set_ah_ump_queue_priority<T: Config>() {
		_set_ah_ump_queue_priority::<T>(true /* enable checks */);
	}

	#[cfg(feature = "std")]
	pub fn test_set_manager<T: Config>() {
		_set_manager::<T>(true /* enable checks */);
	}
}
