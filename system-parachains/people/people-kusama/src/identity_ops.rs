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

	/// Weight information for extrinsics in this pallet.
	pub trait WeightInfo {
		/// Weight for clearing judgement.
		fn clear_judgement() -> Weight;
	}

	type IdentityPallet = pallet_identity::Pallet<Runtime>;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// No judgement to clear.
		NotFound,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The invalid judgements have been cleared.
		JudgementsCleared { target: AccountId },
	}

	pub(crate) type Identity = (
		pallet_identity::Registration<
			<Runtime as pallet_balances::Config>::Balance,
			<Runtime as pallet_identity::Config>::MaxRegistrars,
			<Runtime as pallet_identity::Config>::IdentityInformation,
		>,
		Option<BoundedVec<u8, <Runtime as pallet_identity::Config>::MaxUsernameLength>>,
	);

	/// Alias for `IdentityOf` from `pallet_identity`.
	#[frame_support::storage_alias(pallet_name)]
	pub(crate) type IdentityOf =
		StorageMap<IdentityPallet, Twox64Concat, AccountId, Identity, OptionQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Clear requested judgements that do not have a corresponding deposit reserved.
		///
		/// This is successful only if the `target` account has judgements to clear. The transaction
		/// fee is refunded to the caller if successful.
		#[pallet::call_index(0)]
		#[pallet::weight(weights::pallet_identity_ops::WeightInfo::<Runtime>::clear_judgement())]
		pub fn clear_judgement(
			_origin: OriginFor<T>,
			target: AccountId,
		) -> DispatchResultWithPostInfo {
			let identity = IdentityPallet::identity(&target).ok_or(Error::<T>::NotFound)?;
			let (removed, identity) = Self::do_clear_judgement(&target, identity);
			ensure!(removed > 0, Error::<T>::NotFound);

			IdentityOf::insert(&target, identity);

			Self::deposit_event(Event::JudgementsCleared { target });

			Ok(Pays::No.into())
		}
	}

	impl<T: Config> Pallet<T> {
		fn do_clear_judgement(account_id: &AccountId, mut identity: Identity) -> (u32, Identity) {
			// `subs_of`'s query kind is value option, if non subs the deposit is zero.
			let (subs_deposit, _) = IdentityPallet::subs_of(account_id);
			// deposit without deposits for judgement request.
			let identity_deposit = identity.0.deposit.saturating_add(subs_deposit);
			// total reserved balance.
			let reserved = Balances::reserved_balance(account_id);
			// expected deposit with judgement deposits.
			let mut expected_total_deposit = identity_deposit;
			// count before cleaning up the judgements.
			let judgements_count = identity.0.judgements.len();

			identity.0.judgements.retain(|(_, judgement)| {
				if let Judgement::FeePaid(deposit) = judgement {
					expected_total_deposit = expected_total_deposit.saturating_add(*deposit);
					reserved >= expected_total_deposit && *deposit > 0
				} else {
					true
				}
			});

			((judgements_count - identity.0.judgements.len()) as u32, identity)
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		#[cfg(feature = "try-runtime")]
		fn try_state(_: BlockNumberFor<T>) -> Result<(), sp_runtime::TryRuntimeError> {
			use crate::Balance;
			use sp_core::crypto::Ss58Codec;

			let mut total_invalid_identity_count = 0;
			let mut total_invalid_judgement_count = 0;
			let mut identity_count = 0;
			IdentityOf::iter().for_each(|(account_id, registration)| {
				let (identity, username) = registration;

				let (subs_deposit, _) = IdentityPallet::subs_of(&account_id);
				let deposit_wo_judgement = identity.deposit.saturating_add(subs_deposit);
				let reserved = Balances::reserved_balance(&account_id);

				let (invalid_judgement_count, expected_total_deposit) =
					identity.judgements.iter().fold(
						(0, deposit_wo_judgement),
						|(count, total_deposit): (u32, Balance), (_, judgement)| {
							if let Judgement::FeePaid(deposit) = judgement {
								if total_deposit.saturating_add(*deposit) > reserved ||
									*deposit == 0
								{
									return (count + 1, total_deposit.saturating_add(*deposit))
								}
							}
							(count, total_deposit)
						},
					);

				if expected_total_deposit >= reserved && invalid_judgement_count > 0 {
					total_invalid_identity_count += 1;
					total_invalid_judgement_count += invalid_judgement_count;

					log::info!(
						"account with invalid state: {:?}, expected reserve at least: {:?}, actual: {:?}, invalid judgements: {:?}",
						account_id.clone().to_ss58check_with_version(2u8.into()),
						expected_total_deposit,
						reserved,
						invalid_judgement_count,
					);

					assert_eq!(
						invalid_judgement_count,
						Self::do_clear_judgement(&account_id, (identity, username)).0
					);

					if deposit_wo_judgement != reserved {
						log::warn!(
							"unexpected state: {:?}, deposit w/o judgement: {:?}, not equal to the total reserved: {:?}",
							account_id.clone().to_ss58check_with_version(2u8.into()),
							deposit_wo_judgement,
							reserved,
						);
					}
				} else {
					assert_eq!(0, Self::do_clear_judgement(&account_id, (identity, username)).0);
				}
				if deposit_wo_judgement > reserved {
					log::warn!(
						"unexpected state: {:?}, deposit w/o judgement: {:?}, greater than the total reserved: {:?}",
						account_id.clone().to_ss58check_with_version(2u8.into()),
						deposit_wo_judgement,
						reserved,
					);
				}
				identity_count += 1;
			});

			log::info!("total identities processed: {:?}", identity_count);
			log::info!("invalid identities: {:?}", total_invalid_identity_count);
			log::info!("invalid judgements: {:?}", total_invalid_judgement_count);

			Ok(())
		}
	}
}

#[cfg(feature = "runtime-benchmarks")]
#[frame_benchmarking::v2::benchmarks(where T: pallet_identity_ops::Config)]
mod benchmarks {
	use crate::{people::IdentityInfo, *};
	use frame_benchmarking::BenchmarkError;
	use frame_system::RawOrigin;
	use pallet_identity::{IdentityInformationProvider, Judgement};
	use pallet_identity_ops::{Event, Identity, *};
	use parachains_common::{AccountId, Balance};
	use sp_core::Get;
	use sp_runtime::traits::One;

	#[benchmark]
	fn clear_judgement() -> Result<(), BenchmarkError> {
		let max_registrars =
			<<Runtime as pallet_identity::Config>::MaxRegistrars as Get<u32>>::get();
		let mut judgements = Vec::<(u32, Judgement<Balance>)>::new();
		for i in 0..max_registrars {
			judgements.push((i, Judgement::FeePaid(Balance::one())));
		}
		let identity: Identity = (
			pallet_identity::Registration {
				deposit: Balance::one(),
				judgements: judgements.try_into().unwrap(),
				info: IdentityInfo::create_identity_info(),
			},
			None,
		);

		let target: AccountId = [1u8; 32].into();

		IdentityOf::insert(&target, identity);

		#[extrinsic_call]
		_(RawOrigin::None, target.clone());

		crate::System::assert_last_event(Event::<Runtime>::JudgementsCleared { target }.into());

		Ok(())
	}
}
