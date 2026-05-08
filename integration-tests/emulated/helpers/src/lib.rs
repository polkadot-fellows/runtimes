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

pub use paste;

// Substrate
pub use pallet_balances;
pub use pallet_message_queue;

// Polkadot
pub use pallet_xcm;
pub use xcm::prelude::{AccountId32, VersionedAssetId, VersionedAssets, Weight, WeightLimit};
pub use xcm_runtime_apis::fees::runtime_decl_for_xcm_payment_api::XcmPaymentApiV2;

// Cumulus
pub use cumulus_pallet_xcmp_queue;
pub use emulated_integration_tests_common::*;
pub use xcm_emulator::Chain;

pub mod common;

/// Read the balance of an `Assets`-pallet asset on a chain.
///
/// Centralised here so the asset-hub-{kusama,polkadot} and bridge-hub-{kusama,polkadot}
/// test crates share one definition. The polkadot-sdk emulated tests define the same
/// macro per-test-crate (it is not exported from `emulated-integration-tests-common`),
/// so we cannot pull it from upstream and instead host it here.
#[macro_export]
macro_rules! assets_balance_on {
	( $chain:ident, $id:expr, $who:expr ) => {
		$crate::paste::paste! {
			<$chain>::execute_with(|| {
				type Assets = <$chain as [<$chain Pallet>]>::Assets;
				<Assets as frame_support::traits::fungibles::Inspect<_>>::balance($id, $who)
			})
		}
	};
}
