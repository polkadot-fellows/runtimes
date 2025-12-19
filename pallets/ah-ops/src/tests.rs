// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Cumulus.
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

use crate::{
	mock::{assert_last_event, new_test_ext, Runtime as AssetHub},
	Error, Event,
};
use frame_support::assert_noop;
use sp_runtime::AccountId32;
use std::str::FromStr;

#[test]
fn sovereign_account_translation() {
	// https://docs.google.com/document/d/1DXYWPXEwi0DkDfG8Fb2ZTI4DQBAz87DBCIW7yQIVrj0
	let bifrost_cases = [
		// Bifrost Polkadot #1
		(
			// para 2030
			"13YMK2eeopZtUNpeHnJ1Ws2HqMQG6Ts9PGCZYGyFbSYoZfcm",
			// sibl 2030
			"13cKp89TtYknbyYnqnF6dWN75q5ZosvFSuqzoEVkUAaNR47A",
			None,
		),
		// Bifrost Polkadot #2
		(
			// para 2030 index 0
			"14vtfeKAVKh1Jzb3s7e43SqZ3zB5MLsdCxZPoKDxeoCFKLu5",
			// sibl 2030 index 0
			"5ETehspFKFNpBbe5DsfuziN6BWq5Qwp1J8qcTQQoAxwa7BsS",
			// derivation proof (para 2030, index 0)
			Some(("13YMK2eeopZtUNpeHnJ1Ws2HqMQG6Ts9PGCZYGyFbSYoZfcm", 0u16)),
		),
		// Bifrost Polkadot #3
		(
			// para 2030 index 1
			"14QkQ7wVVDRrhbC1UqHsFwKFUns1SRud94CXMWGHWB8Jhtro",
			// sibl 2030 index 1
			"5DNWZkkAxLhqF8tevcbRGyARAVM7abukftmqvoDFUN5dDDDz",
			// derivation proof (para 2030, index 1)
			Some(("13YMK2eeopZtUNpeHnJ1Ws2HqMQG6Ts9PGCZYGyFbSYoZfcm", 1u16)),
		),
		// Bifrost Polkadot #4
		(
			// para 2030 index 2
			"13hLwqcVHqjiJMbZhR9LtfdhoxmTdssi7Kp8EJaW2yfk3knK",
			// sibl 2030 index 2
			"5EmiwjDYiackJma1GW3aBbQ74rLfWh756UKDb7Cm83XDkUUZ",
			// derivation proof (para 2030, index 2)
			Some(("13YMK2eeopZtUNpeHnJ1Ws2HqMQG6Ts9PGCZYGyFbSYoZfcm", 2u16)),
		),
		// Bifrost Kusama #1
		(
			// para 2001
			"5Ec4AhPV91i9yNuiWuNunPf6AQCYDhFTTA4G5QCbtqYApH9E",
			// sibl 2001
			"5Eg2fntJDju46yds4uKzu2zuQssqw7JZWohhLMj6mZZjg2pK",
			None,
		),
		// Bifrost Kusama #2
		(
			// para 2001 index 0
			"5E78xTBiaN3nAGYtcNnqTJQJqYAkSDGggKqaDfpNsKyPpbcb",
			// sibl 2001 index 0
			"5CzXNqgBZT5yMpMETdfH55saYNKQoJBXsSfnu4d2s1ejYFir",
			// derivation proof (para 2001, index 0)
			Some(("5Ec4AhPV91i9yNuiWuNunPf6AQCYDhFTTA4G5QCbtqYApH9E", 0u16)),
		),
		// Bifrost Kusama #3
		(
			// para 2001 index 1
			"5HXi9pzWnTQzk7VKzY6VQn92KfWCcA5NbSm53uKHrYU1VsjP",
			// sibl 2001 index 1
			"5GcexD4YNqcKTbW1YWDRczQzpxic61byeNeLaHgqQHk8pxQJ",
			// derivation proof (para 2001, index 1)
			Some(("5Ec4AhPV91i9yNuiWuNunPf6AQCYDhFTTA4G5QCbtqYApH9E", 1u16)),
		),
		// Bifrost Kusama #4
		(
			// para 2001 index 2
			"5CkKS3YMx64TguUYrMERc5Bn6Mn2aKMUkcozUFREQDgHS3Tv",
			// sibl 2001 index 2
			"5FoYMVucmT552GDMWfYNxcF2XnuuvLbJHt7mU6DfDCpUAS2Y",
			// derivation proof (para 2001, index 2)
			Some(("5Ec4AhPV91i9yNuiWuNunPf6AQCYDhFTTA4G5QCbtqYApH9E", 2u16)),
		),
		// Bifrost Kusama #5
		(
			// para 2001 index 3
			"5Crxhmiw5CQq3Mnfcu3dR3yJ3YpjbxjqaeDFtNNtqgmcnN4S",
			// sibl 2001 index 3
			"5FP39fgPYhJw3vcLwSMqMnwBuEVGexUMG6JQLPR9yPVhq6Wy",
			// derivation proof (para 2001, index 3)
			Some(("5Ec4AhPV91i9yNuiWuNunPf6AQCYDhFTTA4G5QCbtqYApH9E", 3u16)),
		),
		// Bifrost Kusama #5
		(
			// para 2001 index 3
			"5DAZP4gZKZafGv42uoWNTMau4tYuDd2XteJLGL4upermhQpn",
			// sibl 2001 index 3
			"5ExtLdYnjHLJbngU1QpumjPieCGaCXwwkH1JrFBQ9GATuNGv",
			// derivation proof (para 2001, index 4)
			Some(("5Ec4AhPV91i9yNuiWuNunPf6AQCYDhFTTA4G5QCbtqYApH9E", 4u16)),
		),
	];

	for (from, to, derivation) in bifrost_cases {
		let from = AccountId32::from_str(from).unwrap();
		let to = AccountId32::from_str(to).unwrap();

		println!("Translating {from}/{derivation:?} -> {to}");
		if let Some((parent, index)) = derivation {
			let parent = AccountId32::from_str(parent).unwrap();
			let (got_to, _) =
				crate::Pallet::<AssetHub>::try_rc_sovereign_derived_to_ah(&from, &parent, index)
					.unwrap();
			assert_eq!(got_to, to);
		} else {
			let (got_to, _) =
				crate::Pallet::<AssetHub>::try_translate_rc_sovereign_to_ah(&from).unwrap();
			assert_eq!(got_to, to);
		}
	}
}

#[test]
fn translate_derived_account() {
	let child = AccountId32::from_str("13YMK2eZbf9AyGhewRs6W6QTJvBSM5bxpnTD8WgeDofbg8Q1").unwrap();
	let sibl = AccountId32::from_str("13cKp89NgPL56sRoVRpBcjkGZPrk4Vf4tS6ePUD96XhAXozG").unwrap();
	let derivation = vec![5, 2];

	new_test_ext().execute_with(|| {
		// wrong para id
		assert_noop!(
			crate::Pallet::<AssetHub>::do_translate_para_sovereign_child_to_sibling_derived(
				2005,
				derivation.clone(),
				child.clone(),
				sibl.clone(),
			),
			Error::<AssetHub>::WrongDerivedTranslation
		);

		// wrong derivation path
		assert_noop!(
			crate::Pallet::<AssetHub>::do_translate_para_sovereign_child_to_sibling_derived(
				2004,
				vec![5, 3],
				child.clone(),
				sibl.clone(),
			),
			Error::<AssetHub>::WrongDerivedTranslation
		);

		// wrong acc
		assert_noop!(
			crate::Pallet::<AssetHub>::do_translate_para_sovereign_child_to_sibling_derived(
				2004,
				derivation.clone(),
				child.clone(),
				child.clone(),
			),
			Error::<AssetHub>::WrongDerivedTranslation
		);

		// wrong acc
		assert_noop!(
			crate::Pallet::<AssetHub>::do_translate_para_sovereign_child_to_sibling_derived(
				2004,
				derivation.clone(),
				sibl.clone(),
				sibl.clone(),
			),
			Error::<AssetHub>::WrongDerivedTranslation
		);

		// wrong acc
		assert_noop!(
			crate::Pallet::<AssetHub>::do_translate_para_sovereign_child_to_sibling_derived(
				2004,
				derivation.clone(),
				sibl.clone(),
				child.clone(),
			),
			Error::<AssetHub>::WrongDerivedTranslation
		);
	});
}

#[test]
fn moonbeam_stellaswap_double_derived_translation() {
	new_test_ext().execute_with(|| {
		let child_5_2 =
			AccountId32::from_str("14KQD8dRoT3q2fCbCC49bFjU1diFu1d516tYuGmSUMmEoGNa").unwrap();
		let sibl_5_2 =
			AccountId32::from_str("123oqim7B24XzwB1hC4Fh7LGwbTas3QmxL6v6sVd95eTD5ee").unwrap();
		let derivation_path = vec![5, 2];

		crate::Pallet::<AssetHub>::do_translate_para_sovereign_child_to_sibling_derived(
			2004,
			derivation_path.clone(),
			child_5_2.clone(),
			sibl_5_2.clone(),
		)
		.unwrap();
	});
}
