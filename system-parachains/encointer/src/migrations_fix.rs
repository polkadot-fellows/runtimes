// Copyright (c) 2023 Encointer Association
// This file is part of Encointer
//
// Encointer is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Encointer is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Encointer.  If not, see <http://www.gnu.org/licenses/>.

// the following are temporary local migration fixes to solve inconsistencies caused by not
// migrating Storage at the time of migrating runtime code

pub mod balances {
	use frame_support::traits::OnRuntimeUpgrade;
	use pallet_balances::*;
	#[cfg(feature = "try-runtime")]
	use sp_runtime::TryRuntimeError;

	/// The log target.
	const TARGET: &str = "runtime::fix::balances::migration";
	pub mod v1 {
		use super::*;
		use frame_support::pallet_prelude::*;
		pub struct BruteForceToV1<T>(sp_std::marker::PhantomData<T>);

		impl<T: Config> OnRuntimeUpgrade for BruteForceToV1<T> {
			#[cfg(feature = "try-runtime")]
			fn pre_upgrade() -> Result<sp_std::vec::Vec<u8>, TryRuntimeError> {
				Ok(().encode())
			}

			fn on_runtime_upgrade() -> Weight {
				let onchain_version = Pallet::<T>::on_chain_storage_version();
				if onchain_version >= 1 {
					log::warn!(
						target: TARGET,
						"skipping bruteforce to v1 migration: executed on wrong storage version."
					);
					return T::DbWeight::get().reads(1)
				}
				log::info!(target: TARGET, "bruteforcing from {:?} to 1", onchain_version);
				StorageVersion::new(1).put::<Pallet<T>>();

				T::DbWeight::get().reads_writes(1, 1)
			}

			#[cfg(feature = "try-runtime")]
			fn post_upgrade(_state: sp_std::vec::Vec<u8>) -> Result<(), TryRuntimeError> {
				ensure!(StorageVersion::get::<Pallet<T>>() == 1, "Must upgrade");
				Ok(())
			}
		}
	}
}
pub mod scheduler {
	// this is necessary because migrations from v0 to v3 are no longer available in the scheduler
	// pallet code and migrating is only possible from v3. The strategy here is to empty the agenda
	// (has been empty since genesis)
	use frame_support::traits::OnRuntimeUpgrade;
	use frame_system::pallet_prelude::BlockNumberFor;
	use pallet_scheduler::*;
	use sp_std::vec::Vec;

	#[cfg(feature = "try-runtime")]
	use sp_runtime::TryRuntimeError;

	/// The log target.
	const TARGET: &str = "runtime::fix::scheduler::migration";

	pub mod v1 {
		use super::*;
		use frame_support::{pallet_prelude::*, traits::schedule};

		#[cfg_attr(any(feature = "std", test), derive(PartialEq, Eq))]
		#[derive(Clone, RuntimeDebug, Encode, Decode)]
		pub(crate) struct ScheduledV1<Call, BlockNumber> {
			maybe_id: Option<Vec<u8>>,
			priority: schedule::Priority,
			call: Call,
			maybe_periodic: Option<schedule::Period<BlockNumber>>,
		}

		#[frame_support::storage_alias]
		pub(crate) type Agenda<T: Config> = StorageMap<
			Pallet<T>,
			Twox64Concat,
			BlockNumberFor<T>,
			Vec<Option<ScheduledV1<<T as Config>::RuntimeCall, BlockNumberFor<T>>>>,
			ValueQuery,
		>;

		#[frame_support::storage_alias]
		pub(crate) type Lookup<T: Config> =
			StorageMap<Pallet<T>, Twox64Concat, Vec<u8>, TaskAddress<BlockNumberFor<T>>>;
	}

	pub mod v3 {
		use super::*;
		use frame_support::pallet_prelude::*;

		#[frame_support::storage_alias]
		pub(crate) type Agenda<T: Config> = StorageMap<
			Pallet<T>,
			Twox64Concat,
			BlockNumberFor<T>,
			Vec<Option<ScheduledV3Of<T>>>,
			ValueQuery,
		>;

		#[frame_support::storage_alias]
		pub(crate) type Lookup<T: Config> =
			StorageMap<Pallet<T>, Twox64Concat, Vec<u8>, TaskAddress<BlockNumberFor<T>>>;
	}

	pub mod v4 {
		use super::*;
		use frame_support::pallet_prelude::*;

		#[frame_support::storage_alias]
		pub type Agenda<T: Config> = StorageMap<
			Pallet<T>,
			Twox64Concat,
			BlockNumberFor<T>,
			BoundedVec<
				Option<ScheduledOf<T>>,
				<T as pallet_scheduler::Config>::MaxScheduledPerBlock,
			>,
			ValueQuery,
		>;

		#[cfg(feature = "try-runtime")]
		pub(crate) type TaskName = [u8; 32];

		#[cfg(feature = "try-runtime")]
		#[frame_support::storage_alias]
		pub(crate) type Lookup<T: Config> =
			StorageMap<Pallet<T>, Twox64Concat, TaskName, TaskAddress<BlockNumberFor<T>>>;

		/// Migrate the scheduler pallet from V0 to V4 by brute-force emptying the agenda.
		pub struct MigrateToV4<T>(sp_std::marker::PhantomData<T>);

		impl<T: Config> OnRuntimeUpgrade for MigrateToV4<T> {
			#[cfg(feature = "try-runtime")]
			fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
				let agendas = v1::Agenda::<T>::iter_keys().count() as u32;
				let lookups = v1::Lookup::<T>::iter_keys().count() as u32;
				log::info!(target: TARGET, "agendas present which will be dropped: {}/{}...", agendas, lookups);
				Ok((agendas, lookups).encode())
			}

			fn on_runtime_upgrade() -> Weight {
				let onchain_version = Pallet::<T>::on_chain_storage_version();
				if onchain_version >= 3 {
					log::warn!(
						target: TARGET,
						"skipping v0 to v4 migration: executed on wrong storage version.\
					Expected version < 3, found {:?}",
						onchain_version,
					);
					return T::DbWeight::get().reads(1)
				}
				log::info!(target: TARGET, "migrating from {:?} to 4", onchain_version);
				let purged_agendas = v1::Agenda::<T>::clear(u32::MAX, None).unique as u64;
				let purged_lookups = v1::Lookup::<T>::clear(u32::MAX, None).unique as u64;
				StorageVersion::new(4).put::<Pallet<T>>();

				T::DbWeight::get()
					.reads_writes(purged_agendas + purged_lookups, purged_agendas + purged_lookups)
			}

			#[cfg(feature = "try-runtime")]
			fn post_upgrade(_state: Vec<u8>) -> Result<(), TryRuntimeError> {
				ensure!(StorageVersion::get::<Pallet<T>>() == 4, "Must upgrade");

				let agendas = Agenda::<T>::iter_keys().count() as u32;
				ensure!(agendas == 0, "agenda must be empty after now");
				let lookups = Lookup::<T>::iter_keys().count() as u32;
				ensure!(lookups == 0, "agenda must be empty after now");

				Ok(())
			}
		}
	}
}
