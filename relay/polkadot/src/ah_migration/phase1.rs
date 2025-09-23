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

//! First phase of the Asset Hub Migration.

use crate::*;
use frame_support::traits::Contains;
use pallet_rc_migrator::types::PortableFreezeReason;

/// Contains all calls that are enabled during the migration.
pub struct CallsEnabledDuringMigration;
impl Contains<<Runtime as frame_system::Config>::RuntimeCall> for CallsEnabledDuringMigration {
	fn contains(call: &<Runtime as frame_system::Config>::RuntimeCall) -> bool {
		let (during, _after) = call_allowed_status(call);
		if !during {
			log::warn!("Call bounced by the filter during the migration: {call:?}");
		}
		during
	}
}

/// Contains all calls that are enabled after the migration.
pub struct CallsEnabledAfterMigration;
impl Contains<<Runtime as frame_system::Config>::RuntimeCall> for CallsEnabledAfterMigration {
	fn contains(call: &<Runtime as frame_system::Config>::RuntimeCall) -> bool {
		let (_during, after) = call_allowed_status(call);
		if !after {
			log::warn!("Call bounced by the filter after the migration: {call:?}");
		}
		after
	}
}

/// The hold reason for staking delegation.
pub struct StakingDelegationReason;
impl Get<RuntimeHoldReason> for StakingDelegationReason {
	fn get() -> RuntimeHoldReason {
		RuntimeHoldReason::DelegatedStaking(pallet_delegated_staking::HoldReason::StakingDelegation)
	}
}

/// Return whether a call should be enabled during and/or after the migration.
///
/// Time line of the migration looks like this:
///
/// --------|-----------|--------->
///       Start        End
///
/// We now define 2 periods:
///
/// 1. During the migration: [Start, End]
/// 2. After the migration: (End, ∞)
///
/// Visually:
///
/// ```text
///         |-----1-----|
///                      |---2---->
/// --------|-----------|--------->
///       Start        End
/// ```
///
/// This call returns a 2-tuple to indicate whether a call is enabled during these periods. The
/// Start period contains our Warmup period and the End period contains our Cool-off period.
pub fn call_allowed_status(call: &<Runtime as frame_system::Config>::RuntimeCall) -> (bool, bool) {
	use RuntimeCall::*;
	const ON: bool = true;
	const OFF: bool = false;

	match call {
		System(..) => (ON, ON), // Remarks, root calls and `set_code` if we need for emergency.
		Scheduler(..) => (OFF, OFF), // Only for governance, hence disabled.
		Preimage(..) => (OFF, OFF), // Only for governance, hence disabled.
		Babe(..) => (ON, ON),   // For equivocation proof submissions; security relevant
		Timestamp(..) => (ON, ON), // only `set` inherit
		Indices(..) => (OFF, OFF), // Not needed anymore and migrated to AH.
		Balances(..) => (OFF, ON), // Disabled during migration to avoid confusing externals.
		Staking(..) => (OFF, OFF),
		StakingAhClient(..) => (ON, ON), // Only permissioned calls and needed for the migration.
		Session(..) => (ON, ON),         // Does not affect any migrating pallet.
		Grandpa(..) => (ON, ON),         // For equivocation proof submissions; security relevant
		Treasury(..) => (OFF, OFF),
		ConvictionVoting(..) => (OFF, OFF),
		Referenda(..) => (OFF, OFF),
		Whitelist(..) => (OFF, OFF),
		Claims(..) => (OFF, OFF),
		Vesting(..) => (OFF, OFF),
		Utility(..) => (ON, ON),   // batching etc
		Proxy(..) => (OFF, ON),    // On after the migration to keep proxy accounts accessible.
		Multisig(..) => (OFF, ON), // On after the migration to keep multisig accounts accessible.
		Bounties(..) => (OFF, OFF),
		ChildBounties(..) => (OFF, OFF),
		ElectionProviderMultiPhase(..) => (OFF, OFF),
		VoterList(..) => (OFF, OFF),
		NominationPools(..) => (OFF, OFF),
		FastUnstake(..) => (OFF, OFF),
		Configuration(..) => (ON, ON),
		ParasShared(parachains_shared::Call::__Ignore { .. }) => (ON, ON), // Has no calls
		ParaInclusion(parachains_inclusion::Call::__Ignore { .. }) => (ON, ON), // Has no calls
		ParaInherent(..) => (ON, ON),                                      // only inherents
		Paras(..) => (ON, ON),                                             /* Only root and one
		                                                                     * security relevant
		                                                                     * call: */
		// `include_pvf_check_statement`
		Initializer(..) => (ON, ON), // Only root calls. Fine to keep.
		Hrmp(..) => (ON, ON),        /* open close hrmp channels by parachains or root force. */
		// no concerns.
		ParasDisputes(..) => (ON, ON), // Only a single root call. Fine to keep.
		ParasSlashing(..) => (ON, ON), /* Security critical. If disabled there will be no */
		// slashes or offences generated for malicious
		// validators.
		OnDemand(..) => (OFF, ON),
		Registrar(..) => (OFF, ON),
		Slots(..) => (OFF, OFF),
		Auctions(..) => (OFF, OFF),
		Crowdloan(
			crowdloan::Call::<Runtime>::dissolve { .. } |
			crowdloan::Call::<Runtime>::refund { .. } |
			crowdloan::Call::<Runtime>::withdraw { .. },
		) => (OFF, ON),
		Crowdloan(..) => (OFF, OFF),
		Coretime(coretime::Call::<Runtime>::request_revenue_at { .. }) => (OFF, ON),
		Coretime(..) => (ON, ON),             // Only permissioned calls.
		StateTrieMigration(..) => (OFF, OFF), // Deprecated
		XcmPallet(..) => (ON, ON),            // during migration can only send XCMs to other
		MessageQueue(..) => (ON, ON),         // contains non-permissioned service calls
		AssetRate(..) => (OFF, OFF),
		Beefy(..) => (ON, ON), // For reporting equivocation proofs; security relevant
		RcMigrator(..) => (ON, ON), // Required for the migration, only permissioned calls
	}
	// Exhaustive match. Compiler ensures that we did not miss any.
}

// Type safe mapping of RC hold reason to portable format.
impl pallet_rc_migrator::types::IntoPortable for RuntimeHoldReason {
	type Portable = pallet_rc_migrator::types::PortableHoldReason;

	fn into_portable(self) -> Self::Portable {
		use pallet_rc_migrator::types::PortableHoldReason;

		match self {
			RuntimeHoldReason::Preimage(inner) => PortableHoldReason::Preimage(inner),
			RuntimeHoldReason::StateTrieMigration(inner) =>
				PortableHoldReason::StateTrieMigration(inner),
			RuntimeHoldReason::DelegatedStaking(inner) =>
				PortableHoldReason::DelegatedStaking(inner),
			RuntimeHoldReason::Staking(inner) => PortableHoldReason::Staking(inner),
			RuntimeHoldReason::Session(inner) => PortableHoldReason::Session(inner),
			RuntimeHoldReason::XcmPallet(inner) => PortableHoldReason::XcmPallet(inner),
		}
	}
}

impl pallet_rc_migrator::types::IntoPortable for RuntimeFreezeReason {
	type Portable = pallet_rc_migrator::types::PortableFreezeReason;

	fn into_portable(self) -> Self::Portable {
		match self {
			RuntimeFreezeReason::NominationPools(inner) =>
				PortableFreezeReason::NominationPools(inner),
		}
	}
}
