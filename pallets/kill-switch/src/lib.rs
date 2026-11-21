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
//! A [`Target`] is either a single call or a group of calls. The runtime defines the groups
//! through [`Config::Groups`], each with its own deposit, like everything around a stablecoin. Any
//! call that [`Config::ValidCall`] accepts can also be paused on its own. That costs
//! [`Config::CallDeposit`], or the deposit of the priciest group the call is in, and isn't possible
//! for a call of a group only Root can pause. Either way the calls are paused through
//! [`Config::TransactionPause`].
//!
//! - [`Call::pause`]: anyone. Holds the target's deposit and pauses its calls. The pause ends by
//!   itself after [`Config::PauseDuration`].
//! - [`Call::extend`]: anyone. Holds another deposit and pushes the end out by another
//!   [`Config::PauseDuration`].
//! - [`Call::unpause`]: [`Config::AdminOrigin`]. Ends the pause and slashes or refunds every
//!   deposit behind it. A slash also puts the target on cooldown for [`Config::Cooldown`], along
//!   with each of its calls on its own, so it can't be paused again right away.
//! - [`Call::force_extend`]: [`Config::AdminOrigin`]. Moves the end of a pause.
//! - [`Call::force_pause`]: [`Config::AdminOrigin`]. No deposit and no cooldown check. Takes over a
//!   pause someone paid for and refunds it. Only whoever can start such a pause can end or move it.
//!
//! [`Config::AdminOrigin`] gets told what the caller wants to do as an [`AdminAction`], so the
//! runtime decides who can do what. Usually the Fellowship can unpause and extend, and Root can do
//! everything.
//!
//! A pause that nobody ends runs out by itself and its deposits are given back. If the Fellowship
//! thinks a pause was abuse, it has to slash it with [`Call::unpause`] while it is still active.
//!
//! Time is measured with [`Config::BlockNumberProvider`], usually relay chain blocks. Pauses with
//! an end are kept in one list of ids, sorted by when they end, and `on_poll` ends the ones that
//! are due. There are only ever a handful of pauses, so a single bounded list is enough. Single
//! calls can't take the slots the groups need, so cheap call pauses can't stop a group from being
//! paused.
//!
//! Targets can share calls. A call stays paused as long as any active pause holds it, and every
//! pause keeps the list of calls it paused, so a runtime upgrade that changes a target can't leave
//! a call paused or unpaused by mistake.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
pub mod runtime_api;
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
		Contains, EnsureOriginWithArg, OnUnbalanced, TransactionPause,
	},
	weights::WeightMeter,
};
use frame_system::pallet_prelude::*;
use sp_runtime::traits::{BlockNumberProvider, Saturating, Zero};

/// The log target of this pallet.
pub const LOG_TARGET: &str = "runtime::kill-switch";

/// Block number of [`Config::BlockNumberProvider`].
pub type BlockNumberOf<T> =
	<<T as Config>::BlockNumberProvider as BlockNumberProvider>::BlockNumber;
pub type BalanceOf<T> =
	<<T as Config>::Currency as Inspect<<T as frame_system::Config>::AccountId>>::Balance;
pub type CreditOf<T> = Credit<<T as frame_system::Config>::AccountId, <T as Config>::Currency>;
/// The calls one pause holds.
pub type CallsOf<T> = BoundedVec<<T as Config>::CallName, <T as Config>::MaxCallsPerTarget>;
pub type TargetOf<T> = Target<<T as Config>::Group, <T as Config>::CallName>;
/// Identifies a pause in the expiry list.
pub type PauseId = u32;

/// A pause in the expiry list.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
pub struct Expiry<BlockNumber> {
	/// When the pause ends.
	pub at: BlockNumber,
	/// Which pause it is.
	pub id: PauseId,
	/// Whether it pauses a single call. Those only get the slots the groups don't need.
	pub single_call: bool,
}

/// Something that can be paused.
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
)]
pub enum Target<Group, CallName> {
	/// A group of calls the runtime defines.
	#[codec(index = 0)]
	Group(Group),
	/// A single call.
	#[codec(index = 1)]
	Call(CallName),
}

/// A group as the runtime offers it.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub struct GroupInfo<Group, CallName, Balance> {
	/// The group itself.
	pub group: Group,
	/// The calls pausing it pauses.
	pub calls: alloc::vec::Vec<CallName>,
	/// The deposit for pausing or extending it. `None` means only Root can pause it.
	pub deposit: Option<Balance>,
}

/// What [`Config::AdminOrigin`] is asked to allow.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AdminAction<Target> {
	/// End a pause with [`Call::unpause`].
	Unpause(Target),
	/// Move the end of a pause with [`Call::force_extend`].
	Extend(Target),
	/// Start a pause with [`Call::force_pause`]. Also needed to end or move a pause started that
	/// way.
	ForcePause(Target),
}

/// The groups of calls a runtime lets people pause.
pub trait PauseGroups<Group, CallName, Balance> {
	/// Every group, so they can be listed and checked.
	fn all() -> alloc::vec::Vec<Group>;

	/// The calls pausing `group` pauses. Empty if `group` isn't one of [`Self::all`].
	fn calls(group: &Group) -> alloc::vec::Vec<CallName>;

	/// The deposit for pausing or extending `group`. `None` means only [`Call::force_pause`] can
	/// pause it.
	fn deposit(group: &Group) -> Option<Balance>;
}

/// An active pause.
#[derive(
	Encode, Decode, CloneNoBound, PartialEqNoBound, EqNoBound, DebugNoBound, TypeInfo, MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct PauseInfo<T: Config> {
	/// Its id in the expiry list.
	pub id: PauseId,
	/// The calls this pause holds, as the target had them when the pause started.
	pub calls: CallsOf<T>,
	/// When the pause ends by itself. `None` means it only ends through [`Call::unpause`].
	pub until: Option<BlockNumberOf<T>>,
	/// The deposits behind this pause, from [`Call::pause`] and [`Call::extend`].
	pub deposits: BoundedVec<Deposit<T>, T::MaxDepositsPerPause>,
	/// Started through [`Call::force_pause`]. Ending or moving it needs
	/// [`AdminAction::ForcePause`].
	pub forced: bool,
}

/// A deposit on hold behind a pause.
#[derive(
	Encode, Decode, CloneNoBound, PartialEqNoBound, EqNoBound, DebugNoBound, TypeInfo, MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct Deposit<T: Config> {
	/// Who put it up.
	pub who: T::AccountId,
	/// How much is on hold.
	pub amount: BalanceOf<T>,
}

/// Benchmark setup that only the runtime knows.
#[cfg(feature = "runtime-benchmarks")]
pub trait BenchmarkHelper<Group> {
	/// A group anyone can pause. Should be the one with the most calls.
	fn group() -> Group;

	/// Turns on pausing with a deposit, if the runtime ships with it off.
	fn enable() {}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config<RuntimeEvent: From<Event<Self>>> {
		/// A group of calls that can be paused together. Defined by the runtime.
		type Group: Parameter + MaxEncodedLen;

		/// The name of a single call, the `(pallet, call)` pair `pallet-tx-pause` uses.
		type CallName: Parameter + MaxEncodedLen;

		/// What each group pauses and what it costs.
		///
		/// None of the calls may be this pallet's own, or anything the Fellowship needs to reach
		/// [`Call::unpause`].
		type Groups: PauseGroups<Self::Group, Self::CallName, BalanceOf<Self>>;

		/// Calls that can be paused on their own. Usually every call that exists, tx-pause still
		/// refuses the ones it never pauses.
		type ValidCall: Contains<Self::CallName>;

		/// The least it costs to pause or extend a single call. A call in a group costs at least
		/// what the group costs. Zero means single calls can only be paused by
		/// [`Call::force_pause`].
		type CallDeposit: Get<BalanceOf<Self>>;

		/// Pauses and unpauses single calls. Nothing else should be able to touch it.
		type TransactionPause: TransactionPause<CallIdentifier = Self::CallName>;

		/// Holds the deposits.
		type Currency: MutateHold<Self::AccountId, Reason = Self::RuntimeHoldReason>
			+ BalancedHold<Self::AccountId>
			+ Mutate<Self::AccountId>;

		/// The overarching hold reason.
		type RuntimeHoldReason: From<HoldReason>;

		/// Where slashed deposits go.
		type OnSlash: OnUnbalanced<CreditOf<Self>>;

		/// Who can unpause, extend without a deposit, and force pause. See [`AdminAction`].
		type AdminOrigin: EnsureOriginWithArg<Self::RuntimeOrigin, AdminAction<TargetOf<Self>>>;

		/// Measures time for pauses and cooldowns.
		type BlockNumberProvider: BlockNumberProvider;

		/// How long a pause lasts, and how much each [`Call::extend`] adds. Zero means nothing can
		/// be paused with a deposit.
		type PauseDuration: Get<BlockNumberOf<Self>>;

		/// How long the calls of a pause can't be paused again after an unpause with slash.
		type Cooldown: Get<BlockNumberOf<Self>>;

		/// How many calls one group can pause.
		#[pallet::constant]
		type MaxCallsPerTarget: Get<u32>;

		/// How many deposits can back one pause.
		#[pallet::constant]
		type MaxDepositsPerPause: Get<u32>;

		/// How many pauses with an end can be active at once. Must be more than the number of
		/// groups, single calls only get what the groups don't need.
		#[pallet::constant]
		type MaxActivePauses: Get<u32>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;

		#[cfg(feature = "runtime-benchmarks")]
		type BenchmarkHelper: BenchmarkHelper<Self::Group>;
	}

	#[pallet::composite_enum]
	pub enum HoldReason {
		/// Deposit for pausing or extending a pause.
		#[codec(index = 0)]
		Pause,
	}

	/// Active pauses.
	#[pallet::storage]
	pub type Pauses<T: Config> = StorageMap<_, Blake2_128Concat, TargetOf<T>, PauseInfo<T>>;

	/// The target of each active pause, by id.
	#[pallet::storage]
	pub type PauseTargets<T: Config> = StorageMap<_, Twox64Concat, PauseId, TargetOf<T>>;

	/// The id the next pause gets.
	#[pallet::storage]
	pub type NextPauseId<T: Config> = StorageValue<_, PauseId, ValueQuery>;

	/// Pauses that end by themselves, sorted by the block they end at.
	#[pallet::storage]
	pub type Expiries<T: Config> =
		StorageValue<_, BoundedVec<Expiry<BlockNumberOf<T>>, T::MaxActivePauses>, ValueQuery>;

	/// How many active pauses hold each call. A call is only unpaused once none do.
	#[pallet::storage]
	pub type PausedCalls<T: Config> = StorageMap<_, Blake2_128Concat, T::CallName, u32, ValueQuery>;

	/// Targets that can't be paused until the given block, except by [`Call::force_pause`].
	#[pallet::storage]
	pub type Cooldowns<T: Config> = StorageMap<_, Blake2_128Concat, TargetOf<T>, BlockNumberOf<T>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A target was paused. `who` is `None` for [`Call::force_pause`].
		Paused { target: TargetOf<T>, who: Option<T::AccountId>, until: Option<BlockNumberOf<T>> },
		/// A pause now ends later. `who` is `None` for [`Call::force_extend`].
		Extended { target: TargetOf<T>, who: Option<T::AccountId>, until: BlockNumberOf<T> },
		/// The Fellowship or Root ended a pause.
		Unpaused { target: TargetOf<T>, slashed: bool },
		/// A pause ran out and its deposits were given back.
		Expired { target: TargetOf<T> },
		/// `target` and each of its calls can't be paused again until `until`.
		CooldownStarted { target: TargetOf<T>, until: BlockNumberOf<T> },
		/// A deposit was put on hold.
		DepositHeld { target: TargetOf<T>, who: T::AccountId, amount: BalanceOf<T> },
		/// A deposit was given back.
		DepositReleased { target: TargetOf<T>, who: T::AccountId, amount: BalanceOf<T> },
		/// A deposit was slashed.
		DepositSlashed { target: TargetOf<T>, who: T::AccountId, amount: BalanceOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The target doesn't exist, has no deposit, or one of its calls can't be paused.
		NotPausable,
		/// The target is already paused.
		AlreadyPaused,
		/// The target isn't paused.
		NotPaused,
		/// A pause of this target was slashed recently, it can't be paused again yet.
		InCooldown,
		/// The pause has no end. It only ends through `unpause`.
		OpenEnded,
		/// The pause already ran out, it just hasn't been cleaned up yet.
		AlreadyExpired,
		/// The new end isn't later than the current one.
		NotLater,
		/// The end is not in the future.
		InPast,
		/// The pause already has the maximum number of deposits.
		TooManyDeposits,
		/// `TransactionPause` refused to pause one of the calls.
		PauseFailed,
		/// The pause was started with `force_pause`, so it can't be extended with a deposit.
		ForcedPause,
		/// There's no room left for another pause with an end.
		TooManyPauses,
		/// The group pauses more than [`Config::MaxCallsPerTarget`] calls.
		TooManyCalls,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_poll(_: BlockNumberFor<T>, meter: &mut WeightMeter) {
			Self::process_expiries(meter);
		}

		fn integrity_test() {
			assert!(T::MaxDepositsPerPause::get() > 0, "a pause needs room for its first deposit");
			assert!(T::MaxActivePauses::get() > 0, "there must be room for at least one pause");

			let groups = T::Groups::all();
			assert!(
				groups.len() < T::MaxActivePauses::get() as usize,
				"every group must be able to be paused at the same time, with room left for calls"
			);
			for group in groups {
				let calls = T::Groups::calls(&group);
				assert!(!calls.is_empty(), "a group has no calls: {group:?}");
				assert!(
					calls.len() <= T::MaxCallsPerTarget::get() as usize,
					"a group has too many calls: {group:?}"
				);
				assert!(
					calls.iter().all(T::ValidCall::contains),
					"a group has a call that doesn't exist: {group:?}"
				);
			}
		}

		#[cfg(feature = "try-runtime")]
		fn try_state(_: BlockNumberFor<T>) -> Result<(), sp_runtime::TryRuntimeError> {
			Self::do_try_state()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Pause `target` by putting up its deposit.
		///
		/// The pause ends by itself after [`Config::PauseDuration`], unless someone extends it or
		/// the Fellowship or Root ends it earlier.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::pause())]
		pub fn pause(origin: OriginFor<T>, target: TargetOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let now = T::BlockNumberProvider::current_block_number();

			ensure!(!Pauses::<T>::contains_key(&target), Error::<T>::AlreadyPaused);
			if let Some(end) = Cooldowns::<T>::get(&target) {
				ensure!(now >= end, Error::<T>::InCooldown);
				Cooldowns::<T>::remove(&target);
			}
			let calls = Self::calls_to_pause(&target)?;

			let deposit = Self::hold_deposit(&who, &target)?;
			let until = now.saturating_add(Self::pause_duration()?);
			let deposits = BoundedVec::truncate_from(alloc::vec![deposit]);
			Self::do_pause(&target, calls, Some(until), deposits, false)?;

			Self::deposit_event(Event::Paused { target, who: Some(who), until: Some(until) });
			Ok(())
		}

		/// Push the end of an active pause out by [`Config::PauseDuration`], with another
		/// deposit.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::extend())]
		pub fn extend(origin: OriginFor<T>, target: TargetOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let now = T::BlockNumberProvider::current_block_number();

			let mut pause = Pauses::<T>::get(&target).ok_or(Error::<T>::NotPaused)?;
			// Otherwise anyone could stretch a Root pause that the Fellowship then can't lift.
			ensure!(!pause.forced, Error::<T>::ForcedPause);
			let until = pause.until.ok_or(Error::<T>::OpenEnded)?;
			// If the hook is behind, the pause is still stored after it ran out. Extending it
			// then would bring it back to life.
			ensure!(until > now, Error::<T>::AlreadyExpired);
			ensure!(
				pause.deposits.len() < T::MaxDepositsPerPause::get() as usize,
				Error::<T>::TooManyDeposits
			);

			let deposit = Self::hold_deposit(&who, &target)?;
			pause.deposits.try_push(deposit).map_err(|_| Error::<T>::TooManyDeposits)?;
			let new_until = until.saturating_add(Self::pause_duration()?);
			Self::reschedule_expiry(&target, &mut pause, new_until)?;
			Pauses::<T>::insert(&target, pause);

			Self::deposit_event(Event::Extended { target, who: Some(who), until: new_until });
			Ok(())
		}

		/// End a pause and slash or refund every deposit behind it.
		///
		/// A slash also puts `target` and each of its calls on cooldown for [`Config::Cooldown`].
		/// Only the calls on their own, so a cheap single call that gets slashed can't block the
		/// groups it's in.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::unpause(T::MaxDepositsPerPause::get()))]
		pub fn unpause(
			origin: OriginFor<T>,
			target: TargetOf<T>,
			slash: bool,
		) -> DispatchResultWithPostInfo {
			let now = T::BlockNumberProvider::current_block_number();

			let pause = Pauses::<T>::get(&target).ok_or(Error::<T>::NotPaused)?;
			Self::ensure_admin(origin, &pause, AdminAction::Unpause(target.clone()))?;
			// A forced pause has no deposit, so there's nothing to slash and no cooldown.
			let slash = slash && !pause.deposits.is_empty();
			Self::do_unpause(&target, &pause);
			Self::settle_deposits(&target, &pause.deposits, slash);

			let cooldown = T::Cooldown::get();
			if slash && !cooldown.is_zero() {
				let until = now.saturating_add(cooldown);
				Cooldowns::<T>::insert(&target, until);
				// Otherwise the same calls could be paused again one by one.
				for call in pause.calls.iter() {
					Cooldowns::<T>::insert(Target::Call(call.clone()), until);
				}
				Self::deposit_event(Event::CooldownStarted { target: target.clone(), until });
			}

			Self::deposit_event(Event::Unpaused { target, slashed: slash });
			Ok(Some(T::WeightInfo::unpause(pause.deposits.len() as u32)).into())
		}

		/// Move the end of a pause to `until`.
		///
		/// This also works on a pause that ran out but that the hook hasn't ended yet, as long as
		/// `until` is in the future.
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::force_extend())]
		pub fn force_extend(
			origin: OriginFor<T>,
			target: TargetOf<T>,
			until: BlockNumberOf<T>,
		) -> DispatchResult {
			let now = T::BlockNumberProvider::current_block_number();

			let mut pause = Pauses::<T>::get(&target).ok_or(Error::<T>::NotPaused)?;
			Self::ensure_admin(origin, &pause, AdminAction::Extend(target.clone()))?;
			let current = pause.until.ok_or(Error::<T>::OpenEnded)?;
			ensure!(until > current, Error::<T>::NotLater);
			ensure!(until > now, Error::<T>::InPast);
			Self::reschedule_expiry(&target, &mut pause, until)?;
			Pauses::<T>::insert(&target, pause);

			Self::deposit_event(Event::Extended { target, who: None, until });
			Ok(())
		}

		/// Pause `target` without a deposit. `until: None` means it only ends through
		/// [`Call::unpause`].
		///
		/// If someone already paused `target` with a deposit, this takes the pause over and gives
		/// the deposits back.
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::force_pause(T::MaxDepositsPerPause::get()))]
		pub fn force_pause(
			origin: OriginFor<T>,
			target: TargetOf<T>,
			until: Option<BlockNumberOf<T>>,
		) -> DispatchResultWithPostInfo {
			T::AdminOrigin::ensure_origin(origin, &AdminAction::ForcePause(target.clone()))?;
			let now = T::BlockNumberProvider::current_block_number();

			if let Some(until) = until {
				ensure!(until > now, Error::<T>::InPast);
			}
			let mut refunded = 0;
			if let Some(pause) = Pauses::<T>::get(&target) {
				ensure!(!pause.forced, Error::<T>::AlreadyPaused);
				Self::do_unpause(&target, &pause);
				Self::settle_deposits(&target, &pause.deposits, false);
				refunded = pause.deposits.len() as u32;
				Self::deposit_event(Event::Unpaused { target: target.clone(), slashed: false });
			}
			let calls = Self::calls_to_pause(&target)?;
			Self::do_pause(&target, calls, until, Default::default(), true)?;

			Self::deposit_event(Event::Paused { target, who: None, until });
			Ok(Some(T::WeightInfo::force_pause(refunded)).into())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Checks that `origin` can do `action` to `pause`. A pause started with
	/// [`Call::force_pause`] needs [`AdminAction::ForcePause`] for anything.
	fn ensure_admin(
		origin: OriginFor<T>,
		pause: &PauseInfo<T>,
		action: AdminAction<TargetOf<T>>,
	) -> DispatchResult {
		let action = match action {
			AdminAction::Unpause(target) | AdminAction::Extend(target) if pause.forced =>
				AdminAction::ForcePause(target),
			action => action,
		};
		T::AdminOrigin::ensure_origin(origin, &action)?;
		Ok(())
	}

	/// Every group the runtime offers, with its calls and deposit. For the runtime API.
	pub fn pause_groups() -> alloc::vec::Vec<GroupInfo<T::Group, T::CallName, BalanceOf<T>>> {
		T::Groups::all()
			.into_iter()
			.map(|group| GroupInfo {
				calls: T::Groups::calls(&group),
				deposit: Self::group_deposit(&group),
				group,
			})
			.collect()
	}

	/// What pausing `group` costs. `None` if only Root can pause it.
	fn group_deposit(group: &T::Group) -> Option<BalanceOf<T>> {
		// A free pause would make the whole thing pointless, so zero means it's off.
		T::Groups::deposit(group).filter(|deposit| !deposit.is_zero())
	}

	/// What pausing `call` on its own costs. `None` if it can't be paused on its own with a
	/// deposit: it doesn't exist, tx-pause never pauses it, single calls are off, or a group it's
	/// in can only be paused by Root.
	pub fn call_deposit(call: &T::CallName) -> Option<BalanceOf<T>> {
		if !T::ValidCall::contains(call) || !Self::can_pause(call) {
			return None;
		}
		let mut deposit = Some(T::CallDeposit::get()).filter(|deposit| !deposit.is_zero())?;
		for group in T::Groups::all() {
			if T::Groups::calls(&group).contains(call) {
				deposit = deposit.max(Self::group_deposit(&group)?);
			}
		}
		Some(deposit)
	}

	/// Whether tx-pause would pause `call`. A call another pause already holds looks unpausable
	/// to tx-pause, but that's fine.
	fn can_pause(call: &T::CallName) -> bool {
		PausedCalls::<T>::contains_key(call) || T::TransactionPause::can_pause(call.clone())
	}

	/// How long a pause lasts. Zero means pausing with a deposit is off.
	fn pause_duration() -> Result<BlockNumberOf<T>, DispatchError> {
		let duration = T::PauseDuration::get();
		ensure!(!duration.is_zero(), Error::<T>::NotPausable);
		Ok(duration)
	}

	/// The calls `target` pauses, if all of them can be paused.
	fn calls_to_pause(target: &TargetOf<T>) -> Result<CallsOf<T>, DispatchError> {
		let calls = match target {
			Target::Group(group) => T::Groups::calls(group),
			Target::Call(call) => {
				ensure!(T::ValidCall::contains(call), Error::<T>::NotPausable);
				alloc::vec![call.clone()]
			},
		};
		ensure!(!calls.is_empty(), Error::<T>::NotPausable);
		let calls = CallsOf::<T>::try_from(calls).map_err(|_| Error::<T>::TooManyCalls)?;
		ensure!(calls.iter().all(Self::can_pause), Error::<T>::NotPausable);
		Ok(calls)
	}

	fn hold_deposit(who: &T::AccountId, target: &TargetOf<T>) -> Result<Deposit<T>, DispatchError> {
		let amount = match target {
			Target::Group(group) => Self::group_deposit(group).ok_or(Error::<T>::NotPausable)?,
			Target::Call(call) => Self::call_deposit(call).ok_or(Error::<T>::NotPausable)?,
		};
		T::Currency::hold(&HoldReason::Pause.into(), who, amount)?;

		Self::deposit_event(Event::DepositHeld {
			target: target.clone(),
			who: who.clone(),
			amount,
		});
		Ok(Deposit { who: who.clone(), amount })
	}

	/// Slashes or gives back the deposits of a pause that already ended.
	///
	/// This can't fail, the pause is gone either way. A release only fails if the account can't
	/// take the funds back, which shouldn't happen for an account that holds them.
	fn settle_deposits(target: &TargetOf<T>, deposits: &[Deposit<T>], slash: bool) {
		let reason = HoldReason::Pause.into();
		for Deposit { who, amount } in deposits.iter().cloned() {
			if slash {
				let (credit, missing) =
					<T::Currency as BalancedHold<T::AccountId>>::slash(&reason, &who, amount);
				if !missing.is_zero() {
					log::warn!(target: LOG_TARGET, "deposit of {who:?} was short by {missing:?}");
				}
				T::OnSlash::on_unbalanced(credit);
				let amount = amount.saturating_sub(missing);
				Self::deposit_event(Event::DepositSlashed { target: target.clone(), who, amount });
			} else {
				match T::Currency::release(&reason, &who, amount, Precision::BestEffort) {
					Ok(amount) => Self::deposit_event(Event::DepositReleased {
						target: target.clone(),
						who,
						amount,
					}),
					Err(e) => {
						log::error!(target: LOG_TARGET, "could not release deposit of {who:?}: {e:?}");
						frame_support::defensive!("a deposit could not be released");
					},
				}
			}
		}
	}

	fn do_pause(
		target: &TargetOf<T>,
		calls: CallsOf<T>,
		until: Option<BlockNumberOf<T>>,
		deposits: BoundedVec<Deposit<T>, T::MaxDepositsPerPause>,
		forced: bool,
	) -> DispatchResult {
		let id = NextPauseId::<T>::mutate(|next| {
			let id = *next;
			*next = next.wrapping_add(1);
			id
		});
		if let Some(until) = until {
			Self::schedule_expiry(until, id, target)?;
		}
		for call in calls.iter() {
			Self::hold_call(call)?;
		}
		PauseTargets::<T>::insert(id, target);
		Pauses::<T>::insert(target, PauseInfo { id, calls, until, deposits, forced });
		Ok(())
	}

	/// Takes the pause off the expiry list, removes it and lifts its calls. Doesn't touch the
	/// deposits.
	fn do_unpause(target: &TargetOf<T>, pause: &PauseInfo<T>) {
		if pause.until.is_some() {
			Self::unschedule_expiry(pause.id);
		}
		Self::lift(target, pause);
	}

	/// Removes the pause and lifts its calls.
	fn lift(target: &TargetOf<T>, pause: &PauseInfo<T>) {
		Pauses::<T>::remove(target);
		PauseTargets::<T>::remove(pause.id);
		for call in pause.calls.iter() {
			Self::release_call(call);
		}
	}

	/// Counts one more pause holding `call`, and pauses it if it's the first.
	fn hold_call(call: &T::CallName) -> DispatchResult {
		PausedCalls::<T>::try_mutate(call, |holders| {
			if *holders == 0 {
				T::TransactionPause::pause(call.clone()).map_err(|_| Error::<T>::PauseFailed)?;
			}
			holders.saturating_inc();
			Ok(())
		})
	}

	/// Counts one pause less holding `call`, and unpauses it once none do.
	fn release_call(call: &T::CallName) {
		let holders = PausedCalls::<T>::get(call).saturating_sub(1);
		if holders > 0 {
			PausedCalls::<T>::insert(call, holders);
			return;
		}
		PausedCalls::<T>::remove(call);
		// Nothing else can touch tx-pause, so anything we paused is still paused.
		if T::TransactionPause::unpause(call.clone()).is_err() {
			frame_support::defensive!("could not unpause a call the kill switch paused");
		}
	}

	/// Ends a pause that ran out and gives its deposits back. The hook already took it off the
	/// expiry list.
	pub(crate) fn do_expire(target: &TargetOf<T>, pause: &PauseInfo<T>) {
		Self::lift(target, pause);
		Self::settle_deposits(target, &pause.deposits, false);
		Self::deposit_event(Event::Expired { target: target.clone() });
	}

	fn reschedule_expiry(
		target: &TargetOf<T>,
		pause: &mut PauseInfo<T>,
		until: BlockNumberOf<T>,
	) -> DispatchResult {
		if pause.until.is_some() {
			Self::unschedule_expiry(pause.id);
		}
		Self::schedule_expiry(until, pause.id, target)?;
		pause.until = Some(until);
		Ok(())
	}

	/// How many slots of the expiry list single calls can use. The rest is kept for the groups.
	fn room_for_calls() -> usize {
		(T::MaxActivePauses::get() as usize).saturating_sub(T::Groups::all().len())
	}

	fn schedule_expiry(at: BlockNumberOf<T>, id: PauseId, target: &TargetOf<T>) -> DispatchResult {
		let single_call = matches!(target, Target::Call(_));
		Expiries::<T>::try_mutate(|queue| {
			// A group always fits, since every group takes at most one slot and single calls
			// leave room for all of them.
			if single_call {
				let calls = queue.iter().filter(|expiry| expiry.single_call).count();
				ensure!(calls < Self::room_for_calls(), Error::<T>::TooManyPauses);
			}
			// After everything that ends at the same block or earlier, so the list stays sorted.
			let index = queue.partition_point(|expiry| expiry.at <= at);
			queue
				.try_insert(index, Expiry { at, id, single_call })
				.map_err(|_| Error::<T>::TooManyPauses)
		})?;
		Ok(())
	}

	fn unschedule_expiry(id: PauseId) {
		Expiries::<T>::mutate(|queue| queue.retain(|expiry| expiry.id != id));
	}

	/// Ends every pause that is due, as far as `meter` allows. The rest is picked up in the next
	/// block.
	pub(crate) fn process_expiries(meter: &mut WeightMeter) {
		if meter.try_consume(T::WeightInfo::on_poll_base()).is_err() {
			return;
		}
		let now = T::BlockNumberProvider::current_block_number();
		let mut queue = Expiries::<T>::get();

		let mut due = 0;
		for expiry in queue.iter() {
			if expiry.at > now || meter.try_consume(T::WeightInfo::on_poll_expire()).is_err() {
				break;
			}
			due += 1;
		}
		if due == 0 {
			return;
		}

		// Take them off the list first, so ending them doesn't have to touch it again.
		let expired: alloc::vec::Vec<_> = queue.drain(..due).collect();
		Expiries::<T>::put(queue);
		for Expiry { at, id, .. } in expired {
			Self::expire_queued(at, id);
		}
	}

	pub(crate) fn expire_queued(at: BlockNumberOf<T>, id: PauseId) {
		let Some(target) = PauseTargets::<T>::get(id) else {
			frame_support::defensive!("an expiry was queued for a pause that doesn't exist");
			return;
		};
		match Pauses::<T>::get(&target) {
			Some(pause) if pause.id == id && pause.until == Some(at) =>
				Self::do_expire(&target, &pause),
			_ => {
				frame_support::defensive!("an expiry was queued for a pause that doesn't end then");
			},
		}
	}

	/// Checks that the expiry queue, the pauses, the paused calls and the deposits agree with each
	/// other.
	#[cfg(any(feature = "try-runtime", test))]
	pub(crate) fn do_try_state() -> Result<(), sp_runtime::TryRuntimeError> {
		use frame_support::traits::fungible::InspectHold;

		let queue = Expiries::<T>::get();
		ensure!(queue.windows(2).all(|w| w[0].at <= w[1].at), "the expiry queue isn't sorted");
		let calls_queued = queue.iter().filter(|expiry| expiry.single_call).count();
		ensure!(calls_queued <= Self::room_for_calls(), "single calls took slots the groups need");
		for Expiry { at, id, single_call } in queue.iter() {
			let target = PauseTargets::<T>::get(id).ok_or("queued expiry without a pause")?;
			ensure!(
				*single_call == matches!(target, Target::Call(_)),
				"queued expiry has the wrong kind"
			);
			let pause = Pauses::<T>::get(target).ok_or("queued expiry without a pause")?;
			ensure!(pause.id == *id, "queued expiry points at another pause");
			ensure!(pause.until == Some(*at), "queued expiry doesn't match the pause");
		}
		ensure!(
			PauseTargets::<T>::iter().count() == Pauses::<T>::iter().count(),
			"pause ids and pauses don't match"
		);

		// What every account should have on hold, going by the pauses.
		let mut held = alloc::collections::BTreeMap::<T::AccountId, BalanceOf<T>>::new();
		for (call, holders) in PausedCalls::<T>::iter() {
			let expected: usize = Pauses::<T>::iter_values()
				.map(|p| p.calls.iter().filter(|c| **c == call).count())
				.sum();
			ensure!(holders as usize == expected, "a call's pause count is off");
			ensure!(T::TransactionPause::is_paused(call), "a held call isn't paused");
		}

		for (target, pause) in Pauses::<T>::iter() {
			ensure!(
				PauseTargets::<T>::get(pause.id) == Some(target),
				"a pause's id points somewhere else"
			);
			ensure!(!pause.calls.is_empty(), "a pause holds no calls");
			for call in pause.calls.iter() {
				ensure!(PausedCalls::<T>::contains_key(call), "a pause's call isn't counted");
			}
			if pause.until.is_some() {
				let queued = queue.iter().filter(|expiry| expiry.id == pause.id).count();
				ensure!(queued == 1, "a pause isn't queued exactly once");
			}
			ensure!(pause.forced == pause.deposits.is_empty(), "a pause has the wrong deposits");
			for Deposit { who, amount } in pause.deposits {
				let total = held.entry(who).or_default();
				*total = total.saturating_add(amount);
			}
		}
		let reason = HoldReason::Pause.into();
		for (who, amount) in held {
			ensure!(
				T::Currency::balance_on_hold(&reason, &who) == amount,
				"held deposits don't match the pauses"
			);
		}
		Ok(())
	}
}
