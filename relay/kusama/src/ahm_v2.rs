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
	type TimeProvider = crate::Timestamp;
	type CtOrigin = EnsureXcm<Equals<Broker>>;
	type CoolOffPeriod = MigrationCoolOffPeriod;
}

#[cfg(test)]
mod tests {
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
