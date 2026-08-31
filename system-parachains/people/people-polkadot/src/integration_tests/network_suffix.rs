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

//! The network suffix that scopes every Individuality product context to this network.
//!
//! People Polkadot does not deploy the score or people-airdrops pallets, so unlike the original
//! test this checks the lite-people auth and resources contexts.

use super::*;
use frame_support::{assert_ok, traits::Get};
use indiv_support::context::{build_product_context, personhood, ProductContextNetworkSuffix};

#[test]
fn genesis_config_selects_network_suffix() {
	let mut config = RuntimeGenesisConfig::default();
	config.network_suffix.network_suffix = b"test".to_vec().try_into().unwrap();
	let mut ext: TestExternalities =
		config.build_storage().expect("runtime genesis storage builds").into();

	ext.execute_with(|| {
		assert_eq!(<NetworkSuffix as Get<ProductContextNetworkSuffix>>::get().as_slice(), b"test");
		assert_eq!(
			PeopleLite::auth_context(),
			build_product_context(personhood::PRODUCT_NAME, b"test", personhood::PEOPLE_LITE_AUTH),
		);
	});
}

#[test]
fn default_suffix_is_polkadot() {
	new_test_ext().execute_with(|| {
		assert_eq!(
			<NetworkSuffix as Get<ProductContextNetworkSuffix>>::get().as_slice(),
			system_parachains_constants::polkadot::INDIVIDUALITY_NETWORK_SUFFIX,
		);
	});
}

#[test]
fn root_suffix_override_updates_all_people_product_contexts() {
	new_test_ext().execute_with(|| {
		assert_ok!(NetworkSuffix::set_network_suffix(
			RuntimeOrigin::root(),
			b"test".to_vec().try_into().unwrap(),
		));

		assert_eq!(<NetworkSuffix as Get<ProductContextNetworkSuffix>>::get().as_slice(), b"test");
		assert_eq!(
			PeopleLite::auth_context(),
			build_product_context(personhood::PRODUCT_NAME, b"test", personhood::PEOPLE_LITE_AUTH,),
		);
		assert_eq!(
			Resources::resources_context(),
			build_product_context(personhood::PRODUCT_NAME, b"test", personhood::RESOURCES),
		);
	});
}
