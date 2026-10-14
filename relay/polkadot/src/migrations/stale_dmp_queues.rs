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

//! Removes the downward message queues of paras that no longer exist.
//!
//! Before paritytech/polkadot-sdk#6604, DMP accepted messages for paras without a head. Nothing
//! removes those queues, and a para onboarding under one of these ids would inherit them. See
//! <https://github.com/polkadot-fellows/runtimes/issues/513>.

use crate::{parachains_dmp, parachains_paras, Dmp, Runtime};
use frame_support::{
	pallet_prelude::ValueQuery, storage_alias, traits::OnRuntimeUpgrade, weights::Weight,
	Twox64Concat,
};
use polkadot_primitives::{Hash, Id as ParaId};
#[cfg(feature = "try-runtime")]
use {
	alloc::vec::Vec,
	codec::{Decode, Encode},
	frame_support::ensure,
	sp_runtime::TryRuntimeError,
};

const LOG_TARGET: &str = "runtime::polkadot::migrations";

/// Paras with a downward message queue but no head in live state at block 33036100.
const STALE_PARAS: [u32; 8] = [0, 666, 2001, 2050, 2087, 3350, 3351, 4009];

/// `parachains_dmp::DownwardMessageQueueHeads` is not public.
#[storage_alias(pallet_name)]
type DownwardMessageQueueHeads = StorageMap<Dmp, Twox64Concat, ParaId, Hash, ValueQuery>;

fn has_head(para: &ParaId) -> bool {
	parachains_paras::Heads::<Runtime>::contains_key(para)
}

/// Removes the downward message queue and queue head of each of [`STALE_PARAS`], like offboarding
/// does. Skips paras that have a head, because they may be running.
///
/// Queue and head have to go together, and only for a para that cannot be running: the para
/// mirrors the head in `cumulus_pallet_parachain_system::LastDmqMqcHead` and asserts the two match
/// on every block, so one that already consumed messages would be left at a non-zero hash that the
/// reset relay side can never reach again, panicking with `DMQ head mismatch` from then on.
/// Skipping is harmless: queue and head were built up from zero in lockstep, so a para onboarding
/// under that id replays the stale messages and arrives at the stored head.
pub struct RemoveStaleDmpQueues;

impl OnRuntimeUpgrade for RemoveStaleDmpQueues {
	fn on_runtime_upgrade() -> Weight {
		let mut removed = 0;
		for para in STALE_PARAS.map(ParaId::from) {
			if has_head(&para) {
				log::warn!(target: LOG_TARGET, "Keeping the downward message queue of {para:?}, it has a head");
				continue;
			}
			parachains_dmp::DownwardMessageQueues::<Runtime>::remove(para);
			DownwardMessageQueueHeads::remove(para);
			removed += 1;
			log::info!(target: LOG_TARGET, "Removed the stale downward message queue of {para:?}");
		}
		<Runtime as frame_system::Config>::DbWeight::get()
			.reads_writes(STALE_PARAS.len() as u64, 2 * removed)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		Ok(entries_with_head().encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		let pre = <(u32, u32)>::decode(&mut &state[..])
			.map_err(|_| "Failed to decode pre-upgrade state")?;
		ensure!(
			parachains_dmp::DownwardMessageQueues::<Runtime>::iter_keys().all(|p| has_head(&p)) &&
				DownwardMessageQueueHeads::iter_keys().all(|p| has_head(&p)),
			"A para without a head still has a downward message queue"
		);
		ensure!(
			entries_with_head() == pre,
			"A downward message queue of a para with a head is gone"
		);
		Ok(())
	}
}

/// Number of downward message queues and queue heads of paras that have a head.
#[cfg(feature = "try-runtime")]
fn entries_with_head() -> (u32, u32) {
	(
		parachains_dmp::DownwardMessageQueues::<Runtime>::iter_keys()
			.filter(has_head)
			.count() as u32,
		DownwardMessageQueueHeads::iter_keys().filter(has_head).count() as u32,
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::parachains_configuration::HostConfiguration;

	/// A para that has a head and is not listed.
	const LIVE_PARA: u32 = 1000;

	/// Queues a message to `para` through DMP, which requires a head, then removes the head unless
	/// `keep_head`.
	fn queue_message(para: u32, keep_head: bool) {
		let config = HostConfiguration { max_downward_message_size: 1024, ..Default::default() };
		parachains_dmp::Pallet::<Runtime>::make_parachain_reachable(para);
		parachains_dmp::Pallet::<Runtime>::queue_downward_message(&config, para.into(), vec![1])
			.unwrap();
		if !keep_head {
			parachains_paras::Heads::<Runtime>::remove(ParaId::from(para));
		}
	}

	/// Whether `para` has a downward message queue and a queue head.
	fn entries(para: u32) -> (bool, bool) {
		let para = ParaId::from(para);
		(
			parachains_dmp::DownwardMessageQueues::<Runtime>::contains_key(para),
			DownwardMessageQueueHeads::contains_key(para),
		)
	}

	/// Queues for all listed paras, of which only the first has a head, and for [`LIVE_PARA`].
	fn stale_state() -> sp_io::TestExternalities {
		let mut ext = sp_io::TestExternalities::default();
		ext.execute_with(|| {
			queue_message(STALE_PARAS[0], true);
			STALE_PARAS[1..].iter().for_each(|para| queue_message(*para, false));
			queue_message(LIVE_PARA, true);
		});
		ext
	}

	#[test]
	fn removes_listed_queues_of_paras_without_head() {
		stale_state().execute_with(|| {
			for para in STALE_PARAS.into_iter().chain([LIVE_PARA]) {
				assert_eq!(entries(para), (true, true));
			}

			RemoveStaleDmpQueues::on_runtime_upgrade();

			for para in &STALE_PARAS[1..] {
				assert_eq!(entries(*para), (false, false));
			}
			for para in [STALE_PARAS[0], LIVE_PARA] {
				assert_eq!(entries(para), (true, true));
			}
		});
	}

	#[cfg(feature = "try-runtime")]
	#[test]
	fn post_upgrade_fails_on_unlisted_queue_without_head() {
		stale_state().execute_with(|| {
			assert!(RemoveStaleDmpQueues::try_on_runtime_upgrade(true).is_ok());
		});
		stale_state().execute_with(|| {
			queue_message(LIVE_PARA + 1, false);
			assert!(RemoveStaleDmpQueues::try_on_runtime_upgrade(true).is_err());
		});
	}
}
