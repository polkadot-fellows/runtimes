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

//! AHM v2 migration wiring: the Coretime-chain side of receiving the relay chain's remaining
//! state.
//!
//! Compiled only with the `ahm-v2` feature, which released runtimes do not enable. The
//! integration tests turn it on to drive the real runtime.

use crate::{xcm_config::XcmRouter, Runtime, RuntimeEvent};

#[cfg(feature = "on-chain-release-build")]
compile_error!("the `ahm-v2` feature must not be enabled in a release build");

impl pallet_ct_migrator::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type SendXcm = XcmRouter;
}

#[cfg(test)]
mod tests {
	use frame_support::traits::PalletInfoAccess;

	/// `pallet-rc2-migrator` hand-encodes this pallet's index; the compiler checks none of it.
	#[test]
	fn migrator_pallet_index_matches_what_the_relay_chain_encodes() {
		assert_eq!(
			crate::CtMigrator::index(),
			pallet_rc2_migrator::CT_MIGRATOR_PALLET_INDEX as usize
		);
	}
}
