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
	Waiting,
}

#[derive(Encode, Decode)]
enum AssetHubPalletConfig {
	#[codec(index = 244)]
	AhmController(AhmCall),
}

/// Call encoding for the calls needed from the Broker pallet.
#[derive(Encode, Decode)]
enum AhmCall {
	#[codec(index = 0)]
	EmitHey,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
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
		ErrorSending,
	}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			if T::Role::get() != Role::Relay {
				return Weight::zero();
			}

			let message = Xcm(vec![
				Instruction::UnpaidExecution {
					weight_limit: WeightLimit::Unlimited,
					check_origin: None,
				},
				Instruction::Transact {
					origin_kind: OriginKind::Superuser,
					require_weight_at_most: Weight::from_parts(10 * WEIGHT_REF_TIME_PER_MILLIS, 30000), // TODO
					call: AssetHubPalletConfig::AhmController(AhmCall::EmitHey).encode().into(),
				}
			]);

			for _ in 0..100 {
				match send_xcm::<T::SendXcm>(
					Location::new(0, [Junction::Parachain(1000)]),
					message.clone(),
				) {
					Ok(_) => {
						Self::deposit_event(Event::SentDownward);
					},
					Err(_) => {
						Self::deposit_event(Event::ErrorSending);
					},
				}
			}

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
}
