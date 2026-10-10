// Copyright (C) Polkadot Fellows.
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
// along with Polkadot. If not, see <http://www.gnu.org/licenses/>.

//! Wire types of the AHM v2 migration: what `pallet-rc2-migrator` sends and the receiving
//! chains' migrator pallets decode.
//!
//! They live in their own crate so that a receiving runtime does not have to depend on the
//! Relay Chain migrator pallet to speak the format. A runtime declares what it can represent
//! with `From` / `TryFrom` impls on these types.
//!
//! # Differences from AHM v1
//!
//! In v1 these types lived in `pallet_rc_migrator` (`types.rs`) and `pallet_ah_migrator` depended
//! on that crate, which was ok as AH had all the same pallets that were migrating from RC.
//! The Coretime chain must not depend on `runtime-parachains`, so for AHMv2, we extract a separate
//! types crate.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use polkadot_parachain_primitives::primitives::{Id as ParaId, Sibling};
use scale_info::TypeInfo;
use sp_runtime::{traits::AccountIdConversion, AccountId32};

/// Sovereign account of `para_id` as seen from a sibling parachain (`sibl` + para id).
///
/// Same derivation as `SiblingParachainConvertsVia` in the receiving runtime's XCM config. The
/// Relay Chain sends deposits to this account and the receiving chain looks them up on it, so
/// both sides must derive it identically.
pub fn sibling_account<AccountId>(para_id: u32) -> AccountId
where
	Sibling: AccountIdConversion<AccountId>,
{
	Sibling::from(ParaId::from(para_id)).into_account_truncating()
}

/// Account under which `who`'s balance and records continue on the destination chain.
///
/// A para's sovereign account is `para` + id on the Relay Chain and `sibl` + id on another
/// parachain, so those are rewritten. Every other account keeps its address. Every account that
/// leaves the Relay Chain goes through here, so that a balance and the records that refer to it
/// land on the same account.
pub fn translate_destination(who: &AccountId32) -> AccountId32 {
	match ParaId::try_from_account(who) {
		Some(para_id) => sibling_account(para_id.into()),
		None => who.clone(),
	}
}

/// Relay Chain reserve, classified for the receiving chain.
///
/// The migrated pallets only use unnamed reserves on the Relay Chain. The sender splits each
/// account's reserve into these variants, and the receiving runtime maps each one to its own
/// `RuntimeHoldReason` with a `From<PortableHoldReason>` impl.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Copy,
	Clone,
	PartialEq,
	Eq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
)]
pub enum PortableHoldReason {
	///  Registrar deposits. Re-attributed to the owning pallet when its state arrives.
	#[codec(index = 0)]
	RegistrarDeposit,
	/// HRMP deposits. Re-attributed to the owning pallet when its state arrives.
	#[codec(index = 1)]
	HrmpDeposit,
	/// Proxy deposit of a delegator whose definitions travel too. Resized to the destination's
	/// rates when they arrive.
	#[codec(index = 2)]
	ProxyDeposit,
	/// Reserve that no deposit record on the Relay Chain accounts for. Parked on the destination
	/// for investigation.
	#[codec(index = 3)]
	UnattributedReserve,
}

/// Relay Chain proxy permission.
///
/// Only the permissions the destination represents; others are not sent.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Copy,
	Clone,
	PartialEq,
	Eq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
)]
pub enum PortableProxyType {
	#[codec(index = 0)]
	Any,
	#[codec(index = 1)]
	NonTransfer,
	#[codec(index = 2)]
	CancelProxy,
	#[codec(index = 3)]
	ParaRegistration,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn sibling_account_derivation_is_the_wire_contract() {
		// Pinned to raw bytes so a change in either derivation crate shows up here.
		let sov: AccountId32 = sibling_account(2000);
		let bytes: &[u8] = sov.as_ref();
		assert!(bytes.starts_with(b"sibl"));
		assert_eq!(bytes[4..8], 2000u32.to_le_bytes());
		assert!(bytes[8..].iter().all(|b| *b == 0));
	}

	#[test]
	fn translate_destination_rewrites_only_child_sovereigns() {
		let child: AccountId32 = ParaId::from(2000).into_account_truncating();
		assert_eq!(translate_destination(&child), sibling_account::<AccountId32>(2000));

		// A regular account keeps its address.
		let alice = AccountId32::new([1u8; 32]);
		assert_eq!(translate_destination(&alice), alice);

		// A sibling-format sovereign already is the destination address.
		let sibling: AccountId32 = sibling_account(2000);
		assert_eq!(translate_destination(&sibling), sibling);
	}
}
