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
use remote_externalities::{Builder, Mode, OfflineConfig};
use sp_runtime::AccountId32;
use std::{env::var, str::FromStr};

/*
- Child 2004 (need to translate this)
	- 13YMK2eZbf9AyGhewRs6W6QTJvBSM5bxpnTD8WgeDofbg8Q1
	- Index 5: 14vDXpWfcSRPn8eWPKt2Xc8KN57tGNnAo7Z8M2C8kpkJav5q
	- Index 5/2: 14KQD8dRoT3q2fCbCC49bFjU1diFu1d516tYuGmSUMmEoGNa
- Sibling 2004 (to this)
	- 13cKp89NgPL56sRoVRpBcjkGZPrk4Vf4tS6ePUD96XhAXozG
	- Index 5: 12gb2DBw5HfpmUyBKCHxJWwGxMyXbUgf6a7bagNduHZC5S9z
	- Index 5/2: 123oqim7B24XzwB1hC4Fh7LGwbTas3QmxL6v6sVd95eTD5ee

Produces this output:

5FP74oNMwfnMb8C5EZ19T6uKA1icCi4vvcA4jyn5vGjicw7e
	Info: AccountInfo { nonce: 0, consumers: 3, providers: 1, sufficients: 0, data: AccountData { free: 100717000000, reserved: 5148160913549619, frozen: 0, flags: ExtraFlags(170141183460469231731687303715884105728) } }
	Ledger: Some(StakingLedger { stash: 92bd3f2458616fa5b5fd5ff200c095f1c19bf8a0acba0284bc87895be40da62b (5FP74oNM...), total: 5148160913549619, active: 3956658596678572, unlocking: BoundedVec([UnlockChunk { value: 155291379466697, era: 1967 }, UnlockChunk { value: 117832905688130, era: 1968 }, UnlockChunk { value: 214505811290115, era: 1969 }, UnlockChunk { value: 75033107810864, era: 1970 }, UnlockChunk { value: 627033020239963, era: 1971 }, UnlockChunk { value: 1806092375278, era: 1972 }], 32), controller: None })
5D7WhPW3KEo4ZQAVjZ1FYxW85yTwAjrdsqNRwaWGazcw2g7R
	Info: AccountInfo { nonce: 0, consumers: 0, providers: 0, sufficients: 0, data: AccountData { free: 0, reserved: 0, frozen: 0, flags: ExtraFlags(170141183460469231731687303715884105728) } }
	Ledger: None

EventRecord { phase: Phase::Initialization, event: Staking(Event::StakerRemoved { stash: 92bd3f2458616fa5b5fd5ff200c095f1c19bf8a0acba0284bc87895be40da62b (5FP74oNM...) }), topics: [] }
EventRecord { phase: Phase::Initialization, event: System(Event::KilledAccount { account: 92bd3f2458616fa5b5fd5ff200c095f1c19bf8a0acba0284bc87895be40da62b (5FP74oNM...) }), topics: [] }
EventRecord { phase: Phase::Initialization, event: System(Event::NewAccount { account: 2e6066d99766402e55498a92019a1be865caf195b7f2c9f1e5258a331f131ae0 (5D7WhPW3...) }), topics: [] }
EventRecord { phase: Phase::Initialization, event: Balances(Event::Endowed { account: 2e6066d99766402e55498a92019a1be865caf195b7f2c9f1e5258a331f131ae0 (5D7WhPW3...), free_balance: 5148261630549619 }), topics: [] }
EventRecord { phase: Phase::Initialization, event: Balances(Event::Transfer { from: 92bd3f2458616fa5b5fd5ff200c095f1c19bf8a0acba0284bc87895be40da62b (5FP74oNM...), to: 2e6066d99766402e55498a92019a1be865caf195b7f2c9f1e5258a331f131ae0 (5D7WhPW3...), amount: 5148261630549619 }), topics: [] }
EventRecord { phase: Phase::Initialization, event: Staking(Event::Bonded { stash: 2e6066d99766402e55498a92019a1be865caf195b7f2c9f1e5258a331f131ae0 (5D7WhPW3...), amount: 3956658596678572 }), topics: [] }
EventRecord { phase: Phase::Initialization, event: AhOps(Event::SovereignMigrated { para_id: 2004, from: 92bd3f2458616fa5b5fd5ff200c095f1c19bf8a0acba0284bc87895be40da62b (5FP74oNM...), to: 2e6066d99766402e55498a92019a1be865caf195b7f2c9f1e5258a331f131ae0 (5D7WhPW3...), derivation_path: [5, 2] }), topics: [] }

5FP74oNMwfnMb8C5EZ19T6uKA1icCi4vvcA4jyn5vGjicw7e
	Info: AccountInfo { nonce: 0, consumers: 0, providers: 0, sufficients: 0, data: AccountData { free: 0, reserved: 0, frozen: 0, flags: ExtraFlags(170141183460469231731687303715884105728) } }
	Ledger: None
5D7WhPW3KEo4ZQAVjZ1FYxW85yTwAjrdsqNRwaWGazcw2g7R
	Info: AccountInfo { nonce: 0, consumers: 3, providers: 1, sufficients: 0, data: AccountData { free: 1191603033871047, reserved: 3956658596678572, frozen: 0, flags: ExtraFlags(170141183460469231731687303715884105728) } }
	Ledger: Some(StakingLedger { stash: 2e6066d99766402e55498a92019a1be865caf195b7f2c9f1e5258a331f131ae0 (5D7WhPW3...), total: 3956658596678572, active: 3956658596678572, unlocking: BoundedVec([], 32), controller: None })

*/
#[tokio::test]
async fn moonbeam_stellaswap_translation() {
	sp_tracing::try_init_simple();
	let Some(state_snapshot) = var("SNAP").map(|s| s.into()).ok() else {
		return;
	};

	let mut ext = Builder::<Block>::default()
		.mode(Mode::Offline(OfflineConfig { state_snapshot }))
		.build()
		.await
		.unwrap();
	ext.execute_with(|| {
		frame_system::Pallet::<Runtime>::reset_events();

		// 5FP74oNMwfnMb8C5EZ19T6uKA1icCi4vvcA4jyn5vGjicw7e
		let child_5_2 =
			AccountId32::from_str("14KQD8dRoT3q2fCbCC49bFjU1diFu1d516tYuGmSUMmEoGNa").unwrap();
		// 5D7WhPW3KEo4ZQAVjZ1FYxW85yTwAjrdsqNRwaWGazcw2g7R
		let sibl_5_2 =
			AccountId32::from_str("123oqim7B24XzwB1hC4Fh7LGwbTas3QmxL6v6sVd95eTD5ee").unwrap();
		let derivation_path = vec![5, 2];

		let child_before = summary(&child_5_2);
		assert_eq!(summary(&sibl_5_2), 0, "Sibl acc should be empty");

		pallet_ah_ops::Pallet::<Runtime>::do_translate_para_sovereign_child_to_sibling_derived(
			2004,
			derivation_path.clone(),
			child_5_2.clone(),
			sibl_5_2.clone(),
		)
		.unwrap();

		for event in frame_system::Pallet::<Runtime>::events() {
			println!("{:?}", event);
		}

		assert_eq!(summary(&child_5_2), 0, "Child acc should be empty");
		assert_eq!(summary(&sibl_5_2), child_before, "Sibl should have child balance");
	});
}

/// Account summary and return the total balance.
fn summary(acc: &AccountId32) -> u128 {
	let info = frame_system::Account::<Runtime>::get(&acc);
	let ledger = pallet_staking_async::Ledger::<Runtime>::get(&acc);
	println!("{}\n\tInfo: {:?}\n\tLedger: {:?}", acc, info, ledger);

	info.data.free + info.data.reserved
}
