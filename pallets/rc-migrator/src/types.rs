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

//! Types

use super::*;
use pallet_referenda::{ReferendumInfoOf, TrackIdOf};

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

/// Relay Chain Freeze Reason
#[derive(Encode, Decode)]
pub enum AssetHubPalletConfig<T: Config> {
	#[codec(index = 255)]
	AhmController(AhMigratorCall<T>),
}

/// Call encoding for the calls needed from the Broker pallet.
#[derive(Encode, Decode)]
pub enum AhMigratorCall<T: Config> {
	#[codec(index = 0)]
	ReceiveAccounts { accounts: Vec<accounts::AccountFor<T>> },
	#[codec(index = 1)]
	ReceiveMultisigs { multisigs: Vec<multisig::RcMultisigOf<T>> },
	#[codec(index = 2)]
	ReceiveProxyProxies { proxies: Vec<proxy::RcProxyLocalOf<T>> },
	#[codec(index = 3)]
	ReceiveProxyAnnouncements { announcements: Vec<RcProxyAnnouncementOf<T>> },
	#[codec(index = 4)]
	ReceivePreimageChunks { chunks: Vec<preimage::RcPreimageChunk> },
	#[codec(index = 5)]
	ReceivePreimageRequestStatus { request_status: Vec<preimage::RcPreimageRequestStatusOf<T>> },
	#[codec(index = 6)]
	ReceivePreimageLegacyStatus { legacy_status: Vec<preimage::RcPreimageLegacyStatusOf<T>> },
	#[codec(index = 7)]
	ReceiveNomPoolsMessages { messages: Vec<staking::nom_pools::RcNomPoolsMessage<T>> },
	#[codec(index = 8)]
	ReceiveFastUnstakeMessages { messages: Vec<staking::fast_unstake::RcFastUnstakeMessage<T>> },
	#[codec(index = 9)]
	ReceiveReferendaValues {
		referendum_count: u32,
		deciding_count: Vec<(TrackIdOf<T, ()>, u32)>,
		track_queue: Vec<(TrackIdOf<T, ()>, Vec<(u32, u128)>)>,
	},
	#[codec(index = 10)]
	ReceiveReferendums { referendums: Vec<(u32, ReferendumInfoOf<T, ()>)> },
	#[codec(index = 10)]
	ReceiveClaimsMessages { messages: Vec<claims::RcClaimsMessageOf<T>> },
	#[codec(index = 11)]
	ReceiveBagsListMessages { messages: Vec<staking::bags_list::RcBagsListMessage<T>> },
	#[codec(index = 12)]
	ReceiveSchedulerMessages { messages: Vec<scheduler::RcSchedulerMessageOf<T>> },
}

/// Copy of `ParaInfo` type from `paras_registrar` pallet.
///
/// From: https://github.com/paritytech/polkadot-sdk/blob/b7afe48ed0bfef30836e7ca6359c2d8bb594d16e/polkadot/runtime/common/src/paras_registrar/mod.rs#L50-L59
#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug, TypeInfo)]
pub struct ParaInfo<AccountId, Balance> {
	/// The account that has placed a deposit for registering this para.
	pub manager: AccountId,
	/// The amount reserved by the `manager` account for the registration.
	pub deposit: Balance,
	/// Whether the para registration should be locked from being controlled by the manager.
	/// None means the lock had not been explicitly set, and should be treated as false.
	pub locked: Option<bool>,
}

/// Weight information for the processing the packages from this pallet on the Asset Hub.
pub trait AhWeightInfo {
	/// Weight for processing a single account on AH.
	fn migrate_account() -> Weight;
}

impl AhWeightInfo for () {
	fn migrate_account() -> Weight {
		Weight::from_all(1)
	}
}

pub trait PalletMigration {
	type Key: codec::MaxEncodedLen;
	type Error;

	/// Migrate until the weight is exhausted. The give key is the last one that was migrated.
	///
	/// Should return the last key that was migrated. This will then be passed back into the next
	/// call.
	fn migrate_many(
		last_key: Option<Self::Key>,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<Self::Key>, Self::Error>;
}

/// Trait to run some checks before and after a pallet migration.
///
/// This needs to be called by the test harness.
pub trait PalletMigrationChecks {
	type Payload;

	/// Run some checks before the migration and store intermediate payload.
	fn pre_check() -> Self::Payload;

	/// Run some checks after the migration and use the intermediate payload.
	fn post_check(payload: Self::Payload);
}
