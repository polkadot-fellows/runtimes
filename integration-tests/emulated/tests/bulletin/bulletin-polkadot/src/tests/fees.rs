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

use crate::*;
use emulated_integration_tests_common::impls::bx;
use integration_tests_helpers::{frame_support::traits::fungible::Inspect as _, pallet_balances};

/// XCM delivery fees reach the accumulation account, which forwards them to the DAP, rather than
/// the `py/trsry` account this chain cannot spend from.
#[test]
fn xcm_delivery_fees_reach_the_accumulation_account() {
	type BulletinRuntime = <BulletinPolkadot as Chain>::Runtime;
	type BulletinBalances = pallet_balances::Pallet<BulletinRuntime>;

	let accumulation_account = BulletinPolkadot::execute_with(
		integration_tests_helpers::pallet_accumulate_and_forward::Pallet::<
			BulletinRuntime,
		>::accumulation_account,
	);
	let amount = BULLETIN_POLKADOT_ED * 1000;

	let accumulated_before =
		BulletinPolkadot::execute_with(|| BulletinBalances::balance(&accumulation_account));

	// A signed local origin is not in `WaivedLocations`, so it pays the delivery fee.
	BulletinPolkadot::execute_with(|| {
		let dest = BulletinPolkadot::sibling_location_of(AssetHubPolkadot::para_id());
		let beneficiary: Location =
			AccountId32Junction { network: None, id: AssetHubPolkadotReceiver::get().into() }
				.into();
		let assets: Assets = (Parent, amount).into();
		assert_ok!(pallet_xcm::Pallet::<BulletinRuntime>::limited_teleport_assets(
			<BulletinPolkadot as Chain>::RuntimeOrigin::signed(BulletinPolkadotSender::get()),
			bx!(dest.into()),
			bx!(beneficiary.into()),
			bx!(assets.into()),
			0,
			WeightLimit::Unlimited,
		));
	});

	let accumulated_after =
		BulletinPolkadot::execute_with(|| BulletinBalances::balance(&accumulation_account));

	assert!(
		accumulated_after > accumulated_before,
		"the delivery fee reached the accumulation account"
	);
}
