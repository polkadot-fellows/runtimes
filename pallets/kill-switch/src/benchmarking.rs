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
	let deposit = T::DepositFor::convert(T::BenchmarkHelper::target()).expect("target is priced");
	let amount = T::Currency::minimum_balance()
		.saturating_add(deposit.saturating_mul((T::MaxDepositsPerPause::get() + 1).into()));
	T::Currency::set_balance(&who, amount);
	who
}

/// Pauses the benchmark target with `deposits` deposits behind it.
fn pause_with_deposits<T: Config>(deposits: u32) -> T::Target {
	let target = T::BenchmarkHelper::target();
	let who = funded_account::<T>("pauser", 0);
	Pallet::<T>::pause(RawOrigin::Signed(who).into(), target.clone()).expect("can pause");
	for i in 1..deposits {
		let who = funded_account::<T>("extender", i);
		Pallet::<T>::extend(RawOrigin::Signed(who).into(), target.clone()).expect("can extend");
	}
	target
}

fn fellowship_origin<T: Config>() -> Result<T::RuntimeOrigin, BenchmarkError> {
	T::FellowshipOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn pause() {
		set_relay_block::<T>(10);
		let target = T::BenchmarkHelper::target();
		let who = funded_account::<T>("pauser", 0);
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

		#[extrinsic_call]
		_(RawOrigin::Signed(who), target.clone());

		let info = Pauses::<T>::get(&target).unwrap();
		assert_eq!(info.deposits.len() as u32, T::MaxDepositsPerPause::get());
	}

	#[benchmark]
	fn unpause(d: Linear<1, { T::MaxDepositsPerPause::get() }>) -> Result<(), BenchmarkError> {
		set_relay_block::<T>(10);
		let target = pause_with_deposits::<T>(d);
		let origin = fellowship_origin::<T>()?;

		// Slashing is the more expensive path, it also starts the cooldown.
		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, target.clone(), true);

		assert!(!Pauses::<T>::contains_key(&target));
		assert_eq!(Deposits::<T>::iter().count(), 0);
		Ok(())
	}

	#[benchmark]
	fn force_extend() -> Result<(), BenchmarkError> {
		set_relay_block::<T>(10);
		let target = pause_with_deposits::<T>(1);
		let until = Pauses::<T>::get(&target).unwrap().until.unwrap();
		let new_until = until.saturating_add(One::one());
		let origin = fellowship_origin::<T>()?;

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, target.clone(), new_until);

		assert_eq!(Pauses::<T>::get(&target).unwrap().until, Some(new_until));
		Ok(())
	}

	#[benchmark]
	fn force_pause() -> Result<(), BenchmarkError> {
		set_relay_block::<T>(10);
		let target = T::BenchmarkHelper::target();
		let origin =
			T::ForceOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, target.clone(), Some(BlockNumberOf::<T>::from(20u32)));

		assert!(Pauses::<T>::contains_key(&target));
		Ok(())
	}

	#[benchmark]
	fn expire() {
		set_relay_block::<T>(10);
		// Every deposit gets its release block set, so a full pause is the worst case.
		let target = pause_with_deposits::<T>(T::MaxDepositsPerPause::get());
		let until = Pauses::<T>::get(&target).unwrap().until.unwrap();
		T::BlockNumberProvider::set_block_number(until);
		let caller = funded_account::<T>("caller", 0);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller), target.clone());

		assert!(!Pauses::<T>::contains_key(&target));
	}

	#[benchmark]
	fn release_deposit() {
		set_relay_block::<T>(10);
		let target = pause_with_deposits::<T>(1);
		Pallet::<T>::do_expire(&target, &Pauses::<T>::get(&target).unwrap(), 10u32.into());
		let now = BlockNumberOf::<T>::from(10u32).saturating_add(T::ReleaseDelay::get());
		T::BlockNumberProvider::set_block_number(now);
		let caller = funded_account::<T>("caller", 0);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller), 0);

		assert!(!Deposits::<T>::contains_key(0));
	}

	#[benchmark]
	fn slash_deposit() -> Result<(), BenchmarkError> {
		set_relay_block::<T>(10);
		let target = pause_with_deposits::<T>(1);
		Pallet::<T>::do_expire(&target, &Pauses::<T>::get(&target).unwrap(), 10u32.into());
		let origin = fellowship_origin::<T>()?;

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, 0);

		assert!(!Deposits::<T>::contains_key(0));
		Ok(())
	}

	/// Something is queued but nothing is due yet.
	#[benchmark]
	fn on_poll_base() {
		set_relay_block::<T>(10);
		let target = pause_with_deposits::<T>(1);

		#[block]
		{
			Pallet::<T>::process_expiries(&mut WeightMeter::new());
		}

		assert!(Pauses::<T>::contains_key(&target));
	}

	/// One empty block in the queue.
	#[benchmark]
	fn on_poll_step() {
		set_relay_block::<T>(10);
		let target = pause_with_deposits::<T>(1);
		ExpiryCursor::<T>::put(BlockNumberOf::<T>::from(10u32));

		#[block]
		{
			Pallet::<T>::process_expiries(&mut WeightMeter::with_limit(
				T::WeightInfo::on_poll_base().saturating_add(T::WeightInfo::on_poll_step()),
			));
		}

		assert!(Pauses::<T>::contains_key(&target));
		assert_eq!(ExpiryCursor::<T>::get(), Some(BlockNumberOf::<T>::from(11u32)));
	}

	/// Ending one pause that is due, on top of the step that found it.
	#[benchmark]
	fn on_poll_expire() {
		set_relay_block::<T>(10);
		let target = pause_with_deposits::<T>(T::MaxDepositsPerPause::get());
		let until = Pauses::<T>::get(&target).unwrap().until.unwrap();
		T::BlockNumberProvider::set_block_number(until);

		#[block]
		{
			Pallet::<T>::expire_queued(until, target.clone(), until);
		}

		assert!(!Pauses::<T>::contains_key(&target));
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
