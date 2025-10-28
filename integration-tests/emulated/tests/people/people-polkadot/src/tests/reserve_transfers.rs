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
use frame_support::traits::fungibles;
use people_polkadot_runtime::xcm_config::XcmConfig;

#[test]
fn can_receive_hollar_from_hydration() {
	const HYDRATION_PARA_ID: u32 = 2034;
	const HOLLAR_ASSET_ID: u128 = 222;

	let hydration_location = Location::new(1, [Parachain(HYDRATION_PARA_ID)]);
	let hydration_sovereign_account =
		PeoplePolkadot::sovereign_account_id_of(hydration_location.clone());
	let hollar_id = Location::new(1, [Parachain(HYDRATION_PARA_ID), GeneralIndex(HOLLAR_ASSET_ID)]);

	PeoplePolkadot::fund_accounts(vec![(
		hydration_sovereign_account.clone(),
		ASSET_HUB_POLKADOT_ED * 10,
	)]);

	PeoplePolkadot::execute_with(|| {
		type PeopleAssets = <PeoplePolkadot as PeoplePolkadotPallet>::Assets;

		// HOLLAR is not registered at first.
		assert!(!<PeopleAssets as fungibles::Inspect<_>>::asset_exists(hollar_id.clone()));

		// We force create it via root.
		assert_ok!(PeopleAssets::force_create(
			<PeoplePolkadot as Chain>::RuntimeOrigin::root(),
			hollar_id.clone(),
			hydration_sovereign_account.into(),
			true,
			1,
		));

		// Now it's registered.
		assert!(<PeopleAssets as fungibles::Inspect<_>>::asset_exists(hollar_id.clone()));

		// The receiver starts with no HOLLAR.
		let receiver = PeoplePolkadotReceiver::get();
		let balance_before =
			<PeopleAssets as fungibles::Inspect<_>>::balance(hollar_id.clone(), &receiver);
		assert_eq!(balance_before, 0);

		// And we can transfer it from Hydration.
		let transfer_amount = 10_000_000_000_000_000_000u128;
		let transfer_xcm = Xcm::builder_unsafe()
			.reserve_asset_deposited((hollar_id.clone(), transfer_amount))
			.buy_execution((hollar_id.clone(), transfer_amount), Unlimited)
			.deposit_asset(AllCounted(1), receiver.clone())
			.build();
		let mut hash = transfer_xcm.using_encoded(sp_io::hashing::blake2_256);
		assert_ok!(xcm_executor::XcmExecutor::<XcmConfig>::prepare_and_execute(
			hydration_location,
			transfer_xcm,
			&mut hash,
			Weight::MAX,
			Weight::zero(),
		)
		.ensure_complete());

		let balance_after = <PeopleAssets as fungibles::Inspect<_>>::balance(hollar_id, &receiver);
		// TODO: Need to benchmark.
		let fees = 1;
		assert_eq!(balance_after, transfer_amount - fees);
	});
}
