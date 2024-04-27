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
use asset_hub_polkadot_runtime::xcm_config::LocationToAccountId;
use collectives_polkadot_runtime::fellowship::FellowshipSalaryPaymaster;
use frame_support::{
	assert_ok,
	traits::{
		fungibles::{Create, Mutate},
		tokens::Pay,
	},
};
use xcm_executor::traits::ConvertLocation;

const FELLOWSHIP_SALARY_PALLET_ID: u8 =
	collectives_polkadot_runtime_constants::FELLOWSHIP_SALARY_PALLET_INDEX;

#[test]
fn pay_salary() {
	let asset_id: u32 = 1984;
	let fellowship_salary = (
		Parent,
		Parachain(CollectivesPolkadot::para_id().into()),
		PalletInstance(FELLOWSHIP_SALARY_PALLET_ID),
	);
	let pay_from = LocationToAccountId::convert_location(&fellowship_salary.into()).unwrap();
	let pay_to = Polkadot::account_id_of(ALICE);
	let pay_amount = 9000;

	AssetHubPolkadot::execute_with(|| {
		type AssetHubAssets = <AssetHubPolkadot as AssetHubPolkadotPallet>::Assets;

		assert_ok!(<AssetHubAssets as Create<_>>::create(
			asset_id,
			pay_to.clone(),
			true,
			pay_amount / 2
		));
		assert_ok!(<AssetHubAssets as Mutate<_>>::mint_into(asset_id, &pay_from, pay_amount * 2));
	});

	CollectivesPolkadot::execute_with(|| {
		type RuntimeEvent = <CollectivesPolkadot as Chain>::RuntimeEvent;

		assert_ok!(FellowshipSalaryPaymaster::pay(&pay_to, (), pay_amount));
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
			RuntimeEvent::Assets(pallet_assets::Event::Transferred { .. }) => {},
			RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true ,.. }) => {},
				]
		);
	});
}
