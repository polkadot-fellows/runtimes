// Copyright (C) Parity Technologies and the various Polkadot contributors, see Contributions.md
// for a list of specific contributors.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Runtime API for the kill switch.

use crate::{GroupInfo, PauseId, Target};
use alloc::vec::Vec;
use codec::{Codec, Decode, Encode};
use scale_info::TypeInfo;

/// A pause that ran out and still waits for its deposits to be refunded or slashed.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub struct EndedPauseInfo<Target, AccountId, Balance> {
	/// The id to pass to `settle`.
	pub id: PauseId,
	/// What was paused.
	pub target: Target,
	/// Who put up each deposit, and how much.
	pub deposits: Vec<(AccountId, Balance)>,
}

sp_api::decl_runtime_apis! {
	/// What can be paused on this chain. Meant for wallets and monitors.
	pub trait KillSwitchApi<Group, CallName, AccountId, Balance>
	where
		Group: Codec,
		CallName: Codec,
		AccountId: Codec,
		Balance: Codec,
	{
		/// Every group, with the calls it pauses and the deposit it needs.
		fn pause_groups() -> Vec<GroupInfo<Group, CallName, Balance>>;

		/// The deposit for pausing `call` on its own. `None` if it can't be paused on its own.
		fn call_deposit(call: CallName) -> Option<Balance>;

		/// Pauses that ran out by themselves and still wait for the Fellowship or Root to refund
		/// or slash their deposits.
		fn ended_pauses() -> Vec<EndedPauseInfo<Target<Group, CallName>, AccountId, Balance>>;
	}
}
