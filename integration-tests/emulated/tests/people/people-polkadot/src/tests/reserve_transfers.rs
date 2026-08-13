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
use emulated_integration_tests_common::macros::{AssetTransferFilter, XcmPaymentApiV2};
use frame_support::traits::fungibles;
use people_polkadot_runtime::xcm_config::{RelayTreasuryPalletAccount, XcmConfig};

#[test]
fn can_receive_hollar_from_hydration() {
	let hydration_location = HydrationLocation::get();
	let hydration_sovereign_account =
		PeoplePolkadot::sovereign_account_id_of(hydration_location.clone());
	let hollar_id = HollarLocation::get();

	PeoplePolkadot::fund_accounts(vec![(
		hydration_sovereign_account.clone(),
		ASSET_HUB_POLKADOT_ED * 10,
	)]);

	// We need to first register HOLLAR.
	register_hollar();

	PeoplePolkadot::execute_with(|| {
		type Runtime = <PeoplePolkadot as Chain>::Runtime;
		type PeopleAssets = <PeoplePolkadot as PeoplePolkadotPallet>::Assets;

		// The receiver starts with no HOLLAR.
		let receiver = PeoplePolkadotReceiver::get();
		let balance_before =
			<PeopleAssets as fungibles::Inspect<_>>::balance(hollar_id.clone(), &receiver);
		assert_eq!(balance_before, 0);

		// And we can transfer it from Hydration.
		let transfer_amount = 10 * HOLLAR_UNITS;
		let transfer_xcm = Xcm::builder_unsafe()
			.reserve_asset_deposited((hollar_id.clone(), transfer_amount))
			.buy_execution((hollar_id.clone(), transfer_amount), Unlimited)
			.deposit_asset(AllCounted(1), receiver.clone())
			.build();
		let mut hash = transfer_xcm.using_encoded(sp_io::hashing::blake2_256);
		assert_ok!(xcm_executor::XcmExecutor::<XcmConfig>::prepare_and_execute(
			hydration_location,
			transfer_xcm.clone(),
			&mut hash,
			Weight::MAX,
			Weight::zero(),
		)
		.ensure_complete());

		let balance_after = <PeopleAssets as fungibles::Inspect<_>>::balance(hollar_id, &receiver);

		// Calculate actual fees.
		let transfer_xcm_weight =
			Runtime::query_xcm_weight(VersionedXcm::from(transfer_xcm.into())).unwrap();
		let fees = Runtime::query_weight_to_asset_fee(
			transfer_xcm_weight,
			VersionedAssetId::from(HollarId::get()),
		)
		.unwrap();
		assert_eq!(balance_after, transfer_amount - fees);
	});
}

#[test]
fn can_send_hollar_back_to_hydration() {
	let hydration_location = HydrationLocation::get();
	let hydration_sovereign_account =
		PeoplePolkadot::sovereign_account_id_of(hydration_location.clone());
	let hollar_id = HollarLocation::get();

	PeoplePolkadot::fund_accounts(vec![(
		hydration_sovereign_account.clone(),
		ASSET_HUB_POLKADOT_ED * 10,
	)]);

	// First we register HOLLAR.
	register_hollar();

	PeoplePolkadot::execute_with(|| {
		type RuntimeOrigin = <PeoplePolkadot as Chain>::RuntimeOrigin;
		type PeopleAssets = <PeoplePolkadot as PeoplePolkadotPallet>::Assets;
		type PolkadotXcm = <PeoplePolkadot as PeoplePolkadotPallet>::PolkadotXcm;
		let sender = PeoplePolkadotSender::get();
		let receiver = PeoplePolkadotReceiver::get();
		// Delivery fees are collected here, in whichever asset they were paid.
		let fee_receiver = RelayTreasuryPalletAccount::get();
		// We need to open a channel between People and Hydration.
		<PeoplePolkadot as Para>::ParachainSystem::open_outbound_hrmp_channel_for_benchmarks_or_tests(HYDRATION_PARA_ID.into());
		let transfer_amount = 10 * HOLLAR_UNITS;
		let fees_amount = HOLLAR_UNITS;
		// We need to mint some HOLLAR into our sender: HOLLAR pays for everything here, both
		// execution and delivery, so no DOT is needed.
		assert_ok!(<PeopleAssets as fungibles::Mutate<_>>::mint_into(
			HollarLocation::get(),
			&sender,
			transfer_amount + fees_amount,
		));
		assert_eq!(
			<PeopleAssets as fungibles::Inspect<_>>::balance(hollar_id.clone(), &fee_receiver),
			0
		);
		let transfer_xcm = Xcm::builder()
			.withdraw_asset((hollar_id.clone(), transfer_amount + fees_amount))
			.pay_fees((hollar_id.clone(), fees_amount))
			.initiate_transfer(
				hydration_location,
				Some(AssetTransferFilter::ReserveWithdraw(Definite(
					(hollar_id.clone(), transfer_amount.saturating_div(10)).into(),
				))),
				false,
				vec![AssetTransferFilter::ReserveWithdraw(
					AllOfCounted { id: hollar_id.clone().into(), fun: WildFungible, count: 1 }
						.into(),
				)],
				Xcm::<()>::builder_unsafe()
					.refund_surplus()
					.deposit_asset(AllCounted(1), receiver)
					.build(),
			)
			.refund_surplus()
			.deposit_asset(AllCounted(2), sender.clone())
			.build();
		assert_ok!(PolkadotXcm::execute(
			RuntimeOrigin::signed(sender),
			Box::new(VersionedXcm::from(transfer_xcm)),
			Weight::MAX,
		));

		// The delivery fees of the message sent to Hydration were paid in HOLLAR.
		assert!(
			<PeopleAssets as fungibles::Inspect<_>>::balance(hollar_id, &fee_receiver) > 0,
			"delivery fees should have been collected in HOLLAR",
		);
	});
}

fn register_hollar() {
	let hydration_location = HydrationLocation::get();
	let hydration_sovereign_account =
		PeoplePolkadot::sovereign_account_id_of(hydration_location.clone());
	let hollar_id = HollarLocation::get();

	PeoplePolkadot::execute_with(|| {
		type RuntimeOrigin = <PeoplePolkadot as Chain>::RuntimeOrigin;
		type PeopleAssets = <PeoplePolkadot as PeoplePolkadotPallet>::Assets;
		type AssetRate = <PeoplePolkadot as PeoplePolkadotPallet>::AssetRate;

		// HOLLAR is not registered at first.
		assert!(!<PeopleAssets as fungibles::Inspect<_>>::asset_exists(hollar_id.clone()));

		// We force create it via root.
		assert_ok!(PeopleAssets::force_create(
			RuntimeOrigin::root(),
			hollar_id.clone(),
			hydration_sovereign_account.into(),
			true,
			1,
		));

		// Now it's registered.
		assert!(<PeopleAssets as fungibles::Inspect<_>>::asset_exists(hollar_id.clone()));

		// We need to create a rate between DOT and HOLLAR
		// to be able to pay fees in HOLLAR.
		assert_ok!(AssetRate::create(
			RuntimeOrigin::root(),
			Box::new(HollarLocation::get()),
			1u128.into(),
		));
	});
}
