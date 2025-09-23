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
// along with Polkadot. If not, see <http://www.gnu.org/licenses/>.

use crate::porting_prelude::*;
use pallet_ah_migrator::types::AhMigrationCheck;
use pallet_rc_migrator::{
	recovery::{PortableActiveRecovery, PortableRecoveryConfig},
	types::{IntoPortable, RcMigrationCheck},
};
use sp_core::crypto::AccountId32;

pub struct RecoveryDataMigrated;

#[derive(Clone, PartialEq, Eq)]
pub struct TestData {
	recoverable: Vec<(AccountId32, PortableRecoveryConfig)>,
	active_recoveries: Vec<(AccountId32, AccountId32, PortableActiveRecovery)>,
	proxy: Vec<(AccountId32, AccountId32)>,
}

impl RcMigrationCheck for RecoveryDataMigrated {
	type RcPrePayload = TestData;

	fn pre_check() -> Self::RcPrePayload {
		TestData {
			recoverable: pallet_recovery::Recoverable::<RcRuntime>::iter()
				.map(|(who, config)| (who, config.into_portable()))
				.collect(),
			active_recoveries: pallet_recovery::ActiveRecoveries::<RcRuntime>::iter()
				.map(|(w1, w2, config)| (w1, w2, config.into_portable()))
				.collect(),
			proxy: pallet_recovery::Proxy::<RcRuntime>::iter().collect(),
		}
	}

	fn post_check(_: Self::RcPrePayload) {
		assert_eq!(pallet_recovery::Recoverable::<RcRuntime>::iter_keys().count(), 0);
		assert_eq!(pallet_recovery::ActiveRecoveries::<RcRuntime>::iter_keys().count(), 0);
		assert_eq!(pallet_recovery::Proxy::<RcRuntime>::iter_keys().count(), 0);
	}
}

impl AhMigrationCheck for RecoveryDataMigrated {
	type RcPrePayload = TestData;
	type AhPrePayload = (); // Not deployed on AH pre

	fn pre_check(_: Self::RcPrePayload) -> Self::AhPrePayload {
		assert_eq!(pallet_recovery::Recoverable::<AhRuntime>::iter_keys().count(), 0);
		assert_eq!(pallet_recovery::ActiveRecoveries::<AhRuntime>::iter_keys().count(), 0);
		assert_eq!(pallet_recovery::Proxy::<AhRuntime>::iter_keys().count(), 0);
	}

	fn post_check(rc_pre_payload: Self::RcPrePayload, _: Self::AhPrePayload) {
		// sanity checks
		assert!(!rc_pre_payload.recoverable.is_empty());
		assert!(!rc_pre_payload.active_recoveries.is_empty());
		assert!(!rc_pre_payload.proxy.is_empty());

		assert_eq!(
			pallet_recovery::Recoverable::<AhRuntime>::iter().collect::<Vec<_>>(),
			rc_pre_payload
				.recoverable
				.into_iter()
				.map(|(who, config)| (who, config.into()))
				.collect::<Vec<_>>()
		);
		assert_eq!(
			pallet_recovery::ActiveRecoveries::<AhRuntime>::iter().collect::<Vec<_>>(),
			rc_pre_payload
				.active_recoveries
				.into_iter()
				.map(|(w1, w2, config)| (w1, w2, config.into()))
				.collect::<Vec<_>>()
		);
		assert_eq!(
			pallet_recovery::Proxy::<AhRuntime>::iter().collect::<Vec<_>>(),
			rc_pre_payload.proxy
		);
	}
}
