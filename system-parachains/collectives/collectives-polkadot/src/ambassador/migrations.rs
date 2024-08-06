// Copyright (C) 2022 Parity Technologies (UK) Ltd.
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

//! Migrations for the Ambassador Program.

use super::*;
use crate::AccountId;
#[cfg(feature = "try-runtime")]
use frame_support::ensure;
use frame_support::traits::{Defensive, OnRuntimeUpgrade, UnfilteredDispatchable};
use frame_system::RawOrigin;
use pallet_ranked_collective::WeightInfo;
use sp_core::crypto::Ss58Codec;

/// The first 21 Head Ambassadors that have been inducted.
///
/// Query `AmbassadorCollective::members` at block 4,326,024 (hash
/// `0x7ca86d410526e9e3cefa210bfb71b3abc52396e658d5c63f553c8a6ee5845289`).
fn first_21() -> [AccountId; 21] {
	[
		AccountId::from_ss58check("14Gn7SEmCgMX7Ukuppnw5TRjA7pao2HFpuJo39frB42tYLEh").unwrap(),
		AccountId::from_ss58check("1LboBQLsa1iTpGzZvXcSd5VF7jfUYf6MzPNoRy2HT9D9FNk").unwrap(),
		AccountId::from_ss58check("12xRcHjvStkUYgzTh9vyqinN3tUpddgcPnSUcLXZ3ty44Mq1").unwrap(),
		AccountId::from_ss58check("16SQKanFTrN18k9UE8EFbtyeSGNFPjRBg8caVhGoUNL8cdh6").unwrap(),
		AccountId::from_ss58check("1HPKZzzd9nyr2DdvtPxytNMZm3Ld5nh3BBY4Ecgg9JxgL7G").unwrap(),
		AccountId::from_ss58check("13Ec62Cvw9jmPxA23EidSwASPs9X2Vohqv9RCogCfDvXC4c8").unwrap(),
		AccountId::from_ss58check("14N5cTFuzJf6irrQkKNAjiADKsCxgk48LUKx2fA77YRruzMW").unwrap(),
		AccountId::from_ss58check("1hFmn2CuqXqxHgKDqqs2xRBpsPkiRXzJfcLbfDgsW7qgmpA").unwrap(),
		AccountId::from_ss58check("15rYBV5YwGmhzee5PWqrnQtb2HhwWP2rK2f4cLMhFfcNdPZL").unwrap(),
		AccountId::from_ss58check("14x5RbyJxD6KvNyncbJQuJJJ3zHinXg57YKwhJ7q9T9aJq4n").unwrap(),
		AccountId::from_ss58check("1ZSPR3zNg5Po3obkhXTPR95DepNBzBZ3CyomHXGHK9Uvx6w").unwrap(),
		AccountId::from_ss58check("1yCg8NSCgjS4K5KDK5DZGhxUCxmgVyhyG6vBPn5wqUmLuYo").unwrap(),
		AccountId::from_ss58check("14VDkd5mWY9SajUUnEg2LgMgVrbY412H7xn7Y7EXjhnGkiBF").unwrap(),
		AccountId::from_ss58check("1CRmVRcQymMpot855oKDq76kF19jJezMMkGcrvHh1ozEqXa").unwrap(),
		AccountId::from_ss58check("15cZn8K1DaE7qiBWK6mGFJMKYKjFrALTVwe5urpD9PzKSsPY").unwrap(),
		AccountId::from_ss58check("146ZZqm2cMHLf3ju7oc8M9JnPaAktuADAKThagKnXqzjPJbZ").unwrap(),
		AccountId::from_ss58check("14DJADQdE3bUQMFsjsejwCLiG1tiMDsFhCCiXavyBTKHu6kr").unwrap(),
		AccountId::from_ss58check("1HGnvAkk9nbfZ58CzUJhjcrLdEDMkr5qNkqqYkyD5BF5v6Y").unwrap(),
		AccountId::from_ss58check("155G4q3yS7gW933PrdxrY4ersg2YhqWUnGC8GUd7NWiZwuKj").unwrap(),
		AccountId::from_ss58check("16XYgDGN6MxvdmjhRsHLT1oqQVDwGdEPVQqC42pRXiZrE8su").unwrap(),
		AccountId::from_ss58check("15fHj7Q7SYxqMgZ38UpjXS8cxdq77rczTP3JgY9JVi5piMPN").unwrap(),
	]
}

/// Removes everyone from the Ambassador collective besides the first 21 HAs that were registered.
pub struct TruncateHeadAmbassadors;

impl OnRuntimeUpgrade for TruncateHeadAmbassadors {
	fn on_runtime_upgrade() -> Weight {
		let mut to_be_removed =
			pallet_ranked_collective::Members::<Runtime, AmbassadorCollectiveInstance>::iter_keys()
				.collect::<Vec<_>>();

		// first 21 should not be removed
		let first_21 = first_21();
		to_be_removed.retain(|acc| !first_21.contains(acc));

		log::info!("Removing {} member(s) from the Ambassador Collective.", to_be_removed.len());

		for acc in &to_be_removed {
			log::info!("Removing member {}", acc.to_ss58check());

			// The pallet has no nice trait that we could call, so need to use the extrinsic...
			let origin: RuntimeOrigin = RawOrigin::Root.into();

			let call = pallet_ranked_collective::Call::<Runtime, AmbassadorCollectiveInstance>::remove_member { who: acc.clone().into(), min_rank: 3 };
			let _ = call.dispatch_bypass_filter(origin).defensive();
		}

		crate::weights::pallet_ranked_collective_ambassador_collective::WeightInfo::<Runtime>::remove_member(3).saturating_mul(to_be_removed.len() as u64)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let num =
			pallet_ranked_collective::Members::<Runtime, AmbassadorCollectiveInstance>::iter_keys()
				.count();
		ensure!(num <= 21, "There must be not more than 21 Head Ambassadors.");

		let seed = first_21();
		for ambassador in
			pallet_ranked_collective::Members::<Runtime, AmbassadorCollectiveInstance>::iter_keys()
		{
			ensure!(
				seed.contains(&ambassador),
				"Ambassador is not in the seed and should have been removed."
			);
		}

		Ok(())
	}
}
