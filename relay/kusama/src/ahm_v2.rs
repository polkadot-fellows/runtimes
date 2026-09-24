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
	xcm_config::{Broker, XcmRouter},
	AccountId, Balance, Balances, BlockNumber, BrokerId, MessageQueue, OnDemandPalletId, Runtime,
	RuntimeCall, RuntimeEvent, SocietyPalletId, Timestamp, TreasuryPalletId, EXISTENTIAL_DEPOSIT,
	UNITS,
};
use alloc::{vec, vec::Vec};
use frame_support::{parameter_types, traits::Equals, PalletId};
use frame_system::EnsureRoot;
use kusama_runtime_constants::system_parachain;
use pallet_xcm::{EnsureResponse, EnsureXcm};
use sp_runtime::traits::AccountIdConversion;

parameter_types! {
	pub const AssetHubId: u32 = system_parachain::ASSET_HUB_ID;
	/// Leftover pots emptied by the migration's `Sweep` stage.
	///
	/// Kusama's list is not Polkadot's: there is no retired direct-allocation pot here, and the
	/// Society pot is Kusama-only. Each entry is a pot whose balance has no owner to migrate it
	/// to, so it is swept rather than left stranded on a chain that will hold no DOT/KSM.
	pub SweepAccounts: Vec<AccountId> = vec![
		TreasuryPalletId::get().into_account_truncating(),
		SocietyPalletId::get().into_account_truncating(),
		OnDemandPalletId::get().into_account_truncating(),
		// The accumulate-and-forward pot that collects relay-chain dust for Asset Hub.
		PalletId(*b"acf/ksmt").into_account_truncating(),
	];
	/// Where swept pots and dust land on Asset Hub. Kusama sweeps to the treasury; Polkadot
	/// sweeps to its DAP buffer. Same `PalletId` derivation, so the same address on both sides.
	pub SweepBeneficiary: AccountId = TreasuryPalletId::get().into_account_truncating();
	/// Audited issuance held by no account ("phantom issuance"), burned at the end of the
	/// migration.
	///
	/// Measured by the `balance_census` test against the 22 Sep 2026 snapshot, which prints the
	/// exact planck value. Re-measure and update ahead of the real run: this is a one-shot burn,
	/// and burning more than the chain actually carries is unrecoverable.
	pub const TiCorrection: u128 = 2_054_657_180_420;
	/// Working buffer of free balance that follows a migrated deposit to the Coretime chain.
	/// One KSM, mirroring Polkadot's one DOT — the two are different amounts of money, and the
	/// point is a usable buffer on each chain rather than a matching number.
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
	pub const MigrationMultisigStartRound: u32 = 200;
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
	/// `system_parachains_constants::kusama::currency::SYSTEM_PARA_EXISTENTIAL_DEPOSIT`
	/// without pulling that crate into the relay runtime.
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
	type CtOrigin = EnsureXcm<Equals<Broker>>;
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
	type ResponseOrigin = EnsureResponse<Equals<Broker>>;
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
