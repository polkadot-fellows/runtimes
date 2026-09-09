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

//! AHM v2 migration wiring: the relay-chain side of moving this chain's remaining state to the
//! Coretime chain and Asset Hub.
//!
//! Compiled only with the `ahm-v2` feature, which released runtimes do not enable. The
//! integration tests turn it on to drive the real runtime.

use crate::{
	xcm_config::{Broker, XcmRouter},
	BlockNumber, BrokerId, Runtime, RuntimeEvent,
};
use frame_support::{parameter_types, traits::Equals};
use kusama_runtime_constants::time::MINUTES;
use pallet_xcm::EnsureXcm;

#[cfg(feature = "on-chain-release-build")]
compile_error!("the `ahm-v2` feature must not be enabled in a release build");

parameter_types! {
	/// Manual verification window between the last data stage and finishing.
	pub const MigrationCoolOffPeriod: BlockNumber = 30 * MINUTES;
}

impl pallet_rc2_migrator::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type SendXcm = XcmRouter;
	type CtParaId = BrokerId;
	type CtOrigin = EnsureXcm<Equals<Broker>>;
	type CoolOffPeriod = MigrationCoolOffPeriod;
}

#[cfg(test)]
mod tests {
	use super::*;
	use frame_support::traits::PalletInfoAccess;

	/// `pallet-ct-migrator` hand-encodes this pallet's index; the compiler checks none of it.
	#[test]
	fn migrator_pallet_index_matches_what_the_coretime_chain_encodes() {
		assert_eq!(
			crate::Rc2Migrator::index(),
			pallet_ct_migrator::RC2_MIGRATOR_PALLET_INDEX as usize
		);
	}
}
