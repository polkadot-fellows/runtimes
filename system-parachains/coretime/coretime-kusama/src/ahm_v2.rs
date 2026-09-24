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

use crate::{
	xcm_config::XcmRouter, AccountId, Balances, BlockNumber, HrmpPara, MessageQueue, RegistrarPara,
	Runtime, RuntimeEvent, RuntimeHoldReason,
};
use frame_support::{parameter_types, traits::ConstU32};
use frame_system::EnsureRoot;

parameter_types! {
	/// While the migration runs, the relay chain's downward queue is served first for this many
	/// blocks out of every cycle, and every queue takes its turn for the rest.
	pub const DmpQueuePriorityPattern: (BlockNumber, BlockNumber) = (18, 2);
}

impl pallet_ct_migrator::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	// Relay blocks are 6s, this chain's are 12s: migrated proxy delays halve.
	type RcBlockTimeRatio = ConstU32<2>;
	// Migrated records are handed straight to the pallets that own them, which take their own
	// deposits at this chain's rates rather than inheriting the relay chain's amounts.
	type RegistrarReceiver = RegistrarPara;
	type HrmpReceiver = HrmpPara;
	type SendXcm = XcmRouter;
	type AdminOrigin = EnsureRoot<AccountId>;
	type MessageQueue = MessageQueue;
	type DmpQueuePriorityPattern = DmpQueuePriorityPattern;
}

#[cfg(test)]
mod tests {
	use crate::{Runtime, RuntimeCall};
	use codec::Encode;
	use pallet_ct_migrator::Call as C;
	use pallet_rc2_migrator::{CtMigratorCall as M, CtRuntimeCall};

	/// Ensure the pallet + call index aligns.
	#[test]
	fn the_relay_chain_encodes_this_chains_calls_correctly() {
		let cases: Vec<(CtRuntimeCall, RuntimeCall)> = vec![
			(M::StartMigration, C::<Runtime>::start_migration {}),
			(M::EndLockdown, C::end_lockdown {}),
			(M::ReceiveAccounts { accounts: vec![] }, C::receive_accounts { accounts: vec![] }),
			(
				M::ReconcileBalances { rc_kept: 1, rc_migrated: 2 },
				C::reconcile_balances { rc_kept: 1, rc_migrated: 2 },
			),
			(M::ReceiveProxies { proxies: vec![] }, C::receive_proxies { proxies: vec![] }),
			(
				M::ReceiveRegistrar { paras: vec![], next_free_para_id: Some(7) },
				C::receive_registrar { paras: vec![], next_free_para_id: Some(7) },
			),
			(M::ReceiveHrmp { channels: vec![] }, C::receive_hrmp { channels: vec![] }),
			(
				M::ReceiveHrmpRequests { requests: vec![] },
				C::receive_hrmp_requests { requests: vec![] },
			),
		]
		.into_iter()
		.map(|(sent, real)| (CtRuntimeCall::CtMigrator(sent), RuntimeCall::CtMigrator(real)))
		.collect();
		for (sent, real) in cases {
			assert_eq!(sent.encode(), real.encode(), "{real:?}");
		}
	}
}
