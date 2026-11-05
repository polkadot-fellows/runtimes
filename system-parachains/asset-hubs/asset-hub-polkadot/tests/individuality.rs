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

use asset_hub_polkadot_runtime::{
	individuality::{PgasAssetId, PgasMinBalance},
	Assets, Balances, Executive, ExistentialDeposit, Runtime, RuntimeCall, RuntimeEvent,
	SessionKeys, System, TxExtensionV1, UncheckedExtrinsic,
};
use asset_test_utils::ExtBuilder;
use codec::Encode;
use frame_support::{
	assert_ok,
	dispatch::GetDispatchInfo,
	traits::{
		fungible::{Inspect as FungibleInspect, Mutate as FungibleMutate},
		fungibles::{Inspect as FungiblesInspect, Mutate as FungiblesMutate},
	},
};
use parachains_common::{AccountId, AssetHubPolkadotAuraId as AuraId};
use polkadot_runtime_common::claims as pallet_claims;
use polkadot_runtime_constants::system_parachain::ASSET_HUB_ID;
use sp_keyring::Sr25519Keyring;
use sp_runtime::{
	generic,
	traits::{ExtensionVariant, PipelineAtVers, TransactionExtension},
	MultiSignature,
};

const ALICE: [u8; 32] = [1u8; 32];

/// The extension version byte a general (v5) transaction selects for `TxExtensionV1`.
const TX_EXT_VERSION: u8 = 1;

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
		generic::Preamble::General(ExtensionVariant::Other(PipelineAtVers::new(tx_ext))),
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
