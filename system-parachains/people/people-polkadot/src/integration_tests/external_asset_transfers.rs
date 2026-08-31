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

//! Filter behavior tests for external-asset transfers between this People parachain and the rest
//! of the network. Pins the result of `IsTeleporter::contains` and `IsReserve::contains` across
//! the relevant origin / asset combinations.
//!
//! On Paseo the external asset was a trust-backed Asset Hub asset moved by teleport; on Polkadot
//! it is HOLLAR, reserve-backed by Hydration, so the pinned combinations differ: HOLLAR must only
//! ever arrive as a reserve transfer from Hydration, and never by teleport.

use crate::{
	assets::hollar::{HollarLocation, HydrationLocation},
	xcm_config::{AssetHubLocation, RelayLocation, XcmConfig},
};
use frame_support::traits::ContainsPair;
use polkadot_runtime_constants::system_parachain::ASSET_HUB_ID;
use xcm::latest::prelude::*;

type IsTeleporter = <XcmConfig as xcm_executor::Config>::IsTeleporter;
type IsReserve = <XcmConfig as xcm_executor::Config>::IsReserve;

fn external_asset(amount: u128) -> Asset {
	(HollarLocation::get(), amount).into()
}

/// Build the canonical AH-side asset Location for an arbitrary trust-backed asset id from AH.
fn ah_asset(asset_id: u128, amount: u128) -> Asset {
	(
		Location::new(1, [Parachain(ASSET_HUB_ID), PalletInstance(50), GeneralIndex(asset_id)]),
		amount,
	)
		.into()
}

// --- IsReserve --------------------------------------------------------------

#[test]
fn external_asset_reserve_from_hydration_is_accepted() {
	assert!(IsReserve::contains(&external_asset(1_000), &HydrationLocation::get()));
}

#[test]
fn external_asset_reserve_from_asset_hub_is_rejected() {
	// HOLLAR's only reserve is Hydration; Asset Hub must not gain a second one.
	assert!(!IsReserve::contains(&external_asset(1_000), &AssetHubLocation::get()));
}

#[test]
fn external_asset_reserve_from_relay_is_rejected() {
	assert!(!IsReserve::contains(&external_asset(1_000), &RelayLocation::get()));
}

#[test]
fn other_ah_asset_reserve_from_asset_hub_is_accepted() {
	// Regression: AH-issued trust-backed assets still flow as reserve transfers from AH.
	let other = ah_asset(1337, 1_000);
	assert!(IsReserve::contains(&other, &AssetHubLocation::get()));
}

#[test]
fn other_ah_asset_reserve_from_relay_is_rejected() {
	let other = ah_asset(1337, 1_000);
	assert!(!IsReserve::contains(&other, &RelayLocation::get()));
}

// --- IsTeleporter -----------------------------------------------------------

#[test]
fn external_asset_teleport_is_rejected_from_everywhere() {
	for origin in [
		HydrationLocation::get(),
		AssetHubLocation::get(),
		RelayLocation::get(),
		Location::new(1, [Parachain(4242)]),
	] {
		assert!(
			!IsTeleporter::contains(&external_asset(1_000), &origin),
			"HOLLAR must never be teleported (origin {origin:?})"
		);
	}
}

#[test]
fn dot_teleport_from_relay_still_works() {
	// Regression: relay native teleport rule unaffected.
	let dot: Asset = (RelayLocation::get(), 1_000u128).into();
	assert!(IsTeleporter::contains(&dot, &RelayLocation::get()));
}

#[test]
fn dot_teleport_from_asset_hub_still_works() {
	let dot: Asset = (RelayLocation::get(), 1_000u128).into();
	assert!(IsTeleporter::contains(&dot, &AssetHubLocation::get()));
}
