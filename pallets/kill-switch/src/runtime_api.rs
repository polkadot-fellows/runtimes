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

use crate::GroupInfo;
use alloc::vec::Vec;
use codec::Codec;

sp_api::decl_runtime_apis! {
	/// What can be paused on this chain. Meant for wallets and monitors.
	pub trait KillSwitchApi<Group, CallName, Balance>
	where
		Group: Codec,
		CallName: Codec,
		Balance: Codec,
	{
		/// Every group, with the calls it pauses and the deposit it needs.
		fn pause_groups() -> Vec<GroupInfo<Group, CallName, Balance>>;

		/// The deposit for pausing `call` on its own. `None` if it can't be paused on its own.
		fn call_deposit(call: CallName) -> Option<Balance>;
	}
}
