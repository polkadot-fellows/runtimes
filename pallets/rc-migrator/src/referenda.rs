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
use pallet_referenda::{DecidingCount, MetadataOf, ReferendumCount, ReferendumIndex, ReferendumInfoFor, ReferendumInfoOf, TrackIdOf, TrackQueue, VotesOf};

/// The stages of the referenda pallet migration.
#[derive(Encode, Decode, Clone, Default, RuntimeDebug, TypeInfo, MaxEncodedLen, PartialEq, Eq)]
pub enum ReferendaStage {
	#[default]
	StorageValues,
	Metadata(Option<u32>),
	ReferendumInfo(Option<u32>),
}

pub struct ReferendaMigrator<T> {
	_phantom: sp_std::marker::PhantomData<T>,
}

impl<T: Config> PalletMigration for ReferendaMigrator<T> {
	type Key = ReferendaStage;
	type Error = Error<T>;

	fn migrate_many(
		last_key: Option<Self::Key>,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<Self::Key>, Self::Error> {
		let stage = match last_key {
			None | Some(ReferendaStage::StorageValues) => {
				Self::migrate_values(weight_counter)?;
				Some(ReferendaStage::Metadata(None))
			},
			Some(ReferendaStage::Metadata(last_key)) =>
				Self::migrate_many_metadata(last_key, weight_counter)?
					.map_or(Some(ReferendaStage::ReferendumInfo(None)), |last_key| {
						Some(ReferendaStage::Metadata(Some(last_key)))
					}),
			Some(ReferendaStage::ReferendumInfo(last_key)) =>
				Self::migrate_many_referendum_info(last_key, weight_counter)?
					.map(|last_key| ReferendaStage::ReferendumInfo(Some(last_key))),
		};
		Ok(stage)
	}
}

impl<T: Config> ReferendaMigrator<T> {
	fn migrate_values(_weight_counter: &mut WeightMeter) -> Result<(), Error<T>> {
		log::debug!(target: LOG_TARGET, "Migrating referenda values");

		let referendum_count = ReferendumCount::<T, ()>::take();
		const TRACKS_COUNT: usize = 16;

		// track_id, count
		let deciding_count = DecidingCount::<T, ()>::iter().drain().collect::<Vec<_>>();
		defensive_assert!(
			deciding_count.len() <= TRACKS_COUNT,
			"Deciding count unexpectedly large"
		);

		// (track_id, vec<(referendum_id, votes)>)
		let track_queue = TrackQueue::<T, ()>::iter()
			.drain()
			.map(|(track_id, queue)| (track_id, queue.into_inner()))
			.collect::<Vec<_>>();
		defensive_assert!(track_queue.len() <= TRACKS_COUNT, "Track queue unexpectedly large");

		Pallet::<T>::send_xcm_and_track(
			types::AhMigratorCall::<T>::ReceiveReferendaValues {
				referendum_count,
				deciding_count,
				track_queue,
			},
			Weight::from_all(1),
		)?;

		Ok(())
	}

	fn migrate_many_metadata(
		mut last_key: Option<u32>,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<u32>, Error<T>> {
		log::debug!(target: LOG_TARGET, "Migrating referenda metadata");

		// we should not send more than AH can handle within the block.
		let mut ah_weight_counter = WeightMeter::with_limit(T::MaxAhWeight::get());
		let mut batch = Vec::new();

		let last_key = loop {
			if weight_counter.try_consume(T::DbWeight::get().reads_writes(1, 1)).is_err() {
				if batch.is_empty() {
					defensive!("Out of weight too early");
					return Err(Error::OutOfWeight);
				} else {
					break last_key;
				}
			}

			// TODO: replace by the actual weight.
			if ah_weight_counter.try_consume(Weight::from_all(1)).is_err() {
				if batch.is_empty() {
					defensive!("Out of weight too early");
					return Err(Error::OutOfWeight);
				} else {
					break last_key;
				}
			}

			let next_key = match last_key {
				Some(last_key) => {
					let Some(next_key) = MetadataOf::<T, ()>::iter_keys_from_key(last_key).next()
					else {
						break None;
					};
					next_key
				},
				None => {
					let Some(next_key) = MetadataOf::<T, ()>::iter_keys().next() else {
						break None;
					};
					next_key
				},
			};

			let Some(hash) = MetadataOf::<T, ()>::take(next_key) else {
				defensive!("MetadataOf is empty");
				last_key = MetadataOf::<T, ()>::iter_keys_from_key(next_key).next();
				continue;
			};

			batch.push((next_key, hash));
			last_key = Some(next_key);
		};

		if !batch.is_empty() {
			Pallet::<T>::send_chunked_xcm_and_track(
				batch,
				|batch| types::AhMigratorCall::<T>::ReceiveReferendaMetadata { metadata: batch },
				|_| Weight::from_all(1), // TODO
			)?;
		}

		Ok(last_key)
	}

	fn migrate_many_referendum_info(
		mut last_key: Option<u32>,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<u32>, Error<T>> {
		log::debug!(target: LOG_TARGET, "Migrating referenda info");

		// we should not send more than AH can handle within the block.
		let mut ah_weight_counter = WeightMeter::with_limit(T::MaxAhWeight::get());

		let mut batch = Vec::new();

		// TODO: account transport/XCM weight.

		let last_key = loop {
			if weight_counter.try_consume(T::DbWeight::get().reads_writes(1, 1)).is_err() {
				if batch.is_empty() {
					defensive!("Out of weight too early");
					return Err(Error::OutOfWeight);
				} else {
					break last_key;
				}
			}

			// TODO: replace by the actual weight.
			if ah_weight_counter.try_consume(Weight::from_all(1)).is_err() {
				if batch.is_empty() {
					defensive!("Out of weight too early");
					return Err(Error::OutOfWeight);
				} else {
					break last_key;
				}
			}

			let next_key = match last_key {
				Some(last_key) => {
					let Some(next_key) =
						ReferendumInfoFor::<T, ()>::iter_keys_from_key(last_key).next()
					else {
						break None;
					};
					next_key
				},
				None => {
					let Some(next_key) = ReferendumInfoFor::<T, ()>::iter_keys().next() else {
						break None;
					};
					next_key
				},
			};

			let Some(info) = ReferendumInfoFor::<T, ()>::take(next_key) else {
				defensive!("ReferendumInfoFor is empty");
				last_key = ReferendumInfoFor::<T, ()>::iter_keys_from_key(next_key).next();
				continue;
			};

			batch.push((next_key, info));
			last_key = Some(next_key);
		};

		if !batch.is_empty() {
			Pallet::<T>::send_chunked_xcm_and_track(
				batch,
				|batch| types::AhMigratorCall::<T>::ReceiveReferendums { referendums: batch },
				|_| Weight::from_all(1), // TODO
			)?;
		}

		Ok(last_key)
	}
}

// (ReferendumCount, DecidingCount, TrackQueue, MetadataOf, ReferendumInfoFor)
pub type RcPrePayload<T> = (
    ReferendumIndex,
    Vec<(TrackIdOf<T, ()>, u32)>,
    Vec<(TrackIdOf<T, ()>, Vec<(ReferendumIndex, VotesOf<T, ()>)> )>,
    Vec<(ReferendumIndex, <T as frame_system::Config>::Hash)>, 
    Vec<(ReferendumIndex, ReferendumInfoOf<T, ()>)>,
);

#[cfg(feature = "std")]
impl<T: Config> crate::types::RcMigrationCheck for ReferendaMigrator<T>
{
	type RcPrePayload = Vec<u8>;

	fn pre_check() -> Self::RcPrePayload {
		let count = ReferendumCount::<T, ()>::get();
		let deciding_count: Vec<_> = DecidingCount::<T, ()>::iter().collect();
		let track_queue: Vec<_> = TrackQueue::<T, ()>::iter()
			.map(|(track_id, queue)| (track_id, queue.into_inner()))
			.collect();
		let metadata: Vec<_> = MetadataOf::<T, ()>::iter().collect();
		let referenda: Vec<_> = ReferendumInfoFor::<T, ()>::iter().collect();
		(count, deciding_count, track_queue, metadata, referenda).encode()
	}

	fn post_check(_rc_pre_payload: Self::RcPrePayload) {
		// Assert storage 'Referenda::ReferendumCount::rc_post::empty'
		assert_eq!(
			ReferendumCount::<T, ()>::get(),
			0,
			"Referendum count should be 0 on RC post migration"
		);

		// Assert storage 'Referenda::DecidingCount::rc_post::empty'
		assert!(
			DecidingCount::<T, ()>::iter().next().is_none(),
			"Deciding count map should be empty on RC post migration"
		);

		// Assert storage 'Referenda::TrackQueue::rc_post::empty'
		assert!(
			TrackQueue::<T, ()>::iter().next().is_none(),
			"Track queue map should be empty on RC post migration"
		);

		// Assert storage 'Referenda::MetadataOf::rc_post::empty'
		assert!(
			MetadataOf::<T, ()>::iter().next().is_none(),
			"MetadataOf map should be empty on RC post migration"
		);

		// Assert storage 'Referenda::ReferendumInfoFor::rc_post::empty'
		assert!(
			ReferendumInfoFor::<T, ()>::iter().next().is_none(),
			"Referendum info for map should be empty on RC post migration"
		);
	}
}