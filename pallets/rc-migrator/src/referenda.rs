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
	DecidingCount, MetadataOf, ReferendumCount, ReferendumInfo, ReferendumInfoFor,
	ReferendumInfoOf, TrackQueue,
};

/// The stages of the referenda pallet migration.
#[derive(
	Encode,
	DecodeWithMemTracking,
	Decode,
	Clone,
	Default,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	PartialEq,
	Eq,
)]
pub enum ReferendaStage {
	#[default]
	StorageValues,
	Metadata(Option<u32>),
	ReferendumInfo(Option<u32>),
}

pub struct ReferendaMigrator<T> {
	_phantom: sp_std::marker::PhantomData<T>,
}

#[derive(Encode, Decode, DecodeWithMemTracking, Debug, Clone, TypeInfo, PartialEq, Eq)]
pub struct ReferendaMessage<Track> {
	pub referendum_count: Option<u32>,
	/// (track_id, count)
	pub deciding_count: Vec<(Track, u32)>,
	/// (referendum_id, votes)
	pub track_queue: Vec<(Track, Vec<(u32, u128)>)>,
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
			Some(ReferendaStage::Metadata(last_key)) => {
				Self::migrate_many_metadata(last_key, weight_counter)?
					.map_or(Some(ReferendaStage::ReferendumInfo(None)), |last_key| {
						Some(ReferendaStage::Metadata(Some(last_key)))
					})
			},
			Some(ReferendaStage::ReferendumInfo(last_key)) => {
				Self::migrate_many_referendum_info(last_key, weight_counter)?
					.map(|last_key| ReferendaStage::ReferendumInfo(Some(last_key)))
			},
		};
		Ok(stage)
	}
}

impl<T: Config> ReferendaMigrator<T> {
	fn migrate_values(weight_counter: &mut WeightMeter) -> Result<(), Error<T>> {
		log::debug!(target: LOG_TARGET, "Migrating referenda values");

		let referendum_count =
			ReferendumCount::<T, ()>::exists().then(ReferendumCount::<T, ()>::take);

		// expected tracks count.
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

		if referendum_count.is_none() && deciding_count.is_empty() && track_queue.is_empty() {
			log::info!(
				target: LOG_TARGET,
				"Referenda values are empty. Skipping referenda values migration.",
			);
			return Ok(());
		}

		let mut batch = XcmBatchAndMeter::new_from_config::<T>();
		batch.push(ReferendaMessage { referendum_count, deciding_count, track_queue });
		weight_counter.consume(batch.consume_weight());

		Pallet::<T>::send_chunked_xcm_and_track(batch, |batch| {
			types::AhMigratorCall::<T>::ReceiveReferendaValues { values: batch }
		})?;

		Ok(())
	}

	fn migrate_many_metadata(
		mut last_key: Option<u32>,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<u32>, Error<T>> {
		log::debug!(target: LOG_TARGET, "Migrating referenda metadata");

		let mut batch = XcmBatchAndMeter::new_from_config::<T>();

		let last_key = loop {
			if weight_counter.try_consume(T::DbWeight::get().reads_writes(1, 1)).is_err()
				|| weight_counter.try_consume(batch.consume_weight()).is_err()
			{
				log::info!(
					target: LOG_TARGET,
					"RC weight limit reached at batch length {}, stopping",
					batch.len()
				);
				if batch.is_empty() {
					defensive!("Out of weight too early");
					return Err(Error::OutOfWeight);
				} else {
					break last_key;
				}
			}

			if T::MaxAhWeight::get()
				.any_lt(T::AhWeightInfo::receive_referenda_metadata(batch.len() + 1))
			{
				log::info!(
					target: LOG_TARGET,
					"AH weight limit reached at batch length {}, stopping",
					batch.len()
				);
				if batch.is_empty() {
					defensive!("Out of weight too early");
					return Err(Error::OutOfWeight);
				} else {
					break last_key;
				}
			}

			if batch.len() >= MAX_ITEMS_PER_BLOCK {
				log::info!(
					target: LOG_TARGET,
					"Maximum number of items ({:?}) to migrate per block reached, current batch size: {}",
					MAX_ITEMS_PER_BLOCK,
					batch.len()
				);
				break last_key;
			}

			if batch.batch_count() >= MAX_XCM_MSG_PER_BLOCK {
				log::info!(
					target: LOG_TARGET,
					"Reached the maximum number of batches ({:?}) allowed per block; current batch count: {}",
					MAX_XCM_MSG_PER_BLOCK,
					batch.batch_count()
				);
				break last_key;
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
			Pallet::<T>::send_chunked_xcm_and_track(batch, |batch| {
				types::AhMigratorCall::<T>::ReceiveReferendaMetadata { metadata: batch }
			})?;
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

		let mut batch = XcmBatchAndMeter::new_from_config::<T>();

		let last_key = loop {
			if weight_counter.try_consume(T::DbWeight::get().reads_writes(1, 1)).is_err()
				|| weight_counter.try_consume(batch.consume_weight()).is_err()
			{
				log::info!(
					target: LOG_TARGET,
					"RC weight limit reached at batch length {}, stopping",
					batch.len()
				);
				if batch.is_empty() {
					defensive!("Out of weight too early");
					return Err(Error::OutOfWeight);
				} else {
					break last_key;
				}
			}

			if batch.len() >= MAX_ITEMS_PER_BLOCK {
				log::info!(
					target: LOG_TARGET,
					"Maximum number of items ({:?}) to migrate per block reached, current batch size: {}",
					MAX_ITEMS_PER_BLOCK,
					batch.len()
				);
				break last_key;
			}

			if batch.batch_count() >= MAX_XCM_MSG_PER_BLOCK {
				log::info!(
					target: LOG_TARGET,
					"Reached the maximum number of batches ({:?}) allowed per block; current batch count: {}",
					MAX_XCM_MSG_PER_BLOCK,
					batch.batch_count()
				);
				break last_key;
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

			let Some(info) = ReferendumInfoFor::<T, ()>::get(next_key) else {
				defensive!("ReferendumInfoFor is empty");
				last_key = ReferendumInfoFor::<T, ()>::iter_keys_from_key(next_key).next();
				continue;
			};

			if ah_weight_counter
				.try_consume(Self::weight_ah_referendum_info(batch.len(), &info))
				.is_err()
			{
				log::info!(
					target: LOG_TARGET,
					"AH weight limit reached at batch length {}, stopping",
					batch.len()
				);
				if batch.is_empty() {
					defensive!("Out of weight too early");
					return Err(Error::OutOfWeight);
				} else {
					break last_key;
				}
			}

			ReferendumInfoFor::<T, ()>::remove(next_key);
			batch.push((next_key, info));
			last_key = Some(next_key);
		};

		if !batch.is_empty() {
			Pallet::<T>::send_chunked_xcm_and_track(batch, |batch| {
				types::AhMigratorCall::<T>::ReceiveReferendums { referendums: batch }
			})?;
		}

		Ok(last_key)
	}

	/// Get the weight for importing a single referendum info on Asset Hub.
	///
	/// The base weight is only included for the first imported referendum info.
	pub fn weight_ah_referendum_info(batch_len: u32, info: &ReferendumInfoOf<T, ()>) -> Weight {
		match info {
			ReferendumInfo::Ongoing(status) => {
				let len = status.proposal.len().defensive_unwrap_or(
					// should not happen, but we assume some sane call length.
					512,
				);
				T::AhWeightInfo::receive_single_active_referendums(len)
			},
			_ => {
				if batch_len == 0 {
					T::AhWeightInfo::receive_complete_referendums(1)
				} else {
					T::AhWeightInfo::receive_complete_referendums(1)
						.saturating_sub(T::AhWeightInfo::receive_complete_referendums(0))
				}
			},
		}
	}
}

#[cfg(feature = "std")]
impl<T: Config> crate::types::RcMigrationCheck for ReferendaMigrator<T> {
	type RcPrePayload = Vec<u8>;

	fn pre_check() -> Self::RcPrePayload {
		let count = ReferendumCount::<T, ()>::get();
		let deciding_count: Vec<_> = DecidingCount::<T, ()>::iter().collect();
		let track_queue: Vec<_> = TrackQueue::<T, ()>::iter()
			.map(|(track_id, queue)| (track_id, queue.into_inner()))
			.collect();
		let metadata: Vec<_> = MetadataOf::<T, ()>::iter().collect();
		let referenda: Vec<_> = ReferendumInfoFor::<T, ()>::iter().collect();
		// (ReferendumCount, DecidingCount, TrackQueue, MetadataOf, ReferendumInfoFor)
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
