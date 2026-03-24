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

use crate::{assets_balance_on, create_pool_with_ksm_on, foreign_balance_on, *};
use asset_hub_kusama_runtime::xcm_config::KsmLocation;
use frame_support::traits::tokens::fungibles::Mutate;
use xcm::latest::AssetTransferFilter;
use xcm_builder::{DescribeAllTerminal, DescribeFamily, HashedDescription};
use xcm_executor::traits::ConvertLocation;

const USDT_ID: u32 = 1984;

fn transfer_and_transact_in_same_xcm(
	destination: Location,
	usdt: Asset,
	beneficiary: Location,
	call: xcm::DoubleEncoded<()>,
) {
	let signed_origin = <PenpalA as Chain>::RuntimeOrigin::signed(PenpalASender::get());
	let context = PenpalUniversalLocation::get();
	let asset_hub_location = PenpalA::sibling_location_of(AssetHubKusama::para_id());

	let Fungible(total_usdt) = usdt.fun else { unreachable!() };

	let local_fees_amount = 80_000_000_000;
	let ah_fees_amount = 90_000_000_000;
	let usdt_to_ah_then_onward_amount = total_usdt - local_fees_amount - ah_fees_amount;

	let local_fees: Asset = (usdt.id.clone(), local_fees_amount).into();
	let fees_for_ah: Asset = (usdt.id.clone(), ah_fees_amount).into();
	let usdt_to_ah_then_onward: Asset = (usdt.id.clone(), usdt_to_ah_then_onward_amount).into();

	let xcm_on_dest = Xcm(vec![
		Transact { origin_kind: OriginKind::Xcm, call, fallback_max_weight: None },
		ExpectTransactStatus(MaybeErrorCode::Success),
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

#[test]
fn transact_from_para_to_para_through_asset_hub() {
	let destination = PenpalA::sibling_location_of(PenpalB::para_id());
	let sender = PenpalASender::get();
	let fee_amount_to_send: Balance = KUSAMA_ED * 10000;
	let sender_chain_as_seen_by_asset_hub = AssetHubKusama::sibling_location_of(PenpalA::para_id());
	let sov_of_sender_on_asset_hub =
		AssetHubKusama::sovereign_account_id_of(sender_chain_as_seen_by_asset_hub);
	let receiver_as_seen_by_asset_hub = AssetHubKusama::sibling_location_of(PenpalB::para_id());
	let sov_of_receiver_on_asset_hub =
		AssetHubKusama::sovereign_account_id_of(receiver_as_seen_by_asset_hub);

	AssetHubKusama::fund_accounts(vec![
		(sov_of_sender_on_asset_hub.clone(), ASSET_HUB_KUSAMA_ED),
		(sov_of_receiver_on_asset_hub.clone(), ASSET_HUB_KUSAMA_ED),
	]);

	AssetHubKusama::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaPallet>::Assets;
		assert_ok!(<Assets as Mutate<_>>::mint_into(
			USDT_ID,
			&sov_of_sender_on_asset_hub.clone(),
			fee_amount_to_send,
		));
	});

	let usdt = Location::new(0, [PalletInstance(ASSETS_PALLET_ID), GeneralIndex(USDT_ID.into())]);
	create_pool_with_ksm_on!(
		AssetHubKusama,
		usdt,
		false,
		AssetHubKusamaSender::get(),
		1_000_000_000_000,
		20_000_000_000
	);
	create_pool_with_ksm_on!(PenpalA, PenpalUsdtFromAssetHub::get(), true, PenpalAssetOwner::get());
	create_pool_with_ksm_on!(PenpalB, PenpalUsdtFromAssetHub::get(), true, PenpalAssetOwner::get());

	let usdt_from_asset_hub = PenpalUsdtFromAssetHub::get();
	PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		assert_ok!(<ForeignAssets as Mutate<_>>::mint_into(
			usdt_from_asset_hub.clone(),
			&sender,
			fee_amount_to_send,
		));
	});

	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		KsmLocation::get(),
		sender.clone(),
		10_000_000_000_000,
	);

	let receiver = PenpalBReceiver::get();

	let sender_assets_before = foreign_balance_on!(PenpalA, usdt_from_asset_hub.clone(), &sender);
	let receiver_assets_before =
		foreign_balance_on!(PenpalB, usdt_from_asset_hub.clone(), &receiver);

	let usdt_to_send: Asset = (usdt_from_asset_hub.clone(), fee_amount_to_send).into();
	let asset_location_on_penpal_a =
		Location::new(0, [PalletInstance(ASSETS_PALLET_ID), GeneralIndex(ASSET_ID.into())]);
	let penpal_a_as_seen_by_penpal_b = PenpalB::sibling_location_of(PenpalA::para_id());
	let sender_as_seen_by_penpal_b =
		penpal_a_as_seen_by_penpal_b.clone().appended_with(sender.clone()).unwrap();
	let foreign_asset_at_penpal_b =
		penpal_a_as_seen_by_penpal_b.appended_with(asset_location_on_penpal_a).unwrap();
	let call = PenpalB::create_foreign_asset_call(
		foreign_asset_at_penpal_b.clone(),
		ASSET_MIN_BALANCE,
		receiver.clone(),
	);
	PenpalA::execute_with(|| {
		transfer_and_transact_in_same_xcm(destination, usdt_to_send, receiver.clone().into(), call);
		PenpalA::assert_xcm_pallet_attempted_complete(None);
	});
	AssetHubKusama::execute_with(|| {
		let sov_penpal_a_on_ah = AssetHubKusama::sovereign_account_id_of(
			AssetHubKusama::sibling_location_of(PenpalA::para_id()),
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

	let sender_assets_after = foreign_balance_on!(PenpalA, usdt_from_asset_hub.clone(), &sender);
	let receiver_assets_after = foreign_balance_on!(PenpalB, usdt_from_asset_hub, &receiver);

	assert_eq!(sender_assets_after, sender_assets_before - fee_amount_to_send);
	assert!(receiver_assets_after > receiver_assets_before);
}

fn asset_hub_hop_assertions(sender_sa: AccountId) {
	type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
	assert_expected_events!(
		AssetHubKusama,
		vec![
			RuntimeEvent::Assets(
				pallet_assets::Event::Burned { owner, .. }
			) => {
				owner: *owner == sender_sa,
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
	let sov_of_penpal_on_asset_hub = AssetHubKusama::sovereign_account_id_of(
		AssetHubKusama::sibling_location_of(PenpalA::para_id()),
	);
	let ksm_from_parachain_pov: Location = KsmLocation::get();
	let usdt_asset_hub_pov =
		Location::new(0, [PalletInstance(ASSETS_PALLET_ID), GeneralIndex(USDT_ID.into())]);
	let usdt_penpal_pov = PenpalUsdtFromAssetHub::get();
	let amount_of_ksm_to_transfer_to_ah = KUSAMA_ED * 1_000_000_000u128;
	let amount_of_usdt_we_want_from_exchange = 1_000_000_000u128;
	let max_amount_of_ksm_we_allow_for_exchange = 1_000_000_000_000u128;
	let sender_as_seen_from_ah = Location::new(
		1,
		[
			Parachain(2000),
			Junction::AccountId32 { network: Some(NetworkId::Kusama), id: sender.clone().into() },
		],
	);

	AssetHubKusama::fund_accounts(vec![(
		sov_of_penpal_on_asset_hub.clone(),
		ASSET_HUB_KUSAMA_ED + amount_of_ksm_to_transfer_to_ah,
	)]);
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		ksm_from_parachain_pov.clone(),
		sender.clone(),
		amount_of_ksm_to_transfer_to_ah,
	);

	create_pool_with_ksm_on!(
		AssetHubKusama,
		usdt_asset_hub_pov.clone(),
		false,
		AssetHubKusamaSender::get(),
		1_000_000_000_000,
		20_000_000_000
	);

	AssetHubKusama::execute_with(|| {
		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::PolkadotXcm::add_authorized_alias(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(sender.clone()),
			Box::new(sender_as_seen_from_ah.into()),
			None
		));
	});

	let sender_usdt_on_penpal_before =
		foreign_balance_on!(PenpalA, usdt_penpal_pov.clone(), &sender);
	let sender_usdt_on_ah_before = assets_balance_on!(AssetHubKusama, USDT_ID, &sender);

	let call = <AssetHubKusama as Chain>::RuntimeCall::AssetConversion(
		pallet_asset_conversion::Call::swap_tokens_for_exact_tokens {
			path: vec![
				Box::new(ksm_from_parachain_pov.clone()),
				Box::new(usdt_asset_hub_pov.clone()),
			],
			amount_out: amount_of_usdt_we_want_from_exchange,
			amount_in_max: max_amount_of_ksm_we_allow_for_exchange,
			send_to: sender.clone(),
			keep_alive: true,
		},
	)
	.encode()
	.into();

	let asset_hub_location_penpal_pov = PenpalA::sibling_location_of(AssetHubKusama::para_id());
	let penpal_location_ah_pov = AssetHubKusama::sibling_location_of(PenpalA::para_id());

	PenpalA::execute_with(|| {
		let sender_signed_origin = <PenpalA as Chain>::RuntimeOrigin::signed(sender.clone());

		let local_fees_amount = 80_000_000_000_000u128;
		let remote_fees_amount = 90_000_000_000_000u128;

		let penpal_local_fees: Asset = (ksm_from_parachain_pov.clone(), local_fees_amount).into();
		let ah_remote_fees: Asset = (ksm_from_parachain_pov.clone(), remote_fees_amount).into();
		let penpal_remote_fees: Asset = (ksm_from_parachain_pov.clone(), remote_fees_amount).into();
		let ksm_to_withdraw: Asset =
			(ksm_from_parachain_pov.clone(), amount_of_ksm_to_transfer_to_ah).into();

		let xcm_back_on_penpal = Xcm(vec![
			RefundSurplus,
			DepositAsset { assets: Wild(All), beneficiary: sender.clone().into() },
		]);
		let xcm_on_ah = Xcm(vec![
			AliasOrigin(Location::new(
				0,
				[Junction::AccountId32 { network: None, id: sender.clone().into() }],
			)),
			DepositAsset {
				assets: Definite(
					(ksm_from_parachain_pov.clone(), max_amount_of_ksm_we_allow_for_exchange)
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
		let xcm = Xcm::<()>(vec![
			WithdrawAsset(ksm_to_withdraw.into()),
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
		<PenpalA as PenpalAPallet>::PolkadotXcm::execute(
			sender_signed_origin,
			bx!(xcm::VersionedXcm::from(xcm.into())),
			Weight::MAX,
		)
		.unwrap();

		PenpalA::assert_xcm_pallet_attempted_complete(None);
	});
	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubKusama,
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

	let sender_usdt_on_ah_after = assets_balance_on!(AssetHubKusama, USDT_ID, &sender);
	let sender_usdt_on_penpal_after =
		foreign_balance_on!(PenpalA, usdt_penpal_pov.clone(), &sender);

	assert_eq!(
		sender_usdt_on_penpal_after,
		sender_usdt_on_penpal_before + amount_of_usdt_we_want_from_exchange
	);
	assert_eq!(sender_usdt_on_ah_before, sender_usdt_on_ah_after);
}

#[test]
fn transact_using_sov_account_from_para_to_asset_hub_and_back_to_para() {
	// ... (same pattern as above but without AliasOrigin, deposits to sov_of_sender_on_asset_hub)
	// This test is very similar to the authorized alias one but uses sovereign account approach
	let sender = PenpalASender::get();
	let sov_of_penpal_on_asset_hub = AssetHubKusama::sovereign_account_id_of(
		AssetHubKusama::sibling_location_of(PenpalA::para_id()),
	);
	let ksm_from_parachain_pov: Location = KsmLocation::get();
	let usdt_asset_hub_pov =
		Location::new(0, [PalletInstance(ASSETS_PALLET_ID), GeneralIndex(USDT_ID.into())]);
	let usdt_penpal_pov = PenpalUsdtFromAssetHub::get();
	let amount_of_ksm_to_transfer_to_ah = KUSAMA_ED * 1_000_000_000u128;
	let amount_of_usdt_we_want_from_exchange = 1_000_000_000u128;
	let max_amount_of_ksm_we_allow_for_exchange = 1_000_000_000_000u128;
	let sender_as_seen_from_ah = Location::new(
		1,
		[
			Parachain(2000),
			Junction::AccountId32 { network: Some(NetworkId::Kusama), id: sender.clone().into() },
		],
	);
	let sov_of_sender_on_asset_hub =
		AssetHubKusama::sovereign_account_id_of(sender_as_seen_from_ah.clone());

	AssetHubKusama::fund_accounts(vec![(
		sov_of_penpal_on_asset_hub.clone(),
		ASSET_HUB_KUSAMA_ED + amount_of_ksm_to_transfer_to_ah,
	)]);
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		ksm_from_parachain_pov.clone(),
		sender.clone(),
		amount_of_ksm_to_transfer_to_ah,
	);

	create_pool_with_ksm_on!(
		AssetHubKusama,
		usdt_asset_hub_pov.clone(),
		false,
		AssetHubKusamaSender::get(),
		1_000_000_000_000,
		20_000_000_000
	);

	let sender_usdt_on_penpal_before =
		foreign_balance_on!(PenpalA, usdt_penpal_pov.clone(), &sender);
	let sender_usdt_on_ah_before =
		assets_balance_on!(AssetHubKusama, USDT_ID, &sov_of_sender_on_asset_hub);

	let call = <AssetHubKusama as Chain>::RuntimeCall::AssetConversion(
		pallet_asset_conversion::Call::swap_tokens_for_exact_tokens {
			path: vec![
				Box::new(ksm_from_parachain_pov.clone()),
				Box::new(usdt_asset_hub_pov.clone()),
			],
			amount_out: amount_of_usdt_we_want_from_exchange,
			amount_in_max: max_amount_of_ksm_we_allow_for_exchange,
			send_to: sov_of_sender_on_asset_hub.clone(),
			keep_alive: false,
		},
	)
	.encode()
	.into();

	let asset_hub_location_penpal_pov = PenpalA::sibling_location_of(AssetHubKusama::para_id());
	let penpal_location_ah_pov = AssetHubKusama::sibling_location_of(PenpalA::para_id());

	PenpalA::execute_with(|| {
		let sender_signed_origin = <PenpalA as Chain>::RuntimeOrigin::signed(sender.clone());

		let local_fees_amount = 80_000_000_000_000u128;
		let remote_fees_amount = 90_000_000_000_000u128;

		let penpal_local_fees: Asset = (ksm_from_parachain_pov.clone(), local_fees_amount).into();
		let ah_remote_fees: Asset = (ksm_from_parachain_pov.clone(), remote_fees_amount).into();
		let penpal_remote_fees: Asset = (ksm_from_parachain_pov.clone(), remote_fees_amount).into();
		let ksm_to_withdraw: Asset =
			(ksm_from_parachain_pov.clone(), amount_of_ksm_to_transfer_to_ah).into();

		let xcm_back_on_penpal = Xcm(vec![
			RefundSurplus,
			DepositAsset { assets: Wild(All), beneficiary: sender.clone().into() },
		]);
		let xcm_on_ah = Xcm(vec![
			DepositAsset {
				assets: Definite(
					(ksm_from_parachain_pov.clone(), max_amount_of_ksm_we_allow_for_exchange)
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
		let xcm = Xcm::<()>(vec![
			WithdrawAsset(ksm_to_withdraw.into()),
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
		<PenpalA as PenpalAPallet>::PolkadotXcm::execute(
			sender_signed_origin,
			bx!(xcm::VersionedXcm::from(xcm.into())),
			Weight::MAX,
		)
		.unwrap();

		PenpalA::assert_xcm_pallet_attempted_complete(None);
	});
	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubKusama,
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

	let sender_usdt_on_ah_after =
		assets_balance_on!(AssetHubKusama, USDT_ID, &sov_of_sender_on_asset_hub);
	let sender_usdt_on_penpal_after =
		foreign_balance_on!(PenpalA, usdt_penpal_pov.clone(), &sender);

	assert_eq!(
		sender_usdt_on_penpal_after,
		sender_usdt_on_penpal_before + amount_of_usdt_we_want_from_exchange
	);
	assert_eq!(sender_usdt_on_ah_before, sender_usdt_on_ah_after);
}
