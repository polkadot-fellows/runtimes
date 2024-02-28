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

pub use asset_hub_kusama_emulated_chain;
pub use asset_hub_polkadot_emulated_chain;
pub use bridge_hub_kusama_emulated_chain;
pub use bridge_hub_polkadot_emulated_chain;
pub use kusama_emulated_chain;
pub use penpal_emulated_chain;
pub use polkadot_emulated_chain;

use asset_hub_kusama_emulated_chain::AssetHubKusama;
use asset_hub_polkadot_emulated_chain::AssetHubPolkadot;
use bridge_hub_kusama_emulated_chain::BridgeHubKusama;
use bridge_hub_polkadot_emulated_chain::BridgeHubPolkadot;
use kusama_emulated_chain::Kusama;
use penpal_emulated_chain::PenpalA;
use polkadot_emulated_chain::Polkadot;

// Cumulus
use emulated_integration_tests_common::{
	accounts::{ALICE, BOB},
	impls::{BridgeHubMessageHandler, BridgeMessagesInstance1},
	xcm_emulator::{
		decl_test_bridges, decl_test_networks, decl_test_sender_receiver_accounts_parameter_types,
		Chain,
	},
};

decl_test_networks! {
	pub struct KusamaMockNet {
		relay_chain = Kusama,
		parachains = vec![
			AssetHubKusama,
			BridgeHubKusama,
			PenpalA,
		],
		bridge = KusamaPolkadotMockBridge

	},
	pub struct PolkadotMockNet {
		relay_chain = Polkadot,
		parachains = vec![
			AssetHubPolkadot,
			BridgeHubPolkadot,
		],
		bridge = PolkadotKusamaMockBridge
	},
}

decl_test_bridges! {
	pub struct KusamaPolkadotMockBridge {
		source = BridgeHubKusamaPara,
		target = BridgeHubPolkadotPara,
		handler = KusamaPolkadotMessageHandler
	},
	pub struct PolkadotKusamaMockBridge {
		source = BridgeHubPolkadotPara,
		target = BridgeHubKusamaPara,
		handler = PolkadotKusamaMessageHandler
	}
}

type BridgeHubKusamaRuntime = <BridgeHubKusamaPara as Chain>::Runtime;
type BridgeHubPolkadotRuntime = <BridgeHubPolkadotPara as Chain>::Runtime;

pub type KusamaPolkadotMessageHandler = BridgeHubMessageHandler<
	BridgeHubKusamaRuntime,
	BridgeMessagesInstance1,
	BridgeHubPolkadotRuntime,
	BridgeMessagesInstance1,
>;
pub type PolkadotKusamaMessageHandler = BridgeHubMessageHandler<
	BridgeHubPolkadotRuntime,
	BridgeMessagesInstance1,
	BridgeHubKusamaRuntime,
	BridgeMessagesInstance1,
>;

decl_test_sender_receiver_accounts_parameter_types! {
	KusamaRelay { sender: ALICE, receiver: BOB },
	AssetHubKusamaPara { sender: ALICE, receiver: BOB },
	BridgeHubKusamaPara { sender: ALICE, receiver: BOB },
	PolkadotRelay { sender: ALICE, receiver: BOB },
	AssetHubPolkadotPara { sender: ALICE, receiver: BOB },
	BridgeHubPolkadotPara { sender: ALICE, receiver: BOB },
	PenpalAPara { sender: ALICE, receiver: BOB }
}
