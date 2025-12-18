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
	bridge_common_config::BridgeReward,
	xcm_config::{self, RelayNetwork, RelayTreasuryPalletAccount, RootLocation, UniversalLocation},
	Balances, BridgeRelayers, EthereumBeaconClient, EthereumInboundQueue, EthereumInboundQueueV2,
	EthereumOutboundQueue, EthereumOutboundQueueV2, EthereumSystem, EthereumSystemV2, MessageQueue,
	Runtime, RuntimeEvent, TransactionByteFee,
};
use bp_asset_hub_polkadot::SystemFrontendPalletInstance;
use bp_bridge_hub_polkadot::snowbridge::{
	CreateAssetCall, InboundQueuePalletInstance, InboundQueueV2PalletInstance, Parameters,
};
pub use bp_bridge_hub_polkadot::snowbridge::{EthereumLocation, EthereumNetwork};
use frame_support::{parameter_types, traits::Contains, weights::ConstantMultiplier};
use frame_system::EnsureRootWithSuccess;
use hex_literal::hex;
use pallet_xcm::EnsureXcm;
use parachains_common::{AccountId, Balance};
use polkadot_runtime_constants::system_parachain::AssetHubParaId;
use snowbridge_beacon_primitives::{Fork, ForkVersions};
use snowbridge_core::AllowSiblingsOnly;
use snowbridge_inbound_queue_primitives::v1::MessageToXcm;
use snowbridge_outbound_queue_primitives::{
	v1::{ConstantGasMeter, EthereumBlobExporter},
	v2::{ConstantGasMeter as ConstantGasMeterV2, EthereumBlobExporter as EthereumBlobExporterV2},
};
use sp_core::H160;
use sp_runtime::traits::{ConstU32, ConstU8, Keccak256};
use system_parachains_constants::polkadot::fee::WeightToFee;
use xcm::prelude::{GlobalConsensus, InteriorLocation, Location, PalletInstance, Parachain};
use xcm_executor::XcmExecutor;

pub const SLOTS_PER_EPOCH: u32 = snowbridge_pallet_ethereum_client::config::SLOTS_PER_EPOCH as u32;

/// Exports message to the Ethereum Gateway contract.
pub type SnowbridgeExporter = EthereumBlobExporter<
	UniversalLocation,
	EthereumNetwork,
	snowbridge_pallet_outbound_queue::Pallet<Runtime>,
	snowbridge_core::AgentIdOf,
	EthereumSystem,
>;

pub type SnowbridgeExporterV2 = EthereumBlobExporterV2<
	UniversalLocation,
	EthereumNetwork,
	EthereumOutboundQueueV2,
	EthereumSystemV2,
	AssetHubParaId,
>;

parameter_types! {
	// The gateway address is set by governance.
	pub storage EthereumGatewayAddress: H160 = H160::zero();
	pub AssetHubFromEthereum: Location = Location::new(1, [GlobalConsensus(RelayNetwork::get()),Parachain(polkadot_runtime_constants::system_parachain::ASSET_HUB_ID)]);
	pub EthereumUniversalLocation: InteriorLocation = [GlobalConsensus(EthereumNetwork::get())].into();
	pub AssetHubUniversalLocation: InteriorLocation = [GlobalConsensus(RelayNetwork::get()), Parachain(polkadot_runtime_constants::system_parachain::ASSET_HUB_ID)].into();
	pub InboundQueueV2Location: InteriorLocation = [PalletInstance(InboundQueueV2PalletInstance::get())].into();
	pub const SnowbridgeReward: BridgeReward = BridgeReward::Snowbridge;
	pub SnowbridgeFrontendLocation: Location = Location::new(1, [Parachain(polkadot_runtime_constants::system_parachain::ASSET_HUB_ID), PalletInstance(SystemFrontendPalletInstance::get())]);
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

impl snowbridge_pallet_inbound_queue_v2::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Verifier = EthereumBeaconClient;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type XcmSender = xcm_config::XcmRouter;
	#[cfg(feature = "runtime-benchmarks")]
	type XcmSender = benchmark_helpers::DoNothingRouter;
	type GatewayAddress = EthereumGatewayAddress;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = Runtime;
	type WeightInfo = crate::weights::snowbridge_pallet_inbound_queue_v2::WeightInfo<Runtime>;
	type AssetHubParaId = AssetHubParaId;
	type XcmExecutor = XcmExecutor<xcm_config::XcmConfig>;
	type MessageConverter = snowbridge_inbound_queue_primitives::v2::MessageToXcm<
		CreateAssetCall,
		bp_asset_hub_polkadot::CreateForeignAssetDeposit,
		EthereumNetwork,
		InboundQueueV2Location,
		EthereumSystem,
		EthereumGatewayAddress,
		EthereumUniversalLocation,
		AssetHubFromEthereum,
		AssetHubUniversalLocation,
		AccountId,
	>;
	type AccountToLocation = xcm_builder::AliasesIntoAccountId32<
		xcm_config::RelayNetwork,
		<Runtime as frame_system::Config>::AccountId,
	>;
	type RewardKind = BridgeReward;
	type DefaultRewardKind = SnowbridgeReward;
	type RewardPayment = BridgeRelayers;
}

impl snowbridge_pallet_outbound_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Hashing = Keccak256;
	type MessageQueue = MessageQueue;
	type Decimals = ConstU8<10>;
	type MaxMessagePayloadSize = ConstU32<2048>;
	type MaxMessagesPerBlock = ConstU32<32>;
	type GasMeter = ConstantGasMeter;
	type Balance = Balance;
	type WeightToFee = WeightToFee;
	type WeightInfo = crate::weights::snowbridge_pallet_outbound_queue::WeightInfo<Runtime>;
	type PricingParameters = EthereumSystem;
	type Channels = EthereumSystem;
}

impl snowbridge_pallet_outbound_queue_v2::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Hashing = Keccak256;
	type MessageQueue = MessageQueue;
	// Maximum payload size for outbound messages.
	type MaxMessagePayloadSize = ConstU32<2048>;
	// Maximum number of outbound messages that can be committed per block.
	// It's benchmarked, including the entire process flow(initialize,submit,commit) in the
	// worst-case, Benchmark results in `../weights/snowbridge_pallet_outbound_queue_v2.
	// rs` show that the `process` function consumes less than 1% of the block capacity, which is
	// safe enough.
	type MaxMessagesPerBlock = ConstU32<32>;
	type GasMeter = ConstantGasMeterV2;
	type Balance = Balance;
	type WeightToFee = WeightToFee;
	type Verifier = EthereumBeaconClient;
	type GatewayAddress = EthereumGatewayAddress;
	type WeightInfo = crate::weights::snowbridge_pallet_outbound_queue_v2::WeightInfo<Runtime>;
	type EthereumNetwork = EthereumNetwork;
	type RewardKind = BridgeReward;
	type DefaultRewardKind = SnowbridgeReward;
	type RewardPayment = BridgeRelayers;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = Runtime;
}

#[cfg(not(any(feature = "std", feature = "runtime-benchmarks", test)))]
parameter_types! {
	pub const ChainForkVersions: ForkVersions = ForkVersions {
		genesis: Fork {
			version: hex!("00000000"),
			epoch: 0,
		},
		altair: Fork {
			version: hex!("01000000"),
			epoch: 74240,
		},
		bellatrix: Fork {
			version: hex!("02000000"),
			epoch: 144896,
		},
		capella: Fork {
			version: hex!("03000000"),
			epoch: 194048,
		},
		deneb: Fork {
			version: hex!("04000000"),
			epoch: 269568,
		},
		electra: Fork {
			version: hex!("05000000"),
			epoch: 364032,
		},
		fulu: Fork {
			version: hex!("06000000"), // https://notes.ethereum.org/@bbusa/fusaka-bpo-timeline
			epoch: 411392,
		},
	};
}

#[cfg(any(feature = "std", feature = "runtime-benchmarks", test))]
parameter_types! {
	pub const ChainForkVersions: ForkVersions = ForkVersions {
		genesis: Fork {
			version: hex!("00000000"),
			epoch: 0,
		},
		altair: Fork {
			version: hex!("01000000"),
			epoch: 0,
		},
		bellatrix: Fork {
			version: hex!("02000000"),
			epoch: 0,
		},
		capella: Fork {
			version: hex!("03000000"),
			epoch: 0,
		},
		deneb: Fork {
			version: hex!("04000000"),
			epoch: 0,
		},
		electra: Fork {
			version: hex!("05000000"),
			epoch: 0,
		},
		fulu: Fork {
			version: hex!("06000000"),
			epoch: 50000000,
		},
	};
}

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

pub struct AllowFromEthereumFrontend;
impl Contains<Location> for AllowFromEthereumFrontend {
	fn contains(location: &Location) -> bool {
		match location.unpack() {
			(1, [Parachain(para_id), PalletInstance(index)]) =>
				*para_id == polkadot_runtime_constants::system_parachain::ASSET_HUB_ID &&
					*index == SystemFrontendPalletInstance::get(),
			_ => false,
		}
	}
}

impl snowbridge_pallet_system_v2::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OutboundQueue = EthereumOutboundQueueV2;
	type InboundQueue = EthereumInboundQueueV2;
	type FrontendOrigin = EnsureXcm<AllowFromEthereumFrontend>;
	type WeightInfo = crate::weights::snowbridge_pallet_system_v2::WeightInfo<Runtime>;
	type GovernanceOrigin = EnsureRootWithSuccess<crate::AccountId, RootLocation>;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = ();
}

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmark_helpers {
	use super::{EthereumGatewayAddress, RelayTreasuryPalletAccount, Runtime};
	use crate::{Balances, EthereumBeaconClient, ExistentialDeposit, RuntimeOrigin};
	use codec::Encode;
	use frame_support::{parameter_types, traits::fungible};
	use hex_literal::hex;
	use snowbridge_beacon_primitives::BeaconHeader;
	use snowbridge_pallet_inbound_queue::BenchmarkHelper;
	use snowbridge_pallet_inbound_queue_v2::BenchmarkHelper as InboundQueueBenchmarkHelperV2;
	use snowbridge_pallet_outbound_queue_v2::BenchmarkHelper as OutboundQueueBenchmarkHelperV2;
	use sp_core::{H160, H256};
	use xcm::latest::{Assets, Location, SendError, SendResult, SendXcm, Xcm, XcmHash};

	parameter_types! {
		// The fixture data for benchmark tests in the Polkadot SDK relies on these gateway addresses,
		// which is validated in the pallets.
		pub EthereumGatewayAddressV1: H160 = hex!["eda338e4dc46038493b885327842fd3e301cab39"].into();
		pub EthereumGatewayAddressV2: H160 = hex!["b1185ede04202fe62d38f5db72f71e38ff3e8305"].into();
	}

	impl<T: snowbridge_pallet_ethereum_client::Config> BenchmarkHelper<T> for Runtime {
		fn initialize_storage(beacon_header: BeaconHeader, block_roots_root: H256) {
			initialize_storage_for_benchmarks(
				EthereumGatewayAddressV1::get(),
				beacon_header,
				block_roots_root,
			);
		}
	}

	impl<T: snowbridge_pallet_inbound_queue_v2::Config> InboundQueueBenchmarkHelperV2<T> for Runtime {
		fn initialize_storage(beacon_header: BeaconHeader, block_roots_root: H256) {
			initialize_storage_for_benchmarks(
				EthereumGatewayAddressV2::get(),
				beacon_header,
				block_roots_root,
			);
		}
	}

	impl<T: snowbridge_pallet_outbound_queue_v2::Config> OutboundQueueBenchmarkHelperV2<T> for Runtime {
		fn initialize_storage(beacon_header: BeaconHeader, block_roots_root: H256) {
			initialize_storage_for_benchmarks(
				EthereumGatewayAddressV2::get(),
				beacon_header,
				block_roots_root,
			);
		}
	}

	fn initialize_storage_for_benchmarks(
		gateway_address: H160,
		beacon_header: BeaconHeader,
		block_roots_root: H256,
	) {
		EthereumBeaconClient::store_finalized_header(beacon_header, block_roots_root).unwrap();
		EthereumGatewayAddress::set(&gateway_address);
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

	impl snowbridge_pallet_system_v2::BenchmarkHelper<RuntimeOrigin> for () {
		fn make_xcm_origin(location: Location) -> RuntimeOrigin {
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

	#[test]
	fn bridge_hub_inbound_v2_queue_pallet_index_is_correct() {
		assert_eq!(
			InboundQueueV2PalletInstance::get(),
			<EthereumInboundQueueV2 as frame_support::traits::PalletInfoAccess>::index() as u8
		);
	}
}
