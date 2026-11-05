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
use frame_support::{
	assert_noop, assert_ok,
	traits::fungibles::{
		metadata::Inspect as FungiblesMetadataInspect, Create as FungiblesCreate,
		Inspect as FungiblesInspect, Mutate as FungiblesMutate,
	},
	PalletId, StoragePrefixedMap,
};
use remote_externalities::{Builder, Mode, OnlineConfig};
use sp_runtime::{traits::AccountIdConversion, Permill};
use std::env::var;
use xcm::latest::prelude::*;

const PSM_INTERNAL_ASSET_ID: u32 = 50_000_342;
const USDT_ASSET_ID: u32 = 1_984;
const PSM_UNIT: u128 = 1_000_000;
const PSM_SWAP_AMOUNT: u128 = 1_000 * PSM_UNIT;

struct PsmTestEnv {
	internal_asset: Location,
	external_asset: Location,
	caller: AccountId,
	psm_account: AccountId,
}

fn local_asset_location(asset_id: u32) -> Location {
	Location::new(0, [PalletInstance(50), GeneralIndex(asset_id.into())])
}

async fn live_psm_ext() -> remote_externalities::RemoteExternalities<Block> {
	let usdt_metadata_key =
		pallet_assets::Metadata::<Runtime, TrustBackedAssetsInstance>::hashed_key_for(
			USDT_ASSET_ID,
		);
	let uri = var("PSM_RPC")
		.unwrap_or_else(|_| "wss://polkadot-asset-hub-rpc.polkadot.io:443".to_string());

	Builder::<Block>::new()
		.mode(Mode::Online(OnlineConfig {
			transport_uris: vec![uri],
			child_trie: false,
			hashed_prefixes: vec![
				pallet_assets::Asset::<Runtime, TrustBackedAssetsInstance>::final_prefix().to_vec(),
			],
			hashed_keys: vec![usdt_metadata_key],
			..Default::default()
		}))
		.build()
		.await
		.expect("fetch live Asset Hub Polkadot USDT state")
}

fn setup_psm() -> PsmTestEnv {
	let internal_asset = local_asset_location(PSM_INTERNAL_ASSET_ID);
	let external_asset = local_asset_location(USDT_ASSET_ID);
	let caller: AccountId = PalletId(*b"py/test!").into_account_truncating();
	let fee_destination: AccountId = PalletId(*b"psm/test").into_account_truncating();
	let psm_account = Psm::psm_account(&internal_asset);

	assert!(PsmAssets::asset_exists(external_asset.clone()), "USDT must exist on-chain");
	assert_eq!(PsmAssets::decimals(external_asset.clone()), 6);

	if !<crate::Assets as FungiblesInspect<AccountId>>::asset_exists(PSM_INTERNAL_ASSET_ID) {
		pallet_assets::NextAssetId::<Runtime, TrustBackedAssetsInstance>::put(
			PSM_INTERNAL_ASSET_ID,
		);
		assert_ok!(<crate::Assets as FungiblesCreate<AccountId>>::create(
			PSM_INTERNAL_ASSET_ID,
			psm_account.clone(),
			true,
			10_000,
		));
		assert_ok!(crate::Assets::force_set_metadata(
			RuntimeOrigin::root(),
			PSM_INTERNAL_ASSET_ID.into(),
			b"internal".to_vec(),
			b"internal".to_vec(),
			6,
			false,
		));
	}

	for account in [&caller, &psm_account, &fee_destination] {
		let _ = frame_system::Pallet::<Runtime>::inc_providers(account);
	}

	let root_origin: OriginCaller = frame_system::RawOrigin::<AccountId>::Root.into();
	pallet_psm::Psm::<Runtime>::insert(
		&internal_asset,
		pallet_psm::PsmInfo::<Runtime> {
			fee_destination,
			max_debt: 5_000_000 * PSM_UNIT,
			min_swap_amount: 1,
			internal_decimals: 6,
			external_count: 0,
		},
	);
	pallet_psm::PsmAdmin::<Runtime>::insert(
		&internal_asset,
		pallet_psm::PsmAdminInfo::<Runtime> {
			full_admin: root_origin.clone(),
			emergency_admin: root_origin,
			deposit: None,
		},
	);

	assert_ok!(Psm::add_external_asset(
		RuntimeOrigin::root(),
		internal_asset.clone(),
		external_asset.clone(),
	));
	assert_ok!(Psm::set_minting_fee(
		RuntimeOrigin::root(),
		internal_asset.clone(),
		external_asset.clone(),
		Permill::zero(),
	));
	assert_ok!(Psm::set_redemption_fee(
		RuntimeOrigin::root(),
		internal_asset.clone(),
		external_asset.clone(),
		Permill::from_rational(1u32, 10_000u32),
	));
	assert_ok!(Psm::set_asset_ceiling_weight(
		RuntimeOrigin::root(),
		internal_asset.clone(),
		external_asset.clone(),
		Permill::from_percent(100),
	));
	assert_ok!(PsmAssets::mint_into(external_asset.clone(), &caller, 2_000 * PSM_UNIT,));

	PsmTestEnv { internal_asset, external_asset, caller, psm_account }
}

// NOTE: These tests are intentionally ignored because they rely on a live RPC and could make CI
// flaky. Run them manually with:
//
// PSM_RPC=wss://your-rpc.example cargo test -p asset-hub-polkadot-runtime \
//     --features try-runtime remote_tests::psm -- --ignored
#[tokio::test]
#[ignore = "requires a live Asset Hub Polkadot RPC"]
async fn psm_mint_and_redeem_against_live_state() {
	sp_tracing::try_init_simple();
	let mut ext = live_psm_ext().await;
	ext.execute_with(|| {
		let PsmTestEnv { internal_asset, external_asset, caller, psm_account } = setup_psm();
		let external_before = PsmAssets::balance(external_asset.clone(), &caller);

		assert_ok!(Psm::mint(
			RuntimeOrigin::signed(caller.clone()),
			internal_asset.clone(),
			external_asset.clone(),
			PSM_SWAP_AMOUNT,
			Permill::zero(),
		));
		assert_eq!(
			PsmAssets::balance(external_asset.clone(), &caller),
			external_before - PSM_SWAP_AMOUNT
		);
		assert_eq!(PsmAssets::balance(external_asset.clone(), &psm_account), PSM_SWAP_AMOUNT);
		assert_eq!(
			pallet_psm::PsmDebt::<Runtime>::get(&internal_asset, &external_asset),
			PSM_SWAP_AMOUNT
		);

		let internal_balance = PsmAssets::balance(internal_asset.clone(), &caller);
		assert_ok!(Psm::redeem(
			RuntimeOrigin::signed(caller.clone()),
			internal_asset.clone(),
			external_asset.clone(),
			internal_balance,
			Permill::from_rational(1u32, 10_000u32),
		));
		assert_eq!(PsmAssets::balance(internal_asset.clone(), &caller), 0);
		let debt_after = pallet_psm::PsmDebt::<Runtime>::get(&internal_asset, &external_asset);
		assert!(debt_after > 0 && debt_after < PSM_SWAP_AMOUNT);
		let fee_destination = pallet_psm::Psm::<Runtime>::get(internal_asset)
			.expect("PSM configured")
			.fee_destination;
		assert!(
			PsmAssets::balance(local_asset_location(PSM_INTERNAL_ASSET_ID), &fee_destination) > 0
		);
	});
}

#[tokio::test]
#[ignore = "requires a live Asset Hub Polkadot RPC"]
async fn psm_circuit_breaker_against_live_state() {
	sp_tracing::try_init_simple();
	let mut ext = live_psm_ext().await;
	ext.execute_with(|| {
		let PsmTestEnv { internal_asset, external_asset, caller, .. } = setup_psm();
		assert_ok!(Psm::mint(
			RuntimeOrigin::signed(caller.clone()),
			internal_asset.clone(),
			external_asset.clone(),
			PSM_SWAP_AMOUNT,
			Permill::zero(),
		));

		assert_ok!(Psm::set_asset_status(
			RuntimeOrigin::root(),
			internal_asset.clone(),
			external_asset.clone(),
			pallet_psm::CircuitBreakerLevel::MintingDisabled,
		));
		assert_noop!(
			Psm::mint(
				RuntimeOrigin::signed(caller.clone()),
				internal_asset.clone(),
				external_asset.clone(),
				PSM_SWAP_AMOUNT,
				Permill::zero(),
			),
			pallet_psm::Error::<Runtime>::MintingStopped
		);
		assert_ok!(Psm::redeem(
			RuntimeOrigin::signed(caller.clone()),
			internal_asset.clone(),
			external_asset.clone(),
			100 * PSM_UNIT,
			Permill::from_rational(1u32, 10_000u32),
		));

		assert_ok!(Psm::set_asset_status(
			RuntimeOrigin::root(),
			internal_asset.clone(),
			external_asset.clone(),
			pallet_psm::CircuitBreakerLevel::AllDisabled,
		));
		assert_noop!(
			Psm::mint(
				RuntimeOrigin::signed(caller.clone()),
				internal_asset.clone(),
				external_asset.clone(),
				PSM_SWAP_AMOUNT,
				Permill::zero(),
			),
			pallet_psm::Error::<Runtime>::MintingStopped
		);
		assert_noop!(
			Psm::redeem(
				RuntimeOrigin::signed(caller.clone()),
				internal_asset.clone(),
				external_asset.clone(),
				100 * PSM_UNIT,
				Permill::from_rational(1u32, 10_000u32),
			),
			pallet_psm::Error::<Runtime>::AllSwapsStopped
		);

		assert_ok!(Psm::set_asset_status(
			RuntimeOrigin::root(),
			internal_asset.clone(),
			external_asset.clone(),
			pallet_psm::CircuitBreakerLevel::AllEnabled,
		));
		assert_ok!(Psm::mint(
			RuntimeOrigin::signed(caller.clone()),
			internal_asset.clone(),
			external_asset.clone(),
			PSM_SWAP_AMOUNT,
			Permill::zero(),
		));
		assert_ok!(Psm::redeem(
			RuntimeOrigin::signed(caller),
			internal_asset,
			external_asset,
			100 * PSM_UNIT,
			Permill::from_rational(1u32, 10_000u32),
		));
	});
}
