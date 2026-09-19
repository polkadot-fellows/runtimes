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

//! Portable ("chain-agnostic") wire types of the AHM v2 migration, plus the few helpers whose
//! behavior both sides of the wire must agree on.
//!
//! These are the payloads exchanged between the relay-chain sender (`pallet-rc2-migrator`) and
//! the receiving chains' migrator pallets. They live in their own crate so that no runtime has
//! to depend on another chain's pallets just to speak the wire format: the sending side encodes
//! from these types, each receiving runtime declares what it can represent via ordinary
//! `From`/`TryFrom` impls on them.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::{
	storage::{transactional::with_transaction_opaque_err, TransactionOutcome},
	traits::ConstU32,
	BoundedVec,
};
use polkadot_parachain_primitives::primitives::{Id as ParaId, Sibling};
use scale_info::TypeInfo;
use sp_runtime::traits::AccountIdConversion;

/// Run `f` inside a storage transaction: `Ok` commits, `Err` rolls every write back.
///
/// This is the rollback primitive of the whole migration pipeline — per-item and per-block
/// isolation on both sides use it, so a failure can never leave state half-written.
pub fn with_rollback<R, E>(f: impl FnOnce() -> Result<R, E>) -> Result<R, E> {
	with_transaction_opaque_err(|| match f() {
		Ok(r) => TransactionOutcome::Commit(Ok(r)),
		Err(e) => TransactionOutcome::Rollback(Err(e)),
	})
	.expect("Layer limit is never reached with per-block nesting; qed")
}

/// The sibling-sovereign account of a para: where a (child) para sovereign's balances continue on
/// a parachain.
///
/// Part of the wire contract: the relay chain sends deposits to this account and the receiving
/// chain looks for them on it, so both sides must derive it identically.
pub fn sibling_account<AccountId>(para_id: u32) -> AccountId
where
	Sibling: AccountIdConversion<AccountId>,
{
	Sibling::from(ParaId::from(para_id)).into_account_truncating()
}

/// The account id under which `who`'s balances and records continue on the destination chains.
///
/// A child para sovereign (`para…`) becomes the sibling sovereign (`sibl…`) — the same parachain
/// as seen from a sibling chain; everyone else keeps their address. Part of the wire contract for
/// the same reason as [`sibling_account`]: the accounts stage moves the *money* to the translated
/// address, so any stage that ships a record under the raw relay-chain key splits that record from
/// its backing. Every account-typed field that leaves the relay chain must come through here.
pub fn translate_destination(who: &sp_runtime::AccountId32) -> sp_runtime::AccountId32 {
	match ParaId::try_from_account(who) {
		Some(para_id) => sibling_account(para_id.into()),
		None => who.clone(),
	}
}

/// Account balance payload in portable format.
///
/// The relay chain withdraws an account into this shape and the receiving chain integrates it
/// through its regular fungible APIs, so refcounts and events are indistinguishable from locally
/// created state.
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
)]
pub struct PortableAccount<AccountId, Balance> {
	/// The account address. Sent verbatim; no account-id translation happens for regular
	/// accounts.
	pub who: AccountId,
	/// Balance that stays liquid on the receiving chain.
	pub free: Balance,
	/// Balance that was not liquid on the relay chain; re-established as holds on the receiving
	/// chain, one per entry, translated via `From<PortableHoldReason>`.
	pub holds: BoundedVec<PortableHold<Balance>, ConstU32<5>>,
}

/// One non-liquid part of a migrated account's balance.
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
)]
pub struct PortableHold<Balance> {
	pub reason: PortableHoldReason,
	pub amount: Balance,
}

/// Chain-agnostic identity of balance that was not liquid on the relay chain.
///
/// This enum is the wire-level contract for hold translation: the relay-chain migrator classifies
/// every non-liquid part of an account into one of these variants, and each receiving runtime
/// declares what the variant becomes locally by implementing `From<PortableHoldReason>` for its
/// `RuntimeHoldReason`. The mapping is therefore an explicit `match` per runtime, with no
/// pallet-index coupling on the wire.
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
	/// Reserved on the relay chain without a named reason, via the old `Currency` API — how all
	/// relay-chain deposits are placed. Carries the registrar and HRMP deposits; attribution to
	/// the pallet owning the deposit happens when that pallet's own state migrates.
	#[codec(index = 0)]
	UnnamedReserve,
	/// A proxy deposit of a delegator whose (portable) definitions travel to the destination.
	/// Resized there when the definitions arrive: the destination reserves its own rates for the
	/// recreated entry and releases the rest as free balance.
	#[codec(index = 1)]
	ProxyDeposit,
	/// Reserve that no pallet's deposit records account for (a known on-chain anomaly). Nothing
	/// stays behind on the relay chain, so it travels under its own reason and stays parked at
	/// the destination for investigation; no stage ever re-attributes it.
	#[codec(index = 2)]
	UnattributedReserve,
}

/// Registrar record (`paras_registrar::ParaInfo`) in portable format.
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
)]
pub struct PortableParaInfo<AccountId, Balance> {
	pub para_id: u32,
	/// The account that placed the registration deposit and manages the para.
	pub manager: AccountId,
	/// The deposit as recorded by the registrar. Reconciled against the balance that actually
	/// arrived held during the accounts stage; never trusted on its own.
	pub deposit: Balance,
	/// Whether the para is locked from manager control, as the registrar recorded it.
	///
	/// Three-valued, and carried whole rather than collapsed. `None` means never locked and still
	/// eligible for the destination's automatic lock; `Some(false)` means deliberately unlocked
	/// and must stay that way. Collapsing the two would opt every never-locked para out of ever
	/// locking again.
	pub locked: Option<bool>,
	/// Whether the para is actually onboarded, or merely has its id reserved.
	///
	/// The discriminator is `paras::ParaLifecycles`, which stays on the relay chain — so the
	/// destination cannot work this out for itself and it has to travel.
	pub registered: bool,
	/// Length of the para's current head data, in bytes.
	///
	/// Carried so the arriving registration can be priced exactly the way a fresh one is on the
	/// destination, rather than at some worst case. Only the sending chain can measure it. Zero
	/// for a para that is merely reserved.
	pub head_len: u32,
}

/// HRMP channel record in portable format.
///
/// Records only: the dynamic message state (`msg_count`, `total_size`, `mqc_head`) is
/// deliberately not migrated.
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
)]
pub struct PortableHrmpChannel<Balance> {
	pub sender: u32,
	pub recipient: u32,
	pub max_capacity: u32,
	pub max_total_size: u32,
	pub max_message_size: u32,
	pub sender_deposit: Balance,
	pub recipient_deposit: Balance,
}

/// A pending HRMP open-channel request in portable format: para `sender` asked to open a
/// channel to `recipient` and reserved `sender_deposit`; the handshake finishes (or not) on the
/// destination chain under its future HRMP system.
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
)]
pub struct PortableHrmpRequest<Balance> {
	pub sender: u32,
	pub recipient: u32,
	/// Whether the recipient had already accepted.
	pub confirmed: bool,
	pub sender_deposit: Balance,
	pub max_message_size: u32,
	pub max_capacity: u32,
	pub max_total_size: u32,
}

/// Relay-chain proxy permission in portable format.
///
/// Deliberately carries ONLY the permissions the destination represents: the relay side filters
/// before sending, so untranslatable proxy types (Staking, Governance, …) never travel — their
/// definitions stay on the relay chain.
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

/// One proxy delegation of a migrated delegator.
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
)]
pub struct PortableProxyDelegate<AccountId> {
	/// The account which may act on behalf of the delegator; already translated by the sender.
	pub delegate: AccountId,
	pub proxy_type: PortableProxyType,
	/// The number of blocks that an announcement must be in place for before the corresponding
	/// call may be dispatched. In relay-chain blocks; the receiving chain converts it to its own
	/// block time.
	pub delay: u32,
}

/// Proxy delegations of one delegator, in portable format.
///
/// The deposit does not travel with the delegations: the accounts stage moves it as a
/// `ProxyDeposit` hold, which the receiving chain resizes to its own rates when the delegations
/// arrive.
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
)]
pub struct PortableProxy<AccountId> {
	/// The account that is delegating to their proxies; already translated by the sender.
	pub delegator: AccountId,
	/// The proxies that were delegated to and that can act on behalf of the delegator. Bounded
	/// by the relay chain's `MaxProxies`.
	pub delegates: BoundedVec<PortableProxyDelegate<AccountId>, ConstU32<32>>,
}

#[cfg(test)]
mod tests {
	use super::*;
	use sp_runtime::AccountId32;

	#[test]
	fn sibling_account_derivation_is_the_wire_contract() {
		// The relay chain sends deposits to this address and the receiving chain looks for them
		// on it. Pinned to raw bytes so a change in either derivation crate shows up here.
		let sov: AccountId32 = sibling_account(2000);
		let bytes: &[u8] = sov.as_ref();
		assert!(bytes.starts_with(b"sibl"));
		assert_eq!(bytes[4..8], 2000u32.to_le_bytes());
		assert!(bytes[8..].iter().all(|b| *b == 0));
	}

	#[test]
	fn with_rollback_commits_ok_and_rolls_back_err() {
		sp_io::TestExternalities::default().execute_with(|| {
			let key = b"test:key";

			let r: Result<(), ()> = with_rollback(|| {
				frame_support::storage::unhashed::put(key, &1u32);
				Ok(())
			});
			assert_eq!(r, Ok(()));
			assert_eq!(frame_support::storage::unhashed::get::<u32>(key), Some(1));

			let r: Result<(), ()> = with_rollback(|| {
				frame_support::storage::unhashed::put(key, &2u32);
				Err(())
			});
			assert_eq!(r, Err(()));
			assert_eq!(
				frame_support::storage::unhashed::get::<u32>(key),
				Some(1),
				"an Err must roll every write back"
			);

			// Nesting: an inner commit is still undone by an outer rollback — the per-account /
			// per-block isolation the migrators stack on top of each other.
			let r: Result<(), ()> = with_rollback(|| {
				let inner: Result<(), ()> = with_rollback(|| {
					frame_support::storage::unhashed::put(key, &3u32);
					Ok(())
				});
				assert_eq!(inner, Ok(()));
				Err(())
			});
			assert_eq!(r, Err(()));
			assert_eq!(frame_support::storage::unhashed::get::<u32>(key), Some(1));
		});
	}
}

/// How a migrator prioritises the other chain's message queue while the migration runs.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	Default,
	PartialEq,
	Eq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
)]
pub enum QueuePriority<BlockNumber> {
	/// The runtime's configured pattern.
	#[default]
	Config,
	/// `(priority_blocks, round_robin_blocks)`: force the queue to the head of the service ring
	/// for the first, let every queue take its turn for the second, repeat.
	OverrideConfig(BlockNumber, BlockNumber),
	/// No queue gets priority.
	Disabled,
}
