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

use crate::*;
use frame_support::traits::{schedule::v3::TaskName, DefensiveTruncateFrom};
use pallet_rc_migrator::scheduler::{alias::Scheduled, RcSchedulerMessage, SchedulerMigrator};
use pallet_scheduler::{RetryConfig, TaskAddress};

/// Messages sent from the RC Migrator concerning the Scheduler pallet.
pub type RcSchedulerMessageOf<T> = RcSchedulerMessage<BlockNumberFor<T>, RcScheduledOf<T>>;

/// Relay Chain `Scheduled` type.
// From https://github.com/paritytech/polkadot-sdk/blob/f373af0d1c1e296c1b07486dd74710b40089250e/substrate/frame/scheduler/src/lib.rs#L203
pub type RcScheduledOf<T> =
	Scheduled<call::BoundedCallOf<T>, BlockNumberFor<T>, <T as Config>::RcPalletsOrigin>;

impl<T: Config> Pallet<T> {
	pub fn do_receive_scheduler_messages(
		messages: Vec<RcSchedulerMessageOf<T>>,
	) -> Result<(), Error<T>> {
		log::info!(target: LOG_TARGET, "Processing {} scheduler messages", messages.len());
		Self::deposit_event(Event::BatchReceived {
			pallet: PalletEventName::Scheduler,
			count: messages.len() as u32,
		});
		let (mut count_good, mut count_bad) = (0, 0);

		for message in messages {
			match Self::do_process_scheduler_message(message) {
				Ok(()) => count_good += 1,
				Err(_) => count_bad += 1,
			}
		}

		Self::deposit_event(Event::BatchProcessed {
			pallet: PalletEventName::Scheduler,
			count_good,
			count_bad,
		});
		log::info!(target: LOG_TARGET, "Processed {} scheduler messages", count_good);

		Ok(())
	}

	fn do_process_scheduler_message(message: RcSchedulerMessageOf<T>) -> Result<(), Error<T>> {
		log::debug!(target: LOG_TARGET, "Processing scheduler message: {:?}", message);

		match message {
			RcSchedulerMessage::IncompleteSince(block_number) => {
				pallet_scheduler::IncompleteSince::<T>::put(block_number);
			},
			RcSchedulerMessage::Agenda((block_number, tasks)) => {
				let mut ah_tasks = Vec::new();
				for task in tasks {
					let task = if let Some(task) = task {
						let origin = match T::RcToAhPalletsOrigin::try_convert(task.origin.clone())
						{
							Ok(origin) => origin,
							Err(_) => {
								// we map all existing cases and do not expect this to happen.
								defensive!(
									"Failed to convert scheduler call origin: {:?}",
									task.origin
								);
								continue;
							},
						};
						let call = if let Ok(call) = Self::map_rc_ah_call(&task.call) {
							call
						} else {
							log::error!(
								target: LOG_TARGET,
								"Failed to convert RC call to AH call for task at block number {:?}",
								block_number
							);
							continue;
						};

						let task = Scheduled {
							maybe_id: task.maybe_id,
							priority: task.priority,
							call,
							maybe_periodic: task.maybe_periodic,
							origin,
						};
						Some(task)
					} else {
						// skip empty tasks
						continue;
					};
					ah_tasks.push(task);
				}
				if ah_tasks.len() > 0 {
					let ah_tasks =
						BoundedVec::<_, T::MaxScheduledPerBlock>::defensive_truncate_from(ah_tasks);
					pallet_rc_migrator::scheduler::alias::Agenda::<T>::insert(
						block_number,
						ah_tasks,
					);
				}
			},
			RcSchedulerMessage::Retries((task_address, retry_config)) => {
				pallet_scheduler::Retries::<T>::insert(task_address, retry_config);
			},
			RcSchedulerMessage::Lookup((task_name, task_address)) => {
				pallet_rc_migrator::scheduler::alias::Lookup::<T>::insert(task_name, task_address);
			},
		}

		Ok(())
	}
}

// (IncompleteSince, Agenda, Agenda call encodings, Retries, Lookup)
#[derive(Decode)]
pub struct RcPrePayload<T: Config> {
	incomplete_since: Option<BlockNumberFor<T>>,
	agenda: Vec<(BlockNumberFor<T>, Vec<Option<RcScheduledOf<T>>>)>,
	agenda_call_encodings: Vec<(BlockNumberFor<T>, Vec<Option<Vec<u8>>>)>,
	retries: Vec<(TaskAddress<BlockNumberFor<T>>, RetryConfig<BlockNumberFor<T>>)>,
	lookup: Vec<(TaskName, TaskAddress<BlockNumberFor<T>>)>,
}

#[cfg(feature = "std")]
impl<T: Config> crate::types::AhMigrationCheck for SchedulerMigrator<T> {
	type RcPrePayload = Vec<u8>;
	type AhPrePayload = ();

	fn pre_check(_rc_pre_payload: Self::RcPrePayload) -> Self::AhPrePayload {
		// Assert storage 'Scheduler::IncompleteSince::ah_pre::empty'
		assert!(
			pallet_scheduler::IncompleteSince::<T>::get().is_none(),
			"IncompleteSince should be empty on asset hub before migration"
		);

		// Assert storage 'Scheduler::Agenda::ah_pre::empty'
		assert!(
			pallet_rc_migrator::scheduler::alias::Agenda::<T>::iter().next().is_none(),
			"Agenda map should be empty on asset hub before migration"
		);

		// Assert storage 'Scheduler::Lookup::ah_pre::empty'
		assert!(
			pallet_rc_migrator::scheduler::alias::Lookup::<T>::iter().next().is_none(),
			"Lookup map should be empty on asset hub before migration"
		);

		// Assert storage 'Scheduler::Retries::ah_pre::empty'
		assert!(
			pallet_scheduler::Retries::<T>::iter().next().is_none(),
			"Retries map should be empty on asset hub before migration"
		);
	}

	fn post_check(rc_pre_payload: Self::RcPrePayload, _ah_pre_payload: Self::AhPrePayload) {
		let rc_payload = RcPrePayload::<T>::decode(&mut &rc_pre_payload[..])
			.expect("Failed to decode RcPrePayload bytes");

		// Assert storage 'Scheduler::IncompleteSince::ah_post::correct'
		assert_eq!(
			pallet_scheduler::IncompleteSince::<T>::get(),
			rc_payload.incomplete_since,
			"IncompleteSince on Asset Hub should match the RC value"
		);

		// Mirror the Agenda conversion in `do_process_scheduler_message` above ^ to construct
		// expected Agendas. Critically, use the passed agenda call encodings to remove reliance
		// on pallet-preimage state, which will have been changed from the actual migration.
		let mut expected_ah_agenda: Vec<_> = rc_payload
			.agenda
			.into_iter()
			.zip(rc_payload.agenda_call_encodings.into_iter())
			.filter_map(|((block_number, rc_tasks), (_, rc_calls_bytes))| {
				let mut ah_tasks = Vec::new();
				for (index, rc_task_opt) in rc_tasks.into_iter().enumerate() {
					if let Some(rc_task) = rc_task_opt {
						// Attempt to convert origin.
						let ah_origin =
							match T::RcToAhPalletsOrigin::try_convert(rc_task.origin.clone()) {
								Ok(origin) => origin,
								Err(_) => {
									// Origin conversion failed, skip task.
									continue;
								},
							};

						// Attempt to convert call.
						let maybe_bytes = rc_calls_bytes[index].clone();
						let ah_call = if let Some(bytes) = maybe_bytes {
							match Pallet::<T>::map_rc_ah_call_no_preimage(bytes) {
								Ok(c) => c,
								// Call conversion failed, skip.
								Err(_) => continue,
							}
						} else {
							// Call encoding was blank, skip.
							continue
						};

						// Build new task.
						let ah_task = Scheduled {
							maybe_id: rc_task.maybe_id,
							priority: rc_task.priority,
							call: ah_call,
							maybe_periodic: rc_task.maybe_periodic,
							origin: ah_origin,
						};
						ah_tasks.push(Some(ah_task));
					} else {
					}
				}
				// Filter out blocks that end up with no valid tasks after conversion.
				if !ah_tasks.is_empty() {
					Some((block_number, ah_tasks))
				} else {
					None
				}
			})
			.collect();

		// Collect agendas on AH.
		let mut ah_agenda: Vec<_> = pallet_rc_migrator::scheduler::alias::Agenda::<T>::iter()
			.map(|(bn, tasks)| (bn, tasks.into_inner()))
			.collect();

		// Assert storage 'Scheduler::Agenda::ah_post::length'
		assert_eq!(
			ah_agenda.len(),
			expected_ah_agenda.len(),
			"Agenda map length on Asset Hub should match converted RC value" /* Original assertion message */
		);

		// Sort to ensure no ordering issues.
		ah_agenda.sort_by_key(|(index, _)| *index);
		expected_ah_agenda.sort_by_key(|(index, _)| *index);

		// Assert storage 'Scheduler::Agenda::ah_post::correct'
		assert_eq!(
			ah_agenda, expected_ah_agenda,
			"Agenda map value on Asset Hub should match the converted RC value"
		);

		// Assert storage 'Scheduler::Lookup::ah_post::length'
		assert_eq!(
			pallet_rc_migrator::scheduler::alias::Lookup::<T>::iter().count(),
			rc_payload.lookup.len(),
			"Lookup map length on Asset Hub should match the RC value"
		);

		// Assert storage 'Scheduler::Lookup::ah_post::correct'
		assert_eq!(
			pallet_rc_migrator::scheduler::alias::Lookup::<T>::iter().collect::<Vec<_>>(),
			rc_payload.lookup,
			"Lookup map value on Asset Hub should match the RC value"
		);

		// Assert storage 'Scheduler::Retries::ah_post::length'
		assert_eq!(
			pallet_scheduler::Retries::<T>::iter().count(),
			rc_payload.retries.len(),
			"Retries map length on Asset Hub should match the RC value"
		);

		// Assert storage 'Scheduler::Retries::ah_post::correct'
		assert_eq!(
			pallet_scheduler::Retries::<T>::iter().collect::<Vec<_>>(),
			rc_payload.retries,
			"Retries map value on Asset Hub should match the RC value"
		);
	}
}

impl<T: Config> Pallet<T> {
	// Helper to convert the call without using the preimage pallet. Used in migration checks.
	pub fn map_rc_ah_call_no_preimage(
		encoded_call: Vec<u8>,
	) -> Result<call::BoundedCallOf<T>, Error<T>> {
		use frame_support::traits::{Bounded, BoundedInline};
		use sp_runtime::traits::Hash;

		// Convert call.
		let call = if let Ok(call) = T::RcToAhCall::try_convert(&encoded_call) {
			call
		} else {
			return Err(Error::<T>::FailedToConvertCall);
		};

		// Bound it.
		let data = call.encode();
		let len = data.len() as u32;
		Ok(match BoundedInline::try_from(data) {
			Ok(bounded) => Bounded::Inline(bounded),
			Err(unbounded) => Bounded::Lookup {
				hash: <<T as frame_system::Config>::Hashing as Hash>::hash(&unbounded[..]),
				len,
			},
		})
	}
}
