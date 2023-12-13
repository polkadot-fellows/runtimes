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

#![cfg(not(feature = "runtime-benchmarks"))]

pub mod constants;
pub mod impls;
pub mod xcm_helpers;

use constants::{
	accounts::{ALICE, BOB},
	asset_hub_kusama, asset_hub_polkadot, bridge_hub_kusama, bridge_hub_polkadot, collectives,
	kusama, penpal, polkadot,
};

// Substrate
use frame_support::traits::OnInitialize;

// Cumulus
use xcm_emulator::{
	// decl_test_bridges,
	decl_test_networks,
	decl_test_parachains,
	decl_test_relay_chains,
	decl_test_sender_receiver_accounts_parameter_types,
	DefaultMessageProcessor,
};

use polkadot_primitives::runtime_api::runtime_decl_for_parachain_host::ParachainHostV7;

decl_test_relay_chains! {
	#[api_version(5)]
	pub struct Polkadot {
		genesis = polkadot::genesis(),
		on_init = (),
		runtime = polkadot_runtime,
		core = {
			MessageProcessor: DefaultMessageProcessor<Polkadot>,
			SovereignAccountOf: polkadot_runtime::xcm_config::SovereignAccountOf,
		},
		pallets = {
			XcmPallet: polkadot_runtime::XcmPallet,
			Balances: polkadot_runtime::Balances,
			Hrmp: polkadot_runtime::Hrmp,
		}
	},
	#[api_version(5)]
	pub struct Kusama {
		genesis = kusama::genesis(),
		on_init = (),
		runtime = kusama_runtime,
		core = {
			MessageProcessor: DefaultMessageProcessor<Kusama>,
			SovereignAccountOf: kusama_runtime::xcm_config::SovereignAccountOf,
		},
		pallets = {
			XcmPallet: kusama_runtime::XcmPallet,
			Balances: kusama_runtime::Balances,
			Hrmp: kusama_runtime::Hrmp,
		}
	},
}

decl_test_parachains! {
	// Polkadot Parachains
	pub struct AssetHubPolkadot {
		genesis = asset_hub_polkadot::genesis(),
		on_init = {
			asset_hub_polkadot_runtime::AuraExt::on_initialize(1);
		},
		runtime = asset_hub_polkadot_runtime,
		core = {
			XcmpMessageHandler: asset_hub_polkadot_runtime::XcmpQueue,
			DmpMessageHandler: asset_hub_polkadot_runtime::DmpQueue,
			LocationToAccountId: asset_hub_polkadot_runtime::xcm_config::LocationToAccountId,
			ParachainInfo: asset_hub_polkadot_runtime::ParachainInfo,
		},
		pallets = {
			PolkadotXcm: asset_hub_polkadot_runtime::PolkadotXcm,
			Assets: asset_hub_polkadot_runtime::Assets,
			Balances: asset_hub_polkadot_runtime::Balances,
		}
	},
	pub struct Collectives {
		genesis = collectives::genesis(),
		on_init = {
			collectives_polkadot_runtime::AuraExt::on_initialize(1);
		},
		runtime = collectives_polkadot_runtime,
		core = {
			XcmpMessageHandler: collectives_polkadot_runtime::XcmpQueue,
			DmpMessageHandler: collectives_polkadot_runtime::DmpQueue,
			LocationToAccountId: collectives_polkadot_runtime::xcm_config::LocationToAccountId,
			ParachainInfo: collectives_polkadot_runtime::ParachainInfo,
		},
		pallets = {
			PolkadotXcm: collectives_polkadot_runtime::PolkadotXcm,
			Balances: collectives_polkadot_runtime::Balances,
		}
	},
	pub struct BridgeHubPolkadot {
		genesis = bridge_hub_polkadot::genesis(),
		on_init = {
			bridge_hub_polkadot_runtime::AuraExt::on_initialize(1);
		},
		runtime = bridge_hub_polkadot_runtime,
		core = {
			XcmpMessageHandler: bridge_hub_polkadot_runtime::XcmpQueue,
			DmpMessageHandler: bridge_hub_polkadot_runtime::DmpQueue,
			LocationToAccountId: bridge_hub_polkadot_runtime::xcm_config::LocationToAccountId,
			ParachainInfo: bridge_hub_polkadot_runtime::ParachainInfo,
		},
		pallets = {
			PolkadotXcm: bridge_hub_polkadot_runtime::PolkadotXcm,
		}
	},
	pub struct PenpalPolkadotA {
		genesis = penpal::genesis(penpal::PARA_ID_A),
		on_init = {
			penpal_runtime::AuraExt::on_initialize(1);
		},
		runtime = penpal_runtime,
		core = {
			XcmpMessageHandler: penpal_runtime::XcmpQueue,
			DmpMessageHandler: penpal_runtime::DmpQueue,
			LocationToAccountId: penpal_runtime::xcm_config::LocationToAccountId,
			ParachainInfo: penpal_runtime::ParachainInfo,
		},
		pallets = {
			PolkadotXcm: penpal_runtime::PolkadotXcm,
			Assets: penpal_runtime::Assets,
		}
	},
	pub struct PenpalPolkadotB {
		genesis = penpal::genesis(penpal::PARA_ID_B),
		on_init = {
			penpal_runtime::AuraExt::on_initialize(1);
		},
		runtime = penpal_runtime,
		core = {
			XcmpMessageHandler: penpal_runtime::XcmpQueue,
			DmpMessageHandler: penpal_runtime::DmpQueue,
			LocationToAccountId: penpal_runtime::xcm_config::LocationToAccountId,
			ParachainInfo: penpal_runtime::ParachainInfo,
		},
		pallets = {
			PolkadotXcm: penpal_runtime::PolkadotXcm,
			Assets: penpal_runtime::Assets,
		}
	},
	// Kusama Parachains
	pub struct AssetHubKusama {
		genesis = asset_hub_kusama::genesis(),
		on_init = {
			asset_hub_kusama_runtime::AuraExt::on_initialize(1);
		},
		runtime = asset_hub_kusama_runtime,
		core = {
			XcmpMessageHandler: asset_hub_kusama_runtime::XcmpQueue,
			DmpMessageHandler: asset_hub_kusama_runtime::DmpQueue,
			LocationToAccountId: asset_hub_kusama_runtime::xcm_config::LocationToAccountId,
			ParachainInfo: asset_hub_kusama_runtime::ParachainInfo,
		},
		pallets = {
			PolkadotXcm: asset_hub_kusama_runtime::PolkadotXcm,
			Assets: asset_hub_kusama_runtime::Assets,
			ForeignAssets: asset_hub_kusama_runtime::ForeignAssets,
			PoolAssets: asset_hub_kusama_runtime::PoolAssets,
			AssetConversion: asset_hub_kusama_runtime::AssetConversion,
			Balances: asset_hub_kusama_runtime::Balances,
		}
	},
	pub struct BridgeHubKusama {
		genesis = bridge_hub_kusama::genesis(),
		on_init = {
			bridge_hub_kusama_runtime::AuraExt::on_initialize(1);
		},
		runtime = bridge_hub_kusama_runtime,
		core = {
			XcmpMessageHandler: bridge_hub_kusama_runtime::XcmpQueue,
			DmpMessageHandler: bridge_hub_kusama_runtime::DmpQueue,
			LocationToAccountId: bridge_hub_kusama_runtime::xcm_config::LocationToAccountId,
			ParachainInfo: bridge_hub_kusama_runtime::ParachainInfo,
		},
		pallets = {
			PolkadotXcm: bridge_hub_kusama_runtime::PolkadotXcm,
		}
	},
	pub struct PenpalKusamaA {
		genesis = penpal::genesis(penpal::PARA_ID_A),
		on_init = {
			penpal_runtime::AuraExt::on_initialize(1);
		},
		runtime = penpal_runtime,
		core = {
			XcmpMessageHandler: penpal_runtime::XcmpQueue,
			DmpMessageHandler: penpal_runtime::DmpQueue,
			LocationToAccountId: penpal_runtime::xcm_config::LocationToAccountId,
			ParachainInfo: penpal_runtime::ParachainInfo,
		},
		pallets = {
			PolkadotXcm: penpal_runtime::PolkadotXcm,
			Assets: penpal_runtime::Assets,
		}
	},
	pub struct PenpalKusamaB {
		genesis = penpal::genesis(penpal::PARA_ID_B),
		on_init = {
			penpal_runtime::AuraExt::on_initialize(1);
		},
		runtime = penpal_runtime,
		core = {
			XcmpMessageHandler: penpal_runtime::XcmpQueue,
			DmpMessageHandler: penpal_runtime::DmpQueue,
			LocationToAccountId: penpal_runtime::xcm_config::LocationToAccountId,
			ParachainInfo: penpal_runtime::ParachainInfo,
		},
		pallets = {
			PolkadotXcm: penpal_runtime::PolkadotXcm,
			Assets: penpal_runtime::Assets,
		}
	},
}

decl_test_networks! {
	pub struct PolkadotMockNet {
		relay_chain = Polkadot,
		parachains = vec![
			AssetHubPolkadot,
			Collectives,
			BridgeHubPolkadot,
			PenpalPolkadotA,
			PenpalPolkadotB,
		],
		// TODO: uncomment when https://github.com/polkadot-fellows/runtimes/pull/108 is merged
		// bridge = PolkadotKusamaMockBridge
		bridge = ()
	},
	pub struct KusamaMockNet {
		relay_chain = Kusama,
		parachains = vec![
			AssetHubKusama,
			BridgeHubKusama,
			PenpalKusamaA,
			PenpalKusamaB,
		],
		// TODO: uncomment when https://github.com/polkadot-fellows/runtimes/pull/108 is merged
		// bridge = KusamaPolkadotMockBridge
		bridge = ()
	},
}

// TODO: uncomment when https://github.com/polkadot-fellows/runtimes/pull/108 is merged
// decl_test_bridges! {
// 	pub struct PolkadotKusamaMockBridge {
// 		source = BridgeHubPolkadot,
// 		target = BridgeHubKusama,
// 	 handler = PolkadotKusamaMessageHandler
// 	},
// 	pub struct KusamaPolkadotMockBridge {
// 		source = BridgeHubKusama,
// 		target = BridgeHubPolkadot,
// 		handler = KusamaPolkadotMessageHandler
// 	}
// }

// Polkadot implementation
impl_accounts_helpers_for_relay_chain!(Polkadot);
impl_assert_events_helpers_for_relay_chain!(Polkadot);
impl_hrmp_channels_helpers_for_relay_chain!(Polkadot);

// Kusama implementation
impl_accounts_helpers_for_relay_chain!(Kusama);
impl_assert_events_helpers_for_relay_chain!(Kusama);
impl_hrmp_channels_helpers_for_relay_chain!(Kusama);

// AssetHubPolkadot implementation
impl_accounts_helpers_for_parachain!(AssetHubPolkadot);
impl_assets_helpers_for_parachain!(AssetHubPolkadot, Polkadot);
impl_assert_events_helpers_for_parachain!(AssetHubPolkadot);

// AssetHubKusama implementation
impl_accounts_helpers_for_parachain!(AssetHubKusama);
impl_assets_helpers_for_parachain!(AssetHubKusama, Kusama);
impl_assert_events_helpers_for_parachain!(AssetHubKusama);

// PenpalPolkadot implementations
impl_assert_events_helpers_for_parachain!(PenpalPolkadotA);
impl_assert_events_helpers_for_parachain!(PenpalPolkadotB);

// PenpalKusama implementations
impl_assert_events_helpers_for_parachain!(PenpalKusamaA);
impl_assert_events_helpers_for_parachain!(PenpalKusamaB);

// Collectives implementation
impl_accounts_helpers_for_parachain!(Collectives);
impl_assert_events_helpers_for_parachain!(Collectives);

decl_test_sender_receiver_accounts_parameter_types! {
	// Relays
	Polkadot { sender: ALICE, receiver: BOB },
	Kusama { sender: ALICE, receiver: BOB },
	// Asset Hubs
	AssetHubPolkadot { sender: ALICE, receiver: BOB },
	AssetHubKusama { sender: ALICE, receiver: BOB },
	// Collectives
	Collectives { sender: ALICE, receiver: BOB },
	// Bridged Hubs
	BridgeHubPolkadot { sender: ALICE, receiver: BOB },
	BridgeHubKusama { sender: ALICE, receiver: BOB },
	// Penpals
	PenpalPolkadotA { sender: ALICE, receiver: BOB },
	PenpalPolkadotB { sender: ALICE, receiver: BOB },
	PenpalKusamaA { sender: ALICE, receiver: BOB },
	PenpalKusamaB { sender: ALICE, receiver: BOB }
}
