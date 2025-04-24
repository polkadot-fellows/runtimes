// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::*;
use frame_benchmarking::v2::*;
use frame_support::traits::Currency;
use frame_system::Account as SystemAccount;

#[benchmarks]
pub mod benchmarks {
	use super::*;

	#[benchmark]
	fn withdraw_account() {
		let create_liquid_account = |n: u8| {
			let who: AccountId32 = [n; 32].into();
			let ed = <pallet_balances::Pallet<T> as Currency<_>>::minimum_balance();
			let _ = <pallet_balances::Pallet<T> as Currency<_>>::deposit_creating(&who, ed);
		};

		let n = 50;
		let messages = (0..n).map(|i| create_liquid_account(i)).collect::<Vec<_>>();
		let last_key: AccountId32 = [n / 2; 32].into();

		RcMigratedBalance::<T>::mutate(|tracker| {
			tracker.kept = <<T as Config>::Currency as Currency<_>>::total_issuance();
		});

		#[block]
		{
			let (who, account_info) = SystemAccount::<T>::iter_from_key(last_key).next().unwrap();
			let mut ah_weight = WeightMeter::new();
			let batch_len = 0;
			let res = AccountsMigrator::<T>::withdraw_account(
				who,
				account_info,
				&mut ah_weight,
				batch_len,
			);
			assert!(res.unwrap().is_some());
		}
	}

	#[cfg(feature = "std")]
	pub fn test_withdraw_account<T: Config>() {
		_withdraw_account::<T>(true /* enable checks */)
	}
}
