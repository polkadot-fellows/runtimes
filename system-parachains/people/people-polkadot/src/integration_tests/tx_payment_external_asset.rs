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

//! Tests for transaction payment with the external asset (HOLLAR).

use super::*;
use crate::xcm_config::XcmConfig;
use xcm::{
	latest::{
		Asset,
		AssetFilter::Wild,
		Instruction::*,
		Junction, Location,
		WeightLimit::{Limited, Unlimited},
		WildAsset::AllCounted,
		Xcm,
	},
	VersionedXcm,
};
use xcm_executor::traits::WeightBounds;

// Helper: build a signed extrinsic that pays transaction fee in external asset and optional tip.
fn build_signed_ext_with_external_asset_payment(
	who: &sr25519::Pair,
	call: RuntimeCall,
	tip: Balance,
) -> UncheckedExtrinsic {
	let mut tx_ext = base_tx_ext(call.clone());

	let who_account = pair_to_account_id(who);

	// update payment extension to pay in external asset with optional tip
	tx_ext.0 .9 = pallet_asset_tx_payment::ChargeAssetTxPayment::<Runtime>::from(
		tip,
		Some(ExternalAssetLocation::get()),
	);

	// update CheckNonce
	{
		let nonce = frame_system::Pallet::<Runtime>::account_nonce(&who_account);
		tx_ext.0 .7 = frame_system::CheckNonce::<Runtime>::from(nonce);
	}

	// update VerifySignature
	{
		let rest_ext = (
			(
				tx_ext.0 .0 .2.clone(),
				tx_ext.0 .0 .3.clone(),
				tx_ext.0 .0 .4.clone(),
				tx_ext.0 .0 .5.clone(),
				tx_ext.0 .0 .6.clone(),
				tx_ext.0 .0 .7.clone(),
			),
			tx_ext.0 .1.clone(),
			tx_ext.0 .2.clone(),
			tx_ext.0 .3.clone(),
			tx_ext.0 .4.clone(),
			tx_ext.0 .5.clone(),
			tx_ext.0 .6.clone(),
			tx_ext.0 .7.clone(),
			tx_ext.0 .8.clone(),
			tx_ext.0 .9.clone(),
			tx_ext.0 .10.clone(),
		);

		let msg = {
			let implication_base = (TX_EXT_VERSION, &call);
			let implication_explicit = &rest_ext;
			let implication_implicit = &rest_ext.implicit().unwrap();
			let encoded_implications =
				(implication_base, implication_explicit, implication_implicit).encode();
			sp_io::hashing::blake2_256(&encoded_implications)
		};

		let raw_sig = who.sign(&msg);

		tx_ext.0 .0 .1 = pallet_verify_signature::VerifySignature::<Runtime>::new_with_signature(
			MultiSignature::from(raw_sig),
			who_account.clone(),
		);
	}

	finalize_uxt(call, tx_ext)
}

#[test]
fn tx_fee_in_external_asset_with_refund() {
	new_test_ext().execute_with(|| {
		let bob = Sr25519Keyring::Bob.pair();
		let bob_id = Sr25519Keyring::Bob.to_account_id();

		// Mint enough external asset to Bob
		FungibleExternalAsset::mint_into(&bob_id, 10 * HOLLAR_UNITS)
			.expect("mint external asset to bob");

		// Pre-measure pot and bob balances
		let pot = CollatorSelection::account_id();
		let pot_before = FungibleExternalAsset::balance(&pot);
		let bob_before = FungibleExternalAsset::balance(&bob_id);

		// A trivial remark; the fee and tip must both be taken in the external asset.
		let tip: Balance = CENTS; // small but visible
		let call = frame_system::Call::<Runtime>::remark { remark: Vec::new() };
		let uxt = build_signed_ext_with_external_asset_payment(&bob, call.into(), tip);
		Executive::apply_extrinsic(uxt).expect("tx valid").expect("dispatch success");

		// Whatever bob paid must have reached the collators pot in the external asset.
		let pot_after = FungibleExternalAsset::balance(&pot);
		let bob_after = FungibleExternalAsset::balance(&bob_id);

		assert!(pot_after > pot_before);
		assert_eq!(bob_before.saturating_sub(bob_after), pot_after.saturating_sub(pot_before));

		// Ensure native balance unchanged (fee not taken in native)
		// Note: no native mint here, so just check it's zero both before and after.
		assert_eq!(Balances::free_balance(bob_id.clone()), 0);
	});
}

#[test]
fn xcm_execute_paid_in_external_asset() {
	new_test_ext().execute_with(|| {
		let bob = Sr25519Keyring::Bob.pair();
		let bob_id = Sr25519Keyring::Bob.to_account_id();

		// Mint external asset to Bob to cover tx fee and XCM weight purchase
		FungibleExternalAsset::mint_into(&bob_id, 100 * HOLLAR_UNITS)
			.expect("mint external asset to bob");

		// Measure balances before
		let collator_pot = CollatorSelection::account_id();
		let collator_pot_before = FungibleExternalAsset::balance(&collator_pot);
		let bob_before = FungibleExternalAsset::balance(&bob_id);

		// Build an XCM which buys execution using the external asset. Provide a large fee asset;
		// the trader will withdraw only what is needed, and the surplus is deposited back.
		//
		// NOTE: `BuyExecution { weight_limit: Unlimited }` is a no-op in the executor (execution
		// then rides on the weight credit `pallet_xcm::execute` grants), so to actually exercise
		// the HOLLAR trader the message must buy a `Limited` amount — exactly the weighed weight
		// of the message, so that no bought-but-unused weight is left to be refunded into the
		// holding after the final `DepositAsset` (where it would be trapped).
		let fees_asset: Asset = (ExternalAssetLocation::get(), 10 * HOLLAR_UNITS).into();
		let beneficiary =
			Location::new(0, [Junction::AccountId32 { network: None, id: bob_id.clone().into() }]);
		let mut msg = Xcm::<RuntimeCall>(vec![
			WithdrawAsset(fees_asset.clone().into()),
			BuyExecution { fees: fees_asset.clone(), weight_limit: Unlimited },
			RefundSurplus,
			DepositAsset { assets: Wild(AllCounted(1)), beneficiary },
		]);
		let weighed =
			<<XcmConfig as xcm_executor::Config>::Weigher as WeightBounds<RuntimeCall>>::weight(
				&mut msg,
				Weight::MAX,
			)
			.expect("the message is weighable");
		msg.0[1] = BuyExecution { fees: fees_asset, weight_limit: Limited(weighed) };
		let vmsg = VersionedXcm::from(msg);

		// Execute the XCM locally; pay the extrinsic fee in the external asset too (no tip).
		let call =
			pallet_xcm::Call::<Runtime>::execute { message: Box::new(vmsg), max_weight: weighed };
		let uxt = build_signed_ext_with_external_asset_payment(&bob, call.into(), 0);
		Executive::apply_extrinsic(uxt).expect("tx valid").expect("dispatch success");

		// After execution:
		// - The staking pot receives the transaction fee (in external asset) via asset-tx-payment,
		//   and the XCM execution fee via the HOLLAR trader — both credit the same pot.
		let collator_pot_after = FungibleExternalAsset::balance(&collator_pot);
		let bob_after = FungibleExternalAsset::balance(&bob_id);

		assert!(collator_pot_after > collator_pot_before, "collator pot should increase");
		assert!(bob_after < bob_before, "bob should spend external asset");
		assert_eq!(
			collator_pot_after.saturating_sub(collator_pot_before),
			bob_before.saturating_sub(bob_after)
		);

		// Native unaffected for this payer
		assert_eq!(Balances::free_balance(bob_id.clone()), 0);
	});
}
