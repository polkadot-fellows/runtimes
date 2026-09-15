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
	macros::{AssetTransferFilter, XcmPaymentApiV2},
	ASSETS_PALLET_ID, USDT_ID,
};
use frame_support::traits::fungibles;
use people_polkadot_runtime::xcm_config::XcmConfig;
use polkadot_runtime_constants::currency::CENTS as DOT_CENTS;
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
		// We need to open a channel between People and Hydration.
		<PeoplePolkadot as Para>::ParachainSystem::open_outbound_hrmp_channel_for_benchmarks_or_tests(HYDRATION_PARA_ID.into());
		// We need to mint some HOLLAR into our sender.
		assert_ok!(<PeopleAssets as fungibles::Mutate<_>>::mint_into(
			HollarLocation::get(),
			&sender,
			10 * HOLLAR_UNITS,
		));
		let transfer_amount = 10 * HOLLAR_UNITS;
		let fees_amount = 10 * DOT_CENTS;
		let transfer_xcm = Xcm::builder()
			.withdraw_asset((Parent, fees_amount))
			// We need DOT to pay for delivery fees so we need
			// to use all DOT here.
			// TODO: Accept HOLLAR for delivery fees as well.
			.pay_fees((Parent, fees_amount))
			.withdraw_asset((hollar_id.clone(), transfer_amount))
			.initiate_transfer(
				hydration_location,
				Some(AssetTransferFilter::ReserveWithdraw(Definite(
					(hollar_id.clone(), transfer_amount.saturating_div(10)).into(),
				))),
				false,
				vec![AssetTransferFilter::ReserveWithdraw(
					AllOfCounted { id: hollar_id.into(), fun: WildFungible, count: 1 }.into(),
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

/// USDT as it is known on Asset Hub: `PalletInstance(50)/GeneralIndex(1984)`, local to AHP.
fn usdt_on_asset_hub() -> Location {
	Location::new(0, [PalletInstance(ASSETS_PALLET_ID), GeneralIndex(USDT_ID.into())])
}

/// The very same asset, as People has to refer to it.
fn usdt_on_people() -> Location {
	Location::new(
		1,
		[
			Parachain(AssetHubPolkadot::para_id().into()),
			PalletInstance(ASSETS_PALLET_ID),
			GeneralIndex(USDT_ID.into()),
		],
	)
}

/// Executes `xcm` on People as if it had been sent by `origin`.
fn execute_on_people(
	origin: Location,
	xcm: Xcm<<PeoplePolkadot as Chain>::RuntimeCall>,
) -> Result<(), InstructionError> {
	let mut hash = xcm.using_encoded(sp_io::hashing::blake2_256);
	xcm_executor::XcmExecutor::<XcmConfig>::prepare_and_execute(
		origin,
		xcm,
		&mut hash,
		Weight::MAX,
		Weight::zero(),
	)
	.ensure_complete()
}

/// Registers `asset` on People. `pallet-assets` there has `CreateOrigin = EnsureNever`, so an
/// incoming asset is only ever credited if root registered it first — which is what keeps the
/// broad reserve trust below narrow in practice.
fn force_create_on_people(asset: Location, owner: AccountId) {
	PeoplePolkadot::execute_with(|| {
		type RuntimeOrigin = <PeoplePolkadot as Chain>::RuntimeOrigin;
		type PeopleAssets = <PeoplePolkadot as PeoplePolkadotPallet>::Assets;

		assert_ok!(PeopleAssets::force_create(
			RuntimeOrigin::root(),
			asset,
			owner.into(),
			true, // is_sufficient
			1,    // min_balance
		));
	});
}

/// Asset Hub is a trusted reserve for the assets it issues, so one of them can be reserve
/// transferred to People and credited there.
///
/// Execution on People is paid for in DOT teleported alongside the asset: People prices XCM
/// execution in DOT (and, for HOLLAR, at a governance-registered rate), so an arbitrary Asset Hub
/// asset cannot yet pay its own way in. That is a fee question, orthogonal to which reserves are
/// trusted, which is what this test pins down.
#[test]
fn asset_hub_can_reserve_transfer_its_own_assets_to_people() {
	let people_location = AssetHubPolkadot::sibling_location_of(PeoplePolkadot::para_id());
	let people_sov_account = AssetHubPolkadot::sovereign_account_id_of(people_location.clone());
	let sender = AssetHubPolkadotSender::get();
	let receiver = PeoplePolkadotReceiver::get();
	let transfer_amount = 1_000_000_000;
	let fee_amount = 10 * DOT_CENTS;

	// People's sovereign account on Asset Hub is where the reserve is actually held.
	AssetHubPolkadot::fund_accounts(vec![(people_sov_account, ASSET_HUB_POLKADOT_ED * 10)]);
	// The beneficiary needs to exist on People to receive the DOT left over from fees.
	PeoplePolkadot::fund_accounts(vec![(receiver.clone(), PEOPLE_POLKADOT_ED * 10)]);

	force_create_on_people(usdt_on_people(), receiver.clone());

	AssetHubPolkadot::execute_with(|| {
		type AssetHubAssets = <AssetHubPolkadot as AssetHubPolkadotPallet>::Assets;
		assert_ok!(<AssetHubAssets as fungibles::Mutate<_>>::mint_into(
			USDT_ID,
			&sender,
			transfer_amount * 2,
		));
	});

	AssetHubPolkadot::execute_with(|| {
		type RuntimeOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;
		type AssetHubXcm = <AssetHubPolkadot as AssetHubPolkadotPallet>::PolkadotXcm;

		let assets: Assets =
			vec![(Parent, fee_amount).into(), (usdt_on_asset_hub(), transfer_amount).into()].into();

		assert_ok!(AssetHubXcm::transfer_assets_using_type_and_then(
			RuntimeOrigin::signed(sender.clone()),
			bx!(people_location.clone().into()),
			bx!(assets.into()),
			// Asset Hub issues USDT, so it is the reserve for it.
			bx!(TransferType::LocalReserve),
			// Fees on People are paid in DOT, which is teleported rather than reserved.
			bx!(AssetId(Location::parent()).into()),
			bx!(TransferType::Teleport),
			bx!(VersionedXcm::from(
				Xcm::<()>::builder_unsafe()
					.deposit_asset(AllCounted(2), receiver.clone())
					.build()
			)),
			Unlimited,
		));
		AssetHubPolkadot::assert_xcm_pallet_attempted_complete(None);
	});

	PeoplePolkadot::execute_with(|| {
		type RuntimeEvent = <PeoplePolkadot as Chain>::RuntimeEvent;
		type PeopleAssets = <PeoplePolkadot as PeoplePolkadotPallet>::Assets;

		assert_expected_events!(
			PeoplePolkadot,
			vec![
				RuntimeEvent::Assets(pallet_assets::Event::Deposited { asset_id, who, amount }) => {
					asset_id: *asset_id == usdt_on_people(),
					who: *who == receiver,
					amount: *amount == transfer_amount,
				},
			]
		);

		assert_eq!(
			<PeopleAssets as fungibles::Inspect<_>>::balance(usdt_on_people(), &receiver),
			transfer_amount,
		);
	});
}

/// Asset Hub is only the reserve for the assets it issues itself, not for everything it happens to
/// custody, and only Asset Hub may vouch for its own assets.
///
/// Trusting a chain as the reserve for an asset it did not issue gives that asset two reserves, and
/// `ReserveAssetDeposited` *mints* locally — so the impostor reserve could credit People with
/// holdings nobody is backing.
#[test]
fn people_only_trusts_asset_hub_for_asset_hub_native_assets() {
	let asset_hub = PeoplePolkadot::sibling_location_of(AssetHubPolkadot::para_id());
	let hydration = HydrationLocation::get();
	let amount = 1_000_000_000;

	let rejected = |asset: Location, origin: Location| {
		// The barrier only admits messages that pay for their execution up front. Which asset is
		// offered does not matter: `ReserveAssetDeposited` is rejected before `BuyExecution` runs.
		let xcm = Xcm::builder_unsafe()
			.reserve_asset_deposited((asset.clone(), amount))
			.buy_execution((asset, amount), Unlimited)
			.deposit_asset(AllCounted(1), PeoplePolkadotReceiver::get())
			.build();
		assert_eq!(
			execute_on_people(origin, xcm).unwrap_err().error,
			XcmError::UntrustedReserveLocation,
		);
	};

	PeoplePolkadot::execute_with(|| {
		// DOT must never arrive as a reserve asset: it is minted into `Balances` on deposit, with
		// no checking account to bound it. DOT only ever arrives by teleport.
		rejected(Location::parent(), asset_hub.clone());

		// HOLLAR has a reserve already — Hydration — and must not gain a second one.
		rejected(HollarLocation::get(), asset_hub.clone());

		// Nor may Asset Hub vouch for the assets of other chains it merely custodies.
		rejected(Location::new(1, [Parachain(2000), GeneralIndex(1)]), asset_hub.clone());
		rejected(
			Location::new(2, [GlobalConsensus(NetworkId::Ethereum { chain_id: 1 })]),
			asset_hub,
		);

		// And an Asset Hub asset is only accepted when Asset Hub is the one sending it.
		rejected(usdt_on_people(), hydration);
		rejected(usdt_on_people(), Location::parent());
	});
}

/// Reserve transfer is the only way in for an Asset Hub asset: People teleports DOT and nothing
/// else.
#[test]
fn asset_hub_assets_are_not_teleportable_to_people() {
	let asset_hub = PeoplePolkadot::sibling_location_of(AssetHubPolkadot::para_id());
	let transfer_amount = 1_000_000_000;

	PeoplePolkadot::execute_with(|| {
		// Pay for execution so the barrier admits the message and the teleport check is what fails.
		let xcm = Xcm::builder_unsafe()
			.receive_teleported_asset((usdt_on_people(), transfer_amount))
			.buy_execution((usdt_on_people(), transfer_amount), Unlimited)
			.deposit_asset(AllCounted(1), PeoplePolkadotReceiver::get())
			.build();
		assert_eq!(
			execute_on_people(asset_hub, xcm).unwrap_err().error,
			XcmError::UntrustedTeleportLocation,
		);
	});
}
