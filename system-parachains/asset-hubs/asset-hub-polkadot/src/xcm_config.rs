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
	treasury, AccountId, AllPalletsWithSystem, AssetConversion, Assets, Balance, Balances,
	CollatorSelection, FellowshipAdmin, ForeignAssets, GeneralAdmin, NativeAndAssets,
	ParachainInfo, ParachainSystem, PolkadotXcm, PoolAssets, PriceForParentDelivery, Runtime,
	RuntimeCall, RuntimeEvent, RuntimeHoldReason, RuntimeOrigin, StakingAdmin, ToKusamaXcmRouter,
	Treasurer, WeightToFee, XcmpQueue,
};
use alloc::{collections::BTreeSet, vec, vec::Vec};
use assets_common::{
	matching::{FromNetwork, FromSiblingParachain, IsForeignConcreteAsset, ParentLocation},
	TrustBackedAssetsAsLocation,
};
use core::marker::PhantomData;
use frame_support::{
	pallet_prelude::Get,
	parameter_types,
	traits::{
		fungible::HoldConsideration,
		tokens::imbalance::{ResolveAssetTo, ResolveTo},
		ConstU32, Contains, ContainsPair, Equals, Everything, FromContains, LinearStoragePrice,
		PalletInfoAccess,
	},
};
use frame_system::EnsureRoot;
use pallet_xcm::{AuthorizedAliasers, XcmPassthrough};
use parachains_common::xcm_config::{
	AllSiblingSystemParachains, ConcreteAssetFromSystem, ParentRelayOrSiblingParachains,
	RelayOrOtherSystemParachains,
};
use polkadot_parachain_primitives::primitives::Sibling;
use polkadot_runtime_constants::{system_parachain, xcm::body::FELLOWSHIP_ADMIN_INDEX};
use snowbridge_outbound_queue_primitives::v2::exporter::PausableExporter;
use sp_runtime::traits::TryConvertInto;
use xcm::latest::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AliasChildLocation, AliasOriginRootUsingFilter,
	AllowExplicitUnpaidExecutionFrom, AllowKnownQueryResponses, AllowSubscriptionsFrom,
	AllowTopLevelPaidExecutionFrom, DenyReserveTransferToRelayChain, DenyThenTry,
	DescribeAllTerminal, DescribeFamily, EnsureXcmOrigin, ExternalConsensusLocationsConverterFor,
	FrameTransactionalProcessor, FungibleAdapter, FungiblesAdapter, HashedDescription, IsConcrete,
	IsSiblingSystemParachain, LocalMint, MatchedConvertedConcreteId, MintLocation, NoChecking,
	OriginToPluralityVoice, ParentAsSuperuser, ParentIsPreset, RelayChainAsNative,
	SendXcmFeeToAccount, SiblingParachainAsNative, SiblingParachainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SingleAssetExchangeAdapter,
	SovereignSignedViaLocation, StartsWith, StartsWithExplicitGlobalConsensus, TakeWeightCredit,
	TrailingSetTopicAsId, UnpaidRemoteExporter, UsingComponents, WeightInfoBounds,
	WithComputedOrigin, WithLatestLocationConverter, WithUniqueTopic, XcmFeeManagerFromComponents,
};
use xcm_executor::{traits::ConvertLocation, XcmExecutor};

pub use system_parachains_constants::polkadot::locations::{AssetHubLocation, RelayChainLocation};

parameter_types! {
	pub const RootLocation: Location = Location::here();
	pub const DotLocation: Location = Location::parent();
	pub const RelayNetwork: Option<NetworkId> = Some(NetworkId::Polkadot);
	pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
	pub UniversalLocation: InteriorLocation =
		[GlobalConsensus(RelayNetwork::get().unwrap()), Parachain(ParachainInfo::parachain_id().into())].into();
	pub UniversalLocationNetworkId: NetworkId = UniversalLocation::get().global_consensus().unwrap();
	pub TrustBackedAssetsPalletIndex: u8 = <Assets as PalletInfoAccess>::index() as u8;
	pub TrustBackedAssetsPalletLocation: Location =
		PalletInstance(TrustBackedAssetsPalletIndex::get()).into();
	pub CheckingAccount: AccountId = PolkadotXcm::check_account();
	pub FellowshipLocation: Location = Location::new(1, Parachain(system_parachain::COLLECTIVES_ID));
	pub RelayTreasuryLocation: Location = (Parent, PalletInstance(polkadot_runtime_constants::TREASURY_PALLET_ID)).into();
	pub PoolAssetsPalletLocation: Location =
		PalletInstance(<PoolAssets as PalletInfoAccess>::index() as u8).into();
	pub StakingPot: AccountId = CollatorSelection::account_id();
	// Test [`crate::tests::treasury_pallet_account_not_none`] ensures that the result of location
	// conversion is not `None`.
	// Account address: `14xmwinmCEz6oRrFdczHKqHgWNMiCysE2KrA4jXXAAM1Eogk`
	pub PreMigrationRelayTreasuryPalletAccount: AccountId =
		LocationToAccountId::convert_location(&RelayTreasuryLocation::get())
			.unwrap_or(treasury::TreasuryAccount::get());
	pub PostMigrationTreasuryAccount: AccountId = treasury::TreasuryAccount::get();
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
	// Different global consensus locations sovereign accounts.
	ExternalConsensusLocationsConverterFor<UniversalLocation, AccountId>,
);

/// Means for transacting the native currency on this chain.
pub type FungibleTransactor = FungibleAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<DotLocation>,
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

/// Location type to determine the Technical Fellowship related
/// pallets for use in XCM.
pub struct FellowshipEntities;
impl Contains<Location> for FellowshipEntities {
	fn contains(location: &Location) -> bool {
		matches!(
			location.unpack(),
			(
				1,
				[
					Parachain(system_parachain::COLLECTIVES_ID),
					Plurality { id: BodyId::Technical, .. }
				]
			) | (
				1,
				[
					Parachain(system_parachain::COLLECTIVES_ID),
					PalletInstance(
						collectives_polkadot_runtime_constants::FELLOWSHIP_SALARY_PALLET_INDEX
					)
				]
			) | (
				1,
				[
					Parachain(system_parachain::COLLECTIVES_ID),
					PalletInstance(
						collectives_polkadot_runtime_constants::FELLOWSHIP_TREASURY_PALLET_INDEX
					)
				]
			)
		)
	}
}

/// Location type to determine the Ambassador Collective
/// pallets for use in XCM.
pub struct AmbassadorEntities;
impl Contains<Location> for AmbassadorEntities {
	fn contains(location: &Location) -> bool {
		matches!(
			location.unpack(),
			(
				1,
				[
					Parachain(system_parachain::COLLECTIVES_ID),
					PalletInstance(
						collectives_polkadot_runtime_constants::AMBASSADOR_SALARY_PALLET_INDEX
					)
				]
			) | (
				1,
				[
					Parachain(system_parachain::COLLECTIVES_ID),
					PalletInstance(
						collectives_polkadot_runtime_constants::AMBASSADOR_TREASURY_PALLET_INDEX
					)
				]
			)
		)
	}
}

/// Location type to determine the Secretary Collective related
/// pallets for use in XCM.
pub struct SecretaryEntities;
impl Contains<Location> for SecretaryEntities {
	fn contains(location: &Location) -> bool {
		matches!(
			location.unpack(),
			(
				1,
				[
					Parachain(system_parachain::COLLECTIVES_ID),
					PalletInstance(
						collectives_polkadot_runtime_constants::SECRETARY_SALARY_PALLET_INDEX
					)
				]
			)
		)
	}
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
					// The locations listed below get free execution.
					// Parent, its pluralities (i.e. governance bodies), the Fellows plurality and
					// sibling bridge hub get free execution.
					AllowExplicitUnpaidExecutionFrom<(
						ParentOrParentsPlurality,
						FellowshipEntities,
						Equals<RelayTreasuryLocation>,
						Equals<bridging::SiblingBridgeHub>,
						AmbassadorEntities,
						SecretaryEntities,
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
	AmbassadorEntities,
	SecretaryEntities,
	LocalPlurality,
);

/// Cases where a remote origin is accepted as trusted Teleporter for a given asset:
///
/// - DOT with the parent Relay Chain and sibling system parachains; and
/// - Sibling parachains' assets from where they originate (as `ForeignCreators`).
pub type TrustedTeleporters = (
	ConcreteAssetFromSystem<DotLocation>,
	IsForeignConcreteAsset<FromSiblingParachain<parachain_info::Pallet<Runtime>>>,
);

/// During migration we only allow teleports of foreign assets (not DOT).
///
/// - Sibling parachains' assets from where they originate (as `ForeignCreators`).
pub type TrustedTeleportersWhileMigrating =
	IsForeignConcreteAsset<FromSiblingParachain<parachain_info::Pallet<Runtime>>>;

/// Defines all global consensus locations that Kusama Asset Hub is allowed to alias into.
pub struct KusamaGlobalConsensus;
impl Contains<Location> for KusamaGlobalConsensus {
	fn contains(location: &Location) -> bool {
		matches!(location.unpack(), (2, [GlobalConsensus(network_id)]) | (2, [GlobalConsensus(network_id), ..])
				if matches!(*network_id, NetworkId::Kusama))
	}
}

/// Defines origin aliasing rules for this chain.
///
/// - Allow any origin to alias into a child sub-location (equivalent to DescendOrigin),
/// - Allow origins explicitly authorized by the alias target location.
/// - Allow cousin Kusama Asset Hub to alias into Kusama (bridged) origins.
pub type TrustedAliasers = (
	AliasChildLocation,
	AuthorizedAliasers<Runtime>,
	AliasOriginRootUsingFilter<bridging::to_kusama::AssetHubKusama, KusamaGlobalConsensus>,
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
	// held). Asset Hub may _act_ as a reserve location for DOT and assets created
	// under `pallet-assets`. Users must use teleport where allowed (e.g. DOT with the Relay Chain).
	type IsReserve = (
		bridging::to_kusama::KusamaAssetFromAssetHubKusama,
		bridging::to_ethereum::EthereumAssetFromEthereum,
	);
	type IsTeleporter = pallet_ah_migrator::xcm_config::TrustedTeleporters<
		crate::AhMigrator,
		TrustedTeleportersWhileMigrating,
		TrustedTeleporters,
	>;
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = WeightInfoBounds<
		crate::weights::xcm::AssetHubPolkadotXcmWeight<RuntimeCall>,
		RuntimeCall,
		MaxInstructions,
	>;
	type Trader = (
		UsingComponents<
			WeightToFee,
			DotLocation,
			AccountId,
			Balances,
			ResolveTo<StakingPot, Balances>,
		>,
		// This trader allows to pay with any assets exchangeable to DOT with
		// [`AssetConversion`].
		cumulus_primitives_utility::SwapFirstAssetTrader<
			DotLocation,
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
	type UniversalAliases =
		(bridging::to_kusama::UniversalAliases, bridging::to_ethereum::UniversalAliases);
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
	// `GeneralAdmin` pluralistic body.
	pub const GeneralAdminBodyId: BodyId = BodyId::Administration;
	// StakingAdmin pluralistic body.
	pub const StakingAdminBodyId: BodyId = BodyId::Defense;
	// FellowshipAdmin pluralistic body.
	pub const FellowshipAdminBodyId: BodyId = BodyId::Index(FELLOWSHIP_ADMIN_INDEX);
	// `Treasurer` pluralistic body.
	pub const TreasurerBodyId: BodyId = BodyId::Treasury;
}

/// Type to convert the `GeneralAdmin` origin to a Plurality `Location` value.
pub type GeneralAdminToPlurality =
	OriginToPluralityVoice<RuntimeOrigin, GeneralAdmin, GeneralAdminBodyId>;

/// Type to convert the `StakingAdmin` origin to a Plurality `Location` value.
pub type StakingAdminToPlurality =
	OriginToPluralityVoice<RuntimeOrigin, StakingAdmin, StakingAdminBodyId>;

/// Type to convert the `FellowshipAdmin` origin to a Plurality `Location` value.
pub type FellowshipAdminToPlurality =
	OriginToPluralityVoice<RuntimeOrigin, FellowshipAdmin, FellowshipAdminBodyId>;

/// Type to convert the `Treasurer` origin to a Plurality `Location` value.
pub type TreasurerToPlurality = OriginToPluralityVoice<RuntimeOrigin, Treasurer, TreasurerBodyId>;

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
	// `Treasurer` origin to be used in XCM as a corresponding Plurality `Location` value.
	TreasurerToPlurality,
	// And a usual Signed origin to be used in XCM as a corresponding `AccountId32`.
	SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>,
);

/// For routing XCM messages which do not cross local consensus boundary.
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
	// Router which wraps and sends xcm to BridgeHub to be delivered to the Kusama
	// GlobalConsensus
	ToKusamaXcmRouter,
	// Router which wraps and sends xcm to BridgeHub to be delivered to the Ethereum
	// GlobalConsensus
	PausableExporter<
		crate::SnowbridgeSystemFrontend,
		(
			UnpaidRemoteExporter<
				(
					bridging::to_ethereum::EthereumNetworkExportTableV2,
					bridging::to_ethereum::EthereumNetworkExportTableV1,
				),
				XcmpQueue,
				UniversalLocation,
			>,
		),
	>,
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
		crate::weights::xcm::AssetHubPolkadotXcmWeight<RuntimeCall>,
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
	use xcm_builder::NetworkExportTableItem;

	parameter_types! {
		/// Base price of every Polkadot -> Kusama message. Can be adjusted via
		/// governance `set_storage` call.
		pub storage XcmBridgeHubRouterBaseFee: Balance = bp_bridge_hub_polkadot::estimate_polkadot_to_kusama_message_fee(
			bp_bridge_hub_kusama::BridgeHubKusamaBaseDeliveryFeeInKsms::get()
		);
		/// Price of every byte of the Polkadot -> Kusama message. Can be adjusted via
		/// governance `set_storage` call.
		pub storage XcmBridgeHubRouterByteFee: Balance = bp_bridge_hub_polkadot::estimate_polkadot_to_kusama_byte_fee();

		pub SiblingBridgeHubParaId: u32 = bp_bridge_hub_polkadot::BRIDGE_HUB_POLKADOT_PARACHAIN_ID;
		pub SiblingBridgeHub: Location = Location::new(1, Parachain(SiblingBridgeHubParaId::get()));
		/// Router expects payment with this `AssetId`.
		/// (`AssetId` has to be aligned with `BridgeTable`)
		pub XcmBridgeHubRouterFeeAssetId: AssetId = DotLocation::get().into();

		pub BridgeTable: Vec<NetworkExportTableItem> =
			Vec::new().into_iter()
			.chain(to_kusama::BridgeTable::get())
			.collect();
	}

	pub type NetworkExportTable = xcm_builder::NetworkExportTable<BridgeTable>;

	pub mod to_kusama {
		use super::*;

		parameter_types! {
			pub SiblingBridgeHubWithBridgeHubKusamaInstance: Location = Location::new(
				1,
				[
					Parachain(SiblingBridgeHubParaId::get()),
					PalletInstance(bp_bridge_hub_polkadot::WITH_BRIDGE_POLKADOT_TO_KUSAMA_MESSAGES_PALLET_INDEX),
				]
			);

			pub const KusamaNetwork: NetworkId = NetworkId::Kusama;
			pub AssetHubKusama: Location = Location::new(
				2,
				[
					GlobalConsensus(KusamaNetwork::get()),
					Parachain(kusama_runtime_constants::system_parachain::ASSET_HUB_ID),
				],
			);
			pub KsmLocation: Location = Location::new(2, GlobalConsensus(KusamaNetwork::get()));

			/// Set up exporters configuration.
			/// `Option<Asset>` represents static "base fee" which is used for total delivery fee calculation.
			pub BridgeTable: Vec<NetworkExportTableItem> = vec![
				NetworkExportTableItem::new(
					KusamaNetwork::get(),
					Some(vec![
						AssetHubKusama::get().interior.split_global().expect("invalid configuration for AssetHubPolkadot").1,
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
					(SiblingBridgeHubWithBridgeHubKusamaInstance::get(), GlobalConsensus(KusamaNetwork::get()))
				]
			);
		}

		impl Contains<(Location, Junction)> for UniversalAliases {
			fn contains(alias: &(Location, Junction)) -> bool {
				UniversalAliases::get().contains(alias)
			}
		}
		/// Allow any asset native to the Kusama ecosystem if it comes from Kusama Asset Hub.
		pub type KusamaAssetFromAssetHubKusama =
			RemoteAssetFromLocation<StartsWith<KsmLocation>, AssetHubKusama>;

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

	pub mod to_ethereum {
		use super::*;
		pub use bp_bridge_hub_polkadot::snowbridge::EthereumNetwork;
		use bp_bridge_hub_polkadot::snowbridge::{
			InboundQueuePalletInstance, InboundQueueV2PalletInstance,
		};

		parameter_types! {
			/// User fee for transfers from Polkadot to Ethereum.
			/// The fee is set to max Balance to disable the bridge until a fee is set by
			/// governance.
			pub const DefaultBridgeHubEthereumBaseFee: Balance = Balance::MAX;
			pub const DefaultBridgeHubEthereumBaseFeeV2: Balance = Balance::MAX;
			pub storage BridgeHubEthereumBaseFee: Balance = DefaultBridgeHubEthereumBaseFee::get();
			pub storage BridgeHubEthereumBaseFeeV2: Balance = DefaultBridgeHubEthereumBaseFeeV2::get();
			pub SiblingBridgeHubWithEthereumInboundQueueInstance: Location = Location::new(
				1,
				[
					Parachain(SiblingBridgeHubParaId::get()),
					PalletInstance(InboundQueuePalletInstance::get()),
				]
			);
			pub SiblingBridgeHubWithEthereumInboundQueueV2Instance: Location = Location::new(
				1,
				[
					Parachain(SiblingBridgeHubParaId::get()),
					PalletInstance(InboundQueueV2PalletInstance::get()),
				]
			);

			/// Set up exporters configuration.
			/// `Option<MultiAsset>` represents static "base fee" which is used for total delivery fee calculation.
			pub EthereumBridgeTableV1: Vec<NetworkExportTableItem> = vec![
				NetworkExportTableItem::new(
					EthereumNetwork::get(),
					Some(vec![crate::Junctions::Here]),
					SiblingBridgeHub::get(),
					Some((
						XcmBridgeHubRouterFeeAssetId::get(),
						BridgeHubEthereumBaseFee::get(),
					).into())
				),
			];

			pub EthereumBridgeTableV2: Vec<NetworkExportTableItem> = vec![
				NetworkExportTableItem::new(
					EthereumNetwork::get(),
					Some(vec![crate::Junctions::Here]),
					SiblingBridgeHub::get(),
					Some((
						XcmBridgeHubRouterFeeAssetId::get(),
						BridgeHubEthereumBaseFeeV2::get(),
					).into())
				),
			];

			/// Universal aliases
			pub UniversalAliases: BTreeSet<(Location, Junction)> = BTreeSet::from_iter(
				vec![
					(SiblingBridgeHubWithEthereumInboundQueueV2Instance::get(), GlobalConsensus(EthereumNetwork::get())),
					(SiblingBridgeHubWithEthereumInboundQueueInstance::get(), GlobalConsensus(EthereumNetwork::get())),
				]
			);
		}

		pub type EthereumNetworkExportTableV1 =
			xcm_builder::NetworkExportTable<EthereumBridgeTableV1>;

		pub type EthereumNetworkExportTableV2 =
			snowbridge_outbound_queue_primitives::v2::XcmFilterExporter<
				xcm_builder::NetworkExportTable<EthereumBridgeTableV2>,
				snowbridge_outbound_queue_primitives::v2::XcmForSnowbridgeV2,
			>;

		pub type EthereumAssetFromEthereum =
			IsForeignConcreteAsset<FromNetwork<UniversalLocation, EthereumNetwork>>;

		impl Contains<(Location, Junction)> for UniversalAliases {
			fn contains(alias: &(Location, Junction)) -> bool {
				UniversalAliases::get().contains(alias)
			}
		}
	}

	/// Benchmarks helper for bridging configuration.
	#[cfg(feature = "runtime-benchmarks")]
	pub struct BridgingBenchmarksHelper;

	#[cfg(feature = "runtime-benchmarks")]
	impl BridgingBenchmarksHelper {
		pub fn prepare_universal_alias() -> Option<(Location, Junction)> {
			let alias =
				to_kusama::UniversalAliases::get().into_iter().find_map(|(location, junction)| {
					match to_kusama::SiblingBridgeHubWithBridgeHubKusamaInstance::get()
						.eq(&location)
					{
						true => Some((location, junction)),
						false => None,
					}
				});
			Some(alias.expect("we expect here BridgeHubPolkadot to Kusama mapping at least"))
		}
	}
}

#[test]
fn foreign_pallet_has_correct_local_account() {
	use sp_core::crypto::{Ss58AddressFormat, Ss58Codec};
	use xcm_executor::traits::ConvertLocation;

	const COLLECTIVES_PARAID: u32 = 1001;
	const FELLOWSHIP_SALARY_PALLET_ID: u8 =
		collectives_polkadot_runtime_constants::FELLOWSHIP_SALARY_PALLET_INDEX;
	let fellowship_salary =
		(Parent, Parachain(COLLECTIVES_PARAID), PalletInstance(FELLOWSHIP_SALARY_PALLET_ID));
	let account = LocationToAccountId::convert_location(&fellowship_salary.into()).unwrap();
	let polkadot = Ss58AddressFormat::try_from("polkadot").unwrap();
	let address = Ss58Codec::to_ss58check_with_version(&account, polkadot);
	assert_eq!(address, "13w7NdvSR1Af8xsQTArDtZmVvjE8XhWNdL4yed3iFHrUNCnS");
}
