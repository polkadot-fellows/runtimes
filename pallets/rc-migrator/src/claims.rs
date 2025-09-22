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
use frame_support::traits::{Currency, VestingSchedule};
use pallet_claims::{EthereumAddress, StatementKind};

#[derive(
	Encode,
	DecodeWithMemTracking,
	Decode,
	Clone,
	PartialEq,
	Eq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
pub enum ClaimsStage<AccountId> {
	StorageValues,
	Claims(Option<EthereumAddress>),
	Vesting(Option<EthereumAddress>),
	Signing(Option<EthereumAddress>),
	Preclaims(Option<AccountId>),
	Finished,
}

#[derive(
	Encode,
	DecodeWithMemTracking,
	Decode,
	MaxEncodedLen,
	TypeInfo,
	RuntimeDebug,
	Clone,
	PartialEq,
	Eq,
)]
pub enum RcClaimsMessage<AccountId, Balance, BlockNumber> {
	StorageValues { total: Balance },
	Claims((EthereumAddress, Balance)),
	Vesting { who: EthereumAddress, schedule: (Balance, Balance, BlockNumber) },
	Signing((EthereumAddress, StatementKind)),
	Preclaims((AccountId, EthereumAddress)),
}
pub type RcClaimsMessageOf<T> =
	RcClaimsMessage<<T as frame_system::Config>::AccountId, BalanceOf<T>, BlockNumberFor<T>>;

pub type BalanceOf<T> =
	<CurrencyOf<T> as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type CurrencyOf<T> = <<T as pallet_claims::Config>::VestingSchedule as VestingSchedule<
	<T as frame_system::Config>::AccountId,
>>::Currency;

pub struct ClaimsMigrator<T> {
	_phantom: PhantomData<T>,
}

impl<T: Config> PalletMigration for ClaimsMigrator<T> {
	type Key = ClaimsStage<T::AccountId>;
	type Error = Error<T>;

	fn migrate_many(
		current_key: Option<Self::Key>,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<Self::Key>, Self::Error> {
		let mut inner_key = current_key.unwrap_or(ClaimsStage::StorageValues);
		let mut messages = XcmBatchAndMeter::new_from_config::<T>();

		loop {
			if weight_counter.try_consume(T::DbWeight::get().reads_writes(1, 1)).is_err() ||
				weight_counter.try_consume(messages.consume_weight()).is_err()
			{
				log::info!(
					target: LOG_TARGET,
					"RC weight limit reached at batch length {}, stopping",
					messages.len()
				);
				if messages.is_empty() {
					return Err(Error::OutOfWeight);
				} else {
					break;
				}
			}
			if T::MaxAhWeight::get()
				.any_lt(T::AhWeightInfo::receive_claims((messages.len() + 1)))
			{
				log::info!(
					target: LOG_TARGET,
					"AH weight limit reached at batch length {}, stopping",
					messages.len()
				);
				if messages.is_empty() {
					return Err(Error::OutOfWeight);
				} else {
					break;
				}
			}

			if messages.len() > MAX_ITEMS_PER_BLOCK {
				log::info!(
					target: LOG_TARGET,
					"Maximum number of items ({:?}) to migrate per block reached, current batch size: {}",
					MAX_ITEMS_PER_BLOCK,
					messages.len()
				);
				break;
			}

			if messages.batch_count() >= MAX_XCM_MSG_PER_BLOCK {
				log::info!(
					target: LOG_TARGET,
					"Reached the maximum number of batches ({:?}) allowed per block; current batch count: {}",
					MAX_XCM_MSG_PER_BLOCK,
					messages.batch_count()
				);
				break;
			}

			inner_key = match inner_key {
				ClaimsStage::StorageValues => {
					if pallet_claims::Total::<T>::exists() {
						let total = pallet_claims::Total::<T>::take();
						messages.push(RcClaimsMessage::StorageValues { total });
					} else {
						log::debug!(target: LOG_TARGET, "Not migrating empty claims::Total");
					}
					ClaimsStage::Claims(None)
				},
				ClaimsStage::Claims(address) => {
					let mut iter = match address {
						Some(address) => pallet_claims::Claims::<T>::iter_from(
							pallet_claims::Claims::<T>::hashed_key_for(address),
						),
						None => pallet_claims::Claims::<T>::iter(),
					};

					match iter.next() {
						Some((address, amount)) => {
							pallet_claims::Claims::<T>::remove(address);
							messages.push(RcClaimsMessage::Claims((address, amount)));
							ClaimsStage::Claims(Some(address))
						},
						None => ClaimsStage::Vesting(None),
					}
				},
				ClaimsStage::Vesting(address) => {
					let mut iter = match address {
						Some(address) => pallet_claims::Vesting::<T>::iter_from(
							pallet_claims::Vesting::<T>::hashed_key_for(address),
						),
						None => pallet_claims::Vesting::<T>::iter(),
					};

					match iter.next() {
						Some((address, schedule)) => {
							pallet_claims::Vesting::<T>::remove(address);
							messages.push(RcClaimsMessage::Vesting { who: address, schedule });
							ClaimsStage::Vesting(Some(address))
						},
						None => ClaimsStage::Signing(None),
					}
				},
				ClaimsStage::Signing(address) => {
					let mut iter = match address {
						Some(address) => pallet_claims::Signing::<T>::iter_from(
							pallet_claims::Signing::<T>::hashed_key_for(address),
						),
						None => pallet_claims::Signing::<T>::iter(),
					};

					match iter.next() {
						Some((address, statement)) => {
							pallet_claims::Signing::<T>::remove(address);
							messages.push(RcClaimsMessage::Signing((address, statement)));
							ClaimsStage::Signing(Some(address))
						},
						None => ClaimsStage::Preclaims(None),
					}
				},
				ClaimsStage::Preclaims(address) => {
					let mut iter = match address {
						Some(address) => pallet_claims::Preclaims::<T>::iter_from(
							pallet_claims::Preclaims::<T>::hashed_key_for(address),
						),
						None => pallet_claims::Preclaims::<T>::iter(),
					};

					match iter.next() {
						Some((address, statement)) => {
							pallet_claims::Preclaims::<T>::remove(&address);
							messages.push(RcClaimsMessage::Preclaims((address.clone(), statement)));
							ClaimsStage::Preclaims(Some(address))
						},
						None => ClaimsStage::Finished,
					}
				},
				ClaimsStage::Finished => {
					break;
				},
			}
		}

		if !messages.is_empty() {
			Pallet::<T>::send_chunked_xcm_and_track(messages, |messages| types::AhMigratorCall::<
				T,
			>::ReceiveClaimsMessages {
				messages,
			})?;
		}

		if inner_key == ClaimsStage::Finished {
			Ok(None)
		} else {
			Ok(Some(inner_key))
		}
	}
}

#[cfg(feature = "std")]
impl<T: Config> crate::types::RcMigrationCheck for ClaimsMigrator<T> {
	type RcPrePayload = Vec<RcClaimsMessageOf<T>>;

	fn pre_check() -> Self::RcPrePayload {
		let mut messages = Vec::new();

		// Collect StorageValues
		let total = pallet_claims::Total::<T>::get();
		messages.push(RcClaimsMessage::StorageValues { total });

		// Collect Claims
		for (address, amount) in pallet_claims::Claims::<T>::iter() {
			messages.push(RcClaimsMessage::Claims((address, amount)));
		}

		// Collect Vesting
		for (address, schedule) in pallet_claims::Vesting::<T>::iter() {
			messages.push(RcClaimsMessage::Vesting { who: address, schedule });
		}

		// Collect Signing
		for (address, statement) in pallet_claims::Signing::<T>::iter() {
			messages.push(RcClaimsMessage::Signing((address, statement)));
		}

		// Collect Preclaims
		for (account_id, address) in pallet_claims::Preclaims::<T>::iter() {
			messages.push(RcClaimsMessage::Preclaims((account_id, address)));
		}

		messages
	}

	fn post_check(_: Self::RcPrePayload) {
		assert!(
			!pallet_claims::Total::<T>::exists(),
			"Assert storage 'Claims::Total::rc_post::empty'"
		);
		assert!(
			pallet_claims::Claims::<T>::iter().next().is_none(),
			"Assert storage 'Claims::Claims::rc_post::empty'"
		);
		assert!(
			pallet_claims::Vesting::<T>::iter().next().is_none(),
			"Assert storage 'Claims::Vesting::rc_post::empty'"
		);
		assert!(
			pallet_claims::Signing::<T>::iter().next().is_none(),
			"Assert storage 'Claims::Signing::rc_post::empty'"
		);
		assert!(
			pallet_claims::Preclaims::<T>::iter().next().is_none(),
			"Assert storage 'Claims::Preclaims::rc_post::empty'"
		);

		log::info!("All claims data successfully migrated and cleared from the Relay Chain.");
	}
}
