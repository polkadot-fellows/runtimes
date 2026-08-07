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

//! Shared helpers for the multi-block migrations of the system parachains.

use alloc::vec::Vec;
use core::marker::PhantomData;
use frame_support::{
	migrations::{SteppedMigrationError, SteppedMigrations},
	traits::Get,
	weights::WeightMeter,
};

/// Caps the [`SteppedMigration::max_steps`] of every migration in `M` at `Cap`.
///
/// `pallet-migrations` advances a migration at most once per block and compares its `max_steps`
/// against the number of blocks that the migration has been running for. A step limit is therefore
/// a duration limit, and capping it bounds how long a single migration can block the chain.
///
/// A migration that does not declare a `max_steps` of its own is unbounded and would keep the
/// chain in migration mode forever if it never makes progress. This wrapper closes that gap: the
/// cap applies both as a fallback for `None` and as an upper bound for migrations that ask for
/// more.
///
/// Note that the cap is per migration, so the aggregated tuple can still take `len() * Cap`
/// blocks. A migration exceeding the cap is treated as failed and handed to the
/// [`frame_support::migrations::FailedMigrationHandler`] of the runtime.
///
/// [`SteppedMigration::max_steps`]: frame_support::migrations::SteppedMigration::max_steps
pub struct MaxStepsCapped<M, Cap>(PhantomData<(M, Cap)>);

impl<M: SteppedMigrations, Cap: Get<u32>> SteppedMigrations for MaxStepsCapped<M, Cap> {
	fn len() -> u32 {
		M::len()
	}

	fn nth_id(n: u32) -> Option<Vec<u8>> {
		M::nth_id(n)
	}

	fn nth_max_steps(n: u32) -> Option<Option<u32>> {
		M::nth_max_steps(n)
			.map(|max_steps| Some(max_steps.map_or(Cap::get(), |m| m.min(Cap::get()))))
	}

	fn nth_step(
		n: u32,
		cursor: Option<Vec<u8>>,
		meter: &mut WeightMeter,
	) -> Option<Result<Option<Vec<u8>>, SteppedMigrationError>> {
		M::nth_step(n, cursor, meter)
	}

	fn nth_transactional_step(
		n: u32,
		cursor: Option<Vec<u8>>,
		meter: &mut WeightMeter,
	) -> Option<Result<Option<Vec<u8>>, SteppedMigrationError>> {
		M::nth_transactional_step(n, cursor, meter)
	}

	fn nth_migrating_prefixes(n: u32) -> Option<Option<Vec<Vec<u8>>>> {
		M::nth_migrating_prefixes(n)
	}

	#[cfg(feature = "try-runtime")]
	fn nth_pre_upgrade(n: u32) -> Option<Result<Vec<u8>, sp_runtime::TryRuntimeError>> {
		M::nth_pre_upgrade(n)
	}

	#[cfg(feature = "try-runtime")]
	fn nth_post_upgrade(n: u32, state: Vec<u8>) -> Option<Result<(), sp_runtime::TryRuntimeError>> {
		M::nth_post_upgrade(n, state)
	}

	fn cursor_max_encoded_len() -> usize {
		M::cursor_max_encoded_len()
	}

	fn identifier_max_encoded_len() -> usize {
		M::identifier_max_encoded_len()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use frame_support::{migrations::SteppedMigration, traits::ConstU32};

	/// A migration that declares `MAX_STEPS` as its step limit, or none if it is `u32::MAX`.
	struct Migration<const ID: u8, const MAX_STEPS: u32>;

	impl<const ID: u8, const MAX_STEPS: u32> SteppedMigration for Migration<ID, MAX_STEPS> {
		type Cursor = u64;
		type Identifier = u8;

		fn id() -> Self::Identifier {
			ID
		}

		fn max_steps() -> Option<u32> {
			(MAX_STEPS != u32::MAX).then_some(MAX_STEPS)
		}

		fn step(
			_cursor: Option<Self::Cursor>,
			_meter: &mut WeightMeter,
		) -> Result<Option<Self::Cursor>, SteppedMigrationError> {
			Ok(None)
		}
	}

	/// Declares no `max_steps`.
	type Unbounded = Migration<0, { u32::MAX }>;
	/// Declares a `max_steps` below the cap.
	type Short = Migration<1, 10>;
	/// Declares a `max_steps` above the cap.
	type Long = Migration<2, 1_000>;

	type Migrations = (Unbounded, Short, Long);
	type Capped = MaxStepsCapped<Migrations, ConstU32<100>>;

	#[test]
	fn unset_max_steps_falls_back_to_the_cap() {
		assert_eq!(Migrations::nth_max_steps(0), Some(None));
		assert_eq!(Capped::nth_max_steps(0), Some(Some(100)));
	}

	#[test]
	fn max_steps_below_the_cap_is_kept() {
		assert_eq!(Capped::nth_max_steps(1), Some(Some(10)));
	}

	#[test]
	fn max_steps_above_the_cap_is_capped() {
		assert_eq!(Migrations::nth_max_steps(2), Some(Some(1_000)));
		assert_eq!(Capped::nth_max_steps(2), Some(Some(100)));
	}

	#[test]
	fn out_of_bounds_index_stays_none() {
		assert_eq!(Capped::nth_max_steps(3), None);
		assert_eq!(Capped::nth_id(3), None);
	}

	#[test]
	fn everything_else_is_forwarded() {
		assert_eq!(Capped::len(), Migrations::len());
		assert_eq!(Capped::nth_id(1), Migrations::nth_id(1));
		assert_eq!(Capped::nth_migrating_prefixes(1), Migrations::nth_migrating_prefixes(1));
		assert_eq!(Capped::cursor_max_encoded_len(), Migrations::cursor_max_encoded_len());
		assert_eq!(Capped::identifier_max_encoded_len(), Migrations::identifier_max_encoded_len());

		let mut meter = WeightMeter::new();
		assert!(matches!(Capped::nth_step(1, None, &mut meter), Some(Ok(None))));

		sp_state_machine::BasicExternalities::default().execute_with(|| {
			assert!(matches!(Capped::nth_transactional_step(1, None, &mut meter), Some(Ok(None))));

			// The cap does not change how the pallet sees the tuple otherwise.
			Capped::integrity_test().unwrap();
		});
	}
}
