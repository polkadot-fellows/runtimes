// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Cumulus.
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

#![cfg(test)]

use bulletin_polkadot_runtime::{
	storage::{StorageCallInspector, StoragePriorityBoost, ValidateBulletinCalls},
	xcm_config::{GovernanceLocation, LocationToAccountId, PeopleLocation},
	Balances, Block, Executive, HopPromotion, Runtime, RuntimeCall, RuntimeOrigin, System,
	TransactionStorage, TxExtension, UncheckedExtrinsic,
};
use bulletin_transaction_storage_primitives::cids::{
	calculate_cid, CidConfig, HashingAlgorithm, RAW_CODEC,
};
use codec::Encode;
use frame_support::{
	assert_err, assert_noop, assert_ok,
	dispatch::GetDispatchInfo,
	traits::{fungible::Mutate, Contains, Get, Hooks},
};
use pallet_bulletin_data_renewal::{Call as RenewalCall, WeightInfo as _};
use pallet_bulletin_transaction_storage::{
	extension::{AllowanceBasedPriority, ALLOWANCE_PRIORITY_BOOST},
	AllowedAuthorizers, AuthorizationExtent, AuthorizationScope, AuthorizerBudget,
	Call as TxStorageCall, Config as TxStorageConfig, Origin as TxStorageOrigin, Quota,
	TransactionRef, DEFAULT_MAX_TRANSACTION_SIZE, MAX_WRAPPER_DEPTH,
};
use parachains_common::{AccountId, BlockNumber, Signature};
use parachains_runtimes_test_utils::GovernanceOrigin;
use sp_core::{crypto::Ss58Codec, Pair};
use sp_io::TestExternalities;
use sp_keyring::Sr25519Keyring;
use sp_runtime::{
	traits::{TransactionExtension, TxBaseImplication},
	transaction_validity::{
		InvalidTransaction, TransactionPriority, TransactionSource, TransactionValidityError,
	},
	ApplyExtrinsicResult, Either,
};
use std::collections::HashMap;
use system_parachains_constants::polkadot::fee::WeightToFee;
use xcm::latest::prelude::*;
use xcm_runtime_apis::conversions::LocationToAccountHelper;

const ALICE: [u8; 32] = [1u8; 32];

/// Build test externalities, letting the caller adjust the transaction-storage genesis.
fn new_test_ext_with(
	mutate: impl FnOnce(&mut pallet_bulletin_transaction_storage::GenesisConfig<Runtime>),
) -> TestExternalities {
	use bulletin_polkadot_runtime::{BuildStorage, RuntimeGenesisConfig};
	let mut transaction_storage = pallet_bulletin_transaction_storage::GenesisConfig {
		retention_period: 10,
		byte_fee: 0,
		entry_fee: 0,
		..Default::default()
	};
	mutate(&mut transaction_storage);
	let genesis = RuntimeGenesisConfig { transaction_storage, ..Default::default() };
	sp_io::TestExternalities::new(genesis.build_storage().unwrap())
}

fn new_test_ext() -> TestExternalities {
	new_test_ext_with(|_| {})
}

fn advance_block() {
	let current = System::block_number();
	TransactionStorage::on_finalize(current);
	System::on_finalize(current);
	let next = current + 1;
	System::set_block_number(next);
	// `System::on_initialize` alone does not reset the per-block consumed weight/length the way
	// `Executive::initialize_block` does, so clear them here. Tests that apply several
	// large extrinsics across blocks would otherwise hit `ExhaustsResources` spuriously.
	frame_system::BlockWeight::<Runtime>::kill();
	frame_system::BlockSize::<Runtime>::kill();
	System::on_initialize(next);
	TransactionStorage::on_initialize(next);
}

/// Assert that the extrinsic was both accepted by validation and dispatched successfully.
fn assert_ok_ok(apply_result: ApplyExtrinsicResult) {
	assert_ok!(apply_result);
	assert_ok!(apply_result.unwrap());
}

/// Fund `who` so that fee-paying (non-feeless) calls reach the check under test instead of
/// failing earlier at `ChargeTransactionPayment`.
fn fund(who: &AccountId) {
	Balances::mint_into(who, 1_000_000_000_000).unwrap();
}

/// An [`AuthorizerBudget`] with a finite quota, no expiry and fees charged.
fn authorizer_budget(transactions: u32, bytes: u64) -> AuthorizerBudget<BlockNumber> {
	AuthorizerBudget {
		quota: Some(Quota { transactions, bytes }),
		valid_until: None,
		feeless: false,
	}
}

/// Wrap `call` in each `Utility` dispatcher variant, paired with a label for assertions.
fn wrap_call_utility_variants(call: RuntimeCall) -> Vec<(RuntimeCall, &'static str)> {
	vec![
		(
			RuntimeCall::Utility(pallet_utility::Call::batch { calls: vec![call.clone()] }),
			"utility::batch",
		),
		(
			RuntimeCall::Utility(pallet_utility::Call::batch_all { calls: vec![call.clone()] }),
			"utility::batch_all",
		),
		(
			RuntimeCall::Utility(pallet_utility::Call::force_batch { calls: vec![call.clone()] }),
			"utility::force_batch",
		),
		(
			RuntimeCall::Utility(pallet_utility::Call::as_derivative {
				index: 0,
				call: Box::new(call.clone()),
			}),
			"utility::as_derivative",
		),
		// Only one branch runs, but `inspect_wrapper` returns both so that authorization is
		// validated for whichever it turns out to be.
		(
			RuntimeCall::Utility(pallet_utility::Call::if_else {
				main: Box::new(call.clone()),
				fallback: Box::new(call),
			}),
			"utility::if_else",
		),
	]
}

/// Build and sign an `UncheckedExtrinsic` for `sender`, mirroring the runtime's
/// [`bulletin_polkadot_runtime::TxExtension`] (and its order) exactly.
fn construct_extrinsic(sender: sp_core::sr25519::Pair, call: RuntimeCall) -> UncheckedExtrinsic {
	// Provide a known genesis block hash for the immortal era check.
	frame_system::BlockHash::<Runtime>::insert(0, sp_core::H256::default());
	let account_id = AccountId::from(sender.public());
	let nonce = frame_system::Pallet::<Runtime>::account(&account_id).nonce;
	let tx_ext: TxExtension =
		cumulus_pallet_weight_reclaim::StorageWeightReclaim::<Runtime, _>::new((
			frame_system::AuthorizeCall::<Runtime>::new(),
			frame_system::CheckNonZeroSender::<Runtime>::new(),
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckEra::<Runtime>::from(sp_runtime::generic::Era::Immortal),
			frame_system::CheckNonce::<Runtime>::from(nonce),
			frame_system::CheckWeight::<Runtime>::new(),
			pallet_skip_feeless_payment::SkipCheckIfFeeless::from(
				pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(0),
			),
			ValidateBulletinCalls::default(),
			StoragePriorityBoost::default(),
			frame_metadata_hash_extension::CheckMetadataHash::<Runtime>::new(false),
		));
	let payload = sp_runtime::generic::SignedPayload::new(call.clone(), tx_ext.clone())
		.expect("signed payload should be valid");
	let signature = payload.using_encoded(|e| sender.sign(e));
	UncheckedExtrinsic::new_signed(call, account_id.into(), Signature::Sr25519(signature), tx_ext)
}

/// Sign `call` as `sender` and run it through `Executive`, exercising the full
/// transaction-extension pipeline (notably the fee path).
fn construct_and_apply_extrinsic(
	sender: sp_core::sr25519::Pair,
	call: RuntimeCall,
) -> ApplyExtrinsicResult {
	Executive::apply_extrinsic(construct_extrinsic(sender, call))
}

/// Executes `call` as an XCM `Transact` sent by [`GovernanceLocation`], returning the outcome.
fn transact_from_governance(call: RuntimeCall) -> Outcome {
	let message: Xcm<RuntimeCall> = Xcm::builder_unsafe()
		.unpaid_execution(Unlimited, None)
		.transact(OriginKind::Superuser, None, call.encode())
		.build();
	let mut id = [0u8; 32];
	xcm_executor::XcmExecutor::<bulletin_polkadot_runtime::xcm_config::XcmConfig>::prepare_and_execute(
		GovernanceLocation::get(),
		message,
		&mut id,
		Weight::MAX,
		Weight::MAX,
	)
}

mod runtime_sanity {
	use super::*;

	#[test]
	fn location_conversion_works() {
		// the purpose of hardcoded values is to catch an unintended location conversion logic
		// change.
		struct TestCase {
			description: &'static str,
			location: Location,
			expected_account_id_str: &'static str,
		}

		let test_cases = vec![
			// DescribeTerminus
			TestCase {
				description: "DescribeTerminus Parent",
				location: Location::new(1, Here),
				expected_account_id_str: "5Dt6dpkWPwLaH4BBCKJwjiWrFVAGyYk3tLUabvyn4v7KtESG",
			},
			TestCase {
				description: "DescribeTerminus Sibling",
				location: Location::new(1, [Parachain(1111)]),
				expected_account_id_str: "5Eg2fnssmmJnF3z1iZ1NouAuzciDaaDQH7qURAy3w15jULDk",
			},
			// DescribePalletTerminal
			TestCase {
				description: "DescribePalletTerminal Parent",
				location: Location::new(1, [PalletInstance(50)]),
				expected_account_id_str: "5CnwemvaAXkWFVwibiCvf2EjqwiqBi29S5cLLydZLEaEw6jZ",
			},
			TestCase {
				description: "DescribePalletTerminal Sibling",
				location: Location::new(1, [Parachain(1111), PalletInstance(50)]),
				expected_account_id_str: "5GFBgPjpEQPdaxEnFirUoa51u5erVx84twYxJVuBRAT2UP2g",
			},
			// DescribeAccountId32Terminal
			TestCase {
				description: "DescribeAccountId32Terminal Parent",
				location: Location::new(
					1,
					[Junction::AccountId32 { network: None, id: AccountId::from(ALICE).into() }],
				),
				expected_account_id_str: "5DN5SGsuUG7PAqFL47J9meViwdnk9AdeSWKFkcHC45hEzVz4",
			},
			TestCase {
				description: "DescribeAccountId32Terminal Sibling",
				location: Location::new(
					1,
					[
						Parachain(1111),
						Junction::AccountId32 { network: None, id: AccountId::from(ALICE).into() },
					],
				),
				expected_account_id_str: "5DGRXLYwWGce7wvm14vX1Ms4Vf118FSWQbJkyQigY2pfm6bg",
			},
			// DescribeAccountKey20Terminal
			TestCase {
				description: "DescribeAccountKey20Terminal Parent",
				location: Location::new(1, [AccountKey20 { network: None, key: [0u8; 20] }]),
				expected_account_id_str: "5F5Ec11567pa919wJkX6VHtv2ZXS5W698YCW35EdEbrg14cg",
			},
			TestCase {
				description: "DescribeAccountKey20Terminal Sibling",
				location: Location::new(
					1,
					[Parachain(1111), AccountKey20 { network: None, key: [0u8; 20] }],
				),
				expected_account_id_str: "5CB2FbUds2qvcJNhDiTbRZwiS3trAy6ydFGMSVutmYijpPAg",
			},
			// DescribeTreasuryVoiceTerminal
			TestCase {
				description: "DescribeTreasuryVoiceTerminal Parent",
				location: Location::new(
					1,
					[Plurality { id: BodyId::Treasury, part: BodyPart::Voice }],
				),
				expected_account_id_str: "5CUjnE2vgcUCuhxPwFoQ5r7p1DkhujgvMNDHaF2bLqRp4D5F",
			},
			TestCase {
				description: "DescribeTreasuryVoiceTerminal Sibling",
				location: Location::new(
					1,
					[Parachain(1111), Plurality { id: BodyId::Treasury, part: BodyPart::Voice }],
				),
				expected_account_id_str: "5G6TDwaVgbWmhqRUKjBhRRnH4ry9L9cjRymUEmiRsLbSE4gB",
			},
			// DescribeBodyTerminal
			TestCase {
				description: "DescribeBodyTerminal Parent",
				location: Location::new(1, [Plurality { id: BodyId::Unit, part: BodyPart::Voice }]),
				expected_account_id_str: "5EBRMTBkDisEXsaN283SRbzx9Xf2PXwUxxFCJohSGo4jYe6B",
			},
			TestCase {
				description: "DescribeBodyTerminal Sibling",
				location: Location::new(
					1,
					[Parachain(1111), Plurality { id: BodyId::Unit, part: BodyPart::Voice }],
				),
				expected_account_id_str: "5DBoExvojy8tYnHgLL97phNH975CyT45PWTZEeGoBZfAyRMH",
			},
		];

		for tc in test_cases {
			let expected = AccountId::from_string(tc.expected_account_id_str)
				.expect("Invalid AccountId string");

			let got = LocationToAccountHelper::<AccountId, LocationToAccountId>::convert_location(
				tc.location.into(),
			)
			.unwrap();

			assert_eq!(got, expected, "{}", tc.description);
		}
	}

	#[test]
	fn xcm_payment_api_works() {
		parachains_runtimes_test_utils::test_cases::xcm_payment_api_with_native_token_works::<
			Runtime,
			RuntimeCall,
			RuntimeOrigin,
			Block,
			WeightToFee<Runtime>,
		>();
	}

	#[test]
	fn governance_authorize_upgrade_works() {
		use polkadot_runtime_constants::system_parachain::{ASSET_HUB_ID, COLLECTIVES_ID};

		// no - random para
		assert_err!(
			parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
				Runtime,
				RuntimeOrigin,
			>(GovernanceOrigin::Location(Location::new(1, Parachain(12334)))),
			Either::Right(InstructionError { index: 0, error: XcmError::Barrier })
		);
		// ok - AssetHub
		assert_ok!(parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::Location(Location::new(1, Parachain(ASSET_HUB_ID)))));
		// no - Collectives
		assert_err!(
			parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
				Runtime,
				RuntimeOrigin,
			>(GovernanceOrigin::Location(Location::new(1, Parachain(COLLECTIVES_ID)))),
			Either::Right(InstructionError { index: 0, error: XcmError::Barrier })
		);
		// no - Collectives Voice of Fellows plurality
		assert_err!(
			parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
				Runtime,
				RuntimeOrigin,
			>(GovernanceOrigin::LocationAndDescendOrigin(
				Location::new(1, Parachain(COLLECTIVES_ID)),
				Plurality { id: BodyId::Technical, part: BodyPart::Voice }.into()
			)),
			Either::Right(InstructionError { index: 2, error: XcmError::BadOrigin })
		);

		// ok - relaychain
		assert_ok!(parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::Location(Location::parent())));

		// ok - governance location
		assert_ok!(parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::Location(GovernanceLocation::get())));
	}
}

mod invariants {
	//! Asserted against the configured runtime; no state needed.

	use super::*;

	/// See [`pallet_bulletin_transaction_storage::ensure_weight_sanity`].
	#[test]
	fn transaction_storage_weight_sanity() {
		// Collator-side PoV cap: default 85% of max_pov_size.
		// See cumulus/client/consensus/aura/src/collators/slot_based/block_builder_task.rs
		const POV_PERCENT: Option<u64> = Some(85);
		// The expiry sweep and this drain inherent share one block's mandatory budget.
		let renewal_drain =
			<Runtime as pallet_bulletin_data_renewal::Config>::WeightInfo::process_pending_renewals(
				<Runtime as TxStorageConfig>::MaxBlockTransactions::get(),
			);
		pallet_bulletin_transaction_storage::ensure_weight_sanity::<Runtime>(
			POV_PERCENT,
			renewal_drain,
		);
		pallet_bulletin_data_renewal::ensure_weight_sanity::<Runtime>(POV_PERCENT);
	}

	/// Neither pallet's own `integrity_test` sees the other's params.
	#[test]
	fn renew_and_promote_tag_prefixes_differ() {
		TransactionStorage::assert_pool_families_distinct(&[
			(
				"RenewTxParams",
				<Runtime as pallet_bulletin_data_renewal::Config>::RenewTxParams::get(),
			),
			(
				"PromoteTxParams",
				<Runtime as pallet_bulletin_hop_promotion::Config>::PromoteTxParams::get(),
			),
		]);
	}
}

mod granting {
	//! Who may authorize, and what the grant looks like.

	use super::*;

	#[test]
	fn authorize_account_via_root_works() {
		new_test_ext().execute_with(|| {
			let who: AccountId = Sr25519Keyring::Alice.to_account_id();
			assert_ok!(TransactionStorage::authorize_account(
				RuntimeOrigin::root(),
				who.clone(),
				5,
				1024 * 1024,
			));
			assert_eq!(
				TransactionStorage::account_authorization_extent(who),
				AuthorizationExtent {
					transactions_allowance: 5,
					bytes_allowance: 1024 * 1024,
					..Default::default()
				},
			);
		});
	}

	#[test]
	fn authorize_preimage_via_root_works() {
		new_test_ext().execute_with(|| {
			let content_hash = [42u8; 32];
			assert_ok!(TransactionStorage::authorize_preimage(
				RuntimeOrigin::root(),
				content_hash,
				DEFAULT_MAX_TRANSACTION_SIZE as u64,
			));
			assert_eq!(
				TransactionStorage::preimage_authorization_extent(content_hash),
				AuthorizationExtent {
					transactions_allowance: 1,
					bytes_allowance: DEFAULT_MAX_TRANSACTION_SIZE as u64,
					..Default::default()
				},
			);
		});
	}

	#[test]
	fn authorize_account_rejects_unsigned() {
		new_test_ext().execute_with(|| {
			let who: AccountId = Sr25519Keyring::Alice.to_account_id();
			assert_noop!(
				TransactionStorage::authorize_account(RuntimeOrigin::none(), who, 1, 100),
				sp_runtime::DispatchError::BadOrigin,
			);
		});
	}

	#[test]
	fn authorize_account_rejects_signed_non_authorizer() {
		new_test_ext().execute_with(|| {
			let who: AccountId = Sr25519Keyring::Alice.to_account_id();
			assert_noop!(
				TransactionStorage::authorize_account(
					RuntimeOrigin::signed(who.clone()),
					who,
					1,
					100,
				),
				sp_runtime::DispatchError::BadOrigin,
			);
		});
	}

	#[test]
	fn non_authorizer_cannot_sign_authorize_account_extrinsic() {
		// A signer that is not an accepted authorizer is rejected at validation with
		// `BadSigner` (via `ValidateAuthorizedCalls` -> `check_signed`), even when funded.
		new_test_ext().execute_with(|| {
			let eve = Sr25519Keyring::Eve;
			// Fund Eve so the fee check (which runs before the signer check) passes.
			Balances::mint_into(&eve.to_account_id(), 1_000_000_000_000).unwrap();

			let call =
				RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::authorize_account {
					who: Sr25519Keyring::Ferdie.to_account_id(),
					transactions: 0,
					bytes: 1024,
				});

			assert_eq!(
				construct_and_apply_extrinsic(eve.pair(), call),
				Err(TransactionValidityError::Invalid(InvalidTransaction::BadSigner)),
			);
		});
	}

	#[test]
	fn account_authorizer_consumes_quota() {
		new_test_ext().execute_with(|| {
			let authorizer: AccountId = Sr25519Keyring::Charlie.to_account_id();
			let target: AccountId = Sr25519Keyring::Dave.to_account_id();

			// Root registers Charlie as an account-based authorizer with a finite quota.
			assert_ok!(TransactionStorage::add_authorizer(
				RuntimeOrigin::root(),
				authorizer.clone(),
				authorizer_budget(10, 8192),
			));

			// Charlie (a signed `AllowedAuthorizers` entry) authorizes Dave.
			assert_ok!(TransactionStorage::authorize_account(
				RuntimeOrigin::signed(authorizer.clone()),
				target.clone(),
				3,
				1024,
			));

			// Dave received exactly the granted allowance.
			assert_eq!(
				TransactionStorage::account_authorization_extent(target),
				AuthorizationExtent {
					transactions_allowance: 3,
					bytes_allowance: 1024,
					..Default::default()
				},
			);

			// Charlie's authorizer quota was decremented by the granted amounts.
			let remaining =
				AllowedAuthorizers::<Runtime>::get(&authorizer).expect("still registered");
			assert_eq!(remaining.quota, Some(Quota { transactions: 7, bytes: 7168 }));

			// The same works as a signed extrinsic; Charlie pays fees (the budget is not
			// feeless), hence the funding.
			fund(&authorizer);
			let authorize =
				RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::authorize_account {
					who: Sr25519Keyring::Ferdie.to_account_id(),
					transactions: 3,
					bytes: 1024,
				});
			assert_ok_ok(construct_and_apply_extrinsic(
				Sr25519Keyring::Charlie.pair(),
				authorize.clone(),
			));

			// Once removed, the ex-authorizer is rejected at validation, before dispatch.
			assert_ok!(TransactionStorage::remove_authorizer(
				RuntimeOrigin::root(),
				authorizer.clone(),
			));
			assert!(AllowedAuthorizers::<Runtime>::get(&authorizer).is_none());
			assert_eq!(
				construct_and_apply_extrinsic(Sr25519Keyring::Charlie.pair(), authorize),
				Err(TransactionValidityError::Invalid(InvalidTransaction::BadSigner)),
			);
		});
	}

	#[test]
	fn authorizer_registration_is_root_only() {
		// `AuthorizerRegistrarOrigin` is Root. Membership in `AllowedAuthorizers` grants the
		// right to authorize storage, not to register or revoke other authorizers, so even an
		// allowed authorizer is rejected here. The one non-Root removal path,
		// `remove_exhausted_authorizer`, is permissionless but fenced to inactive budgets.
		new_test_ext().execute_with(|| {
			let authorizer: AccountId = Sr25519Keyring::Charlie.to_account_id();
			assert_ok!(TransactionStorage::add_authorizer(
				RuntimeOrigin::root(),
				authorizer.clone(),
				authorizer_budget(10, 1024),
			));

			let signer: AccountId = Sr25519Keyring::Alice.to_account_id();
			let who: AccountId = Sr25519Keyring::Bob.to_account_id();
			for origin in [
				RuntimeOrigin::signed(signer),
				RuntimeOrigin::signed(authorizer.clone()),
				RuntimeOrigin::none(),
			] {
				assert_noop!(
					TransactionStorage::add_authorizer(
						origin.clone(),
						who.clone(),
						authorizer_budget(10, 1024),
					),
					sp_runtime::DispatchError::BadOrigin,
				);
				assert_noop!(
					TransactionStorage::remove_authorizer(origin, authorizer.clone()),
					sp_runtime::DispatchError::BadOrigin,
				);
			}
			assert!(AllowedAuthorizers::<Runtime>::get(&who).is_none());

			// The permissionless cleanup cannot strip an authorizer whose budget is active.
			assert_noop!(
				TransactionStorage::remove_exhausted_authorizer(
					RuntimeOrigin::none(),
					authorizer.clone(),
				),
				pallet_bulletin_transaction_storage::Error::<Runtime>::AuthorizerBudgetNotExhausted,
			);
			assert!(AllowedAuthorizers::<Runtime>::get(&authorizer).is_some());
		});
	}

	#[test]
	fn add_remove_authorizer_manages_system_providers() {
		// Registering an authorizer holds a System provider reference (so a `feeless`
		// authorizer with no balance is not reaped); removing it releases the reference.
		new_test_ext().execute_with(|| {
			let who: AccountId = Sr25519Keyring::Charlie.to_account_id();
			let providers_of = |a: &AccountId| frame_system::Account::<Runtime>::get(a).providers;
			assert_eq!(providers_of(&who), 0);

			assert_ok!(TransactionStorage::add_authorizer(
				RuntimeOrigin::root(),
				who.clone(),
				authorizer_budget(100, 1024),
			));
			assert_eq!(providers_of(&who), 1);

			// Re-adding must not double-bump the provider reference.
			assert_ok!(TransactionStorage::add_authorizer(
				RuntimeOrigin::root(),
				who.clone(),
				authorizer_budget(200, 2048),
			));
			assert_eq!(providers_of(&who), 1);

			assert_ok!(TransactionStorage::remove_authorizer(RuntimeOrigin::root(), who.clone()));
			assert_eq!(providers_of(&who), 0);

			// Re-removing must not underflow.
			assert_ok!(TransactionStorage::remove_authorizer(RuntimeOrigin::root(), who.clone()));
			assert_eq!(providers_of(&who), 0);
		});
	}

	#[test]
	fn valid_until_clamps_granted_authorization_expiry() {
		// `AuthorizationPeriod` on this runtime is 14 days; an authorizer with a short
		// `valid_until` must clamp the grants it issues — a grant cannot outlive its
		// grantor. Expiry after a handful of blocks (rather than 14 days) proves clamping.
		new_test_ext().execute_with(|| {
			advance_block(); // move to block 1 so `valid_until = 4 > now` is accepted.

			let authorizer: AccountId = Sr25519Keyring::Charlie.to_account_id();
			let target: AccountId = Sr25519Keyring::Dave.to_account_id();
			assert_ok!(TransactionStorage::add_authorizer(
				RuntimeOrigin::root(),
				authorizer.clone(),
				AuthorizerBudget { valid_until: Some(4), ..authorizer_budget(10, 100_000) },
			));
			assert_ok!(TransactionStorage::authorize_account(
				RuntimeOrigin::signed(authorizer),
				target.clone(),
				1,
				1024,
			));

			// Still valid before `valid_until`.
			advance_block(); // block 2
			advance_block(); // block 3
			assert_eq!(
				TransactionStorage::account_authorization_extent(target.clone()),
				AuthorizationExtent {
					transactions_allowance: 1,
					bytes_allowance: 1024,
					..Default::default()
				},
			);

			// At `valid_until` (block 4) the grant has expired — clamped, not 14 days out.
			advance_block(); // block 4
			assert_eq!(
				TransactionStorage::account_authorization_extent(target),
				AuthorizationExtent::default(),
			);
		});
	}

	#[test]
	fn authorize_account_fee_path_follows_feeless_flag() {
		// `feeless: false` → fee charged, so an unfunded authorizer is rejected with
		// `Payment`. `feeless: true` → `SkipCheckIfFeeless` skips the charge, so the
		// same unfunded authorizer succeeds.
		let authorizer = Sr25519Keyring::Charlie;
		let call = RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::authorize_account {
			who: Sr25519Keyring::Dave.to_account_id(),
			transactions: 0,
			bytes: 1024,
		});

		let attempt = |feeless: bool, funded: bool| -> ApplyExtrinsicResult {
			new_test_ext().execute_with(|| {
				// Root registers Charlie as an authorizer (this also holds a provider ref,
				// so the account exists even while unfunded).
				assert_ok!(TransactionStorage::add_authorizer(
					RuntimeOrigin::root(),
					authorizer.to_account_id(),
					AuthorizerBudget { feeless, ..authorizer_budget(10, 1024 * 1024) },
				));
				if funded {
					Balances::mint_into(&authorizer.to_account_id(), 1_000_000_000_000_000)
						.unwrap();
				}
				construct_and_apply_extrinsic(authorizer.pair(), call.clone())
			})
		};

		// Charged + funded → dispatched successfully.
		assert_ok!(attempt(false, true).expect("valid"));
		// Charged + unfunded → rejected at the fee check.
		assert_eq!(
			attempt(false, false),
			Err(TransactionValidityError::Invalid(InvalidTransaction::Payment)),
		);
		// Feeless + unfunded → fee skipped, dispatched successfully.
		assert_ok!(attempt(true, false).expect("valid"));
	}
}

mod granting_over_xcm {
	use super::*;

	#[test]
	fn xcm_from_people_chain_is_accepted_as_authorizer() {
		// Construct the XCM origin as it would arrive from the People chain (a sibling parachain).
		// `EnsureXcm<Equals<PeopleLocation>>` accepts origins whose location equals PeopleLocation.
		let people_origin = RuntimeOrigin::from(pallet_xcm::Origin::Xcm(PeopleLocation::get()));
		new_test_ext().execute_with(|| {
			let who: AccountId = Sr25519Keyring::Bob.to_account_id();
			assert_ok!(TransactionStorage::authorize_account(
				people_origin,
				who.clone(),
				3,
				512 * 1024,
			));
			assert_eq!(
				TransactionStorage::account_authorization_extent(who),
				AuthorizationExtent {
					transactions_allowance: 3,
					bytes_allowance: 512 * 1024,
					..Default::default()
				},
			);
		});
	}

	#[test]
	fn xcm_from_non_people_sibling_is_rejected_as_authorizer() {
		use polkadot_runtime_constants::system_parachain::ASSET_HUB_ID;
		let asset_hub_location = Location::new(1, [Parachain(ASSET_HUB_ID)]);
		let non_people_origin = RuntimeOrigin::from(pallet_xcm::Origin::Xcm(asset_hub_location));
		new_test_ext().execute_with(|| {
			let who: AccountId = Sr25519Keyring::Bob.to_account_id();
			assert_noop!(
				TransactionStorage::authorize_account(non_people_origin, who, 1, 100),
				sp_runtime::DispatchError::BadOrigin,
			);
		});
	}

	#[test]
	fn xcm_transact_authorize_account_works() {
		// The counterpart to the two tests above: `authorize_account` is a management call, not a
		// storage-mutating one, so the filter must let it through.
		new_test_ext().execute_with(|| {
			advance_block();

			let target: AccountId = Sr25519Keyring::Ferdie.to_account_id();
			assert_eq!(
				TransactionStorage::account_authorization_extent(target.clone()),
				AuthorizationExtent::default(),
			);

			let authorize =
				RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::authorize_account {
					who: target.clone(),
					transactions: 0,
					bytes: 1024,
				});
			let outcome = transact_from_governance(authorize);

			assert!(
				outcome.clone().ensure_complete().is_ok(),
				"XCM Transact authorize_account must succeed, got: {outcome:?}",
			);
			assert_eq!(
				TransactionStorage::account_authorization_extent(target),
				AuthorizationExtent { bytes_allowance: 1024, ..Default::default() },
			);
		});
	}

	#[test]
	fn people_chain_can_authorize_storage_with_transact() {
		// End-to-end at the executor level: the People chain's `OriginKind::Xcm` origin is
		// accepted as an authorizer. (The emulated variant lives in the integration tests.)
		new_test_ext().execute_with(|| {
			let target: AccountId = Sr25519Keyring::Ferdie.to_account_id();
			let call =
				RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::authorize_account {
					who: target.clone(),
					transactions: 0,
					bytes: 1024,
				});

			let message: Xcm<RuntimeCall> = Xcm::builder_unsafe()
				.unpaid_execution(Unlimited, None)
				.transact(OriginKind::Xcm, None, call.encode())
				.build();
			let mut id = [0u8; 32];
			let outcome = xcm_executor::XcmExecutor::<
				bulletin_polkadot_runtime::xcm_config::XcmConfig,
			>::prepare_and_execute(
				PeopleLocation::get(), message, &mut id, Weight::MAX, Weight::MAX
			);
			assert!(
				outcome.clone().ensure_complete().is_ok(),
				"expected success, got: {outcome:?}"
			);

			assert_eq!(
				TransactionStorage::account_authorization_extent(target),
				AuthorizationExtent { bytes_allowance: 1024, ..Default::default() },
			);
		});
	}

	#[test]
	fn xcm_transact_authorize_account_from_asset_hub_contract() {
		// An `AccountKey20`-descended Asset Hub origin (e.g. a pallet-revive contract) converts to
		// a `Signed` origin via `HashedDescription`. Membership in `AllowedAuthorizers` is what
		// gates the dispatch; on success the hashed account's quota is decremented.
		use polkadot_runtime_constants::system_parachain::ASSET_HUB_ID;

		let contract_addr = [0xAAu8; 20];
		let hashed: AccountId =
			LocationToAccountHelper::<AccountId, LocationToAccountId>::convert_location(
				Location::new(
					1,
					[Parachain(ASSET_HUB_ID), AccountKey20 { network: None, key: contract_addr }],
				)
				.into(),
			)
			.expect("HashedDescription resolves sibling + AccountKey20");

		let target: AccountId = Sr25519Keyring::Ferdie.to_account_id();
		let (txs_budget, bytes_budget) = (1000u32, 100 * 1024 * 1024u64);
		let (txs, bytes) = (3u32, 1024u64);

		let execute_xcm = |registered: bool| {
			new_test_ext_with(|genesis| {
				if registered {
					genesis.allowed_authorizers = vec![(hashed.clone(), txs_budget, bytes_budget)];
				}
			})
			.execute_with(|| {
				let call = RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::authorize_account {
					who: target.clone(),
					transactions: txs,
					bytes,
				});
				let message: Xcm<RuntimeCall> = Xcm::builder_unsafe()
					.unpaid_execution(Unlimited, None)
					.descend_origin(Junctions::from([AccountKey20 {
						network: None,
						key: contract_addr,
					}]))
					.transact(OriginKind::SovereignAccount, None, call.encode())
					.build();
				let mut id = [0u8; 32];
				xcm_executor::XcmExecutor::<bulletin_polkadot_runtime::xcm_config::XcmConfig>::prepare_and_execute(
					Location::new(1, [Parachain(ASSET_HUB_ID)]),
					message,
					&mut id,
					Weight::MAX,
					Weight::MAX,
				);
				(
					TransactionStorage::account_authorization_extent(target.clone()),
					AllowedAuthorizers::<Runtime>::get(&hashed),
				)
			})
		};

		// Unregistered: the inner dispatch fails on origin; nothing is granted.
		assert_eq!(execute_xcm(false), (AuthorizationExtent::default(), None));

		// Registered: the grant lands and the contract's quota shrinks by exactly what it gave.
		let (extent, budget) = execute_xcm(true);
		assert_eq!(
			extent,
			AuthorizationExtent {
				transactions_allowance: txs,
				bytes_allowance: bytes,
				..Default::default()
			},
		);
		let quota = budget
			.expect("hashed contract account is still registered after a partial spend")
			.quota
			.expect("authorizer has a tracked quota");
		assert_eq!(quota.transactions, txs_budget - txs);
		assert_eq!(quota.bytes, bytes_budget - bytes);
	}
}

mod safe_call_filter {
	//! Calls that commit data must be unreachable via XCM `Transact`.
	//! `GovernanceLocation` (Asset Hub) passes the barrier and maps to Superuser, so the filter
	//! is the only thing left to reject the call.

	use super::*;

	#[test]
	fn xcm_transact_store_is_blocked() {
		new_test_ext().execute_with(|| {
			advance_block();

			let who: AccountId = Sr25519Keyring::Alice.to_account_id();
			let data = vec![42u8; 100];

			// Authorize the account, so that a missing authorization cannot be what rejects the
			// call — the filter must block it regardless.
			assert_ok!(TransactionStorage::authorize_account(
				RuntimeOrigin::root(),
				who.clone(),
				0,
				data.len() as u64,
			));
			let granted = TransactionStorage::account_authorization_extent(who.clone());
			assert_ne!(granted, AuthorizationExtent::default());

			let store = RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store { data });
			// Assert the filter's own error: `store` reaches `do_store` without an extrinsic
			// index here and would fail with `BadContext` anyway, so a bare `is_err()` would
			// pass even with the filter removed.
			assert_eq!(
				transact_from_governance(store).ensure_complete().map_err(|e| e.error),
				Err(XcmError::NoPermission),
				"XCM Transact store must be rejected by the SafeCallFilter",
			);
			assert_eq!(
				TransactionStorage::account_authorization_extent(who),
				granted,
				"authorization must remain unconsumed since the XCM was blocked",
			);
		});
	}

	#[test]
	fn xcm_transact_wrapped_store_is_blocked() {
		// `StorageCallInspector` recursively inspects `Utility` wrappers, so nesting the `store`
		// in a `batch` must not get it past the filter.
		new_test_ext().execute_with(|| {
			advance_block();

			let who: AccountId = Sr25519Keyring::Alice.to_account_id();
			let data = vec![42u8; 100];

			assert_ok!(TransactionStorage::authorize_account(
				RuntimeOrigin::root(),
				who.clone(),
				0,
				data.len() as u64,
			));
			let granted = TransactionStorage::account_authorization_extent(who.clone());

			let store = RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store { data });
			for (wrapped, label) in wrap_call_utility_variants(store) {
				assert_eq!(
					transact_from_governance(wrapped).ensure_complete().map_err(|e| e.error),
					Err(XcmError::NoPermission),
					"XCM Transact {label}(store) must be rejected by the recursive SafeCallFilter",
				);
			}
			assert_eq!(TransactionStorage::account_authorization_extent(who), granted);
		});
	}

	#[test]
	fn xcm_transact_renewals_are_blocked() {
		// `force_renew` accepts Root, so only the filter stands between an XCM `Transact` and an
		// unpaid permanent commitment. The registrations would hit `BadOrigin` anyway.
		// `disable_auto_renew` stays dispatchable: Root needs it for cleanup.
		new_test_ext().execute_with(|| {
			advance_block();

			// Store a real entry first, so a `RenewedNotFound` dispatch error cannot be what makes
			// the `Transact` fail — the filter has to be the only thing rejecting it.
			let account = Sr25519Keyring::Alice;
			let who: AccountId = account.to_account_id();
			let data = vec![42u8; 100];
			let content_hash = sp_io::hashing::blake2_256(&data);
			let entry = TransactionRef::ContentHash(content_hash);
			assert_ok!(TransactionStorage::authorize_account(
				RuntimeOrigin::root(),
				who,
				1,
				data.len() as u64,
			));
			assert_ok_ok(construct_and_apply_extrinsic(
				account.pair(),
				RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store { data }),
			));
			advance_block();

			let blocked = [
				("renew", RenewalCall::<Runtime>::renew { entry: entry.clone() }),
				("force_renew", RenewalCall::<Runtime>::force_renew { entry }),
				("enable_auto_renew", RenewalCall::<Runtime>::enable_auto_renew { content_hash }),
			];
			for (name, call) in blocked {
				let call = RuntimeCall::DataRenewal(call);
				// The filter is a pure predicate, so the whole wrapper matrix goes through it...
				for (wrapped, label) in wrap_call_utility_variants(call.clone()) {
					assert!(
						StorageCallInspector::contains(&wrapped),
						"SafeCallFilter must claim {label}({name})",
					);
				}
				// ...and one `Transact` per call proves the predicate is wired as the
				// `SafeCallFilter`. Assert the filter's own error: these calls would fail to
				// dispatch anyway (`BadContext`/`BadOrigin`), so a bare `is_err()` would pass
				// even with the filter removed.
				assert!(StorageCallInspector::contains(&call), "SafeCallFilter must claim {name}");
				assert_eq!(
					transact_from_governance(call).ensure_complete().map_err(|e| e.error),
					Err(XcmError::NoPermission),
					"XCM Transact {name} must be rejected by the SafeCallFilter",
				);
			}

			assert!(
				!StorageCallInspector::contains(&RuntimeCall::DataRenewal(
					RenewalCall::<Runtime>::disable_auto_renew { content_hash }
				)),
				"disable_auto_renew must stay dispatchable for governance cleanup",
			);
		});
	}
}

mod storing {
	//! Sizes, throughput, fees and pool priority.

	use super::*;

	#[test]
	fn store_with_cid_config_works() {
		new_test_ext().execute_with(|| {
			let account = Sr25519Keyring::Alice;
			let who: AccountId = account.to_account_id();
			let data = vec![0u8; 4 * 1024];
			let total_bytes = data.len() as u64;

			assert_ok!(TransactionStorage::authorize_account(
				RuntimeOrigin::root(),
				who.clone(),
				0,
				3 * total_bytes,
			));
			assert_eq!(
				TransactionStorage::account_authorization_extent(who.clone()),
				AuthorizationExtent { bytes_allowance: 3 * total_bytes, ..Default::default() },
			);
			advance_block();
			let block_number = System::block_number();

			// 1. Store data with plain `store` (defaults to Blake2b256, RAW_CODEC 0x55).
			assert_ok_ok(construct_and_apply_extrinsic(
				account.pair(),
				RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store {
					data: data.clone(),
				}),
			));

			// 2. Store with explicit Blake2b256 + RAW_CODEC — should produce the same content_hash.
			assert_ok_ok(construct_and_apply_extrinsic(
				account.pair(),
				RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store_with_cid_config {
					cid: CidConfig { codec: RAW_CODEC, hashing: HashingAlgorithm::Blake2b256 },
					data: data.clone(),
				}),
			));

			// 3. Store with Sha2_256 + dag-pb codec (0x70) — should produce a different
			//    content_hash.
			assert_ok_ok(construct_and_apply_extrinsic(
				account.pair(),
				RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store_with_cid_config {
					cid: CidConfig { codec: 0x70, hashing: HashingAlgorithm::Sha2_256 },
					data: data.clone(),
				}),
			));

			// The three stores consumed exactly the granted byte allowance.
			assert_eq!(
				TransactionStorage::account_authorization_extent(who),
				AuthorizationExtent {
					transactions: 3,
					bytes: 3 * total_bytes,
					bytes_allowance: 3 * total_bytes,
					..Default::default()
				},
			);

			TransactionStorage::on_finalize(block_number);

			let stored_txs = TransactionStorage::transaction_roots(block_number)
				.unwrap()
				.into_iter()
				.enumerate()
				.collect::<HashMap<_, _>>();

			assert_eq!(stored_txs.len(), 3);

			let default_hash = calculate_cid(
				&data,
				CidConfig { codec: RAW_CODEC, hashing: HashingAlgorithm::Blake2b256 },
			)
			.unwrap()
			.content_hash;
			assert_eq!(stored_txs[&0].content_hash, default_hash);
			// Explicit Blake2b256 matches the plain-store default.
			assert_eq!(stored_txs[&0].content_hash, stored_txs[&1].content_hash);
			// Sha2_256 produces a distinct hash.
			assert_ne!(stored_txs[&0].content_hash, stored_txs[&2].content_hash);
		});
	}

	#[test]
	fn transaction_storage_runtime_sizes() {
		// Sweep the whole valid size range through the real extrinsic pipeline (not the pallet
		// call directly), then confirm `MaxTransactionSize + 1` is refused.
		new_test_ext().execute_with(|| {
			let account = Sr25519Keyring::Alice;
			let who: AccountId = account.to_account_id();
			let max =
				<<Runtime as TxStorageConfig>::MaxTransactionSize as Get<u32>>::get() as usize;
			let sizes: [usize; 6] = [1, 2000, max / 4, max / 2, max * 3 / 4, max];
			let total_bytes: u64 = sizes.iter().map(|s| *s as u64).sum();

			assert_ok!(TransactionStorage::authorize_account(
				RuntimeOrigin::root(),
				who.clone(),
				0,
				total_bytes,
			));

			for size in sizes {
				advance_block();
				assert_ok_ok(construct_and_apply_extrinsic(
					account.pair(),
					RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store {
						data: vec![0u8; size],
					}),
				));
			}
			assert_eq!(
				TransactionStorage::account_authorization_extent(who.clone()),
				AuthorizationExtent {
					transactions: sizes.len() as u32,
					bytes: total_bytes,
					bytes_allowance: total_bytes,
					..Default::default()
				},
			);

			// Re-authorizing inside the unexpired window tops up the allowance; used bytes stay.
			let oversized = max as u64 + 1;
			assert_ok!(TransactionStorage::authorize_account(
				RuntimeOrigin::root(),
				who.clone(),
				0,
				oversized,
			));
			assert_eq!(
				TransactionStorage::account_authorization_extent(who),
				AuthorizationExtent {
					transactions: sizes.len() as u32,
					bytes: total_bytes,
					bytes_allowance: total_bytes + oversized,
					..Default::default()
				},
			);

			advance_block();
			let res = construct_and_apply_extrinsic(
				account.pair(),
				RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store {
					data: vec![0u8; oversized as usize],
				}),
			);
			// An over-max payload may be refused by the pallet's own size check, or earlier by
			// block length/weight limits — both are correct rejections.
			assert!(
				res == Err(pallet_bulletin_transaction_storage::BAD_DATA_SIZE.into()) ||
					res == Err(InvalidTransaction::ExhaustsResources.into()),
				"unexpected error: {res:?}",
			);
		});
	}

	#[test]
	fn transaction_storage_max_throughput_per_block() {
		// Per-block capacity has two independent bounds:
		//   - `MaxBlockTransactions = 512` caps indexed entries (stores and renewals combined);
		//   - `MaxTransactionSize = 2 MiB` caps one blob, and a `store` carries its data inline, so
		//     block length (10 MiB, 90% normal class) admits at most four max-sized stores per
		//     block.
		// Only renewals — small extrinsics referencing already-stored blobs — can fill all
		// 512 slots, re-committing up to 512 × 2 MiB = 1 GiB without carrying the data.
		// Pinned as literals so an upstream change of the pallet defaults is noticed here.
		let max_block_txs: u32 = <Runtime as TxStorageConfig>::MaxBlockTransactions::get();
		assert_eq!(max_block_txs, 512u32);
		let max_size: u32 = <Runtime as TxStorageConfig>::MaxTransactionSize::get();
		assert_eq!(max_size, 2 * 1024 * 1024);

		const NUM_TRANSACTIONS: u32 = 8;
		const TRANSACTION_SIZE: u64 = 1024 * 1024; // 1 MiB

		new_test_ext().execute_with(|| {
			let max_size = max_size as usize;

			advance_block();

			// A maximum-sized transaction (2 MiB) can be stored.
			assert_ok!(TransactionStorage::store(RuntimeOrigin::root(), vec![0u8; max_size]));

			// Data that exceeds MaxTransactionSize is rejected.
			assert_err!(
				TransactionStorage::store(RuntimeOrigin::root(), vec![0u8; max_size + 1]),
				pallet_bulletin_transaction_storage::Error::<Runtime>::BadDataSize,
			);

			// The length bound: stores accumulate in one block until the normal-class block
			// length is spent. 8 one-MiB stores fit; the 9th does not.
			let account = Sr25519Keyring::Alice;
			let who: AccountId = account.to_account_id();
			assert_ok!(TransactionStorage::authorize_account(
				RuntimeOrigin::root(),
				who.clone(),
				0,
				(NUM_TRANSACTIONS as u64 + 1) * TRANSACTION_SIZE,
			));
			advance_block();

			for index in 0..NUM_TRANSACTIONS {
				assert_ok_ok(construct_and_apply_extrinsic(
					account.pair(),
					RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store {
						data: vec![index as u8; TRANSACTION_SIZE as usize],
					}),
				));
			}
			assert_err!(
				construct_and_apply_extrinsic(
					account.pair(),
					RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store {
						data: vec![0u8; TRANSACTION_SIZE as usize],
					}),
				),
				TransactionValidityError::Invalid(InvalidTransaction::ExhaustsResources),
			);

			// Exactly the eight applied stores consumed allowance.
			assert_eq!(
				TransactionStorage::account_authorization_extent(who.clone()),
				AuthorizationExtent {
					transactions: NUM_TRANSACTIONS,
					bytes: NUM_TRANSACTIONS as u64 * TRANSACTION_SIZE,
					bytes_allowance: (NUM_TRANSACTIONS as u64 + 1) * TRANSACTION_SIZE,
					..Default::default()
				},
			);

			// The entry bound: `MaxBlockTransactions` counts entries, not bytes, so only
			// entries that carry no data — renewals, or this test helper — can reach it.
			// At 511 entries one more store fits; at 512 the block is full and validation
			// rejects further stores outright.
			advance_block();
			TransactionStorage::fill_block_transactions(max_block_txs - 1, 1);
			assert!(!TransactionStorage::block_transactions_full());
			assert_ok_ok(construct_and_apply_extrinsic(
				account.pair(),
				RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store {
					data: vec![1u8; 32],
				}),
			));
			assert!(TransactionStorage::block_transactions_full());
			assert_err!(
				construct_and_apply_extrinsic(
					account.pair(),
					RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store {
						data: vec![2u8; 32],
					}),
				),
				TransactionValidityError::Invalid(InvalidTransaction::ExhaustsResources),
			);
		});
	}

	#[test]
	fn authorized_storage_transactions_are_for_free() {
		// Authorized storage calls are feeless: an account with no balance can store.
		new_test_ext().execute_with(|| {
			let account = Sr25519Keyring::Eve;
			let who: AccountId = account.to_account_id();
			let data = vec![0u8; 24];
			let store = RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store {
				data: data.clone(),
			});

			// Unauthorized and unfunded: rejected.
			assert_err!(
				construct_and_apply_extrinsic(account.pair(), store.clone()),
				TransactionValidityError::Invalid(InvalidTransaction::Payment),
			);

			assert_ok!(TransactionStorage::authorize_account(
				RuntimeOrigin::root(),
				who.clone(),
				0,
				48
			));

			// Still unfunded, but now feeless.
			let before = TransactionStorage::account_authorization_extent(who.clone());
			let stored_block = System::block_number();
			assert_ok_ok(construct_and_apply_extrinsic(account.pair(), store));

			let after = TransactionStorage::account_authorization_extent(who.clone());
			assert_eq!(after.transactions, before.transactions + 1);
			assert_eq!(after.bytes, before.bytes + data.len() as u64);
			advance_block();

			// `force_renew` is feeless as well: still no balance, and it commits the entry's
			// size to the permanent counter.
			let before = after;
			assert_ok_ok(construct_and_apply_extrinsic(
				account.pair(),
				RuntimeCall::DataRenewal(RenewalCall::<Runtime>::force_renew {
					entry: TransactionRef::Position { block: stored_block, index: 0 },
				}),
			));
			let after = TransactionStorage::account_authorization_extent(who.clone());
			assert_eq!(after.transactions, before.transactions + 1);
			assert_eq!(after.bytes, before.bytes);
			assert_eq!(
				after.extra.bytes_permanent,
				before.extra.bytes_permanent + data.len() as u64,
			);
			advance_block();

			// `enable_auto_renew` is feeless and pre-pays one cycle at registration: one tx
			// slot plus the entry's size in permanent bytes, like `force_renew`.
			let content_hash = sp_io::hashing::blake2_256(&data);
			let before = after;
			assert_ok_ok(construct_and_apply_extrinsic(
				account.pair(),
				RuntimeCall::DataRenewal(RenewalCall::<Runtime>::enable_auto_renew {
					content_hash,
				}),
			));
			let after = TransactionStorage::account_authorization_extent(who);
			assert_eq!(after.transactions, before.transactions + 1);
			assert_eq!(after.bytes, before.bytes);
			assert_eq!(
				after.extra.bytes_permanent,
				before.extra.bytes_permanent + data.len() as u64,
			);
		});
	}

	#[test]
	fn allowance_based_priority_works() {
		new_test_ext().execute_with(|| {
			let who: AccountId = Sr25519Keyring::Eve.to_account_id();
			// `ValidateAuthorizedCalls` rewrites the origin to `Origin::Authorized` before
			// `AllowanceBasedPriority` runs; build that origin directly here.
			let origin: RuntimeOrigin = TxStorageOrigin::<Runtime>::Authorized {
				who: who.clone(),
				scope: AuthorizationScope::Account(who.clone()),
			}
			.into();
			let store = RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store {
				data: vec![0u8; 1],
			});
			let priority = |origin: RuntimeOrigin, call: &RuntimeCall| -> TransactionPriority {
				let info = call.get_dispatch_info();
				AllowanceBasedPriority::<Runtime>::default()
					.validate(
						origin,
						call,
						&info,
						0,
						(),
						&TxBaseImplication(()),
						TransactionSource::External,
					)
					.expect("validate should not fail")
					.0
					.priority
			};

			// No authorization → no boost.
			assert_eq!(priority(origin.clone(), &store), 0);

			// In-budget → flat boost.
			assert_ok!(TransactionStorage::authorize_account(
				RuntimeOrigin::root(),
				who.clone(),
				10,
				4_000,
			));
			assert_eq!(priority(origin.clone(), &store), ALLOWANCE_PRIORITY_BOOST);

			// Non-store calls get no boost. Must be a call of *this* pallet: anything else exits at
			// `AllowanceBasedPriority`'s `is_sub_type` check and would pass vacuously.
			let other =
				RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::authorize_account {
					who: who.clone(),
					transactions: 1,
					bytes: 1,
				});
			assert_eq!(priority(origin.clone(), &other), 0);

			// Eager-compute: a single tx whose size alone would push the signer over the cap
			// is demoted to no boost on entry, even though nothing is consumed yet.
			let oversized = RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store {
				data: vec![0u8; 4_001],
			});
			assert_eq!(priority(origin.clone(), &oversized), 0);

			// Consume the entire byte allowance -> over budget -> the boost is withdrawn.
			advance_block();
			assert_ok_ok(construct_and_apply_extrinsic(
				Sr25519Keyring::Eve.pair(),
				RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store {
					data: vec![0u8; 4_000],
				}),
			));
			advance_block();
			assert_eq!(priority(origin.clone(), &store), 0);

			// Renewals carry `Origin::Authorized` too, but only stores are boosted.
			let renew = RuntimeCall::DataRenewal(RenewalCall::<Runtime>::force_renew {
				entry: TransactionRef::Position { block: 1, index: 0 },
			});
			assert_eq!(priority(origin, &renew), 0);
		});
	}

	#[test]
	fn preimage_authorized_storage_transactions_work() {
		// Preimage authorization lets *anyone* store that exact content once.
		new_test_ext().execute_with(|| {
			advance_block();
			let account = Sr25519Keyring::Alice;
			fund(&account.to_account_id());

			let data = vec![0u8; 24];
			let content_hash = sp_io::hashing::blake2_256(&data);
			let store = RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store {
				data: data.clone(),
			});

			// No authorization of any kind yet.
			assert_eq!(
				construct_and_apply_extrinsic(account.pair(), store.clone()),
				Err(TransactionValidityError::Invalid(InvalidTransaction::Payment)),
			);

			assert_ok!(TransactionStorage::authorize_preimage(
				RuntimeOrigin::root(),
				content_hash,
				data.len() as u64,
			));

			assert_ok_ok(construct_and_apply_extrinsic(account.pair(), store));

			assert_eq!(
				TransactionStorage::preimage_authorization_extent(content_hash),
				AuthorizationExtent {
					transactions: 1,
					transactions_allowance: 1,
					bytes: 24,
					bytes_allowance: 24,
					..Default::default()
				},
			);
		});
	}

	#[test]
	fn signed_store_prefers_preimage_authorization_over_account() {
		// With both kinds available the preimage grant is spent first, leaving the (scarcer)
		// per-account allowance intact.
		new_test_ext().execute_with(|| {
			advance_block();
			let account = Sr25519Keyring::Alice;
			let who: AccountId = account.to_account_id();
			let data = vec![0u8; 100];
			let content_hash = sp_io::hashing::blake2_256(&data);

			assert_ok!(TransactionStorage::authorize_account(
				RuntimeOrigin::root(),
				who.clone(),
				0,
				500,
			));
			assert_ok!(TransactionStorage::authorize_preimage(
				RuntimeOrigin::root(),
				content_hash,
				data.len() as u64,
			));

			assert_ok_ok(construct_and_apply_extrinsic(
				account.pair(),
				RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store { data }),
			));

			assert_eq!(
				TransactionStorage::preimage_authorization_extent(content_hash),
				AuthorizationExtent {
					transactions: 1,
					transactions_allowance: 1,
					bytes: 100,
					bytes_allowance: 100,
					..Default::default()
				},
				"preimage authorization should be consumed",
			);
			assert_eq!(
				TransactionStorage::account_authorization_extent(who),
				AuthorizationExtent { bytes_allowance: 500, ..Default::default() },
				"account authorization should be untouched",
			);
		});
	}
}

mod wrappers {
	//! Ported from polkadot-bulletin-chain's bulletin-westend tests.
	//! `ValidateAuthorizedCalls` consumes the caller's authorization and refuses to do so for
	//! calls nested inside a dispatcher, so stores are direct-only.

	use super::*;

	#[test]
	fn wrapped_store_requires_authorization() {
		new_test_ext().execute_with(|| {
			advance_block();
			let account = Sr25519Keyring::Alice;
			fund(&account.to_account_id());

			let store = RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store {
				data: vec![42u8; 100],
			});

			// Direct: rejected for missing authorization.
			assert_eq!(
				construct_and_apply_extrinsic(account.pair(), store.clone()),
				Err(TransactionValidityError::Invalid(InvalidTransaction::Payment)),
				"store: direct",
			);

			// Wrapped: rejected because `store` is not allowed inside a dispatcher at all.
			for (wrapped, name) in wrap_call_utility_variants(store) {
				assert_eq!(
					construct_and_apply_extrinsic(account.pair(), wrapped),
					Err(TransactionValidityError::Invalid(InvalidTransaction::Call)),
					"store: via {name}",
				);
			}
		});
	}

	#[test]
	fn wrapped_store_with_cid_config_requires_authorization() {
		new_test_ext().execute_with(|| {
			advance_block();
			let account = Sr25519Keyring::Alice;
			fund(&account.to_account_id());

			let store =
				RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store_with_cid_config {
					cid: CidConfig { codec: RAW_CODEC, hashing: HashingAlgorithm::Blake2b256 },
					data: vec![42u8; 100],
				});

			assert_eq!(
				construct_and_apply_extrinsic(account.pair(), store.clone()),
				Err(TransactionValidityError::Invalid(InvalidTransaction::Payment)),
				"store_with_cid_config: direct",
			);

			for (wrapped, name) in wrap_call_utility_variants(store) {
				assert_eq!(
					construct_and_apply_extrinsic(account.pair(), wrapped),
					Err(TransactionValidityError::Invalid(InvalidTransaction::Call)),
					"store_with_cid_config: via {name}",
				);
			}
		});
	}

	#[test]
	fn authorized_wrapped_store_rejected() {
		// Being authorized does not buy you the wrapper path.
		new_test_ext().execute_with(|| {
			advance_block();
			let account = Sr25519Keyring::Alice;
			let who: AccountId = account.to_account_id();
			let data = vec![42u8; 100];
			fund(&who);

			assert_ok!(TransactionStorage::authorize_account(
				RuntimeOrigin::root(),
				who.clone(),
				0,
				4 * data.len() as u64,
			));

			let store = RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store {
				data: data.clone(),
			});

			// Direct store succeeds.
			assert_ok_ok(construct_and_apply_extrinsic(account.pair(), store.clone()));

			for (wrapped, name) in wrap_call_utility_variants(store) {
				assert_eq!(
					construct_and_apply_extrinsic(account.pair(), wrapped),
					Err(TransactionValidityError::Invalid(InvalidTransaction::Call)),
					"{name}: wrapped store must be rejected",
				);
			}

			// Only the direct store consumed anything.
			assert_eq!(
				TransactionStorage::account_authorization_extent(who),
				AuthorizationExtent {
					transactions: 1,
					bytes: data.len() as u64,
					bytes_allowance: 4 * data.len() as u64,
					..Default::default()
				},
			);
		});
	}

	#[test]
	fn batch_store_with_mixed_preimage_and_account_auth_rejected() {
		// A batch whose two stores are covered by *different* authorization kinds is still a batch.
		new_test_ext().execute_with(|| {
			advance_block();
			let account = Sr25519Keyring::Alice;
			let who: AccountId = account.to_account_id();
			fund(&who);

			let data_a = vec![42u8; 100];
			let data_b = vec![99u8; 200];
			let content_hash_a = sp_io::hashing::blake2_256(&data_a);

			assert_ok!(TransactionStorage::authorize_preimage(
				RuntimeOrigin::root(),
				content_hash_a,
				data_a.len() as u64,
			));
			assert_ok!(TransactionStorage::authorize_account(
				RuntimeOrigin::root(),
				who.clone(),
				0,
				data_b.len() as u64,
			));

			let batch = RuntimeCall::Utility(pallet_utility::Call::batch {
				calls: vec![
					RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store {
						data: data_a,
					}),
					RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store {
						data: data_b,
					}),
				],
			});

			assert_eq!(
				construct_and_apply_extrinsic(account.pair(), batch),
				Err(TransactionValidityError::Invalid(InvalidTransaction::Call)),
			);

			// Rejected before `prepare`, so neither authorization was touched.
			assert_eq!(
				TransactionStorage::preimage_authorization_extent(content_hash_a),
				AuthorizationExtent {
					transactions_allowance: 1,
					bytes_allowance: 100,
					..Default::default()
				},
				"preimage authorization must not be consumed",
			);
			assert_eq!(
				TransactionStorage::account_authorization_extent(who),
				AuthorizationExtent { bytes_allowance: 200, ..Default::default() },
				"account authorization must not be consumed",
			);
		});
	}

	#[test]
	fn mixed_batch_store_and_authorize_rejected() {
		new_test_ext().execute_with(|| {
			advance_block();
			let account = Sr25519Keyring::Alice;
			let who: AccountId = account.to_account_id();
			let data = vec![42u8; 100];
			fund(&who);

			assert_ok!(TransactionStorage::authorize_account(
				RuntimeOrigin::root(),
				who.clone(),
				0,
				data.len() as u64,
			));

			let store = RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store {
				data: data.clone(),
			});
			let authorize =
				RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::authorize_account {
					who: Sr25519Keyring::Bob.to_account_id(),
					transactions: 0,
					bytes: 1024,
				});

			for batch in [
				RuntimeCall::Utility(pallet_utility::Call::batch {
					calls: vec![store.clone(), authorize.clone()],
				}),
				RuntimeCall::Utility(pallet_utility::Call::batch_all {
					calls: vec![store.clone(), authorize.clone()],
				}),
				RuntimeCall::Utility(pallet_utility::Call::force_batch {
					calls: vec![store.clone(), authorize.clone()],
				}),
			] {
				assert_err!(
					construct_and_apply_extrinsic(account.pair(), batch),
					TransactionValidityError::Invalid(InvalidTransaction::Call),
				);
			}

			assert_eq!(
				TransactionStorage::account_authorization_extent(who),
				AuthorizationExtent { bytes_allowance: data.len() as u64, ..Default::default() },
			);
		});
	}

	#[test]
	fn mixed_batch_store_and_non_storage_call_rejected() {
		// Padding the batch with an innocuous call does not launder the `store`.
		new_test_ext().execute_with(|| {
			advance_block();
			let account = Sr25519Keyring::Alice;
			let who: AccountId = account.to_account_id();
			let data = vec![42u8; 100];
			fund(&who);

			assert_ok!(TransactionStorage::authorize_account(
				RuntimeOrigin::root(),
				who.clone(),
				0,
				data.len() as u64,
			));

			let batch = RuntimeCall::Utility(pallet_utility::Call::batch {
				calls: vec![
					RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store {
						data: data.clone(),
					}),
					RuntimeCall::System(frame_system::Call::remark { remark: vec![1, 2, 3] }),
				],
			});

			assert_err!(
				construct_and_apply_extrinsic(account.pair(), batch),
				TransactionValidityError::Invalid(InvalidTransaction::Call),
			);

			assert_eq!(
				TransactionStorage::account_authorization_extent(who),
				AuthorizationExtent { bytes_allowance: data.len() as u64, ..Default::default() },
			);
		});
	}

	#[test]
	fn max_recursion_depth_is_enforced() {
		// Nesting past `MAX_WRAPPER_DEPTH` must not let the inspector run out of budget and
		// silently treat the payload as non-storage.
		new_test_ext().execute_with(|| {
			advance_block();
			let account = Sr25519Keyring::Alice;
			let who: AccountId = account.to_account_id();
			let data = vec![42u8; 100];
			fund(&who);

			assert_ok!(TransactionStorage::authorize_account(
				RuntimeOrigin::root(),
				who,
				0,
				data.len() as u64,
			));

			let mut call =
				RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store { data });
			for _ in 0..=MAX_WRAPPER_DEPTH {
				call = RuntimeCall::Utility(pallet_utility::Call::batch { calls: vec![call] });
			}

			assert_err!(
				construct_and_apply_extrinsic(account.pair(), call),
				TransactionValidityError::Invalid(InvalidTransaction::Call),
			);
		});
	}

	#[test]
	fn wrapped_authorize_account_requires_authorizer_origin() {
		// `authorize_account` *is* permitted inside a dispatcher — but the origin stays `Signed`,
		// so a non-authorizer gains nothing from the wrapper.
		new_test_ext().execute_with(|| {
			advance_block();
			let attacker = Sr25519Keyring::Bob;
			let who: AccountId = attacker.to_account_id();
			fund(&who);

			let call =
				RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::authorize_account {
					who: who.clone(),
					transactions: 0,
					bytes: 1024,
				});

			// Direct: rejected at validation.
			assert_eq!(
				construct_and_apply_extrinsic(attacker.pair(), call.clone()),
				Err(TransactionValidityError::Invalid(InvalidTransaction::BadSigner)),
			);

			// Via batch: the batch validates, but the inner dispatch must fail on origin.
			let batch = RuntimeCall::Utility(pallet_utility::Call::batch { calls: vec![call] });
			let _ = construct_and_apply_extrinsic(attacker.pair(), batch);
			assert_eq!(
				TransactionStorage::account_authorization_extent(who),
				AuthorizationExtent::default(),
				"authorize_account via batch must not succeed for a non-authorizer",
			);
		});
	}

	#[test]
	fn wrapped_authorize_account_succeeds() {
		// The counterpart: a genuine authorizer wrapping `authorize_account` in `batch_all` must
		// still work — the origin must remain `Signed`, not be rewritten to `Authorized`.
		let authorizer = Sr25519Keyring::Alice;
		new_test_ext_with(|genesis| {
			genesis.allowed_authorizers =
				vec![(authorizer.to_account_id(), 1000, 100 * 1024 * 1024)];
		})
		.execute_with(|| {
			advance_block();
			let target: AccountId = Sr25519Keyring::Bob.to_account_id();
			fund(&authorizer.to_account_id());

			let authorize =
				RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::authorize_account {
					who: target.clone(),
					transactions: 10,
					bytes: 10 * 1024,
				});
			let batch =
				RuntimeCall::Utility(pallet_utility::Call::batch_all { calls: vec![authorize] });

			assert_ok_ok(construct_and_apply_extrinsic(authorizer.pair(), batch));
			assert_eq!(
				TransactionStorage::account_authorization_extent(target),
				AuthorizationExtent {
					transactions_allowance: 10,
					bytes_allowance: 10 * 1024,
					..Default::default()
				},
			);

			// And the grant is usable: Bob can now store.
			assert_ok_ok(construct_and_apply_extrinsic(
				Sr25519Keyring::Bob.pair(),
				RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store {
					data: vec![42u8; 100],
				}),
			));
		});
	}
}

mod renewals {
	use super::*;

	/// `force_renew` renews synchronously: it commits `info.size` to both the caller's
	/// `bytes_permanent` and the chain-wide `PermanentStorageUsed`.
	#[test]
	fn force_renew_commits_permanent_bytes() {
		new_test_ext_with(|genesis| genesis.retention_period = 100).execute_with(|| {
			advance_block();
			let account = Sr25519Keyring::Alice;
			let who: AccountId = account.to_account_id();
			let data = vec![42u8; 100];
			fund(&who);

			assert_ok!(TransactionStorage::authorize_account(
				RuntimeOrigin::root(),
				who.clone(),
				0,
				data.len() as u64,
			));
			assert_ok_ok(construct_and_apply_extrinsic(
				account.pair(),
				RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store {
					data: data.clone(),
				}),
			));
			let stored_block = System::block_number();
			advance_block();

			assert_ok_ok(construct_and_apply_extrinsic(
				account.pair(),
				RuntimeCall::DataRenewal(RenewalCall::<Runtime>::force_renew {
					entry: TransactionRef::Position { block: stored_block, index: 0 },
				}),
			));

			let after = TransactionStorage::account_authorization_extent(who);
			assert_eq!(after.extra.bytes_permanent, data.len() as u64);
			assert_eq!(
				pallet_bulletin_data_renewal::PermanentStorageUsed::<Runtime>::get(),
				data.len() as u64,
			);

			// Like `renew`, `force_renew` is refused inside every dispatcher. The account is
			// funded, so fees are not what rejects the (non-feeless) wrappers.
			let force_renew = RuntimeCall::DataRenewal(RenewalCall::<Runtime>::force_renew {
				entry: TransactionRef::Position { block: stored_block, index: 0 },
			});
			for (wrapped, label) in wrap_call_utility_variants(force_renew) {
				assert_eq!(
					construct_and_apply_extrinsic(account.pair(), wrapped),
					Err(TransactionValidityError::Invalid(InvalidTransaction::Call)),
					"force_renew via {label} must be rejected",
				);
			}
		});
	}

	/// One-shot `renew` pre-pays `bytes_permanent`, and is rejected inside a dispatcher. Also
	/// covers `can_renew` / `account_authorization`, composed in `impl_runtime_apis!`.
	#[test]
	fn renew_one_shot_prepays_bytes_permanent() {
		use pallet_bulletin_transaction_storage_runtime_api::runtime_decl_for_bulletin_transaction_storage_api::BulletinTransactionStorageApiV1;

		new_test_ext().execute_with(|| {
			let account = Sr25519Keyring::Bob;
			let who: AccountId = account.to_account_id();
			let data = vec![0u8; 24];
			let content_hash = sp_io::hashing::blake2_256(&data);
			let entry = TransactionRef::ContentHash(content_hash);

			// Two tx slots: one for the `store`, one for the `renew`.
			assert_ok!(TransactionStorage::authorize_account(
				RuntimeOrigin::root(),
				who.clone(),
				2,
				48,
			));
			assert_ok_ok(construct_and_apply_extrinsic(
				account.pair(),
				RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store {
					data: data.clone(),
				}),
			));
			advance_block();

			let before = TransactionStorage::account_authorization_extent(who.clone());
			assert!(Runtime::can_renew(who.clone(), entry.clone()));
			assert_ok_ok(construct_and_apply_extrinsic(
				account.pair(),
				RuntimeCall::DataRenewal(RenewalCall::<Runtime>::renew { entry }),
			));

			let after = TransactionStorage::account_authorization_extent(who.clone());
			assert_eq!(
				after.extra.bytes_permanent,
				before.extra.bytes_permanent + data.len() as u64
			);
			assert_eq!(after.transactions, before.transactions + 1);
			assert_eq!(after.bytes, before.bytes);

			// The runtime API surfaces that same counter as `bytes_permanent_used`.
			let summary =
				Runtime::account_authorization(who.clone()).expect("authorization is active");
			assert_eq!(summary.bytes_permanent_used, after.extra.bytes_permanent);
			assert_eq!(summary.bytes_used, after.bytes);
			assert_eq!(summary.transactions_used, after.transactions);
			assert_eq!(summary.bytes_allowance, after.bytes_allowance);

			// `RenewalLeaves` refuses wrapped leaves, so no dispatcher can consume the allowance.
			// Funded, so `ChargeTransactionPayment` is not what rejects the (non-feeless) wrapper.
			fund(&who);
			let renew = RuntimeCall::DataRenewal(RenewalCall::<Runtime>::renew {
				entry: TransactionRef::ContentHash(content_hash),
			});
			for (wrapped, label) in wrap_call_utility_variants(renew) {
				assert_eq!(
					construct_and_apply_extrinsic(account.pair(), wrapped),
					Err(TransactionValidityError::Invalid(InvalidTransaction::Call)),
					"renew via {label} must be rejected",
				);
			}
		});
	}
}

mod hop_promotion {
	//! HOP promotions: near-expiry pool blobs land on chain as authorized general
	//! transactions — no outer signature, no fees, and no debit of the submitter's allowance.

	use super::*;
	use sp_hop::runtime_decl_for_hop_runtime_api::HopRuntimeApiV1;
	use sp_runtime::{MultiSignature, MultiSigner};

	/// Signer, payload signature and content hash for promoting `data` at `submit_timestamp`.
	fn promotion_parts(
		account: Sr25519Keyring,
		data: &[u8],
		submit_timestamp: u64,
	) -> (MultiSigner, MultiSignature, [u8; 32]) {
		let content_hash = sp_io::hashing::blake2_256(data);
		let payload =
			pallet_bulletin_hop_promotion::signing_payload(&content_hash, submit_timestamp);
		(
			MultiSigner::Sr25519(account.public()),
			MultiSignature::Sr25519(account.pair().sign(&payload)),
			content_hash,
		)
	}

	#[test]
	fn is_promoted_on_chain_works() {
		new_test_ext().execute_with(|| {
			let account = Sr25519Keyring::Alice;
			let who: AccountId = account.to_account_id();
			let data = b"some-promoted-blob".to_vec();
			let content_hash = sp_io::hashing::blake2_256(&data);

			// Nothing stored yet — unknown hash returns false.
			assert!(!HopPromotion::is_promoted_on_chain(content_hash));

			// Authorize Alice and store the blob via `TransactionStorage::store`. Default
			// hashing is `Blake2b256`, which matches `content_hash` above.
			assert_ok!(TransactionStorage::authorize_account(
				RuntimeOrigin::root(),
				who.clone(),
				0,
				data.len() as u64,
			));
			advance_block();
			assert_ok_ok(construct_and_apply_extrinsic(
				account.pair(),
				RuntimeCall::TransactionStorage(TxStorageCall::<Runtime>::store {
					data: data.clone(),
				}),
			));
			// `BlockTransactions` is moved into `Transactions[block]` in `on_finalize`.
			advance_block();

			// Stored hash is now visible; an unrelated hash is not. Both directly and via the
			// runtime API the HOP maintenance task uses.
			assert!(HopPromotion::is_promoted_on_chain(content_hash));
			assert!(!HopPromotion::is_promoted_on_chain([0xAB; 32]));
			assert!(Runtime::is_promoted_on_chain(content_hash));
		});
	}

	/// The full node-side flow: build the extrinsic through the `HopRuntimeApi` composition in
	/// `impl_runtime_apis!` — which also pins the extension values hand-built in
	/// `create_extension` — and apply it through `Executive`.
	#[test]
	fn promotion_via_runtime_api_works() {
		new_test_ext().execute_with(|| {
			let account = Sr25519Keyring::Alice;
			let who: AccountId = account.to_account_id();
			let data = b"hop-promoted-data".to_vec();
			// `pallet_timestamp` reads 0 in a fresh externality; zero skew is within tolerance.
			let submit_timestamp: u64 = 0;
			let (signer, signature, content_hash) =
				promotion_parts(account, &data, submit_timestamp);

			let max_size: u32 = <Runtime as TxStorageConfig>::MaxTransactionSize::get();
			assert_eq!(Runtime::max_promotion_size(), max_size);

			// Not authorized yet: the API says no, and the authorize closure rejects the
			// extrinsic as a bad signer.
			assert!(!Runtime::can_account_promote(who.clone(), data.len() as u32));
			let xt = Runtime::create_promotion_extrinsic(
				data.clone(),
				signer.clone(),
				signature.clone(),
				submit_timestamp,
			);
			assert_eq!(
				Executive::apply_extrinsic(xt),
				Err(TransactionValidityError::Invalid(InvalidTransaction::BadSigner)),
			);

			// An active authorization makes the account eligible.
			assert_ok!(TransactionStorage::authorize_account(
				RuntimeOrigin::root(),
				who.clone(),
				0,
				data.len() as u64,
			));
			advance_block();
			assert!(Runtime::can_account_promote(who.clone(), data.len() as u32));
			let before = TransactionStorage::account_authorization_extent(who.clone());

			// A signature over different data is refused before anything is stored.
			let (_, wrong_signature, _) =
				promotion_parts(account, b"different-data", submit_timestamp);
			let xt = Runtime::create_promotion_extrinsic(
				data.clone(),
				signer.clone(),
				wrong_signature,
				submit_timestamp,
			);
			assert_eq!(
				Executive::apply_extrinsic(xt),
				Err(TransactionValidityError::Invalid(InvalidTransaction::BadProof)),
			);

			// A submit timestamp outside `SubmitTimestampTolerance` (48h) is stale.
			let late = 49 * 60 * 60 * 1000;
			let (_, late_signature, _) = promotion_parts(account, &data, late);
			let xt = Runtime::create_promotion_extrinsic(
				data.clone(),
				signer.clone(),
				late_signature,
				late,
			);
			assert_eq!(
				Executive::apply_extrinsic(xt),
				Err(TransactionValidityError::Invalid(InvalidTransaction::Stale)),
			);

			// The genuine promotion is accepted and dispatches.
			let xt = Runtime::create_promotion_extrinsic(data, signer, signature, submit_timestamp);
			assert_ok_ok(Executive::apply_extrinsic(xt));

			// Promotion consumes none of the account's allowance: the authorization only
			// gates eligibility.
			assert_eq!(TransactionStorage::account_authorization_extent(who), before);

			// After block flush, the promoted hash is visible on-chain.
			advance_block();
			assert!(HopPromotion::is_promoted_on_chain(content_hash));
		});
	}
}
