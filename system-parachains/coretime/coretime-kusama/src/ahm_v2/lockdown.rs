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

//! The Coretime chain's AHM v2 lockdown: which calls each migration stage allows.

use crate::RuntimeCall;
use frame_support::traits::Contains;

/// Contains all calls that are enabled before the migration starts.
pub struct CallsEnabledBeforeMigration;
impl Contains<RuntimeCall> for CallsEnabledBeforeMigration {
	fn contains(call: &RuntimeCall) -> bool {
		call_allowed_status(call).0
	}
}

/// Contains all calls that are enabled during the migration.
pub struct CallsEnabledDuringMigration;
impl Contains<RuntimeCall> for CallsEnabledDuringMigration {
	fn contains(call: &RuntimeCall) -> bool {
		call_allowed_status(call).1
	}
}

/// Return whether a call is enabled before and during the migration. Every call is enabled after
/// it.
///
/// During is from the relay chain's start signal until it ends the lockdown.
///
/// Proxy definitions arrive from the relay chain and are merged into `pallet_proxy`, so during the
/// migration every proxy call that changes the proxy map, or the deposits recorded in it, is
/// disabled. Using a proxy and handling announcements stay enabled. Announcements are not
/// migrated: the relay chain releases their deposits and sends them to Asset Hub as free balance.
pub fn call_allowed_status(call: &RuntimeCall) -> (bool, bool) {
	use RuntimeCall::*;
	const ON: bool = true;
	const OFF: bool = false;

	match call {
		System(..) => (ON, ON),
		ParachainSystem(..) => (ON, ON),
		Timestamp(..) => (ON, ON),
		ParachainInfo(..) => (ON, ON),
		Balances(..) => (ON, ON),
		CollatorSelection(..) => (ON, ON),
		Session(..) => (ON, ON),
		XcmpQueue(..) => (ON, ON),
		PolkadotXcm(..) => (ON, ON),
		CumulusXcm(..) => (ON, ON),
		MessageQueue(..) => (ON, ON),
		Utility(..) => (ON, ON),
		Multisig(..) => (ON, ON),
		// Leave the proxy map untouched.
		Proxy(
			pallet_proxy::Call::proxy { .. } |
			pallet_proxy::Call::proxy_announced { .. } |
			pallet_proxy::Call::announce { .. } |
			pallet_proxy::Call::remove_announcement { .. } |
			pallet_proxy::Call::reject_announcement { .. },
		) => (ON, ON),
		Broker(..) => (ON, ON),
		CtMigrator(..) => (ON, ON),

		// TODO(ahm-v2): `RegistrarPara` and `HrmpPara` join as `(OFF, OFF)`.
		Proxy(..) => (ON, OFF),
	}
}

#[cfg(test)]
mod tests {
	use crate::{AccountId, ProxyType, Runtime, RuntimeCall};
	use frame_support::traits::Contains;
	use pallet_ct_migrator::{CtMigrationStage, MigrationStage};
	use parachains_runtimes_test_utils::ExtBuilder;

	fn allowed_at(stage: &MigrationStage, call: &RuntimeCall) -> bool {
		ExtBuilder::<Runtime>::default().build().execute_with(|| {
			CtMigrationStage::<Runtime>::put(stage);
			<Runtime as frame_system::Config>::BaseCallFilter::contains(call)
		})
	}

	#[test]
	fn proxy_changes_are_refused_while_the_migration_runs() {
		let alice = AccountId::new([1; 32]); // any account
		let remark = RuntimeCall::System(frame_system::Call::remark { remark: vec![1] });
		let changes = [
			RuntimeCall::Proxy(pallet_proxy::Call::add_proxy {
				delegate: alice.clone().into(),
				proxy_type: ProxyType::Any,
				delay: 0,
			}),
			RuntimeCall::Proxy(pallet_proxy::Call::remove_proxy {
				delegate: alice.clone().into(),
				proxy_type: ProxyType::Any,
				delay: 0,
			}),
			RuntimeCall::Proxy(pallet_proxy::Call::remove_proxies {}),
			RuntimeCall::Proxy(pallet_proxy::Call::create_pure {
				proxy_type: ProxyType::Any,
				delay: 0,
				index: 0,
			}),
			RuntimeCall::Proxy(pallet_proxy::Call::kill_pure {
				spawner: alice.clone().into(),
				proxy_type: ProxyType::Any,
				index: 0,
				height: 0,
				ext_index: 0,
			}),
			RuntimeCall::Proxy(pallet_proxy::Call::poke_deposit {}),
		];
		let uses = [
			RuntimeCall::Proxy(pallet_proxy::Call::proxy {
				real: alice.clone().into(),
				force_proxy_type: None,
				call: Box::new(remark.clone()),
			}),
			RuntimeCall::Proxy(pallet_proxy::Call::proxy_announced {
				delegate: alice.clone().into(),
				real: alice.clone().into(),
				force_proxy_type: None,
				call: Box::new(remark.clone()),
			}),
			RuntimeCall::Proxy(pallet_proxy::Call::announce {
				real: alice.clone().into(),
				call_hash: Default::default(),
			}),
			RuntimeCall::Proxy(pallet_proxy::Call::remove_announcement {
				real: alice.clone().into(),
				call_hash: Default::default(),
			}),
			RuntimeCall::Proxy(pallet_proxy::Call::reject_announcement {
				delegate: alice.clone().into(),
				call_hash: Default::default(),
			}),
			remark,
		];

		for stage in [
			MigrationStage::Pending,
			MigrationStage::DataMigrationOngoing,
			MigrationStage::MigrationDone,
		] {
			// GIVEN the chain at `stage`.
			// THEN a proxy change is refused exactly while the migration runs.
			for call in &changes {
				assert_eq!(allowed_at(&stage, call), !stage.is_ongoing(), "{call:?} at {stage:?}");
			}
			// THEN using a proxy, handling announcements, and any other call, pass at every stage.
			for call in &uses {
				assert!(allowed_at(&stage, call), "{call:?} refused at {stage:?}");
			}
		}
	}
}
