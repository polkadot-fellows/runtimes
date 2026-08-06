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

//! The current, intentionally failing People-to-Bulletin long-term-storage protocol.

use crate::*;
use indiv_pallet_resources::{
	types::MembershipCollection, Origin as ResourcesOrigin, SpentLongTermStorageAliases,
};
use indiv_support::utils::BigEndianU32;

/// This is the silent-failure trap: People accepts a long-term-storage allocation and sends an
/// XCM `Transact`, but Bulletin does not include `pallet-transaction-storage` at the configured
/// index. Once Bulletin deploys that pallet, replace the receiver assertion with the created
/// authorization state and keep the sender-side assertion.
#[test]
fn people_long_term_storage_grant_is_not_applied_on_bulletin_yet() {
	let who: AccountId = [42; 32].into();

	PeoplePolkadot::execute_with(|| {
		let alias = [7; 32];
		assert_ok!(people_polkadot_runtime::Resources::claim_long_term_storage(
			people_polkadot_runtime::RuntimeOrigin::from(
				people_polkadot_runtime::OriginCaller::Resources(
					ResourcesOrigin::LongTermStorageClaim(alias, MembershipCollection::People),
				),
			),
			0,
			0,
			who.clone(),
		));
		assert!(SpentLongTermStorageAliases::<people_polkadot_runtime::Runtime>::contains_key(
			BigEndianU32::from(0),
			alias,
		));
		assert!(people_polkadot_runtime::System::events().iter().any(|record| {
			matches!(
				&record.event,
				people_polkadot_runtime::RuntimeEvent::Resources(
					indiv_pallet_resources::Event::LongTermStorageClaimed { account, .. }
				) if account == &who
			)
		}));
	});

	BulletinPolkadot::execute_with(|| {
		// The queue rejects the message before dispatch: Bulletin has no TransactionStorage runtime
		// call to decode. This is the current silent-failure trap to flip after deployment.
		assert!(bulletin_polkadot_runtime::System::events().iter().any(|record| {
			matches!(
				record.event,
				bulletin_polkadot_runtime::RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::ProcessingFailed {
						error: frame_support::traits::ProcessMessageError::Unsupported,
						..
					}
				)
			)
		}));
	});
}
