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
use pallet_referenda::{
	DecidingCount, MetadataOf, ReferendumCount, ReferendumInfo, ReferendumInfoFor, TrackIdOf,
	TrackQueue,
};

impl<T: Config> Pallet<T> {
	pub fn do_receive_referendums(
		referendums: Vec<(u32, ReferendumInfoOf<T, ()>)>,
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
		referendum: ReferendumInfoOf<T, ()>,
	) -> Result<(), Error<T>> {
		log::debug!(target: LOG_TARGET, "Integrating referendum {}", id);

		let referendum = if let ReferendumInfo::Ongoing(mut status) = referendum {
			// TODO: map call and update preimage
			// TODO: if mapping fails cancel referendum

			ReferendumInfo::Ongoing(status)
		} else {
			referendum
		};

		ReferendumInfoFor::<T, ()>::insert(id, referendum);

		log::debug!(target: LOG_TARGET, "Referendum {} integrated", id);

		Ok(())
	}

	pub fn do_receive_referenda(
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
			defensive_assert!(queue.len() as u32 <= T::MaxQueued::get(), "track queue too large");
			let queue = BoundedVec::<_, T::MaxQueued>::truncate_from(queue);
			TrackQueue::<T, ()>::insert(track_id, queue);
		});

		Self::deposit_event(Event::ReferendaProcessed);
		log::info!(target: LOG_TARGET, "Referenda pallet integrated");
		Ok(())
	}
}

// TODO: shift referendums' time block by the time of the migration
