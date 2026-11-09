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
// along with Polkadot. If not, see <http://www.gnu.org/licenses/>.

//! Repair `pallet-referenda` track bookkeeping that the Asset Hub Migration left inconsistent.
//!
//! See <https://github.com/polkadot-fellows/runtimes/issues/1323>.
//!
//! # Phantom deciding slots
//!
//! The migration turned ongoing referenda whose call could not be mapped to Asset Hub into
//! `Cancelled` by writing `ReferendumInfoFor` directly. For a referendum that was deciding this
//! skipped `note_one_fewer_deciding`, so its slot was never released, and `DecidingCount` was then
//! copied from the relay chain unchanged. Each such track counts one deciding referendum more than
//! it has, for good: Kusama Asset Hub track 1 and Polkadot Asset Hub track 32 today.
//!
//! # Stale queue entries
//!
//! `cancel` and `kill` do not remove a queued referendum from `TrackQueue`, and
//! `one_fewer_deciding` does not write back a queue it drained to empty. Polkadot #693, cancelled
//! on the relay chain while queued, was carried into Asset Hub's `TrackQueue(0)` this way.
//! <https://github.com/paritytech/polkadot-sdk/pull/13236> fixes the pallet; this migration cleans
//! the entry that is already in storage.
//!
//! # What the migration does
//!
//! 1. Drops every `TrackQueue` entry that is not an `Ongoing` referendum of that track with
//!    `in_queue` set, and any duplicate.
//! 2. For each track, frees `DecidingCount - (deciding referenda + pending alarms)` slots by
//!    calling the pallet's own `one_fewer_deciding`. That starts the best-backed queued referendum
//!    if there is one, and decrements the count otherwise, exactly as on chain.
//!
//! It never raises a count, and running it again changes nothing.
//!
//! # Before release
//!
//! A slot that governance is already freeing must not be freed here too. The migration counts a
//! scheduled bare `one_fewer_deciding(track)` as pending, but it cannot see one wrapped in another
//! call (a whitelisted dispatch, a batch, an XCM `Transact`). If such a call is still to enact when
//! this runs, both free the slot and the track admits one referendum more than `max_deciding`.
//! Check that no ongoing referendum or pending call does this.

use alloc::{
	collections::{btree_map::BTreeMap, btree_set::BTreeSet},
	vec::Vec,
};
use codec::{Decode, Encode};
use core::marker::PhantomData;
use frame_support::{
	traits::{Get, OnRuntimeUpgrade},
	weights::Weight,
};
use pallet_referenda::{
	CallOf, DecidingCount, ReferendumIndex, ReferendumInfo, ReferendumInfoFor, TrackIdOf,
	TrackQueue, TracksInfo, WeightInfo,
};
use sp_runtime::{traits::Hash, Saturating};

#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

#[cfg(test)]
mod tests;

const LOG_TARGET: &str = "runtime::referenda-tracks";

/// How one track's `DecidingCount` compares with the referenda it counts.
#[derive(Encode, Decode, Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct TrackCensus {
	/// `DecidingCount(track)` as stored.
	pub stored: u32,
	/// `Ongoing` referenda on the track that are deciding.
	pub deciding: u32,
	/// Scheduled `one_fewer_deciding(track)` calls that have not run yet.
	///
	/// When a deciding referendum ends, the pallet schedules this call for the next block and
	/// keeps the slot counted until then. Such a slot is not a phantom.
	pub pending: u32,
}

impl TrackCensus {
	/// Slots that a referendum holds or that a pending alarm will release.
	pub fn occupied(&self) -> u32 {
		self.deciding.saturating_add(self.pending)
	}

	/// Slots that nothing holds and nothing will release.
	pub fn phantoms(&self) -> u32 {
		self.stored.saturating_sub(self.occupied())
	}
}

/// The referenda and scheduler state that the repair reads, gathered in one pass.
pub struct Census<T: pallet_referenda::Config<I>, I: 'static = ()> {
	/// Every track that has a `DecidingCount`, a deciding referendum or a pending alarm.
	pub tracks: BTreeMap<TrackIdOf<T, I>, TrackCensus>,
	/// `Ongoing` referenda with `in_queue` set, and their track.
	pub queued: BTreeMap<ReferendumIndex, TrackIdOf<T, I>>,
	/// Storage reads it took to gather this.
	pub reads: u64,
}

impl<T, I> Census<T, I>
where
	T: pallet_referenda::Config<I> + pallet_scheduler::Config,
	I: 'static,
{
	/// Read `ReferendumInfoFor`, `DecidingCount` and the scheduler agenda.
	pub fn take() -> Self {
		let mut tracks = BTreeMap::<TrackIdOf<T, I>, TrackCensus>::new();
		let mut queued = BTreeMap::new();
		let mut reads = 0u64;

		for (index, info) in ReferendumInfoFor::<T, I>::iter() {
			reads.saturating_inc();
			if let ReferendumInfo::Ongoing(status) = info {
				if status.deciding.is_some() {
					tracks.entry(status.track).or_default().deciding.saturating_inc();
				}
				if status.in_queue {
					queued.insert(index, status.track);
				}
			}
		}

		for (track, stored) in DecidingCount::<T, I>::iter() {
			reads.saturating_inc();
			tracks.entry(track).or_default().stored = stored;
		}

		// The pallet schedules `one_fewer_deciding { track }` as an inline bounded call. Match on
		// the call hash, which `Bounded::hash` gives for every variant. This also counts a
		// `one_fewer_deciding` that governance scheduled, which only makes the repair do less.
		let alarms: BTreeMap<T::Hash, TrackIdOf<T, I>> = T::Tracks::track_ids()
			.map(|track| {
				let call: CallOf<T, I> =
					pallet_referenda::Call::<T, I>::one_fewer_deciding { track }.into();
				(T::Hashing::hash_of(&call), track)
			})
			.collect();
		for agenda in pallet_scheduler::Agenda::<T>::iter_values() {
			reads.saturating_inc();
			for task in agenda.into_iter().flatten() {
				if let Some(track) = alarms.get(&task.call.hash()) {
					tracks.entry(*track).or_default().pending.saturating_inc();
				}
			}
		}

		Self { tracks, queued, reads }
	}
}

/// Frees phantom deciding slots and removes stale `TrackQueue` entries. See the [module
/// docs](self).
pub struct RepairReferendaTracks<T, I = ()>(PhantomData<(T, I)>);

impl<T, I> RepairReferendaTracks<T, I>
where
	T: pallet_referenda::Config<I> + pallet_scheduler::Config,
	I: 'static,
{
	fn max_one_fewer_deciding_weight() -> Weight {
		type W<T, I> = <T as pallet_referenda::Config<I>>::WeightInfo;
		let queue_empty = W::<T, I>::one_fewer_deciding_queue_empty();
		let failing = W::<T, I>::one_fewer_deciding_failing();
		let passing = W::<T, I>::one_fewer_deciding_passing();
		queue_empty.max(failing).max(passing)
	}

	/// Drop queue entries that are not a live queued referendum of that track. Returns the weight.
	fn clean_queues(queued: &BTreeMap<ReferendumIndex, TrackIdOf<T, I>>) -> Weight {
		let db = T::DbWeight::get();
		let mut weight = Weight::zero();
		for (track, mut queue) in TrackQueue::<T, I>::iter().collect::<Vec<_>>() {
			weight.saturating_accrue(db.reads(1));
			let mut seen = BTreeSet::new();
			let mut removed = Vec::new();
			queue.retain(|(index, _)| {
				let keep = queued.get(index) == Some(&track) && seen.insert(*index);
				if !keep {
					removed.push(*index);
				}
				keep
			});
			if !removed.is_empty() {
				log::info!(
					target: LOG_TARGET,
					"track {track:?}: removed stale TrackQueue entries {removed:?}",
				);
				if queue.is_empty() {
					TrackQueue::<T, I>::remove(track);
				} else {
					TrackQueue::<T, I>::insert(track, queue);
				}
				weight.saturating_accrue(db.writes(1));
			}
		}
		weight
	}

	/// Free every phantom slot through `one_fewer_deciding`. Returns the weight.
	fn free_phantoms(tracks: &BTreeMap<TrackIdOf<T, I>, TrackCensus>) -> Weight {
		let max_call_weight = Self::max_one_fewer_deciding_weight();
		let mut weight = Weight::zero();
		for (track, census) in tracks {
			if census.stored < census.occupied() {
				// Raising a count could leave a slot that never frees, so leave it alone.
				log::warn!(
					target: LOG_TARGET,
					"track {track:?}: DecidingCount {} is below what is occupied ({census:?}); \
					 not changed",
					census.stored,
				);
			}
			let phantoms = census.phantoms();
			if phantoms == 0 {
				continue;
			}
			log::info!(
				target: LOG_TARGET,
				"track {track:?}: freeing {phantoms} phantom deciding slot(s) ({census:?})",
			);
			for _ in 0..phantoms {
				let origin = frame_system::RawOrigin::<T::AccountId>::Root.into();
				match pallet_referenda::Pallet::<T, I>::one_fewer_deciding(origin, *track) {
					Ok(info) =>
						weight.saturating_accrue(info.actual_weight.unwrap_or(max_call_weight)),
					Err(e) => {
						log::error!(
							target: LOG_TARGET,
							"track {track:?}: one_fewer_deciding failed: {e:?}",
						);
						weight.saturating_accrue(max_call_weight);
						break;
					},
				}
			}
		}
		weight
	}
}

impl<T, I> OnRuntimeUpgrade for RepairReferendaTracks<T, I>
where
	T: pallet_referenda::Config<I> + pallet_scheduler::Config,
	I: 'static,
{
	fn on_runtime_upgrade() -> Weight {
		let census = Census::<T, I>::take();
		let mut weight = T::DbWeight::get().reads(census.reads);
		// Clean the queues first, so `one_fewer_deciding` only ever sees live entries.
		weight.saturating_accrue(Self::clean_queues(&census.queued));
		weight.saturating_accrue(Self::free_phantoms(&census.tracks));
		weight
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		let census = Census::<T, I>::take();
		for (track, c) in census.tracks.iter().filter(|(_, c)| c.stored != c.occupied()) {
			log::info!(target: LOG_TARGET, "pre-upgrade: track {track:?} is inconsistent: {c:?}");
		}
		// Live queue entries: each must still be queued, or deciding, afterwards.
		let live_queued: Vec<(ReferendumIndex, TrackIdOf<T, I>)> = TrackQueue::<T, I>::iter()
			.flat_map(|(track, queue)| {
				queue.into_iter().map(move |(index, _)| (index, track)).collect::<Vec<_>>()
			})
			.filter(|(index, track)| census.queued.get(index) == Some(track))
			.collect();
		Ok((census.tracks, live_queued).encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		use frame_support::ensure;

		type PreState<T, I> =
			(BTreeMap<TrackIdOf<T, I>, TrackCensus>, Vec<(ReferendumIndex, TrackIdOf<T, I>)>);
		let (before, live_queued) = PreState::<T, I>::decode(&mut &state[..])
			.map_err(|_| TryRuntimeError::Other("cannot decode the pre-upgrade census"))?;
		let after = Census::<T, I>::take();

		let tracks: BTreeSet<_> = before.keys().chain(after.tracks.keys()).copied().collect();
		for track in tracks {
			let was = before.get(&track).copied().unwrap_or_default();
			let now = after.tracks.get(&track).copied().unwrap_or_default();
			ensure!(now.stored <= was.stored, "a DecidingCount went up");
			ensure!(now.phantoms() == 0, "a phantom deciding slot is left");
			if was.stored >= was.occupied() {
				ensure!(now.stored == now.occupied(), "DecidingCount does not match its referenda");
			} else {
				ensure!(now.stored == was.stored, "an undercounted DecidingCount was changed");
			}
		}

		for (index, track) in live_queued {
			let still_queued = TrackQueue::<T, I>::get(track).iter().any(|(i, _)| *i == index);
			let now_deciding = matches!(
				ReferendumInfoFor::<T, I>::get(index),
				Some(ReferendumInfo::Ongoing(status)) if status.deciding.is_some()
			);
			ensure!(still_queued || now_deciding, "a live queued referendum was dropped");
		}

		for (track, queue) in TrackQueue::<T, I>::iter() {
			let mut seen = BTreeSet::new();
			for (index, _) in queue.iter() {
				ensure!(
					after.queued.get(index) == Some(&track),
					"a TrackQueue entry is not a queued referendum of its track"
				);
				ensure!(seen.insert(*index), "a TrackQueue holds the same referendum twice");
			}
		}

		Ok(())
	}
}
