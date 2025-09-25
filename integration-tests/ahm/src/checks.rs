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

//! Generic checks for Relay and AH.

use crate::porting_prelude::*;

use pallet_ah_migrator::types::AhMigrationCheck;
use pallet_rc_migrator::types::RcMigrationCheck;

#[cfg(feature = "try-runtime")]
use frame_support::{
	defensive_assert,
	traits::{TryDecodeEntireStorage, TryState},
};

pub struct SanityChecks;

impl RcMigrationCheck for SanityChecks {
	type RcPrePayload = ();

	fn pre_check() -> Self::RcPrePayload {
		let stage = pallet_rc_migrator::RcMigrationStage::<RcRuntime>::get();
		// rust tests trigger pre-checks when pending(0), while the ZB snapshots are from when the
		// migration is still pending. Both okay.
		assert!(
			stage == pallet_rc_migrator::MigrationStage::Pending ||
				stage == pallet_rc_migrator::MigrationStage::Scheduled { start: 0u32 },
		);
	}

	fn post_check(_: Self::RcPrePayload) {
		// assert_eq!(
		// pallet_rc_migrator::RcMigrationStage::<RcRuntime>::get(),
		// pallet_rc_migrator::MigrationStage::MigrationDone
		// );
	}
}

impl AhMigrationCheck for SanityChecks {
	type RcPrePayload = ();
	type AhPrePayload = ();

	fn pre_check(_: Self::RcPrePayload) -> Self::AhPrePayload {
		assert_eq!(
			pallet_ah_migrator::AhMigrationStage::<AhRuntime>::get(),
			pallet_ah_migrator::MigrationStage::Pending
		);
	}

	fn post_check(_rc_pre_payload: Self::RcPrePayload, _: Self::AhPrePayload) {
		assert_eq!(
			pallet_ah_migrator::AhMigrationStage::<AhRuntime>::get(),
			pallet_ah_migrator::MigrationStage::MigrationDone
		);
	}
}

/// Assert that the root hash is what we expect.
pub fn assert_root_hash(chain: &str, want_hex: &str) {
	let got = hex::encode(sp_io::storage::root(sp_runtime::StateVersion::V1));
	println!("{chain} root hash: {got:?}");
	if got == want_hex {
		return;
	}

	panic!("The root hash of {chain} is not as expected. Please adjust the root hash in integration-tests/ahm/src/checks.rs\nExpected: {want_hex}\nGot:      {got}");
}

/// Runs the try-state checks for all pallets once pre-migration on RC, and once post-migration on
/// AH.
///
/// noop if `feature = "try-runtime"` is not enabled.
pub struct PalletsTryStateCheck;
impl RcMigrationCheck for PalletsTryStateCheck {
	type RcPrePayload = ();

	fn pre_check() -> Self::RcPrePayload {
		#[cfg(feature = "try-runtime")]
		{
			let res = polkadot_runtime::AllPalletsWithSystem::try_state(
				frame_system::Pallet::<polkadot_runtime::Runtime>::block_number(),
				frame_support::traits::TryStateSelect::All,
			);
			if res.is_err() {
				log::error!("Pallets try-state check failed: {res:?}");
				defensive_assert!(false, "Pallets try-state check failed");
			}
		}
	}
	fn post_check(_: Self::RcPrePayload) {
		// nada
	}
}

impl AhMigrationCheck for PalletsTryStateCheck {
	type AhPrePayload = ();
	type RcPrePayload = ();

	fn pre_check(_: Self::RcPrePayload) -> Self::AhPrePayload {
		// nada
	}

	fn post_check(_: Self::RcPrePayload, _: Self::AhPrePayload) {
		#[cfg(feature = "try-runtime")]
		{
			let res = asset_hub_polkadot_runtime::AllPalletsWithSystem::try_state(
				frame_system::Pallet::<asset_hub_polkadot_runtime::Runtime>::block_number(),
				frame_support::traits::TryStateSelect::All,
			);

			if let Err(es) = res {
				log::error!("Pallets try-state check failed with error {:?}", es);
				defensive_assert!(false, "Pallets try-state check failed");
			}
		}
	}
}

pub struct EntireStateDecodes;

impl RcMigrationCheck for EntireStateDecodes {
	type RcPrePayload = ();

	fn pre_check() -> Self::RcPrePayload {}

	fn post_check(_: Self::RcPrePayload) {
		#[cfg(feature = "try-runtime")]
		{
			let res = polkadot_runtime::AllPalletsWithSystem::try_decode_entire_state();

			if let Err(es) = res {
				log::error!(
					"Pallets try-decode-entire-storage check failed with {} errors",
					es.len()
				);
				for e in es {
					log::error!("- {}, value: {:?}", &e, e.raw.as_ref().map(hex::encode));
				}
				defensive_assert!(false, "Pallets try-decode-entire-storage check failed");
			}
		}
	}
}

impl AhMigrationCheck for EntireStateDecodes {
	type RcPrePayload = ();
	type AhPrePayload = ();

	fn pre_check(_: Self::RcPrePayload) -> Self::AhPrePayload {}

	fn post_check(_: Self::RcPrePayload, _: Self::AhPrePayload) {
		#[cfg(feature = "try-runtime")]
		{
			let res = asset_hub_polkadot_runtime::AllPalletsWithSystem::try_decode_entire_state();
			if let Err(es) = res {
				log::error!(
					"Pallets try-decode-entire-storage check failed with {} errors",
					es.len()
				);
				for e in es {
					log::error!("- {}, value: {:?}", &e, e.raw.as_ref().map(hex::encode));
				}
				defensive_assert!(false, "Pallets try-decode-entire-storage check failed");
			}
		}
	}
}
