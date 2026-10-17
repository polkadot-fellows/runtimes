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

//! Individuality-related tests for the Asset Hub Polkadot runtime.
//!
//! PGAS as a fee asset: `pallet_pgas_allowance::ChargePGAS` requires a *signed* origin, and it
//! only exists in transaction extension pipeline version 1, reachable through general (v5)
//! transactions. There the signer is authenticated by `pallet_verify_signature`'s
//! `VerifySignature` extension, which turns the `None` origin into a signed one before
//! `ChargePGAS` runs. These tests are ported from the `next-asset-hub-paseo` runtime of the
//! `individuality-community` repository, where the equivalent tests use old-school signed (v4)
//! transactions because that runtime carries `ChargePGAS` in its version-0 pipeline.
//!
//! Scarcity: an NFT-only purse key transacts through `AsScarcity` in transaction extension
//! pipeline version 2, holding neither a System account nor a balance.

use asset_hub_polkadot_runtime::{
	individuality::{PgasAssetId, PgasMinBalance},
	Assets, Balances, Executive, ExistentialDeposit, Runtime, RuntimeCall, RuntimeEvent,
	RuntimeHoldReason, RuntimeOrigin, Scarcity, SessionKeys, System, TxExtensionV1, TxExtensionV2,
	UncheckedExtrinsic,
};
use asset_test_utils::ExtBuilder;
use codec::Encode;
use frame_support::{
	assert_ok,
	dispatch::GetDispatchInfo,
	traits::{
		fungible::{Inspect as FungibleInspect, InspectHold, Mutate as FungibleMutate},
		fungibles::{Inspect as FungiblesInspect, Mutate as FungiblesMutate},
		Get,
	},
};
use parachains_common::{AccountId, AssetHubPolkadotAuraId as AuraId};
use polkadot_runtime_common::claims as pallet_claims;
use polkadot_runtime_constants::system_parachain::ASSET_HUB_ID;
use sp_keyring::Sr25519Keyring;
use sp_runtime::{
	generic,
	traits::{
		DispatchTransaction, ExtensionVariant, MultiVersion, PipelineAtVers, TransactionExtension,
	},
	MultiSignature,
};
use system_parachains_constants::polkadot::currency::UNITS;

const ALICE: [u8; 32] = [1u8; 32];

/// The extension version byte a general (v5) transaction selects for `TxExtensionV1`.
const TX_EXT_VERSION: u8 = 1;

/// The extension version byte a general (v5) transaction selects for `TxExtensionV2`.
const TX_EXT_V2_VERSION: u8 = 2;

fn test_ext() -> sp_io::TestExternalities {
	let alice = AccountId::from(ALICE);
	ExtBuilder::<Runtime>::default()
		.with_collators(vec![alice.clone()])
		.with_session_keys(vec![(
			alice.clone(),
			alice,
			SessionKeys { aura: AuraId::from(sp_core::ed25519::Public::from_raw(ALICE)) },
		)])
		.with_para_id(ASSET_HUB_ID.into())
		.build()
}

/// Builds a general (v5) extrinsic carrying the version-1 extension pipeline, with the signer
/// authenticated by `VerifySignature`. This is the only transaction format whose `ChargePGAS`
/// can take the fee in PGAS.
fn construct_v1_signed_extrinsic(sender: Sr25519Keyring, call: RuntimeCall) -> UncheckedExtrinsic {
	let account_id = AccountId::from(sender.public());
	let nonce = frame_system::Pallet::<Runtime>::account(&account_id).nonce;
	let mut tx_ext: TxExtensionV1 = cumulus_pallet_weight_reclaim::StorageWeightReclaim::new((
		(
			(),
			pallet_verify_signature::VerifySignature::<Runtime>::new_disabled(),
			frame_system::AuthorizeCall::<Runtime>::new(),
			indiv_pallet_pgas::AsPgas::<Runtime>::new(None),
			indiv_pallet_dotns_gateway::AsDotnsGateway::<Runtime>::new(None),
		),
		indiv_pallet_origin_restriction::RestrictOrigin::<Runtime>::new(true),
		frame_system::CheckNonZeroSender::<Runtime>::new(),
		frame_system::CheckSpecVersion::<Runtime>::new(),
		frame_system::CheckTxVersion::<Runtime>::new(),
		frame_system::CheckGenesis::<Runtime>::new(),
		frame_system::CheckEra::<Runtime>::from(generic::Era::Immortal),
		frame_system::CheckNonce::<Runtime>::from(nonce),
		frame_system::CheckWeight::<Runtime>::new(),
		pallet_pgas_allowance::ChargePGAS::<
			Runtime,
			pallet_asset_conversion_tx_payment::ChargeAssetTxPayment<Runtime>,
		>::from(pallet_asset_conversion_tx_payment::ChargeAssetTxPayment::<Runtime>::from(
			0, None,
		)),
		pallet_claims::PrevalidateAttests::<Runtime>::new(),
		(
			frame_metadata_hash_extension::CheckMetadataHash::<Runtime>::new(false),
			pallet_revive::evm::tx_extension::SetOrigin::<Runtime>::default(),
		),
	));

	// The implication of `VerifySignature`: every extension after it in the pipeline.
	let rest_ext = (
		(tx_ext.0 .0 .2.clone(), tx_ext.0 .0 .3.clone(), tx_ext.0 .0 .4.clone()),
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
		tx_ext.0 .11.clone(),
	);
	let msg = {
		let implication_base = (TX_EXT_VERSION, &call);
		let implication_explicit = &rest_ext;
		let implication_implicit =
			&TransactionExtension::<RuntimeCall>::implicit(&rest_ext).unwrap();
		let encoded_implications =
			(implication_base, implication_explicit, implication_implicit).encode();
		sp_io::hashing::blake2_256(&encoded_implications)
	};
	tx_ext.0 .0 .1 = pallet_verify_signature::VerifySignature::<Runtime>::new_with_signature(
		MultiSignature::Sr25519(sender.sign(&msg)),
		account_id,
	);

	sp_runtime::generic::UncheckedExtrinsic::from_parts(
		call,
		generic::Preamble::General(ExtensionVariant::Other(MultiVersion::A(PipelineAtVers::new(
			tx_ext,
		)))),
	)
	.into()
}

/// A general (v5) transaction signed through `VerifySignature` pays its fee in PGAS, holding no
/// native balance at all.
#[test]
fn pgas_pays_the_fee_of_a_v1_signed_call() {
	let bob = AccountId::from(Sr25519Keyring::Bob.public());

	test_ext().execute_with(|| {
		assert_ok!(indiv_pallet_pgas::Pallet::<Runtime>::do_create_pgas_asset());
		let pgas = PgasAssetId::get();
		let endowment = 100 * ExistentialDeposit::get();
		assert_ok!(<Assets as FungiblesMutate<AccountId>>::mint_into(pgas, &bob, endowment));

		let call = RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
		let xt = construct_v1_signed_extrinsic(Sr25519Keyring::Bob, call);

		assert_eq!(<Balances as FungibleInspect<AccountId>>::balance(&bob), 0);
		assert_ok!(Executive::apply_extrinsic(xt).unwrap());

		let paid = endowment - <Assets as FungiblesInspect<AccountId>>::balance(pgas, &bob);
		assert!(paid > 0, "the fee should have been taken in PGAS");
		assert_eq!(
			<Balances as FungibleInspect<AccountId>>::balance(&bob),
			0,
			"the signer holds no native balance, so nothing else can have paid"
		);
		assert!(
			System::events().iter().any(|record| matches!(
				record.event,
				RuntimeEvent::PgasAllowance(
					pallet_pgas_allowance::Event::PGASFeePaid { actual_fee, .. }
				) if actual_fee == paid
			)),
			"a PGASFeePaid event should report the fee burned"
		);
	});
}

/// A signer whose PGAS balance does not cover the fee pays it in the native asset instead.
#[test]
fn dot_pays_the_fee_when_pgas_is_insufficient() {
	let bob = AccountId::from(Sr25519Keyring::Bob.public());
	let endowment = 100 * ExistentialDeposit::get();

	test_ext().execute_with(|| {
		assert_ok!(<Balances as FungibleMutate<AccountId>>::mint_into(&bob, endowment));
		assert_ok!(<Balances as FungibleMutate<AccountId>>::mint_into(
			&pallet_dap::Pallet::<Runtime>::staging_account(),
			ExistentialDeposit::get(),
		));
		assert_ok!(indiv_pallet_pgas::Pallet::<Runtime>::do_create_pgas_asset());
		let pgas = PgasAssetId::get();

		let call = RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
		let xt = construct_v1_signed_extrinsic(Sr25519Keyring::Bob, call);

		let info = xt.get_dispatch_info();
		let fee = pallet_transaction_payment::Pallet::<Runtime>::compute_fee(
			xt.encoded_size() as u32,
			&info,
			0,
		);
		let pgas_endowment = fee - 1;
		assert!(
			pgas_endowment >= PgasMinBalance::get(),
			"the PGAS endowment must be holdable yet insufficient for the fee"
		);
		assert_ok!(<Assets as FungiblesMutate<AccountId>>::mint_into(pgas, &bob, pgas_endowment));

		assert_ok!(Executive::apply_extrinsic(xt).unwrap());

		let paid = endowment - <Balances as FungibleInspect<AccountId>>::balance(&bob);
		assert!(paid > 0, "the fee should have been taken from the native balance");
		assert_eq!(
			<Assets as FungiblesInspect<AccountId>>::balance(pgas, &bob),
			pgas_endowment,
			"the insufficient PGAS balance should be left untouched"
		);
		assert!(
			System::events().iter().any(|record| matches!(
				record.event,
				RuntimeEvent::TransactionPayment(
					pallet_transaction_payment::Event::TransactionFeePaid { actual_fee, .. }
				) if actual_fee == paid
			)),
			"a TransactionFeePaid event should report the native fee"
		);
		assert!(
			!System::events().iter().any(|record| matches!(
				record.event,
				RuntimeEvent::PgasAllowance(pallet_pgas_allowance::Event::PGASFeePaid { .. })
			)),
			"no fee should have been taken in PGAS"
		);
	});
}

/// The version-2 pipeline, authorizing a purse-key call for `state_nonce` of instance 0.
fn scarcity_tx_extension(state_nonce: u64) -> TxExtensionV2 {
	cumulus_pallet_weight_reclaim::StorageWeightReclaim::new((
		(
			(),
			pallet_verify_signature::VerifySignature::<Runtime>::new_disabled(),
			indiv_pallet_scarcity::extension::AsScarcity::<Runtime>::new(Some(
				indiv_pallet_scarcity::extension::AsScarcityInfo::AsNft {
					instance: 0,
					state_nonce,
				},
			)),
			frame_system::AuthorizeCall::<Runtime>::new(),
			indiv_pallet_pgas::AsPgas::<Runtime>::new(None),
			indiv_pallet_dotns_gateway::AsDotnsGateway::<Runtime>::new(None),
		),
		indiv_pallet_origin_restriction::RestrictOrigin::<Runtime>::new(true),
		frame_system::CheckNonZeroSender::<Runtime>::new(),
		frame_system::CheckSpecVersion::<Runtime>::new(),
		frame_system::CheckTxVersion::<Runtime>::new(),
		frame_system::CheckGenesis::<Runtime>::new(),
		frame_system::CheckEra::<Runtime>::from(generic::Era::Immortal),
		frame_system::CheckNonce::<Runtime>::from(0),
		frame_system::CheckWeight::<Runtime>::new(),
		pallet_pgas_allowance::ChargePGAS::<
			Runtime,
			pallet_asset_conversion_tx_payment::ChargeAssetTxPayment<Runtime>,
		>::from(pallet_asset_conversion_tx_payment::ChargeAssetTxPayment::<Runtime>::from(
			0, None,
		)),
		pallet_claims::PrevalidateAttests::<Runtime>::new(),
		(
			frame_metadata_hash_extension::CheckMetadataHash::<Runtime>::new(false),
			pallet_revive::evm::tx_extension::SetOrigin::<Runtime>::default(),
		),
	))
}

/// Runs `test` on the purse key holding instance 0 of a transferable Scarcity item. The key has
/// neither a balance nor a System account.
fn with_scarcity_purse(test: impl FnOnce(AccountId)) {
	test_ext().execute_with(|| {
		frame_system::Pallet::<Runtime>::set_block_number(1);
		pallet_timestamp::Now::<Runtime>::put(1_000_000);

		// The collection owner pays every Scarcity deposit; the purse key pays nothing.
		let owner = AccountId::from(ALICE);
		let purse = AccountId::from([2u8; 32]);
		assert_ok!(<Balances as FungibleMutate<AccountId>>::mint_into(&owner, 100 * UNITS));
		assert_ok!(Scarcity::create_collection(RuntimeOrigin::signed(owner.clone())));
		assert_ok!(Scarcity::define_item(
			RuntimeOrigin::signed(owner.clone()),
			0,
			indiv_pallet_scarcity::Transferability::Transferable,
			Vec::new(),
		));
		assert_ok!(Scarcity::mint(
			RuntimeOrigin::signed(owner.clone()),
			0,
			0,
			purse.clone(),
			Vec::new(),
		));

		let held = Balances::balance_on_hold(
			&RuntimeHoldReason::Scarcity(indiv_pallet_scarcity::HoldReason::StorageDeposit),
			&owner,
		);
		assert!(held > 0, "the collection's storage deposit is held from its owner");
		assert!(!frame_system::Pallet::<Runtime>::account_exists(&purse));

		test(purse);
	});
}

/// An NFT-only purse key can send a feeless transfer through pipeline version 2. This pins the
/// security-critical placement of `AsScarcity` in `TxExtensionV2`.
#[test]
fn nft_only_purse_without_system_account_can_transfer() {
	with_scarcity_purse(|purse| {
		let to = AccountId::from([3u8; 32]);
		let call = RuntimeCall::Scarcity(indiv_pallet_scarcity::Call::transfer { to: to.clone() });
		let info = call.get_dispatch_info();

		let result = scarcity_tx_extension(0).dispatch_transaction(
			RuntimeOrigin::signed(purse.clone()),
			call,
			&info,
			0,
			TX_EXT_V2_VERSION,
		);
		assert!(matches!(result, Ok(Ok(_))), "transaction failed: {result:?}");

		assert!(!frame_system::Pallet::<Runtime>::account_exists(&purse));
		assert!(!indiv_pallet_scarcity::NftsByOwner::<Runtime>::contains_key(&purse));
		assert_eq!(
			indiv_pallet_scarcity::NftsByOwner::<Runtime>::get(&to).map(|nft| nft.state_nonce),
			Some(1),
		);
	});
}

/// A failed purse dispatch charges nothing, restores the NFT, and locks the key for
/// `LockPeriod` — Coinage's retry model.
#[test]
fn failed_scarcity_transfer_is_feeless_and_locks_the_purse() {
	with_scarcity_purse(|purse| {
		// A state nonce at `u64::MAX` passes validation but makes the dispatch overflow.
		indiv_pallet_scarcity::NftsByOwner::<Runtime>::mutate(&purse, |nft| {
			nft.as_mut().expect("purse holds the minted instance").state_nonce = u64::MAX
		});
		let to = AccountId::from([3u8; 32]);
		let call = RuntimeCall::Scarcity(indiv_pallet_scarcity::Call::transfer { to: to.clone() });
		let info = call.get_dispatch_info();

		let result = scarcity_tx_extension(u64::MAX).dispatch_transaction(
			RuntimeOrigin::signed(purse.clone()),
			call,
			&info,
			0,
			TX_EXT_V2_VERSION,
		);
		assert!(matches!(result, Ok(Err(_))), "expected a failed dispatch: {result:?}");

		assert!(!frame_system::Pallet::<Runtime>::account_exists(&purse));
		assert_eq!(
			indiv_pallet_scarcity::NftsByOwner::<Runtime>::get(&purse).map(|nft| nft.state_nonce),
			Some(u64::MAX),
		);
		assert!(!indiv_pallet_scarcity::NftsByOwner::<Runtime>::contains_key(&to));
		let lock_period =
			<<Runtime as indiv_pallet_scarcity::Config>::LockPeriod as Get<u64>>::get();
		assert_eq!(
			indiv_pallet_scarcity::Locked::<Runtime>::get(&purse).map(|lock| lock.until),
			Some(1_000 + lock_period),
		);
	});
}
