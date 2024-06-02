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

#[frame_support::pallet]
pub mod pallet_identity_ops {
	use crate::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use pallet_identity::Judgement;

	type IdentityPallet = pallet_identity::Pallet<Runtime>;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::error]
	pub enum Error<T> {
		/// No judgement to clear.
		NotFound,
	}

	#[frame_support::storage_alias(pallet_name)]
	pub type IdentityMap = StorageMap<
		IdentityPallet,
		Twox64Concat,
		AccountId,
		(
			pallet_identity::Registration<
				<Runtime as pallet_balances::Config>::Balance,
				<Runtime as pallet_identity::Config>::MaxRegistrars,
				<Runtime as pallet_identity::Config>::IdentityInformation,
			>,
			Option<Vec<u8>>,
		),
		OptionQuery,
	>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Clear requested judgements that do not have a corresponding deposit reserved.
		///
		/// This is successful only if the `target` account has judgements to clear. The transaction
		/// fee is refunded to the caller if successful.
		#[pallet::call_index(0)]
		#[pallet::weight(Pallet::<T>::weight_clear_judgement())]
		pub fn clear_judgement(
			_origin: OriginFor<T>,
			target: AccountId,
		) -> DispatchResultWithPostInfo {
			let removed = Self::do_clear_judgement(&target);
			ensure!(removed > 0, Error::<T>::NotFound);

			Ok(Pays::No.into())
		}
	}

	impl<T: Config> Pallet<T> {
		fn do_clear_judgement(target: &AccountId) -> u32 {
			let Some((mut identity, _)) = IdentityPallet::identity(target) else {
				return 0;
			};
			// `subs_of`'s query kind is value option, if non subs the deposit is zero.
			let (subs_deposit, _) = IdentityPallet::subs_of(target);
			// deposit without deposits for judgement request.
			let identity_deposit = identity.deposit.saturating_add(subs_deposit);
			// total reserved balance.
			let reserved = Balances::reserved_balance(target);
			// expected deposit with judgement deposits.
			let mut expected_total_deposit = identity_deposit;

			let judgements_count = identity.judgements.len();

			identity.judgements.retain(|(_, judgement)| {
				if let Judgement::FeePaid(deposit) = judgement {
					expected_total_deposit = expected_total_deposit.saturating_add(*deposit);
					reserved >= expected_total_deposit
				} else {
					true
				}
			});

			(judgements_count - identity.judgements.len()) as u32
		}

		/// Weight calculation for the worst-case scenario of `clear_judgement`.
		/// Equal to 20 registrars reads/writes + 1 identity read + 1 subs read + 1 reserve
		/// balance read.
		fn weight_clear_judgement() -> Weight {
			let max_registrars =
				<<Runtime as pallet_identity::Config>::MaxRegistrars as Get<u32>>::get();
			<Runtime as frame_system::Config>::DbWeight::get()
				.reads_writes((max_registrars + 3).into(), max_registrars.into())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		#[cfg(feature = "try-runtime")]
		fn try_state(_: BlockNumberFor<T>) -> Result<(), sp_runtime::TryRuntimeError> {
			use crate::Balance;
			use sp_core::crypto::Ss58Codec;
			use sp_runtime::traits::Zero;

			let mut invalid_identity_count = 0;
			let mut invalid_judgement_count = 0;
			let mut identity_count = 0;
			IdentityMap::iter().for_each(|(account_id, _)| {
				let (identity, _) = IdentityPallet::identity(&account_id).unwrap();

				let (paid_judgement_count, judgement_deposit) = identity.judgements.iter().fold(
					(0, Zero::zero()),
					|(count, total_deposit): (u32, Balance), (_, judgement)| {
						if let Judgement::FeePaid(deposit) = judgement {
							(count + 1, total_deposit.saturating_add(*deposit))
						} else {
							(count, total_deposit)
						}
					},
				);

				let (subs_deposit, _) = IdentityPallet::subs_of(&account_id);

				let deposit =
					identity.deposit.saturating_add(judgement_deposit).saturating_add(subs_deposit);
				let deposit_wo_judgement = identity.deposit.saturating_add(subs_deposit);
				let reserved = Balances::reserved_balance(&account_id);

				if deposit > reserved && paid_judgement_count > 0 {
					invalid_identity_count += 1;
					invalid_judgement_count += paid_judgement_count;

					log::info!(
						"account with invalid state: {:?}, expected reserve at least: {:?}, actual: {:?}",
						account_id.clone().to_ss58check(),
						deposit,
						reserved,
					);

					assert_eq!(paid_judgement_count, Self::do_clear_judgement(&account_id));

					if deposit_wo_judgement != reserved {
						log::warn!(
							"unexpected state: {:?}, deposit w/o judgement: {:?}, not equal to the total reserved: {:?}",
							account_id.clone().to_ss58check(),
							deposit_wo_judgement,
							reserved,
						);
					}
				} else {
					assert_eq!(0, Self::do_clear_judgement(&account_id));
				}
				if deposit_wo_judgement > reserved {
					log::warn!(
						"unexpected state: {:?}, deposit w/o judgement: {:?}, greater than the total reserved: {:?}",
						account_id.clone().to_ss58check(),
						deposit_wo_judgement,
						reserved,
					);
				}
				identity_count += 1;
			});

			log::info!("total identities processed: {:?}", identity_count);
			log::info!("invalid identities: {:?}", invalid_identity_count);
			log::info!("invalid judgements: {:?}", invalid_judgement_count);

			Ok(())
		}
	}
}
