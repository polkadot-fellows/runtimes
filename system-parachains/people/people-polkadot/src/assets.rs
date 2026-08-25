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

use super::*;

use assets_common::local_and_foreign_assets::TargetFromLeft;
use core::marker::PhantomData;
use frame_support::{
	parameter_types,
	traits::{
		fungible, fungibles,
		tokens::{
			imbalance::ResolveAssetTo, ConversionToAssetBalance, Fortitude, Precision,
			Preservation, WithdrawConsequence,
		},
		AsEnsureOriginWithArg, ConstU128, OnUnbalanced,
	},
};
use frame_system::{EnsureNever, EnsureRoot};
use pallet_asset_conversion_tx_payment::OnChargeAssetTransaction;
use sp_runtime::{
	traits::{DispatchInfoOf, PostDispatchInfoOf, Zero},
	transaction_validity::{InvalidTransaction, TransactionValidityError},
	Either,
};
use xcm::latest::{Asset, AssetId, Junction::*, Location};
use xcm_config::{RelayLocation, StakingPot};

parameter_types! {
	pub const AssetDeposit: Balance = UNITS;
	pub const AssetAccountDeposit: Balance = system_para_deposit(1, 16);
	pub const ApprovalDeposit: Balance = SYSTEM_PARA_EXISTENTIAL_DEPOSIT;
	pub const AssetsStringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = system_para_deposit(1, 68);
	pub const MetadataDepositPerByte: Balance = system_para_deposit(0, 1);
}

impl pallet_assets::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = Location;
	type AssetIdParameter = Location;
	type Currency = Balances;
	// Assets can only be force created by root.
	type CreateOrigin = EnsureNever<AccountId>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = AssetsStringLimit;
	type Holder = AssetsHolder;
	type Freezer = ();
	type Extra = ();
	type WeightInfo = weights::pallet_assets::WeightInfo<Runtime>;
	type CallbackHandle = ();
	type AssetAccountDeposit = AssetAccountDeposit;
	type ReserveData = ();
	type RemoveItemsLimit = frame_support::traits::ConstU32<1000>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = xcm_config::XcmBenchmarkHelper;
}

/// The liquidity pool tokens of [`AssetConversion`].
///
/// They are minted and burned by the asset conversion pallet itself, so nothing else may create
/// them.
pub type PoolAssetsInstance = pallet_assets::Instance1;
impl pallet_assets::Config<PoolAssetsInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = u32;
	type AssetIdParameter = u32;
	type Currency = Balances;
	#[cfg(feature = "runtime-benchmarks")]
	type CreateOrigin =
		AsEnsureOriginWithArg<frame_system::EnsureSignedBy<AssetConversionOrigin, AccountId>>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type CreateOrigin = AsEnsureOriginWithArg<frame_support::traits::NeverEnsureOrigin<AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	// Opening the pool is paid for by `PoolSetupFee`, so its liquidity token costs no deposit.
	type AssetDeposit = ConstU128<0>;
	type AssetAccountDeposit = ConstU128<0>;
	type MetadataDepositBase = ConstU128<0>;
	type MetadataDepositPerByte = ConstU128<0>;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = AssetsStringLimit;
	type Holder = ();
	type Freezer = ();
	type Extra = ();
	type WeightInfo = weights::pallet_assets_pool::WeightInfo<Runtime>;
	type CallbackHandle = ();
	type ReserveData = ();
	type RemoveItemsLimit = frame_support::traits::ConstU32<1000>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

/// The fungibles registry [`AssetConversion`] trades over: DOT, plus everything held in the
/// `Assets` pallet, all keyed by XCM `Location`.
pub type NativeAndAssets = fungible::UnionOf<
	Balances,
	Assets,
	TargetFromLeft<RelayLocation, Location>,
	Location,
	AccountId,
>;

parameter_types! {
	pub const AssetConversionPalletId: PalletId = PalletId(*b"py/ascon");
	pub const LiquidityWithdrawalFee: Permill = Permill::from_percent(0);
	/// The share of every swap that is left in the pool for its liquidity providers.
	pub LpFee: Permill = Permill::from_rational(3u32, 1_000u32); // 0.3%
	/// Storage deposit for the pool entry and for its liquidity token, plus what registering an
	/// asset costs, so that opening a pool is never cheaper than creating the asset it pairs.
	pub const PoolSetupFee: Balance = system_para_deposit(1, 4) + AssetDeposit::get();
}

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	/// Index of the `Assets` pallet, used to build the asset locations of benchmark pools.
	pub AssetsPalletIndex: u32 =
		<Assets as frame_support::traits::PalletInfoAccess>::index() as u32;
}

#[cfg(feature = "runtime-benchmarks")]
frame_support::ord_parameter_types! {
	pub const AssetConversionOrigin: AccountId =
		sp_runtime::traits::AccountIdConversion::<AccountId>::into_account_truncating(
			&AssetConversionPalletId::get(),
		);
}

pub type PoolIdToAccountId =
	pallet_asset_conversion::AccountIdConverter<AssetConversionPalletId, (Location, Location)>;

/// Liquidity pools pricing every registered asset against DOT.
///
/// Anyone can open a pool for an asset the `Assets` pallet knows about and anyone can arbitrage its
/// price, which is what makes paying fees in that asset permissionless: the pool, not governance,
/// says what the asset is worth.
impl pallet_asset_conversion::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type HigherPrecisionBalance = sp_core::U256;
	type AssetKind = Location;
	type Assets = NativeAndAssets;
	type PoolId = (Self::AssetKind, Self::AssetKind);
	// Every pool is paired with DOT, which is what the fee conversion needs and what keeps a swap
	// against the fee asset down to a single hop.
	type PoolLocator = pallet_asset_conversion::WithFirstAsset<
		RelayLocation,
		AccountId,
		Self::AssetKind,
		PoolIdToAccountId,
	>;
	type PoolAssetId = u32;
	type PoolAssets = PoolAssets;
	type PoolSetupFee = PoolSetupFee;
	type PoolSetupFeeAsset = RelayLocation;
	type PoolSetupFeeTarget = ResolveAssetTo<StakingPot, Self::Assets>;
	type LiquidityWithdrawalFee = LiquidityWithdrawalFee;
	type LPFee = LpFee;
	type PalletId = AssetConversionPalletId;
	// Every pool holds DOT on one side (see `PoolLocator`), so swapping one asset for another takes
	// two hops through DOT. Fee payment itself is always a single hop.
	type MaxSwapPathLength = ConstU32<3>;
	type MintMinLiquidity = ConstU128<100>;
	type WeightInfo = weights::pallet_asset_conversion::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = assets_common::benchmarks::AssetPairFactory<
		RelayLocation,
		parachain_info::Pallet<Runtime>,
		AssetsPalletIndex,
		Self::AssetKind,
	>;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct AssetConversionTxHelper;

#[cfg(feature = "runtime-benchmarks")]
impl pallet_asset_conversion_tx_payment::BenchmarkHelperTrait<AccountId, Location, Location>
	for AssetConversionTxHelper
{
	fn create_asset_id_parameter(seed: u32) -> (Location, Location) {
		// Any sibling parachain's asset will do: it only has to be foreign to this chain.
		let asset_id =
			Location::new(1, [Parachain(3000), PalletInstance(53), GeneralIndex(seed.into())]);
		(asset_id.clone(), asset_id)
	}

	fn setup_balances_and_pool(asset_id: Location, account: AccountId) {
		use alloc::boxed::Box;
		use frame_support::{
			assert_ok,
			traits::{
				fungible::Mutate as _,
				fungibles::{Inspect as _, Mutate as _},
			},
		};

		if !Assets::asset_exists(asset_id.clone()) {
			assert_ok!(Assets::force_create(
				RuntimeOrigin::root(),
				asset_id.clone(),
				account.clone().into(), // owner
				true,                   // is_sufficient
				1,
			));
		}

		let lp_provider = account;
		assert_ok!(Balances::mint_into(&lp_provider, u64::MAX.into()));
		assert_ok!(Assets::mint_into(asset_id.clone(), &lp_provider, u64::MAX.into()));

		let token_native = Box::new(RelayLocation::get());
		let token_second = Box::new(asset_id);

		assert_ok!(AssetConversion::create_pool(
			RuntimeOrigin::signed(lp_provider.clone()),
			token_native.clone(),
			token_second.clone()
		));

		// An eighth of what was minted on each side: comfortably above this chain's existential
		// deposit, which the pool's own account has to keep, while leaving the provider the rest
		// of the asset to pay the benchmarked fee with.
		let liquidity: Balance = (u64::MAX / 8).into();
		assert_ok!(AssetConversion::add_liquidity(
			RuntimeOrigin::signed(lp_provider.clone()),
			token_native,
			token_second,
			liquidity, // 1 desired
			liquidity, // 2 desired
			1,         // 1 min
			1,         // 2 min
			lp_provider,
		));
	}
}

/// Charges the transaction fee in kind, at the rate governance registered for the asset in
/// [`AssetRate`].
///
/// Fees are priced in DOT like everything else on this chain; the registered rate turns that into
/// an amount of the asset, which is withdrawn from the payer and resolved to the staking pot as
/// it is. Nothing is swapped, because this is the path for assets that have no pool.
///
/// This is what `pallet-asset-tx-payment` did on this chain before pools, kept as the fallback
/// behind [`ChargeThroughPool`] so an asset with a rate but no pool keeps paying for transactions.
pub struct ChargeAtAssetRate;

impl ChargeAtAssetRate {
	/// What `native` DOT of fee costs in `asset`, at the registered rate.
	fn in_asset(native: Balance, asset: &Location) -> Result<Balance, TransactionValidityError> {
		// The asset's precision is unknown, and integer division can round a small fee down to
		// zero, so a non-zero fee always costs at least one unit of the asset.
		let minimum = if native.is_zero() { 0 } else { 1 };
		AssetRate::to_asset_balance(native, asset.clone())
			.map(|converted| converted.max(minimum))
			.map_err(|_| InvalidTransaction::Payment.into())
	}

	fn ensure_can_withdraw(
		who: &AccountId,
		asset: Location,
		amount: Balance,
	) -> Result<(), TransactionValidityError> {
		match <Assets as fungibles::Inspect<AccountId>>::can_withdraw(asset, who, amount) {
			WithdrawConsequence::Success => Ok(()),
			_ => Err(InvalidTransaction::Payment.into()),
		}
	}
}

impl OnChargeAssetTransaction<Runtime> for ChargeAtAssetRate {
	type Balance = Balance;
	type AssetId = Location;
	type LiquidityInfo = fungibles::Credit<AccountId, Assets>;

	fn withdraw_fee(
		who: &AccountId,
		_call: &RuntimeCall,
		_dispatch_info: &DispatchInfoOf<RuntimeCall>,
		asset_id: Location,
		fee: Balance,
		_tip: Balance,
	) -> Result<Self::LiquidityInfo, TransactionValidityError> {
		let converted_fee = Self::in_asset(fee, &asset_id)?;
		<Assets as fungibles::Balanced<AccountId>>::withdraw(
			asset_id,
			who,
			converted_fee,
			Precision::Exact,
			Preservation::Protect,
			Fortitude::Polite,
		)
		.map_err(|_| InvalidTransaction::Payment.into())
	}

	fn can_withdraw_fee(
		who: &AccountId,
		asset_id: Location,
		fee: Balance,
	) -> Result<(), TransactionValidityError> {
		let converted_fee = Self::in_asset(fee, &asset_id)?;
		Self::ensure_can_withdraw(who, asset_id, converted_fee)
	}

	fn correct_and_deposit_fee(
		who: &AccountId,
		_dispatch_info: &DispatchInfoOf<RuntimeCall>,
		_post_info: &PostDispatchInfoOf<RuntimeCall>,
		corrected_fee: Balance,
		_tip: Balance,
		asset_id: Location,
		paid: Self::LiquidityInfo,
	) -> Result<Balance, TransactionValidityError> {
		let corrected = Self::in_asset(corrected_fee, &asset_id)?;
		let (final_fee, refund) = paid.split(corrected);

		// Give back what the call did not end up costing. If that cannot be resolved — the call
		// may have reaped the payer's account — the refund stays with the fee rather than being
		// burned.
		let final_fee = match <Assets as fungibles::Balanced<AccountId>>::resolve(who, refund) {
			Ok(()) => final_fee,
			Err(refund) => final_fee.merge(refund).unwrap_or_else(|(final_fee, _)| final_fee),
		};

		let charged = final_fee.peek();
		// The tip is not split out: it goes to the staking pot along with the fee, which is where
		// the pool path sends both too.
		ResolveAssetTo::<StakingPot, Assets>::on_unbalanced(final_fee);
		Ok(charged)
	}
}

/// Transaction fees are priced in DOT and paid by swapping just enough of the offered asset for
/// that DOT through an [`AssetConversion`] pool.
pub type ChargeThroughPool = pallet_asset_conversion_tx_payment::SwapAssetAdapter<
	RelayLocation,
	NativeAndAssets,
	AssetConversion,
	ResolveAssetTo<StakingPot, NativeAndAssets>,
>;

/// Quotes what a fee priced in DOT costs in `asset`, without touching any balance.
///
/// This is what [`ChargeCheaper`] compares the two oracles with. Each implementation must agree
/// with the price its `OnChargeAssetTransaction` would actually charge, or the cheaper half would
/// be picked and then charge something else.
pub trait QuoteAssetFee {
	/// `None` when this half cannot price `asset` at all.
	fn quote_fee(asset: &Location, native_fee: Balance) -> Option<Balance>;
}

impl QuoteAssetFee for ChargeAtAssetRate {
	fn quote_fee(asset: &Location, native_fee: Balance) -> Option<Balance> {
		Self::in_asset(native_fee, asset).ok()
	}
}

impl QuoteAssetFee for ChargeThroughPool {
	fn quote_fee(asset: &Location, native_fee: Balance) -> Option<Balance> {
		// Mirrors `SwapAssetAdapter::can_withdraw_fee`: the target asset needs no swap, anything
		// else is quoted against it through the pool, fees included, and a zero quote is refused.
		if asset == &RelayLocation::get() {
			return Some(native_fee);
		}
		<AssetConversion as pallet_asset_conversion::QuotePrice>::quote_price_tokens_for_exact_tokens(
			asset.clone(),
			RelayLocation::get(),
			native_fee,
			true,
		)
		.filter(|quoted| !quoted.is_zero())
	}
}

/// Charges the transaction fee through whichever of `A` and `B` asks the payer for less.
///
/// Both halves price the same fee, in the same asset, so their quotes are directly comparable. A
/// half that cannot price the asset, or that the payer cannot actually pay through, is skipped —
/// so a pool too thin to be worth using never blocks the registered rate, and vice versa. On a
/// tie `A` wins.
///
/// The order is decided from read-only quotes alone; the halves are then simply tried in that
/// order, since either fails without having moved anything. `LiquidityInfo` records the half that
/// charged so the refund in `correct_and_deposit_fee` goes back the same way.
pub struct ChargeCheaper<A, B>(PhantomData<(A, B)>);

impl<A, B> ChargeCheaper<A, B>
where
	A: OnChargeAssetTransaction<Runtime, Balance = Balance, AssetId = Location> + QuoteAssetFee,
	B: OnChargeAssetTransaction<Runtime, Balance = Balance, AssetId = Location> + QuoteAssetFee,
{
	/// Runs `a` and `b` cheapest first, settling for the first that succeeds.
	///
	/// Fails if neither half can price the asset, or neither can settle the fee.
	fn cheapest_first<T>(
		asset_id: &Location,
		fee: Balance,
		a: impl FnOnce() -> Result<T, TransactionValidityError>,
		b: impl FnOnce() -> Result<T, TransactionValidityError>,
	) -> Result<T, TransactionValidityError> {
		let a_first = match (A::quote_fee(asset_id, fee), B::quote_fee(asset_id, fee)) {
			(Some(a), Some(b)) => a <= b,
			(Some(_), None) => true,
			(None, Some(_)) => false,
			(None, None) => return Err(InvalidTransaction::Payment.into()),
		};
		if a_first {
			a().or_else(|_| b())
		} else {
			b().or_else(|_| a())
		}
	}
}

impl<A, B> OnChargeAssetTransaction<Runtime> for ChargeCheaper<A, B>
where
	A: OnChargeAssetTransaction<Runtime, Balance = Balance, AssetId = Location> + QuoteAssetFee,
	B: OnChargeAssetTransaction<Runtime, Balance = Balance, AssetId = Location> + QuoteAssetFee,
{
	type Balance = Balance;
	type AssetId = Location;
	type LiquidityInfo = Either<A::LiquidityInfo, B::LiquidityInfo>;

	fn withdraw_fee(
		who: &AccountId,
		call: &RuntimeCall,
		dispatch_info: &DispatchInfoOf<RuntimeCall>,
		asset_id: Location,
		fee: Balance,
		tip: Balance,
	) -> Result<Self::LiquidityInfo, TransactionValidityError> {
		Self::cheapest_first(
			&asset_id,
			fee,
			|| {
				A::withdraw_fee(who, call, dispatch_info, asset_id.clone(), fee, tip)
					.map(Either::Left)
			},
			|| {
				B::withdraw_fee(who, call, dispatch_info, asset_id.clone(), fee, tip)
					.map(Either::Right)
			},
		)
	}

	fn can_withdraw_fee(
		who: &AccountId,
		asset_id: Location,
		fee: Balance,
	) -> Result<(), TransactionValidityError> {
		Self::cheapest_first(
			&asset_id,
			fee,
			|| A::can_withdraw_fee(who, asset_id.clone(), fee),
			|| B::can_withdraw_fee(who, asset_id.clone(), fee),
		)
	}

	fn correct_and_deposit_fee(
		who: &AccountId,
		dispatch_info: &DispatchInfoOf<RuntimeCall>,
		post_info: &PostDispatchInfoOf<RuntimeCall>,
		corrected_fee: Balance,
		tip: Balance,
		asset_id: Location,
		already_withdrawn: Self::LiquidityInfo,
	) -> Result<Balance, TransactionValidityError> {
		match already_withdrawn {
			Either::Left(paid) => A::correct_and_deposit_fee(
				who,
				dispatch_info,
				post_info,
				corrected_fee,
				tip,
				asset_id,
				paid,
			),
			Either::Right(paid) => B::correct_and_deposit_fee(
				who,
				dispatch_info,
				post_info,
				corrected_fee,
				tip,
				asset_id,
				paid,
			),
		}
	}
}

/// How a transaction fee named in an asset is settled: through whichever of the
/// [`AssetConversion`] pool and the governance-registered rate asks the payer for less. This is
/// the same rule the XCM [`Traders`](xcm_config::Traders) and
/// [`AssetExchangers`](xcm_config::AssetExchangers) apply.
///
/// Pools make an asset usable permissionlessly; the rate keeps an asset usable when its pool is
/// missing or too thin to price a fee sanely.
pub type ChargeTransactionFee = ChargeCheaper<ChargeThroughPool, ChargeAtAssetRate>;

impl pallet_asset_conversion_tx_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AssetId = Location;
	type OnChargeAssetTransaction = ChargeTransactionFee;
	type WeightInfo = weights::pallet_asset_conversion_tx_payment::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = AssetConversionTxHelper;
}

/// Governance-set exchange rates against DOT.
///
/// Pools are the primary way to price an asset against DOT. Rates registered here are the fallback
/// every fee path reaches for when an asset has no pool: [`ChargeAtAssetRate`] for transaction
/// fees, [`xcm_config::WeightToAssetRateFee`] for XCM execution fees and
/// [`xcm_config::FeesAtAssetRate`] for XCM delivery fees.
impl pallet_asset_rate::Config for Runtime {
	type WeightInfo = weights::pallet_asset_rate::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type CreateOrigin = EnsureRoot<AccountId>;
	type RemoveOrigin = EnsureRoot<AccountId>;
	type UpdateOrigin = EnsureRoot<AccountId>;
	type Currency = Balances;
	type AssetKind = <Runtime as pallet_assets::Config>::AssetId;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = AssetRateBenchmarkHelper;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct AssetRateBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl pallet_asset_rate::AssetKindFactory<Location> for AssetRateBenchmarkHelper {
	fn create_asset_kind(seed: u32) -> Location {
		Location::new(
			1,
			[
				xcm::latest::Junction::Parachain(1000),
				xcm::latest::Junction::GeneralIndex(seed as u128),
			],
		)
	}
}

impl pallet_assets_holder::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
}

/// Module that holds everything related to the HOLLAR asset.
pub mod hollar {
	use super::*;
	use frame_support::traits::ContainsPair;

	/// The parachain id of the Hydration DEX.
	pub const HYDRATION_PARA_ID: u32 = 2034;

	/// The asset id of HOLLAR.
	pub const HOLLAR_ASSET_ID: u128 = 222;

	/// A unit of HOLLAR consists of 10^18 plancks.
	pub const HOLLAR_UNITS: u128 = 1_000_000_000_000_000_000u128;

	parameter_types! {
		pub HydrationLocation: Location = Location::new(1, [Parachain(HYDRATION_PARA_ID)]);
		pub HollarLocation: Location = Location::new(1, [Parachain(HYDRATION_PARA_ID), GeneralIndex(HOLLAR_ASSET_ID)]);
		pub HollarId: AssetId = AssetId(HollarLocation::get());
		pub Hollar: Asset = (HollarId::get(), 10 * HOLLAR_UNITS).into();
	}

	/// A type that matches the pair `(Hollar, Hydration)`,
	/// used in the XCM configuration's `IsReserve`.
	pub struct HollarFromHydration;
	impl ContainsPair<Asset, Location> for HollarFromHydration {
		fn contains(asset: &Asset, origin: &Location) -> bool {
			let is_hydration = matches!(origin.unpack(), (1, [Parachain(para_id)]) if *para_id == HYDRATION_PARA_ID);
			let is_hollar = matches!(
				asset.id.0.unpack(),
				(1, [Parachain(para_id), GeneralIndex(asset_id)])
				if *para_id == HYDRATION_PARA_ID && *asset_id == HOLLAR_ASSET_ID
			);

			is_hydration && is_hollar
		}
	}
}
