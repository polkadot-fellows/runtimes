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
//! Behind the `ahm-v2` feature until the migration's storage layout stops changing — the stage
//! machine grows a variant per data stage, and an enum that still moves has no business in a
//! shipped runtime. The feature is what lets the integration tests drive the real runtime
//! meanwhile.

use crate::{parachains_origin, xcm_config::XcmRouter, BlockNumber, Runtime, RuntimeOrigin};
use frame_support::{parameter_types, traits::EnsureOrigin};
use polkadot_runtime_constants::{system_parachain::BROKER_ID, time::MINUTES};

parameter_types! {
	pub const CoretimeParaId: u32 = BROKER_ID;
	/// Manual verification window between the last data stage and finishing, during which the
	/// migration's call filters are still engaged and the end state can be inspected on chain.
	pub const MigrationCoolOffPeriod: BlockNumber = 30 * MINUTES;
}

/// Accepts only the Coretime chain's own parachain origin.
///
/// Same shape as [`crate::EnsureAssetHub`]: match the parachain origin the XCM converter produced
/// and check the id. Root is deliberately not accepted — governance's way into the stage machine
/// is `force_set_stage`, not an acknowledgement it could forge on the Coretime chain's behalf.
pub struct EnsureCoretime;

impl EnsureOrigin<RuntimeOrigin> for EnsureCoretime {
	type Success = ();

	fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
		match <RuntimeOrigin as Into<Result<parachains_origin::Origin, RuntimeOrigin>>>::into(
			o.clone(),
		) {
			Ok(parachains_origin::Origin::Parachain(id)) if id == BROKER_ID.into() => Ok(()),
			_ => Err(o),
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
		Ok(RuntimeOrigin::root())
	}
}

impl pallet_rc2_migrator::Config for Runtime {
	type RuntimeEvent = crate::RuntimeEvent;
	type SendXcm = XcmRouter;
	type CtParaId = CoretimeParaId;
	type CtOrigin = EnsureCoretime;
	type CoolOffPeriod = MigrationCoolOffPeriod;
}
