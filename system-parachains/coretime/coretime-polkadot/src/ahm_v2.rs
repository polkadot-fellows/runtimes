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

//! AHM v2 migration wiring: the Coretime-chain side of receiving account, proxy, registrar and
//! HRMP state from the relay chain.
//!
//! Compiled only with the `ahm-v2` feature, which released runtimes do not enable. The
//! integration tests turn it on to drive the real runtime.

use crate::{xcm_config::XcmRouter, AccountId, Runtime, RuntimeEvent};
use frame_system::EnsureRoot;

impl pallet_ct_migrator::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type SendXcm = XcmRouter;
	type AdminOrigin = EnsureRoot<AccountId>;
}

#[cfg(test)]
mod tests {
	use crate::{Runtime, RuntimeCall};
	use codec::Encode;
	use pallet_rc2_migrator::{CtMigratorCall, CtRuntimeCall};

	/// Ensure the pallet + call index aligns.
	#[test]
	fn the_relay_chain_encodes_this_chains_calls_correctly() {
		assert_eq!(
			CtRuntimeCall::CtMigrator(CtMigratorCall::StartMigration).encode(),
			RuntimeCall::CtMigrator(pallet_ct_migrator::Call::<Runtime>::start_migration {})
				.encode(),
		);
		assert_eq!(
			CtRuntimeCall::CtMigrator(CtMigratorCall::EndLockdown).encode(),
			RuntimeCall::CtMigrator(pallet_ct_migrator::Call::<Runtime>::end_lockdown {})
				.encode(),
		);
	}
}
