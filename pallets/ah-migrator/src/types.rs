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

use super::*;

/// Backward mapping from https://github.com/paritytech/polkadot-sdk/blob/74a5e1a242274ddaadac1feb3990fc95c8612079/substrate/frame/balances/src/types.rs#L38
pub fn map_lock_reason(reasons: LockReasons) -> LockWithdrawReasons {
	match reasons {
		LockReasons::All => LockWithdrawReasons::TRANSACTION_PAYMENT | LockWithdrawReasons::RESERVE,
		LockReasons::Fee => LockWithdrawReasons::TRANSACTION_PAYMENT,
		LockReasons::Misc => LockWithdrawReasons::TIP,
	}
}
