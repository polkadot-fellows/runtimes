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

//! AHM v2 migration wiring: the relay-chain side of moving account, proxy, registrar and HRMP
//! state to the Coretime chain.

use crate::{
	xcm_config::{CoretimeLocation, XcmRouter},
	AccountId, Balance, Balances, BlockNumber, BrokerId, MessageQueue, OnDemandPalletId, Runtime,
	RuntimeCall, RuntimeEvent, Timestamp, TreasuryPalletId, EXISTENTIAL_DEPOSIT, UNITS,
};
use alloc::{vec, vec::Vec};
use frame_support::{parameter_types, traits::Equals, PalletId};
use frame_system::EnsureRoot;
use pallet_xcm::{EnsureResponse, EnsureXcm};
use polkadot_runtime_constants::system_parachain;
use sp_runtime::traits::AccountIdConversion;

parameter_types! {
	pub const AssetHubId: u32 = system_parachain::ASSET_HUB_ID;
	/// Leftover pots emptied by the migration's `Sweep` stage. `dap/satl` is the retired
	/// direct-allocation pot's `PalletId`; the on-demand pot can accrue order revenue up to
	/// the migration.
	pub SweepAccounts: Vec<AccountId> = vec![
		TreasuryPalletId::get().into_account_truncating(),
		PalletId(*b"dap/satl").into_account_truncating(),
		OnDemandPalletId::get().into_account_truncating(),
	];
	/// Where swept pots and dust land on Asset Hub: the AH treasury account (same `PalletId`
	/// derivation, so the same address). TODO: point at the DAP buffer account once governance
	/// designates it.
	pub SweepBeneficiary: AccountId = TreasuryPalletId::get().into_account_truncating();
	/// Audited issuance held by no account ("phantom issuance"), burned at the end of the
	/// migration. Measured at RC block #33,103,807 (`balance_census` prints the exact value);
	/// re-measure and update ahead of the real run.
	pub const TiCorrection: u128 = 216_577_461_180_573;
	/// Working buffer of free balance that follows a migrated deposit to the Coretime chain.
	pub const CtFreeBuffer: Balance = UNITS;
	/// The accounts that may drive the migration collectively. Governance seeds the real set
	/// before a migration is scheduled; empty means only root and the appointed manager can act.
	pub MigrationMultisigMembers: Vec<AccountId> = Vec::new();
	/// Votes needed from distinct members.
	pub const MigrationMultisigThreshold: u32 = 3;
	/// Votes one member may cast per round.
	pub const MigrationMultisigMaxVotesPerRound: u32 = 5;
	/// A vote is signed over (who, call, round) and nothing else, so two networks sitting at the
	/// same round would accept each other's signatures. This is what keeps them apart.
	pub const MigrationMultisigStartRound: u32 = 100;
	/// While the migration runs, the Coretime chain's upward queue is served first for this many
	/// blocks out of every cycle, and every queue takes its turn for the rest.
	pub const CtUmpQueuePriorityPattern: (BlockNumber, BlockNumber) = (18, 2);
	/// How long a batch sent to the Coretime chain may go unanswered before it is reported.
	/// Generous next to a round trip through both message queues: the point is to catch a
	/// message that will never be answered, not to police latency.
	pub const MigrationXcmResponseTimeout: BlockNumber = 100;
	/// How many batches may be outstanding before data extraction pauses for a block. Keeps the
	/// relay chain from running far ahead of what Coretime has acknowledged, without serialising
	/// the migration on a full round trip per batch.
	pub const MigrationUnprocessedMsgBuffer: u32 = 8;
	/// Asset Hub's existential deposit; mirrors
	/// `system_parachains_constants::polkadot::currency::SYSTEM_PARA_EXISTENTIAL_DEPOSIT`
	/// (= relay ED / 10) without pulling that crate into the relay runtime.
	pub const AhExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT / 10;
}

impl pallet_rc2_migrator::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type SendXcm = XcmRouter;
	type CtParaId = BrokerId;
	type AhParaId = AssetHubId;
	type CtFreeBuffer = CtFreeBuffer;
	type AhExistentialDeposit = AhExistentialDeposit;
	type SweepAccounts = SweepAccounts;
	type SweepBeneficiary = SweepBeneficiary;
	type TiCorrection = TiCorrection;
	type TimeProvider = Timestamp;
	type CtOrigin = EnsureXcm<Equals<CoretimeLocation>>;
	type AdminOrigin = EnsureRoot<AccountId>;
	type RuntimeCall = RuntimeCall;
	type MultisigMembers = MigrationMultisigMembers;
	type MultisigThreshold = MigrationMultisigThreshold;
	type MultisigMaxVotesPerRound = MigrationMultisigMaxVotesPerRound;
	type MultisigStartRound = MigrationMultisigStartRound;
	type MessageQueue = MessageQueue;
	type CtUmpQueuePriorityPattern = CtUmpQueuePriorityPattern;
	type XcmResponseTimeout = MigrationXcmResponseTimeout;
	type UnprocessedMsgBuffer = MigrationUnprocessedMsgBuffer;
	type NotifyQueryHandler = Runtime;
	type ResponseOrigin = EnsureResponse<Equals<CoretimeLocation>>;
}

#[cfg(test)]
mod tests {
	use crate::{Runtime, RuntimeCall};
	use codec::Encode;
	use pallet_ct_migrator::{Rc2MigratorCall, Rc2RuntimeCall};

	/// Ensure the pallet + call index aligns.
	#[test]
	fn the_coretime_chain_encodes_this_chains_calls_correctly() {
		assert_eq!(
			Rc2RuntimeCall::Rc2Migrator(Rc2MigratorCall::CtReady).encode(),
			RuntimeCall::Rc2Migrator(pallet_rc2_migrator::Call::<Runtime>::ct_ready {}).encode(),
		);
	}
}
