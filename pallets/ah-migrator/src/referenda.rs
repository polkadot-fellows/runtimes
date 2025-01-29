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
use frame_support::traits::{schedule::v3::Anon, Bounded, BoundedInline, DefensiveTruncateFrom};
use pallet_referenda::{
	BalanceOf, BoundedCallOf, DecidingCount, ReferendumCount, ReferendumInfo, ReferendumInfoFor,
	ReferendumInfoOf, ReferendumStatus, ReferendumStatusOf, ScheduleAddressOf, TallyOf, TrackIdOf,
	TrackQueue,
};

/// ReferendumInfoOf for RC.
///
/// The `RuntimeOrigin` is a type argument that needs to be mapped to AH `RuntimeOrigin`.
/// Inline `proposal`s and the ones stored by `Preimage` pallet should also be mapped to get the
/// final local `pallet_referenda::ReferendumInfoFor::<T, ()>`.
///
/// Reflects: `pallet_referenda::ReferendumInfoOf::<T, ()>`.
pub type RcReferendumInfoOf<T, I> = ReferendumInfo<
	TrackIdOf<T, I>,
	<T as Config>::RcPalletsOrigin,
	BlockNumberFor<T>,
	BoundedCallOf<T, I>,
	BalanceOf<T, I>,
	TallyOf<T, I>,
	<T as frame_system::Config>::AccountId,
	ScheduleAddressOf<T, I>,
>;

/// RcReferendumStatusOf for RC.
///
/// Reflects: `pallet_referenda::ReferendumStatusOf::<T, ()>`.
pub type RcReferendumStatusOf<T, I> = ReferendumStatus<
	TrackIdOf<T, I>,
	<T as Config>::RcPalletsOrigin,
	BlockNumberFor<T>,
	BoundedCallOf<T, I>,
	BalanceOf<T, I>,
	TallyOf<T, I>,
	<T as frame_system::Config>::AccountId,
	ScheduleAddressOf<T, I>,
>;

impl<T: Config> Pallet<T> {
	pub fn do_receive_referendums(
		referendums: Vec<(u32, RcReferendumInfoOf<T, ()>)>,
	) -> Result<(), Error<T>> {
		log::info!(target: LOG_TARGET, "Integrating {} referendums", referendums.len());
		Self::deposit_event(Event::ReferendumsBatchReceived { count: referendums.len() as u32 });
		let (mut count_good, mut count_bad) = (0, 0);

		for (id, referendum) in referendums {
			match Self::do_receive_referendum(id, referendum) {
				Ok(()) => count_good += 1,
				Err(_) => count_bad += 1,
			}
		}

		Self::deposit_event(Event::ReferendumsBatchProcessed { count_good, count_bad });
		log::info!(target: LOG_TARGET, "Processed {} referendums", count_good);

		Ok(())
	}

	pub fn do_receive_referendum(
		id: u32,
		referendum: RcReferendumInfoOf<T, ()>,
	) -> Result<(), Error<T>> {
		log::debug!(target: LOG_TARGET, "Integrating referendum {}", id);

		let referendum: ReferendumInfoOf<T, ()> = match referendum {
			ReferendumInfo::Ongoing(status) => {
				let cancel_referendum = |id, status: RcReferendumStatusOf<T, ()>| {
					if let Some((_, last_alarm)) = status.alarm {
						// TODO: scheduler migrated first?
						let _ = T::Scheduler::cancel(last_alarm);
					}
					// TODO: use referenda block provider
					let now = frame_system::Pallet::<T>::block_number();
					ReferendumInfoFor::<T, ()>::insert(
						id,
						ReferendumInfo::Cancelled(
							now,
							Some(status.submission_deposit),
							status.decision_deposit,
						),
					);
					log::error!("!!! Referendum {} cancelled", id);
				};

				let origin = match T::RcToAhPalletsOrigin::try_convert(status.origin.clone()) {
					Ok(origin) => origin,
					Err(_) => {
						defensive!(
							"Failed to convert RC origin to AH origin for referendum {}",
							id
						);
						cancel_referendum(id, status);
						return Ok(());
					},
				};

				let encoded_call = if let Ok(e) = Self::fetch_preimage(&status.proposal) {
					e
				} else {
					log::warn!("Failed to fetch preimage for referendum {}", id);
					cancel_referendum(id, status);
					return Ok(());
				};

				let call = if let Ok(call) = T::RcToAhCall::try_convert(&encoded_call) {
					call
				} else {
					// TODO: replace with defensive if we expect all referendum calls to be mapped.
					log::warn!("Failed to convert RC call to AH call for referendum {}", id);
					cancel_referendum(id, status);
					return Ok(());
				};

				let inline = if let Ok(i) = BoundedInline::try_from(call.encode()) {
					i
				} else {
					let data = call.encode();
					log::error!("Failed to bound call for referendum {}, orig_len={}, new_len={}, pallet={:?}, call={:?}, hex={}",
								id, encoded_call.len(), data.len(), data.clone()[0], data.clone()[1], hex::encode(data));
					//defensive!("Call encoded length is too large for inline encoding");
					// TODO: if we have such a case we would need to dispatch two call on behalf of
					// the original preimage submitter to release the funds for the new preimage
					// deposit and dispatch the call to note a new preimage. or we provide a
					// better sdk for this case.
					cancel_referendum(id, status);
					return Ok(());
				};

				let status = ReferendumStatusOf::<T, ()> {
					track: status.track,
					origin,
					proposal: Bounded::Inline(inline),
					enactment: status.enactment,
					submitted: status.submitted,
					submission_deposit: status.submission_deposit,
					decision_deposit: status.decision_deposit,
					deciding: status.deciding,
					tally: status.tally,
					in_queue: status.in_queue,
					alarm: status.alarm,
				};

				ReferendumInfo::Ongoing(status)
			},
			ReferendumInfo::Approved(a, b, c) => ReferendumInfo::Approved(a, b, c),
			ReferendumInfo::Rejected(a, b, c) => ReferendumInfo::Rejected(a, b, c),
			ReferendumInfo::Cancelled(a, b, c) => ReferendumInfo::Cancelled(a, b, c),
			ReferendumInfo::TimedOut(a, b, c) => ReferendumInfo::TimedOut(a, b, c),
			ReferendumInfo::Killed(a) => ReferendumInfo::Killed(a),
		};

		ReferendumInfoFor::<T, ()>::insert(id, referendum);

		log::debug!(target: LOG_TARGET, "Referendum {} integrated", id);

		Ok(())
	}

	pub fn do_receive_referenda_values(
		referendum_count: u32,
		deciding_count: Vec<(TrackIdOf<T, ()>, u32)>,
		track_queue: Vec<(TrackIdOf<T, ()>, Vec<(u32, u128)>)>,
	) -> Result<(), Error<T>> {
		log::info!(target: LOG_TARGET, "Integrating referenda pallet");

		ReferendumCount::<T, ()>::put(referendum_count);
		deciding_count.iter().for_each(|(track_id, count)| {
			DecidingCount::<T, ()>::insert(track_id, count);
		});
		track_queue.into_iter().for_each(|(track_id, queue)| {
			let queue = BoundedVec::<_, T::MaxQueued>::defensive_truncate_from(queue);
			TrackQueue::<T, ()>::insert(track_id, queue);
		});

		Self::deposit_event(Event::ReferendaProcessed);
		log::info!(target: LOG_TARGET, "Referenda pallet integrated");
		Ok(())
	}

	fn fetch_preimage(bounded_call: &BoundedCallOf<T, ()>) -> Result<Vec<u8>, Error<T>> {
		match bounded_call {
			Bounded::Inline(encoded) => Ok(encoded.clone().into_inner()),
			Bounded::Legacy { hash, .. } => {
				let encoded = if let Ok(encoded) = T::Preimage::fetch(hash, None) {
					encoded
				} else {
					// not an error since a submitter can delete the preimage for ongoing referendum
					log::warn!("No preimage found for call hash: {:?}", hash);
					return Err(Error::<T>::PreimageNotFound);
				};
				Ok(encoded.into_owned())
			},
			Bounded::Lookup { hash, len } => {
				let encoded = if let Ok(encoded) = T::Preimage::fetch(hash, Some(*len)) {
					encoded
				} else {
					// not an error since a submitter can delete the preimage for ongoing referendum
					log::warn!("No preimage found for call hash: {:?}", (hash, len));
					return Err(Error::<T>::PreimageNotFound);
				};
				Ok(encoded.into_owned())
			},
		}
	}
}

// TODO: shift referendums' time block by the time of the migration
// TODO: schedule `one_fewer_deciding` for referendums canceled during migration
