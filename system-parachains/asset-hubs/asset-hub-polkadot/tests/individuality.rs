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

//! Individuality-related tests for the Asset Hub Polkadot runtime, ported from the
//! `next-asset-hub-paseo` runtime tests of the `individuality-community` repository.
//!
//! Differences from the original test suite:
//!
//! * The scarcity and NFT-claims pallets are not deployed on Asset Hub Polkadot, so the original
//!   `pgas_pays_the_fee_of_a_persons_nft_claim` and `credit_claimant_origin` tests have no
//!   equivalent here.
//! * Asset Hub Polkadot versions its transaction extension pipeline. `ChargePGAS` only exists in
//!   version 1, and version 1 (unlike version 0, and unlike the single Paseo pipeline) contains no
//!   extension that authenticates a signer, so *no signed transaction can currently pay its fee in
//!   PGAS on this runtime*. The original `pgas_pays_the_fee_of_a_non_revive_call` and
//!   `dot_pays_the_fee_when_pgas_is_insufficient` tests are therefore replaced by the [`pgas_fees`]
//!   module below, which pins that behaviour instead.
//! * `DustRemoval` is `()` on Asset Hub Polkadot, so dust is burned instead of routed to the DAP
//!   buffer as it was on Paseo; the dust test pins the burn.
//! * The external asset teleport filters lived on the Paseo pair of runtimes: on Polkadot the
//!   external asset is HOLLAR, reserve-backed by Hydration, and the corresponding filter tests live
//!   in the people-polkadot runtime (`integration_tests::external_asset_transfers`).

use asset_hub_polkadot_runtime::{
	AllPalletsWithoutSystem, Balances, EthExtraImpl, Executive, ExistentialDeposit, Runtime,
	RuntimeCall, RuntimeOrigin, SessionKeys, System, TxExtensionV0, UncheckedExtrinsic,
};
use asset_test_utils::ExtBuilder;
use codec::Encode;
use frame_support::{
	assert_ok,
	traits::{
		fungible::{Inspect as FungibleInspect, Mutate as FungibleMutate},
		OnIdle, SignedTransactionBuilder,
	},
	weights::Weight,
};
use pallet_revive::evm::runtime::EthExtra;
use parachains_common::{AccountId, Balance};
use polkadot_runtime_constants::system_parachain::ASSET_HUB_ID;
use sp_keyring::Sr25519Keyring;
use sp_runtime::MultiSignature;

const ALICE: [u8; 32] = [1u8; 32];
const BOB: [u8; 32] = [2u8; 32];

type AuraId = parachains_common::AssetHubPolkadotAuraId;

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

/// Builds a signed extrinsic carrying the frozen version-0 extension pipeline, the one every
/// wallet-signed transaction uses today. `EthExtra::get_eth_extension` returns that same
/// `TxExtensionV0` constructed with the right defaults for nonce/tip.
fn construct_extrinsic(sender: Sr25519Keyring, call: RuntimeCall) -> UncheckedExtrinsic {
	let account_id = AccountId::from(sender.public());
	let nonce = frame_system::Pallet::<Runtime>::account(&account_id).nonce;
	let tx_ext: TxExtensionV0 = EthExtraImpl::get_eth_extension(nonce, 0);
	let payload = sp_runtime::generic::SignedPayload::new(call.clone(), tx_ext.clone()).unwrap();
	let signature = payload.using_encoded(|e| sender.sign(e));
	UncheckedExtrinsic::new_signed_transaction(
		call,
		account_id.into(),
		MultiSignature::Sr25519(signature),
		tx_ext,
	)
}

mod dap {
	use super::*;

	#[test]
	fn tx_fees_go_to_dap_buffer() {
		let alice = AccountId::from(Sr25519Keyring::Alice);
		let buffer = pallet_dap::Pallet::<Runtime>::buffer_account();
		let staging = pallet_dap::Pallet::<Runtime>::staging_account();
		let ed = ExistentialDeposit::get();

		// `OnUnbalanced` deposits land in the DAP staging account first; `on_idle` later drains
		// the surplus above ED into the buffer. Pre-fund staging with ED so the drain
		// (`Preservation::Preserve`) can transfer the full fee.
		ExtBuilder::<Runtime>::default()
			.with_collators(vec![alice.clone()])
			.with_session_keys(vec![(
				alice.clone(),
				alice.clone(),
				SessionKeys { aura: AuraId::from(sp_core::ed25519::Public::from_raw(ALICE)) },
			)])
			.with_balances(vec![
				(alice.clone(), 100 * ed),
				(buffer.clone(), ed),
				(staging.clone(), ed),
			])
			.with_para_id(ASSET_HUB_ID.into())
			.build()
			.execute_with(|| {
				let alice_before = <Balances as FungibleInspect<AccountId>>::balance(&alice);
				let buffer_before = <Balances as FungibleInspect<AccountId>>::balance(&buffer);
				let issuance_before = <Balances as FungibleInspect<AccountId>>::total_issuance();

				let call = RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
				let xt = construct_extrinsic(Sr25519Keyring::Alice, call);
				assert_ok!(Executive::apply_extrinsic(xt).unwrap());

				let alice_after = <Balances as FungibleInspect<AccountId>>::balance(&alice);
				let fee_paid = alice_before - alice_after;
				assert!(fee_paid > 0, "a fee should have been paid");

				<AllPalletsWithoutSystem as OnIdle<_>>::on_idle(
					System::block_number(),
					Weight::MAX,
				);

				let buffer_after = <Balances as FungibleInspect<AccountId>>::balance(&buffer);
				let issuance_after = <Balances as FungibleInspect<AccountId>>::total_issuance();

				assert_eq!(buffer_after, buffer_before + fee_paid);
				assert_eq!(issuance_before, issuance_after);
			});
	}

	/// On Paseo dust was routed to the DAP buffer; Asset Hub Polkadot configures
	/// `DustRemoval = ()`, so dust is burned. Pin that.
	#[test]
	fn dust_removal_is_burned() {
		let alice = AccountId::from(ALICE);
		let bob = AccountId::from(BOB);
		let ed = ExistentialDeposit::get();
		let dust = ed / 2;

		test_ext().execute_with(|| {
			assert_ok!(<Balances as FungibleMutate<AccountId>>::mint_into(&bob, ed + dust));
			assert_ok!(<Balances as FungibleMutate<AccountId>>::mint_into(&alice, 100 * ed));

			let issuance_before = <Balances as FungibleInspect<AccountId>>::total_issuance();

			// Transfer ED away from bob, leaving dust < ED → account reaped, dust burned.
			assert_ok!(Balances::transfer_allow_death(
				RuntimeOrigin::signed(bob.clone()),
				alice.clone().into(),
				ed,
			));

			assert_eq!(<Balances as FungibleInspect<AccountId>>::balance(&bob), 0);
			assert_eq!(
				<Balances as FungibleInspect<AccountId>>::total_issuance(),
				issuance_before - dust,
			);
		});
	}
}

/// PGAS as a fee asset.
///
/// `pallet_pgas_allowance::ChargePGAS` requires a *signed* origin, and on Asset Hub Polkadot it
/// only exists in transaction extension pipeline version 1 — whose origin-authorizing extensions
/// (`AuthorizeCall`, `AsPgas`, `AsDotnsGateway`) never produce a signed origin. Pipeline version
/// 0, the one signed transactions carry, uses the plain `ChargeAssetTxPayment`. Consequently no
/// extrinsic can currently pay its fee in PGAS on this runtime; these tests pin that fact so a
/// change in either direction is a conscious one.
mod pgas_fees {
	use super::*;
	use asset_hub_polkadot_runtime::{
		individuality::{PgasAssetId, PgasMinBalance},
		Assets, RuntimeEvent,
	};
	use frame_support::traits::fungibles::{
		Inspect as FungiblesInspect, Mutate as FungiblesMutate,
	};

	/// A signed transaction pays its fee in DOT even when the signer holds ample PGAS.
	#[test]
	fn signed_transactions_pay_dot_not_pgas() {
		let bob = AccountId::from(Sr25519Keyring::Bob.public());
		let endowment = 100 * ExistentialDeposit::get();

		test_ext().execute_with(|| {
			assert_ok!(indiv_pallet_pgas::Pallet::<Runtime>::do_create_pgas_asset());
			let pgas = PgasAssetId::get();
			let pgas_endowment: Balance = endowment;
			assert!(pgas_endowment >= PgasMinBalance::get());
			assert_ok!(<Assets as FungiblesMutate<AccountId>>::mint_into(
				pgas,
				&bob,
				pgas_endowment
			));
			assert_ok!(<Balances as FungibleMutate<AccountId>>::mint_into(&bob, endowment));

			let call = RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
			let xt = construct_extrinsic(Sr25519Keyring::Bob, call);
			assert_ok!(Executive::apply_extrinsic(xt).unwrap());

			let dot_paid = endowment - <Balances as FungibleInspect<AccountId>>::balance(&bob);
			assert!(dot_paid > 0, "the fee should have been taken from the native balance");
			assert_eq!(
				<Assets as FungiblesInspect<AccountId>>::balance(pgas, &bob),
				pgas_endowment,
				"the signer's PGAS must be left untouched"
			);
			assert!(
				System::events().iter().any(|record| matches!(
					record.event,
					RuntimeEvent::TransactionPayment(
						pallet_transaction_payment::Event::TransactionFeePaid { actual_fee, .. }
					) if actual_fee == dot_paid
				)),
				"a TransactionFeePaid event should report the native fee"
			);
			assert!(
				!System::events().iter().any(|record| matches!(
					record.event,
					RuntimeEvent::PgasAllowance(pallet_pgas_allowance::Event::PGASFeePaid { .. })
				)),
				"no fee can have been taken in PGAS"
			);
		});
	}

	/// A signer holding only PGAS (no DOT) cannot get a signed transaction included at all: the
	/// PGAS fee path is unreachable from the signed (version 0) pipeline.
	#[test]
	fn a_pgas_only_signer_cannot_pay_the_fee() {
		use sp_runtime::transaction_validity::{InvalidTransaction, TransactionValidityError};

		let bob = AccountId::from(Sr25519Keyring::Bob.public());

		test_ext().execute_with(|| {
			assert_ok!(indiv_pallet_pgas::Pallet::<Runtime>::do_create_pgas_asset());
			let pgas = PgasAssetId::get();
			// Ample PGAS, zero DOT.
			assert_ok!(<Assets as FungiblesMutate<AccountId>>::mint_into(
				pgas,
				&bob,
				1_000_000 * ExistentialDeposit::get(),
			));

			let call = RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
			let xt = construct_extrinsic(Sr25519Keyring::Bob, call);
			assert_eq!(
				Executive::apply_extrinsic(xt),
				Err(TransactionValidityError::Invalid(InvalidTransaction::Payment)),
			);
			assert_eq!(
				<Assets as FungiblesInspect<AccountId>>::balance(pgas, &bob),
				1_000_000 * ExistentialDeposit::get(),
				"a rejected transaction does not touch the signer's PGAS"
			);
		});
	}

	/// The structural reason for the above: version 1 is the only pipeline carrying `ChargePGAS`,
	/// and it contains no signature-verifying extension, so no signed origin can ever reach it.
	#[test]
	fn no_extension_version_offers_signed_pgas_fee_payment() {
		use sp_runtime::traits::{Pipeline, PipelineMetadataBuilder, TransactionExtension};

		// Version 0 (what `Preamble::Signed` carries) has the plain asset-conversion payment
		// extension and no PGAS wrapper.
		let v0: Vec<&str> = <TxExtensionV0 as TransactionExtension<RuntimeCall>>::metadata()
			.into_iter()
			.map(|m| m.identifier)
			.collect();
		assert!(v0.contains(&"ChargeAssetTxPayment"));

		// Version 1 carries no signature-verifying extension, so general (v5) transactions cannot
		// present a signed origin to `ChargePGAS` either.
		let mut builder = PipelineMetadataBuilder::new();
		<asset_hub_polkadot_runtime::TxExtensionOtherVersions as Pipeline<RuntimeCall>>::build_metadata(
			&mut builder,
		);
		let v1_ids: Vec<&str> = builder
			.by_version
			.get(&1)
			.expect("extension version 1 must be advertised")
			.iter()
			.map(|i| builder.in_versions[*i as usize].identifier)
			.collect();
		for signature_ext in ["VerifyMultiSignature", "VerifySignature"] {
			assert!(
				!v1_ids.contains(&signature_ext),
				"pipeline v1 unexpectedly gained `{signature_ext}`: signed transactions may now \
				 reach `ChargePGAS`, so replace these pinning tests with real PGAS fee tests \
				 (see the original next-asset-hub-paseo `pgas_fees` module)"
			);
		}
	}
}

/// Worst-case notifier-to-subscriber calls must fit the per-XCM-message weight budget.
/// The calls arrive only via XCM Transact, so their declared dispatch weight is weighed
/// into the message; a call above the MessageQueue service weight is marked permanently
/// overweight and never executes.
mod members_subscriber_xcm_budget {
	use super::*;
	use frame_support::{dispatch::GetDispatchInfo, traits::Get};
	use indiv_pallet_members_subscriber::types::{
		RingRootOp, RingRootUpdate, RingRootUpdatesBatch,
	};
	use indiv_support::{
		crypto::BandersnatchVrfVerifiable,
		traits::{RingExponent, PEOPLE_IDENTIFIER},
	};
	use sp_runtime::BoundedVec;
	use verifiable::{ring::RingDomainSize, GenerateVerifiable};

	/// A full batch whose ring counter sits at the far end of its range, proving the
	/// weight annotation is capped rather than growing with the counter.
	fn worst_case_batch() -> RingRootUpdatesBatch<Runtime> {
		let root = BandersnatchVrfVerifiable::finish_members(
			BandersnatchVrfVerifiable::start_members(RingDomainSize::Domain11),
		);
		let max_updates =
			<Runtime as indiv_pallet_members_subscriber::Config>::MaxUpdatesPerBatch::get();
		let updates = (0..max_updates)
			.map(|i| RingRootUpdate {
				ring_index: i,
				op: RingRootOp::Built { revision: 1, root: root.clone() },
			})
			.collect::<Vec<_>>();
		RingRootUpdatesBatch {
			identifier: *PEOPLE_IDENTIFIER,
			sequence: 1,
			source_time: 1,
			updates: BoundedVec::try_from(updates).expect("within MaxUpdatesPerBatch"),
			next_ring_index: u32::MAX,
		}
	}

	#[test]
	fn subscriber_calls_fit_the_xcm_message_budget() {
		sp_io::TestExternalities::default().execute_with(|| {
			let budget =
				asset_hub_polkadot_runtime::dynamic_params::message_queue::MaxOnInitWeight::get()
					.expect("MQ service weight configured");

			// Transact instruction overhead is negligible next to the slack this asserts.
			let calls = [
				(
					"initialize_ring_roots",
					indiv_pallet_members_subscriber::Call::<Runtime>::initialize_ring_roots {
						ring_exponent: RingExponent::R2e9,
						roots: worst_case_batch(),
					},
				),
				(
					"process_ring_updates",
					indiv_pallet_members_subscriber::Call::<Runtime>::process_ring_updates {
						batch: worst_case_batch(),
					},
				),
				(
					"terminate_subscription",
					indiv_pallet_members_subscriber::Call::<Runtime>::terminate_subscription {},
				),
			];
			for (name, call) in calls {
				let weight = call.get_dispatch_info().call_weight;
				assert!(
					weight.all_lte(budget),
					"`{name}` worst-case weight {weight:?} exceeds the XCM message budget {budget:?}",
				);
			}
		});
	}
}
