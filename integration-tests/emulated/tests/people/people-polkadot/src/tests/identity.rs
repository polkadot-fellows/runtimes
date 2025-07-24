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

//! Tests related to cross-chain identity operations.

use crate::*;
use codec::Encode;
use emulated_integration_tests_common::accounts::ALICE;
use frame_support::BoundedVec;
use pallet_identity::Data;
use people_polkadot_runtime::people::{IdentityField, IdentityInfo};
use xcm::latest::AssetTransferFilter;

#[test]
fn set_identity_cross_chain() {
	type Identity = <PeoplePolkadot as PeoplePolkadotPallet>::Identity;

	let asset_hub_polkadot_alice = AssetHubPolkadot::account_id_of(ALICE);
	let people_polkadot_alice = PeoplePolkadot::account_id_of(ALICE);
	AssetHubPolkadot::fund_accounts(vec![(asset_hub_polkadot_alice.clone(), POLKADOT_ED * 10000)]);
	PeoplePolkadot::fund_accounts(vec![(people_polkadot_alice.clone(), POLKADOT_ED * 10000)]);

	PeoplePolkadot::execute_with(|| {
		// No identity for Alice
		assert!(!Identity::has_identity(&people_polkadot_alice, IdentityField::Email as u64));
	});

	let destination = AssetHubPolkadot::sibling_location_of(PeoplePolkadot::para_id());
	let total_fees: Asset = (Location::parent(), POLKADOT_ED * 1000).into();
	let fees: Asset = (Location::parent(), POLKADOT_ED * 500).into();
	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

		let identity_info = IdentityInfo {
			email: Data::Raw(b"test@test.io".to_vec().try_into().unwrap()),
			..Default::default()
		};
		// Set Alice identity on People from Alice on AH
		let set_identity_call =
			<PeoplePolkadot as Chain>::RuntimeCall::Identity(pallet_identity::Call::<
				<PeoplePolkadot as Chain>::Runtime,
			>::set_identity {
				info: bx!(identity_info),
			});
		let xcm_message = Xcm::<()>(vec![
			WithdrawAsset(total_fees.into()),
			PayFees { asset: fees.clone() },
			InitiateTransfer {
				destination,
				remote_fees: Some(AssetTransferFilter::Teleport(fees.clone().into())),
				preserve_origin: true,
				assets: BoundedVec::new(),
				remote_xcm: Xcm(vec![
					// try to alias into `Alice` account local to People chain
					AliasOrigin(people_polkadot_alice.clone().into()),
					// set identity for the local Alice account
					Transact {
						origin_kind: OriginKind::SovereignAccount,
						call: set_identity_call.encode().into(),
						fallback_max_weight: None,
					},
					ExpectTransactStatus(MaybeErrorCode::Success),
					RefundSurplus,
					DepositAsset {
						assets: Wild(AllCounted(1)),
						beneficiary: people_polkadot_alice.clone().into(),
					},
				]),
			},
			RefundSurplus,
			DepositAsset {
				assets: Wild(AllCounted(1)),
				beneficiary: asset_hub_polkadot_alice.clone().into(),
			},
		]);

		let signed_origin =
			<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(asset_hub_polkadot_alice);
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::PolkadotXcm::execute(
			signed_origin,
			bx!(xcm::VersionedXcm::from(xcm_message.into())),
			Weight::MAX
		));
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::PolkadotXcm(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	PeoplePolkadot::execute_with(|| {
		// Verify Alice on People now has identity
		assert!(Identity::has_identity(&people_polkadot_alice, IdentityField::Email as u64));
	});
}
