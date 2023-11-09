//! A copy of the migration found in the polkadot sdk repo.
//!
//! It is copied as the version of the migration found in the crate used by this runtime is broken.

use frame_support::{
	migrations::VersionedMigration, pallet_prelude::ValueQuery, storage_alias,
	traits::OnRuntimeUpgrade, weights::Weight,
};
use frame_system::pallet_prelude::BlockNumberFor;
use parity_scale_codec::{Decode, Encode};
use primitives::{
	v5::{Assignment, ParasEntry},
	CoreIndex, CoreOccupied, GroupIndex, Id as ParaId,
};
use runtime_parachains::scheduler::*;
use scale_info::TypeInfo;
use sp_core::{Get, RuntimeDebug};
use sp_std::{
	collections::{btree_map::BTreeMap, vec_deque::VecDeque},
	prelude::*,
};

const LOG_TARGET: &str = "runtime::parachains::scheduler";

mod v0 {
	use super::*;

	use primitives::{CollatorId, Id};

	#[storage_alias]
	pub(super) type Scheduled<T: Config> = StorageValue<Pallet<T>, Vec<CoreAssignment>, ValueQuery>;

	#[derive(Clone, Encode, Decode)]
	#[cfg_attr(feature = "std", derive(PartialEq))]
	pub struct ParathreadClaim(pub Id, pub CollatorId);

	#[derive(Clone, Encode, Decode)]
	#[cfg_attr(feature = "std", derive(PartialEq))]
	pub struct ParathreadEntry {
		/// The claim.
		pub claim: ParathreadClaim,
		/// Number of retries.
		pub retries: u32,
	}

	/// What is occupying a specific availability core.
	#[derive(Clone, Encode, Decode)]
	#[cfg_attr(feature = "std", derive(PartialEq))]
	pub enum CoreOccupied {
		/// A parathread.
		Parathread(ParathreadEntry),
		/// A parachain.
		Parachain,
	}

	/// The actual type isn't important, as we only delete the key in the state.
	#[storage_alias]
	pub(crate) type AvailabilityCores<T: Config> =
		StorageValue<Pallet<T>, Vec<Option<CoreOccupied>>, ValueQuery>;

	/// The actual type isn't important, as we only delete the key in the state.
	#[storage_alias]
	pub(super) type ParathreadQueue<T: Config> = StorageValue<Pallet<T>, (), ValueQuery>;

	#[storage_alias]
	pub(super) type ParathreadClaimIndex<T: Config> = StorageValue<Pallet<T>, (), ValueQuery>;

	/// The assignment type.
	#[derive(Clone, Encode, Decode, TypeInfo, RuntimeDebug)]
	#[cfg_attr(feature = "std", derive(PartialEq))]
	pub enum AssignmentKind {
		/// A parachain.
		Parachain,
		/// A parathread.
		Parathread(CollatorId, u32),
	}

	/// How a free core is scheduled to be assigned.
	#[derive(Clone, Encode, Decode, TypeInfo, RuntimeDebug)]
	#[cfg_attr(feature = "std", derive(PartialEq))]
	pub struct CoreAssignment {
		/// The core that is assigned.
		pub core: CoreIndex,
		/// The unique ID of the para that is assigned to the core.
		pub para_id: ParaId,
		/// The kind of the assignment.
		pub kind: AssignmentKind,
		/// The index of the validator group assigned to the core.
		pub group_idx: GroupIndex,
	}
}

pub mod v1 {
	use super::*;

	#[storage_alias]
	pub(crate) type AvailabilityCores<T: Config> =
		StorageValue<Pallet<T>, Vec<CoreOccupied<BlockNumberFor<T>>>, ValueQuery>;

	#[storage_alias]
	pub(crate) type ClaimQueue<T: Config> = StorageValue<
		Pallet<T>,
		BTreeMap<CoreIndex, VecDeque<Option<ParasEntry<BlockNumberFor<T>>>>>,
		ValueQuery,
	>;

	#[allow(deprecated)]
	pub type MigrateToV1<T> = VersionedMigration<
		0,
		1,
		UncheckedMigrateToV1<T>,
		Pallet<T>,
		<T as frame_system::Config>::DbWeight,
	>;

	#[deprecated(note = "Use MigrateToV1 instead")]
	pub struct UncheckedMigrateToV1<T>(sp_std::marker::PhantomData<T>);
	#[allow(deprecated)]
	impl<T: Config> OnRuntimeUpgrade for UncheckedMigrateToV1<T> {
		fn on_runtime_upgrade() -> Weight {
			let weight_consumed = migrate_to_v1::<T>();

			log::info!(target: LOG_TARGET, "Migrating para scheduler storage to v1");

			weight_consumed
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::DispatchError> {
			let n: u32 = v0::Scheduled::<T>::get().len() as u32 +
				v0::AvailabilityCores::<T>::get().iter().filter(|c| c.is_some()).count() as u32;

			log::info!(
				target: LOG_TARGET,
				"Number of scheduled and waiting for availability before: {n}",
			);

			Ok(n.encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::DispatchError> {
			log::info!(target: LOG_TARGET, "Running post_upgrade()");

			frame_support::ensure!(
				v0::Scheduled::<T>::get().is_empty(),
				"Scheduled should be empty after the migration"
			);

			let expected_len = u32::decode(&mut &state[..]).unwrap();
			let availability_cores_waiting = AvailabilityCores::<T>::get()
				.iter()
				.filter(|c| !matches!(c, CoreOccupied::Free))
				.count();

			frame_support::ensure!(
				ClaimQueue::<T>::get().iter().map(|la_vec| la_vec.1.len()).sum::<usize>() as u32 +
					availability_cores_waiting as u32 ==
					expected_len,
				"ClaimQueue and AvailabilityCores should have the correct length",
			);

			Ok(())
		}
	}
}

pub fn migrate_to_v1<T: Config>() -> Weight {
	let mut weight: Weight = Weight::zero();

	v0::ParathreadQueue::<T>::kill();
	v0::ParathreadClaimIndex::<T>::kill();

	let now = <frame_system::Pallet<T>>::block_number();
	let scheduled = v0::Scheduled::<T>::take();
	let sched_len = scheduled.len() as u64;
	for core_assignment in scheduled {
		let core_idx = core_assignment.core;
		let assignment = Assignment::new(core_assignment.para_id);
		let pe = ParasEntry::new(assignment, now);

		v1::ClaimQueue::<T>::mutate(|la| {
			la.entry(core_idx).or_default().push_back(Some(pe));
		});
	}

	let parachains = runtime_parachains::paras::Pallet::<T>::parachains();
	let availability_cores = v0::AvailabilityCores::<T>::take();
	let mut new_availability_cores = Vec::new();

	for (core_index, core) in availability_cores.into_iter().enumerate() {
		let new_core = if let Some(core) = core {
			match core {
				v0::CoreOccupied::Parachain => CoreOccupied::Paras(ParasEntry::new(
					Assignment::new(parachains[core_index]),
					now,
				)),
				v0::CoreOccupied::Parathread(entry) =>
					CoreOccupied::Paras(ParasEntry::new(Assignment::new(entry.claim.0), now)),
			}
		} else {
			CoreOccupied::Free
		};

		new_availability_cores.push(new_core);
	}

	v1::AvailabilityCores::<T>::set(new_availability_cores);

	// 2x as once for Scheduled and once for Claimqueue
	weight = weight.saturating_add(T::DbWeight::get().reads_writes(2 * sched_len, 2 * sched_len));
	// reading parachains + availability_cores, writing AvailabilityCores
	weight = weight.saturating_add(T::DbWeight::get().reads_writes(2, 1));
	// 2x kill
	weight = weight.saturating_add(T::DbWeight::get().writes(2));

	weight
}
