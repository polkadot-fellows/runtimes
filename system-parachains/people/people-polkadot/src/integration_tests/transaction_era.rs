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

//! Transaction-era policy for runtime-constructed (offchain-worker) authorized transactions.

use super::*;
use frame_system::offchain::CreateAuthorizedTransaction;

/// The extension attached to authorized transactions the runtime submits itself must carry a
/// short mortal era anchored at the parent block, not `Immortal`. Pinning the exact era guards
/// against silently reverting to `Immortal` and against an off-by-one in the birth anchoring.
#[test]
fn authorized_transaction_extension_uses_parent_anchored_mortal_era() {
	new_test_ext().execute_with(|| {
		let current: BlockNumber = 1_000;
		System::set_block_number(current);

		let ext = <Runtime as CreateAuthorizedTransaction<RuntimeCall>>::create_extension();
		// `StorageWeightReclaim(inner)` -> inner tuple field `.5` is `CheckEra` -> `.0` is the era.
		let era = ext.0 .5 .0;

		assert!(!era.is_immortal(), "OCW transactions must be mortal");

		// Anchored at the parent (`current - 1`): while the OCW builds the transaction executing
		// on `current`, that block's own hash is not yet in storage, so the birth must be its
		// parent.
		let birth = era.birth(current.into());
		assert_eq!(birth, u64::from(current) - 1);

		// The mortality window equals the configured period.
		assert_eq!(era.death(birth) - birth, u64::from(TRANSACTION_MORTALITY_PERIOD));
	});
}
