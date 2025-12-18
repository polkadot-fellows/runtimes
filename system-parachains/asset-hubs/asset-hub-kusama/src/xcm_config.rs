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

use cumulus_primitives_core::ParaId;
pub use TreasuryAccount as RelayTreasuryPalletAccount;

use super::{
	AccountId, AllPalletsWithSystem, AssetConversion, Assets, Balance, Balances, CollatorSelection,
	FellowshipAdmin, GeneralAdmin, NativeAndAssets, ParachainInfo, ParachainSystem, PolkadotXcm,
	PoolAssets, PriceForParentDelivery, Runtime, RuntimeCall, RuntimeEvent, RuntimeHoldReason,
	RuntimeOrigin, StakingAdmin, ToPolkadotXcmRouter, WeightToFee, XcmpQueue,
};
use crate::ForeignAssets;
use alloc::{vec, vec::Vec};
use assets_common::{
	matching::{FromSiblingParachain, IsForeignConcreteAsset, ParentLocation},
	TrustBackedAssetsAsLocation,
};
use core::marker::PhantomData;
use frame_support::{
	pallet_prelude::Get,
	parameter_types,
	traits::{
		fungible::HoldConsideration,
		tokens::imbalance::{ResolveAssetTo, ResolveTo},
		ConstU32, Contains, ContainsPair, Defensive, Equals, Everything, FromContains,
		LinearStoragePrice, PalletInfoAccess,
	},
};
use frame_system::EnsureRoot;
use kusama_runtime_constants::xcm::body::FELLOWSHIP_ADMIN_INDEX;
use pallet_xcm::{AuthorizedAliasers, XcmPassthrough};
use parachains_common::xcm_config::{
	AllSiblingSystemParachains, ConcreteAssetFromSystem, ParentRelayOrSiblingParachains,
	RelayOrOtherSystemParachains,
};
use polkadot_parachain_primitives::primitives::Sibling;
use snowbridge_inbound_queue_primitives::EthereumLocationsConverterFor;
use sp_runtime::traits::TryConvertInto;
use xcm::latest::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AliasChildLocation, AliasOriginRootUsingFilter,
	AllowExplicitUnpaidExecutionFrom, AllowKnownQueryResponses, AllowSubscriptionsFrom,
	AllowTopLevelPaidExecutionFrom, DenyReserveTransferToRelayChain, DenyThenTry,
	DescribeAllTerminal, DescribeFamily, EnsureXcmOrigin, FrameTransactionalProcessor,
	FungibleAdapter, FungiblesAdapter, GlobalConsensusParachainConvertsFor, HashedDescription,
	IsConcrete, IsSiblingSystemParachain, LocalMint, MatchedConvertedConcreteId, MintLocation,
	NoChecking, OriginToPluralityVoice, ParentAsSuperuser, ParentIsPreset, RelayChainAsNative,
	SendXcmFeeToAccount, SiblingParachainAsNative, SiblingParachainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SingleAssetExchangeAdapter,
	SovereignSignedViaLocation, StartsWith, StartsWithExplicitGlobalConsensus, TakeWeightCredit,
	TrailingSetTopicAsId, UsingComponents, WeightInfoBounds, WithComputedOrigin,
	WithLatestLocationConverter, WithUniqueTopic, XcmFeeManagerFromComponents,
};
use xcm_executor::{traits::ConvertLocation, XcmExecutor};

pub use system_parachains_constants::kusama::locations::RelayChainLocation;

parameter_types! {
	pub const RootLocation: Location = Location::here();
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
	pub FellowshipLocation: Location = RelayChainLocation::get();
	pub RelayTreasuryLocation: Location = (Parent, PalletInstance(kusama_runtime_constants::TREASURY_PALLET_ID)).into();
	pub StakingPot: AccountId = CollatorSelection::account_id();
	// Test [`crate::tests::treasury_pallet_account_not_none`] ensures that the result of location
	// conversion is not `None`.
	// Account address: `5Gzx76VEMzLpMp9HBarpkJ62WMSNeRfdD1jLjpvpZtY37Wum`
	pub PreMigrationRelayTreasuryPalletAccount: AccountId =
		LocationToAccountId::convert_location(&RelayTreasuryLocation::get())
			.defensive_unwrap_or(crate::treasury::TreasuryAccount::get());
	pub PostMigrationTreasuryAccount: AccountId = crate::treasury::TreasuryAccount::get();
	/// The Checking Account along with the indication that the local chain is able to mint tokens.
	pub TeleportTracking: Option<(AccountId, MintLocation)> = crate::AhMigrator::teleport_tracking();
	pub const Here: Location = Location::here();
	pub SelfParaId: ParaId = ParachainInfo::parachain_id();
}

/// Treasury account that changes once migration ends.
pub type TreasuryAccount = pallet_ah_migrator::xcm_config::TreasuryAccount<
	crate::AhMigrator,
	PreMigrationRelayTreasuryPalletAccount,
	PostMigrationTreasuryAccount,
>;

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
	EthereumLocationsConverterFor<AccountId>,
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
	// Teleports tracking is managed by `AhMigrator`: no tracking before, track after.
	TeleportTracking,
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
	Location,
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

/// Means for transacting assets on this chain.
pub type AssetTransactors = (FungibleTransactor, FungiblesTransactor, ForeignFungiblesTransactor);

/// Asset converter for pool assets.
/// Used to convert one asset to another, when there is a pool available between the two.
/// This type thus allows paying delivery fees with any asset as long as there is a pool between
/// said asset and the asset required for fee payment.
pub type PoolAssetsExchanger = SingleAssetExchangeAdapter<
	AssetConversion,
	NativeAndAssets,
	(
		TrustBackedAssetsAsLocation<TrustBackedAssetsPalletLocation, Balance, Location>,
		ForeignAssetsConvertedConcreteId,
		// `ForeignAssetsConvertedConcreteId` doesn't include Relay token, so we handle it
		// explicitly here.
		MatchedConvertedConcreteId<
			Location,
			Balance,
			Equals<ParentLocation>,
			WithLatestLocationConverter<Location>,
			TryConvertInto,
		>,
	),
	AccountId,
>;

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
}

pub struct ParentOrParentsPlurality;
impl Contains<Location> for ParentOrParentsPlurality {
	fn contains(location: &Location) -> bool {
		matches!(location.unpack(), (1, []) | (1, [Plurality { .. }]))
	}
}

pub struct LocalPlurality;
impl Contains<Location> for LocalPlurality {
	fn contains(loc: &Location) -> bool {
		matches!(loc.unpack(), (0, [Plurality { .. }]))
	}
}

/// Location type to determine the Technical Fellowship related
/// pallets for use in XCM.
pub struct FellowshipEntities;
impl Contains<Location> for FellowshipEntities {
	fn contains(location: &Location) -> bool {
		matches!(location.unpack(), (1, [Plurality { id: BodyId::Technical, .. }]))
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
						FellowshipEntities,
						IsSiblingSystemParachain<ParaId, parachain_info::Pallet<Runtime>>,
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

/// Locations that will not be charged fees in the executor,
/// either execution or delivery.
/// We only waive fees for system functions, which these locations represent.
pub type WaivedLocations = (
	Equals<RootLocation>,
	RelayOrOtherSystemParachains<AllSiblingSystemParachains, Runtime>,
	Equals<RelayTreasuryLocation>,
	FellowshipEntities,
	LocalPlurality,
);

/// Cases where a remote origin is accepted as trusted Teleporter for a given asset:
///
/// - KSM with the parent Relay Chain and sibling system parachains; and
/// - Sibling parachains' assets from where they originate (as `ForeignCreators`).
pub type TrustedTeleporters = (
	ConcreteAssetFromSystem<KsmLocation>,
	IsForeignConcreteAsset<FromSiblingParachain<parachain_info::Pallet<Runtime>>>,
);

/// During migration we only allow teleports of foreign assets (not DOT).
///
/// - Sibling parachains' assets from where they originate (as `ForeignCreators`).
pub type TrustedTeleportersWhileMigrating =
	IsForeignConcreteAsset<FromSiblingParachain<parachain_info::Pallet<Runtime>>>;

/// Defines all global consensus locations that Polkadot Asset Hub is allowed to alias into.
pub struct PolkadotOrEthereumGlobalConsensus;
impl Contains<Location> for PolkadotOrEthereumGlobalConsensus {
	fn contains(location: &Location) -> bool {
		matches!(location.unpack(), (2, [GlobalConsensus(network_id)]) | (2, [GlobalConsensus(network_id), ..])
			if matches!(*network_id, NetworkId::Polkadot | NetworkId::Ethereum { chain_id: 1 }))
	}
}

/// Defines origin aliasing rules for this chain.
///
/// - Allow any origin to alias into a child sub-location (equivalent to DescendOrigin),
/// - Allow origins explicitly authorized by the alias target location.
/// - Allow cousin Polkadot Asset Hub to alias into Polkadot or Ethereum (bridged) origins.
pub type TrustedAliasers = (
	AliasChildLocation,
	AuthorizedAliasers<Runtime>,
	AliasOriginRootUsingFilter<
		bridging::to_polkadot::AssetHubPolkadot,
		PolkadotOrEthereumGlobalConsensus,
	>,
);

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	type XcmRecorder = PolkadotXcm;
	type AssetTransactor = AssetTransactors;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	// Asset Hub trusts only particular, pre-configured bridged locations from a different consensus
	// as reserve locations (we trust the Bridge Hub to relay the message that a reserve is being
	// held). On Kusama Asset Hub, we allow Polkadot Asset Hub to act as reserve for any asset
	// native to the Polkadot or Ethereum ecosystems.
	type IsReserve = (bridging::to_polkadot::PolkadotOrEthereumAssetFromAssetHubPolkadot,);
	type IsTeleporter = pallet_ah_migrator::xcm_config::TrustedTeleporters<
		crate::AhMigrator,
		TrustedTeleportersWhileMigrating,
		TrustedTeleporters,
	>;
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
				TrustBackedAssetsAsLocation<TrustBackedAssetsPalletLocation, Balance, Location>,
				ForeignAssetsConvertedConcreteId,
			),
			ResolveAssetTo<StakingPot, NativeAndAssets>,
			AccountId,
		>,
	);
	type ResponseHandler = PolkadotXcm;
	type AssetTrap = PolkadotXcm;
	type AssetClaims = PolkadotXcm;
	type SubscriptionService = PolkadotXcm;
	type PalletInstancesInfo = AllPalletsWithSystem;
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	type AssetLocker = ();
	type AssetExchanger = PoolAssetsExchanger;
	type FeeManager = XcmFeeManagerFromComponents<
		WaivedLocations,
		SendXcmFeeToAccount<Self::AssetTransactor, TreasuryAccount>,
	>;
	type MessageExporter = ();
	type UniversalAliases = (bridging::to_polkadot::UniversalAliases,);
	type CallDispatcher = RuntimeCall;
	type SafeCallFilter = Everything;
	type Aliasers = TrustedAliasers;
	type TransactionalProcessor = FrameTransactionalProcessor;
	type HrmpNewChannelOpenRequestHandler = ();
	type HrmpChannelAcceptedHandler = ();
	type HrmpChannelClosingHandler = ();
	type XcmEventEmitter = PolkadotXcm;
}

parameter_types! {
	// StakingAdmin pluralistic body.
	pub const StakingAdminBodyId: BodyId = BodyId::Defense;
	// Fellows pluralistic body.
	pub const FellowsBodyId: BodyId = BodyId::Technical;
	// `GeneralAdmin` pluralistic body.
	pub const GeneralAdminBodyId: BodyId = BodyId::Administration;
	// `FellowshipAdmin` pluralistic body.
	pub const FellowshipAdminBodyId: BodyId = BodyId::Index(FELLOWSHIP_ADMIN_INDEX);
}

/// Type to convert the `StakingAdmin` origin to a Plurality `Location` value.
pub type StakingAdminToPlurality =
	OriginToPluralityVoice<RuntimeOrigin, StakingAdmin, StakingAdminBodyId>;

/// Type to convert the `GeneralAdmin` origin to a Plurality `Location` value.
pub type GeneralAdminToPlurality =
	OriginToPluralityVoice<RuntimeOrigin, GeneralAdmin, GeneralAdminBodyId>;
/// Type to convert the `FellowshipAdmin` origin to a Plurality `Location` value.
pub type FellowshipAdminToPlurality =
	OriginToPluralityVoice<RuntimeOrigin, FellowshipAdmin, FellowshipAdminBodyId>;

/// Converts a local signed origin into an XCM `Location`.
/// Forms the basis for local origins sending/executing XCMs.
pub type LocalSignedOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

/// Type to convert a pallet `Origin` type value into a `Location` value which represents an
/// interior location of this chain for a destination chain.
pub type LocalPalletOrSignedOriginToLocation = (
	// GeneralAdmin origin to be used in XCM as a corresponding Plurality `Location` value.
	GeneralAdminToPlurality,
	// StakingAdmin origin to be used in XCM as a corresponding Plurality `Location` value.
	StakingAdminToPlurality,
	// FellowshipAdmin origin to be used in XCM as a corresponding Plurality `Location` value.
	FellowshipAdminToPlurality,
	// And a usual Signed origin to be used in XCM as a corresponding `AccountId32`.
	SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>,
);

/// Use [`LocalXcmRouter`] instead.
pub(crate) type LocalXcmRouterWithoutException = (
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm, PriceForParentDelivery>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
);

/// For routing XCM messages which do not cross local consensus boundary.
type LocalXcmRouter = pallet_ah_migrator::RouteInnerWithException<
	LocalXcmRouterWithoutException,
	// Exception: query responses to Relay Chain (`ParentLocation`) which initiated (`Querier`) by
	// the Relay Chain (`Here`, since from the perspective of the receiver).
	// See: https://github.com/paritytech/polkadot-sdk/blob/28b7c7770e9e7abf5b561fc42cfe565baf076cb7/polkadot/xcm/xcm-executor/src/lib.rs#L728
	//
	// This exception is required for the migration flow-control system to send query responses
	// to the Relay Chain, confirming that data messages have been received.
	FromContains<Equals<ParentLocation>, pallet_ah_migrator::ExceptResponseFor<Equals<Here>>>,
	crate::AhMigrator,
>;

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

parameter_types! {
	pub const DepositPerItem: Balance = crate::system_para_deposit(1, 0);
	pub const DepositPerByte: Balance = crate::system_para_deposit(0, 1);
	pub const AuthorizeAliasHoldReason: RuntimeHoldReason =
		RuntimeHoldReason::PolkadotXcm(pallet_xcm::HoldReason::AuthorizeAlias);
}

impl pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	// Any local signed origin can send XCM messages.
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalPalletOrSignedOriginToLocation>;
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
	// xcm_executor::Config::Aliasers uses pallet_xcm::AuthorizedAliasers.
	type AuthorizedAliasConsideration = HoldConsideration<
		AccountId,
		Balances,
		AuthorizeAliasHoldReason,
		LinearStoragePrice<DepositPerItem, DepositPerByte, Balance>,
	>;
}

impl cumulus_pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

/// Simple conversion of `u32` into an `AssetId` for use in benchmarking.
pub struct XcmBenchmarkHelper;
#[cfg(feature = "runtime-benchmarks")]
impl pallet_assets::BenchmarkHelper<Location> for XcmBenchmarkHelper {
	fn create_asset_id_parameter(id: u32) -> Location {
		Location::new(1, Parachain(id))
	}
}

/// All configuration related to bridging
pub mod bridging {
	use super::*;
	use alloc::collections::BTreeSet;
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

		pub BridgeTable: Vec<NetworkExportTableItem> =
			Vec::new().into_iter()
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
					Parachain(kusama_runtime_constants::system_parachain::ASSET_HUB_ID),
			],
			);

			/// Set up exporters configuration.
			/// `Option<Asset>` represents static "base fee" which is used for total delivery fee calculation.
			pub BridgeTable: Vec<NetworkExportTableItem> = vec![
				NetworkExportTableItem::new(
					PolkadotNetwork::get(),
					Some(vec![
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
				vec![
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
			PhantomData<(AssetsAllowedNetworks, OriginLocation)>,
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
