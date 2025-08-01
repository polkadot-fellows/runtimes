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
use people_polkadot_runtime::xcm_config::XcmConfig;
use xcm::latest::Junctions::*;

const ALLOWED: bool = true;
const DENIED: bool = false;

const TELEPORT_FEES: bool = true;
const RESERVE_TRANSFER_FEES: bool = false;

#[test]
fn account_on_sibling_syschain_aliases_into_same_local_account() {
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

	// Aliasing same account on different chains
	test_cross_chain_alias!(
		vec![
			// between AH and People: allowed
			(AssetHubPolkadot, PeoplePolkadot, TELEPORT_FEES, ALLOWED),
			// between BH and People: allowed
			(BridgeHubPolkadot, PeoplePolkadot, TELEPORT_FEES, ALLOWED),
			// between Collectives and People: allowed
			(CollectivesPolkadot, PeoplePolkadot, TELEPORT_FEES, ALLOWED),
			// between Coretime and People: allowed
			(CoretimePolkadot, PeoplePolkadot, TELEPORT_FEES, ALLOWED),
			// between Penpal and People: denied
			(PenpalA, PeoplePolkadot, RESERVE_TRANSFER_FEES, DENIED)
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
			// between AH and People: denied
			(AssetHubPolkadot, PeoplePolkadot, TELEPORT_FEES, DENIED),
			// between BH and People: denied
			(BridgeHubPolkadot, PeoplePolkadot, TELEPORT_FEES, DENIED),
			// between Collectives and People: denied
			(CollectivesPolkadot, PeoplePolkadot, TELEPORT_FEES, DENIED),
			// between Coretime and People: denied
			(CoretimePolkadot, PeoplePolkadot, TELEPORT_FEES, DENIED),
			// between Penpal and People: denied
			(PenpalA, PeoplePolkadot, RESERVE_TRANSFER_FEES, DENIED)
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

	let pal_admin = <PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get());
	PenpalA::mint_foreign_asset(pal_admin.clone(), Location::parent(), origin.clone(), fees * 10);
	PenpalA::mint_foreign_asset(pal_admin, Location::parent(), bad_origin.clone(), fees * 10);
	PeoplePolkadot::fund_accounts(vec![(target.clone(), fees * 10)]);

	// let's authorize `origin` on Penpal to alias `target` on Coretime
	PeoplePolkadot::execute_with(|| {
		let penpal_origin = Location::new(
			1,
			X2([
				Parachain(PenpalA::para_id().into()),
				AccountId32Junction { network: Some(Polkadot), id: origin.clone().into() },
			]
			.into()),
		);
		// `target` adds `penpal_origin` as authorized alias
		assert_ok!(<PeoplePolkadot as PeoplePolkadotPallet>::PolkadotXcm::add_authorized_alias(
			<PeoplePolkadot as Chain>::RuntimeOrigin::signed(target.clone()),
			Box::new(penpal_origin.into()),
			None
		));
	});
	// Verify that unauthorized `bad_origin` cannot alias into `target`, from any chain.
	test_cross_chain_alias!(
		vec![
			// between AH and People: denied
			(AssetHubPolkadot, PeoplePolkadot, TELEPORT_FEES, DENIED),
			// between BH and People: denied
			(BridgeHubPolkadot, PeoplePolkadot, TELEPORT_FEES, DENIED),
			// between Coretime and People: denied
			(CoretimePolkadot, PeoplePolkadot, TELEPORT_FEES, DENIED),
			// between Penpal and People: denied
			(PenpalA, PeoplePolkadot, RESERVE_TRANSFER_FEES, DENIED)
		],
		bad_origin,
		target,
		fees
	);
	// Verify that only authorized `penpal::origin` can alias into `target`, while `origin` on other
	// chains cannot.
	test_cross_chain_alias!(
		vec![
			// between AH and People: denied
			(AssetHubPolkadot, PeoplePolkadot, TELEPORT_FEES, DENIED),
			// between BH and People: denied
			(BridgeHubPolkadot, PeoplePolkadot, TELEPORT_FEES, DENIED),
			// between Coretime and People: denied
			(CoretimePolkadot, PeoplePolkadot, TELEPORT_FEES, DENIED),
			// between Penpal and People: allowed
			(PenpalA, PeoplePolkadot, RESERVE_TRANSFER_FEES, ALLOWED)
		],
		origin,
		target,
		fees
	);
	// remove authorization for `origin` on Penpal to alias `target` on Coretime
	PeoplePolkadot::execute_with(|| {
		// `target` removes all authorized aliases
		assert_ok!(
			<PeoplePolkadot as PeoplePolkadotPallet>::PolkadotXcm::remove_all_authorized_aliases(
				<PeoplePolkadot as Chain>::RuntimeOrigin::signed(target.clone())
			)
		);
	});
	// Verify `penpal::origin` can no longer alias into `target` on Coretime.
	test_cross_chain_alias!(
		vec![(PenpalA, PeoplePolkadot, RESERVE_TRANSFER_FEES, DENIED)],
		origin,
		target,
		fees
	);
}

#[test]
fn aliasing_child_locations() {
	PeoplePolkadot::execute_with(|| {
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
fn asset_hub_root_aliases_anything() {
	PeoplePolkadot::execute_with(|| {
		// Allows AH root to alias anything.
		let origin = Location::new(1, X1([Parachain(1000)].into()));

		let target = Location::new(1, X1([Parachain(2000)].into()));
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let target =
			Location::new(1, X1([AccountId32Junction { network: None, id: [1u8; 32] }].into()));
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let target = Location::new(
			1,
			X2([Parachain(8), AccountId32Junction { network: None, id: [1u8; 32] }].into()),
		);
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let target =
			Location::new(1, X3([Parachain(42), PalletInstance(8), GeneralIndex(9)].into()));
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let target = Location::new(2, X1([GlobalConsensus(Ethereum { chain_id: 1 })].into()));
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let target = Location::new(2, X2([GlobalConsensus(Kusama), Parachain(1000)].into()));
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let target = Location::new(0, X2([PalletInstance(8), GeneralIndex(9)].into()));
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));

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
