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

//! Lite-person registration paths and the free (fee-less but rate-limited) lite transactions.

use super::*;
use crate::{
	individuality::RestrictedEntity,
	parameters::{dynamic_params::lite_personhood, LitePersonRegistrationFee, RuntimeParameters},
};
use frame_support::{assert_noop, assert_ok};

fn fee_registration_payload(
	lite_account: &AccountId32,
	seed: [u8; 32],
) -> (<Crypto as GenerateVerifiable>::Member, <Crypto as GenerateVerifiable>::Signature) {
	let ring_secret = Crypto::new_secret(seed);
	let ring_member = Crypto::member_from_secret(&ring_secret);
	let message = lite_account.using_encoded(|account_bytes| {
		ring_member.using_encoded(|ring_bytes| {
			[&indiv_pallet_people_lite::MSG_PREFIX[..], account_bytes, ring_bytes].concat()
		})
	});
	let proof = Crypto::sign(&ring_secret, &message)
		.expect("ring key can sign the fee registration payload");
	(ring_member, proof)
}

fn consumer_registration_params(
	lite_account: &AccountId32,
	lite_pair: &sr25519::Pair,
	verifier: &AccountId32,
) -> indiv_pallet_people_lite::types::LiteConsumerRegistrationParams<AccountId32, MultiSignature> {
	let mut params = indiv_pallet_people_lite::types::LiteConsumerRegistrationParams {
		signature: MultiSignature::from(lite_pair.sign(b"placeholder")),
		account: lite_account.clone(),
		identifier_key: [7; 65],
		username: indiv_support::traits::Username::try_from(b"liteperson.12".to_vec())
			.expect("valid username"),
		reserved_username: None,
	};
	params.signature = MultiSignature::from(lite_pair.sign(&params.signing_payload(verifier)));
	params
}

fn register_lite_person(
	attester: &AccountId32,
	lite_pair: &sr25519::Pair,
) -> <Crypto as verifiable::GenerateVerifiable>::Secret {
	let lite_account = pair_to_account_id(lite_pair);
	let ring_secret = Crypto::new_secret([42u8; 32]);
	let ring_member = Crypto::member_from_secret(&ring_secret);

	let msg = lite_account.using_encoded(|account_bytes| {
		ring_member.using_encoded(|ring_bytes| {
			[&indiv_pallet_people_lite::MSG_PREFIX[..], account_bytes, ring_bytes].concat()
		})
	});

	let candidate_signature = MultiSignature::from(lite_pair.sign(&msg));
	let proof =
		Crypto::sign(&ring_secret, &msg).expect("ring key can sign the attestation payload");

	assert_ok!(indiv_pallet_people_lite::Pallet::<Runtime>::attest(
		RuntimeOrigin::signed(attester.clone()),
		lite_account,
		candidate_signature,
		ring_member,
		proof,
		None,
	));

	ring_secret
}

#[test]
fn fee_registration_transfers_native_balance_to_the_pallet_pot() {
	new_test_ext().execute_with(|| {
		let lite_pair = sr25519::Pair::from_seed(&[78u8; 32]);
		let lite_account = pair_to_account_id(&lite_pair);
		let (ring_member, proof) = fee_registration_payload(&lite_account, [43u8; 32]);
		let fee = LitePersonRegistrationFee::get();
		let required_balance = fee.saturating_add(Balances::minimum_balance());
		let pot = indiv_pallet_people_lite::Pallet::<Runtime>::lite_people_pot_id();

		Balances::set_balance(&lite_account, required_balance);
		assert_eq!(Balances::free_balance(&pot), 0);

		assert_ok!(indiv_pallet_people_lite::Pallet::<Runtime>::register_with_fee(
			RuntimeOrigin::signed(lite_account.clone()),
			ring_member,
			proof,
			None,
		));

		assert!(matches!(
			indiv_pallet_people_lite::LitePeople::<Runtime>::get(&lite_account)
				.expect("candidate is registered")
				.method,
			indiv_pallet_people_lite::types::RecognitionMethod::Fee,
		));
		assert_eq!(Balances::free_balance(&lite_account), Balances::minimum_balance());
		assert_eq!(Balances::free_balance(&pot), fee);
		assert_eq!(Balances::total_balance_on_hold(&lite_account), 0);
	});
}

#[test]
fn root_parameter_update_changes_the_next_registration_fee() {
	new_test_ext().execute_with(|| {
		let updated_fee = LitePersonRegistrationFee::get().saturating_mul(2);
		assert_ok!(Parameters::set_parameter(
			RuntimeOrigin::root(),
			RuntimeParameters::LitePersonhood(
				(lite_personhood::RegistrationFee, updated_fee).into()
			),
		));

		let lite_pair = sr25519::Pair::from_seed(&[79u8; 32]);
		let lite_account = pair_to_account_id(&lite_pair);
		let (ring_member, proof) = fee_registration_payload(&lite_account, [44u8; 32]);
		let pot = indiv_pallet_people_lite::Pallet::<Runtime>::lite_people_pot_id();

		Balances::set_balance(
			&lite_account,
			updated_fee.saturating_add(Balances::minimum_balance()),
		);
		assert_ok!(indiv_pallet_people_lite::Pallet::<Runtime>::register_with_fee(
			RuntimeOrigin::signed(lite_account.clone()),
			ring_member,
			proof,
			None,
		));
		assert_eq!(Balances::free_balance(&lite_account), Balances::minimum_balance());
		assert_eq!(Balances::free_balance(&pot), updated_fee);
	});
}

#[test]
fn root_parameter_update_below_the_existential_deposit_uses_the_minimum_fee() {
	new_test_ext().execute_with(|| {
		let minimum_fee = ExistentialDeposit::get();
		assert!(minimum_fee > 0);
		let updated_fee = minimum_fee.saturating_sub(1);
		assert_ok!(Parameters::set_parameter(
			RuntimeOrigin::root(),
			RuntimeParameters::LitePersonhood(
				(lite_personhood::RegistrationFee, updated_fee).into()
			),
		));

		assert_eq!(lite_personhood::RegistrationFee::get(), updated_fee);
		assert_eq!(LitePersonRegistrationFee::get(), minimum_fee);
	});
}

#[test]
fn fee_registration_verifies_consumer_signature_with_the_candidate_as_verifier() {
	new_test_ext().execute_with(|| {
		let lite_pair = sr25519::Pair::from_seed(&[80u8; 32]);
		let lite_account = pair_to_account_id(&lite_pair);
		let (ring_member, proof) = fee_registration_payload(&lite_account, [45u8; 32]);
		let fee = LitePersonRegistrationFee::get();

		Balances::set_balance(&lite_account, fee.saturating_add(Balances::minimum_balance()));
		assert_ok!(indiv_pallet_people_lite::Pallet::<Runtime>::register_with_fee(
			RuntimeOrigin::signed(lite_account.clone()),
			ring_member,
			proof,
			Some(consumer_registration_params(&lite_account, &lite_pair, &lite_account)),
		));
		assert!(indiv_pallet_resources::Consumers::<Runtime>::contains_key(&lite_account));
	});
}

#[test]
fn fee_registration_rejects_consumer_signature_for_a_different_verifier() {
	new_test_ext().execute_with(|| {
		let lite_pair = sr25519::Pair::from_seed(&[81u8; 32]);
		let lite_account = pair_to_account_id(&lite_pair);
		let wrong_verifier = pair_to_account_id(&sr25519::Pair::from_seed(&[82u8; 32]));
		let (ring_member, proof) = fee_registration_payload(&lite_account, [46u8; 32]);
		let fee = LitePersonRegistrationFee::get();
		let required_balance = fee.saturating_add(Balances::minimum_balance());
		let pot = indiv_pallet_people_lite::Pallet::<Runtime>::lite_people_pot_id();

		Balances::set_balance(&lite_account, required_balance);
		assert_noop!(
			indiv_pallet_people_lite::Pallet::<Runtime>::register_with_fee(
				RuntimeOrigin::signed(lite_account.clone()),
				ring_member,
				proof,
				Some(consumer_registration_params(&lite_account, &lite_pair, &wrong_verifier)),
			),
			indiv_pallet_people_lite::Error::<Runtime>::InvalidAttestationSignature,
		);
		assert!(!indiv_pallet_people_lite::LitePeople::<Runtime>::contains_key(&lite_account));
		assert_eq!(Balances::free_balance(&lite_account), required_balance);
		assert_eq!(Balances::free_balance(&pot), 0);
	});
}

fn build_lite_person_signed_ext(who: &sr25519::Pair, call: RuntimeCall) -> UncheckedExtrinsic {
	build_people_lite_auth_ext(
		who,
		indiv_pallet_people_lite::PeopleLiteAuthData::AsLitePerson,
		call,
	)
}

#[test]
fn lite_people_free_transaction_updates_usage() {
	new_test_ext().execute_with(|| {
		let attester = Sr25519Keyring::Bob.to_account_id();
		assert_ok!(indiv_pallet_people_lite::Pallet::<Runtime>::increase_attestation_allowance(
			RuntimeOrigin::root(),
			attester.clone(),
			1,
		));

		let lite_pair = sr25519::Pair::from_seed(&[77u8; 32]);
		let lite_account = pair_to_account_id(&lite_pair);
		assert_eq!(Balances::free_balance(lite_account.clone()), 0);

		register_lite_person(&attester, &lite_pair);
		assert!(
			indiv_pallet_people_lite::LitePeople::<Runtime>::contains_key(&lite_account),
			"lite person must be registered before running the free transaction"
		);

		let nested_call = RuntimeCall::System(frame_system::Call::<Runtime>::remark_with_event {
			remark: b"lite people free tx".to_vec(),
		});

		let outer_call = RuntimeCall::PeopleLite(
			indiv_pallet_people_lite::Call::<Runtime>::dispatch_as_signer {
				call: Box::new(nested_call),
			},
		);

		let entity = RestrictedEntity::LitePerson(lite_account.clone());
		assert!(
			indiv_pallet_origin_restriction::Usages::<Runtime>::get(entity.clone()).is_none(),
			"usage starts empty for lite person"
		);

		let uxt = build_lite_person_signed_ext(&lite_pair, outer_call);
		Executive::apply_extrinsic(uxt)
			.expect("lite person transaction is valid")
			.expect("lite person dispatch succeeds");
		assert_eq!(Balances::free_balance(lite_account.clone()), 0);

		let remarked = frame_system::Pallet::<Runtime>::events().iter().any(|rec| {
			matches!(
				rec.event,
				RuntimeEvent::System(frame_system::Event::Remarked { ref sender, .. })
					if *sender == lite_account
			)
		});
		assert!(remarked, "lite person remark_with_event must emit the System::Remarked event");

		let usage = indiv_pallet_origin_restriction::Usages::<Runtime>::get(entity)
			.expect("usage should be tracked for lite person");
		assert!(
			usage.used > 0,
			"usage must be accounted even when the transaction is otherwise free"
		);
		assert_eq!(
			usage.at_block,
			RelaychainDataProvider::<Runtime>::current_block_number(),
			"usage must be stamped with the relay chain block number, not the parachain one"
		);
	});
}
