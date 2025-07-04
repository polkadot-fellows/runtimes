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

//! Tests related to XCM aliasing

use crate::*;
use asset_hub_kusama_runtime::xcm_config::XcmConfig;
use frame_support::{traits::ContainsPair};
use xcm::latest::Junctions::*;

const ETHEREUM_BOB: [u8; 20] = hex_literal::hex!("11b0b11000011b0b11000011b0b11000011b0b11");

#[test]
fn asset_hub_polkadot_root_aliases_into_polkadot_origins() {
    AssetHubKusama::execute_with(|| {
        let origin = Location::new(2, X2([GlobalConsensus(Polkadot), Parachain(1000)].into()));

        let target = Location::new(2, X2([GlobalConsensus(Polkadot), Parachain(2000)].into()));
        assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));

        let target = Location::new(2, X3([GlobalConsensus(Polkadot), Parachain(2000), AccountId32Junction { network: None, id: AssetHubKusamaSender::get().into() }].into()));
        assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));

        let target = Location::new(2, X4([GlobalConsensus(Polkadot), Parachain(2000), PalletInstance(8), GeneralIndex(9)].into()));
        assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
    });
}

#[test]
fn asset_hub_polkadot_root_aliases_into_ethereum_mainnet_origins() {
    AssetHubKusama::execute_with(|| {
        let origin = Location::new(2, X2([GlobalConsensus(Polkadot), Parachain(1000)].into()));

        let target = Location::new(2, X1([GlobalConsensus(Ethereum { chain_id: 1 })].into()));
        assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));

        let target = Location::new(2, X1([GlobalConsensus(Ethereum { chain_id: 2 })].into()));
        assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));

        let target = Location::new(2, X2([GlobalConsensus(Ethereum { chain_id: 1 }), AccountKey20 { network: None, key: ETHEREUM_BOB }].into()));
        assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
    });
}
