// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot. If not, see <http://www.gnu.org/licenses/>.

//! XCM configurations for the Kusama runtime.

use super::{
	parachains_origin, AccountId, AllPalletsWithSystem, Balances, Dmp, Fellows, ParaId, Runtime,
	RuntimeCall, RuntimeEvent, RuntimeOrigin, StakingAdmin, TransactionByteFee, Treasury,
	WeightToFee, XcmPallet,
};
use frame_support::{
	parameter_types,
	traits::{Contains, Equals, Everything, Nothing},
};
use frame_system::EnsureRoot;
use kusama_runtime_constants::{currency::CENTS, system_parachain::*};
use polkadot_runtime_common::{
	xcm_sender::{ChildParachainRouter, ExponentialPrice},
	ToAuthor,
};
use sp_core::ConstU32;
use xcm::latest::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AllowExplicitUnpaidExecutionFrom, AllowKnownQueryResponses,
	AllowSubscriptionsFrom, AllowTopLevelPaidExecutionFrom, ChildParachainAsNative,
	ChildParachainConvertsVia, DescribeAllTerminal, DescribeFamily, FrameTransactionalProcessor,
	FungibleAdapter, HashedDescription, IsChildSystemParachain, IsConcrete, MintLocation,
	OriginToPluralityVoice, SignedAccountId32AsNative, SignedToAccountId32,
	SovereignSignedViaLocation, TakeWeightCredit, TrailingSetTopicAsId, UsingComponents,
	WeightInfoBounds, WithComputedOrigin, WithUniqueTopic, XcmFeeManagerFromComponents,
	XcmFeeToAccount,
};

parameter_types! {
	pub const RootLocation: Location = Here.into_location();
	/// The location of the KSM token, from the context of this chain. Since this token is native to this
	/// chain, we make it synonymous with it and thus it is the `Here` location, which means "equivalent to
	/// the context".
	pub const TokenLocation: Location = Here.into_location();
	/// The Kusama network ID. This is named.
	pub const ThisNetwork: NetworkId = Kusama;
	/// Our XCM location ancestry - i.e. our location within the Consensus Universe.
	///
	/// Since Kusama is a top-level relay-chain with its own consensus, it's just our network ID.
	pub UniversalLocation: InteriorLocation = ThisNetwork::get().into();
	/// The check account, which holds any native assets that have been teleported out and not back in (yet).
	pub CheckAccount: AccountId = XcmPallet::check_account();
	/// The check account that is allowed to mint assets locally.
	pub LocalCheckAccount: (AccountId, MintLocation) = (CheckAccount::get(), MintLocation::Local);
	/// Account of the treasury pallet.
	pub TreasuryAccount: AccountId = Treasury::account_id();
}

/// The canonical means of converting a `Location` into an `AccountId`, used when we want to
/// determine the sovereign account controlled by a location.
pub type SovereignAccountOf = (
	// We can convert a child parachain using the standard `AccountId` conversion.
	ChildParachainConvertsVia<ParaId, AccountId>,
	// We can directly alias an `AccountId32` into a local account.
	AccountId32Aliases<ThisNetwork, AccountId>,
	// Foreign locations alias into accounts according to a hash of their standard description.
	HashedDescription<AccountId, DescribeFamily<DescribeAllTerminal>>,
);

/// Our asset transactor. This is what allows us to interest with the runtime facilities from the
/// point of view of XCM-only concepts like `Location` and `Asset`.
///
/// Ours is only aware of the Balances pallet, which is mapped to `TokenLocation`.
pub type LocalAssetTransactor = FungibleAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<TokenLocation>,
	// We can convert the Locations with our converter above:
	SovereignAccountOf,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We track our teleports in/out to keep total issuance correct.
	LocalCheckAccount,
>;

/// The means that we convert the XCM message origin location into a local dispatch origin.
type LocalOriginConverter = (
	// A `Signed` origin of the sovereign account that the original location controls.
	SovereignSignedViaLocation<SovereignAccountOf, RuntimeOrigin>,
	// A child parachain, natively expressed, has the `Parachain` origin.
	ChildParachainAsNative<parachains_origin::Origin, RuntimeOrigin>,
	// The AccountId32 location type can be expressed natively as a `Signed` origin.
	SignedAccountId32AsNative<ThisNetwork, RuntimeOrigin>,
);

parameter_types! {
	/// The amount of weight an XCM operation takes. This is a safe overestimate.
	pub const BaseXcmWeight: Weight = Weight::from_parts(1_000_000_000, 64 * 1024);
	/// Maximum number of instructions in a single XCM fragment. A sanity check against weight
	/// calculations getting too crazy.
	pub const MaxInstructions: u32 = 100;
	/// The asset ID for the asset that we use to pay for message delivery fees.
	pub FeeAssetId: AssetId = AssetId(TokenLocation::get());
	/// The base fee for the message delivery fees.
	pub const BaseDeliveryFee: u128 = CENTS.saturating_mul(3);
}

pub type PriceForChildParachainDelivery =
	ExponentialPrice<FeeAssetId, BaseDeliveryFee, TransactionByteFee, Dmp>;

/// The XCM router. When we want to send an XCM message, we use this type. It amalgamates all of our
/// individual routers.
pub type XcmRouter = WithUniqueTopic<(
	// Only one router so far - use DMP to communicate with child parachains.
	ChildParachainRouter<Runtime, XcmPallet, PriceForChildParachainDelivery>,
)>;

parameter_types! {
	pub const Ksm: AssetFilter = Wild(AllOf { fun: WildFungible, id: AssetId(TokenLocation::get()) });
	pub AssetHubLocation: Location = Parachain(ASSET_HUB_ID).into_location();
	pub KsmForAssetHub: (AssetFilter, Location) = (Ksm::get(), AssetHubLocation::get());
	pub Encointer: Location = Parachain(ENCOINTER_ID).into_location();
	pub KsmForEncointer: (AssetFilter, Location) = (Ksm::get(), Encointer::get());
	pub BridgeHubLocation: Location = Parachain(BRIDGE_HUB_ID).into_location();
	pub KsmForBridgeHub: (AssetFilter, Location) = (Ksm::get(), BridgeHubLocation::get());
	pub Broker: Location = Parachain(BROKER_ID).into_location();
	pub KsmForBroker: (AssetFilter, Location) = (Ksm::get(), Broker::get());
	pub People: Location = Parachain(PEOPLE_ID).into_location();
	pub KsmForPeople: (AssetFilter, Location) = (Ksm::get(), People::get());
	pub const MaxAssetsIntoHolding: u32 = 64;
}

/// Kusama Relay recognizes/respects AssetHub, Encointer, and BridgeHub chains as teleporters.
pub type TrustedTeleporters = (
	xcm_builder::Case<KsmForAssetHub>,
	xcm_builder::Case<KsmForEncointer>,
	xcm_builder::Case<KsmForBridgeHub>,
	xcm_builder::Case<KsmForBroker>,
	xcm_builder::Case<KsmForPeople>,
);

pub struct OnlyParachains;
impl Contains<Location> for OnlyParachains {
	fn contains(loc: &Location) -> bool {
		matches!(loc.unpack(), (0, [Parachain(_)]))
	}
}

pub struct LocalPlurality;
impl Contains<Location> for LocalPlurality {
	fn contains(loc: &Location) -> bool {
		matches!(loc.unpack(), (0, [Plurality { .. }]))
	}
}

/// The barriers one of which must be passed for an XCM message to be executed.
pub type Barrier = TrailingSetTopicAsId<(
	// Weight that is paid for may be consumed.
	TakeWeightCredit,
	// Expected responses are OK.
	AllowKnownQueryResponses<XcmPallet>,
	WithComputedOrigin<
		(
			// If the message is one that immediately attempts to pay for execution, then allow it.
			AllowTopLevelPaidExecutionFrom<Everything>,
			// Messages coming from system parachains need not pay for execution.
			AllowExplicitUnpaidExecutionFrom<IsChildSystemParachain<ParaId>>,
			// Subscriptions for version tracking are OK.
			AllowSubscriptionsFrom<OnlyParachains>,
		),
		UniversalLocation,
		ConstU32<8>,
	>,
)>;

/// Locations that will not be charged fees in the executor, neither for execution nor delivery.
/// We only waive fees for system functions, which these locations represent.
pub type WaivedLocations = (SystemParachains, Equals<RootLocation>, LocalPlurality);

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	type AssetTransactor = LocalAssetTransactor;
	type OriginConverter = LocalOriginConverter;
	type IsReserve = ();
	type IsTeleporter = TrustedTeleporters;
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = WeightInfoBounds<
		crate::weights::xcm::KusamaXcmWeight<RuntimeCall>,
		RuntimeCall,
		MaxInstructions,
	>;
	// The weight trader piggybacks on the existing transaction-fee conversion logic.
	type Trader =
		UsingComponents<WeightToFee, TokenLocation, AccountId, Balances, ToAuthor<Runtime>>;
	type ResponseHandler = XcmPallet;
	type AssetTrap = XcmPallet;
	type AssetLocker = ();
	type AssetExchanger = ();
	type AssetClaims = XcmPallet;
	type SubscriptionService = XcmPallet;
	type PalletInstancesInfo = AllPalletsWithSystem;
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	type FeeManager = XcmFeeManagerFromComponents<
		WaivedLocations,
		XcmFeeToAccount<Self::AssetTransactor, AccountId, TreasuryAccount>,
	>;
	// No bridges yet...
	type MessageExporter = ();
	type UniversalAliases = Nothing;
	type CallDispatcher = RuntimeCall;
	type SafeCallFilter = Everything;
	type Aliasers = Nothing;
	type TransactionalProcessor = FrameTransactionalProcessor;
}

parameter_types! {
	// StakingAdmin pluralistic body.
	pub const StakingAdminBodyId: BodyId = BodyId::Defense;
	// Fellows pluralistic body.
	pub const FellowsBodyId: BodyId = BodyId::Technical;
}

/// Type to convert an `Origin` type value into a `Location` value which represents an interior
/// location of this chain.
pub type LocalOriginToLocation = (
	// And a usual Signed origin to be used in XCM as a corresponding AccountId32
	SignedToAccountId32<RuntimeOrigin, AccountId, ThisNetwork>,
);

/// Type to convert the `StakingAdmin` origin to a Plurality `Location` value.
pub type StakingAdminToPlurality =
	OriginToPluralityVoice<RuntimeOrigin, StakingAdmin, StakingAdminBodyId>;

/// Type to convert the Fellows origin to a Plurality `Location` value.
pub type FellowsToPlurality = OriginToPluralityVoice<RuntimeOrigin, Fellows, FellowsBodyId>;

/// Type to convert a pallet `Origin` type value into a `Location` value which represents an
/// interior location of this chain for a destination chain.
pub type LocalPalletOrSignedOriginToLocation = (
	// StakingAdmin origin to be used in XCM as a corresponding Plurality `Location` value.
	StakingAdminToPlurality,
	// Fellows origin to be used in XCM as a corresponding Plurality `Location` value.
	FellowsToPlurality,
	// And a usual Signed origin to be used in XCM as a corresponding AccountId32
	SignedToAccountId32<RuntimeOrigin, AccountId, ThisNetwork>,
);

impl pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	// This is basically safe to enable for everyone (safe the possibility of someone spamming the
	// parachain if they're willing to pay the KSM to send from the Relay-chain).
	type SendXcmOrigin =
		xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalPalletOrSignedOriginToLocation>;
	type XcmRouter = XcmRouter;
	// Anyone can execute XCM messages locally.
	type ExecuteXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = xcm_executor::XcmExecutor<XcmConfig>;
	// Anyone is able to use teleportation regardless of who they are and what they want to
	// teleport.
	type XcmTeleportFilter = Everything;
	// Anyone is able to use reserve transfers regardless of who they are and what they want to
	// transfer.
	type XcmReserveTransferFilter = Everything;
	type Weigher = WeightInfoBounds<
		crate::weights::xcm::KusamaXcmWeight<RuntimeCall>,
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
	type SovereignAccountOf = SovereignAccountOf;
	type MaxLockers = ConstU32<8>;
	type MaxRemoteLockConsumers = ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
	type WeightInfo = crate::weights::pallet_xcm::WeightInfo<Runtime>;
	type AdminOrigin = EnsureRoot<AccountId>;
}

#[test]
fn karura_liquid_staking_xcm_has_sane_weight_upper_limt() {
	use codec::Decode;
	use frame_support::dispatch::GetDispatchInfo;
	use xcm::VersionedXcm;
	use xcm_executor::traits::WeightBounds;

	// should be [WithdrawAsset, BuyExecution, Transact, RefundSurplus, DepositAsset]
	let blob = hex_literal::hex!("02140004000000000700e40b540213000000000700e40b54020006010700c817a804341801000006010b00c490bf4302140d010003ffffffff000100411f");
	let Ok(VersionedXcm::V2(old_xcm_v2)) =
		VersionedXcm::<super::RuntimeCall>::decode(&mut &blob[..])
	else {
		panic!("can't decode XCM blob")
	};
	let old_xcm_v3: xcm::v3::Xcm<super::RuntimeCall> =
		old_xcm_v2.try_into().expect("conversion from v2 to v3 works");
	let mut xcm: Xcm<super::RuntimeCall> =
		old_xcm_v3.try_into().expect("conversion from v3 to latest works");
	let weight = <XcmConfig as xcm_executor::Config>::Weigher::weight(&mut xcm)
		.expect("weighing XCM failed");

	// Test that the weigher gives us a sensible weight but don't exactly hard-code it, otherwise it
	// will be out of date after each re-run.
	assert!(weight.all_lte(Weight::from_parts(30_313_281_000, 72_722)));

	let Some(Transact { require_weight_at_most, call, .. }) =
		xcm.inner_mut().iter_mut().find(|inst| matches!(inst, Transact { .. }))
	else {
		panic!("no Transact instruction found")
	};
	// should be pallet_utility.as_derivative { index: 0, call: pallet_staking::bond_extra {
	// max_additional: 2490000000000 } }
	let message_call = call.take_decoded().expect("can't decode Transact call");
	let call_weight = message_call.get_dispatch_info().weight;
	// Ensure that the Transact instruction is giving a sensible `require_weight_at_most` value
	assert!(
		call_weight.all_lte(*require_weight_at_most),
		"call weight ({:?}) was not less than or equal to require_weight_at_most ({:?})",
		call_weight,
		require_weight_at_most
	);
}
