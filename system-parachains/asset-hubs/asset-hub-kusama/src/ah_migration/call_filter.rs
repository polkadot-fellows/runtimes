// Copyright (C) Parity Technologies (UK) Ltd.
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
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! Call filters for Asset Hub during the Asset Hub Migration.

use crate::*;

/// Contains all calls that are enabled before the migration.
pub struct CallsEnabledBeforeMigration;
impl Contains<<Runtime as frame_system::Config>::RuntimeCall> for CallsEnabledBeforeMigration {
	fn contains(call: &<Runtime as frame_system::Config>::RuntimeCall) -> bool {
		let (before, _, _) = call_allowed_status(call);
		if !before {
			log::warn!("Call bounced by the filter before the migration: {call:?}",);
		}
		before
	}
}

/// Contains all calls that are enabled during the migration.
pub struct CallsEnabledDuringMigration;
impl Contains<<Runtime as frame_system::Config>::RuntimeCall> for CallsEnabledDuringMigration {
	fn contains(call: &<Runtime as frame_system::Config>::RuntimeCall) -> bool {
		let (_before, during, _after) = call_allowed_status(call);
		if !during {
			log::warn!("Call bounced by the filter during the migration: {call:?}",);
		}
		during
	}
}

/// Contains all calls that are enabled after the migration.
pub struct CallsEnabledAfterMigration;
impl Contains<<Runtime as frame_system::Config>::RuntimeCall> for CallsEnabledAfterMigration {
	fn contains(call: &<Runtime as frame_system::Config>::RuntimeCall) -> bool {
		let (_before, _during, after) = call_allowed_status(call);
		if !after {
			log::warn!("Call bounced by the filter after the migration: {call:?}",);
		}
		after
	}
}

/// Return whether a call should be enabled before, during and/or after the migration.
///
/// Time line of the migration looks like this:
///
/// --------|-----------|--------->
///       Start        End
///
/// We now define 3 periods:
///
/// 1. Before the migration: [0, Start)
/// 2. During the migration: [Start, End]
/// 3. After the migration: (End, ∞)
///
/// Visually:
///
/// ```text
/// |--1---|
///         |-----2-----|
///                      |---3---->
/// --------|-----------|--------->
///       Start        End
/// ```
///
/// This call returns a 3-tuple to indicate whether a call is enabled during these periods. The
/// Start period contains our Warmup period and the End period contains our Cool-off period.
pub fn call_allowed_status(
	call: &<Runtime as frame_system::Config>::RuntimeCall,
) -> (bool, bool, bool) {
	use RuntimeCall::*;
	const ON: bool = true;
	const OFF: bool = false;
	let before_migration = call_allowed_before_migration(call);

	let during_migration = match call {
		AhMigrator(..) => ON, // required for the migration, only permissioned calls
		AhOps(..) => OFF,     // Not needed during the migration
		AssetConversion(..) => ON, // no reason to disable it, just convenience
		AssetRate(..) => OFF,
		Assets(..) => ON,   // no reason to disable it, just convenience
		Balances(..) => ON, // no reason to disable it, just convenience
		Bounties(..) => OFF,
		ChildBounties(..) => OFF,
		Claims(..) => OFF,
		MultiAssetBounties(..) => OFF,
		CollatorSelection(..) => ON, // Why?
		ConvictionVoting(..) => OFF,
		CumulusXcm(..) => OFF, /* Empty call enum, see https://github.com/paritytech/polkadot-sdk/issues/8222 */
		ForeignAssets(..) => ON, // no reason to disable it, just convenience
		Indices(..) => OFF,
		MultiBlockElection(..) => OFF,
		MultiBlockElectionSigned(..) => OFF,
		MultiBlockElectionUnsigned(..) => OFF,
		MultiBlockElectionVerifier(..) => OFF,
		MessageQueue(..) => ON, // contains non-permissioned service calls
		Multisig(..) => OFF,
		Nfts(..) => ON, // no reason to disable it, just convenience
		NominationPools(..) => OFF,
		ParachainInfo(parachain_info::Call::__Ignore { .. }) => ON, // Has no calls
		ParachainSystem(..) => ON,                                  // Only inherent and root calls
		PolkadotXcm(..) => ON,                                      /* no reason to disable it, */
		// just convenience
		PoolAssets(..) => ON, // no reason to disable it, just convenience
		Preimage(..) => OFF,
		Proxy(..) => OFF,
		Referenda(..) => OFF,
		Scheduler(..) => ON, // only permissioned service calls
		Session(..) => OFF,
		Staking(..) => OFF,
		StakingRcClient(..) => ON,     // Keep on for incoming RC calls over XCM
		StateTrieMigration(..) => OFF, // Deprecated
		System(..) => ON,              // remark plus root calls
		Timestamp(..) => ON,           // only `set` inherit
		ToPolkadotXcmRouter(..) => ON, // Allow to report bridge congestion
		Treasury(..) => OFF,
		Uniques(..) => OFF,
		Utility(..) => ON, // batching etc, just convenience
		Vesting(..) => OFF,
		VoterList(..) => OFF,
		Whitelist(..) => OFF,
		XcmpQueue(..) => ON, // Allow updating XCM settings. Only by Fellowship and root.
		RemoteProxyRelayChain(..) => OFF,
		NftFractionalization(..) => OFF,
		Recovery(..) => OFF,
		MultiBlockMigrations(..) => OFF, // has not calls
		Revive(..) => OFF,
		Parameters(..) => ON,
		Society(..) => OFF, // migrating pallet
	};
	// Exhaustive match. Compiler ensures that we did not miss any.

	// All pallets are enabled on Asset Hub after the migration :)
	let after_migration = ON;
	(before_migration, during_migration, after_migration)
}

/// Whether a call is enabled before the migration starts.
pub fn call_allowed_before_migration(
	call: &<Runtime as frame_system::Config>::RuntimeCall,
) -> bool {
	use RuntimeCall::*;
	const ON: bool = true;
	const OFF: bool = false;

	match call {
		// Disabled to avoid state insert conflicts.
		Staking(..) => OFF,
		// Not needed since staking is off.
		MultiBlockElection(..) => OFF,
		MultiBlockElectionSigned(..) => OFF,
		MultiBlockElectionUnsigned(..) => OFF,
		MultiBlockElectionVerifier(..) => OFF,
		NominationPools(..) => OFF,
		VoterList(..) => OFF,
		// To avoid insert issues.
		Indices(..) => OFF,
		Vesting(..) => OFF,
		// Governance disabled before migration starts.
		Bounties(..) => OFF,
		ChildBounties(..) => OFF,
		MultiAssetBounties(..) => OFF,
		ConvictionVoting(..) => OFF,
		Referenda(..) => OFF,
		Treasury(..) => OFF,
		Recovery(..) => OFF,
		Society(..) => OFF,              // migrating pallet
		MultiBlockMigrations(..) => OFF, // has not calls
		// Everything else is enabled before the migration.
		// Exhaustive match in case a pallet is added:
		AhMigrator(..) |
		AhOps(..) |
		AssetConversion(..) |
		AssetRate(..) |
		Assets(..) |
		Balances(..) |
		Claims(..) |
		CollatorSelection(..) |
		CumulusXcm(..) |
		ForeignAssets(..) |
		MessageQueue(..) |
		Multisig(..) |
		Nfts(..) |
		ParachainInfo(..) |
		ParachainSystem(..) |
		PolkadotXcm(..) |
		PoolAssets(..) |
		Preimage(..) |
		Proxy(..) |
		Scheduler(..) |
		Session(..) |
		StakingRcClient(..) |
		StateTrieMigration(..) |
		System(..) |
		Timestamp(..) |
		ToPolkadotXcmRouter(..) |
		Uniques(..) |
		Utility(..) |
		Whitelist(..) |
		XcmpQueue(..) |
		RemoteProxyRelayChain(..) |
		NftFractionalization(..) |
		Revive(..) |
		Parameters(..) => ON,
	}
}
