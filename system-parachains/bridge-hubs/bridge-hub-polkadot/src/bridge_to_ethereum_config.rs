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

use crate::{
	xcm_config::{self, RelayNetwork, RelayTreasuryPalletAccount, UniversalLocation},
	Balances, EthereumInboundQueue, EthereumOutboundQueue, EthereumSystem, MessageQueue, Runtime,
	RuntimeEvent, TransactionByteFee,
};
use bp_bridge_hub_polkadot::snowbridge::{CreateAssetCall, InboundQueuePalletInstance, Parameters};
pub use bp_bridge_hub_polkadot::snowbridge::{EthereumLocation, EthereumNetwork};
use frame_support::{parameter_types, weights::ConstantMultiplier};
use pallet_xcm::EnsureXcm;
use parachains_common::{AccountId, Balance};
use snowbridge_beacon_primitives::{Fork, ForkVersions};
use snowbridge_core::AllowSiblingsOnly;
use snowbridge_router_primitives::{inbound::MessageToXcm, outbound::EthereumBlobExporter};
use sp_core::H160;
use sp_runtime::traits::{ConstU32, ConstU8, Keccak256};
use system_parachains_constants::polkadot::fee::WeightToFee;
use xcm::prelude::{GlobalConsensus, InteriorLocation, Location, Parachain};

/// Exports message to the Ethereum Gateway contract.
pub type SnowbridgeExporter = EthereumBlobExporter<
	UniversalLocation,
	EthereumNetwork,
	snowbridge_pallet_outbound_queue::Pallet<Runtime>,
	snowbridge_core::AgentIdOf,
	EthereumSystem,
>;

parameter_types! {
	// The gateway address is set by governance.
	pub storage EthereumGatewayAddress: H160 = H160::zero();
	pub AssetHubFromEthereum: Location = Location::new(1,[GlobalConsensus(RelayNetwork::get()),Parachain(polkadot_runtime_constants::system_parachain::ASSET_HUB_ID)]);
	pub EthereumUniversalLocation: InteriorLocation = [GlobalConsensus(EthereumNetwork::get())].into();
}

impl snowbridge_pallet_inbound_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Verifier = snowbridge_pallet_ethereum_client::Pallet<Runtime>;
	type Token = Balances;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type XcmSender = xcm_config::XcmRouter;
	#[cfg(feature = "runtime-benchmarks")]
	type XcmSender = benchmark_helpers::DoNothingRouter;
	type ChannelLookup = EthereumSystem;
	type GatewayAddress = EthereumGatewayAddress;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = Runtime;
	type MessageConverter = MessageToXcm<
		CreateAssetCall,
		bp_asset_hub_polkadot::CreateForeignAssetDeposit,
		InboundQueuePalletInstance,
		AccountId,
		Balance,
		EthereumSystem,
		EthereumUniversalLocation,
		AssetHubFromEthereum,
	>;
	type WeightToFee = WeightToFee;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type MaxMessageSize = ConstU32<2048>;
	type WeightInfo = crate::weights::snowbridge_pallet_inbound_queue::WeightInfo<Runtime>;
	type PricingParameters = EthereumSystem;
	type AssetTransactor = <xcm_config::XcmConfig as xcm_executor::Config>::AssetTransactor;
}

impl snowbridge_pallet_outbound_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Hashing = Keccak256;
	type MessageQueue = MessageQueue;
	type Decimals = ConstU8<10>;
	type MaxMessagePayloadSize = ConstU32<2048>;
	type MaxMessagesPerBlock = ConstU32<32>;
	type GasMeter = snowbridge_core::outbound::ConstantGasMeter;
	type Balance = Balance;
	type WeightToFee = WeightToFee;
	type WeightInfo = crate::weights::snowbridge_pallet_outbound_queue::WeightInfo<Runtime>;
	type PricingParameters = EthereumSystem;
	type Channels = EthereumSystem;
}

#[cfg(not(any(feature = "std", feature = "runtime-benchmarks", test)))]
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

#[cfg(any(feature = "std", feature = "runtime-benchmarks", test))]
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

pub const SLOTS_PER_EPOCH: u32 = snowbridge_pallet_ethereum_client::config::SLOTS_PER_EPOCH as u32;

impl snowbridge_pallet_ethereum_client::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ForkVersions = ChainForkVersions;
	type FreeHeadersInterval = ConstU32<SLOTS_PER_EPOCH>;
	type WeightInfo = crate::weights::snowbridge_pallet_ethereum_client::WeightInfo<Runtime>;
}

impl snowbridge_pallet_system::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OutboundQueue = EthereumOutboundQueue;
	type SiblingOrigin = EnsureXcm<AllowSiblingsOnly>;
	type AgentIdOf = snowbridge_core::AgentIdOf;
	type TreasuryAccount = RelayTreasuryPalletAccount;
	type Token = Balances;
	type WeightInfo = crate::weights::snowbridge_pallet_system::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = Runtime;
	type DefaultPricingParameters = Parameters;
	type InboundDeliveryCost = EthereumInboundQueue;
	type UniversalLocation = UniversalLocation;
	type EthereumLocation = EthereumLocation;
}

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmark_helpers {
	use super::{EthereumGatewayAddress, RelayTreasuryPalletAccount, Runtime};
	use crate::{Balances, EthereumBeaconClient, ExistentialDeposit, RuntimeOrigin};
	use codec::Encode;
	use frame_support::traits::fungible;
	use hex_literal::hex;
	use snowbridge_beacon_primitives::BeaconHeader;
	use snowbridge_pallet_inbound_queue::BenchmarkHelper;
	use sp_core::H256;
	use xcm::latest::{Assets, Location, SendError, SendResult, SendXcm, Xcm, XcmHash};

	impl<T: snowbridge_pallet_ethereum_client::Config> BenchmarkHelper<T> for Runtime {
		fn initialize_storage(beacon_header: BeaconHeader, block_roots_root: H256) {
			EthereumBeaconClient::store_finalized_header(beacon_header, block_roots_root).unwrap();
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

	impl snowbridge_pallet_system::BenchmarkHelper<RuntimeOrigin> for Runtime {
		fn make_xcm_origin(location: Location) -> RuntimeOrigin {
			// Drip ED to the `TreasuryAccount`
			<Balances as fungible::Mutate<_>>::set_balance(
				&RelayTreasuryPalletAccount::get(),
				ExistentialDeposit::get(),
			);

			RuntimeOrigin::from(pallet_xcm::Origin::Xcm(location))
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn bridge_hub_inbound_queue_pallet_index_is_correct() {
		assert_eq!(
			InboundQueuePalletInstance::get(),
			<EthereumInboundQueue as frame_support::traits::PalletInfoAccess>::index() as u8
		);
	}
}
