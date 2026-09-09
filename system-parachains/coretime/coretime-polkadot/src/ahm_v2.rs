// Copyright (C) Polkadot Fellows.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot. If not, see <http://www.gnu.org/licenses/>.

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
