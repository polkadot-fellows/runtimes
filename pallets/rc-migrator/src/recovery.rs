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

use crate::*;
use crate::types::DefensiveTruncateInto;
use crate::types::TranslateAccounts;

/// Hard-code the number of max friends in Kusama for simplicity.
pub const MAX_FRIENDS: u32 = 9;

#[derive(Encode, DecodeWithMemTracking, Decode, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen, PartialEq, Eq)]
pub enum RecoveryStage {
	Recoverable(Option<AccountId32>),
	ActiveRecoveries(Option<(AccountId32, AccountId32)>),
	Proxy(Option<AccountId32>),
	Finished,
}

#[derive(Encode, DecodeWithMemTracking, Decode, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen, PartialEq, Eq)]
pub enum PortableRecoveryMessage {
	Recoverable((AccountId32, PortableRecoveryConfig)),
	ActiveRecoveries((AccountId32, AccountId32, PortableActiveRecovery)),
	Proxy((AccountId32, AccountId32)),
}

#[derive(Encode, DecodeWithMemTracking, Decode, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen, PartialEq, Eq)]
pub struct PortableRecoveryConfig {
	pub delay_period: u32,
	pub deposit: u128,
	pub friends: PortableRecoveryFriends,
	pub threshold: u16,
}

#[derive(Encode, DecodeWithMemTracking, Decode, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen, PartialEq, Eq)]
pub struct PortableActiveRecovery {
	pub created: u32,
	pub deposit: u128,
	pub friends: PortableRecoveryFriends,
}

#[derive(Encode, DecodeWithMemTracking, Decode, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen, PartialEq, Eq)]
pub struct PortableRecoveryFriends {
	pub friends: BoundedVec<AccountId32, ConstU32<MAX_FRIENDS>>,
}

// Acc Translation
impl TranslateAccounts for PortableRecoveryMessage {
	fn translate_accounts(self, f: impl Fn(AccountId32) -> AccountId32) -> Self {
		match self {
			PortableRecoveryMessage::Recoverable((who, config)) => {
				PortableRecoveryMessage::Recoverable((f(who), config.translate_accounts(f)))
			},
			PortableRecoveryMessage::ActiveRecoveries((w1, w2, config)) => {
				PortableRecoveryMessage::ActiveRecoveries((f(w1), f(w2), config.translate_accounts(f)))
			},
			PortableRecoveryMessage::Proxy((w1, w2)) => {
				PortableRecoveryMessage::Proxy((f(w1), f(w2)))
			},
		}
	}
}

// RC -> Portable
impl IntoPortable for pallet_recovery::RecoveryConfig<u32, u128, BoundedVec<AccountId32, ConstU32<MAX_FRIENDS>>> {
	type Portable = PortableRecoveryConfig;

	fn into_portable(self) -> Self::Portable {
		PortableRecoveryConfig {
			delay_period: self.delay_period,
			deposit: self.deposit,
			friends: self.friends.into_portable(),
			threshold: self.threshold,
		}
	}
}

// Acc Translation
impl TranslateAccounts for PortableRecoveryConfig {
	fn translate_accounts(self, f: impl Fn(AccountId32) -> AccountId32) -> Self {
		Self {
			friends: self.friends.translate_accounts(f),
			..self
		}
	}
}

// Portable -> AH
impl Into<pallet_recovery::RecoveryConfig<u32, u128, BoundedVec<AccountId32, ConstU32<MAX_FRIENDS>>>> for PortableRecoveryConfig {
	fn into(self) -> pallet_recovery::RecoveryConfig<u32, u128, BoundedVec<AccountId32, ConstU32<MAX_FRIENDS>>> {
		pallet_recovery::RecoveryConfig {
			delay_period: self.delay_period,
			deposit: self.deposit,
			friends: self.friends.into(),
			threshold: self.threshold,
		}
	}
}

// Acc Translation
impl TranslateAccounts for PortableActiveRecovery {
	fn translate_accounts(self, f: impl Fn(AccountId32) -> AccountId32) -> Self {
		Self {
			friends: self.friends.translate_accounts(f),
			..self
		}
	}
}

// RC -> Portable
impl IntoPortable for pallet_recovery::ActiveRecovery<u32, u128, BoundedVec<AccountId32, ConstU32<MAX_FRIENDS>>> {
	type Portable = PortableActiveRecovery;

	fn into_portable(self) -> Self::Portable {
		PortableActiveRecovery {
			created: self.created,
			deposit: self.deposit,
			friends: self.friends.into_portable(),
		}
	}
}

// Portable -> AH
impl Into<pallet_recovery::ActiveRecovery<u32, u128, BoundedVec<AccountId32, ConstU32<MAX_FRIENDS>>>> for PortableActiveRecovery {
	fn into(self) -> pallet_recovery::ActiveRecovery<u32, u128, BoundedVec<AccountId32, ConstU32<MAX_FRIENDS>>> {
		pallet_recovery::ActiveRecovery {
			created: self.created,
			deposit: self.deposit,
			friends: self.friends.into(),
		}
	}
}

// RC -> Portable
impl IntoPortable for BoundedVec<AccountId32, ConstU32<MAX_FRIENDS>>{
	type Portable = PortableRecoveryFriends;

	fn into_portable(self) -> Self::Portable {
		PortableRecoveryFriends { friends: self }
	}
}

// Acc Translation
impl TranslateAccounts for PortableRecoveryFriends {
	fn translate_accounts(self, f: impl Fn(AccountId32) -> AccountId32) -> Self {
		Self { friends: self.friends.into_iter().map(f).collect::<Vec<_>>().defensive_truncate_into() } // TODO @ggwpez iter_mut?
	}
}

// Portable -> AH
impl Into<BoundedVec<AccountId32, ConstU32<MAX_FRIENDS>>> for PortableRecoveryFriends {
	fn into(self) -> BoundedVec<AccountId32, ConstU32<MAX_FRIENDS>> {
		self.friends
	}
}

pub struct RecoveryMigrator<T> {
	_phantom: sp_std::marker::PhantomData<T>,
}

impl<T: Config> PalletMigration for RecoveryMigrator<T> {
	type Key = RecoveryStage;
	type Error = Error<T>;

	fn migrate_many(last_key: Option<Self::Key>, weight_counter: &mut WeightMeter) -> Result<Option<Self::Key>, Self::Error> {
		let mut last_key = last_key.unwrap_or(RecoveryStage::Recoverable(None));
		let mut messages = XcmBatchAndMeter::new_from_config::<T>();

		loop {
			if weight_counter.try_consume(T::DbWeight::get().reads_writes(1, 1)).is_err() ||
				weight_counter.try_consume(messages.consume_weight()).is_err()
			{
				log::info!("RC weight limit reached at batch length {}, stopping", messages.len());
				if messages.is_empty() {
					return Err(Error::OutOfWeight);
				} else {
					break;
				}
			}

			// TODO @ggwpez weight
			if T::MaxAhWeight::get()
				.any_lt(T::AhWeightInfo::receive_vesting_schedules((messages.len() + 1) as u32))
			{
				log::info!("AH weight limit reached at batch length {}, stopping", messages.len());
				if messages.is_empty() {
					return Err(Error::OutOfWeight);
				} else {
					break;
				}
			}

			if messages.len() > MAX_ITEMS_PER_BLOCK {
				log::info!(
					"Maximum number of items ({:?}) to migrate per block reached, current batch size: {}",
					MAX_ITEMS_PER_BLOCK,
					messages.len()
				);
				break;
			}

			last_key = match last_key {
				RecoveryStage::Recoverable(last_key) => {
					let mut iter = match last_key {
						Some(last_key) => pallet_recovery::Recoverable::<T::KusamaConfig>::iter_from_key(last_key),
						None => pallet_recovery::Recoverable::<T::KusamaConfig>::iter(),
					};

					match iter.next() {
						Some((who, config)) => {
							pallet_recovery::Recoverable::<T::KusamaConfig>::remove(&who);
							messages.push(PortableRecoveryMessage::Recoverable((who.clone(), config.into_portable())));
							RecoveryStage::Recoverable(Some(who))
						},
						None => RecoveryStage::ActiveRecoveries(None),
					}
				}
				RecoveryStage::ActiveRecoveries(last_key) => {
					let mut iter = match last_key {
						Some((w1, w2)) => pallet_recovery::ActiveRecoveries::<T::KusamaConfig>::iter_from(
							pallet_recovery::ActiveRecoveries::<T::KusamaConfig>::hashed_key_for(w1, w2),
						),
						None => pallet_recovery::ActiveRecoveries::<T::KusamaConfig>::iter(),
					};

					match iter.next() {
						Some((w1, w2, config)) => {
							pallet_recovery::ActiveRecoveries::<T::KusamaConfig>::remove(&w1, &w2);
							messages.push(PortableRecoveryMessage::ActiveRecoveries((w1.clone(), w2.clone(), config.into_portable())));
							RecoveryStage::ActiveRecoveries(Some((w1, w2)))
						},
						None => RecoveryStage::Proxy(None),
					}
				}
				RecoveryStage::Proxy(last_key) => {
					let mut iter = match last_key {
						Some(last_key) => pallet_recovery::Proxy::<T::KusamaConfig>::iter_from_key(last_key),
						None => pallet_recovery::Proxy::<T::KusamaConfig>::iter(),
					};

					match iter.next() {
						Some((w1, w2)) => {
							pallet_recovery::Proxy::<T::KusamaConfig>::remove(&w1);
							messages.push(PortableRecoveryMessage::Proxy((w1.clone(), w2.clone())));
							RecoveryStage::Proxy(Some(w1))
						},
						None => RecoveryStage::Finished,
					}
				},
				RecoveryStage::Finished => {
					break;
				},
			}
		}

		if !messages.is_empty() {
			Pallet::<T>::send_chunked_xcm_and_track(messages, |messages| {
				types::AhMigratorCall::ReceiveRecoveryMessages { messages }
			})?;
		}

		if last_key == RecoveryStage::Finished {
			Ok(None)
		} else {
			Ok(Some(last_key))
		}
	}
}
