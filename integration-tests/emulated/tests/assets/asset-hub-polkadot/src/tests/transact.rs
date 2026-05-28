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

use crate::{assets_balance_on, create_pool_with_dot_on, foreign_balance_on, *};
use asset_hub_polkadot_runtime::xcm_config::DotLocation;
use frame_support::traits::tokens::fungibles::Mutate;
use xcm::latest::AssetTransferFilter;
use xcm_builder::{DescribeAllTerminal, DescribeFamily, HashedDescription};
use xcm_executor::traits::ConvertLocation;

const USDT_ID: u32 = 1984;

/// PenpalA transacts on PenpalB, paying fees using USDT. XCM has to go through Asset Hub as the
/// reserve location of USDT. The original origin `PenpalA/PenpalASender` is proxied by Asset Hub.
fn transfer_and_transact_in_same_xcm(
	destination: Location,
	usdt: Asset,
	beneficiary: Location,
	call: xcm::DoubleEncoded<()>,
) {
	let signed_origin = <PenpalA as Chain>::RuntimeOrigin::signed(PenpalASender::get());
	let context = PenpalUniversalLocation::get();
	let asset_hub_location = PenpalA::sibling_location_of(AssetHubPolkadot::para_id());

	let Fungible(total_usdt) = usdt.fun else { unreachable!() };

	let local_fees_amount = 80_000_000_000;
	let ah_fees_amount = 90_000_000_000;
	let usdt_to_ah_then_onward_amount = total_usdt - local_fees_amount - ah_fees_amount;

	let local_fees: Asset = (usdt.id.clone(), local_fees_amount).into();
	let fees_for_ah: Asset = (usdt.id.clone(), ah_fees_amount).into();
	let usdt_to_ah_then_onward: Asset = (usdt.id.clone(), usdt_to_ah_then_onward_amount).into();

	// xcm to be executed at dest
	let xcm_on_dest = Xcm(vec![
		Transact { origin_kind: OriginKind::Xcm, call, fallback_max_weight: None },
		ExpectTransactStatus(MaybeErrorCode::Success),
		// since this is the last hop, we don't need to further use any assets previously
		// reserved for fees (there are no further hops to cover delivery fees for); we
		// RefundSurplus to get back any unspent fees
		RefundSurplus,
		DepositAsset { assets: Wild(All), beneficiary },
	]);
	let destination = destination.reanchored(&asset_hub_location, &context).unwrap();
	let xcm_on_ah = Xcm(vec![InitiateTransfer {
		destination,
		remote_fees: Some(AssetTransferFilter::ReserveDeposit(Wild(All))),
		preserve_origin: true,
		assets: BoundedVec::new(),
		remote_xcm: xcm_on_dest,
	}]);
	let xcm = Xcm::<()>(vec![
		WithdrawAsset(usdt.into()),
		PayFees { asset: local_fees },
		InitiateTransfer {
			destination: asset_hub_location,
			remote_fees: Some(AssetTransferFilter::ReserveWithdraw(fees_for_ah.into())),
			preserve_origin: true,
			assets: BoundedVec::truncate_from(vec![AssetTransferFilter::ReserveWithdraw(
				usdt_to_ah_then_onward.into(),
			)]),
			remote_xcm: xcm_on_ah,
		},
	]);
	<PenpalA as PenpalAPallet>::PolkadotXcm::execute(
		signed_origin,
		bx!(xcm::VersionedXcm::from(xcm.into())),
		Weight::MAX,
	)
	.unwrap();
}

/// PenpalA transacts on PenpalB, paying fees using USDT. XCM has to go through Asset Hub as the
/// reserve location of USDT. The original origin `PenpalA/PenpalASender` is proxied by Asset Hub.
#[test]
fn transact_from_para_to_para_through_asset_hub() {
	let destination = PenpalA::sibling_location_of(PenpalB::para_id());
	let sender = PenpalASender::get();
	let fee_amount_to_send: Balance = POLKADOT_ED * 10000;
	let sender_chain_as_seen_by_asset_hub =
		AssetHubPolkadot::sibling_location_of(PenpalA::para_id());
	let sov_of_sender_on_asset_hub =
		AssetHubPolkadot::sovereign_account_id_of(sender_chain_as_seen_by_asset_hub);
	let receiver_as_seen_by_asset_hub = AssetHubPolkadot::sibling_location_of(PenpalB::para_id());
	let sov_of_receiver_on_asset_hub =
		AssetHubPolkadot::sovereign_account_id_of(receiver_as_seen_by_asset_hub);

	// Create SA-of-Penpal-on-AHP with ED.
	AssetHubPolkadot::fund_accounts(vec![
		(sov_of_sender_on_asset_hub.clone(), ASSET_HUB_POLKADOT_ED),
		(sov_of_receiver_on_asset_hub.clone(), ASSET_HUB_POLKADOT_ED),
	]);

	// Prefund USDT to sov account of sender.
	AssetHubPolkadot::execute_with(|| {
		type Assets = <AssetHubPolkadot as AssetHubPolkadotPallet>::Assets;
		assert_ok!(<Assets as Mutate<_>>::mint_into(
			USDT_ID,
			&sov_of_sender_on_asset_hub.clone(),
			fee_amount_to_send,
		));
	});

	// We create a pool between DOT and USDT in AssetHub.
	let usdt = Location::new(0, [PalletInstance(ASSETS_PALLET_ID), GeneralIndex(USDT_ID.into())]);
	create_pool_with_dot_on!(
		AssetHubPolkadot,
		usdt,
		false,
		AssetHubPolkadotSender::get(),
		1_000_000_000_000,
		20_000_000_000
	);
	// We also need a pool between DOT and USDT on PenpalA.
	create_pool_with_dot_on!(PenpalA, PenpalUsdtFromAssetHub::get(), true, PenpalAssetOwner::get());
	// We also need a pool between DOT and USDT on PenpalB.
	create_pool_with_dot_on!(PenpalB, PenpalUsdtFromAssetHub::get(), true, PenpalAssetOwner::get());

	let usdt_from_asset_hub = PenpalUsdtFromAssetHub::get();
	PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		assert_ok!(<ForeignAssets as Mutate<_>>::mint_into(
			usdt_from_asset_hub.clone(),
			&sender,
			fee_amount_to_send,
		));
	});

	// Give the sender enough Relay tokens to pay for local delivery fees.
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		DotLocation::get(),
		sender.clone(),
		10_000_000_000_000, // Large estimate to make sure it works.
	);

	// Init values for Parachain Destination
	let receiver = PenpalBReceiver::get();

	// Query initial balances
	let sender_assets_before = foreign_balance_on!(PenpalA, usdt_from_asset_hub.clone(), &sender);
	let receiver_assets_before =
		foreign_balance_on!(PenpalB, usdt_from_asset_hub.clone(), &receiver);

	// Now register a new asset on PenpalB from PenpalA/sender account while paying fees using USDT
	// (going through Asset Hub)

	let usdt_to_send: Asset = (usdt_from_asset_hub.clone(), fee_amount_to_send).into();
	let asset_location_on_penpal_a =
		Location::new(0, [PalletInstance(ASSETS_PALLET_ID), GeneralIndex(ASSET_ID.into())]);
	let penpal_a_as_seen_by_penpal_b = PenpalB::sibling_location_of(PenpalA::para_id());
	let sender_as_seen_by_penpal_b =
		penpal_a_as_seen_by_penpal_b.clone().appended_with(sender.clone()).unwrap();
	let foreign_asset_at_penpal_b =
		penpal_a_as_seen_by_penpal_b.appended_with(asset_location_on_penpal_a).unwrap();
	// Encoded `create_asset` call to be executed in PenpalB
	let call = PenpalB::create_foreign_asset_call(
		foreign_asset_at_penpal_b.clone(),
		ASSET_MIN_BALANCE,
		receiver.clone(),
	);
	PenpalA::execute_with(|| {
		// initiate transaction
		transfer_and_transact_in_same_xcm(destination, usdt_to_send, receiver.clone().into(), call);

		// verify expected events;
		PenpalA::assert_xcm_pallet_attempted_complete(None);
	});
	AssetHubPolkadot::execute_with(|| {
		let sov_penpal_a_on_ah = AssetHubPolkadot::sovereign_account_id_of(
			AssetHubPolkadot::sibling_location_of(PenpalA::para_id()),
		);
		asset_hub_hop_assertions(sov_penpal_a_on_ah);
	});
	PenpalB::execute_with(|| {
		let expected_creator =
			HashedDescription::<AccountId, DescribeFamily<DescribeAllTerminal>>::convert_location(
				&sender_as_seen_by_penpal_b,
			)
			.unwrap();
		penpal_b_assertions(foreign_asset_at_penpal_b, expected_creator, receiver.clone());
	});

	// Query final balances
	let sender_assets_after = foreign_balance_on!(PenpalA, usdt_from_asset_hub.clone(), &sender);
	let receiver_assets_after = foreign_balance_on!(PenpalB, usdt_from_asset_hub, &receiver);

	// Sender's balance is reduced by amount
	assert_eq!(sender_assets_after, sender_assets_before - fee_amount_to_send);
	// Receiver's balance is increased
	assert!(receiver_assets_after > receiver_assets_before);
}

fn asset_hub_hop_assertions(sender_sa: AccountId) {
	type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
	assert_expected_events!(
		AssetHubPolkadot,
		vec![
			// Withdrawn from sender parachain SA
			RuntimeEvent::Assets(
				pallet_assets::Event::Withdrawn { who, .. }
			) => {
				who: *who == sender_sa,
			},
			RuntimeEvent::MessageQueue(
				pallet_message_queue::Event::Processed { success: true, .. }
			) => {},
		]
	);
}

fn penpal_b_assertions(
	expected_asset: Location,
	expected_creator: AccountId,
	expected_owner: AccountId,
) {
	type RuntimeEvent = <PenpalB as Chain>::RuntimeEvent;
	PenpalB::assert_xcmp_queue_success(None);
	assert_expected_events!(
		PenpalB,
		vec![
			RuntimeEvent::ForeignAssets(
				pallet_assets::Event::Created { asset_id, creator, owner }
			) => {
				asset_id: *asset_id == expected_asset,
				creator: *creator == expected_creator,
				owner: *owner == expected_owner,
			},
		]
	);
}

#[test]
fn transact_using_authorized_alias_from_para_to_asset_hub_and_back_to_para() {
	let sender = PenpalASender::get();
	let sov_of_penpal_on_asset_hub = AssetHubPolkadot::sovereign_account_id_of(
		AssetHubPolkadot::sibling_location_of(PenpalA::para_id()),
	);
	let dot_from_parachain_pov: Location = DotLocation::get();
	let usdt_asset_hub_pov =
		Location::new(0, [PalletInstance(ASSETS_PALLET_ID), GeneralIndex(USDT_ID.into())]);
	let usdt_penpal_pov = PenpalUsdtFromAssetHub::get();
	let amount_of_dot_to_transfer_to_ah = POLKADOT_ED * 1_000_000_000u128;
	let amount_of_usdt_we_want_from_exchange = 1_000_000_000u128;
	let max_amount_of_dot_we_allow_for_exchange = 1_000_000_000_000u128;
	// PenpalA uses Kusama as its relay network in the emulated test setup
	let sender_as_seen_from_ah = Location::new(
		1,
		[
			Parachain(2000),
			Junction::AccountId32 { network: Some(NetworkId::Kusama), id: sender.clone().into() },
		],
	);

	// SA-of-Penpal-on-AHP should contain DOT amount equal at least the amount that will be
	// transferred-in to AH Since AH is the reserve for DOT
	AssetHubPolkadot::fund_accounts(vec![(
		sov_of_penpal_on_asset_hub.clone(),
		ASSET_HUB_POLKADOT_ED + amount_of_dot_to_transfer_to_ah,
	)]);
	// Give the sender enough DOT
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		dot_from_parachain_pov.clone(),
		sender.clone(),
		amount_of_dot_to_transfer_to_ah,
	);

	// We create a pool between DOT and USDT in AssetHub so we can do the exchange
	create_pool_with_dot_on!(
		AssetHubPolkadot,
		usdt_asset_hub_pov.clone(),
		false,
		AssetHubPolkadotSender::get(),
		1_000_000_000_000,
		20_000_000_000
	);

	// We add authorized alias on AH so sender from Penpal can AliasOrigin into itself on AH
	// (instead of aliasing into Sovereign Account of sender)
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(
			<AssetHubPolkadot as AssetHubPolkadotPallet>::PolkadotXcm::add_authorized_alias(
				<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(sender.clone()),
				Box::new(sender_as_seen_from_ah.into()),
				None
			)
		);
	});

	// Query initial balances
	let sender_usdt_on_penpal_before =
		foreign_balance_on!(PenpalA, usdt_penpal_pov.clone(), &sender);
	let sender_usdt_on_ah_before = assets_balance_on!(AssetHubPolkadot, USDT_ID, &sender);

	// Encoded `swap_tokens_for_exact_tokens` call to be executed in AH
	let call = <AssetHubPolkadot as Chain>::RuntimeCall::AssetConversion(
		pallet_asset_conversion::Call::swap_tokens_for_exact_tokens {
			path: vec![
				Box::new(dot_from_parachain_pov.clone()),
				Box::new(usdt_asset_hub_pov.clone()),
			],
			amount_out: amount_of_usdt_we_want_from_exchange,
			amount_in_max: max_amount_of_dot_we_allow_for_exchange,
			send_to: sender.clone(),
			keep_alive: true,
		},
	)
	.encode()
	.into();

	let asset_hub_location_penpal_pov = PenpalA::sibling_location_of(AssetHubPolkadot::para_id());
	let penpal_location_ah_pov = AssetHubPolkadot::sibling_location_of(PenpalA::para_id());

	PenpalA::execute_with(|| {
		let sender_signed_origin = <PenpalA as Chain>::RuntimeOrigin::signed(sender.clone());

		let local_fees_amount = 80_000_000_000_000u128;
		let remote_fees_amount = 90_000_000_000_000u128;

		let penpal_local_fees: Asset = (dot_from_parachain_pov.clone(), local_fees_amount).into();
		let ah_remote_fees: Asset = (dot_from_parachain_pov.clone(), remote_fees_amount).into();
		let penpal_remote_fees: Asset = (dot_from_parachain_pov.clone(), remote_fees_amount).into();
		let dot_to_withdraw: Asset =
			(dot_from_parachain_pov.clone(), amount_of_dot_to_transfer_to_ah).into();

		// xcm to be executed by penpal, sent by ah
		let xcm_back_on_penpal = Xcm(vec![
			RefundSurplus,
			DepositAsset { assets: Wild(All), beneficiary: sender.clone().into() },
		]);
		// xcm to be executed by ah, sent by penpal
		let xcm_on_ah = Xcm(vec![
			// aliasing into sender itself, as opposed to sender's sovereign account
			// its possible due to add_authorized_alias above
			AliasOrigin(Location::new(
				0,
				[Junction::AccountId32 { network: None, id: sender.clone().into() }],
			)),
			DepositAsset {
				assets: Definite(
					(dot_from_parachain_pov.clone(), max_amount_of_dot_we_allow_for_exchange)
						.into(),
				),
				beneficiary: sender.clone().into(),
			},
			Transact { origin_kind: OriginKind::SovereignAccount, call, fallback_max_weight: None },
			ExpectTransactStatus(MaybeErrorCode::Success),
			WithdrawAsset(
				(usdt_asset_hub_pov.clone(), amount_of_usdt_we_want_from_exchange).into(),
			),
			InitiateTransfer {
				destination: penpal_location_ah_pov,
				remote_fees: Some(AssetTransferFilter::ReserveDeposit(
					penpal_remote_fees.clone().into(),
				)),
				preserve_origin: false,
				assets: BoundedVec::truncate_from(vec![AssetTransferFilter::ReserveDeposit(Wild(
					All,
				))]),
				remote_xcm: xcm_back_on_penpal,
			},
			RefundSurplus,
			DepositAsset { assets: Wild(All), beneficiary: sender.clone().into() },
		]);
		// xcm to be executed locally on penpal as starting point
		let xcm = Xcm::<()>(vec![
			WithdrawAsset(dot_to_withdraw.into()),
			PayFees { asset: penpal_local_fees },
			InitiateTransfer {
				destination: asset_hub_location_penpal_pov,
				remote_fees: Some(AssetTransferFilter::ReserveWithdraw(
					ah_remote_fees.clone().into(),
				)),
				preserve_origin: true,
				assets: BoundedVec::truncate_from(vec![AssetTransferFilter::ReserveWithdraw(
					Wild(All),
				)]),
				remote_xcm: xcm_on_ah,
			},
			RefundSurplus,
			DepositAsset { assets: Wild(All), beneficiary: sender.clone().into() },
		]);
		// initiate transaction
		<PenpalA as PenpalAPallet>::PolkadotXcm::execute(
			sender_signed_origin,
			bx!(xcm::VersionedXcm::from(xcm.into())),
			Weight::MAX,
		)
		.unwrap();

		// verify expected events;
		PenpalA::assert_xcm_pallet_attempted_complete(None);
	});
	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
				RuntimeEvent::AssetConversion(
					pallet_asset_conversion::Event::SwapExecuted { amount_out, ..}
				) => { amount_out: *amount_out == amount_of_usdt_we_want_from_exchange, },
			]
		);
	});

	PenpalA::execute_with(|| {
		type RuntimeEvent = <PenpalA as Chain>::RuntimeEvent;
		assert_expected_events!(
			PenpalA,
			vec![
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	// Query final balances
	let sender_usdt_on_ah_after = assets_balance_on!(AssetHubPolkadot, USDT_ID, &sender);
	let sender_usdt_on_penpal_after =
		foreign_balance_on!(PenpalA, usdt_penpal_pov.clone(), &sender);

	// Receiver's balance is increased by usdt amount we got from exchange
	assert_eq!(
		sender_usdt_on_penpal_after,
		sender_usdt_on_penpal_before + amount_of_usdt_we_want_from_exchange
	);
	// Usdt amount on senders account AH side should stay the same i.e. all usdt came from exchange
	// not free balance
	assert_eq!(sender_usdt_on_ah_before, sender_usdt_on_ah_after);
}

#[test]
fn transact_using_sov_account_from_para_to_asset_hub_and_back_to_para() {
	let sender = PenpalASender::get();
	let sov_of_penpal_on_asset_hub = AssetHubPolkadot::sovereign_account_id_of(
		AssetHubPolkadot::sibling_location_of(PenpalA::para_id()),
	);
	let dot_from_parachain_pov: Location = DotLocation::get();
	let usdt_asset_hub_pov =
		Location::new(0, [PalletInstance(ASSETS_PALLET_ID), GeneralIndex(USDT_ID.into())]);
	let usdt_penpal_pov = PenpalUsdtFromAssetHub::get();
	let amount_of_dot_to_transfer_to_ah = POLKADOT_ED * 1_000_000_000u128;
	let amount_of_usdt_we_want_from_exchange = 1_000_000_000u128;
	let max_amount_of_dot_we_allow_for_exchange = 1_000_000_000_000u128;
	// PenpalA uses Kusama as its relay network in the emulated test setup
	let sender_as_seen_from_ah = Location::new(
		1,
		[
			Parachain(2000),
			Junction::AccountId32 { network: Some(NetworkId::Kusama), id: sender.clone().into() },
		],
	);
	let sov_of_sender_on_asset_hub =
		AssetHubPolkadot::sovereign_account_id_of(sender_as_seen_from_ah.clone());

	// SA-of-Penpal-on-AHP should contain DOT amount equal at least the amount that will be
	// transferred-in to AH Since AH is the reserve for DOT
	AssetHubPolkadot::fund_accounts(vec![(
		sov_of_penpal_on_asset_hub.clone(),
		ASSET_HUB_POLKADOT_ED + amount_of_dot_to_transfer_to_ah,
	)]);
	// Give the sender enough DOT
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		dot_from_parachain_pov.clone(),
		sender.clone(),
		amount_of_dot_to_transfer_to_ah,
	);

	// We create a pool between DOT and USDT in AssetHub so we can do the exchange
	create_pool_with_dot_on!(
		AssetHubPolkadot,
		usdt_asset_hub_pov.clone(),
		false,
		AssetHubPolkadotSender::get(),
		1_000_000_000_000,
		20_000_000_000
	);

	// Query initial balances
	let sender_usdt_on_penpal_before =
		foreign_balance_on!(PenpalA, usdt_penpal_pov.clone(), &sender);
	let sender_usdt_on_ah_before =
		assets_balance_on!(AssetHubPolkadot, USDT_ID, &sov_of_sender_on_asset_hub);

	// Encoded `swap_tokens_for_exact_tokens` call to be executed in AH
	let call = <AssetHubPolkadot as Chain>::RuntimeCall::AssetConversion(
		pallet_asset_conversion::Call::swap_tokens_for_exact_tokens {
			path: vec![
				Box::new(dot_from_parachain_pov.clone()),
				Box::new(usdt_asset_hub_pov.clone()),
			],
			amount_out: amount_of_usdt_we_want_from_exchange,
			amount_in_max: max_amount_of_dot_we_allow_for_exchange,
			send_to: sov_of_sender_on_asset_hub.clone(),
			keep_alive: false,
		},
	)
	.encode()
	.into();

	let asset_hub_location_penpal_pov = PenpalA::sibling_location_of(AssetHubPolkadot::para_id());
	let penpal_location_ah_pov = AssetHubPolkadot::sibling_location_of(PenpalA::para_id());

	PenpalA::execute_with(|| {
		let sender_signed_origin = <PenpalA as Chain>::RuntimeOrigin::signed(sender.clone());

		let local_fees_amount = 80_000_000_000_000u128;
		let remote_fees_amount = 90_000_000_000_000u128;

		let penpal_local_fees: Asset = (dot_from_parachain_pov.clone(), local_fees_amount).into();
		let ah_remote_fees: Asset = (dot_from_parachain_pov.clone(), remote_fees_amount).into();
		let penpal_remote_fees: Asset = (dot_from_parachain_pov.clone(), remote_fees_amount).into();
		let dot_to_withdraw: Asset =
			(dot_from_parachain_pov.clone(), amount_of_dot_to_transfer_to_ah).into();

		// xcm to be executed by penpal, sent by ah
		let xcm_back_on_penpal = Xcm(vec![
			RefundSurplus,
			DepositAsset { assets: Wild(All), beneficiary: sender.clone().into() },
		]);
		// xcm to be executed by ah, sent by penpal
		let xcm_on_ah = Xcm(vec![
			DepositAsset {
				assets: Definite(
					(dot_from_parachain_pov.clone(), max_amount_of_dot_we_allow_for_exchange)
						.into(),
				),
				beneficiary: sov_of_sender_on_asset_hub.clone().into(),
			},
			Transact { origin_kind: OriginKind::SovereignAccount, call, fallback_max_weight: None },
			ExpectTransactStatus(MaybeErrorCode::Success),
			WithdrawAsset(
				(usdt_asset_hub_pov.clone(), amount_of_usdt_we_want_from_exchange).into(),
			),
			InitiateTransfer {
				destination: penpal_location_ah_pov,
				remote_fees: Some(AssetTransferFilter::ReserveDeposit(
					penpal_remote_fees.clone().into(),
				)),
				preserve_origin: false,
				assets: BoundedVec::truncate_from(vec![AssetTransferFilter::ReserveDeposit(Wild(
					All,
				))]),
				remote_xcm: xcm_back_on_penpal,
			},
			RefundSurplus,
			DepositAsset {
				assets: Wild(All),
				beneficiary: sov_of_sender_on_asset_hub.clone().into(),
			},
		]);
		// xcm to be executed locally on penpal as starting point
		let xcm = Xcm::<()>(vec![
			WithdrawAsset(dot_to_withdraw.into()),
			PayFees { asset: penpal_local_fees },
			InitiateTransfer {
				destination: asset_hub_location_penpal_pov,
				remote_fees: Some(AssetTransferFilter::ReserveWithdraw(
					ah_remote_fees.clone().into(),
				)),
				preserve_origin: true,
				assets: BoundedVec::truncate_from(vec![AssetTransferFilter::ReserveWithdraw(
					Wild(All),
				)]),
				remote_xcm: xcm_on_ah,
			},
			RefundSurplus,
			DepositAsset { assets: Wild(All), beneficiary: sender.clone().into() },
		]);
		// initiate transaction
		<PenpalA as PenpalAPallet>::PolkadotXcm::execute(
			sender_signed_origin,
			bx!(xcm::VersionedXcm::from(xcm.into())),
			Weight::MAX,
		)
		.unwrap();

		// verify expected events;
		PenpalA::assert_xcm_pallet_attempted_complete(None);
	});
	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
				RuntimeEvent::AssetConversion(
					pallet_asset_conversion::Event::SwapExecuted { amount_out, ..}
				) => { amount_out: *amount_out == amount_of_usdt_we_want_from_exchange, },
			]
		);
	});

	PenpalA::execute_with(|| {
		type RuntimeEvent = <PenpalA as Chain>::RuntimeEvent;
		assert_expected_events!(
			PenpalA,
			vec![
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	// Query final balances
	let sender_usdt_on_ah_after =
		assets_balance_on!(AssetHubPolkadot, USDT_ID, &sov_of_sender_on_asset_hub);
	let sender_usdt_on_penpal_after =
		foreign_balance_on!(PenpalA, usdt_penpal_pov.clone(), &sender);

	// Receiver's balance is increased by usdt amount we got from exchange
	assert_eq!(
		sender_usdt_on_penpal_after,
		sender_usdt_on_penpal_before + amount_of_usdt_we_want_from_exchange
	);
	// Usdt amount on senders account AH side should stay the same i.e. all usdt came from exchange
	// not free balance
	assert_eq!(sender_usdt_on_ah_before, sender_usdt_on_ah_after);
}
