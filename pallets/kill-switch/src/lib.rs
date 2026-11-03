// Copyright (C) Parity Technologies and the various Polkadot contributors, see Contributions.md
// for a list of specific contributors.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # Kill switch
//!
//! Lets anyone pause something on the chain by putting up a deposit, and lets the Fellowship (or
//! Root) clean up after it.
//!
//! - [`Call::pause`]: anyone. Holds a deposit priced by [`Config::DepositFor`] and pauses the
//!   target through [`Config::Pauser`]. The pause ends by itself after [`Config::PauseDuration`].
//! - [`Call::extend`]: anyone. Holds another deposit and pushes the end out by another
//!   [`Config::PauseDuration`].
//! - [`Call::unpause`]: Fellowship or Root. Ends the pause and slashes or refunds every deposit
//!   behind it. A slash also puts the target on cooldown for [`Config::Cooldown`], so it can't be
//!   paused again right away.
//! - [`Call::force_extend`]: Fellowship, up to [`Config::MaxFellowshipExtension`] in total per
//!   pause, or Root without a limit.
//! - [`Call::force_pause`]: Root. No deposit and no cooldown check.
//! - [`Call::expire`]: anyone, once a pause is past its end. Backup for the `on_poll` hook.
//! - [`Call::release_deposit`] and [`Call::slash_deposit`]: deposits of pauses that ran out on
//!   their own stay held for [`Config::ReleaseDelay`], so the Fellowship or Root can still slash
//!   them.
//!
//! Time is measured with [`Config::BlockNumberProvider`], usually relay chain blocks. Expiries
//! are queued by block number and handled in `on_poll`. The provider's block number can jump by
//! more than one between two of our blocks, so a cursor remembers the last block that was handled.
//!
//! The pallet doesn't decide what can be paused. [`Config::DepositFor`] returns `None` for targets
//! that can't be paused, and [`Config::Pauser`] has the last word through
//! [`TransactionPause::can_pause`].

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

pub use pallet::*;
pub use weights::WeightInfo;

use frame_support::{
	pallet_prelude::*,
	traits::{
		fungible::{BalancedHold, Credit, Inspect, Mutate, MutateHold},
		tokens::Precision,
		OnUnbalanced, TransactionPause,
	},
	weights::WeightMeter,
};
use frame_system::pallet_prelude::*;
use sp_runtime::traits::{BlockNumberProvider, Convert, Saturating, Zero};

/// The log target of this pallet.
pub const LOG_TARGET: &str = "runtime::kill-switch";

/// Block number of [`Config::BlockNumberProvider`].
pub type BlockNumberOf<T> =
	<<T as Config>::BlockNumberProvider as BlockNumberProvider>::BlockNumber;
pub type BalanceOf<T> =
	<<T as Config>::Currency as Inspect<<T as frame_system::Config>::AccountId>>::Balance;
pub type CreditOf<T> = Credit<<T as frame_system::Config>::AccountId, <T as Config>::Currency>;

/// Identifies a deposit.
pub type DepositId = u64;

/// A running pause.
#[derive(
	Encode, Decode, CloneNoBound, PartialEqNoBound, EqNoBound, DebugNoBound, TypeInfo, MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct PauseInfo<T: Config> {
	/// When the pause ends by itself. `None` means it only ends through [`Call::unpause`].
	pub until: Option<BlockNumberOf<T>>,
	/// How far the Fellowship has pushed `until` out so far.
	pub fellowship_extension: BlockNumberOf<T>,
	/// The deposits behind this pause, from [`Call::pause`] and [`Call::extend`].
	pub deposits: BoundedVec<DepositId, T::MaxDepositsPerPause>,
}

/// A deposit that is on hold.
#[derive(
	Encode, Decode, CloneNoBound, PartialEqNoBound, EqNoBound, DebugNoBound, TypeInfo, MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct DepositInfo<T: Config> {
	pub who: T::AccountId,
	pub amount: BalanceOf<T>,
	/// What it was paid for.
	pub target: T::Target,
	pub created_at: BlockNumberOf<T>,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// What can be paused. For calls this is the `(pallet, call)` name `pallet-tx-pause`
		/// uses.
		type Target: Parameter + MaxEncodedLen;

		/// Applies and lifts pauses.
		type Pauser: TransactionPause<CallIdentifier = Self::Target>;

		/// The deposit needed to pause or extend a target. `None` means it can't be paused.
		type DepositFor: Convert<Self::Target, Option<BalanceOf<Self>>>;

		/// Holds the deposits.
		type Currency: MutateHold<Self::AccountId, Reason = Self::RuntimeHoldReason>
			+ BalancedHold<Self::AccountId>
			+ Mutate<Self::AccountId>;

		/// The overarching hold reason.
		type RuntimeHoldReason: From<HoldReason>;

		/// Where slashed deposits go.
		type OnSlash: OnUnbalanced<CreditOf<Self>>;

		/// Can unpause, slash, and extend up to [`Config::MaxFellowshipExtension`].
		type FellowshipOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Can do everything the Fellowship can without the limit, plus [`Call::force_pause`].
		type ForceOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Measures time for pauses, cooldowns and deposits.
		type BlockNumberProvider: BlockNumberProvider;

		/// How long a pause lasts, and how much each [`Call::extend`] adds.
		#[pallet::constant]
		type PauseDuration: Get<BlockNumberOf<Self>>;

		/// How long the deposit of a pause that ran out on its own stays slashable.
		#[pallet::constant]
		type ReleaseDelay: Get<BlockNumberOf<Self>>;

		/// How long a target can't be paused after an unpause with slash.
		#[pallet::constant]
		type Cooldown: Get<BlockNumberOf<Self>>;

		/// How far the Fellowship can push out a single pause in total.
		#[pallet::constant]
		type MaxFellowshipExtension: Get<BlockNumberOf<Self>>;

		/// How many deposits can back one pause.
		#[pallet::constant]
		type MaxDepositsPerPause: Get<u32>;

		/// Upper bound on the work `on_poll` does in one block, counted in queue lookups.
		#[pallet::constant]
		type MaxPollSteps: Get<u32>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;

	}

	#[pallet::composite_enum]
	pub enum HoldReason {
		/// Deposit for pausing or extending a pause.
		#[codec(index = 0)]
		Pause,
	}

	/// Running pauses.
	#[pallet::storage]
	pub type Pauses<T: Config> = StorageMap<_, Blake2_128Concat, T::Target, PauseInfo<T>>;

	/// Pauses that end by themselves, keyed by the block they end at.
	#[pallet::storage]
	pub type Expiries<T: Config> =
		StorageDoubleMap<_, Twox64Concat, BlockNumberOf<T>, Blake2_128Concat, T::Target, ()>;

	/// Number of entries in [`Expiries`].
	#[pallet::storage]
	pub type ExpiryCount<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// The next block of [`Expiries`] that `on_poll` looks at. `None` when nothing is queued.
	#[pallet::storage]
	pub type ExpiryCursor<T: Config> = StorageValue<_, BlockNumberOf<T>>;

	/// Targets that can't be paused until the given block.
	#[pallet::storage]
	pub type Cooldowns<T: Config> = StorageMap<_, Blake2_128Concat, T::Target, BlockNumberOf<T>>;

	/// Deposits on hold.
	#[pallet::storage]
	pub type Deposits<T: Config> = StorageMap<_, Twox64Concat, DepositId, DepositInfo<T>>;

	/// The id of the next deposit.
	#[pallet::storage]
	pub type NextDepositId<T: Config> = StorageValue<_, DepositId, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A target was paused. `who` is `None` for [`Call::force_pause`].
		Paused { target: T::Target, who: Option<T::AccountId>, until: Option<BlockNumberOf<T>> },
		/// A pause now ends later. `who` is `None` for [`Call::force_extend`].
		Extended { target: T::Target, who: Option<T::AccountId>, until: BlockNumberOf<T> },
		/// The Fellowship or Root ended a pause.
		Unpaused { target: T::Target, slashed: bool },
		/// A pause ran out.
		Expired { target: T::Target },
		/// A target can't be paused again until `until`.
		CooldownStarted { target: T::Target, until: BlockNumberOf<T> },
		/// A deposit was put on hold.
		DepositHeld { id: DepositId, who: T::AccountId, amount: BalanceOf<T> },
		/// A deposit was given back.
		DepositReleased { id: DepositId, who: T::AccountId, amount: BalanceOf<T> },
		/// A deposit was slashed.
		DepositSlashed { id: DepositId, who: T::AccountId, amount: BalanceOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The target can't be paused.
		NotPausable,
		/// The target is already paused.
		AlreadyPaused,
		/// The target isn't paused.
		NotPaused,
		/// A pause of this target was slashed recently, it can't be paused again yet.
		InCooldown,
		/// The pause has no end. It only ends through `unpause`.
		OpenEnded,
		/// The pause hasn't run out yet.
		NotExpired,
		/// The new end isn't later than the current one.
		NotLater,
		/// The end is not in the future.
		InPast,
		/// The Fellowship can't extend this pause any further.
		ExtensionTooLong,
		/// The pause already has the maximum number of deposits.
		TooManyDeposits,
		/// There is no deposit with this id.
		UnknownDeposit,
		/// The deposit still backs a running pause. Use `unpause` instead.
		StillPaused,
		/// The deposit can't be released yet.
		TooEarly,
		/// The pauser refused to pause the target.
		PauseFailed,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_poll(_: BlockNumberFor<T>, meter: &mut WeightMeter) {
			Self::process_expiries(meter);
		}

		fn integrity_test() {
			assert!(!T::PauseDuration::get().is_zero(), "a pause must last at least one block");
			assert!(T::MaxDepositsPerPause::get() > 0, "a pause needs room for its first deposit");
			assert!(T::MaxPollSteps::get() > 0, "on_poll must be able to make progress");
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Pause `target` by putting up the deposit [`Config::DepositFor`] asks for.
		///
		/// The pause ends by itself after [`Config::PauseDuration`], unless someone extends it or
		/// the Fellowship or Root ends it earlier.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::pause())]
		pub fn pause(origin: OriginFor<T>, target: T::Target) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let now = T::BlockNumberProvider::current_block_number();

			ensure!(!Pauses::<T>::contains_key(&target), Error::<T>::AlreadyPaused);
			if let Some(end) = Cooldowns::<T>::get(&target) {
				ensure!(now >= end, Error::<T>::InCooldown);
				Cooldowns::<T>::remove(&target);
			}
			ensure!(T::Pauser::can_pause(target.clone()), Error::<T>::NotPausable);

			let id = Self::hold_deposit(&who, &target, now)?;
			let until = now.saturating_add(T::PauseDuration::get());
			let deposits = BoundedVec::truncate_from(alloc::vec![id]);
			Self::do_pause(&target, Some(until), deposits)?;

			Self::deposit_event(Event::Paused { target, who: Some(who), until: Some(until) });
			Ok(())
		}

		/// Push the end of a running pause out by [`Config::PauseDuration`], with another
		/// deposit.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::extend())]
		pub fn extend(origin: OriginFor<T>, target: T::Target) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let now = T::BlockNumberProvider::current_block_number();

			let mut pause = Pauses::<T>::get(&target).ok_or(Error::<T>::NotPaused)?;
			let until = pause.until.ok_or(Error::<T>::OpenEnded)?;
			ensure!(
				pause.deposits.len() < T::MaxDepositsPerPause::get() as usize,
				Error::<T>::TooManyDeposits
			);

			let id = Self::hold_deposit(&who, &target, now)?;
			pause.deposits.try_push(id).map_err(|_| Error::<T>::TooManyDeposits)?;
			let new_until = until.max(now).saturating_add(T::PauseDuration::get());
			Self::reschedule_expiry(&target, &mut pause, new_until);
			Pauses::<T>::insert(&target, pause);

			Self::deposit_event(Event::Extended { target, who: Some(who), until: new_until });
			Ok(())
		}

		/// End a pause and slash or refund every deposit behind it.
		///
		/// A slash also puts `target` on cooldown for [`Config::Cooldown`].
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::unpause(T::MaxDepositsPerPause::get()))]
		pub fn unpause(
			origin: OriginFor<T>,
			target: T::Target,
			slash: bool,
		) -> DispatchResultWithPostInfo {
			Self::ensure_fellowship_or_force(origin)?;
			let now = T::BlockNumberProvider::current_block_number();

			let pause = Pauses::<T>::get(&target).ok_or(Error::<T>::NotPaused)?;
			Self::do_unpause(&target, &pause);
			for id in pause.deposits.iter() {
				Self::settle_deposit(*id, slash)?;
			}

			let cooldown = T::Cooldown::get();
			if slash && !cooldown.is_zero() {
				let until = now.saturating_add(cooldown);
				Cooldowns::<T>::insert(&target, until);
				Self::deposit_event(Event::CooldownStarted { target: target.clone(), until });
			}

			Self::deposit_event(Event::Unpaused { target, slashed: slash });
			Ok(Some(T::WeightInfo::unpause(pause.deposits.len() as u32)).into())
		}

		/// Move the end of a running pause to `until`.
		///
		/// The Fellowship can push a pause out by at most [`Config::MaxFellowshipExtension`] in
		/// total. Root has no limit.
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::force_extend())]
		pub fn force_extend(
			origin: OriginFor<T>,
			target: T::Target,
			until: BlockNumberOf<T>,
		) -> DispatchResult {
			let is_force = Self::ensure_fellowship_or_force(origin)?;

			let mut pause = Pauses::<T>::get(&target).ok_or(Error::<T>::NotPaused)?;
			let current = pause.until.ok_or(Error::<T>::OpenEnded)?;
			ensure!(until > current, Error::<T>::NotLater);
			if !is_force {
				let total = pause.fellowship_extension.saturating_add(until - current);
				ensure!(total <= T::MaxFellowshipExtension::get(), Error::<T>::ExtensionTooLong);
				pause.fellowship_extension = total;
			}
			Self::reschedule_expiry(&target, &mut pause, until);
			Pauses::<T>::insert(&target, pause);

			Self::deposit_event(Event::Extended { target, who: None, until });
			Ok(())
		}

		/// Pause `target` without a deposit. `until: None` means it only ends through
		/// [`Call::unpause`].
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::force_pause())]
		pub fn force_pause(
			origin: OriginFor<T>,
			target: T::Target,
			until: Option<BlockNumberOf<T>>,
		) -> DispatchResult {
			T::ForceOrigin::ensure_origin(origin)?;
			let now = T::BlockNumberProvider::current_block_number();

			ensure!(!Pauses::<T>::contains_key(&target), Error::<T>::AlreadyPaused);
			if let Some(until) = until {
				ensure!(until > now, Error::<T>::InPast);
			}
			ensure!(T::Pauser::can_pause(target.clone()), Error::<T>::NotPausable);
			Self::do_pause(&target, until, Default::default())?;

			Self::deposit_event(Event::Paused { target, who: None, until });
			Ok(())
		}

		/// End a pause that is past its end. The `on_poll` hook normally does this.
		///
		/// The deposits stay on hold until [`Call::release_deposit`] or [`Call::slash_deposit`].
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::expire())]
		pub fn expire(origin: OriginFor<T>, target: T::Target) -> DispatchResult {
			ensure_signed(origin)?;
			let now = T::BlockNumberProvider::current_block_number();

			let pause = Pauses::<T>::get(&target).ok_or(Error::<T>::NotPaused)?;
			let until = pause.until.ok_or(Error::<T>::OpenEnded)?;
			ensure!(until <= now, Error::<T>::NotExpired);
			Self::do_unpause(&target, &pause);

			Self::deposit_event(Event::Expired { target });
			Ok(())
		}

		/// Give back a deposit once [`Config::ReleaseDelay`] has passed since it was made.
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::release_deposit())]
		pub fn release_deposit(origin: OriginFor<T>, id: DepositId) -> DispatchResult {
			ensure_signed(origin)?;
			let now = T::BlockNumberProvider::current_block_number();

			let deposit = Deposits::<T>::get(id).ok_or(Error::<T>::UnknownDeposit)?;
			ensure!(!Self::deposit_in_use(id, &deposit.target), Error::<T>::StillPaused);
			ensure!(
				now >= deposit.created_at.saturating_add(T::ReleaseDelay::get()),
				Error::<T>::TooEarly
			);
			Self::settle_deposit(id, false)
		}

		/// Slash the deposit of a pause that already ended.
		///
		/// For a running pause use [`Call::unpause`].
		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::slash_deposit())]
		pub fn slash_deposit(origin: OriginFor<T>, id: DepositId) -> DispatchResult {
			Self::ensure_fellowship_or_force(origin)?;

			let deposit = Deposits::<T>::get(id).ok_or(Error::<T>::UnknownDeposit)?;
			ensure!(!Self::deposit_in_use(id, &deposit.target), Error::<T>::StillPaused);
			Self::settle_deposit(id, true)
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Returns `true` for [`Config::ForceOrigin`] and `false` for [`Config::FellowshipOrigin`].
	fn ensure_fellowship_or_force(origin: OriginFor<T>) -> Result<bool, DispatchError> {
		match T::ForceOrigin::try_origin(origin) {
			Ok(_) => Ok(true),
			Err(origin) =>
				T::FellowshipOrigin::ensure_origin(origin).map(|_| false).map_err(Into::into),
		}
	}

	fn hold_deposit(
		who: &T::AccountId,
		target: &T::Target,
		now: BlockNumberOf<T>,
	) -> Result<DepositId, DispatchError> {
		let amount = T::DepositFor::convert(target.clone()).ok_or(Error::<T>::NotPausable)?;
		T::Currency::hold(&HoldReason::Pause.into(), who, amount)?;

		let id = NextDepositId::<T>::mutate(|next| {
			let id = *next;
			*next = next.saturating_add(1);
			id
		});
		Deposits::<T>::insert(
			id,
			DepositInfo { who: who.clone(), amount, target: target.clone(), created_at: now },
		);
		Self::deposit_event(Event::DepositHeld { id, who: who.clone(), amount });
		Ok(id)
	}

	fn settle_deposit(id: DepositId, slash: bool) -> DispatchResult {
		let deposit = Deposits::<T>::take(id).ok_or(Error::<T>::UnknownDeposit)?;
		let reason = HoldReason::Pause.into();
		let DepositInfo { who, amount, .. } = deposit;

		if slash {
			let (credit, missing) =
				<T::Currency as BalancedHold<T::AccountId>>::slash(&reason, &who, amount);
			if !missing.is_zero() {
				log::warn!(target: LOG_TARGET, "deposit {id} was short by {missing:?} when slashed");
			}
			T::OnSlash::on_unbalanced(credit);
			Self::deposit_event(Event::DepositSlashed { id, who, amount: amount - missing });
		} else {
			let released = T::Currency::release(&reason, &who, amount, Precision::BestEffort)?;
			Self::deposit_event(Event::DepositReleased { id, who, amount: released });
		}
		Ok(())
	}

	fn deposit_in_use(id: DepositId, target: &T::Target) -> bool {
		Pauses::<T>::get(target).is_some_and(|pause| pause.deposits.contains(&id))
	}

	fn do_pause(
		target: &T::Target,
		until: Option<BlockNumberOf<T>>,
		deposits: BoundedVec<DepositId, T::MaxDepositsPerPause>,
	) -> DispatchResult {
		T::Pauser::pause(target.clone()).map_err(|_| Error::<T>::PauseFailed)?;
		if let Some(until) = until {
			Self::schedule_expiry(until, target);
		}
		Pauses::<T>::insert(
			target,
			PauseInfo { until, fellowship_extension: Zero::zero(), deposits },
		);
		Ok(())
	}

	/// Removes the pause and lifts it. Doesn't touch the deposits.
	fn do_unpause(target: &T::Target, pause: &PauseInfo<T>) {
		if let Some(until) = pause.until {
			Self::unschedule_expiry(until, target);
		}
		Pauses::<T>::remove(target);
		// The pauser must be able to lift anything it paused, so this can't fail as long as
		// nothing else touches it.
		if T::Pauser::unpause(target.clone()).is_err() {
			log::error!(target: LOG_TARGET, "pauser could not lift the pause of {target:?}");
		}
	}

	fn reschedule_expiry(target: &T::Target, pause: &mut PauseInfo<T>, until: BlockNumberOf<T>) {
		if let Some(old) = pause.until {
			Self::unschedule_expiry(old, target);
		}
		Self::schedule_expiry(until, target);
		pause.until = Some(until);
	}

	fn schedule_expiry(at: BlockNumberOf<T>, target: &T::Target) {
		Expiries::<T>::insert(at, target, ());
		ExpiryCount::<T>::mutate(|count| count.saturating_inc());
		// The cursor must not be past anything queued, or the hook skips it.
		ExpiryCursor::<T>::mutate(|cursor| {
			if cursor.map_or(true, |cursor| cursor > at) {
				*cursor = Some(at);
			}
		});
	}

	fn unschedule_expiry(at: BlockNumberOf<T>, target: &T::Target) {
		if Expiries::<T>::take(at, target).is_some() {
			let count = ExpiryCount::<T>::mutate(|count| {
				count.saturating_dec();
				*count
			});
			if count == 0 {
				ExpiryCursor::<T>::kill();
			}
		}
	}

	/// Ends every pause that is due, as far as `meter` and [`Config::MaxPollSteps`] allow. The
	/// rest is picked up in the next block.
	pub(crate) fn process_expiries(meter: &mut WeightMeter) {
		if meter.try_consume(T::WeightInfo::on_poll_base()).is_err() {
			return;
		}
		let Some(mut cursor) = ExpiryCursor::<T>::get() else { return };
		let now = T::BlockNumberProvider::current_block_number();

		let mut steps = 0;
		while cursor <= now && steps < T::MaxPollSteps::get() {
			steps += 1;
			if meter.try_consume(T::WeightInfo::on_poll_step()).is_err() {
				break;
			}
			match Expiries::<T>::iter_key_prefix(cursor).next() {
				None => cursor.saturating_inc(),
				Some(target) => {
					if meter.try_consume(T::WeightInfo::on_poll_expire()).is_err() {
						break;
					}
					Self::do_expire(cursor, target);
				},
			}
		}

		// `unschedule_expiry` clears the cursor once the queue is empty.
		if ExpiryCursor::<T>::exists() {
			ExpiryCursor::<T>::put(cursor);
		}
	}

	fn do_expire(at: BlockNumberOf<T>, target: T::Target) {
		match Pauses::<T>::get(&target) {
			Some(pause) if pause.until == Some(at) => {
				Self::do_unpause(&target, &pause);
				Self::deposit_event(Event::Expired { target });
			},
			_ => {
				log::error!(target: LOG_TARGET, "stale expiry of {target:?} at {at:?}, dropping it");
				Self::unschedule_expiry(at, &target);
			},
		}
	}
}
