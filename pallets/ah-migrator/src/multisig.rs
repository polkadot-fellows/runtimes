// This file is part of Substrate.

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

use crate::*;

#[cfg(feature = "std")]
use pallet_rc_migrator::types::AccountIdOf;

impl<T: Config> Pallet<T> {
	pub fn do_receive_multisigs(multisigs: Vec<RcMultisigOf<T>>) -> Result<(), Error<T>> {
		Self::deposit_event(Event::BatchReceived {
			pallet: PalletEventName::Multisig,
			count: multisigs.len() as u32,
		});
		let (mut count_good, mut count_bad) = (0, 0);
		log::info!(target: LOG_TARGET, "Integrating {} multisigs", multisigs.len());

		for multisig in multisigs {
			match Self::do_receive_multisig(multisig) {
				Ok(()) => count_good += 1,
				Err(e) => {
					count_bad += 1;
					log::error!(target: LOG_TARGET, "Error while integrating multisig: {e:?}");
				},
			}
		}
		Self::deposit_event(Event::BatchProcessed {
			pallet: PalletEventName::Multisig,
			count_good,
			count_bad,
		});

		Ok(())
	}

	pub fn do_receive_multisig(multisig: RcMultisigOf<T>) -> Result<(), Error<T>> {
		// Translate the creator account from RC to AH format
		let translated_creator = Self::translate_account_rc_to_ah(multisig.creator.clone());

		let missing = <T as pallet_multisig::Config>::Currency::unreserve(
			&translated_creator,
			multisig.deposit,
		);

		if missing != Default::default() {
			log::error!(
				target: LOG_TARGET,
				"Failed to unreserve deposit for multisig {}: missing amount: {:?}",
				translated_creator.to_ss58check(),
				missing
			);

			Self::deposit_event(Event::FailedToUnreserveMultisigDeposit {
				expected_amount: multisig.deposit,
				missing_amount: missing,
				account: translated_creator,
			});
		}

		Ok(())
	}
}

#[cfg(feature = "std")]
impl<T: Config> crate::types::AhMigrationCheck for MultisigMigrationChecker<T> {
	// Vec of multisig account ids with non-zero balance on the relay chain before migration
	type RcPrePayload = Vec<AccountIdOf<T>>;
	// Number of multisigs on Asset Hub before migration
	type AhPrePayload = u32;

	fn pre_check(_: Self::RcPrePayload) -> Self::AhPrePayload {
		pallet_multisig::Multisigs::<T>::iter_keys().count() as u32
	}

	fn post_check(rc_pre_payload: Self::RcPrePayload, ah_pre_payload: Self::AhPrePayload) {
		// Assert storage 'Multisig::Multisigs::ah_post::correct'
		assert_eq!(
			pallet_multisig::Multisigs::<T>::iter_keys().count() as u32,
			ah_pre_payload,
			"Number of multisigs on Asset Hub should be the same before and after migration"
		);

		// Apply account translation to RC pre-check data for consistent comparison
		for account_id in rc_pre_payload {
			// Translate the account ID to match the migration logic
			let translated_account_id = Pallet::<T>::translate_account_rc_to_ah(account_id.clone());

			// Assert storage 'Multisig::Multisigs::ah_post::consistent'
			assert!(
				frame_system::Account::<T>::contains_key(&translated_account_id),
				"Translated multisig account {:?} -> {:?} from Relay Chain should be present on Asset Hub",
				account_id.to_ss58check(),
				translated_account_id.to_ss58check()
			);
		}
	}
}
