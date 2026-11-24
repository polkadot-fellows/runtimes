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

use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use sp_runtime::traits::One;

fn set_relay_block<T: Config>(n: u32) {
	T::BlockNumberProvider::set_block_number(n.into());
}

/// A funded account that can pay for every deposit of a full pause.
fn funded_account<T: Config>(name: &'static str, index: u32) -> T::AccountId {
	let who: T::AccountId = account(name, index, 0);
	let deposit = Pallet::<T>::group_deposit(&T::BenchmarkHelper::group())
		.expect("group is priced")
		.max(T::CallDeposit::get());
	let amount = T::Currency::minimum_balance()
		.saturating_add(deposit.saturating_mul((T::MaxDepositsPerPause::get() + 1).into()));
	T::Currency::set_balance(&who, amount);
	who
}

/// The group with the most calls.
fn target<T: Config>() -> TargetOf<T> {
	T::BenchmarkHelper::enable();
	Target::Group(T::BenchmarkHelper::group())
}

/// Pauses the benchmark target with `deposits` deposits behind it.
fn pause_with_deposits<T: Config>(deposits: u32) -> TargetOf<T> {
	let target = target::<T>();
	let who = funded_account::<T>("pauser", 0);
	Pallet::<T>::pause(RawOrigin::Signed(who).into(), target.clone()).expect("can pause");
	for i in 1..deposits {
		let who = funded_account::<T>("extender", i);
		Pallet::<T>::extend(RawOrigin::Signed(who).into(), target.clone()).expect("can extend");
	}
	target
}

/// Fills the expiry queue with entries that end long after anything else, leaving `room` free.
///
/// They use ids from the top down so they never match a real pause. They never come due, they are
/// only there so the queue has its full size.
fn fill_queue<T: Config>(room: u32) {
	let later = BlockNumberOf::<T>::from(u32::MAX);
	let size = T::MaxActivePauses::get().saturating_sub(room) as usize;
	Expiries::<T>::mutate(|queue| {
		let mut id = PauseId::MAX;
		while queue.len() < size &&
			queue.try_push(Expiry { at: later, id, single_call: false }).is_ok()
		{
			id -= 1;
		}
	});
}

/// An origin that can do `action`.
fn admin_origin<T: Config>(
	action: AdminAction<TargetOf<T>>,
) -> Result<T::RuntimeOrigin, BenchmarkError> {
	T::AdminOrigin::try_successful_origin(&action).map_err(|_| BenchmarkError::Weightless)
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn pause() {
		set_relay_block::<T>(10);
		let target = target::<T>();
		let who = funded_account::<T>("pauser", 0);
		fill_queue::<T>(1);
		// Worst case: an old cooldown has to be cleaned up.
		Cooldowns::<T>::insert(&target, BlockNumberOf::<T>::from(10u32));

		#[extrinsic_call]
		_(RawOrigin::Signed(who), target.clone());

		assert!(Pauses::<T>::contains_key(&target));
		assert!(!Cooldowns::<T>::contains_key(&target));
	}

	#[benchmark]
	fn extend() {
		set_relay_block::<T>(10);
		let target = pause_with_deposits::<T>(T::MaxDepositsPerPause::get() - 1);
		let who = funded_account::<T>("extender", T::MaxDepositsPerPause::get());
		fill_queue::<T>(0);

		#[extrinsic_call]
		_(RawOrigin::Signed(who), target.clone());

		let info = Pauses::<T>::get(&target).unwrap();
		assert_eq!(info.deposits.len() as u32, T::MaxDepositsPerPause::get());
	}

	#[benchmark]
	fn unpause(d: Linear<1, { T::MaxDepositsPerPause::get() }>) -> Result<(), BenchmarkError> {
		set_relay_block::<T>(10);
		let target = pause_with_deposits::<T>(d);
		fill_queue::<T>(0);
		let origin = admin_origin::<T>(AdminAction::Unpause(target.clone()))?;

		// Slashing is the more expensive path, it also starts the cooldown.
		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, target.clone(), true);

		assert!(!Pauses::<T>::contains_key(&target));
		Ok(())
	}

	#[benchmark]
	fn force_extend() -> Result<(), BenchmarkError> {
		set_relay_block::<T>(10);
		let target = pause_with_deposits::<T>(1);
		let until = Pauses::<T>::get(&target).unwrap().until.unwrap();
		let new_until = until.saturating_add(One::one());
		fill_queue::<T>(0);
		let origin = admin_origin::<T>(AdminAction::Extend(target.clone()))?;

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, target.clone(), new_until);

		assert_eq!(Pauses::<T>::get(&target).unwrap().until, Some(new_until));
		Ok(())
	}

	/// Taking over a pause with `d` deposits, or a fresh pause for `d = 0`.
	#[benchmark]
	fn force_pause(d: Linear<0, { T::MaxDepositsPerPause::get() }>) -> Result<(), BenchmarkError> {
		set_relay_block::<T>(10);
		let target = if d > 0 { pause_with_deposits::<T>(d) } else { target::<T>() };
		fill_queue::<T>(if d > 0 { 0 } else { 1 });
		let origin = admin_origin::<T>(AdminAction::ForcePause(target.clone()))?;

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, target.clone(), Some(BlockNumberOf::<T>::from(20u32)));

		assert!(Pauses::<T>::get(&target).unwrap().forced);
		Ok(())
	}

	/// A full queue where nothing is due yet.
	#[benchmark]
	fn on_poll_base() {
		set_relay_block::<T>(10);
		let target = pause_with_deposits::<T>(1);
		fill_queue::<T>(0);

		#[block]
		{
			Pallet::<T>::process_expiries(&mut WeightMeter::new());
		}

		assert!(Pauses::<T>::contains_key(&target));
	}

	/// Ending one full pause out of a full queue. Also covers what `on_poll_base` measures.
	#[benchmark]
	fn on_poll_expire() {
		set_relay_block::<T>(10);
		let target = pause_with_deposits::<T>(T::MaxDepositsPerPause::get());
		fill_queue::<T>(0);
		let info = Pauses::<T>::get(&target).unwrap();
		let (until, pause_id) = (info.until.unwrap(), info.id);
		T::BlockNumberProvider::set_block_number(until);

		#[block]
		{
			Pallet::<T>::process_expiries(&mut WeightMeter::new());
		}

		assert!(!Pauses::<T>::contains_key(&target));
		assert!(EndedPauses::<T>::contains_key(pause_id));
	}

	/// Settling an ended pause with `d` deposits. Slashing is the more expensive path, it also
	/// starts the cooldown.
	#[benchmark]
	fn settle(d: Linear<1, { T::MaxDepositsPerPause::get() }>) -> Result<(), BenchmarkError> {
		set_relay_block::<T>(10);
		let target = pause_with_deposits::<T>(d);
		let info = Pauses::<T>::get(&target).unwrap();
		T::BlockNumberProvider::set_block_number(info.until.unwrap());
		Pallet::<T>::process_expiries(&mut WeightMeter::new());
		assert!(EndedPauses::<T>::contains_key(info.id));
		let origin = admin_origin::<T>(AdminAction::Settle(target.clone()))?;

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, info.id, true);

		assert!(!EndedPauses::<T>::contains_key(info.id));
		Ok(())
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
