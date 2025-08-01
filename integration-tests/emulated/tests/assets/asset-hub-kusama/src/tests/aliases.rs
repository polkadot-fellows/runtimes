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
use frame_support::{traits::ContainsPair, BoundedVec};
use xcm::latest::Junctions::*;
use AssetHubKusamaXcmConfig as XcmConfig;

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
	let fees = KUSAMA_ED * 1000;

	PenpalB::mint_foreign_asset(
		<PenpalB as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
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
			(BridgeHubKusama, AssetHubKusama, TELEPORT_FEES, DENIED),
			// between Coretime and AH: denied
			(CoretimeKusama, AssetHubKusama, TELEPORT_FEES, DENIED),
			// between People and AH: denied
			(PeopleKusama, AssetHubKusama, TELEPORT_FEES, DENIED),
			// between Penpal and AH: denied
			(PenpalB, AssetHubKusama, RESERVE_TRANSFER_FEES, DENIED)
		],
		origin,
		target,
		fees
	);
}

#[test]
fn account_on_sibling_syschain_cannot_alias_into_different_local_account() {
	// origin and target are different accounts on different chains
	let origin: AccountId = [1; 32].into();
	let target: AccountId = [2; 32].into();
	let fees = KUSAMA_ED * 1000;

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
			(BridgeHubKusama, AssetHubKusama, TELEPORT_FEES, DENIED),
			// between Coretime and AH: denied
			(CoretimeKusama, AssetHubKusama, TELEPORT_FEES, DENIED),
			// between People and AH: denied
			(PeopleKusama, AssetHubKusama, TELEPORT_FEES, DENIED),
			// between Penpal and AH: denied
			(PenpalA, AssetHubKusama, RESERVE_TRANSFER_FEES, DENIED)
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
	let fees = KUSAMA_ED * 1000;

	let pal_admin = <PenpalB as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get());
	PenpalB::mint_foreign_asset(pal_admin.clone(), Location::parent(), origin.clone(), fees * 10);
	PenpalB::mint_foreign_asset(pal_admin, Location::parent(), bad_origin.clone(), fees * 10);
	AssetHubKusama::fund_accounts(vec![(target.clone(), fees * 10)]);

	// let's authorize `origin` on Penpal to alias `target` on AssetHub
	AssetHubKusama::execute_with(|| {
		let penpal_origin = Location::new(
			1,
			X2([
				Parachain(PenpalB::para_id().into()),
				AccountId32Junction { network: Some(Kusama), id: origin.clone().into() },
			]
			.into()),
		);
		// `target` adds `penpal_origin` as authorized alias
		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::PolkadotXcm::add_authorized_alias(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(target.clone()),
			Box::new(penpal_origin.into()),
			None
		));
	});
	// Verify that unauthorized `bad_origin` cannot alias into `target`, from any chain.
	test_cross_chain_alias!(
		vec![
			// between BH and AssetHub: denied
			(BridgeHubKusama, AssetHubKusama, TELEPORT_FEES, DENIED),
			// between Coretime and AssetHub: denied
			(CoretimeKusama, AssetHubKusama, TELEPORT_FEES, DENIED),
			// between People and AssetHub: denied
			(PeopleKusama, AssetHubKusama, TELEPORT_FEES, DENIED),
			// between Penpal and AssetHub: denied
			(PenpalB, AssetHubKusama, RESERVE_TRANSFER_FEES, DENIED)
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
			(BridgeHubKusama, AssetHubKusama, TELEPORT_FEES, DENIED),
			// between Coretime and AssetHub: denied
			(CoretimeKusama, AssetHubKusama, TELEPORT_FEES, DENIED),
			// between People and AssetHub: denied
			(PeopleKusama, AssetHubKusama, TELEPORT_FEES, DENIED),
			// between Penpal and AssetHub: allowed
			(PenpalB, AssetHubKusama, RESERVE_TRANSFER_FEES, ALLOWED)
		],
		origin,
		target,
		fees
	);
	// remove authorization for `origin` on Penpal to alias `target` on AssetHub
	AssetHubKusama::execute_with(|| {
		// `target` removes all authorized aliases
		assert_ok!(
			<AssetHubKusama as AssetHubKusamaPallet>::PolkadotXcm::remove_all_authorized_aliases(
				<AssetHubKusama as Chain>::RuntimeOrigin::signed(target.clone())
			)
		);
	});
	// Verify `penpal::origin` can no longer alias into `target` on AssetHub.
	test_cross_chain_alias!(
		vec![(PenpalB, AssetHubKusama, RESERVE_TRANSFER_FEES, DENIED)],
		origin,
		target,
		fees
	);
}

#[test]
fn aliasing_child_locations() {
	AssetHubKusama::execute_with(|| {
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
	AssetHubKusama::execute_with(|| {
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
		let target = Location::new(2, X2([GlobalConsensus(Polkadot), Parachain(1000)].into()));
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
		let target = Location::new(2, X2([GlobalConsensus(Polkadot), Parachain(1000)].into()));
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
fn asset_hub_polkadot_root_aliases_into_polkadot_origins() {
	AssetHubKusama::execute_with(|| {
		let origin = Location::new(2, X2([GlobalConsensus(Polkadot), Parachain(1000)].into()));

		let target = Location::new(2, X2([GlobalConsensus(Polkadot), Parachain(2000)].into()));
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));

		let target = Location::new(
			2,
			X3([
				GlobalConsensus(Polkadot),
				Parachain(2000),
				AccountId32Junction { network: None, id: AssetHubKusamaSender::get().into() },
			]
			.into()),
		);
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));

		let target = Location::new(
			2,
			X4([GlobalConsensus(Polkadot), Parachain(2000), PalletInstance(8), GeneralIndex(9)]
				.into()),
		);
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
	});
}

#[test]
fn asset_hub_polkadot_root_aliases_into_ethereum_mainnet_origins() {
	AssetHubKusama::execute_with(|| {
		let origin = Location::new(2, X2([GlobalConsensus(Polkadot), Parachain(1000)].into()));

		let target = Location::new(2, X1([GlobalConsensus(Ethereum { chain_id: 1 })].into()));
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));

		let target = Location::new(2, X1([GlobalConsensus(Ethereum { chain_id: 2 })].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));

		let target = Location::new(
			2,
			X2([
				GlobalConsensus(Ethereum { chain_id: 1 }),
				AccountKey20 { network: None, key: ETHEREUM_BOB },
			]
			.into()),
		);
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
	});
}

#[test]
fn asset_hub_polkadot_root_does_not_alias_into_asset_hub_kusama_origins() {
	AssetHubKusama::execute_with(|| {
		let origin = Location::new(2, X2([GlobalConsensus(Polkadot), Parachain(1000)].into()));

		let target = Location::new(2, X1([GlobalConsensus(Kusama)].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));

		let target = Location::new(2, X2([GlobalConsensus(Kusama), Parachain(2000)].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));

		let target = Location::new(
			2,
			X3([
				GlobalConsensus(Kusama),
				Parachain(2000),
				AccountId32Junction { network: None, id: AssetHubKusamaSender::get().into() },
			]
			.into()),
		);
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));

		let target = Location::new(
			2,
			X4([GlobalConsensus(Kusama), Parachain(2000), PalletInstance(8), GeneralIndex(9)]
				.into()),
		);
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
	});
}
