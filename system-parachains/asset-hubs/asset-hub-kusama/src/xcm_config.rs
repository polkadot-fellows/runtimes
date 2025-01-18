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

use super::{
	AccountId, AllPalletsWithSystem, AssetConversion, Assets, Authorship, Balance, Balances,
	CollatorSelection, NativeAndAssets, ParachainInfo, ParachainSystem, PolkadotXcm, PoolAssets,
	PriceForParentDelivery, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin, ToPolkadotXcmRouter,
	TrustBackedAssetsInstance, WeightToFee, XcmpQueue,
};
use crate::{ForeignAssets, ForeignAssetsInstance};
use assets_common::{
	matching::{FromSiblingParachain, IsForeignConcreteAsset},
	TrustBackedAssetsAsLocation,
};
use frame_support::{
	pallet_prelude::Get,
	parameter_types,
	traits::{
		tokens::imbalance::{ResolveAssetTo, ResolveTo},
		ConstU32, Contains, ContainsPair, Equals, Everything, Nothing, PalletInfoAccess,
	},
};
use frame_system::EnsureRoot;
use pallet_xcm::XcmPassthrough;
use parachains_common::xcm_config::{
	AllSiblingSystemParachains, AssetFeeAsExistentialDepositMultiplier, ConcreteAssetFromSystem,
	ParentRelayOrSiblingParachains, RelayOrOtherSystemParachains,
};
use polkadot_parachain_primitives::primitives::Sibling;
use snowbridge_router_primitives::inbound::GlobalConsensusEthereumConvertsFor;
use sp_runtime::traits::{AccountIdConversion, ConvertInto};
use system_parachains_constants::TREASURY_PALLET_ID;
use xcm::latest::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AllowExplicitUnpaidExecutionFrom, AllowKnownQueryResponses,
	AllowSubscriptionsFrom, AllowTopLevelPaidExecutionFrom, DenyReserveTransferToRelayChain,
	DenyThenTry, DescribeAllTerminal, DescribeFamily, EnsureXcmOrigin, FrameTransactionalProcessor,
	FungibleAdapter, FungiblesAdapter, GlobalConsensusParachainConvertsFor, HashedDescription,
	IsConcrete, LocalMint, NoChecking, ParentAsSuperuser, ParentIsPreset, RelayChainAsNative,
	SendXcmFeeToAccount, SiblingParachainAsNative, SiblingParachainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation, StartsWith,
	StartsWithExplicitGlobalConsensus, TakeWeightCredit, TrailingSetTopicAsId, UsingComponents,
	WeightInfoBounds, WithComputedOrigin, WithUniqueTopic, XcmFeeManagerFromComponents,
};
use xcm_executor::{traits::ConvertLocation, XcmExecutor};

parameter_types! {
	pub const KsmLocation: Location = Location::parent();
	pub const RelayNetwork: Option<NetworkId> = Some(NetworkId::Kusama);
	pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
	pub UniversalLocation: InteriorLocation =
		[GlobalConsensus(RelayNetwork::get().unwrap()), Parachain(ParachainInfo::parachain_id().into())].into();
	pub UniversalLocationNetworkId: NetworkId = UniversalLocation::get().global_consensus().unwrap();

	pub TrustBackedAssetsPalletIndex: u8 = <Assets as PalletInfoAccess>::index() as u8;
	pub TrustBackedAssetsPalletLocation: Location = PalletInstance(TrustBackedAssetsPalletIndex::get()).into();

	pub ForeignAssetsPalletLocation: Location =
		PalletInstance(<ForeignAssets as PalletInfoAccess>::index() as u8).into();
	pub PoolAssetsPalletLocation: Location =
		PalletInstance(<PoolAssets as PalletInfoAccess>::index() as u8).into();
	pub CheckingAccount: AccountId = PolkadotXcm::check_account();
	pub const GovernanceLocation: Location = Location::parent();
	pub const FellowshipLocation: Location = Location::parent();
	pub RelayTreasuryLocation: Location = (Parent, PalletInstance(kusama_runtime_constants::TREASURY_PALLET_ID)).into();
	pub TreasuryAccount: AccountId = TREASURY_PALLET_ID.into_account_truncating();
	pub StakingPot: AccountId = CollatorSelection::account_id();
	// Test [`crate::tests::treasury_pallet_account_not_none`] ensures that the result of location
	// conversion is not `None`.
	pub RelayTreasuryPalletAccount: AccountId =
		LocationToAccountId::convert_location(&RelayTreasuryLocation::get())
			.unwrap_or(TreasuryAccount::get());
}

/// Type for specifying how a `Location` can be converted into an `AccountId`.
///
/// This is used when determining ownership of accounts for asset transacting and when attempting to
/// use XCM `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
	// The parent (Relay-chain) origin converts to the parent `AccountId`.
	ParentIsPreset<AccountId>,
	// Sibling parachain origins convert to AccountId via the `ParaId::into`.
	SiblingParachainConvertsVia<Sibling, AccountId>,
	// Straight up local `AccountId32` origins just alias directly to `AccountId`.
	AccountId32Aliases<RelayNetwork, AccountId>,
	// Foreign locations alias into accounts according to a hash of their standard description.
	HashedDescription<AccountId, DescribeFamily<DescribeAllTerminal>>,
	// Different global consensus parachain sovereign account.
	// (Used for over-bridge transfers and reserve processing)
	GlobalConsensusParachainConvertsFor<UniversalLocation, AccountId>,
	// Ethereum contract sovereign account.
	// (Used to get convert ethereum contract locations to sovereign account)
	GlobalConsensusEthereumConvertsFor<AccountId>,
);

/// Means for transacting the native currency on this chain.
pub type FungibleTransactor = FungibleAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<KsmLocation>,
	// Convert an XCM `Location` into a local account ID:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We don't track any teleports of `Balances`.
	(),
>;

/// `AssetId`/`Balance` converter for `TrustBackedAssets`.
pub type TrustBackedAssetsConvertedConcreteId =
	assets_common::TrustBackedAssetsConvertedConcreteId<TrustBackedAssetsPalletLocation, Balance>;

/// Means for transacting assets besides the native currency on this chain.
pub type FungiblesTransactor = FungiblesAdapter<
	// Use this fungibles implementation:
	Assets,
	// Use this currency when it is a fungible asset matching the given location or name:
	TrustBackedAssetsConvertedConcreteId,
	// Convert an XCM `Location` into a local account ID:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We only want to allow teleports of known assets. We use non-zero issuance as an indication
	// that this asset is known.
	LocalMint<parachains_common::impls::NonZeroIssuance<AccountId, Assets>>,
	// The account to use for tracking teleports.
	CheckingAccount,
>;

/// `AssetId`/`Balance` converter for `ForeignAssets`
pub type ForeignAssetsConvertedConcreteId = assets_common::ForeignAssetsConvertedConcreteId<
	(
		// Ignore `TrustBackedAssets` explicitly
		StartsWith<TrustBackedAssetsPalletLocation>,
		// Ignore assets that start explicitly with our `GlobalConsensus(NetworkId)`, means:
		// - foreign assets from our consensus should be: `Location {parents: 1, X*(Parachain(xyz),
		//   ..)}`
		// - foreign assets outside our consensus with the same `GlobalConsensus(NetworkId)` won't
		//   be accepted here
		StartsWithExplicitGlobalConsensus<UniversalLocationNetworkId>,
	),
	Balance,
	xcm::v4::Location,
>;

/// Means for transacting foreign assets from different global consensus.
pub type ForeignFungiblesTransactor = FungiblesAdapter<
	// Use this fungibles implementation:
	ForeignAssets,
	// Use this currency when it is a fungible asset matching the given location or name:
	ForeignAssetsConvertedConcreteId,
	// Convert an XCM `Location` into a local account ID:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We dont need to check teleports here.
	NoChecking,
	// The account to use for tracking teleports.
	CheckingAccount,
>;

/// `AssetId`/`Balance` converter for `PoolAssets`.
pub type PoolAssetsConvertedConcreteId =
	assets_common::PoolAssetsConvertedConcreteId<PoolAssetsPalletLocation, Balance>;

/// Means for transacting asset conversion pool assets on this chain.
pub type PoolFungiblesTransactor = FungiblesAdapter<
	// Use this fungibles implementation:
	PoolAssets,
	// Use this currency when it is a fungible asset matching the given location or name:
	PoolAssetsConvertedConcreteId,
	// Convert an XCM `Location` into a local account ID:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We only want to allow teleports of known assets. We use non-zero issuance as an indication
	// that this asset is known.
	LocalMint<parachains_common::impls::NonZeroIssuance<AccountId, PoolAssets>>,
	// The account to use for tracking teleports.
	CheckingAccount,
>;

/// Means for transacting assets on this chain.
pub type AssetTransactors =
	(FungibleTransactor, FungiblesTransactor, ForeignFungiblesTransactor, PoolFungiblesTransactor);

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`.
///
/// There is an `OriginKind` which can biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
	// Sovereign account converter; this attempts to derive an `AccountId` from the origin location
	// using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
	// foreign chains who want to have a local sovereign account on this chain which they control.
	SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
	// Native converter for Relay-chain (Parent) location; will convert to a `Relay` origin when
	// recognised.
	RelayChainAsNative<RelayChainOrigin, RuntimeOrigin>,
	// Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
	// recognised.
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, RuntimeOrigin>,
	// Superuser converter for the Relay-chain (Parent) location. This will allow it to issue a
	// transaction from the Root origin.
	ParentAsSuperuser<RuntimeOrigin>,
	// Native signed account converter; this just converts an `AccountId32` origin into a normal
	// `RuntimeOrigin::Signed` origin of the same 32-byte value.
	SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
	// Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
	XcmPassthrough<RuntimeOrigin>,
);

parameter_types! {
	pub const MaxInstructions: u32 = 100;
	pub const MaxAssetsIntoHolding: u32 = 64;
	pub XcmAssetFeesReceiver: Option<AccountId> = Authorship::author();
}

pub struct ParentOrParentsPlurality;
impl Contains<Location> for ParentOrParentsPlurality {
	fn contains(location: &Location) -> bool {
		matches!(location.unpack(), (1, []) | (1, [Plurality { .. }]))
	}
}

pub type Barrier = TrailingSetTopicAsId<
	DenyThenTry<
		DenyReserveTransferToRelayChain,
		(
			TakeWeightCredit,
			// Expected responses are OK.
			AllowKnownQueryResponses<PolkadotXcm>,
			// Allow XCMs with some computed origins to pass through.
			WithComputedOrigin<
				(
					// If the message is one that immediately attempts to pay for execution, then
					// allow it.
					AllowTopLevelPaidExecutionFrom<Everything>,
					// Parent, its pluralities (i.e. governance bodies), parent's treasury and
					// sibling bridge hub get free execution.
					AllowExplicitUnpaidExecutionFrom<(
						ParentOrParentsPlurality,
						Equals<RelayTreasuryLocation>,
						Equals<bridging::SiblingBridgeHub>,
					)>,
					// Subscriptions for version tracking are OK.
					AllowSubscriptionsFrom<ParentRelayOrSiblingParachains>,
				),
				UniversalLocation,
				ConstU32<8>,
			>,
		),
	>,
>;

pub type AssetFeeAsExistentialDepositMultiplierFeeCharger = AssetFeeAsExistentialDepositMultiplier<
	Runtime,
	WeightToFee,
	pallet_assets::BalanceToAssetBalance<Balances, Runtime, ConvertInto, TrustBackedAssetsInstance>,
	TrustBackedAssetsInstance,
>;

/// Locations that will not be charged fees in the executor,
/// either execution or delivery.
/// We only waive fees for system functions, which these locations represent.
pub type WaivedLocations = (
	RelayOrOtherSystemParachains<AllSiblingSystemParachains, Runtime>,
	Equals<RelayTreasuryLocation>,
);

/// Cases where a remote origin is accepted as trusted Teleporter for a given asset:
///
/// - KSM with the parent Relay Chain and sibling system parachains; and
/// - Sibling parachains' assets from where they originate (as `ForeignCreators`).
pub type TrustedTeleporters = (
	ConcreteAssetFromSystem<KsmLocation>,
	IsForeignConcreteAsset<FromSiblingParachain<parachain_info::Pallet<Runtime>>>,
);

/// Multiplier used for dedicated `TakeFirstAssetTrader` with `ForeignAssets` instance.
pub type ForeignAssetFeeAsExistentialDepositMultiplierFeeCharger =
	AssetFeeAsExistentialDepositMultiplier<
		Runtime,
		WeightToFee,
		pallet_assets::BalanceToAssetBalance<Balances, Runtime, ConvertInto, ForeignAssetsInstance>,
		ForeignAssetsInstance,
	>;

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	type XcmRecorder = ();
	type AssetTransactor = AssetTransactors;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	// Asset Hub trusts only particular, pre-configured bridged locations from a different consensus
	// as reserve locations (we trust the Bridge Hub to relay the message that a reserve is being
	// held). On Kusama Asset Hub, we allow Polkadot Asset Hub to act as reserve for any asset
	// native to the Polkadot or Ethereum ecosystems.
	type IsReserve = (bridging::to_polkadot::PolkadotOrEthereumAssetFromAssetHubPolkadot,);
	type IsTeleporter = TrustedTeleporters;
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = WeightInfoBounds<
		crate::weights::xcm::AssetHubKusamaXcmWeight<RuntimeCall>,
		RuntimeCall,
		MaxInstructions,
	>;
	type Trader = (
		UsingComponents<
			WeightToFee,
			KsmLocation,
			AccountId,
			Balances,
			ResolveTo<StakingPot, Balances>,
		>,
		// This trader allows to pay with any assets exchangeable to KSM with
		// [`AssetConversion`].
		cumulus_primitives_utility::SwapFirstAssetTrader<
			KsmLocation,
			AssetConversion,
			WeightToFee,
			NativeAndAssets,
			(
				TrustBackedAssetsAsLocation<
					TrustBackedAssetsPalletLocation,
					Balance,
					xcm::v4::Location,
				>,
				ForeignAssetsConvertedConcreteId,
			),
			ResolveAssetTo<StakingPot, NativeAndAssets>,
			AccountId,
		>,
		// This trader allows to pay with `is_sufficient=true` "Trust Backed" assets from dedicated
		// `pallet_assets` instance - `Assets`.
		cumulus_primitives_utility::TakeFirstAssetTrader<
			AccountId,
			AssetFeeAsExistentialDepositMultiplierFeeCharger,
			TrustBackedAssetsConvertedConcreteId,
			Assets,
			cumulus_primitives_utility::XcmFeesTo32ByteAccount<
				FungiblesTransactor,
				AccountId,
				XcmAssetFeesReceiver,
			>,
		>,
		// This trader allows to pay with `is_sufficient=true` "Foreign" assets from dedicated
		// `pallet_assets` instance - `ForeignAssets`.
		cumulus_primitives_utility::TakeFirstAssetTrader<
			AccountId,
			ForeignAssetFeeAsExistentialDepositMultiplierFeeCharger,
			ForeignAssetsConvertedConcreteId,
			ForeignAssets,
			cumulus_primitives_utility::XcmFeesTo32ByteAccount<
				ForeignFungiblesTransactor,
				AccountId,
				XcmAssetFeesReceiver,
			>,
		>,
	);
	type ResponseHandler = PolkadotXcm;
	type AssetTrap = PolkadotXcm;
	type AssetClaims = PolkadotXcm;
	type SubscriptionService = PolkadotXcm;
	type PalletInstancesInfo = AllPalletsWithSystem;
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	type AssetLocker = ();
	type AssetExchanger = ();
	type FeeManager = XcmFeeManagerFromComponents<
		WaivedLocations,
		SendXcmFeeToAccount<Self::AssetTransactor, RelayTreasuryPalletAccount>,
	>;
	type MessageExporter = ();
	type UniversalAliases = (bridging::to_polkadot::UniversalAliases,);
	type CallDispatcher = RuntimeCall;
	type SafeCallFilter = Everything;
	type Aliasers = Nothing;
	type TransactionalProcessor = FrameTransactionalProcessor;
	type HrmpNewChannelOpenRequestHandler = ();
	type HrmpChannelAcceptedHandler = ();
	type HrmpChannelClosingHandler = ();
}

/// Converts a local signed origin into an XCM `Location`.
/// Forms the basis for local origins sending/executing XCMs.
pub type LocalSignedOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

/// For routing XCM messages which do not cross local consensus boundary.
type LocalXcmRouter = (
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm, PriceForParentDelivery>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
);

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = WithUniqueTopic<(
	// The means for routing XCM messages which are not for local execution into the right message
	// queues.
	LocalXcmRouter,
	// Router which wraps and sends xcm to BridgeHub to be delivered to the Polkadot
	// GlobalConsensus
	ToPolkadotXcmRouter,
)>;

impl pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	// Any local signed origin can send XCM messages.
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalSignedOriginToLocation>;
	type XcmRouter = XcmRouter;
	// Any local signed origin can execute XCM messages.
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalSignedOriginToLocation>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Everything;
	type XcmReserveTransferFilter = Everything;
	type Weigher = WeightInfoBounds<
		crate::weights::xcm::AssetHubKusamaXcmWeight<RuntimeCall>,
		RuntimeCall,
		MaxInstructions,
	>;
	type UniversalLocation = UniversalLocation;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
	type Currency = Balances;
	type CurrencyMatcher = ();
	type TrustedLockers = ();
	type SovereignAccountOf = LocationToAccountId;
	type MaxLockers = ConstU32<8>;
	type WeightInfo = crate::weights::pallet_xcm::WeightInfo<Runtime>;
	type AdminOrigin = EnsureRoot<AccountId>;
	type MaxRemoteLockConsumers = ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
}

impl cumulus_pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

pub type ForeignCreatorsSovereignAccountOf = (
	SiblingParachainConvertsVia<Sibling, AccountId>,
	AccountId32Aliases<RelayNetwork, AccountId>,
	ParentIsPreset<AccountId>,
	GlobalConsensusEthereumConvertsFor<AccountId>,
	GlobalConsensusParachainConvertsFor<UniversalLocation, AccountId>,
);

/// Simple conversion of `u32` into an `AssetId` for use in benchmarking.
pub struct XcmBenchmarkHelper;
#[cfg(feature = "runtime-benchmarks")]
impl pallet_assets::BenchmarkHelper<xcm::v4::Location> for XcmBenchmarkHelper {
	fn create_asset_id_parameter(id: u32) -> xcm::v4::Location {
		xcm::v4::Location::new(1, xcm::v4::Junction::Parachain(id))
	}
}

/// All configuration related to bridging
pub mod bridging {
	use super::*;
	use sp_std::collections::btree_set::BTreeSet;
	use xcm_builder::NetworkExportTableItem;

	parameter_types! {
		/// Base price of every Kusama -> Polkadot message. Can be adjusted via
		/// governance `set_storage` call.
		pub storage XcmBridgeHubRouterBaseFee: Balance = bp_bridge_hub_kusama::estimate_kusama_to_polkadot_message_fee(
			bp_bridge_hub_polkadot::BridgeHubPolkadotBaseDeliveryFeeInDots::get()
		);
		/// Price of every byte of the Kusama -> Polkadot message. Can be adjusted via
		/// governance `set_storage` call.
		pub storage XcmBridgeHubRouterByteFee: Balance = bp_bridge_hub_kusama::estimate_kusama_to_polkadot_byte_fee();

		pub SiblingBridgeHubParaId: u32 = bp_bridge_hub_kusama::BRIDGE_HUB_KUSAMA_PARACHAIN_ID;
		pub SiblingBridgeHub: Location = Location::new(1, Parachain(SiblingBridgeHubParaId::get()));
		/// Router expects payment with this `AssetId`.
		/// (`AssetId` has to be aligned with `BridgeTable`)
		pub XcmBridgeHubRouterFeeAssetId: AssetId = KsmLocation::get().into();

		pub BridgeTable: sp_std::vec::Vec<NetworkExportTableItem> =
			sp_std::vec::Vec::new().into_iter()
			.chain(to_polkadot::BridgeTable::get())
			.collect();
	}

	pub type NetworkExportTable = xcm_builder::NetworkExportTable<BridgeTable>;

	pub mod to_polkadot {
		use super::*;

		parameter_types! {
			pub SiblingBridgeHubWithBridgeHubPolkadotInstance: Location = Location::new(
				1,
				[
					Parachain(SiblingBridgeHubParaId::get()),
					PalletInstance(bp_bridge_hub_kusama::WITH_BRIDGE_KUSAMA_TO_POLKADOT_MESSAGES_PALLET_INDEX),
				]
			);

			pub const PolkadotNetwork: NetworkId = NetworkId::Polkadot;
			pub const EthereumNetwork: NetworkId = NetworkId::Ethereum { chain_id: 1 };
			pub EthereumEcosystem: Location = Location::new(2, [GlobalConsensus(EthereumNetwork::get())]);
			pub DotLocation: Location = Location::new(2, [GlobalConsensus(PolkadotNetwork::get())]);
			pub AssetHubPolkadot: Location = Location::new(
				2,
				[
					GlobalConsensus(PolkadotNetwork::get()),
					Parachain(polkadot_runtime_constants::system_parachain::ASSET_HUB_ID),
			],
			);

			/// Set up exporters configuration.
			/// `Option<Asset>` represents static "base fee" which is used for total delivery fee calculation.
			pub BridgeTable: sp_std::vec::Vec<NetworkExportTableItem> = sp_std::vec![
				NetworkExportTableItem::new(
					PolkadotNetwork::get(),
					Some(sp_std::vec![
						AssetHubPolkadot::get().interior.split_global().expect("invalid configuration for AssetHubKusama").1,
					]),
					SiblingBridgeHub::get(),
					// base delivery fee to local `BridgeHub`
					Some((
						XcmBridgeHubRouterFeeAssetId::get(),
						XcmBridgeHubRouterBaseFee::get(),
					).into())
				)
			];

			/// Universal aliases
			pub UniversalAliases: BTreeSet<(Location, Junction)> = BTreeSet::from_iter(
				sp_std::vec![
					(SiblingBridgeHubWithBridgeHubPolkadotInstance::get(), GlobalConsensus(PolkadotNetwork::get()))
				]
			);
		}

		impl Contains<(Location, Junction)> for UniversalAliases {
			fn contains(alias: &(Location, Junction)) -> bool {
				UniversalAliases::get().contains(alias)
			}
		}

		/// Allow any asset native to the Polkadot or Ethereum ecosystems if it comes from Polkadot
		/// Asset Hub.
		pub type PolkadotOrEthereumAssetFromAssetHubPolkadot = RemoteAssetFromLocation<
			(StartsWith<DotLocation>, StartsWith<EthereumEcosystem>),
			AssetHubPolkadot,
		>;

		// TODO: get this from `assets_common v0.17.1` when SDK deps are upgraded
		/// Accept an asset if it is native to `AssetsAllowedNetworks` and it is coming from
		/// `OriginLocation`.
		pub struct RemoteAssetFromLocation<AssetsAllowedNetworks, OriginLocation>(
			sp_std::marker::PhantomData<(AssetsAllowedNetworks, OriginLocation)>,
		);
		impl<
				L: TryInto<Location> + Clone,
				AssetsAllowedNetworks: Contains<Location>,
				OriginLocation: Get<Location>,
			> ContainsPair<L, L> for RemoteAssetFromLocation<AssetsAllowedNetworks, OriginLocation>
		{
			fn contains(asset: &L, origin: &L) -> bool {
				let Ok(asset) = asset.clone().try_into() else {
					return false;
				};
				let Ok(origin) = origin.clone().try_into() else {
					return false;
				};

				let expected_origin = OriginLocation::get();
				// ensure `origin` is expected `OriginLocation`
				if !expected_origin.eq(&origin) {
					log::trace!(
						target: "xcm::contains",
						"RemoteAssetFromLocation asset: {asset:?}, origin: {origin:?} is not from expected {expected_origin:?}"
					);
					return false;
				} else {
					log::trace!(
						target: "xcm::contains",
						"RemoteAssetFromLocation asset: {asset:?}, origin: {origin:?}",
					);
				}

				// ensure `asset` is from remote consensus listed in `AssetsAllowedNetworks`
				AssetsAllowedNetworks::contains(&asset)
			}
		}
		impl<AssetsAllowedNetworks: Contains<Location>, OriginLocation: Get<Location>>
			ContainsPair<Asset, Location>
			for RemoteAssetFromLocation<AssetsAllowedNetworks, OriginLocation>
		{
			fn contains(asset: &Asset, origin: &Location) -> bool {
				<Self as ContainsPair<Location, Location>>::contains(&asset.id.0, origin)
			}
		}
	}

	/// Benchmarks helper for bridging configuration.
	#[cfg(feature = "runtime-benchmarks")]
	pub struct BridgingBenchmarksHelper;

	#[cfg(feature = "runtime-benchmarks")]
	impl BridgingBenchmarksHelper {
		pub fn prepare_universal_alias() -> Option<(Location, Junction)> {
			let alias = to_polkadot::UniversalAliases::get().into_iter().find_map(
				|(location, junction)| {
					match to_polkadot::SiblingBridgeHubWithBridgeHubPolkadotInstance::get()
						.eq(&location)
					{
						true => Some((location, junction)),
						false => None,
					}
				},
			);
			Some(alias.expect("we expect here BridgeHubKusama to Polkadot mapping at least"))
		}
	}
}
