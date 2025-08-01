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

//! Bridge definitions used for bridging with Kusama Bridge Hub.

use crate::{
	bridge_common_config::BridgeRelayersInstance,
	weights,
	xcm_config::{UniversalLocation, XcmRouter},
	AccountId, Balance, Balances, BridgeKusamaMessages, PolkadotXcm, Runtime, RuntimeEvent,
	RuntimeHoldReason, XcmOverBridgeHubKusama, XcmpQueue,
};

pub use bp_bridge_hub_kusama::bp_kusama;
use bp_messages::{
	source_chain::FromBridgedChainMessagesDeliveryProof,
	target_chain::FromBridgedChainMessagesProof, LegacyLaneId,
};
use bp_parachains::SingleParaStoredHeaderDataBuilder;
use bp_runtime::Chain;
use bridge_hub_common::xcm_version::XcmVersionOfDestAndRemoteBridge;
use frame_support::{
	parameter_types,
	traits::{ConstU128, PalletInfoAccess},
};
use frame_system::{EnsureNever, EnsureRoot};
use pallet_bridge_messages::LaneIdOf;
use pallet_bridge_relayers::extension::{
	BridgeRelayersTransactionExtension, WithMessagesExtensionConfig,
};
use pallet_xcm_bridge_hub::{BridgeId, XcmAsPlainPayload};
use parachains_common::xcm_config::{AllSiblingSystemParachains, RelayOrOtherSystemParachains};
use polkadot_parachain_primitives::primitives::Sibling;
use polkadot_runtime_constants as constants;
use sp_runtime::traits::ConstU32;
use xcm::latest::prelude::*;
use xcm_builder::{BridgeBlobDispatcher, ParentIsPreset, SiblingParachainConvertsVia};

// Parameters that may be changed by the governance.
parameter_types! {
	/// Reward that is paid (by the Polkadot Asset Hub) to relayers for delivering a single
	/// Polkadot -> Kusama bridge message.
	///
	/// This payment is tracked by the `pallet_bridge_relayers` pallet at the Polkadot
	/// Bridge Hub.
	pub storage DeliveryRewardInBalance: Balance = constants::currency::UNITS / 2_000;
}

// Parameters, used by both XCM and bridge code.
parameter_types! {
	/// Kusama Network identifier.
	pub KusamaGlobalConsensusNetwork: NetworkId = NetworkId::Kusama;
	/// Kusama Network as `Location`.
	pub KusamaGlobalConsensusNetworkLocation: Location = Location {
		parents: 2,
		interior: [GlobalConsensus(KusamaGlobalConsensusNetwork::get())].into()
	};
	/// Interior location (relative to this runtime) of the with-Kusama messages pallet.
	pub BridgePolkadotToKusamaMessagesPalletInstance: InteriorLocation = PalletInstance(<BridgeKusamaMessages as PalletInfoAccess>::index() as u8).into();

	/// Location of the bridged Kusama Bridge Hub parachain.
	pub BridgeHubKusamaLocation: Location = Location {
		parents: 2,
		interior: [
			GlobalConsensus(KusamaGlobalConsensusNetwork::get()),
			Parachain(<bp_bridge_hub_kusama::BridgeHubKusama as bp_runtime::Parachain>::PARACHAIN_ID)
		].into()
	};
}

// Parameters, used by bridge transport code.
parameter_types! {
	/// Number of Kusama headers to keep in the runtime storage.
	///
	/// Note that we are keeping only required header information, not the whole header itself. Roughly, it
	/// is the 2 hours of real time (assuming that every header is submitted).
	pub const RelayChainHeadersToKeep: u32 = 1_200;
	/// Number of Kusama Bridge Hub headers to keep in the runtime storage.
	///
	/// Note that we are keeping only required header information, not the whole header itself. Roughly, it
	/// is the 2 hours of real time (assuming that every header is submitted).
	pub const ParachainHeadsToKeep: u32 = 600;
	/// Maximal size of Kusama Bridge Hub header **part** that we are storing in the runtime storage.
	pub const MaxParaHeadDataSize: u32 = bp_kusama::MAX_NESTED_PARACHAIN_HEAD_DATA_SIZE;

	/// Bridge specific chain (network) identifier of the Kusama Bridge Hub.
	pub const BridgeHubKusamaChainId: bp_runtime::ChainId = bp_bridge_hub_kusama::BridgeHubKusama::ID;
	/// Name of the `paras` pallet at Kusama that tracks all parachain heads.
	pub const ParachainPalletNameAtKusama: &'static str = bp_kusama::PARAS_PALLET_NAME;

	// see the `FEE_BOOST_PER_MESSAGE` constant to get the meaning of this value
	pub PriorityBoostPerMessage: u64 = 3_641_799_307_958;
}

/// Proof of messages, coming from Kusama.
pub type FromKusamaBridgeHubMessagesProof<MI> =
	FromBridgedChainMessagesProof<bp_bridge_hub_kusama::Hash, LaneIdOf<Runtime, MI>>;
/// Messages delivery proof for Kusama Bridge Hub -> Polkadot Bridge Hub messages.
pub type ToKusamaBridgeHubMessagesDeliveryProof<MI> =
	FromBridgedChainMessagesDeliveryProof<bp_bridge_hub_kusama::Hash, LaneIdOf<Runtime, MI>>;

/// Dispatches received XCM messages from other bridge
pub type FromKusamaMessageBlobDispatcher = BridgeBlobDispatcher<
	XcmRouter,
	UniversalLocation,
	BridgePolkadotToKusamaMessagesPalletInstance,
>;

/// Signed extension that refunds relayers that are delivering messages from the Kusama parachain.
pub type OnBridgeHubPolkadotRefundBridgeHubKusamaMessages = BridgeRelayersTransactionExtension<
	Runtime,
	WithMessagesExtensionConfig<
		StrOnBridgeHubPolkadotRefundBridgeHubKusamaMessages,
		Runtime,
		WithBridgeHubKusamaMessagesInstance,
		BridgeRelayersInstance,
		PriorityBoostPerMessage,
	>,
>;
bp_runtime::generate_static_str_provider!(OnBridgeHubPolkadotRefundBridgeHubKusamaMessages);

/// Add GRANDPA bridge pallet to track Kusama relay chain.
pub type BridgeGrandpaKusamaInstance = pallet_bridge_grandpa::Instance1;
impl pallet_bridge_grandpa::Config<BridgeGrandpaKusamaInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type BridgedChain = bp_kusama::Kusama;
	type HeadersToKeep = RelayChainHeadersToKeep;
	type MaxFreeHeadersPerBlock = ConstU32<4>;
	type FreeHeadersInterval = ConstU32<5>;
	type WeightInfo = weights::pallet_bridge_grandpa::WeightInfo<Runtime>;
}

/// Add parachain bridge pallet to track Kusama BridgeHub parachain.
pub type BridgeParachainKusamaInstance = pallet_bridge_parachains::Instance1;
impl pallet_bridge_parachains::Config<BridgeParachainKusamaInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_bridge_parachains::WeightInfo<Runtime>;
	type BridgesGrandpaPalletInstance = BridgeGrandpaKusamaInstance;
	type ParasPalletName = ParachainPalletNameAtKusama;
	type ParaStoredHeaderDataBuilder =
		SingleParaStoredHeaderDataBuilder<bp_bridge_hub_kusama::BridgeHubKusama>;
	type HeadsToKeep = ParachainHeadsToKeep;
	type MaxParaHeadDataSize = MaxParaHeadDataSize;
	type OnNewHead = ();
}

/// Add XCM messages support for exchanging messages with BridgeHubKusama.
pub type WithBridgeHubKusamaMessagesInstance = pallet_bridge_messages::Instance1;
impl pallet_bridge_messages::Config<WithBridgeHubKusamaMessagesInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_bridge_messages::WeightInfo<Runtime>;

	type ThisChain = bp_bridge_hub_polkadot::BridgeHubPolkadot;
	type BridgedChain = bp_bridge_hub_kusama::BridgeHubKusama;
	type BridgedHeaderChain = pallet_bridge_parachains::ParachainHeaders<
		Runtime,
		BridgeParachainKusamaInstance,
		bp_bridge_hub_kusama::BridgeHubKusama,
	>;

	type OutboundPayload = XcmAsPlainPayload;
	type InboundPayload = XcmAsPlainPayload;
	type LaneId = LegacyLaneId;

	type DeliveryPayments = ();
	type DeliveryConfirmationPayments = pallet_bridge_relayers::DeliveryConfirmationPaymentsAdapter<
		Runtime,
		WithBridgeHubKusamaMessagesInstance,
		BridgeRelayersInstance,
		DeliveryRewardInBalance,
	>;

	type MessageDispatch = XcmOverBridgeHubKusama;
	type OnMessagesDelivered = XcmOverBridgeHubKusama;
}

/// Add support for the export and dispatch of XCM programs.
pub type XcmOverBridgeHubKusamaInstance = pallet_xcm_bridge_hub::Instance1;
impl pallet_xcm_bridge_hub::Config<XcmOverBridgeHubKusamaInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;

	type UniversalLocation = UniversalLocation;
	type BridgedNetwork = KusamaGlobalConsensusNetworkLocation;
	type BridgeMessagesPalletInstance = WithBridgeHubKusamaMessagesInstance;
	// `MessageExportPrice` is simply propagated to the inner `xcm_builder::HaulBlobExporter`, and
	// we do not need or want to add any additional price for exporting here, as it is already
	// covered by the measured weight of the `ExportMessage` instruction.
	type MessageExportPrice = ();
	type DestinationVersion = XcmVersionOfDestAndRemoteBridge<PolkadotXcm, BridgeHubKusamaLocation>;

	type ForceOrigin = EnsureRoot<AccountId>;
	// We don't want to allow creating bridges for this instance with `LegacyLaneId`.
	type OpenBridgeOrigin = EnsureNever<Location>;
	// Converter aligned with `OpenBridgeOrigin`.
	type BridgeOriginAccountIdConverter =
		(ParentIsPreset<AccountId>, SiblingParachainConvertsVia<Sibling, AccountId>);

	// We do not allow creating bridges here (see `T::OpenBridgeOrigin` above), so there is no need
	// to set a deposit.
	type BridgeDeposit = ConstU128<0>;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type AllowWithoutBridgeDeposit =
		RelayOrOtherSystemParachains<AllSiblingSystemParachains, Runtime>;

	type LocalXcmChannelManager = CongestionManager;
	type BlobDispatcher = FromKusamaMessageBlobDispatcher;
}

/// Implementation of `bp_xcm_bridge_hub::LocalXcmChannelManager` for congestion management.
pub struct CongestionManager;
impl pallet_xcm_bridge_hub::LocalXcmChannelManager for CongestionManager {
	type Error = SendError;

	fn is_congested(with: &Location) -> bool {
		// This is used to check the inbound bridge queue/messages to determine if they can be
		// dispatched and sent to the sibling parachain. Therefore, checking outbound `XcmpQueue`
		// is sufficient here.
		use bp_xcm_bridge_hub_router::XcmChannelStatusProvider;
		cumulus_pallet_xcmp_queue::bridging::OutXcmpChannelStatusProvider::<Runtime>::is_congested(
			with,
		)
	}

	fn suspend_bridge(local_origin: &Location, bridge: BridgeId) -> Result<(), Self::Error> {
		// This bridge is intended for AH<>AH communication with a hard-coded/static lane,
		// so `local_origin` is expected to represent only the local AH.
		send_xcm::<XcmpQueue>(
			local_origin.clone(),
			bp_asset_hub_kusama::build_congestion_message(bridge.inner(), true).into(),
		)
		.map(|_| ())
	}

	fn resume_bridge(local_origin: &Location, bridge: BridgeId) -> Result<(), Self::Error> {
		// This bridge is intended for AH<>AH communication with a hard-coded/static lane,
		// so `local_origin` is expected to represent only the local AH.
		send_xcm::<XcmpQueue>(
			local_origin.clone(),
			bp_asset_hub_kusama::build_congestion_message(bridge.inner(), false).into(),
		)
		.map(|_| ())
	}
}

#[cfg(feature = "runtime-benchmarks")]
pub(crate) fn open_bridge_for_benchmarks<R, XBHI, C>(
	with: pallet_xcm_bridge_hub::LaneIdOf<R, XBHI>,
	sibling_para_id: u32,
) -> InteriorLocation
where
	R: pallet_xcm_bridge_hub::Config<XBHI>,
	XBHI: 'static,
	C: xcm_executor::traits::ConvertLocation<
		bp_runtime::AccountIdOf<pallet_xcm_bridge_hub::ThisChainOf<R, XBHI>>,
	>,
{
	use alloc::boxed::Box;
	use pallet_xcm_bridge_hub::{Bridge, BridgeId, BridgeState};
	use sp_runtime::traits::Zero;
	use xcm::VersionedInteriorLocation;

	// insert bridge metadata
	let lane_id = with;
	let sibling_parachain = Location::new(1, [Parachain(sibling_para_id)]);
	let universal_source = [GlobalConsensus(Polkadot), Parachain(sibling_para_id)].into();
	let universal_destination = [GlobalConsensus(Kusama), Parachain(2075)].into();
	let bridge_id = BridgeId::new(&universal_source, &universal_destination);

	// insert only bridge metadata, because the benchmarks create lanes
	pallet_xcm_bridge_hub::Bridges::<R, XBHI>::insert(
		bridge_id,
		Bridge {
			bridge_origin_relative_location: Box::new(sibling_parachain.clone().into()),
			bridge_origin_universal_location: Box::new(VersionedInteriorLocation::from(
				universal_source.clone(),
			)),
			bridge_destination_universal_location: Box::new(VersionedInteriorLocation::from(
				universal_destination,
			)),
			state: BridgeState::Opened,
			bridge_owner_account: C::convert_location(&sibling_parachain).expect("valid AccountId"),
			deposit: Zero::zero(),
			lane_id,
		},
	);
	pallet_xcm_bridge_hub::LaneToBridge::<R, XBHI>::insert(lane_id, bridge_id);

	universal_source
}

#[cfg(test)]
mod tests {
	use super::*;
	use bridge_runtime_common::{
		assert_complete_bridge_types,
		integrity::{
			assert_complete_with_parachain_bridge_constants, check_message_lane_weights,
			AssertChainConstants, AssertCompleteBridgeConstants,
		},
	};

	/// Every additional message in the message delivery transaction boosts its priority.
	/// So the priority of transaction with `N+1` messages is larger than priority of
	/// transaction with `N` messages by the `PriorityBoostPerMessage`.
	///
	/// Economically, it is an equivalent of adding tip to the transaction with `N` messages.
	/// The `FEE_BOOST_PER_MESSAGE` constant is the value of this tip.
	///
	/// We want this tip to be large enough (delivery transactions with more messages = less
	/// operational costs and a faster bridge), so this value should be significant.
	const FEE_BOOST_PER_MESSAGE: Balance = 2 * constants::currency::UNITS;

	#[test]
	fn ensure_bridge_hub_polkadot_message_lane_weights_are_correct() {
		use bp_messages::ChainWithMessages;
		check_message_lane_weights::<
			bp_bridge_hub_polkadot::BridgeHubPolkadot,
			Runtime,
			WithBridgeHubKusamaMessagesInstance,
		>(
			bp_bridge_hub_kusama::EXTRA_STORAGE_PROOF_SIZE,
			bp_bridge_hub_polkadot::BridgeHubPolkadot::MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX,
			bp_bridge_hub_polkadot::BridgeHubPolkadot::MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX,
			true,
		);
	}

	#[test]
	fn ensure_bridge_integrity() {
		assert_complete_bridge_types!(
			runtime: Runtime,
			with_bridged_chain_messages_instance: WithBridgeHubKusamaMessagesInstance,
			this_chain: bp_bridge_hub_polkadot::BridgeHubPolkadot,
			bridged_chain: bp_bridge_hub_kusama::BridgeHubKusama,
			expected_payload_type: XcmAsPlainPayload,
		);

		assert_complete_with_parachain_bridge_constants::<
			Runtime,
			BridgeParachainKusamaInstance,
			WithBridgeHubKusamaMessagesInstance,
		>(AssertCompleteBridgeConstants {
			this_chain_constants: AssertChainConstants {
				block_length: bp_bridge_hub_polkadot::BlockLength::get(),
				block_weights: bp_bridge_hub_polkadot::BlockWeights::get(),
			},
		});

		pallet_bridge_relayers::extension::per_message::ensure_priority_boost_is_sane::<
			Runtime,
			WithBridgeHubKusamaMessagesInstance,
			PriorityBoostPerMessage,
		>(FEE_BOOST_PER_MESSAGE);

		assert_eq!(
			BridgePolkadotToKusamaMessagesPalletInstance::get(),
			Into::<InteriorLocation>::into(PalletInstance(
				bp_bridge_hub_polkadot::WITH_BRIDGE_POLKADOT_TO_KUSAMA_MESSAGES_PALLET_INDEX
			))
		);

		assert!(BridgeHubKusamaLocation::get()
			.starts_with(&KusamaGlobalConsensusNetworkLocation::get()));
	}
}

/// Contains the migrations for a P/K bridge.
pub mod migration {
	use super::*;

	/// Fix data from XCMv4 to XCMv5 because of buggy of XCM `try_as` implementation.
	pub struct MigrateToXcm5<T, I>(core::marker::PhantomData<(T, I)>);

	impl<T: pallet_xcm_bridge_hub::Config<I>, I: 'static> frame_support::traits::OnRuntimeUpgrade
		for MigrateToXcm5<T, I>
	{
		fn on_runtime_upgrade() -> Weight {
			use sp_core::Get;
			use xcm::IntoVersion;
			let mut weight = T::DbWeight::get().reads(1);

			// `Migrate to latest XCM`.
			let translate =
				|mut pre: pallet_xcm_bridge_hub::BridgeOf<T, I>| -> Option<pallet_xcm_bridge_hub::BridgeOf<T, I>> {
					weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));

					if let Ok(latest) = pre.bridge_origin_relative_location.clone().into_latest() {
						pre.bridge_origin_relative_location = alloc::boxed::Box::new(latest);
					}
					if let Ok(latest) = pre.bridge_origin_universal_location.clone().into_latest() {
						pre.bridge_origin_universal_location = alloc::boxed::Box::new(latest);
					}
					if let Ok(latest) = pre.bridge_destination_universal_location.clone().into_latest() {
						pre.bridge_destination_universal_location = alloc::boxed::Box::new(latest);
					}

					Some(pre)
				};
			pallet_xcm_bridge_hub::Bridges::<T, I>::translate_values(translate);

			weight
		}
	}
}
