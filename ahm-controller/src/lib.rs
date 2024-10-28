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

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;

use frame_support::storage::TransactionOutcome;
use xcm::{
	prelude::{send_xcm, Instruction, Junction, Location, OriginKind, SendXcm, WeightLimit, Xcm},
	v4::{
		Asset,
		AssetFilter::Wild,
		AssetId, Assets, Error as XcmError,
		Fungibility::Fungible,
		Instruction::{DepositAsset, ReceiveTeleportedAsset},
		Junctions::Here,
		Reanchorable,
		WildAsset::AllCounted,
		XcmContext,
	},
};
use frame_support::weights::constants::WEIGHT_REF_TIME_PER_MILLIS;

pub use pallet::*;

#[cfg(test)]
mod mock_relay;

#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq, Eq)]
pub enum Role {
	Relay,
	AssetHub,
}

#[derive(Encode, Decode, MaxEncodedLen, TypeInfo)]
pub enum Phase {
	MigrateBalancesEds { next_account: Option<BoundedVec<u8, ConstU32<128>>> },

	AllDone,
}

#[derive(Encode, Decode)]
enum AssetHubPalletConfig<T: Config> {
	#[codec(index = 244)]
	AhmController(AhmCall),
	#[codec(index = 5)]
	Indices(pallet_indices::Call<T>),
}

/// Call encoding for the calls needed from the Broker pallet.
#[derive(Encode, Decode)]
enum AhmCall {
	#[codec(index = 0)]
	EmitHey,
}

/*#[derive(Encode, Decode)]
enum AhIndicesCall<T: Config> {
	#[codec(index = 5)]
	MigrateInNext(<T as pallet_indices::Config>::AccountIndex, T::AccountId, BalanceOf<T>, bool),
}*/

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_indices::Config + pallet_balances::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		#[pallet::constant]
		type Role: Get<Role>;

		/// Send UMP or DMP message - depending on our `Role`.
		type SendXcm: SendXcm;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	pub type Phase<T: Config> = StorageValue<_, super::Phase>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Hey,
		
		SentDownward,
		ErrorSendingDownward,

		PalletIndicesFinished,
	}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			match T::Role::get() {
				Role::Relay => {
					Self::relay_on_init();
				},
				Role::AssetHub => {
				},
			};

			Weight::zero()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(123)]
		pub fn hey(_origin: OriginFor<T>) -> DispatchResult {
			Self::deposit_event(Event::Hey);

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn relay_on_init() {
			// Phase init
			let phase = match Phase::<T>::get() {
				None => Phase::MigrateBalancesEds { next_account: Vec::new() },
				Some(phase) => phase,
			};

			// Phase handling and transistion
			let phase = match phase {
				Phase::MigrateBalancesEds { mut next_account } => {
					Self::migrate_eds(&mut next_account, 100)?;

					if next_account.is_some() {
						Phase::MigrateBalancesEds { next_account }
					} else {
						Phase::AllDone
					}
				},
				other => other,
			};
			// Write back
			Phase::<T>::put(phase);

			/*for _ in 0..10 {
				match Self::migrate_indices() {
					Ok(false) => break,
					Err(_) => break,
					_ => (),
				}
			}*/
		}

		fn migrate_eds(next_acc: &mut Option<Vec<u8>>) -> Result<(), ()> {
			frame_support::storage::transactional::with_transaction_opaque_err::<(), (), _>(|| {
				let Some((call, weight)) = pallet_balances::Pallet::<T>::migrate_ed(&mut next_acc, 100) else {
					return TransactionOutcome::Commit(Ok(()));
				};
				
				let ah_call: xcm::DoubleEncoded<()> = AssetHubPalletConfig::<T>::Indices(
					call,
				).encode().into();

				let message = Xcm(vec![
					Instruction::UnpaidExecution {
						weight_limit: WeightLimit::Unlimited,
						check_origin: None,
					},
					Instruction::Transact {
						origin_kind: OriginKind::Superuser,
						require_weight_at_most: Weight::from_parts(10 * WEIGHT_REF_TIME_PER_MILLIS, 30000), // TODO
						call: ah_call,
					},
				]);

				match send_xcm::<T::SendXcm>(
					Location::new(0, [Junction::Parachain(1000)]),
					message.clone(),
				) {
					Ok(_) => {
						Self::deposit_event(Event::SentDownward);
						TransactionOutcome::Commit(Ok(()))
					},
					Err(_) => {
						Self::deposit_event(Event::ErrorSendingDownward);
						TransactionOutcome::Commit(Err(()))
					},
				}
			})?
		}

		fn migrate_indices() -> Result<bool, ()> {
			frame_support::storage::transactional::with_transaction_opaque_err::<bool, (), _>(|| {
				let Some((call, weight)) = pallet_indices::Pallet::<T>::migrate_next(100) else {
					return TransactionOutcome::Commit(Ok(false));
				};
				
				let ah_call: xcm::DoubleEncoded<()> = AssetHubPalletConfig::<T>::Indices(
					call,
				).encode().into();

				let message = Xcm(vec![
					Instruction::UnpaidExecution {
						weight_limit: WeightLimit::Unlimited,
						check_origin: None,
					},
					Instruction::Transact {
						origin_kind: OriginKind::Superuser,
						require_weight_at_most: Weight::from_parts(10 * WEIGHT_REF_TIME_PER_MILLIS, 30000), // TODO
						call: ah_call,
					},
				]);

				match send_xcm::<T::SendXcm>(
					Location::new(0, [Junction::Parachain(1000)]),
					message.clone(),
				) {
					Ok(_) => {
						Self::deposit_event(Event::SentDownward);
						TransactionOutcome::Commit(Ok(true))
					},
					Err(_) => {
						Self::deposit_event(Event::ErrorSendingDownward);
						TransactionOutcome::Commit(Err(()))
					},
				}
			})?
		}
	}
}
