// Copyright (c) 2023 Encointer Association
// This file is part of Encointer
//
// Encointer is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Encointer is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Encointer.  If not, see <http://www.gnu.org/licenses/>.

// the following are temporary local migration fixes to solve inconsistencies caused by not
// migrating Storage at the time of migrating runtime code

pub mod collator_selection_init {
	use frame_support::traits::OnRuntimeUpgrade;
	#[cfg(feature = "try-runtime")]
	use sp_runtime::TryRuntimeError;

	/// The log target.
	const TARGET: &'static str = "runtime::fix::collator_selection_init";
	pub mod v0 {
		use super::*;
		use crate::SessionKeys;
		use codec::EncodeLike;
		use frame_support::{pallet_prelude::*, traits::Currency};
		use hex_literal::hex;
		use log::info;
		use parachains_common::impls::BalanceOf;
		use sp_core::{crypto::key_types::AURA, sr25519};
		use sp_std::{vec, vec::Vec};

		const INVULNERABLE_A: [u8; 32] =
			hex!("5e962096da68302d5c47fce0178d72fab503c4f00a3f1df64856748f0d9dd51e");
		const INVULNERABLE_B: [u8; 32] =
			hex!("0cecb8d1c2c744ca4c5cea57f5d6c40238f4dad17afa213672b8b7d43b80a659");
		const INVULNERABLE_C: [u8; 32] =
			hex!("ca1951a3c4e100fb5a899e7bae3ea124491930a72000c5e4b2775fea27ecf05d");
		const INVULNERABLE_D: [u8; 32] =
			hex!("484b443bd95068b860c92b0f66487b78f58234eca0f88e2adbe80bae4807b809");
		const INVULNERABLE_E: [u8; 32] =
			hex!("6c642fb4b571a5685a869cd291fafd575be47a918b231ba28165e5c0cd0cfa15");

		pub struct InitInvulnerables<T>(sp_std::marker::PhantomData<T>);
		impl<T> OnRuntimeUpgrade for InitInvulnerables<T>
		where
			T: frame_system::Config
				+ pallet_collator_selection::Config
				+ pallet_session::Config
				+ pallet_balances::Config,
			<T as frame_system::Config>::AccountId: From<[u8; 32]>,
			<T as pallet_session::Config>::ValidatorId: From<[u8; 32]>,
			<T as pallet_session::Config>::Keys: From<SessionKeys>,
			<T as pallet_balances::Config>::Balance: From<u128>,
			<T as pallet_balances::Config>::Balance: EncodeLike<
				<<T as pallet_collator_selection::Config>::Currency as Currency<
					<T as frame_system::Config>::AccountId,
				>>::Balance,
			>,
		{
			#[cfg(feature = "try-runtime")]
			fn pre_upgrade() -> Result<sp_std::vec::Vec<u8>, TryRuntimeError> {
				let invulnerables_len = pallet_collator_selection::Invulnerables::<T>::get().len();
				ensure!(
					invulnerables_len == 0,
					"this migration has been executed previously. won't apply it again"
				);
				Ok(().encode())
			}

			fn on_runtime_upgrade() -> Weight {
				info!(target: TARGET, "initializing the set of invulnerables");

				let raw_keys: Vec<[u8; 32]> = vec![
					INVULNERABLE_A,
					INVULNERABLE_B,
					INVULNERABLE_C,
					INVULNERABLE_D,
					INVULNERABLE_E,
				];

				let validatorids: Vec<<T as pallet_session::Config>::ValidatorId> =
					raw_keys.iter().map(|&pk| pk.into()).collect();

				pallet_session::Validators::<T>::put(validatorids);

				let queued_keys: Vec<(
					<T as pallet_session::Config>::ValidatorId,
					<T as pallet_session::Config>::Keys,
				)> = raw_keys
					.iter()
					.map(|&pk| {
						(
							pk.into(),
							SessionKeys { aura: sr25519::Public::from_raw(pk).into() }.into(),
						)
					})
					.collect();

				pallet_session::QueuedKeys::<T>::put(queued_keys);

				for pk in raw_keys.clone() {
					pallet_session::NextKeys::<T>::insert::<
						<T as pallet_session::Config>::ValidatorId,
						<T as pallet_session::Config>::Keys,
					>(pk.into(), SessionKeys { aura: sr25519::Public::from_raw(pk).into() }.into());
					pallet_session::KeyOwner::<T>::insert::<
						_,
						<T as pallet_session::Config>::ValidatorId,
					>((AURA, pk.encode()), pk.into());
				}

				let mut invulnerables: Vec<<T as frame_system::Config>::AccountId> =
					raw_keys.iter().map(|&pk| pk.into()).collect();
				invulnerables.sort();
				let invulnerables: BoundedVec<_, T::MaxInvulnerables> =
					invulnerables.try_into().unwrap();
				pallet_collator_selection::Invulnerables::<T>::put(invulnerables);

				pallet_collator_selection::CandidacyBond::<T>::put::<BalanceOf<T>>(
					5_000_000_000_000u128.into(),
				);

				T::DbWeight::get().reads_writes(0, 4 + 5 * 2)
			}

			#[cfg(feature = "try-runtime")]
			fn post_upgrade(_state: sp_std::vec::Vec<u8>) -> Result<(), TryRuntimeError> {
				let invulnerables_len = pallet_collator_selection::Invulnerables::<T>::get().len();
				ensure!(
					invulnerables_len > 0,
					"invulnerables are empty after initialization. that should not happen"
				);
				Ok(())
			}
		}
	}
}
