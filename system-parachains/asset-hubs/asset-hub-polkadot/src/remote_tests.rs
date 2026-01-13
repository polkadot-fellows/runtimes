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

//! Tests that run against a state snapshot.

use super::*;
use frame_support::{assert_ok, traits::fungible::Inspect as FungibleInspect};
use remote_externalities::{Builder, Mode, OfflineConfig};
use sp_runtime::AccountId32;
use std::{env::var, str::FromStr};

/// Ensure that the Stellaswap account is correctly migrated.
///
/// This test can be removed after the relevant code was deployed on-chain.
/// See: https://github.com/polkadot-fellows/runtimes/pull/1036
#[tokio::test]
async fn moonbeam_stellaswap_translation() {
	sp_tracing::try_init_simple();
	let Some(state_snapshot) = var("SNAP").map(|s| s.into()).ok() else {
		return;
	};

	let mut ext = Builder::<Block>::default()
		.mode(Mode::Offline(OfflineConfig { state_snapshot }))
		.build()
		.await
		.unwrap();
	ext.execute_with(|| {
		frame_system::Pallet::<Runtime>::reset_events();

		let child_5_1 =
			AccountId32::from_str("1zAWXSCmRTR9ZkRXZXeHZftj1J6rnDe8BLXV8UJ2S2exCvL").unwrap();
		let sibl_5_1 =
			AccountId32::from_str("13GWAfgWAKLGm8AsKLn5pDbDyMHfShFgtMFEqM4TRNhXbSea").unwrap();
		let derivation_path_5_1 = vec![5, 1];

		test_translate(child_5_1, sibl_5_1, derivation_path_5_1);
		println!("Second account:");

		let child_5_2 =
			AccountId32::from_str("14KQD8dRoT3q2fCbCC49bFjU1diFu1d516tYuGmSUMmEoGNa").unwrap();
		let sibl_5_2 =
			AccountId32::from_str("123oqim7B24XzwB1hC4Fh7LGwbTas3QmxL6v6sVd95eTD5ee").unwrap();
		let derivation_path = vec![5, 2];

		test_translate(child_5_2, sibl_5_2, derivation_path);
	});
}

/// Run the actual translation and do total balance checks.
fn test_translate(child_5_2: AccountId32, sibl_5_2: AccountId32, derivation_path: Vec<u16>) {
	let child_before = summary(&child_5_2);
	assert_eq!(summary(&sibl_5_2), 0, "Sibl acc should be empty");

	assert_ok!(
		pallet_ah_ops::Pallet::<Runtime>::do_translate_para_sovereign_child_to_sibling_derived(
			2004,
			derivation_path.clone(),
			child_5_2.clone(),
			sibl_5_2.clone(),
		)
	);

	for event in frame_system::Pallet::<Runtime>::events() {
		println!("{event:?}");
	}

	let child_remaining = summary(&child_5_2);
	let ed = <crate::Balances as FungibleInspect<_>>::minimum_balance();
	// It can still have ED in case that we did not migrate all assets.
	assert!(child_remaining <= ed, "Child remaining should have at most ED");
	assert_eq!(
		summary(&sibl_5_2),
		child_before - child_remaining,
		"Sibl should have child balance"
	);
}

/// Account summary and return the total balance.
fn summary(acc: &AccountId32) -> u128 {
	let info = frame_system::Account::<Runtime>::get(acc);
	let ledger = pallet_staking_async::Ledger::<Runtime>::get(acc);
	println!("{acc}\n\tInfo: {info:?}\n\tLedger: {ledger:?}");

	info.data.free + info.data.reserved
}
