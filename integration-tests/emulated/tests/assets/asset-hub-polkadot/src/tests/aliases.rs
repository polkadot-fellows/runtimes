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

//! Tests related to XCM aliasing.

use crate::*;
use emulated_integration_tests_common::{macros::AccountId, test_cross_chain_alias};
use frame_support::traits::{fungible::Inspect, ContainsPair};
use xcm::latest::Junctions::*;
use xcm_executor::traits::ConvertLocation;
use AssetHubPolkadotXcmConfig as XcmConfig;

const ALLOWED: bool = true;
const DENIED: bool = false;

const TELEPORT_FEES: bool = true;
const RESERVE_TRANSFER_FEES: bool = false;

const ETHEREUM_BOB: [u8; 20] = hex_literal::hex!("11b0b11000011b0b11000011b0b11000011b0b11");

#[test]
fn account_on_sibling_chain_cannot_alias_into_same_local_account() {
	// origin and target are the same account on different chains
	let origin: AccountId = [1; 32].into();
	let target = origin.clone();
	let fees = POLKADOT_ED * 10;

	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		Location::parent(),
		origin.clone(),
		fees * 10,
	);

	// On Asset Hub we don't want to support aliasing from other chains:
	// - there is no real world demand for it, the direction is usually reversed, users already have
	//   accounts on AH and want to use them cross-chain on other chains,
	// - without real world demand, it's better to keep AH permissions as tight as possible.
	// Aliasing same account doesn't work on AH.
	test_cross_chain_alias!(
		vec![
			// between BH and AH: denied
			(BridgeHubPolkadot, AssetHubPolkadot, TELEPORT_FEES, DENIED),
			// between Collectives and AH: denied
			(CollectivesPolkadot, AssetHubPolkadot, TELEPORT_FEES, DENIED),
			// between Coretime and AH: denied
			(CoretimePolkadot, AssetHubPolkadot, TELEPORT_FEES, DENIED),
			// between People and AH: denied
			(PeoplePolkadot, AssetHubPolkadot, TELEPORT_FEES, DENIED),
			// between Penpal and AH: denied
			(PenpalA, AssetHubPolkadot, RESERVE_TRANSFER_FEES, DENIED)
		],
		origin,
		target,
		fees
	);
}

#[test]
fn account_on_sibling_chain_cannot_alias_into_different_local_account() {
	// origin and target are different accounts on different chains
	let origin: AccountId = [1; 32].into();
	let target: AccountId = [2; 32].into();
	let fees = POLKADOT_ED * 10;

	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		Location::parent(),
		origin.clone(),
		fees * 10,
	);

	// Aliasing different account on different chains
	test_cross_chain_alias!(
		vec![
			// between BH and AH: denied
			(BridgeHubPolkadot, AssetHubPolkadot, TELEPORT_FEES, DENIED),
			// between Collectives and AH: denied
			(CollectivesPolkadot, AssetHubPolkadot, TELEPORT_FEES, DENIED),
			// between Coretime and AH: denied
			(CoretimePolkadot, AssetHubPolkadot, TELEPORT_FEES, DENIED),
			// between People and AH: denied
			(PeoplePolkadot, AssetHubPolkadot, TELEPORT_FEES, DENIED),
			// between Penpal and AH: denied
			(PenpalA, AssetHubPolkadot, RESERVE_TRANSFER_FEES, DENIED)
		],
		origin,
		target,
		fees
	);
}

#[test]
fn authorized_cross_chain_aliases() {
	// origin and target are different accounts on different chains
	let origin: AccountId = [100; 32].into();
	let bad_origin: AccountId = [150; 32].into();
	let target: AccountId = [200; 32].into();
	let fees = POLKADOT_ED * 10;

	let pal_admin = <PenpalB as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get());
	PenpalB::mint_foreign_asset(pal_admin.clone(), Location::parent(), origin.clone(), fees * 10);
	PenpalB::mint_foreign_asset(pal_admin, Location::parent(), bad_origin.clone(), fees * 10);
	AssetHubPolkadot::fund_accounts(vec![(target.clone(), fees * 10)]);

	// let's authorize `origin` on Penpal to alias `target` on AssetHub
	AssetHubPolkadot::execute_with(|| {
		let penpal_origin = Location::new(
			1,
			X2([
				Parachain(PenpalB::para_id().into()),
				AccountId32Junction { network: Some(Polkadot), id: origin.clone().into() },
			]
			.into()),
		);
		// `target` adds `penpal_origin` as authorized alias
		assert_ok!(
			<AssetHubPolkadot as AssetHubPolkadotPallet>::PolkadotXcm::add_authorized_alias(
				<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(target.clone()),
				Box::new(penpal_origin.into()),
				None
			)
		);
	});
	// Verify that unauthorized `bad_origin` cannot alias into `target`, from any chain.
	test_cross_chain_alias!(
		vec![
			// between BH and AssetHub: denied
			(BridgeHubPolkadot, AssetHubPolkadot, TELEPORT_FEES, DENIED),
			// between Collectives and AssetHub: denied
			(CollectivesPolkadot, AssetHubPolkadot, TELEPORT_FEES, DENIED),
			// between People and AssetHub: denied
			(PeoplePolkadot, AssetHubPolkadot, TELEPORT_FEES, DENIED),
			// between Penpal and AssetHub: denied
			(PenpalB, AssetHubPolkadot, RESERVE_TRANSFER_FEES, DENIED)
		],
		bad_origin,
		target,
		fees
	);
	// Verify that only authorized `penpal::origin` can alias into `target`, while `origin` on other
	// chains cannot.
	test_cross_chain_alias!(
		vec![
			// between BH and AssetHub: denied
			(BridgeHubPolkadot, AssetHubPolkadot, TELEPORT_FEES, DENIED),
			// between Collectives and AssetHub: denied
			(CollectivesPolkadot, AssetHubPolkadot, TELEPORT_FEES, DENIED),
			// between People and AssetHub: denied
			(PeoplePolkadot, AssetHubPolkadot, TELEPORT_FEES, DENIED),
			// between Penpal and AssetHub: allowed
			(PenpalB, AssetHubPolkadot, RESERVE_TRANSFER_FEES, ALLOWED)
		],
		origin,
		target,
		fees
	);
	// remove authorization for `origin` on Penpal to alias `target` on AssetHub
	AssetHubPolkadot::execute_with(|| {
		// `target` removes all authorized aliases
		assert_ok!(
			<AssetHubPolkadot as AssetHubPolkadotPallet>::PolkadotXcm::remove_all_authorized_aliases(
				<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(target.clone())
			)
		);
	});
	// Verify `penpal::origin` can no longer alias into `target` on AssetHub.
	test_cross_chain_alias!(
		vec![(PenpalB, AssetHubPolkadot, RESERVE_TRANSFER_FEES, DENIED)],
		origin,
		target,
		fees
	);
}

#[test]
fn aliasing_child_locations() {
	AssetHubPolkadot::execute_with(|| {
		// Allows aliasing descendant of origin.
		let origin = Location::new(1, X1([PalletInstance(8)].into()));
		let target = Location::new(1, X2([PalletInstance(8), GeneralIndex(9)].into()));
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let origin = Location::new(1, X1([Parachain(8)].into()));
		let target = Location::new(
			1,
			X2([Parachain(8), AccountId32Junction { network: None, id: [1u8; 32] }].into()),
		);
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let origin = Location::new(1, X1([Parachain(8)].into()));
		let target =
			Location::new(1, X3([Parachain(8), PalletInstance(8), GeneralIndex(9)].into()));
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));

		// Does not allow if not descendant.
		let origin = Location::new(1, X1([PalletInstance(8)].into()));
		let target = Location::new(0, X2([PalletInstance(8), GeneralIndex(9)].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let origin = Location::new(1, X1([Parachain(8)].into()));
		let target = Location::new(
			0,
			X2([Parachain(8), AccountId32Junction { network: None, id: [1u8; 32] }].into()),
		);
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let origin = Location::new(1, X1([Parachain(8)].into()));
		let target =
			Location::new(0, X1([AccountId32Junction { network: None, id: [1u8; 32] }].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let origin =
			Location::new(1, X1([AccountId32Junction { network: None, id: [1u8; 32] }].into()));
		let target =
			Location::new(0, X1([AccountId32Junction { network: None, id: [1u8; 32] }].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
	});
}

#[test]
fn local_asset_hub_root_cannot_alias_external_locations() {
	AssetHubPolkadot::execute_with(|| {
		// Does not allow local/AH root to alias other locations.
		let origin = Location::new(1, X1([Parachain(1000)].into()));

		let target = Location::new(1, X1([Parachain(2000)].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let target =
			Location::new(1, X1([AccountId32Junction { network: None, id: [1u8; 32] }].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let target = Location::new(
			1,
			X2([Parachain(8), AccountId32Junction { network: None, id: [1u8; 32] }].into()),
		);
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let target =
			Location::new(1, X3([Parachain(42), PalletInstance(8), GeneralIndex(9)].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let target = Location::new(2, X1([GlobalConsensus(Ethereum { chain_id: 1 })].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let target = Location::new(2, X2([GlobalConsensus(Kusama), Parachain(1000)].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let target = Location::new(0, X2([PalletInstance(8), GeneralIndex(9)].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));

		// Other AH locations cannot alias anything.
		let origin = Location::new(1, X2([Parachain(1000), GeneralIndex(9)].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let origin = Location::new(1, X2([Parachain(1000), PalletInstance(9)].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let origin = Location::new(
			1,
			X2([Parachain(1000), AccountId32Junction { network: None, id: [1u8; 32] }].into()),
		);
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));

		// Other root locations cannot alias anything.
		let origin = Location::new(1, Here);
		let target = Location::new(2, X1([GlobalConsensus(Ethereum { chain_id: 1 })].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let target = Location::new(2, X2([GlobalConsensus(Kusama), Parachain(1000)].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let target = Location::new(0, X2([PalletInstance(8), GeneralIndex(9)].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));

		let origin = Location::new(0, Here);
		let target = Location::new(1, X1([Parachain(2000)].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let origin = Location::new(1, X1([Parachain(1001)].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let origin = Location::new(1, X1([Parachain(1002)].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
	});
}
#[test]
fn asset_hub_kusama_root_aliases_into_kusama_origins() {
	AssetHubPolkadot::execute_with(|| {
		let origin = Location::new(2, X2([GlobalConsensus(Kusama), Parachain(1000)].into()));

		let target = Location::new(2, X2([GlobalConsensus(Kusama), Parachain(2000)].into()));
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));

		let target = Location::new(
			2,
			X3([
				GlobalConsensus(Kusama),
				Parachain(2000),
				AccountId32Junction { network: None, id: AssetHubPolkadotSender::get().into() },
			]
			.into()),
		);
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));

		let target = Location::new(
			2,
			X4([GlobalConsensus(Kusama), Parachain(2000), PalletInstance(8), GeneralIndex(9)]
				.into()),
		);
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
	});
}

#[test]
fn asset_hub_kusama_root_does_not_alias_into_ethereum_origins() {
	AssetHubPolkadot::execute_with(|| {
		let origin = Location::new(2, X2([GlobalConsensus(Kusama), Parachain(1000)].into()));

		let target = Location::new(2, X1([GlobalConsensus(Ethereum { chain_id: 1 })].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));

		let target = Location::new(
			2,
			X2([
				GlobalConsensus(Ethereum { chain_id: 1 }),
				AccountKey20 { network: None, key: ETHEREUM_BOB },
			]
			.into()),
		);
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
	});
}

#[test]
fn asset_hub_kusama_root_does_not_alias_into_asset_hub_polkadot_origins() {
	AssetHubPolkadot::execute_with(|| {
		let origin = Location::new(2, X2([GlobalConsensus(Kusama), Parachain(1000)].into()));

		let target = Location::new(2, X1([GlobalConsensus(Polkadot)].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));

		let target = Location::new(2, X2([GlobalConsensus(Polkadot), Parachain(2000)].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));

		let target = Location::new(
			2,
			X3([
				GlobalConsensus(Polkadot),
				Parachain(2000),
				AccountId32Junction { network: None, id: AssetHubPolkadotSender::get().into() },
			]
			.into()),
		);
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));

		let target = Location::new(
			2,
			X4([GlobalConsensus(Polkadot), Parachain(2000), PalletInstance(8), GeneralIndex(9)]
				.into()),
		);
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
	});
}

#[test]
fn fellowship_architects_aliases_into_fellowship_treasury_and_salary() {
	AssetHubPolkadot::execute_with(|| {
		let collectives_para_id = CollectivesPolkadot::para_id().into();

		// The Architects origin from Collectives — this is the origin that the
		// Architects track produces via `ArchitectsToLocation`:
		// Technical body (Fellowship) refined to rank 4 (Architects).
		let architects_origin = Location::new(
			1,
			X3([
				Parachain(collectives_para_id),
				Plurality { id: BodyId::Technical, part: BodyPart::Voice },
				GeneralIndex(collectives_polkadot_runtime_constants::ARCHITECTS_RANK),
			]
			.into()),
		);

		// Fellowship Treasury pallet on Collectives.
		let fellowship_treasury_target = Location::new(
			1,
			X2([
				Parachain(collectives_para_id),
				PalletInstance(
					collectives_polkadot_runtime_constants::FELLOWSHIP_TREASURY_PALLET_INDEX,
				),
			]
			.into()),
		);

		// Fellowship Salary pallet on Collectives.
		let fellowship_salary_target = Location::new(
			1,
			X2([
				Parachain(collectives_para_id),
				PalletInstance(
					collectives_polkadot_runtime_constants::FELLOWSHIP_SALARY_PALLET_INDEX,
				),
			]
			.into()),
		);

		// Architects origin can alias into Fellowship Treasury.
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(
			&architects_origin,
			&fellowship_treasury_target,
		));

		// Architects origin can alias into Fellowship Salary.
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(
			&architects_origin,
			&fellowship_salary_target,
		));
	});
}

#[test]
fn non_architects_cannot_alias_into_fellowship_treasury_or_salary() {
	AssetHubPolkadot::execute_with(|| {
		let collectives_para_id = CollectivesPolkadot::para_id().into();

		let fellowship_treasury_target = Location::new(
			1,
			X2([
				Parachain(collectives_para_id),
				PalletInstance(
					collectives_polkadot_runtime_constants::FELLOWSHIP_TREASURY_PALLET_INDEX,
				),
			]
			.into()),
		);

		let fellowship_salary_target = Location::new(
			1,
			X2([
				Parachain(collectives_para_id),
				PalletInstance(
					collectives_polkadot_runtime_constants::FELLOWSHIP_SALARY_PALLET_INDEX,
				),
			]
			.into()),
		);

		// Technical (Fellows) plurality without GeneralIndex cannot alias into Fellowship
		// Treasury or Salary.
		let fellows_origin = Location::new(
			1,
			X2([
				Parachain(collectives_para_id),
				Plurality { id: BodyId::Technical, part: BodyPart::Voice },
			]
			.into()),
		);
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(
			&fellows_origin,
			&fellowship_treasury_target,
		));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(
			&fellows_origin,
			&fellowship_salary_target,
		));

		// Wrong GeneralIndex (rank 3 instead of 4) cannot alias into Fellowship Treasury or
		// Salary.
		let wrong_rank_origin = Location::new(
			1,
			X3([
				Parachain(collectives_para_id),
				Plurality { id: BodyId::Technical, part: BodyPart::Voice },
				GeneralIndex(3),
			]
			.into()),
		);
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(
			&wrong_rank_origin,
			&fellowship_treasury_target,
		));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(
			&wrong_rank_origin,
			&fellowship_salary_target,
		));

		// A regular account on Collectives cannot alias into Fellowship Treasury.
		let account_origin = Location::new(
			1,
			X2([
				Parachain(collectives_para_id),
				AccountId32Junction { network: None, id: [1u8; 32] },
			]
			.into()),
		);
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(
			&account_origin,
			&fellowship_treasury_target,
		));

		// Architects origin from a non-Collectives parachain cannot alias.
		let wrong_chain_origin = Location::new(
			1,
			X3([
				Parachain(9999),
				Plurality { id: BodyId::Technical, part: BodyPart::Voice },
				GeneralIndex(collectives_polkadot_runtime_constants::ARCHITECTS_RANK),
			]
			.into()),
		);
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(
			&wrong_chain_origin,
			&fellowship_treasury_target,
		));

		// Architects origin cannot alias into an unrelated pallet.
		let architects_origin = Location::new(
			1,
			X3([
				Parachain(collectives_para_id),
				Plurality { id: BodyId::Technical, part: BodyPart::Voice },
				GeneralIndex(collectives_polkadot_runtime_constants::ARCHITECTS_RANK),
			]
			.into()),
		);
		let unrelated_pallet_target =
			Location::new(1, X2([Parachain(collectives_para_id), PalletInstance(99)].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(
			&architects_origin,
			&unrelated_pallet_target,
		));
	});
}

/// Helper: sends an XCM from Collectives with the Architects origin to Asset Hub.
/// The message aliases into the Fellowship pallet at `pallet_index`, withdraws DOT from the
/// pallet's sovereign account, and deposits it to a beneficiary.
///
/// This uses `PolkadotXcm::send` on the Collectives chain with the Architects origin, which
/// auto-prepends `DescendOrigin` via `ArchitectsToLocation`. Asset Hub then processes the
/// message: the origin descends to the Architects location, aliases into the target pallet,
/// and performs the transfer.
///
/// Only works for the `FellowshipTreasury` and `FellowshipSalary` pallet instances.
fn architects_alias_into_fellowship_pallet(pallet_index: u8) {
	let collectives_para_id: u32 = CollectivesPolkadot::para_id().into();
	let amount: Balance = POLKADOT_ED * 100;

	// The Fellowship pallet location (treasury or salary) on Collectives, as seen from AH.
	let pallet_location =
		Location::new(1, [Parachain(collectives_para_id), PalletInstance(pallet_index)]);

	// Compute the sovereign account for this pallet location on AH.
	let pallet_sovereign =
		asset_hub_polkadot_runtime::xcm_config::LocationToAccountId::convert_location(
			&pallet_location,
		)
		.expect("Failed to convert pallet location to account");

	let beneficiary: AccountId = [42u8; 32].into();

	// Fund the pallet's sovereign account on AH.
	AssetHubPolkadot::fund_accounts(vec![(pallet_sovereign.clone(), amount * 2)]);

	// Record pre-balances on AH.
	let (pre_sovereign_balance, pre_beneficiary_balance) = AssetHubPolkadot::execute_with(|| {
		type Balances = <AssetHubPolkadot as AssetHubPolkadotPallet>::Balances;
		(
			<Balances as Inspect<_>>::balance(&pallet_sovereign),
			<Balances as Inspect<_>>::balance(&beneficiary),
		)
	});

	// Send XCM from Collectives with the Architects origin.
	// pallet_xcm::send auto-prepends DescendOrigin based on ArchitectsToLocation, which
	// converts Architects to [Plurality { Technical, Voice }, GeneralIndex(ARCHITECTS_RANK)].
	// After DescendOrigin, the executor origin on AH becomes:
	//   (1, [Parachain(1001), Plurality { Technical, Voice }, GeneralIndex(ARCHITECTS_RANK)])
	//
	// The message then:
	// 1. UnpaidExecution — allowed because the computed origin matches FellowshipEntities
	// 2. AliasOrigin — FellowshipArchitectsAlias allows Architects → treasury/salary
	// 3. WithdrawAsset — withdraws DOT from the aliased pallet's sovereign account
	// 4. DepositAsset — deposits to beneficiary
	CollectivesPolkadot::execute_with(|| {
		let architects_origin = collectives_polkadot_runtime::RuntimeOrigin::from(
			collectives_polkadot_runtime::fellowship::pallet_fellowship_origins::Origin::Architects,
		);

		// Destination: sibling Asset Hub.
		let destination: Location =
			Location::new(1, [Parachain(AssetHubPolkadot::para_id().into())]);

		// The XCM body — no DescendOrigin needed, pallet_xcm::send prepends it.
		let xcm = Xcm::<()>(vec![
			UnpaidExecution { weight_limit: Unlimited, check_origin: None },
			AliasOrigin(pallet_location),
			WithdrawAsset((Parent, amount).into()),
			DepositAsset {
				assets: Wild(All),
				beneficiary: Location::new(
					0,
					[AccountId32Junction { network: None, id: beneficiary.clone().into() }],
				),
			},
		]);

		assert_ok!(<CollectivesPolkadot as CollectivesPolkadotPallet>::PolkadotXcm::send(
			architects_origin,
			bx!(VersionedLocation::from(destination)),
			bx!(VersionedXcm::from(xcm)),
		));
	});

	// Verify balance changes on AH.
	AssetHubPolkadot::execute_with(|| {
		type Balances = <AssetHubPolkadot as AssetHubPolkadotPallet>::Balances;

		let post_sovereign_balance = <Balances as Inspect<_>>::balance(&pallet_sovereign);
		let post_beneficiary_balance = <Balances as Inspect<_>>::balance(&beneficiary);

		assert!(
			post_sovereign_balance < pre_sovereign_balance,
			"Sovereign account balance should have decreased: pre={pre_sovereign_balance}, post={post_sovereign_balance}",
		);
		assert!(
			post_beneficiary_balance > pre_beneficiary_balance,
			"Beneficiary balance should have increased: pre={pre_beneficiary_balance}, post={post_beneficiary_balance}",
		);
	});
}

/// Negative test: the Fellows origin (rank 3) should NOT be able to alias into the Fellowship
/// Treasury. Only the Architects origin (rank 4+) can do so.
///
/// The Fellows origin is converted to `[Plurality { Technical, Voice }]` via `FellowsToPlurality`,
/// which does NOT include a `GeneralIndex` suffix. When it attempts `AliasOrigin` into the
/// Fellowship Treasury, `FellowshipArchitectsAlias` rejects it because the origin doesn't have
/// `GeneralIndex(ARCHITECTS_RANK)`. The XCM execution fails and balances remain unchanged.
#[test]
fn fellowship_fellows_cannot_alias_into_treasury_via_xcm() {
	let collectives_para_id: u32 = CollectivesPolkadot::para_id().into();
	let amount: Balance = POLKADOT_ED * 100;

	// Fellowship Treasury pallet location on Collectives, as seen from AH.
	let pallet_location = Location::new(
		1,
		[
			Parachain(collectives_para_id),
			PalletInstance(
				collectives_polkadot_runtime_constants::FELLOWSHIP_TREASURY_PALLET_INDEX,
			),
		],
	);

	// Compute the sovereign account for the treasury pallet on AH.
	let pallet_sovereign =
		asset_hub_polkadot_runtime::xcm_config::LocationToAccountId::convert_location(
			&pallet_location,
		)
		.expect("Failed to convert pallet location to account");

	let beneficiary: AccountId = [43u8; 32].into();

	// Fund the treasury's sovereign account on AH.
	AssetHubPolkadot::fund_accounts(vec![(pallet_sovereign.clone(), amount * 2)]);

	// Record pre-balances on AH.
	let (pre_sovereign_balance, pre_beneficiary_balance) = AssetHubPolkadot::execute_with(|| {
		type Balances = <AssetHubPolkadot as AssetHubPolkadotPallet>::Balances;
		(
			<Balances as Inspect<_>>::balance(&pallet_sovereign),
			<Balances as Inspect<_>>::balance(&beneficiary),
		)
	});

	// Send XCM from Collectives with the Fellows origin (NOT Architects).
	// pallet_xcm::send auto-prepends DescendOrigin based on FellowsToPlurality, which
	// converts Fellows to [Plurality { Technical, Voice }] (no GeneralIndex).
	// After DescendOrigin, the executor origin on AH becomes:
	//   (1, [Parachain(1001), Plurality { Technical, Voice }])
	//
	// The message then:
	// 1. UnpaidExecution — allowed because Fellows matches FellowshipEntities first arm
	// 2. AliasOrigin — REJECTED: FellowshipArchitectsAlias requires GeneralIndex(ARCHITECTS_RANK)
	// 3. XCM execution fails, WithdrawAsset + DepositAsset never execute
	CollectivesPolkadot::execute_with(|| {
		let fellows_origin = collectives_polkadot_runtime::RuntimeOrigin::from(
			collectives_polkadot_runtime::fellowship::pallet_fellowship_origins::Origin::Fellows,
		);

		let destination: Location =
			Location::new(1, [Parachain(AssetHubPolkadot::para_id().into())]);

		let xcm = Xcm::<()>(vec![
			UnpaidExecution { weight_limit: Unlimited, check_origin: None },
			AliasOrigin(pallet_location),
			WithdrawAsset((Parent, amount).into()),
			DepositAsset {
				assets: Wild(All),
				beneficiary: Location::new(
					0,
					[AccountId32Junction { network: None, id: beneficiary.clone().into() }],
				),
			},
		]);

		assert_ok!(<CollectivesPolkadot as CollectivesPolkadotPallet>::PolkadotXcm::send(
			fellows_origin,
			bx!(VersionedLocation::from(destination)),
			bx!(VersionedXcm::from(xcm)),
		));
	});

	// Verify message was processed with failure on AH.
	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: false, .. }
				) => {},
			]
		);
	});

	// Verify balances are unchanged — the alias was rejected so no funds moved.
	AssetHubPolkadot::execute_with(|| {
		type Balances = <AssetHubPolkadot as AssetHubPolkadotPallet>::Balances;

		let post_sovereign_balance = <Balances as Inspect<_>>::balance(&pallet_sovereign);
		let post_beneficiary_balance = <Balances as Inspect<_>>::balance(&beneficiary);

		assert_eq!(
			post_sovereign_balance, pre_sovereign_balance,
			"Sovereign account balance should be unchanged: pre={pre_sovereign_balance}, post={post_sovereign_balance}",
		);
		assert_eq!(
			post_beneficiary_balance, pre_beneficiary_balance,
			"Beneficiary balance should be unchanged: pre={pre_beneficiary_balance}, post={post_beneficiary_balance}",
		);
	});
}

#[test]
fn fellowship_architects_alias_into_treasury_via_xcm() {
	architects_alias_into_fellowship_pallet(
		collectives_polkadot_runtime_constants::FELLOWSHIP_TREASURY_PALLET_INDEX,
	);
}

#[test]
fn fellowship_architects_alias_into_salary_via_xcm() {
	architects_alias_into_fellowship_pallet(
		collectives_polkadot_runtime_constants::FELLOWSHIP_SALARY_PALLET_INDEX,
	);
}
