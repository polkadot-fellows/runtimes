// Copyright (C) Polkadot Fellows.
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

//! Pre- and post-migration checks, in the shape of AHM v1's `RcMigrationCheck` and
//! `AhMigrationCheck`.
//!
//! Every check runs on one chain. `pre_check` records the state the migration is about to move,
//! the migration runs, and `post_check` asserts on the outcome against that record. The Relay
//! Chain's records are handed to the checks on the other chains, so they can assert on what
//! arrived. Every check runs inside `hypothetically!` and cannot change the state it inspects.
//!
//! Each stage adds its checker as one module here and one entry in the test's checker tuple.

pub mod accounts;
pub mod session;

use crate::mock::Para;
use frame_support::hypothetically;

/// Checks run on the Relay Chain before and after the migration.
pub trait RcMigrationCheck {
	/// What the Relay Chain held before the migration.
	type RcPrePayload;
	/// What the Relay Chain booked as sent, for the receiving chains to check against.
	type RcPostPayload;

	/// Record the state the migration is about to move.
	fn pre_check() -> Self::RcPrePayload;

	/// Check that the recorded state has left the Relay Chain as expected.
	fn post_check(rc_pre: &Self::RcPrePayload) -> Self::RcPostPayload;
}

/// Checks run on a parachain the Relay Chain sends to, before and after the migration.
pub trait ParaMigrationCheck<P: Para> {
	/// What the Relay Chain held before the migration.
	type RcPrePayload;
	/// What the Relay Chain booked as sent.
	type RcPostPayload;
	/// What the parachain held before the migration.
	type PrePayload;

	/// Record the state the migration will change.
	fn pre_check(rc_pre: &Self::RcPrePayload) -> Self::PrePayload;

	/// Check that what the Relay Chain sent arrived as expected.
	fn post_check(
		rc_pre: &Self::RcPrePayload,
		rc_post: &Self::RcPostPayload,
		pre: Self::PrePayload,
	);
}

/// Wrapper for the `frame_support::hypothetically` macro since we want to use it in a macro again.
fn hypothetical_fn<R>(f: impl FnOnce() -> R) -> R {
	hypothetically! { f() }
}

#[allow(clippy::unused_unit)]
#[impl_trait_for_tuples::impl_for_tuples(8)]
impl RcMigrationCheck for Tuple {
	for_tuples! { type RcPrePayload = ( #( Tuple::RcPrePayload ),* ); }
	for_tuples! { type RcPostPayload = ( #( Tuple::RcPostPayload ),* ); }

	fn pre_check() -> Self::RcPrePayload {
		(for_tuples! { #( hypothetical_fn(Tuple::pre_check) ),* })
	}

	fn post_check(rc_pre: &Self::RcPrePayload) -> Self::RcPostPayload {
		(for_tuples! { #( hypothetical_fn(|| Tuple::post_check(&rc_pre.Tuple)) ),* })
	}
}

#[allow(clippy::unused_unit)]
#[impl_trait_for_tuples::impl_for_tuples(8)]
impl<P: Para> ParaMigrationCheck<P> for Tuple {
	for_tuples! { type RcPrePayload = ( #( Tuple::RcPrePayload ),* ); }
	for_tuples! { type RcPostPayload = ( #( Tuple::RcPostPayload ),* ); }
	for_tuples! { type PrePayload = ( #( Tuple::PrePayload ),* ); }

	fn pre_check(rc_pre: &Self::RcPrePayload) -> Self::PrePayload {
		(for_tuples! { #( hypothetical_fn(|| Tuple::pre_check(&rc_pre.Tuple)) ),* })
	}

	fn post_check(
		rc_pre: &Self::RcPrePayload,
		rc_post: &Self::RcPostPayload,
		pre: Self::PrePayload,
	) {
		(for_tuples! { #(
			hypothetical_fn(|| Tuple::post_check(&rc_pre.Tuple, &rc_post.Tuple, pre.Tuple))
		),* });
	}
}
