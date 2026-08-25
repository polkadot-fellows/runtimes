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
	assets::{hollar::HollarFromHydration, NativeAndAssets},
	AccountId, AllPalletsWithSystem, AssetConversion, AssetRate, Assets as AssetsPallet, Balance,
	Balances, CollatorSelection, DotWeightToFee as WeightToFee, ParachainInfo, ParachainSystem,
	PolkadotXcm, Runtime, RuntimeCall, RuntimeEvent, RuntimeHoldReason, RuntimeOrigin, XcmpQueue,
};
use crate::{TransactionByteFee, CENTS};
use assets_common::matching::RemoteAssetFromLocation;
use core::marker::PhantomData;
use cumulus_primitives_utility::TakeFirstAssetTrader;
use frame_support::{
	parameter_types,
	traits::{
		fungible::HoldConsideration,
		tokens::{
			imbalance::{ResolveAssetTo, ResolveTo},
			ConversionFromAssetBalance, ConversionToAssetBalance,
		},
		ConstU32, Contains, Equals, Everything, LinearStoragePrice, Nothing,
	},
};
use frame_system::EnsureRoot;
use pallet_xcm::{AuthorizedAliasers, XcmPassthrough};
use parachains_common::{
	xcm_config::{
		AliasAccountId32FromSiblingSystemChain, AllSiblingSystemParachains,
		AssetFeeAsExistentialDepositMultiplier, ConcreteAssetFromSystem,
		ParentRelayOrSiblingParachains, RelayOrOtherSystemParachains,
	},
	TREASURY_PALLET_ID,
};
use polkadot_parachain_primitives::primitives::Sibling;
use polkadot_runtime_constants::{fellowship::IsFellowshipVoice, system_parachain};
use sp_runtime::traits::{AccountIdConversion, TryConvertInto};
use xcm::latest::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AliasChildLocation, AliasOriginRootUsingFilter,
	AllowExplicitUnpaidExecutionFrom, AllowKnownQueryResponses, AllowSubscriptionsFrom,
	AllowTopLevelPaidExecutionFrom, DenyReserveTransferToRelayChain, DenyThenTry,
	DescribeAllTerminal, DescribeFamily, DescribeTerminus, EnsureXcmOrigin,
	FrameTransactionalProcessor, FungibleAdapter, FungiblesAdapter, HashedDescription, IsConcrete,
	LocationAsSuperuser, MatchedConvertedConcreteId, NoChecking, ParentIsPreset,
	RelayChainAsNative, SendXcmFeeToAccount, SiblingParachainAsNative, SiblingParachainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SingleAssetExchangeAdapter,
	SovereignSignedViaLocation, StartsWith, TakeWeightCredit, TrailingSetTopicAsId,
	UsingComponents, WeightInfoBounds, WithComputedOrigin, WithLatestLocationConverter,
	WithUniqueTopic, XcmFeeManagerFromComponents,
};
use xcm_executor::{
	traits::{AssetExchange, ConvertLocation, WeightTrader},
	AssetsInHolding, XcmExecutor,
};

pub use system_parachains_constants::polkadot::locations::{
	AssetHubLocation, AssetHubPlurality, RelayChainLocation,
};

parameter_types! {
	pub const RootLocation: Location = Location::here();
	pub const RelayLocation: Location = Location::parent();
	pub const RelayNetwork: Option<NetworkId> = Some(NetworkId::Polkadot);
	pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
	pub UniversalLocation: InteriorLocation =
		[GlobalConsensus(RelayNetwork::get().unwrap()), Parachain(ParachainInfo::parachain_id().into())].into();
	pub const MaxInstructions: u32 = 100;
	pub const MaxAssetsIntoHolding: u32 = 64;
	pub FellowshipLocation: Location = Location::new(1, Parachain(system_parachain::COLLECTIVES_ID));
	/// The asset ID for the asset that we use to pay for message delivery fees. Just DOT.
	pub FeeAssetId: AssetId = AssetId(RelayLocation::get());
	/// The base fee for the message delivery fees.
	pub const BaseDeliveryFee: u128 = CENTS.saturating_mul(3);
	// TODO: replace this with DAP account (for collecting fees) #1137
	pub TreasuryAccount: AccountId = TREASURY_PALLET_ID.into_account_truncating();
	pub CheckingAccount: AccountId = PolkadotXcm::check_account();
	pub RelayTreasuryLocation: Location =
		(Parent, PalletInstance(polkadot_runtime_constants::TREASURY_PALLET_ID)).into();
	// TODO: replace this with DAP account (for collecting fees) #1137
	pub RelayTreasuryPalletAccount: AccountId =
		LocationToAccountId::convert_location(&RelayTreasuryLocation::get())
			.unwrap_or(TreasuryAccount::get());
	// TODO: replace this with DAP account (for collecting fees) #1137
	pub StakingPot: AccountId = CollatorSelection::account_id();
}

pub type PriceForParentDelivery = polkadot_runtime_common::xcm_sender::ExponentialPrice<
	FeeAssetId,
	BaseDeliveryFee,
	TransactionByteFee,
	ParachainSystem,
>;

pub type PriceForSiblingParachainDelivery = polkadot_runtime_common::xcm_sender::ExponentialPrice<
	FeeAssetId,
	BaseDeliveryFee,
	TransactionByteFee,
	XcmpQueue,
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
	// Here/local root location to `AccountId`.
	HashedDescription<AccountId, DescribeTerminus>,
	// Foreign locations alias into accounts according to a hash of their standard description.
	HashedDescription<AccountId, DescribeFamily<DescribeAllTerminal>>,
);

/// Means for transacting the native currency on this chain.
pub type FungibleTransactor = FungibleAdapter<
	// Use this implementation of `fungible::*`.
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<RelayLocation>,
	// Convert an XCM `Location` into a local account ID:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We don't track any teleports of `Balances`.
	(),
>;

/// Matches every asset that comes from outside, i.e. everything held in the `Assets` pallet.
pub type ForeignAssetsMatcher =
	assets_common::ForeignAssetsConvertedConcreteId<(), Balance, Location>;

/// Means for transacting other fungible tokens on this chain.
pub type FungiblesTransactor = FungiblesAdapter<
	// Use this implementation of `fungibles::*`.
	AssetsPallet,
	// Match everything that comes from outside.
	ForeignAssetsMatcher,
	// Convert an XCM `Location` into a local account ID.
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// No checking.
	NoChecking,
	// We still need to specify the checking account.
	CheckingAccount,
>;

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with XCM's `Transact`.
///
/// There is an `OriginKind` that can bias the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
	// Sovereign account converter; this attempts to derive an `AccountId` from the origin location
	// using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
	// foreign chains who want to have a local sovereign account on this chain that they control.
	SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
	// Native converter for Relay-chain (Parent) location; will convert to a `Relay` origin when
	// recognized.
	RelayChainAsNative<RelayChainOrigin, RuntimeOrigin>,
	// Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
	// recognized.
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, RuntimeOrigin>,
	// AssetHub or Relay can execute as root (based on: https://github.com/polkadot-fellows/runtimes/issues/651).
	// This will allow them to issue a transaction from the Root origin.
	LocationAsSuperuser<(Equals<RelayChainLocation>, Equals<AssetHubLocation>), RuntimeOrigin>,
	// Native signed account converter; this just converts an `AccountId32` origin into a normal
	// `RuntimeOrigin::Signed` origin of the same 32-byte value.
	SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
	// XCM origins can be represented natively under the XCM pallet's `Xcm` origin.
	XcmPassthrough<RuntimeOrigin>,
);

pub struct LocalPlurality;
impl Contains<Location> for LocalPlurality {
	fn contains(location: &Location) -> bool {
		matches!(location.unpack(), (0, [Plurality { .. }]))
	}
}

pub struct ParentOrParentsPlurality;
impl Contains<Location> for ParentOrParentsPlurality {
	fn contains(location: &Location) -> bool {
		matches!(location.unpack(), (1, []) | (1, [Plurality { .. }]))
	}
}

/// A location matching the Core Technical Fellowship.
pub type FellowsPlurality = IsFellowshipVoice<FellowshipLocation>;

pub type Barrier = TrailingSetTopicAsId<
	DenyThenTry<
		DenyReserveTransferToRelayChain,
		(
			// Allow local users to buy weight credit.
			TakeWeightCredit,
			// Expected responses are OK.
			AllowKnownQueryResponses<PolkadotXcm>,
			WithComputedOrigin<
				(
					// If the message is one that immediately attempts to pay for execution, then
					// allow it.
					AllowTopLevelPaidExecutionFrom<Everything>,
					// Parent and its pluralities (i.e. governance bodies) get free execution.
					AllowExplicitUnpaidExecutionFrom<
						(
							ParentOrParentsPlurality,
							FellowsPlurality,
							Equals<RelayTreasuryLocation>,
							Equals<AssetHubLocation>,
							AssetHubPlurality,
						),
						// Barriers run before any fee is taken: this must stay computation-only.
						// Do not pass `TrustedAliasers` here.
						CheapTrustedAliasers,
					>,
					// Subscriptions for version tracking are OK.
					AllowSubscriptionsFrom<ParentRelayOrSiblingParachains>,
				),
				UniversalLocation,
				ConstU32<8>,
			>,
		),
	>,
>;

/// Locations that will not be charged fees in the executor, neither for execution nor delivery. We
/// only waive fees for system functions, which these locations represent.
pub type WaivedLocations = (
	Equals<RootLocation>,
	RelayOrOtherSystemParachains<AllSiblingSystemParachains, Runtime>,
	Equals<RelayTreasuryLocation>,
	FellowsPlurality,
	LocalPlurality,
);

/// Aliasing rules that are pure computation, so the `AllowExplicitUnpaidExecutionFrom` barrier can
/// evaluate them before any fee is charged.
///
/// Do not add storage-reading filters here: the barrier calls this once per `AliasOrigin` (up to 5)
/// on keys the message chooses, and may then reject the message without taking a fee. That is why
/// `AuthorizedAliasers` belongs to [`TrustedAliasers`] alone.
///
/// - Allow any origin to alias into a child sub-location (equivalent to DescendOrigin),
/// - Allow same accounts to alias into each other across system chains,
/// - Allow AssetHub root to alias into anything.
pub type CheapTrustedAliasers = (
	AliasChildLocation,
	AliasAccountId32FromSiblingSystemChain,
	AliasOriginRootUsingFilter<AssetHubLocation, Everything>,
);

/// Defines origin aliasing rules for this chain, used by `xcm_executor::Config::Aliasers` at
/// execution time: everything in [`CheapTrustedAliasers`], plus origins explicitly authorized by
/// the alias target location.
pub type TrustedAliasers = (CheapTrustedAliasers, AuthorizedAliasers<Runtime>);

/// The asset transactors responsible for handling assets in XCM.
pub type AssetTransactors = (FungibleTransactor, FungiblesTransactor);

/// Reserve transfers this chain accepts:
///
/// - HOLLAR from Hydration, which issues it; and
/// - assets *native to Asset Hub* — the ones its trust backed `Assets` and pool instances issue —
///   sent from Asset Hub.
///
/// The second rule is deliberately restricted to Asset Hub's own assets rather than everything
/// Asset Hub happens to custody. Trusting a chain as the reserve for an asset it did not issue
/// gives that asset two reserves, and `ReserveAssetDeposited` *mints* locally, so the second
/// reserve can credit this chain with holdings the real reserve is not backing. Concretely, a
/// blanket rule would let Asset Hub mint DOT here (`FungibleTransactor` has no checking account),
/// and mint HOLLAR that could then be withdrawn against Hydration's backing.
///
/// Broad-ish reserve trust is still paired with a narrow registry: `pallet-assets` here has
/// `CreateOrigin = EnsureNever`, so an incoming asset is only ever credited if root already
/// registered it locally.
pub type TrustedReserves =
	(HollarFromHydration, RemoteAssetFromLocation<StartsWith<AssetHubLocation>, AssetHubLocation>);

pub type WeightToNativeFee = WeightToFee<Runtime>;

/// Matches DOT on top of everything [`ForeignAssetsMatcher`] matches.
///
/// `ForeignAssetsMatcher` deliberately excludes the relay chain's own token, but the assets the
/// executor asks *for* — delivery fees — are priced in DOT, so the exchanger has to recognise it.
pub type NativeAndForeignAssetsMatcher = (
	ForeignAssetsMatcher,
	MatchedConvertedConcreteId<
		Location,
		Balance,
		Equals<RelayLocation>,
		WithLatestLocationConverter<Location>,
		TryConvertInto,
	>,
);

/// Prices weight in any asset that governance registered a rate for in `pallet-asset-rate`.
///
/// The weight is first priced in DOT, then converted to the asset at the registered rate. Assets
/// without a rate are rejected, which makes the trader fall through to the next component.
pub type WeightToAssetRateFee =
	AssetFeeAsExistentialDepositMultiplier<Runtime, WeightToNativeFee, AssetRate, ()>;

/// The fungible amount of `asset`, or [`Balance::MAX`] if it is not fungible, so that a
/// nonsensical quote never wins a price comparison.
fn fungible_amount(asset: &Asset) -> Balance {
	match asset.fun {
		Fungible(amount) => amount,
		_ => Balance::MAX,
	}
}

/// A [`WeightTrader`] that charges through whichever of `A` and `B` asks the payer for less.
///
/// The plain tuple `WeightTrader` takes the first component that succeeds, which would make the
/// price a payer gets depend on declaration order — a pool too thin to price a fee sanely would
/// win simply for being listed first. Here both halves quote in the asset the payer offered, so
/// the quotes are directly comparable, and the cheaper one charges. If it cannot settle, the other
/// is tried. On a tie `A` wins.
///
/// The comparison is made afresh for every purchase: a program may buy weight more than once, an
/// earlier purchase may have moved the pool's price, and a half that already holds credit may
/// refuse to buy again — [`TakeFirstAssetTrader`] does — in which case the other one takes over.
/// Refunds are handed out the way the tuple does it, by the first half that has anything to give
/// back.
pub struct CheaperTrader<A, B> {
	first: A,
	second: B,
}

impl<A: WeightTrader, B: WeightTrader> CheaperTrader<A, B> {
	/// Whether `A` is the half to charge through for `given`.
	fn first_is_cheaper(&mut self, weight: Weight, given: AssetId, context: &XcmContext) -> bool {
		let a = self.first.quote_weight(weight, given.clone(), context).ok();
		let b = self.second.quote_weight(weight, given, context).ok();
		match (a, b) {
			(Some(a), Some(b)) => fungible_amount(&a) <= fungible_amount(&b),
			(Some(_), None) => true,
			(None, Some(_)) => false,
			// Neither can price it; the order they refuse in does not matter.
			(None, None) => true,
		}
	}
}

impl<A: WeightTrader, B: WeightTrader> WeightTrader for CheaperTrader<A, B> {
	fn new() -> Self {
		Self { first: A::new(), second: B::new() }
	}

	fn buy_weight(
		&mut self,
		weight: Weight,
		payment: AssetsInHolding,
		context: &XcmContext,
	) -> Result<AssetsInHolding, (AssetsInHolding, XcmError)> {
		let first_is_cheaper = match payment.fungible.first_key_value() {
			Some((given, _)) => self.first_is_cheaper(weight, given.clone(), context),
			// Nothing to price; let the first half produce the error.
			None => true,
		};

		// The cheaper half charges; if it cannot settle the fee after all, the other one is tried.
		if first_is_cheaper {
			self.first
				.buy_weight(weight, payment, context)
				.or_else(|(payment, _)| self.second.buy_weight(weight, payment, context))
		} else {
			self.second
				.buy_weight(weight, payment, context)
				.or_else(|(payment, _)| self.first.buy_weight(weight, payment, context))
		}
	}

	fn refund_weight(&mut self, weight: Weight, context: &XcmContext) -> Option<AssetsInHolding> {
		self.first
			.refund_weight(weight, context)
			.or_else(|| self.second.refund_weight(weight, context))
	}

	fn quote_weight(
		&mut self,
		weight: Weight,
		given: AssetId,
		context: &XcmContext,
	) -> Result<Asset, XcmError> {
		let a = self.first.quote_weight(weight, given.clone(), context);
		let b = self.second.quote_weight(weight, given, context);
		match (a, b) {
			(Ok(a), Ok(b)) => Ok(if fungible_amount(&a) <= fungible_amount(&b) { a } else { b }),
			(Ok(a), Err(_)) => Ok(a),
			(Err(_), Ok(b)) => Ok(b),
			(Err(error), Err(_)) => Err(error),
		}
	}
}

/// Buys XCM execution weight with any asset that has an [`AssetConversion`] pool against DOT, by
/// swapping exactly enough of it for the DOT the weight costs. The DOT lands in the staking pot.
pub type PoolTrader = cumulus_primitives_utility::SwapFirstAssetTrader<
	RelayLocation,
	AssetConversion,
	WeightToNativeFee,
	NativeAndAssets,
	ForeignAssetsMatcher,
	ResolveAssetTo<StakingPot, NativeAndAssets>,
	AccountId,
>;

/// Buys XCM execution weight with any asset governance registered a rate for, taking it in kind
/// at that rate.
///
/// The fee is deposited *in that asset* into [`StakingPot`], and a deposit that fails is burned
/// rather than refunded. `pallet-assets` refuses to open an account for an asset that is not
/// `is_sufficient` in an account with no provider reference, and refuses any deposit below the
/// asset's `min_balance`. So a rated asset must be registered `is_sufficient = true` with a
/// `min_balance` no larger than the smallest fee, or [`StakingPot`] must be given the asset — or
/// the existential deposit in DOT — before the rate is registered. The same holds for
/// [`FeesAtAssetRate`] and [`RelayTreasuryPalletAccount`].
pub type AssetRateTrader = TakeFirstAssetTrader<
	AccountId,
	WeightToAssetRateFee,
	ForeignAssetsMatcher,
	AssetsPallet,
	ResolveAssetTo<StakingPot, AssetsPallet>,
>;

/// All ways of paying for XCM execution fees: DOT itself, or whichever of the
/// [`AssetConversion`] pool and the governance-registered rate asks the payer for less.
pub type Traders = (
	UsingComponents<
		WeightToNativeFee,
		RelayLocation,
		AccountId,
		Balances,
		ResolveTo<StakingPot, Balances>,
	>,
	CheaperTrader<PoolTrader, AssetRateTrader>,
);

/// Swaps the asset offered for fees against the asset the executor prices them in — DOT — through
/// an [`AssetConversion`] pool.
///
/// This is what lets delivery fees, which the routers always quote in DOT, be paid in another
/// asset: the executor asks this exchanger for DOT, and it sells just enough of the offered asset
/// to get it.
pub type PoolAssetsExchanger = SingleAssetExchangeAdapter<
	AssetConversion,
	NativeAndAssets,
	NativeAndForeignAssetsMatcher,
	AccountId,
>;

/// The single fungible asset of `assets`, if that is all it holds.
fn single_fungible(assets: &Assets) -> Option<(Location, Balance)> {
	match assets.inner().as_slice() {
		[Asset { id: AssetId(location), fun: Fungible(amount) }] =>
			Some((location.clone(), *amount)),
		_ => None,
	}
}

/// An [`AssetExchange`] that settles through whichever of `A` and `B` asks the payer for less.
///
/// The plain tuple `AssetExchange` takes the first component that answers, which would let a pool
/// too thin to price a fee sanely win over a governance rate simply for being listed first.
///
/// Both `maximal` modes are compared the same way, on the smaller quote. That is correct here
/// because this exchanger only ever prices fees: the executor asks for DOT and the payer parts
/// with the other asset, so in both directions the quote is denominated in what the payer gives
/// up. The `ExchangeAsset` instruction, where `maximal` would mean "get me as much as possible",
/// is not weighed on this chain and so is unreachable.
pub struct CheaperExchanger<A, B>(PhantomData<(A, B)>);

impl<A: AssetExchange, B: AssetExchange> CheaperExchanger<A, B> {
	fn quoted_amount<E: AssetExchange>(
		give: &Assets,
		want: &Assets,
		maximal: bool,
	) -> Option<Balance> {
		E::quote_exchange_price(give, want, maximal)
			.and_then(|quote| single_fungible(&quote))
			.map(|(_, amount)| amount)
	}

	/// Whether `A` is the half to settle through.
	fn first_is_cheaper(give: &Assets, want: &Assets, maximal: bool) -> bool {
		match (
			Self::quoted_amount::<A>(give, want, maximal),
			Self::quoted_amount::<B>(give, want, maximal),
		) {
			(Some(a), Some(b)) => a <= b,
			(Some(_), None) => true,
			(None, Some(_)) => false,
			// Neither can price it; the order they refuse in does not matter.
			(None, None) => true,
		}
	}
}

impl<A: AssetExchange, B: AssetExchange> AssetExchange for CheaperExchanger<A, B> {
	fn exchange_asset(
		origin: Option<&Location>,
		give: AssetsInHolding,
		want: &Assets,
		maximal: bool,
	) -> Result<AssetsInHolding, AssetsInHolding> {
		// `give` was sized by a prior `quote_exchange_price`, so settle through the half that
		// produced that quote. Falling back to the other keeps a half that quoted but then could
		// not settle from failing the whole payment.
		let give_view = match give.fungible.first_key_value() {
			Some((AssetId(location), accounting))
				if give.fungible.len() == 1 && give.non_fungible.is_empty() =>
				Some((location.clone(), accounting.amount()).into()),
			_ => None,
		};
		let first_is_cheaper = give_view
			.map(|view: Assets| Self::first_is_cheaper(&view, want, maximal))
			.unwrap_or(true);

		let give = if first_is_cheaper {
			match A::exchange_asset(origin, give, want, maximal) {
				Ok(got) => return Ok(got),
				Err(give) => give,
			}
		} else {
			match B::exchange_asset(origin, give, want, maximal) {
				Ok(got) => return Ok(got),
				Err(give) => give,
			}
		};
		if first_is_cheaper {
			B::exchange_asset(origin, give, want, maximal)
		} else {
			A::exchange_asset(origin, give, want, maximal)
		}
	}

	fn quote_exchange_price(give: &Assets, want: &Assets, maximal: bool) -> Option<Assets> {
		let a = A::quote_exchange_price(give, want, maximal);
		let b = B::quote_exchange_price(give, want, maximal);
		match (a, b) {
			(Some(a), Some(b)) => {
				let cheaper = match (single_fungible(&a), single_fungible(&b)) {
					(Some((_, x)), Some((_, y))) => x <= y,
					(Some(_), None) => true,
					_ => false,
				};
				Some(if cheaper { a } else { b })
			},
			(Some(a), None) => Some(a),
			(None, b) => b,
		}
	}
}

/// All ways of settling a fee the executor priced in DOT — delivery fees — in another asset:
/// whichever of the [`AssetConversion`] pool and the governance-registered rate asks the payer for
/// less.
pub type AssetExchangers = CheaperExchanger<PoolAssetsExchanger, FeesAtAssetRate>;

/// Lets fees that the executor prices in DOT — delivery fees — be settled in any asset that
/// governance registered a rate for in `pallet-asset-rate`. The fallback behind
/// [`PoolAssetsExchanger`], for assets that have a rate but no pool.
///
/// No swap happens, since there is no pool to swap against: the asset offered for fees is priced
/// against the DOT amount the executor asks for using the registered rate, and, if it covers it, is
/// handed straight back so that the `FeeManager` deposits it *in kind* into the fee receiver's
/// account. This is the same deal the [`Traders`] above offer for execution fees.
///
/// The `ExchangeAsset` instruction is not weighed on this chain (its weight is `Weight::MAX`), so
/// this is only ever reachable through fee payment in the XCM executor.
///
/// Because the fee is deposited in kind, into [`RelayTreasuryPalletAccount`] here, the asset has
/// to be one `pallet-assets` will accept into that account: see the note on [`AssetRateTrader`].
/// If it is not, the fee is burned instead of collected.
pub struct FeesAtAssetRate;

impl FeesAtAssetRate {
	/// What `amount` of `from` is worth in `to`, at the rates registered in `pallet-asset-rate`.
	///
	/// Only pairs including DOT are priced, which is all fees ever need: rates are registered
	/// against DOT.
	///
	/// A non-zero amount never converts to nothing. The amount is always a fee, and the asset's
	/// precision is unknown: integer division can round a small fee — or a rate above the fee —
	/// down to zero, which would waive it altogether. `ChargeAtAssetRate::in_asset` and
	/// `TakeFirstAssetTrader` apply the same floor to transaction and execution fees.
	fn convert(amount: Balance, from: &Location, to: &Location) -> Option<Balance> {
		let native = RelayLocation::get();
		let converted = match (from == &native, to == &native) {
			(true, true) => Some(amount),
			(true, false) => AssetRate::to_asset_balance(amount, to.clone()).ok(),
			(false, true) => AssetRate::from_asset_balance(amount, from.clone()).ok(),
			(false, false) => None,
		};
		let minimum = if amount == 0 { 0 } else { 1 };
		converted.map(|converted| converted.max(minimum))
	}
}

impl AssetExchange for FeesAtAssetRate {
	fn exchange_asset(
		_origin: Option<&Location>,
		give: AssetsInHolding,
		want: &Assets,
		_maximal: bool,
	) -> Result<AssetsInHolding, AssetsInHolding> {
		// Only the assets set aside for fee payment, a single fungible, are ever offered here.
		let given = match give.fungible.iter().next() {
			Some((AssetId(location), accounting))
				if give.fungible.len() == 1 && give.non_fungible.is_empty() =>
				Some((location.clone(), accounting.amount())),
			_ => None,
		};
		let (Some((given_asset, given_amount)), Some((wanted_asset, wanted_amount))) =
			(given, single_fungible(want))
		else {
			return Err(give);
		};

		match Self::convert(wanted_amount, &wanted_asset, &given_asset) {
			// What is offered is worth what was asked for, so it settles the fee as it is.
			Some(required) if required <= given_amount => Ok(give),
			_ => Err(give),
		}
	}

	fn quote_exchange_price(give: &Assets, want: &Assets, maximal: bool) -> Option<Assets> {
		let (given_asset, given_amount) = single_fungible(give)?;
		let (wanted_asset, wanted_amount) = single_fungible(want)?;
		if maximal {
			// How much of `want`'s asset is `give` worth?
			let obtained = Self::convert(given_amount, &given_asset, &wanted_asset)?;
			Some((wanted_asset, obtained).into())
		} else {
			// How much of `give`'s asset does it take to cover `want`?
			let required = Self::convert(wanted_amount, &wanted_asset, &given_asset)?;
			Some((given_asset, required).into())
		}
	}
}

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	type AssetTransactor = AssetTransactors;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type IsReserve = TrustedReserves;
	/// Only allow teleportation of DOT.
	type IsTeleporter = ConcreteAssetFromSystem<RelayLocation>;
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = WeightInfoBounds<
		crate::weights::xcm::PeoplePolkadotXcmWeight<RuntimeCall>,
		RuntimeCall,
		MaxInstructions,
	>;
	type Trader = Traders;
	type ResponseHandler = PolkadotXcm;
	type AssetTrap = PolkadotXcm;
	type SubscriptionService = PolkadotXcm;
	type PalletInstancesInfo = AllPalletsWithSystem;
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	type AssetLocker = ();
	// Delivery fees are priced in DOT but can be settled in any asset with a pool, or failing
	// that a registered rate.
	type AssetExchanger = AssetExchangers;
	type FeeManager = XcmFeeManagerFromComponents<
		WaivedLocations,
		SendXcmFeeToAccount<AssetTransactors, RelayTreasuryPalletAccount>,
	>;
	type MessageExporter = ();
	type UniversalAliases = Nothing;
	type CallDispatcher = RuntimeCall;
	type SafeCallFilter = Everything;
	type Aliasers = TrustedAliasers;
	type TransactionalProcessor = FrameTransactionalProcessor;
	type HrmpNewChannelOpenRequestHandler = ();
	type HrmpChannelAcceptedHandler = ();
	type HrmpChannelClosingHandler = ();
	type XcmRecorder = PolkadotXcm;
	type XcmEventEmitter = PolkadotXcm;
}

/// Converts a local signed origin into an XCM `Location`. Forms the basis for local origins
/// sending/executing XCMs.
pub type LocalSignedOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = WithUniqueTopic<(
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm, PriceForParentDelivery>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
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
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalSignedOriginToLocation>;
	type XcmRouter = XcmRouter;
	// Any local signed origin can execute XCM messages.
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalSignedOriginToLocation>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Everything;
	type XcmReserveTransferFilter = Nothing; // This parachain is not meant as a reserve location.
	type Weigher = WeightInfoBounds<
		crate::weights::xcm::PeoplePolkadotXcmWeight<RuntimeCall>,
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
	// xcm_executor::Config::Aliasers includes pallet_xcm::AuthorizedAliasers.
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
impl pallet_assets::BenchmarkHelper<Location, ()> for XcmBenchmarkHelper {
	fn create_asset_id_parameter(id: u32) -> Location {
		Location::new(1, Parachain(id))
	}
	fn create_reserve_id_parameter(_id: u32) {}
}

#[test]
fn treasury_pallet_account_not_none() {
	assert_eq!(
		RelayTreasuryPalletAccount::get(),
		LocationToAccountId::convert_location(&RelayTreasuryLocation::get()).unwrap()
	)
}
