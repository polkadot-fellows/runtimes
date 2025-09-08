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

use crate::{Config, Pallet};
use frame_support::traits::Defensive;
use pallet_rc_migrator::types::ToPolkadotSs58;

impl<T: Config> Pallet<T> {
	/// Translate account from RC format to AH format.
	///
	/// TODO introduce different accountId types for RC and AH
	pub fn translate_account_rc_to_ah(account: T::AccountId) -> T::AccountId {
		let Some(new) = Self::maybe_sovereign_translate(&account)
			.or_else(|| Self::maybe_derived_translate(&account))
		else {
			return account;
		};

		log::info!(
			"Translated account: {} -> {}",
			&account.to_polkadot_ss58(),
			&new.to_polkadot_ss58()
		);

		new
	}

	/// Translate the account if its a parachain sovereign account.
	#[cfg(any(feature = "polkadot-ahm", feature = "kusama-ahm"))]
	fn maybe_sovereign_translate(account: &T::AccountId) -> Option<T::AccountId> {
		let Some(new) = crate::sovereign_account_translation::SOV_TRANSLATIONS
			.binary_search_by_key(account, |((rc_acc, _), _)| rc_acc.clone())
			.map(|i| {
				crate::sovereign_account_translation::SOV_TRANSLATIONS
					.get(i)
					.map(|(_, (ah_acc, _))| ah_acc)
					.defensive()
			})
			.ok()
			.flatten()
			.cloned()
		else {
			return None;
		};

		Self::deposit_event(crate::Event::AccountTranslatedParachainSovereign {
			from: account.clone(),
			to: new.clone(),
		});

		Some(new)
	}

	#[cfg(not(any(feature = "polkadot-ahm", feature = "kusama-ahm")))]
	fn maybe_sovereign_translate(account: &T::AccountId) -> Option<T::AccountId> {
		None
	}

	/// Translate the account if its derived from a parachain sovereign account.
	#[cfg(any(feature = "polkadot-ahm", feature = "kusama-ahm"))]
	fn maybe_derived_translate(account: &T::AccountId) -> Option<T::AccountId> {
		let Some((new, idx)) = crate::sovereign_account_translation::DERIVED_TRANSLATIONS
			.binary_search_by_key(account, |((rc_acc, _), _, _)| rc_acc.clone())
			.map(|i| {
				crate::sovereign_account_translation::DERIVED_TRANSLATIONS
					.get(i)
					.map(|(_, idx, (ah_acc, _))| (ah_acc, idx))
					.defensive()
			})
			.ok()
			.flatten()
		else {
			return None;
		};

		Self::deposit_event(crate::Event::AccountTranslatedParachainSovereignDerived {
			from: account.clone(),
			to: new.clone(),
			derivation_index: *idx,
		});

		Some(new.clone())
	}

	#[cfg(not(any(feature = "polkadot-ahm", feature = "kusama-ahm")))]
	fn maybe_derived_translate(account: &T::AccountId) -> Option<T::AccountId> {
		None
	}
}
