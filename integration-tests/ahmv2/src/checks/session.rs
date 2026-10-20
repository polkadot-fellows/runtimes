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

//! Check that the validators keep working once the accounts stage has drained their accounts.

use crate::{
	checks::{ParaMigrationCheck, RcMigrationCheck},
	mock::{network, Para},
};
use frame_support::traits::OnInitialize;
use sp_runtime::AccountId32;
use std::collections::BTreeMap;

type Rc = network::relay::Runtime;
type Keys = network::relay::SessionKeys;

/// The validator set and its keys before the migration.
#[derive(Clone, Debug)]
pub struct SessionPre {
	pub validators: Vec<AccountId32>,
	pub queued: Vec<(AccountId32, Keys)>,
	/// The keys of every validator, active or queued.
	pub next_keys: BTreeMap<AccountId32, Keys>,
}

/// A validator's balance leaves like any other, its session keys stay, and the next session
/// rotation seats the validator again.
pub struct ValidatorsChecker;

impl RcMigrationCheck for ValidatorsChecker {
	type RcPrePayload = SessionPre;
	type RcPostPayload = ();

	fn pre_check() -> Self::RcPrePayload {
		let validators = pallet_session::Validators::<Rc>::get();
		let queued = pallet_session::QueuedKeys::<Rc>::get();
		assert!(!validators.is_empty(), "the snapshot has no validators to check");
		let next_keys = validators
			.iter()
			.chain(queued.iter().map(|(who, _)| who))
			.map(|who| {
				let keys = pallet_session::NextKeys::<Rc>::get(who)
					.unwrap_or_else(|| panic!("validator {who:?} has no session keys"));
				(who.clone(), keys)
			})
			.collect();
		SessionPre { validators, queued, next_keys }
	}

	fn post_check(rc_pre: &Self::RcPrePayload) -> Self::RcPostPayload {
		// Every validator still has its keys. They are not tied to an account record: a validator
		// whose account left the Relay Chain in AHM v1 is seated with them all the same.
		for (who, keys) in &rc_pre.next_keys {
			assert_eq!(pallet_session::NextKeys::<Rc>::get(who).as_ref(), Some(keys));
		}
		assert_eq!(pallet_session::Validators::<Rc>::get(), rc_pre.validators);
		assert_eq!(pallet_session::QueuedKeys::<Rc>::get(), rc_pre.queued);

		// A rotation seats the queued validators and queues the next set with the keys it loads
		// for them. The next set is what Asset Hub elected, or else the seated one.
		let index = pallet_session::CurrentIndex::<Rc>::get();
		let seated: Vec<_> = rc_pre.queued.iter().map(|(who, _)| who.clone()).collect();
		let next = match pallet_staking_async_ah_client::ValidatorSet::<Rc>::get() {
			Some((_, elected)) => elected,
			None => seated.clone(),
		};
		// Babe enacts the epoch change only in a block it initialised.
		<network::relay::Babe as OnInitialize<_>>::on_initialize(
			frame_system::Pallet::<Rc>::block_number(),
		);
		pallet_session::Pallet::<Rc>::rotate_session();

		assert_eq!(pallet_session::CurrentIndex::<Rc>::get(), index + 1);
		assert_eq!(pallet_session::Validators::<Rc>::get(), seated);
		let queued: Vec<_> = next
			.into_iter()
			.filter_map(|who| pallet_session::NextKeys::<Rc>::get(&who).map(|keys| (who, keys)))
			.collect();
		assert_eq!(pallet_session::QueuedKeys::<Rc>::get(), queued);
	}
}

impl<P: Para> ParaMigrationCheck<P> for ValidatorsChecker {
	type RcPrePayload = SessionPre;
	type RcPostPayload = ();
	type PrePayload = ();

	fn pre_check(_: &Self::RcPrePayload) -> Self::PrePayload {}

	fn post_check(_: &Self::RcPrePayload, _: &Self::RcPostPayload, _: Self::PrePayload) {}
}
