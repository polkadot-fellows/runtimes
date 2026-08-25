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
use emulated_integration_tests_common::{
	create_pool_with_native_location_on,
	macros::{AssetTransferFilter, XcmPaymentApiV2},
};
use frame_support::traits::{fungible, fungibles};
use people_polkadot_runtime::xcm_config::{RelayTreasuryPalletAccount, XcmConfig};
use polkadot_runtime_constants::{
	currency::UNITS as DOT_UNITS,
	system_parachain::{ASSET_HUB_ID, PEOPLE_ID as PEOPLE_PARA_ID},
};
use xcm_executor::traits::TransferType;

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

/// HOLLAR pays for everything on the way out — execution *and* delivery — priced at the rate
/// governance registered for it. Nothing is swapped on this path, so the delivery fee is collected
/// in HOLLAR itself.
#[test]
fn can_send_hollar_back_to_hydration_paying_fees_at_the_registered_rate() {
	let hydration_location = HydrationLocation::get();
	let hydration_sovereign_account =
		PeoplePolkadot::sovereign_account_id_of(hydration_location.clone());
	let hollar_id = HollarLocation::get();

	PeoplePolkadot::fund_accounts(vec![(
		hydration_sovereign_account.clone(),
		ASSET_HUB_POLKADOT_ED * 10,
	)]);

	// First we register HOLLAR, with a rate and no pool.
	register_hollar();

	PeoplePolkadot::execute_with(|| {
		type PeopleAssets = <PeoplePolkadot as PeoplePolkadotPallet>::Assets;
		// Delivery fees are collected here, in whichever asset they were paid.
		let fee_receiver = RelayTreasuryPalletAccount::get();

		assert_eq!(
			<PeopleAssets as fungibles::Inspect<_>>::balance(hollar_id.clone(), &fee_receiver),
			0
		);

		send_hollar_back_to_hydration();

		// The delivery fees of the message sent to Hydration were paid in HOLLAR, in kind: there is
		// no pool to swap them for DOT.
		assert!(
			<PeopleAssets as fungibles::Inspect<_>>::balance(hollar_id, &fee_receiver) > 0,
			"delivery fees should have been collected in HOLLAR",
		);
	});
}

/// The same round trip, but priced by a DOT/HOLLAR pool instead of a governance rate: no rate is
/// registered at all here. The offered HOLLAR is swapped for the DOT the executor asks for, so the
/// delivery fee is collected in DOT.
#[test]
fn can_send_hollar_back_to_hydration_paying_fees_through_a_pool() {
	let hydration_location = HydrationLocation::get();
	let hydration_sovereign_account =
		PeoplePolkadot::sovereign_account_id_of(hydration_location.clone());
	let hollar_id = HollarLocation::get();

	PeoplePolkadot::fund_accounts(vec![
		(hydration_sovereign_account.clone(), ASSET_HUB_POLKADOT_ED * 10),
		// The liquidity provider pays the pool setup fee and puts up the DOT side of the pool.
		(PeoplePolkadotReceiver::get(), 10_000 * DOT_UNITS),
	]);

	// HOLLAR is registered with no rate whatsoever, only a pool.
	force_create_hollar();
	open_dot_hollar_pool();

	PeoplePolkadot::execute_with(|| {
		type PeopleAssets = <PeoplePolkadot as PeoplePolkadotPallet>::Assets;
		type PeopleBalances = <PeoplePolkadot as PeoplePolkadotPallet>::Balances;
		let fee_receiver = RelayTreasuryPalletAccount::get();

		// Nothing here relies on a governance-registered rate: `force_create_hollar` deliberately
		// registers none.
		let dot_before = <PeopleBalances as fungible::Inspect<_>>::balance(&fee_receiver);

		send_hollar_back_to_hydration();

		// The offered HOLLAR was swapped through the pool, so the fee receiver was paid in DOT.
		assert!(
			<PeopleBalances as fungible::Inspect<_>>::balance(&fee_receiver) > dot_before,
			"delivery fees should have been collected in DOT out of the pool",
		);
		assert_eq!(
			<PeopleAssets as fungibles::Inspect<_>>::balance(hollar_id, &fee_receiver),
			0,
			"nothing should have been collected in HOLLAR",
		);
	});
}

/// Sends HOLLAR back to Hydration, paying both the execution and the delivery fee of the outgoing
/// message in HOLLAR. How that HOLLAR is priced against DOT — pool or registered rate — is left to
/// the runtime.
fn send_hollar_back_to_hydration() {
	type RuntimeOrigin = <PeoplePolkadot as Chain>::RuntimeOrigin;
	type PeopleAssets = <PeoplePolkadot as PeoplePolkadotPallet>::Assets;
	type PolkadotXcm = <PeoplePolkadot as PeoplePolkadotPallet>::PolkadotXcm;

	let hydration_location = HydrationLocation::get();
	let hollar_id = HollarLocation::get();
	let sender = PeoplePolkadotSender::get();
	let receiver = PeoplePolkadotReceiver::get();

	// We need to open a channel between People and Hydration.
	<PeoplePolkadot as Para>::ParachainSystem::open_outbound_hrmp_channel_for_benchmarks_or_tests(
		HYDRATION_PARA_ID.into(),
	);

	let transfer_amount = 10 * HOLLAR_UNITS;
	let fees_amount = HOLLAR_UNITS;
	// We need to mint some HOLLAR into our sender: HOLLAR pays for everything here, both execution
	// and delivery, so no DOT is needed.
	assert_ok!(<PeopleAssets as fungibles::Mutate<_>>::mint_into(
		hollar_id.clone(),
		&sender,
		transfer_amount + fees_amount,
	));

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
				AllOfCounted { id: hollar_id.clone().into(), fun: WildFungible, count: 1 }.into(),
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
}

/// Registers HOLLAR as a sufficient asset, without pricing it against DOT in any way.
fn force_create_hollar() {
	let hydration_location = HydrationLocation::get();
	let hydration_sovereign_account =
		PeoplePolkadot::sovereign_account_id_of(hydration_location.clone());
	let hollar_id = HollarLocation::get();

	PeoplePolkadot::execute_with(|| {
		type RuntimeOrigin = <PeoplePolkadot as Chain>::RuntimeOrigin;
		type PeopleAssets = <PeoplePolkadot as PeoplePolkadotPallet>::Assets;

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
	});
}

/// Registers HOLLAR and prices it against DOT with a governance-set rate, the way governance
/// enabled it before pools existed.
fn register_hollar() {
	force_create_hollar();

	PeoplePolkadot::execute_with(|| {
		type RuntimeOrigin = <PeoplePolkadot as Chain>::RuntimeOrigin;
		type AssetRate = <PeoplePolkadot as PeoplePolkadotPallet>::AssetRate;

		// We need to create a rate between DOT and HOLLAR
		// to be able to pay fees in HOLLAR.
		assert_ok!(AssetRate::create(
			RuntimeOrigin::root(),
			Box::new(HollarLocation::get()),
			1u128.into(),
		));
	});
}

/// Opens a DOT/HOLLAR pool and seeds it, which is all it takes — no governance action — to make
/// HOLLAR usable for fees.
fn open_dot_hollar_pool() {
	let provider = PeoplePolkadotReceiver::get();
	let dot_liquidity = 1_000 * DOT_UNITS;
	let hollar_liquidity = 1_000 * HOLLAR_UNITS;

	PeoplePolkadot::execute_with(|| {
		type PeopleAssets = <PeoplePolkadot as PeoplePolkadotPallet>::Assets;

		// Mint more than goes into the pool: `add_liquidity` preserves the provider's accounts, so
		// it cannot hand over every last planck of either side.
		assert_ok!(<PeopleAssets as fungibles::Mutate<_>>::mint_into(
			HollarLocation::get(),
			&provider,
			2 * hollar_liquidity,
		));
	});
	create_pool_with_native_location_on!(
		PeoplePolkadot,
		Location::parent(),
		HollarLocation::get(),
		provider,
		dot_liquidity,
		hollar_liquidity
	);
}

/// The asset id used on Asset Hub for the round-trip test. Any free id will do — 1 and 2 are the
/// emulated network's reservable/teleportable assets and 1984 is USDT.
const AH_ASSET_ID: u32 = 4242;

/// The Asset Hub asset as Asset Hub itself names it.
fn ah_asset_local() -> Location {
	Location::new(0, [PalletInstance(50), GeneralIndex(AH_ASSET_ID as u128)])
}

/// The same asset as People names it.
fn ah_asset_at_people() -> Location {
	Location::new(
		1,
		[Parachain(ASSET_HUB_ID), PalletInstance(50), GeneralIndex(AH_ASSET_ID as u128)],
	)
}

/// An asset created on Asset Hub can be reserve transferred to People and back, and pays its own
/// way in both directions.
///
/// Asset Hub prices it through a pool; People prices it through a governance-registered rate. That
/// is deliberate — it exercises both fee oracles on a single asset, and the asset is neither DOT
/// nor HOLLAR, so nothing here depends on the two assets that already worked.
#[test]
fn asset_hub_asset_reserve_transfers_to_people_and_back() {
	let asset_at_people = ah_asset_at_people();
	let sender = AssetHubPolkadotSender::get();
	let receiver = PeoplePolkadotReceiver::get();
	let transfer_amount = 100 * ASSET_HUB_POLKADOT_ED * 1000;

	AssetHubPolkadot::fund_accounts(vec![(sender.clone(), 1_000 * DOT_UNITS)]);
	PeoplePolkadot::fund_accounts(vec![(receiver.clone(), 10 * DOT_UNITS)]);

	// --- Asset Hub: create the asset, fund the sender, and open a DOT pool so it can pay fees.
	AssetHubPolkadot::execute_with(|| {
		type RuntimeOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;
		type Assets = <AssetHubPolkadot as AssetHubPolkadotPallet>::Assets;

		assert_ok!(Assets::force_create(
			RuntimeOrigin::root(),
			AH_ASSET_ID.into(),
			sender.clone().into(),
			true, // is_sufficient
			1,    // min_balance
		));
		assert_ok!(Assets::mint(
			RuntimeOrigin::signed(sender.clone()),
			AH_ASSET_ID.into(),
			sender.clone().into(),
			100 * transfer_amount,
		));
	});
	// A DOT pool, so Asset Hub can price the asset when it comes home.
	create_pool_with_native_location_on!(
		AssetHubPolkadot,
		Location::parent(),
		ah_asset_local(),
		sender.clone(),
		100 * DOT_UNITS,
		10 * transfer_amount
	);

	// --- People: register the asset and give it a rate, so it can pay for its own arrival.
	PeoplePolkadot::execute_with(|| {
		type RuntimeOrigin = <PeoplePolkadot as Chain>::RuntimeOrigin;
		type PeopleAssets = <PeoplePolkadot as PeoplePolkadotPallet>::Assets;
		type AssetRate = <PeoplePolkadot as PeoplePolkadotPallet>::AssetRate;

		assert_ok!(PeopleAssets::force_create(
			RuntimeOrigin::root(),
			asset_at_people.clone(),
			receiver.clone().into(),
			true, // is_sufficient
			1,    // min_balance
		));
		assert_ok!(AssetRate::create(
			RuntimeOrigin::root(),
			Box::new(asset_at_people.clone()),
			1u128.into(),
		));
	});

	// --- Asset Hub -> People, Asset Hub acting as the reserve.
	AssetHubPolkadot::execute_with(|| {
		type RuntimeOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;
		type PolkadotXcm = <AssetHubPolkadot as AssetHubPolkadotPallet>::PolkadotXcm;

		<AssetHubPolkadot as Para>::ParachainSystem::open_outbound_hrmp_channel_for_benchmarks_or_tests(
			PEOPLE_PARA_ID.into(),
		);

		let assets: Assets = (ah_asset_local(), transfer_amount).into();
		assert_ok!(PolkadotXcm::transfer_assets_using_type_and_then(
			RuntimeOrigin::signed(sender.clone()),
			bx!(AssetHubPolkadot::sibling_location_of(PeoplePolkadot::para_id()).into()),
			bx!(assets.into()),
			bx!(TransferType::LocalReserve),
			bx!(AssetId(ah_asset_local()).into()),
			bx!(TransferType::LocalReserve),
			bx!(VersionedXcm::from(
				Xcm::<()>::builder_unsafe()
					.deposit_asset(AllCounted(1), receiver.clone())
					.build()
			)),
			Unlimited,
		));
	});

	// The asset arrived, minus whatever execution cost on People.
	let received = PeoplePolkadot::execute_with(|| {
		type PeopleAssets = <PeoplePolkadot as PeoplePolkadotPallet>::Assets;
		let balance =
			<PeopleAssets as fungibles::Inspect<_>>::balance(asset_at_people.clone(), &receiver);
		assert!(balance > 0, "the Asset Hub asset should have arrived on People");
		assert!(balance < transfer_amount, "fees should have been paid out of it");
		balance
	});

	// --- People -> Asset Hub, sending it home to its reserve.
	//
	// `transfer_assets_using_type_and_then` is not available here: People sets
	// `XcmReserveTransferFilter = Nothing` because it is not meant to be a reserve, and that filter
	// also gates the `DestinationReserve` path. Sending an asset home is done by executing a
	// `ReserveWithdraw` locally instead, the same way HOLLAR goes back to Hydration above.
	let local_fee = received / 10;
	let remote_fee = received / 10;
	let sent_back = received - local_fee;
	PeoplePolkadot::execute_with(|| {
		type RuntimeOrigin = <PeoplePolkadot as Chain>::RuntimeOrigin;
		type PolkadotXcm = <PeoplePolkadot as PeoplePolkadotPallet>::PolkadotXcm;

		<PeoplePolkadot as Para>::ParachainSystem::open_outbound_hrmp_channel_for_benchmarks_or_tests(
			ASSET_HUB_ID.into(),
		);

		// Everything is denominated in the Asset Hub asset: People prices it with the registered
		// rate, Asset Hub prices it with the pool.
		let transfer_xcm = Xcm::builder()
			.withdraw_asset((asset_at_people.clone(), received))
			.pay_fees((asset_at_people.clone(), local_fee))
			.initiate_transfer(
				PeoplePolkadot::sibling_location_of(AssetHubPolkadot::para_id()),
				Some(AssetTransferFilter::ReserveWithdraw(Definite(
					(asset_at_people.clone(), remote_fee).into(),
				))),
				false,
				vec![AssetTransferFilter::ReserveWithdraw(
					AllOfCounted {
						id: asset_at_people.clone().into(),
						fun: WildFungible,
						count: 1,
					}
					.into(),
				)],
				Xcm::<()>::builder_unsafe()
					.refund_surplus()
					.deposit_asset(AllCounted(1), receiver.clone())
					.build(),
			)
			.refund_surplus()
			.deposit_asset(AllCounted(2), receiver.clone())
			.build();
		assert_ok!(PolkadotXcm::execute(
			RuntimeOrigin::signed(receiver.clone()),
			Box::new(VersionedXcm::from(transfer_xcm)),
			Weight::MAX,
		));
	});

	// It is back on Asset Hub, credited to the beneficiary there.
	AssetHubPolkadot::execute_with(|| {
		type Assets = <AssetHubPolkadot as AssetHubPolkadotPallet>::Assets;
		let balance = <Assets as fungibles::Inspect<_>>::balance(AH_ASSET_ID, &receiver);
		assert!(balance > 0, "the asset should have returned to Asset Hub");
		assert!(balance <= sent_back, "no more than what was sent back can arrive");
	});
}
