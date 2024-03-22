// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Cumulus.

// Cumulus is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Cumulus is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Cumulus.  If not, see <http://www.gnu.org/licenses/>.

use crate::{xcm_config::UniversalLocation, Runtime};
pub use bp_bridge_hub_polkadot::snowbridge::EthereumNetwork;
use frame_support::parameter_types;
use snowbridge_beacon_primitives::{Fork, ForkVersions};
use snowbridge_router_primitives::outbound::EthereumBlobExporter;
use sp_core::H160;

/// Exports message to the Ethereum Gateway contract.
pub type SnowbridgeExporter = EthereumBlobExporter<
	UniversalLocation,
	EthereumNetwork,
	snowbridge_pallet_outbound_queue::Pallet<Runtime>,
	snowbridge_core::AgentIdOf,
>;

parameter_types! {
	// The gateway address is set by governance.
	pub storage EthereumGatewayAddress: H160 = H160::zero();
	pub const MaxExecutionHeadersToKeep: u32 = 8192 * 20;
}

#[cfg(not(any(feature = "std", feature = "fast-runtime", feature = "runtime-benchmarks", test)))]
parameter_types! {
	pub const ChainForkVersions: ForkVersions = ForkVersions {
		genesis: Fork {
			version: [0, 0, 0, 0], // 0x00000000
			epoch: 0,
		},
		altair: Fork {
			version: [1, 0, 0, 0], // 0x01000000
			epoch: 74240,
		},
		bellatrix: Fork {
			version: [2, 0, 0, 0], // 0x02000000
			epoch: 144896,
		},
		capella: Fork {
			version: [3, 0, 0, 0], // 0x03000000
			epoch: 194048,
		},
		deneb: Fork {
			version: [4, 0, 0, 0], // 0x04000000
			epoch: 269568,
		},
	};
}

#[cfg(any(feature = "std", feature = "fast-runtime", feature = "runtime-benchmarks", test))]
parameter_types! {
	pub const ChainForkVersions: ForkVersions = ForkVersions {
		genesis: Fork {
			version: [0, 0, 0, 0], // 0x00000000
			epoch: 0,
		},
		altair: Fork {
			version: [1, 0, 0, 0], // 0x01000000
			epoch: 0,
		},
		bellatrix: Fork {
			version: [2, 0, 0, 0], // 0x02000000
			epoch: 0,
		},
		capella: Fork {
			version: [3, 0, 0, 0], // 0x03000000
			epoch: 0,
		},
		deneb: Fork {
			version: [4, 0, 0, 0], // 0x04000000
			epoch: 0,
		}
	};
}

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmark_helpers {
	use crate::{bridge_to_ethereum_config::EthereumGatewayAddress, EthereumBeaconClient, Runtime, RuntimeOrigin};
	use codec::Encode;
	use hex_literal::hex;
	use snowbridge_beacon_primitives::CompactExecutionHeader;
	use snowbridge_pallet_inbound_queue::BenchmarkHelper;
	use sp_core::H256;
	use xcm::latest::{Assets, Location, SendError, SendResult, SendXcm, Xcm, XcmHash};

	impl<T: snowbridge_pallet_ethereum_client::Config> BenchmarkHelper<T> for Runtime {
		fn initialize_storage(block_hash: H256, header: CompactExecutionHeader) {
			EthereumBeaconClient::store_execution_header(block_hash, header, 0, H256::default());
			EthereumGatewayAddress::set(&hex!["EDa338E4dC46038493b885327842fD3E301CaB39"].into());
		}
	}

	pub struct DoNothingRouter;
	impl SendXcm for DoNothingRouter {
		type Ticket = Xcm<()>;

		fn validate(
			_dest: &mut Option<Location>,
			xcm: &mut Option<Xcm<()>>,
		) -> SendResult<Self::Ticket> {
			Ok((xcm.clone().unwrap(), Assets::new()))
		}
		fn deliver(xcm: Xcm<()>) -> Result<XcmHash, SendError> {
			let hash = xcm.using_encoded(sp_io::hashing::blake2_256);
			Ok(hash)
		}
	}

	impl snowbridge_pallet_system::BenchmarkHelper<RuntimeOrigin> for () {
		fn make_xcm_origin(location: Location) -> RuntimeOrigin {
			RuntimeOrigin::from(pallet_xcm::Origin::Xcm(location))
		}
	}
}
