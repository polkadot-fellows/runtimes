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

//! # Asset Hub Kusama Runtime
//!
//! Asset Hub Kusama, formerly known as "Statemine", is the canary network for its Polkadot cousin.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

extern crate alloc;

// Genesis preset configurations.
pub mod genesis_config_presets;
pub mod governance;
mod impls;
mod weights;
pub mod xcm_config;

use alloc::{borrow::Cow, vec, vec::Vec};
use assets_common::{
	foreign_creators::ForeignCreators,
	local_and_foreign_assets::{LocalFromLeft, TargetFromLeft},
	matching::FromSiblingParachain,
	AssetIdForTrustBackedAssetsConvert,
};
use cumulus_pallet_parachain_system::RelayNumberMonotonicallyIncreases;
use cumulus_primitives_core::{AggregateMessageOrigin, ParaId};
use governance::{
	pallet_custom_origins
};
use kusama_runtime_constants::time::MINUTES as RC_MINUTES;
use pallet_proxy::ProxyDefinition;
use pallet_revive::{evm::runtime::EthExtra, AddressMapper};
use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata, H160, U256};
use sp_runtime::{
	generic, impl_opaque_keys,
	traits::{
		AccountIdConversion, AccountIdLookup, BlakeTwo256, Block as BlockT, Convert, ConvertInto,
		Verify,
	},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, Perbill, Permill,
};

#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::{
	construct_runtime,
	dispatch::{DispatchClass, DispatchInfo},
	genesis_builder_helper::{build_state, get_preset},
	ord_parameter_types, parameter_types,
	traits::{
		fungible, fungibles, tokens::imbalance::ResolveAssetTo, AsEnsureOriginWithArg, ConstBool,
		ConstU128, ConstU32, ConstU64, ConstU8, Contains, EitherOfDiverse, Equals, EverythingBut,
		InstanceFilter, Nothing, TransformOrigin, WithdrawReasons,
	},
	weights::{ConstantMultiplier, Weight, WeightToFee as _},
	BoundedVec, PalletId,
};
use frame_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot, EnsureSigned, EnsureSignedBy,
};
use pallet_nfts::PalletFeatures;
use parachains_common::{
	message_queue::*, AccountId, AssetIdForTrustBackedAssets, AuraId, Balance, BlockNumber, Hash,
	Header, Nonce, Signature,
};
use sp_runtime::{traits::TransactionExtension, RuntimeDebug};
pub use system_parachains_constants::async_backing::SLOT_DURATION;
use system_parachains_constants::{
	async_backing::{
		AVERAGE_ON_INITIALIZE_RATIO, DAYS, HOURS, MAXIMUM_BLOCK_WEIGHT, NORMAL_DISPATCH_RATIO,
	},
	kusama::{
		consensus::{
			async_backing::UNINCLUDED_SEGMENT_CAPACITY, BLOCK_PROCESSING_VELOCITY,
			RELAY_CHAIN_SLOT_DURATION_MILLIS,
		},
		currency::*,
		fee::WeightToFee,
	},
};
use xcm::{
	latest::prelude::{AssetId, BodyId},
	Version as XcmVersion, VersionedAssetId, VersionedAssets, VersionedLocation, VersionedXcm,
};
use xcm_config::{
	FellowshipLocation, ForeignAssetsConvertedConcreteId, ForeignCreatorsSovereignAccountOf,
	GovernanceLocation, KsmLocation, KsmLocationV4, PoolAssetsConvertedConcreteId, StakingPot,
	TrustBackedAssetsConvertedConcreteId, TrustBackedAssetsPalletLocationV4,
};
use xcm_runtime_apis::{
	dry_run::{CallDryRunEffects, Error as XcmDryRunApiError, XcmDryRunEffects},
	fees::Error as XcmPaymentApiError,
};

#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

// Polkadot imports
use pallet_xcm::{EnsureXcm, IsVoiceOfBody};
use polkadot_runtime_common::{BlockHashCount, SlowAdjustingFeeUpdate};

use weights::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight};

impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: Aura,
	}
}

#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	// Note: "statemine" is the legacy name for this chain. It has been renamed to
	// "asset-hub-kusama". Many wallets/tools depend on the `spec_name`, so it remains "statemine"
	// for the time being. Wallets/tools should update to treat "asset-hub-kusama" equally.
	spec_name: Cow::Borrowed("statemine"),
	impl_name: Cow::Borrowed("statemine"),
	authoring_version: 1,
	spec_version: 1_006_000,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 15,
	system_version: 1,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
	pub const SS58Prefix: u8 = 2;
}

pub struct VestedTransferCalls;
impl Contains<RuntimeCall> for VestedTransferCalls {
	fn contains(call: &RuntimeCall) -> bool {
		matches!(call, RuntimeCall::Vesting(pallet_vesting::Call::vested_transfer { .. }))
	}
}

// Configure FRAME pallets to include in runtime.
impl frame_system::Config for Runtime {
	type BaseCallFilter = EverythingBut<VestedTransferCalls>;
	type BlockWeights = RuntimeBlockWeights;
	type BlockLength = RuntimeBlockLength;
	type AccountId = AccountId;
	type RuntimeCall = RuntimeCall;
	type Lookup = AccountIdLookup<AccountId, ()>;
	type Nonce = Nonce;
	type Hash = Hash;
	type Hashing = BlakeTwo256;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeTask = RuntimeTask;
	type RuntimeOrigin = RuntimeOrigin;
	type BlockHashCount = BlockHashCount;
	type DbWeight = RocksDbWeight;
	type Version = Version;
	type PalletInfo = PalletInfo;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type AccountData = pallet_balances::AccountData<Balance>;
	type SystemWeightInfo = weights::frame_system::WeightInfo<Runtime>;
	type ExtensionsWeightInfo = weights::frame_system_extensions::WeightInfo<Runtime>;
	type SS58Prefix = SS58Prefix;
	type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
	type MaxConsumers = ConstU32<256>;
	type SingleBlockMigrations = ();
	type MultiBlockMigrator = ();
	type PreInherents = ();
	type PostInherents = ();
	type PostTransactions = ();
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = ConstU64<0>;
	type WeightInfo = weights::pallet_timestamp::WeightInfo<Runtime>;
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
	type EventHandler = (CollatorSelection,);
}

parameter_types! {
	// This comes from system_parachains_constants::kusama::currency and is the ED for all system
	// parachains. For Asset Hub in particular, we set it to 1/10th of the amount.
	pub const ExistentialDeposit: Balance = SYSTEM_PARA_EXISTENTIAL_DEPOSIT / 10;
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = ConstU32<50>;
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = weights::pallet_balances::WeightInfo<Runtime>;
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = ();
	type MaxFreezes = ConstU32<0>;
	type DoneSlashHandler = ();
}

parameter_types! {
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

impl pallet_vesting::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BlockNumberToBalance = ConvertInto;
	type MinVestedTransfer = ExistentialDeposit;
	type WeightInfo = weights::pallet_vesting::WeightInfo<Runtime>;
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	/// Note for wallets and implementers: This means that vesting schedules are evaluated with the
	/// block number of the Relay Chain, not the parachain. This is because with Coretime and Async
	/// Backing, parachain block numbers may not be a good proxy for time. Vesting schedules should
	/// be set accordingly.
	type BlockNumberProvider = cumulus_pallet_parachain_system::RelaychainDataProvider<Runtime>;
	const MAX_VESTING_SCHEDULES: u32 = 28;
}

parameter_types! {
	/// Relay Chain `TransactionByteFee` / 10
	pub const TransactionByteFee: Balance = system_parachains_constants::kusama::fee::TRANSACTION_BYTE_FEE;
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = impls::tx_payment::FungiblesAdapter<
		NativeAndAssets,
		KsmLocationV4,
		ResolveAssetTo<StakingPot, NativeAndAssets>,
	>;
	type WeightToFee = WeightToFee;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightInfo = weights::pallet_transaction_payment::WeightInfo<Self>;
}

parameter_types! {
	pub const AssetDeposit: Balance = system_para_deposit(1, 190);
	pub const AssetAccountDeposit: Balance = system_para_deposit(1, 16);
	pub const AssetsStringLimit: u32 = 50;
	/// Key = 32 bytes, Value = 36 bytes (32+1+1+1+1)
	// https://github.com/paritytech/substrate/blob/069917b/frame/assets/src/lib.rs#L257L271
	pub const MetadataDepositBase: Balance = system_para_deposit(1, 68);
	pub const MetadataDepositPerByte: Balance = system_para_deposit(0, 1);
}

/// We allow root to execute privileged asset operations.
pub type AssetsForceOrigin = EnsureRoot<AccountId>;

// Called "Trust Backed" assets because these are generally registered by some account, and users of
// the asset assume it has some claimed backing. The pallet is called `Assets` in
// `construct_runtime` to avoid breaking changes on storage reads.
pub type TrustBackedAssetsInstance = pallet_assets::Instance1;
type TrustBackedAssetsCall = pallet_assets::Call<Runtime, TrustBackedAssetsInstance>;
impl pallet_assets::Config<TrustBackedAssetsInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = AssetIdForTrustBackedAssets;
	type AssetIdParameter = codec::Compact<AssetIdForTrustBackedAssets>;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type ForceOrigin = AssetsForceOrigin;
	type AssetDeposit = AssetDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ExistentialDeposit;
	type StringLimit = AssetsStringLimit;
	type Freezer = ();
	type Holder = ();
	type Extra = ();
	type WeightInfo = weights::pallet_assets_local::WeightInfo<Runtime>;
	type CallbackHandle = pallet_assets::AutoIncAssetId<Runtime, TrustBackedAssetsInstance>;
	type AssetAccountDeposit = AssetAccountDeposit;
	type RemoveItemsLimit = frame_support::traits::ConstU32<1000>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

parameter_types! {
	pub const AssetConversionPalletId: PalletId = PalletId(*b"py/ascon");
	pub const LiquidityWithdrawalFee: Permill = Permill::from_percent(0);
	// Storage deposit for pool setup within asset conversion pallet
	// and pool's lp token creation within assets pallet.
	pub const PoolSetupFee: Balance = system_para_deposit(1, 4) + AssetDeposit::get();
}

ord_parameter_types! {
	pub const AssetConversionOrigin: sp_runtime::AccountId32 =
		AccountIdConversion::<sp_runtime::AccountId32>::into_account_truncating(&AssetConversionPalletId::get());
}

pub type PoolAssetsInstance = pallet_assets::Instance3;
impl pallet_assets::Config<PoolAssetsInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type RemoveItemsLimit = ConstU32<1000>;
	type AssetId = u32;
	type AssetIdParameter = u32;
	type Currency = Balances;
	type CreateOrigin =
		AsEnsureOriginWithArg<EnsureSignedBy<AssetConversionOrigin, sp_runtime::AccountId32>>;
	type ForceOrigin = AssetsForceOrigin;
	// Deposits are zero because creation/admin is limited to Asset Conversion pallet.
	type AssetDeposit = ConstU128<0>;
	type AssetAccountDeposit = AssetAccountDeposit;
	type MetadataDepositBase = ConstU128<0>;
	type MetadataDepositPerByte = ConstU128<0>;
	type ApprovalDeposit = ExistentialDeposit;
	type StringLimit = ConstU32<50>;
	type Freezer = ();
	type Holder = ();
	type Extra = ();
	type WeightInfo = weights::pallet_assets_pool::WeightInfo<Runtime>;
	type CallbackHandle = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

/// Union fungibles implementation for `Assets` and `ForeignAssets`.
pub type LocalAndForeignAssets = fungibles::UnionOf<
	Assets,
	ForeignAssets,
	LocalFromLeft<
		AssetIdForTrustBackedAssetsConvert<TrustBackedAssetsPalletLocationV4, xcm::v4::Location>,
		AssetIdForTrustBackedAssets,
		xcm::v4::Location,
	>,
	xcm::v4::Location,
	AccountId,
>;

/// Union fungibles implementation for [`LocalAndForeignAssets`] and `Balances`.
pub type NativeAndAssets = fungible::UnionOf<
	Balances,
	LocalAndForeignAssets,
	TargetFromLeft<KsmLocationV4, xcm::v4::Location>,
	xcm::v4::Location,
	AccountId,
>;

pub type PoolIdToAccountId =
	pallet_asset_conversion::AccountIdConverterNoSeed<(xcm::v4::Location, xcm::v4::Location)>;

impl pallet_asset_conversion::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type HigherPrecisionBalance = sp_core::U256;
	type AssetKind = xcm::v4::Location;
	type Assets = NativeAndAssets;
	type PoolId = (Self::AssetKind, Self::AssetKind);
	type PoolLocator = pallet_asset_conversion::WithFirstAsset<
		KsmLocationV4,
		AccountId,
		Self::AssetKind,
		PoolIdToAccountId,
	>;
	type PoolAssetId = u32;
	type PoolAssets = PoolAssets;
	type PoolSetupFee = PoolSetupFee;
	type PoolSetupFeeAsset = KsmLocationV4;
	type PoolSetupFeeTarget = ResolveAssetTo<xcm_config::RelayTreasuryPalletAccount, Self::Assets>;
	type LiquidityWithdrawalFee = LiquidityWithdrawalFee;
	type LPFee = ConstU32<3>;
	type PalletId = AssetConversionPalletId;
	type MaxSwapPathLength = ConstU32<3>;
	type MintMinLiquidity = ConstU128<100>;
	type WeightInfo = weights::pallet_asset_conversion::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = assets_common::benchmarks::AssetPairFactory<
		KsmLocationV4,
		parachain_info::Pallet<Runtime>,
		xcm_config::TrustBackedAssetsPalletIndex,
		xcm::v4::Location,
	>;
}

parameter_types! {
	// we just reuse the same deposits
	pub const ForeignAssetsAssetDeposit: Balance = AssetDeposit::get();
	pub const ForeignAssetsAssetAccountDeposit: Balance = AssetAccountDeposit::get();
	pub const ForeignAssetsAssetsStringLimit: u32 = AssetsStringLimit::get();
	pub const ForeignAssetsMetadataDepositBase: Balance = MetadataDepositBase::get();
	pub const ForeignAssetsMetadataDepositPerByte: Balance = MetadataDepositPerByte::get();
}

/// Assets managed by some foreign location.
///
/// Note: we do not declare a `ForeignAssetsCall` type, as this type is used in proxy definitions.
/// We assume that a foreign location would not want to set an individual, local account as a proxy
/// for the issuance of their assets. This issuance should be managed by the foreign location's
/// governance.
pub type ForeignAssetsInstance = pallet_assets::Instance2;
impl pallet_assets::Config<ForeignAssetsInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = xcm::v4::Location;
	type AssetIdParameter = xcm::v4::Location;
	type Currency = Balances;
	type CreateOrigin = ForeignCreators<
		(
			FromSiblingParachain<parachain_info::Pallet<Runtime>, xcm::v4::Location>,
			xcm_config::bridging::to_polkadot::PolkadotOrEthereumAssetFromAssetHubPolkadot,
		),
		ForeignCreatorsSovereignAccountOf,
		AccountId,
		xcm::v4::Location,
	>;
	type ForceOrigin = AssetsForceOrigin;
	type AssetDeposit = ForeignAssetsAssetDeposit;
	type MetadataDepositBase = ForeignAssetsMetadataDepositBase;
	type MetadataDepositPerByte = ForeignAssetsMetadataDepositPerByte;
	type ApprovalDeposit = ExistentialDeposit;
	type StringLimit = ForeignAssetsAssetsStringLimit;
	type Freezer = ();
	type Holder = ();
	type Extra = ();
	type WeightInfo = weights::pallet_assets_foreign::WeightInfo<Runtime>;
	type CallbackHandle = ();
	type AssetAccountDeposit = ForeignAssetsAssetAccountDeposit;
	type RemoveItemsLimit = frame_support::traits::ConstU32<1000>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = xcm_config::XcmBenchmarkHelper;
}

parameter_types! {
	// One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
	pub const DepositBase: Balance = system_para_deposit(1, 88);
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = system_para_deposit(0, 32);
	pub const MaxSignatories: u32 = 100;
}

impl pallet_multisig::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = MaxSignatories;
	type WeightInfo = weights::pallet_multisig::WeightInfo<Runtime>;
	type BlockNumberProvider = System;
}

impl pallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = weights::pallet_utility::WeightInfo<Runtime>;
}

parameter_types! {
	// One storage item; key size 32, value size 8; .
	pub const ProxyDepositBase: Balance = system_para_deposit(1, 40);
	// Additional storage item size of 33 bytes.
	pub const ProxyDepositFactor: Balance = system_para_deposit(0, 33);
	pub const MaxProxies: u16 = 32;
	// One storage item; key size 32, value size 16
	pub const AnnouncementDepositBase: Balance = system_para_deposit(1, 48);
	pub const AnnouncementDepositFactor: Balance = system_para_deposit(0, 66);
	pub const MaxPending: u16 = 32;
}

/// The type used to represent the kinds of proxying allowed.
#[derive(
	Copy,
	Clone,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Encode,
	Decode,
	DecodeWithMemTracking,
	RuntimeDebug,
	MaxEncodedLen,
	scale_info::TypeInfo,
	Default,
)]
pub enum ProxyType {
	/// Fully permissioned proxy. Can execute any call on behalf of _proxied_.
	#[default]
	Any,
	/// Can execute any call that does not transfer funds or assets.
	NonTransfer,
	/// Proxy with the ability to reject time-delay proxy announcements.
	CancelProxy,
	/// Assets proxy. Can execute any call from `assets`, **including asset transfers**.
	Assets,
	/// Owner proxy. Can execute calls related to asset ownership.
	AssetOwner,
	/// Asset manager. Can execute calls related to asset management.
	AssetManager,
	/// Collator selection proxy. Can execute calls related to collator selection mechanism.
	Collator,
}

impl InstanceFilter<RuntimeCall> for ProxyType {
	fn filter(&self, c: &RuntimeCall) -> bool {
		match self {
			ProxyType::Any => true,
			ProxyType::NonTransfer => !matches!(
				c,
				RuntimeCall::Balances { .. } |
					RuntimeCall::Assets { .. } |
					RuntimeCall::NftFractionalization { .. } |
					RuntimeCall::Nfts { .. } |
					RuntimeCall::Uniques { .. } |
					// We allow calling `vest` and merging vesting schedules, but obviously not
					// vested transfers.
					RuntimeCall::Vesting(pallet_vesting::Call::vested_transfer { .. })
			),
			ProxyType::CancelProxy => matches!(
				c,
				RuntimeCall::Proxy(pallet_proxy::Call::reject_announcement { .. }) |
					RuntimeCall::Utility { .. } |
					RuntimeCall::Multisig { .. }
			),
			ProxyType::Assets => {
				matches!(
					c,
					RuntimeCall::Assets { .. } |
						RuntimeCall::Utility { .. } |
						RuntimeCall::Multisig { .. } |
						RuntimeCall::NftFractionalization { .. } |
						RuntimeCall::Nfts { .. } |
						RuntimeCall::Uniques { .. }
				)
			},
			ProxyType::AssetOwner => matches!(
				c,
				RuntimeCall::Assets(TrustBackedAssetsCall::create { .. }) |
					RuntimeCall::Assets(TrustBackedAssetsCall::start_destroy { .. }) |
					RuntimeCall::Assets(TrustBackedAssetsCall::destroy_accounts { .. }) |
					RuntimeCall::Assets(TrustBackedAssetsCall::destroy_approvals { .. }) |
					RuntimeCall::Assets(TrustBackedAssetsCall::finish_destroy { .. }) |
					RuntimeCall::Assets(TrustBackedAssetsCall::transfer_ownership { .. }) |
					RuntimeCall::Assets(TrustBackedAssetsCall::set_team { .. }) |
					RuntimeCall::Assets(TrustBackedAssetsCall::set_metadata { .. }) |
					RuntimeCall::Assets(TrustBackedAssetsCall::clear_metadata { .. }) |
					RuntimeCall::Assets(TrustBackedAssetsCall::set_min_balance { .. }) |
					RuntimeCall::Nfts(pallet_nfts::Call::create { .. }) |
					RuntimeCall::Nfts(pallet_nfts::Call::destroy { .. }) |
					RuntimeCall::Nfts(pallet_nfts::Call::redeposit { .. }) |
					RuntimeCall::Nfts(pallet_nfts::Call::transfer_ownership { .. }) |
					RuntimeCall::Nfts(pallet_nfts::Call::set_team { .. }) |
					RuntimeCall::Nfts(pallet_nfts::Call::set_collection_max_supply { .. }) |
					RuntimeCall::Nfts(pallet_nfts::Call::lock_collection { .. }) |
					RuntimeCall::Uniques(pallet_uniques::Call::create { .. }) |
					RuntimeCall::Uniques(pallet_uniques::Call::destroy { .. }) |
					RuntimeCall::Uniques(pallet_uniques::Call::transfer_ownership { .. }) |
					RuntimeCall::Uniques(pallet_uniques::Call::set_team { .. }) |
					RuntimeCall::Uniques(pallet_uniques::Call::set_metadata { .. }) |
					RuntimeCall::Uniques(pallet_uniques::Call::set_attribute { .. }) |
					RuntimeCall::Uniques(pallet_uniques::Call::set_collection_metadata { .. }) |
					RuntimeCall::Uniques(pallet_uniques::Call::clear_metadata { .. }) |
					RuntimeCall::Uniques(pallet_uniques::Call::clear_attribute { .. }) |
					RuntimeCall::Uniques(pallet_uniques::Call::clear_collection_metadata { .. }) |
					RuntimeCall::Uniques(pallet_uniques::Call::set_collection_max_supply { .. }) |
					RuntimeCall::Utility { .. } |
					RuntimeCall::Multisig { .. }
			),
			ProxyType::AssetManager => matches!(
				c,
				RuntimeCall::Assets(TrustBackedAssetsCall::mint { .. }) |
					RuntimeCall::Assets(TrustBackedAssetsCall::burn { .. }) |
					RuntimeCall::Assets(TrustBackedAssetsCall::freeze { .. }) |
					RuntimeCall::Assets(TrustBackedAssetsCall::block { .. }) |
					RuntimeCall::Assets(TrustBackedAssetsCall::thaw { .. }) |
					RuntimeCall::Assets(TrustBackedAssetsCall::freeze_asset { .. }) |
					RuntimeCall::Assets(TrustBackedAssetsCall::thaw_asset { .. }) |
					RuntimeCall::Assets(TrustBackedAssetsCall::touch_other { .. }) |
					RuntimeCall::Assets(TrustBackedAssetsCall::refund_other { .. }) |
					RuntimeCall::Nfts(pallet_nfts::Call::force_mint { .. }) |
					RuntimeCall::Nfts(pallet_nfts::Call::update_mint_settings { .. }) |
					RuntimeCall::Nfts(pallet_nfts::Call::mint_pre_signed { .. }) |
					RuntimeCall::Nfts(pallet_nfts::Call::set_attributes_pre_signed { .. }) |
					RuntimeCall::Nfts(pallet_nfts::Call::lock_item_transfer { .. }) |
					RuntimeCall::Nfts(pallet_nfts::Call::unlock_item_transfer { .. }) |
					RuntimeCall::Nfts(pallet_nfts::Call::lock_item_properties { .. }) |
					RuntimeCall::Nfts(pallet_nfts::Call::set_metadata { .. }) |
					RuntimeCall::Nfts(pallet_nfts::Call::clear_metadata { .. }) |
					RuntimeCall::Nfts(pallet_nfts::Call::set_collection_metadata { .. }) |
					RuntimeCall::Nfts(pallet_nfts::Call::clear_collection_metadata { .. }) |
					RuntimeCall::Uniques(pallet_uniques::Call::mint { .. }) |
					RuntimeCall::Uniques(pallet_uniques::Call::burn { .. }) |
					RuntimeCall::Uniques(pallet_uniques::Call::freeze { .. }) |
					RuntimeCall::Uniques(pallet_uniques::Call::thaw { .. }) |
					RuntimeCall::Uniques(pallet_uniques::Call::freeze_collection { .. }) |
					RuntimeCall::Uniques(pallet_uniques::Call::thaw_collection { .. }) |
					RuntimeCall::Utility { .. } |
					RuntimeCall::Multisig { .. }
			),
			ProxyType::Collator => matches!(
				c,
				RuntimeCall::CollatorSelection { .. } |
					RuntimeCall::Utility { .. } |
					RuntimeCall::Multisig { .. }
			),
		}
	}

	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			(ProxyType::Assets, ProxyType::AssetOwner) => true,
			(ProxyType::Assets, ProxyType::AssetManager) => true,
			(ProxyType::NonTransfer, ProxyType::Collator) => true,
			_ => false,
		}
	}
}

impl pallet_proxy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type MaxProxies = MaxProxies;
	type WeightInfo = weights::pallet_proxy::WeightInfo<Runtime>;
	type MaxPending = MaxPending;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
	type BlockNumberProvider = System;
}

parameter_types! {
	pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
	pub const RelayOrigin: AggregateMessageOrigin = AggregateMessageOrigin::Parent;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnSystemEvent = RemoteProxyRelayChain;
	type SelfParaId = parachain_info::Pallet<Runtime>;
	type DmpQueue = frame_support::traits::EnqueueWithOrigin<MessageQueue, RelayOrigin>;
	type ReservedDmpWeight = ReservedDmpWeight;
	type OutboundXcmpMessageSource = XcmpQueue;
	type XcmpMessageHandler = XcmpQueue;
	type ReservedXcmpWeight = ReservedXcmpWeight;
	type CheckAssociatedRelayNumber = RelayNumberMonotonicallyIncreases;
	type ConsensusHook = ConsensusHook;
	type WeightInfo = weights::cumulus_pallet_parachain_system::WeightInfo<Runtime>;
	type SelectCore = cumulus_pallet_parachain_system::DefaultCoreSelector<Runtime>;
}

type ConsensusHook = cumulus_pallet_aura_ext::FixedVelocityConsensusHook<
	Runtime,
	RELAY_CHAIN_SLOT_DURATION_MILLIS,
	BLOCK_PROCESSING_VELOCITY,
	UNINCLUDED_SEGMENT_CAPACITY,
>;

impl parachain_info::Config for Runtime {}

parameter_types! {
	pub MessageQueueServiceWeight: Weight = Perbill::from_percent(35) * RuntimeBlockWeights::get().max_block;
	pub MessageQueueIdleServiceWeight: Weight = Perbill::from_percent(20) * RuntimeBlockWeights::get().max_block;
}

impl pallet_message_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_message_queue::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type MessageProcessor =
		pallet_message_queue::mock_helpers::NoopMessageProcessor<AggregateMessageOrigin>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type MessageProcessor = xcm_builder::ProcessXcmMessage<
		AggregateMessageOrigin,
		xcm_executor::XcmExecutor<xcm_config::XcmConfig>,
		RuntimeCall,
	>;
	type Size = u32;
	// The XCMP queue pallet is only ever able to handle the `Sibling(ParaId)` origin:
	type QueueChangeHandler = NarrowOriginToSibling<XcmpQueue>;
	type QueuePausedQuery = NarrowOriginToSibling<XcmpQueue>;
	type HeapSize = sp_core::ConstU32<{ 64 * 1024 }>;
	type MaxStale = sp_core::ConstU32<8>;
	type ServiceWeight = MessageQueueServiceWeight;
	type IdleMaxServiceWeight = MessageQueueIdleServiceWeight;
}

impl cumulus_pallet_aura_ext::Config for Runtime {}

parameter_types! {
	// Fellows pluralistic body.
	pub const FellowsBodyId: BodyId = BodyId::Technical;
	/// The asset ID for the asset that we use to pay for message delivery fees.
	pub FeeAssetId: AssetId = AssetId(xcm_config::KsmLocation::get());
	/// The base fee for the message delivery fees.
	pub const ToSiblingBaseDeliveryFee: u128 = CENTS.saturating_mul(3);
	pub const ToParentBaseDeliveryFee: u128 = CENTS.saturating_mul(3);
}

pub type PriceForSiblingParachainDelivery = polkadot_runtime_common::xcm_sender::ExponentialPrice<
	FeeAssetId,
	ToSiblingBaseDeliveryFee,
	TransactionByteFee,
	XcmpQueue,
>;
pub type PriceForParentDelivery = polkadot_runtime_common::xcm_sender::ExponentialPrice<
	FeeAssetId,
	ToParentBaseDeliveryFee,
	TransactionByteFee,
	ParachainSystem,
>;

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ChannelInfo = ParachainSystem;
	type VersionWrapper = PolkadotXcm;
	// Enqueue XCMP messages from siblings for later processing.
	type XcmpQueue = TransformOrigin<MessageQueue, AggregateMessageOrigin, ParaId, ParaIdToSibling>;
	type MaxActiveOutboundChannels = ConstU32<128>;
	// Most on-chain HRMP channels are configured to use 102400 bytes of max message size, so we
	// need to set the page size larger than that until we reduce the channel size on-chain.
	type MaxPageSize = ConstU32<{ 103 * 1024 }>;
	type MaxInboundSuspended = sp_core::ConstU32<1_000>;
	type ControllerOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		EnsureXcm<IsVoiceOfBody<FellowshipLocation, FellowsBodyId>>,
	>;
	type ControllerOriginConverter = xcm_config::XcmOriginToTransactDispatchOrigin;
	type WeightInfo = weights::cumulus_pallet_xcmp_queue::WeightInfo<Runtime>;
	type PriceForSiblingDelivery = PriceForSiblingParachainDelivery;
}

impl cumulus_pallet_xcmp_queue::migration::v5::V5Config for Runtime {
	// This must be the same as the `ChannelInfo` from the `Config`:
	type ChannelList = ParachainSystem;
}

parameter_types! {
	pub const Period: u32 = 6 * HOURS;
	pub const Offset: u32 = 0;
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	// we don't have stash and controller, thus we don't need the convert as well.
	type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionManager = CollatorSelection;
	// Essentially just Aura, but let's be pedantic.
	type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type WeightInfo = weights::pallet_session::WeightInfo<Runtime>;
	type DisablingStrategy = ();
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = ConstU32<100_000>;
	type AllowMultipleBlocksPerSlot = ConstBool<true>;
	type SlotDuration = ConstU64<SLOT_DURATION>;
}

parameter_types! {
	pub const PotId: PalletId = PalletId(*b"PotStake");
	pub const SessionLength: BlockNumber = 6 * HOURS;
	// StakingAdmin pluralistic body.
	pub const StakingAdminBodyId: BodyId = BodyId::Defense;
}

/// We allow root and the `StakingAdmin` to execute privileged collator selection operations.
pub type CollatorSelectionUpdateOrigin = EitherOfDiverse<
	EnsureRoot<AccountId>,
	EnsureXcm<IsVoiceOfBody<GovernanceLocation, StakingAdminBodyId>>,
>;

impl pallet_collator_selection::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type UpdateOrigin = CollatorSelectionUpdateOrigin;
	type PotId = PotId;
	type MaxCandidates = ConstU32<100>;
	type MinEligibleCollators = ConstU32<4>;
	type MaxInvulnerables = ConstU32<20>;
	// should be a multiple of session or things will get inconsistent
	type KickThreshold = Period;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
	type ValidatorRegistration = Session;
	type WeightInfo = weights::pallet_collator_selection::WeightInfo<Runtime>;
}

impl pallet_asset_conversion_tx_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AssetId = xcm::v4::Location;
	type OnChargeAssetTransaction = pallet_asset_conversion_tx_payment::SwapAssetAdapter<
		KsmLocationV4,
		NativeAndAssets,
		AssetConversion,
		ResolveAssetTo<StakingPot, NativeAndAssets>,
	>;
	type WeightInfo = weights::pallet_asset_conversion_tx_payment::WeightInfo<Self>;

	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = AssetConversionTxHelper;
}

parameter_types! {
	pub const UniquesCollectionDeposit: Balance = UNITS / 10; // 1 / 10 UNIT deposit to create a collection
	pub const UniquesItemDeposit: Balance = UNITS / 1_000; // 1 / 1000 UNIT deposit to mint an item
	pub const UniquesMetadataDepositBase: Balance = system_para_deposit(1, 129);
	pub const UniquesAttributeDepositBase: Balance = system_para_deposit(1, 0);
	pub const UniquesDepositPerByte: Balance = system_para_deposit(0, 1);
}

impl pallet_uniques::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type CollectionId = u32;
	type ItemId = u32;
	type Currency = Balances;
	type ForceOrigin = AssetsForceOrigin;
	type CollectionDeposit = UniquesCollectionDeposit;
	type ItemDeposit = UniquesItemDeposit;
	type MetadataDepositBase = UniquesMetadataDepositBase;
	type AttributeDepositBase = UniquesAttributeDepositBase;
	type DepositPerByte = UniquesDepositPerByte;
	type StringLimit = ConstU32<128>;
	type KeyLimit = ConstU32<32>;
	type ValueLimit = ConstU32<64>;
	type WeightInfo = weights::pallet_uniques::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = ();
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type Locker = ();
}

parameter_types! {
	pub const NftFractionalizationPalletId: PalletId = PalletId(*b"fraction");
	pub NewAssetSymbol: BoundedVec<u8, AssetsStringLimit> = (*b"FRAC").to_vec().try_into().unwrap();
	pub NewAssetName: BoundedVec<u8, AssetsStringLimit> = (*b"Frac").to_vec().try_into().unwrap();
}

impl pallet_nft_fractionalization::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Deposit = AssetDeposit;
	type Currency = Balances;
	type NewAssetSymbol = NewAssetSymbol;
	type NewAssetName = NewAssetName;
	type StringLimit = AssetsStringLimit;
	type NftCollectionId = <Self as pallet_nfts::Config>::CollectionId;
	type NftId = <Self as pallet_nfts::Config>::ItemId;
	type AssetBalance = <Self as pallet_balances::Config>::Balance;
	type AssetId = <Self as pallet_assets::Config<TrustBackedAssetsInstance>>::AssetId;
	type Assets = Assets;
	type Nfts = Nfts;
	type PalletId = NftFractionalizationPalletId;
	type WeightInfo = weights::pallet_nft_fractionalization::WeightInfo<Runtime>;
	type RuntimeHoldReason = RuntimeHoldReason;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

parameter_types! {
	pub NftsPalletFeatures: PalletFeatures = PalletFeatures::all_enabled();
	pub const NftsMaxDeadlineDuration: BlockNumber = 12 * 30 * DAYS;
	// re-use the Uniques deposits
	pub const NftsCollectionDeposit: Balance = system_para_deposit(1, 130);
	pub const NftsItemDeposit: Balance = system_para_deposit(1, 164) / 40;
	pub const NftsMetadataDepositBase: Balance = system_para_deposit(1, 129) / 10;
	pub const NftsAttributeDepositBase: Balance = system_para_deposit(1, 0) / 10;
	pub const NftsDepositPerByte: Balance = system_para_deposit(0, 1);
}

impl pallet_nfts::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type CollectionId = u32;
	type ItemId = u32;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type ForceOrigin = AssetsForceOrigin;
	type Locker = ();
	type CollectionDeposit = NftsCollectionDeposit;
	type ItemDeposit = NftsItemDeposit;
	type MetadataDepositBase = NftsMetadataDepositBase;
	type AttributeDepositBase = NftsAttributeDepositBase;
	type DepositPerByte = NftsDepositPerByte;
	type StringLimit = ConstU32<256>;
	type KeyLimit = ConstU32<64>;
	type ValueLimit = ConstU32<256>;
	type ApprovalsLimit = ConstU32<20>;
	type ItemAttributesApprovalsLimit = ConstU32<30>;
	type MaxTips = ConstU32<10>;
	type MaxDeadlineDuration = NftsMaxDeadlineDuration;
	type MaxAttributesPerCall = ConstU32<10>;
	type Features = NftsPalletFeatures;
	type OffchainSignature = Signature;
	type OffchainPublic = <Signature as Verify>::Signer;
	type WeightInfo = weights::pallet_nfts::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = ();
	type BlockNumberProvider = System;
}

/// XCM router instance to BridgeHub with bridging capabilities for `Polkadot` global
/// consensus with dynamic fees and back-pressure.
pub type ToPolkadotXcmRouterInstance = pallet_xcm_bridge_hub_router::Instance1;
impl pallet_xcm_bridge_hub_router::Config<ToPolkadotXcmRouterInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_xcm_bridge_hub_router::WeightInfo<Runtime>;

	type UniversalLocation = xcm_config::UniversalLocation;
	type BridgedNetworkId = xcm_config::bridging::to_polkadot::PolkadotNetwork;
	type Bridges = xcm_config::bridging::NetworkExportTable;
	type DestinationVersion = PolkadotXcm;

	type SiblingBridgeHubLocation = xcm_config::bridging::SiblingBridgeHub;
	type BridgeHubOrigin =
		EitherOfDiverse<EnsureRoot<AccountId>, EnsureXcm<Equals<Self::SiblingBridgeHubLocation>>>;
	type ToBridgeHubSender = XcmpQueue;

	type ByteFee = xcm_config::bridging::XcmBridgeHubRouterByteFee;
	type FeeAsset = xcm_config::bridging::XcmBridgeHubRouterFeeAssetId;
	type LocalXcmChannelManager =
		cumulus_pallet_xcmp_queue::bridging::InAndOutXcmpChannelStatusProvider<Runtime>;
}

/// Converts from the relay chain proxy type to the local proxy type.
pub struct RelayChainToLocalProxyTypeConverter;

impl
	Convert<
		ProxyDefinition<AccountId, kusama_runtime_constants::proxy::ProxyType, BlockNumber>,
		Option<ProxyDefinition<AccountId, ProxyType, BlockNumber>>,
	> for RelayChainToLocalProxyTypeConverter
{
	fn convert(
		a: ProxyDefinition<AccountId, kusama_runtime_constants::proxy::ProxyType, BlockNumber>,
	) -> Option<ProxyDefinition<AccountId, ProxyType, BlockNumber>> {
		let proxy_type = match a.proxy_type {
			kusama_runtime_constants::proxy::ProxyType::Any => ProxyType::Any,
			kusama_runtime_constants::proxy::ProxyType::NonTransfer => ProxyType::NonTransfer,
			kusama_runtime_constants::proxy::ProxyType::CancelProxy => ProxyType::CancelProxy,
			// Proxy types that are not supported on AH.
			kusama_runtime_constants::proxy::ProxyType::Governance |
			kusama_runtime_constants::proxy::ProxyType::Staking |
			kusama_runtime_constants::proxy::ProxyType::Auction |
			kusama_runtime_constants::proxy::ProxyType::Spokesperson |
			kusama_runtime_constants::proxy::ProxyType::NominationPools |
			kusama_runtime_constants::proxy::ProxyType::Society |
			kusama_runtime_constants::proxy::ProxyType::ParaRegistration => return None,
		};

		Some(ProxyDefinition {
			delegate: a.delegate,
			proxy_type,
			// Delays are currently not supported by the remote proxy pallet, but should be
			// converted in the future to the block time used by the local proxy pallet.
			delay: a.delay,
		})
	}
}

impl pallet_remote_proxy::Config for Runtime {
	// The time between creating a proof and using the proof in a transaction.
	type MaxStorageRootsToKeep = ConstU32<{ RC_MINUTES }>;
	type RemoteProxy = kusama_runtime_constants::proxy::RemoteProxyInterface<
		ProxyType,
		RelayChainToLocalProxyTypeConverter,
	>;
	type WeightInfo = weights::pallet_remote_proxy::WeightInfo<Runtime>;
}

parameter_types! {
	pub const DepositPerItem: Balance = system_para_deposit(1, 0);
	pub const DepositPerByte: Balance = system_para_deposit(0, 1);
	pub CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(30);
}

impl pallet_revive::Config for Runtime {
	type Time = Timestamp;
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type CallFilter = Nothing;
	type DepositPerItem = DepositPerItem;
	type DepositPerByte = DepositPerByte;
	type WeightPrice = pallet_transaction_payment::Pallet<Self>;
	type WeightInfo = weights::pallet_revive::WeightInfo<Self>;
	type ChainExtension = ();
	type AddressMapper = pallet_revive::AccountId32Mapper<Self>;
	type RuntimeMemory = ConstU32<{ 128 * 1024 * 1024 }>;
	type PVFMemory = ConstU32<{ 512 * 1024 * 1024 }>;
	type UnsafeUnstableInterface = ConstBool<false>;
	type UploadOrigin = EnsureSigned<Self::AccountId>;
	type InstantiateOrigin = EnsureSigned<Self::AccountId>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent;
	type Xcm = pallet_xcm::Pallet<Self>;
	type ChainId = ConstU64<420_420_418>;
	type NativeToEthRatio = ConstU32<1_000_000>; // 10^(18 - 12) Eth is 10^18, Native is 10^12.
	type EthGasEncoder = ();
	type FindAuthor = <Runtime as pallet_authorship::Config>::FindAuthor;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub enum Runtime
	{
		// System support stuff.
		System: frame_system = 0,
		ParachainSystem: cumulus_pallet_parachain_system = 1,
		// RandomnessCollectiveFlip = 2 removed
		Timestamp: pallet_timestamp = 3,
		ParachainInfo: parachain_info = 4,

		// Monetary stuff.
		Balances: pallet_balances = 10,
		TransactionPayment: pallet_transaction_payment = 11,
		AssetTxPayment: pallet_asset_conversion_tx_payment = 13,
		Vesting: pallet_vesting = 14,

		// Collator support. the order of these 5 are important and shall not change.
		Authorship: pallet_authorship = 20,
		CollatorSelection: pallet_collator_selection = 21,
		Session: pallet_session = 22,
		Aura: pallet_aura = 23,
		AuraExt: cumulus_pallet_aura_ext = 24,

		// XCM helpers.
		XcmpQueue: cumulus_pallet_xcmp_queue = 30,
		PolkadotXcm: pallet_xcm = 31,
		CumulusXcm: cumulus_pallet_xcm = 32,
		// DmpQueue = 33
		ToPolkadotXcmRouter: pallet_xcm_bridge_hub_router::<Instance1> = 34,
		MessageQueue: pallet_message_queue = 35,

		// Handy utilities.
		Utility: pallet_utility = 40,
		Multisig: pallet_multisig = 41,
		Proxy: pallet_proxy = 42,
		RemoteProxyRelayChain: pallet_remote_proxy = 43,

		// The main stage.
		Assets: pallet_assets::<Instance1> = 50,
		Uniques: pallet_uniques = 51,
		Nfts: pallet_nfts = 52,
		ForeignAssets: pallet_assets::<Instance2> = 53,
		NftFractionalization: pallet_nft_fractionalization = 54,

		PoolAssets: pallet_assets::<Instance3> = 55,
		AssetConversion: pallet_asset_conversion = 56,

		Revive: pallet_revive = 60,

		// State trie migration pallet, only temporary.
		StateTrieMigration: pallet_state_trie_migration = 70,

		// OpenGov stuff.
		Treasury: pallet_treasury = 71,
		ConvictionVoting: pallet_conviction_voting = 72,
		Referenda: pallet_referenda = 73,
		Origins: pallet_custom_origins = 74,
		Whitelist: pallet_whitelist = 75,
		// TODO: do we need?
		// Parameters: pallet_parameters = 46,
	}
);

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The TransactionExtension to the basic transaction logic.
pub type TxExtension = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_asset_conversion_tx_payment::ChargeAssetTxPayment<Runtime>,
	frame_metadata_hash_extension::CheckMetadataHash<Runtime>,
);

/// Default extensions applied to Ethereum transactions.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct EthExtraImpl;

impl EthExtra for EthExtraImpl {
	type Config = Runtime;
	type Extension = TxExtension;

	fn get_eth_extension(nonce: u32, tip: Balance) -> Self::Extension {
		(
			frame_system::CheckNonZeroSender::<Runtime>::new(),
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckMortality::from(generic::Era::Immortal),
			frame_system::CheckNonce::<Runtime>::from(nonce),
			frame_system::CheckWeight::<Runtime>::new(),
			pallet_asset_conversion_tx_payment::ChargeAssetTxPayment::<Runtime>::from(tip, None),
			frame_metadata_hash_extension::CheckMetadataHash::<Runtime>::new(false),
		)
	}
}

// This impl is no longer necessary post stable-2506.
impl TryFrom<RuntimeCall> for pallet_revive::Call<Runtime> {
	type Error = ();

	fn try_from(value: RuntimeCall) -> Result<Self, Self::Error> {
		match value {
			RuntimeCall::Revive(call) => Ok(call),
			_ => Err(()),
		}
	}
}

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	pallet_revive::evm::runtime::UncheckedExtrinsic<Address, Signature, EthExtraImpl>;

/// Migrations to apply on runtime upgrade.
pub type Migrations = (
	pallet_session::migrations::v1::MigrateV0ToV1<
		Runtime,
		pallet_session::migrations::v1::InitOffenceSeverity<Runtime>,
	>,
	cumulus_pallet_aura_ext::migration::MigrateV0ToV1<Runtime>,
	// permanent
	pallet_xcm::migration::MigrateToLatestXcmVersion<Runtime>,
);

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
	Migrations,
>;

#[cfg(feature = "runtime-benchmarks")]
pub struct AssetConversionTxHelper;

#[cfg(feature = "runtime-benchmarks")]
pub type AssetConversionAssetIdFor<T> = <T as pallet_asset_conversion_tx_payment::Config>::AssetId;

#[cfg(feature = "runtime-benchmarks")]
impl
	pallet_asset_conversion_tx_payment::BenchmarkHelperTrait<
		AccountId,
		AssetConversionAssetIdFor<Runtime>,
		AssetConversionAssetIdFor<Runtime>,
	> for AssetConversionTxHelper
{
	fn create_asset_id_parameter(
		seed: u32,
	) -> (AssetConversionAssetIdFor<Runtime>, AssetConversionAssetIdFor<Runtime>) {
		// Use a different parachain' foreign assets pallet so that the asset is indeed foreign.
		let asset_id = xcm::v4::Location::new(
			1,
			[
				xcm::v4::Junction::Parachain(3000),
				xcm::v4::Junction::PalletInstance(53),
				xcm::v4::Junction::GeneralIndex(seed.into()),
			],
		);
		(asset_id.clone(), asset_id)
	}

	fn setup_balances_and_pool(asset_id: AssetConversionAssetIdFor<Runtime>, account: AccountId) {
		use alloc::boxed::Box;
		use frame_support::{assert_ok, traits::fungibles::Mutate};

		assert_ok!(ForeignAssets::force_create(
			RuntimeOrigin::root(),
			asset_id.clone(),
			account.clone().into(), /* owner */
			true,                   /* is_sufficient */
			1,
		));

		let lp_provider = account.clone();
		use frame_support::traits::Currency;
		let _ = Balances::deposit_creating(&lp_provider, u64::MAX.into());
		assert_ok!(ForeignAssets::mint_into(asset_id.clone(), &lp_provider, u64::MAX.into()));

		let token_native = Box::new(KsmLocationV4::get());
		let token_second = Box::new(asset_id);

		assert_ok!(AssetConversion::create_pool(
			RuntimeOrigin::signed(lp_provider.clone()),
			token_native.clone(),
			token_second.clone()
		));

		assert_ok!(AssetConversion::add_liquidity(
			RuntimeOrigin::signed(lp_provider.clone()),
			token_native,
			token_second,
			(u32::MAX / 8).into(), // 1 desired
			u32::MAX.into(),       // 2 desired
			1,                     // 1 min
			1,                     // 2 min
			lp_provider,
		));
	}
}

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	use super::*;
	use alloc::boxed::Box;
	use frame_support::assert_ok;

	frame_benchmarking::define_benchmarks!(
		[frame_system, SystemBench::<Runtime>]
		[frame_system_extensions, SystemExtensionsBench::<Runtime>]
		[pallet_assets, Local]
		[pallet_assets, Foreign]
		[pallet_assets, Pool]
		[pallet_asset_conversion, AssetConversion]
		// TODO: Somehow, benchmarks for this pallet are not visible outside the pallet
		[pallet_asset_conversion_tx_payment, AssetTxPayment]
		[pallet_balances, Balances]
		[pallet_message_queue, MessageQueue]
		[pallet_multisig, Multisig]
		[pallet_nft_fractionalization, NftFractionalization]
		[pallet_nfts, Nfts]
		[pallet_proxy, Proxy]
		[pallet_remote_proxy, RemoteProxyRelayChain]
		[pallet_revive, Revive]
		[pallet_session, SessionBench::<Runtime>]
		[pallet_uniques, Uniques]
		[pallet_utility, Utility]
		[pallet_vesting, Vesting]
		[pallet_timestamp, Timestamp]
		[pallet_transaction_payment, TransactionPayment]
		[pallet_collator_selection, CollatorSelection]
		[cumulus_pallet_parachain_system, ParachainSystem]
		[cumulus_pallet_xcmp_queue, XcmpQueue]
		// XCM
		[pallet_xcm, PalletXcmExtrinsicsBenchmark::<Runtime>]
		// Bridges
		[pallet_xcm_bridge_hub_router, ToPolkadot]
		// NOTE: Make sure you point to the individual modules below.
		[pallet_xcm_benchmarks::fungible, XcmBalances]
		[pallet_xcm_benchmarks::generic, XcmGeneric]
	);

	use frame_benchmarking::BenchmarkError;
	use xcm::latest::prelude::{
		Asset, Assets as XcmAssets, Fungible, Here, InteriorLocation, Junction, Junction::*,
		Location, NetworkId, NonFungible, Parent, ParentThen, Response, XCM_VERSION,
	};

	impl frame_system_benchmarking::Config for Runtime {
		fn setup_set_code_requirements(code: &Vec<u8>) -> Result<(), BenchmarkError> {
			ParachainSystem::initialize_for_set_code_benchmark(code.len() as u32);
			Ok(())
		}

		fn verify_set_code() {
			System::assert_last_event(
				cumulus_pallet_parachain_system::Event::<Runtime>::ValidationFunctionStored.into(),
			);
		}
	}

	impl cumulus_pallet_session_benchmarking::Config for Runtime {}

	use pallet_xcm_benchmarks::asset_instance_from;
	use xcm_config::{KsmLocation, MaxAssetsIntoHolding};

	parameter_types! {
		pub ExistentialDepositAsset: Option<Asset> = Some((
			KsmLocation::get(),
			ExistentialDeposit::get()
		).into());
		pub const RandomParaId: ParaId = ParaId::new(43211234);
	}

	impl pallet_xcm::benchmarking::Config for Runtime {
		type DeliveryHelper = (
			cumulus_primitives_utility::ToParentDeliveryHelper<
				xcm_config::XcmConfig,
				ExistentialDepositAsset,
				PriceForParentDelivery,
			>,
			polkadot_runtime_common::xcm_sender::ToParachainDeliveryHelper<
				xcm_config::XcmConfig,
				ExistentialDepositAsset,
				PriceForSiblingParachainDelivery,
				RandomParaId,
				ParachainSystem,
			>,
		);

		fn reachable_dest() -> Option<Location> {
			Some(Parent.into())
		}

		fn teleportable_asset_and_dest() -> Option<(Asset, Location)> {
			// Relay/native token can be teleported between AH and Relay.
			Some((
				Asset { fun: Fungible(ExistentialDeposit::get()), id: AssetId(Parent.into()) },
				Parent.into(),
			))
		}

		fn reserve_transferable_asset_and_dest() -> Option<(Asset, Location)> {
			// AH can reserve transfer native token to some random parachain.
			Some((
				Asset { fun: Fungible(ExistentialDeposit::get()), id: AssetId(Parent.into()) },
				ParentThen(Parachain(RandomParaId::get().into()).into()).into(),
			))
		}

		fn set_up_complex_asset_transfer() -> Option<(XcmAssets, u32, Location, Box<dyn FnOnce()>)>
		{
			// Transfer to Relay some local AH asset (local-reserve-transfer) while paying
			// fees using teleported native token.
			// (We don't care that Relay doesn't accept incoming unknown AH local asset)
			let dest = Parent.into();

			let fee_amount = ExistentialDeposit::get();
			let fee_asset: Asset = (Location::parent(), fee_amount).into();

			let who = frame_benchmarking::whitelisted_caller();
			// Give some multiple of the existential deposit
			let balance = fee_amount + ExistentialDeposit::get() * 1000;
			let _ = <Balances as frame_support::traits::Currency<_>>::make_free_balance_be(
				&who, balance,
			);
			// verify initial balance
			assert_eq!(Balances::free_balance(&who), balance);

			// set up local asset
			let asset_amount = 10u128;
			let initial_asset_amount = asset_amount * 10;
			let (asset_id, _, _) = pallet_assets::benchmarking::create_default_minted_asset::<
				Runtime,
				pallet_assets::Instance1,
			>(true, initial_asset_amount);
			let asset_location =
				Location::new(0, [PalletInstance(50), GeneralIndex(u32::from(asset_id).into())]);
			let transfer_asset: Asset = (asset_location, asset_amount).into();

			let assets: XcmAssets = vec![fee_asset.clone(), transfer_asset].into();
			let fee_index = if assets.get(0).unwrap().eq(&fee_asset) { 0 } else { 1 };

			// verify transferred successfully
			let verify = Box::new(move || {
				// verify native balance after transfer, decreased by transferred fee amount
				// (plus transport fees)
				assert!(Balances::free_balance(&who) <= balance - fee_amount);
				// verify asset balance decreased by exactly transferred amount
				assert_eq!(
					Assets::balance(asset_id.into(), &who),
					initial_asset_amount - asset_amount,
				);
			});
			Some((assets, fee_index as u32, dest, verify))
		}

		fn get_asset() -> Asset {
			Asset { id: AssetId(Location::parent()), fun: Fungible(ExistentialDeposit::get()) }
		}
	}

	impl pallet_xcm_benchmarks::Config for Runtime {
		type XcmConfig = xcm_config::XcmConfig;
		type AccountIdConverter = xcm_config::LocationToAccountId;
		type DeliveryHelper = cumulus_primitives_utility::ToParentDeliveryHelper<
			xcm_config::XcmConfig,
			ExistentialDepositAsset,
			PriceForParentDelivery,
		>;
		fn valid_destination() -> Result<Location, BenchmarkError> {
			Ok(KsmLocation::get())
		}
		fn worst_case_holding(depositable_count: u32) -> XcmAssets {
			// A mix of fungible, non-fungible, and concrete assets.
			let holding_non_fungibles = MaxAssetsIntoHolding::get() / 2 - depositable_count;
			let holding_fungibles = holding_non_fungibles.saturating_sub(2); // -2 for two `iter::once` bellow
			let fungibles_amount: u128 = 100;
			(0..holding_fungibles)
				.map(|i| {
					Asset {
						id: AssetId(GeneralIndex(i as u128).into()),
						fun: Fungible(fungibles_amount * (i + 1) as u128), // non-zero amount
					}
				})
				.chain(core::iter::once(Asset {
					id: AssetId(Here.into()),
					fun: Fungible(u128::MAX),
				}))
				.chain(core::iter::once(Asset {
					id: AssetId(KsmLocation::get()),
					fun: Fungible(1_000_000 * UNITS),
				}))
				.chain((0..holding_non_fungibles).map(|i| Asset {
					id: AssetId(GeneralIndex(i as u128).into()),
					fun: NonFungible(asset_instance_from(i)),
				}))
				.collect::<Vec<_>>()
				.into()
		}
	}

	parameter_types! {
		pub const TrustedTeleporter: Option<(Location, Asset)> = Some((
			KsmLocation::get(),
			Asset { fun: Fungible(UNITS), id: AssetId(KsmLocation::get()) },
		));
		pub const CheckedAccount: Option<(AccountId, xcm_builder::MintLocation)> = None;
		// AssetHubKusama trusts AssetHubPolkadot as reserve for DOTs
		pub TrustedReserve: Option<(Location, Asset)> = Some(
			(
				xcm_config::bridging::to_polkadot::AssetHubPolkadot::get(),
				Asset::from((
					xcm_config::bridging::to_polkadot::DotLocation::get(),
					10000000000_u128,
				))
			)
		);
	}

	impl pallet_xcm_benchmarks::fungible::Config for Runtime {
		type TransactAsset = Balances;

		type CheckedAccount = CheckedAccount;
		type TrustedTeleporter = TrustedTeleporter;
		type TrustedReserve = TrustedReserve;

		fn get_asset() -> Asset {
			Asset { id: AssetId(KsmLocation::get()), fun: Fungible(UNITS) }
		}
	}

	impl pallet_xcm_benchmarks::generic::Config for Runtime {
		type TransactAsset = Balances;
		type RuntimeCall = RuntimeCall;

		fn worst_case_response() -> (u64, Response) {
			(0u64, Response::Version(Default::default()))
		}

		fn worst_case_asset_exchange() -> Result<(XcmAssets, XcmAssets), BenchmarkError> {
			let native_asset_location = xcm::v4::Location::parent();
			let native_asset_id = xcm::v4::AssetId(native_asset_location.clone());
			let (account, _) = pallet_xcm_benchmarks::account_and_location::<Runtime>(1);
			let origin = RuntimeOrigin::signed(account.clone());
			let asset_location = xcm::v4::Location::new(1, [xcm::v4::Junction::Parachain(2001)]);
			let asset_id = xcm::v4::AssetId(asset_location.clone());

			// We set everything up, initial amounts, liquidity pools, liquidity...
			assert_ok!(<Balances as fungible::Mutate<_>>::mint_into(
				&account,
				ExistentialDeposit::get() + (2_000 * UNITS)
			));

			assert_ok!(ForeignAssets::force_create(
				RuntimeOrigin::root(),
				asset_location.clone(),
				account.clone().into(),
				true,
				1,
			));

			assert_ok!(ForeignAssets::mint(
				origin.clone(),
				asset_location.clone(),
				account.clone().into(),
				4_000 * UNITS,
			));

			assert_ok!(AssetConversion::create_pool(
				origin.clone(),
				native_asset_location.clone().into(),
				asset_location.clone().into(),
			));

			// 1 UNIT of the native asset is worth 2 UNITS of the foreign asset.
			assert_ok!(AssetConversion::add_liquidity(
				origin,
				native_asset_location.into(),
				asset_location.into(),
				1_000 * UNITS,
				2_000 * UNITS,
				1,
				1,
				account,
			));

			let native_asset_id_latest: AssetId = native_asset_id.try_into().unwrap();
			let asset_id_latest: AssetId = asset_id.try_into().unwrap();
			let give_assets: XcmAssets = (native_asset_id_latest, 500 * UNITS).into();
			let receive_assets: XcmAssets = (asset_id_latest, 660 * UNITS).into();

			Ok((give_assets, receive_assets))
		}

		fn universal_alias() -> Result<(Location, Junction), BenchmarkError> {
			xcm_config::bridging::BridgingBenchmarksHelper::prepare_universal_alias()
				.ok_or(BenchmarkError::Skip)
		}

		fn transact_origin_and_runtime_call() -> Result<(Location, RuntimeCall), BenchmarkError> {
			Ok((
				KsmLocation::get(),
				frame_system::Call::remark_with_event { remark: vec![] }.into(),
			))
		}

		fn subscribe_origin() -> Result<Location, BenchmarkError> {
			Ok(KsmLocation::get())
		}

		fn claimable_asset() -> Result<(Location, Location, XcmAssets), BenchmarkError> {
			let origin = KsmLocation::get();
			let assets: XcmAssets = (AssetId(KsmLocation::get()), 1_000 * UNITS).into();
			let ticket = Location { parents: 0, interior: Here };
			Ok((origin, ticket, assets))
		}

		fn fee_asset() -> Result<Asset, BenchmarkError> {
			Ok(Asset { id: AssetId(KsmLocation::get()), fun: Fungible(1_000_000 * UNITS) })
		}

		fn unlockable_asset() -> Result<(Location, Location, Asset), BenchmarkError> {
			Err(BenchmarkError::Skip)
		}

		fn export_message_origin_and_destination(
		) -> Result<(Location, NetworkId, InteriorLocation), BenchmarkError> {
			Err(BenchmarkError::Skip)
		}

		fn alias_origin() -> Result<(Location, Location), BenchmarkError> {
			Ok((
				Location::new(1, [Parachain(1001)]),
				Location::new(1, [Parachain(1001), AccountId32 { id: [111u8; 32], network: None }]),
			))
		}
	}

	use pallet_xcm_bridge_hub_router::benchmarking::Config as XcmBridgeHubRouterConfig;

	impl XcmBridgeHubRouterConfig<ToPolkadotXcmRouterInstance> for Runtime {
		fn make_congested() {
			cumulus_pallet_xcmp_queue::bridging::suspend_channel_for_benchmarks::<Runtime>(
				xcm_config::bridging::SiblingBridgeHubParaId::get().into(),
			);
		}

		fn ensure_bridged_target_destination() -> Result<Location, BenchmarkError> {
			ParachainSystem::open_outbound_hrmp_channel_for_benchmarks_or_tests(
				xcm_config::bridging::SiblingBridgeHubParaId::get().into(),
			);
			let bridged_asset_hub = xcm_config::bridging::to_polkadot::AssetHubPolkadot::get();
			PolkadotXcm::force_xcm_version(
				RuntimeOrigin::root(),
				Box::new(bridged_asset_hub.clone()),
				XCM_VERSION,
			)
			.map_err(|e| {
				log::error!(
					"Failed to dispatch `force_xcm_version({:?}, {:?}, {:?})`, error: {:?}",
					RuntimeOrigin::root(),
					bridged_asset_hub,
					XCM_VERSION,
					e
				);
				BenchmarkError::Stop("XcmVersion was not stored!")
			})?;
			Ok(bridged_asset_hub)
		}
	}

	pub use cumulus_pallet_session_benchmarking::Pallet as SessionBench;
	pub use frame_benchmarking::{BenchmarkBatch, BenchmarkList};
	pub use frame_support::traits::{StorageInfoTrait, WhitelistedStorageKeys};
	pub use frame_system_benchmarking::{
		extensions::Pallet as SystemExtensionsBench, Pallet as SystemBench,
	};
	pub use pallet_xcm::benchmarking::Pallet as PalletXcmExtrinsicsBenchmark;
	pub use pallet_xcm_bridge_hub_router::benchmarking::Pallet as XcmBridgeHubRouterBench;
	pub use sp_storage::TrackedStorageKey;
	pub type XcmBalances = pallet_xcm_benchmarks::fungible::Pallet<Runtime>;
	pub type XcmGeneric = pallet_xcm_benchmarks::generic::Pallet<Runtime>;
	pub type Local = pallet_assets::Pallet<Runtime, TrustBackedAssetsInstance>;
	pub type Foreign = pallet_assets::Pallet<Runtime, ForeignAssetsInstance>;
	pub type Pool = pallet_assets::Pallet<Runtime, PoolAssetsInstance>;
	pub type ToPolkadot = XcmBridgeHubRouterBench<Runtime, ToPolkadotXcmRouterInstance>;
}

#[cfg(feature = "runtime-benchmarks")]
use benches::*;

impl_runtime_apis! {
	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(SLOT_DURATION)
		}

		fn authorities() -> Vec<AuraId> {
			pallet_aura::Authorities::<Runtime>::get().into_inner()
		}
	}

	impl cumulus_primitives_aura::AuraUnincludedSegmentApi<Block> for Runtime {
		fn can_build_upon(
			included_hash: <Block as BlockT>::Hash,
			slot: cumulus_primitives_aura::Slot,
		) -> bool {
			ConsensusHook::can_build_upon(included_hash, slot)
		}
	}

	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl pallet_asset_conversion::AssetConversionApi<
		Block,
		Balance,
		xcm::v4::Location,
	> for Runtime
	{
		fn quote_price_exact_tokens_for_tokens(asset1: xcm::v4::Location, asset2: xcm::v4::Location, amount: Balance, include_fee: bool) -> Option<Balance> {
			AssetConversion::quote_price_exact_tokens_for_tokens(asset1, asset2, amount, include_fee)
		}

		fn quote_price_tokens_for_exact_tokens(asset1: xcm::v4::Location, asset2: xcm::v4::Location, amount: Balance, include_fee: bool) -> Option<Balance> {
			AssetConversion::quote_price_tokens_for_exact_tokens(asset1, asset2, amount, include_fee)
		}

		fn get_reserves(asset1: xcm::v4::Location, asset2: xcm::v4::Location) -> Option<(Balance, Balance)> {
			AssetConversion::get_reserves(asset1, asset2).ok()
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
		for Runtime
	{
		fn query_call_info(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl xcm_runtime_apis::fees::XcmPaymentApi<Block> for Runtime {
		fn query_acceptable_payment_assets(xcm_version: xcm::Version) -> Result<Vec<VersionedAssetId>, XcmPaymentApiError> {
			let native_asset = xcm_config::KsmLocation::get();
			// We accept the native asset to pay fees.
			let mut acceptable_assets = vec![AssetId(native_asset.clone())];
			// We also accept all assets in a pool with the native token.
			acceptable_assets.extend(
				assets_common::PoolAdapter::<Runtime>::get_assets_in_pool_with(native_asset)
				.map_err(|()| XcmPaymentApiError::VersionedConversionFailed)?
			);
			PolkadotXcm::query_acceptable_payment_assets(xcm_version, acceptable_assets)
		}

		fn query_weight_to_asset_fee(weight: Weight, asset: VersionedAssetId) -> Result<u128, XcmPaymentApiError> {
			let native_asset = xcm_config::KsmLocation::get();
			let fee_in_native = WeightToFee::weight_to_fee(&weight);
			let latest_asset_id: Result<AssetId, ()> = asset.clone().try_into();
			match latest_asset_id {
				Ok(asset_id) if asset_id.0 == native_asset => {
					// for native asset
					Ok(fee_in_native)
				},
				Ok(asset_id) => {
					// Try to get current price of `asset_id` in `native_asset`.
					if let Ok(Some(swapped_in_native)) = assets_common::PoolAdapter::<Runtime>::quote_price_tokens_for_exact_tokens(
							asset_id.0.clone(),
							native_asset,
							fee_in_native,
							true, // We include the fee.
						) {
						Ok(swapped_in_native)
					} else {
						log::trace!(target: "xcm::xcm_runtime_apis", "query_weight_to_asset_fee - unhandled asset_id: {asset_id:?}!");
						Err(XcmPaymentApiError::AssetNotFound)
					}
				},
				Err(_) => {
					log::trace!(target: "xcm::xcm_runtime_apis", "query_weight_to_asset_fee - failed to convert asset: {asset:?}!");
					Err(XcmPaymentApiError::VersionedConversionFailed)
				}
			}
		}

		fn query_xcm_weight(message: VersionedXcm<()>) -> Result<Weight, XcmPaymentApiError> {
			PolkadotXcm::query_xcm_weight(message)
		}

		fn query_delivery_fees(destination: VersionedLocation, message: VersionedXcm<()>) -> Result<VersionedAssets, XcmPaymentApiError> {
			PolkadotXcm::query_delivery_fees(destination, message)
		}
	}

	impl xcm_runtime_apis::dry_run::DryRunApi<Block, RuntimeCall, RuntimeEvent, OriginCaller> for Runtime {
		fn dry_run_call(origin: OriginCaller, call: RuntimeCall, result_xcms_version: XcmVersion) -> Result<CallDryRunEffects<RuntimeEvent>, XcmDryRunApiError> {
			PolkadotXcm::dry_run_call::<Runtime, xcm_config::XcmRouter, OriginCaller, RuntimeCall>(origin, call, result_xcms_version)
		}

		fn dry_run_xcm(origin_location: VersionedLocation, xcm: VersionedXcm<RuntimeCall>) -> Result<XcmDryRunEffects<RuntimeEvent>, XcmDryRunApiError> {
			PolkadotXcm::dry_run_xcm::<Runtime, xcm_config::XcmRouter, RuntimeCall, xcm_config::XcmConfig>(origin_location, xcm)
		}
	}

	impl xcm_runtime_apis::conversions::LocationToAccountApi<Block, AccountId> for Runtime {
		fn convert_location(location: VersionedLocation) -> Result<
			AccountId,
			xcm_runtime_apis::conversions::Error
		> {
			xcm_runtime_apis::conversions::LocationToAccountHelper::<
				AccountId,
				xcm_config::LocationToAccountId,
			>::convert_location(location)
		}
	}

	impl assets_common::runtime_api::FungiblesApi<
		Block,
		AccountId,
	> for Runtime
	{
		fn query_account_balances(account: AccountId) -> Result<xcm::VersionedAssets, assets_common::runtime_api::FungiblesAccessError> {
			use assets_common::fungible_conversion::{convert, convert_balance};
			Ok([
				// collect pallet_balance
				{
					let balance = Balances::free_balance(account.clone());
					if balance > 0 {
						vec![convert_balance::<KsmLocation, Balance>(balance)?]
					} else {
						vec![]
					}
				},
				// collect pallet_assets (TrustBackedAssets)
				convert::<_, _, _, _, TrustBackedAssetsConvertedConcreteId>(
					Assets::account_balances(account.clone())
						.iter()
						.filter(|(_, balance)| balance > &0)
				)?,
				// collect pallet_assets (ForeignAssets)
				convert::<_, _, _, _, ForeignAssetsConvertedConcreteId>(
					ForeignAssets::account_balances(account.clone())
						.iter()
						.filter(|(_, balance)| balance > &0)
				)?,
				// collect pallet_assets (PoolAssets)
				convert::<_, _, _, _, PoolAssetsConvertedConcreteId>(
					PoolAssets::account_balances(account)
						.iter()
						.filter(|(_, balance)| balance > &0)
				)?,
				// collect ... e.g. other tokens
			].concat().into())
		}
	}

	impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
		fn collect_collation_info(header: &<Block as BlockT>::Header) -> cumulus_primitives_core::CollationInfo {
			ParachainSystem::collect_collation_info(header)
		}
	}

	impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
		fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
			build_state::<RuntimeGenesisConfig>(config)
		}

		fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
			get_preset::<RuntimeGenesisConfig>(id, &genesis_config_presets::get_preset)
		}

		fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
			genesis_config_presets::preset_names()
		}
	}

	impl pallet_revive::ReviveApi<Block, AccountId, Balance, Nonce, BlockNumber> for Runtime
	{
		fn balance(address: H160) -> U256 {
			Revive::evm_balance(&address)
		}

		fn block_gas_limit() -> U256 {
			Revive::evm_block_gas_limit()
		}

		fn gas_price() -> U256 {
			Revive::evm_gas_price()
		}

		fn nonce(address: H160) -> Nonce {
			let account = <Runtime as pallet_revive::Config>::AddressMapper::to_account_id(&address);
			System::account_nonce(account)
		}

		fn eth_transact(tx: pallet_revive::evm::GenericTransaction) -> Result<pallet_revive::EthTransactInfo<Balance>, pallet_revive::EthTransactError>
		{
			let blockweights: BlockWeights = <Runtime as frame_system::Config>::BlockWeights::get();
			let tx_fee = |pallet_call, mut dispatch_info: DispatchInfo| {
				let call = RuntimeCall::Revive(pallet_call);
				dispatch_info.extension_weight = EthExtraImpl::get_eth_extension(0, 0u32.into()).weight(&call);
				let uxt: UncheckedExtrinsic = sp_runtime::generic::UncheckedExtrinsic::new_bare(call).into();

				pallet_transaction_payment::Pallet::<Runtime>::compute_fee(
					uxt.encoded_size() as u32,
					&dispatch_info,
					0u32.into(),
				)
			};

			Revive::bare_eth_transact(tx, blockweights.max_block, tx_fee)
		}

		fn call(
			origin: AccountId,
			dest: H160,
			value: Balance,
			gas_limit: Option<Weight>,
			storage_deposit_limit: Option<Balance>,
			input_data: Vec<u8>,
		) -> pallet_revive::ContractResult<pallet_revive::ExecReturnValue, Balance> {
			let blockweights= <Runtime as frame_system::Config>::BlockWeights::get();
			Revive::bare_call(
				RuntimeOrigin::signed(origin),
				dest,
				value,
				gas_limit.unwrap_or(blockweights.max_block),
				pallet_revive::DepositLimit::Balance(storage_deposit_limit.unwrap_or(u128::MAX)),
				input_data,
			)
		}

		fn instantiate(
			origin: AccountId,
			value: Balance,
			gas_limit: Option<Weight>,
			storage_deposit_limit: Option<Balance>,
			code: pallet_revive::Code,
			data: Vec<u8>,
			salt: Option<[u8; 32]>,
		) -> pallet_revive::ContractResult<pallet_revive::InstantiateReturnValue, Balance>
		{
			let blockweights= <Runtime as frame_system::Config>::BlockWeights::get();
			Revive::bare_instantiate(
				RuntimeOrigin::signed(origin),
				value,
				gas_limit.unwrap_or(blockweights.max_block),
				pallet_revive::DepositLimit::Balance(storage_deposit_limit.unwrap_or(u128::MAX)),
				code,
				data,
				salt,
			)
		}

		fn upload_code(
			origin: AccountId,
			code: Vec<u8>,
			storage_deposit_limit: Option<Balance>,
		) -> pallet_revive::CodeUploadResult<Balance>
		{
			Revive::bare_upload_code(
				RuntimeOrigin::signed(origin),
				code,
				storage_deposit_limit.unwrap_or(u128::MAX),
			)
		}

		fn get_storage(
			address: H160,
			key: [u8; 32],
		) -> pallet_revive::GetStorageResult {
			Revive::get_storage(
				address,
				key
			)
		}

		fn trace_block(
			block: Block,
			tracer_type: pallet_revive::evm::TracerType,
		) -> Vec<(u32, pallet_revive::evm::Trace)> {
			use pallet_revive::tracing::trace;
			let mut tracer = Revive::evm_tracer(tracer_type);
			let mut traces = vec![];
			let (header, extrinsics) = block.deconstruct();
			Executive::initialize_block(&header);
			for (index, ext) in extrinsics.into_iter().enumerate() {
				trace(tracer.as_tracing(), || {
					let _ = Executive::apply_extrinsic(ext);
				});

				if let Some(tx_trace) = tracer.collect_trace() {
					traces.push((index as u32, tx_trace));
				}
			}

			traces
		}

		fn trace_tx(
			block: Block,
			tx_index: u32,
			tracer_type: pallet_revive::evm::TracerType,
		) -> Option<pallet_revive::evm::Trace> {
			use pallet_revive::tracing::trace;
			let mut tracer = Revive::evm_tracer(tracer_type);
			let (header, extrinsics) = block.deconstruct();

			Executive::initialize_block(&header);
			for (index, ext) in extrinsics.into_iter().enumerate() {
				if index as u32 == tx_index {
				trace(tracer.as_tracing(), || {
						let _ = Executive::apply_extrinsic(ext);
					});
					break;
				} else {
					let _ = Executive::apply_extrinsic(ext);
				}
			}

			tracer.collect_trace()
		}

		fn trace_call(
			tx: pallet_revive::evm::GenericTransaction,
			tracer_type: pallet_revive::evm::TracerType,
			)
			-> Result<pallet_revive::evm::Trace, pallet_revive::EthTransactError>
		{
			use pallet_revive::tracing::trace;
			let mut tracer = Revive::evm_tracer(tracer_type);
			let result = trace(tracer.as_tracing(), || Self::eth_transact(tx));

			if let Some(trace) = tracer.collect_trace() {
				Ok(trace)
			} else if let Err(err) = result {
				Err(err)
			} else {
				Ok(tracer.empty_trace())
			}
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, RuntimeBlockWeights::get().max_block)
		}

		fn execute_block(
			block: Block,
			state_root_check: bool,
			signature_check: bool,
			select: frame_try_runtime::TryStateSelect,
		) -> Weight {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here.
			Executive::try_execute_block(block, state_root_check, signature_check, select).unwrap()
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();
			(list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, alloc::string::String> {
			let whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();
			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);

			Ok(batches)
		}
	}
}

cumulus_pallet_parachain_system::register_validate_block! {
	Runtime = Runtime,
	BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
}

parameter_types! {
	// The deposit configuration for the singed migration. Specially if you want to allow any signed account to do the migration (see `SignedFilter`, these deposits should be high)
	pub const MigrationSignedDepositPerItem: Balance = CENTS;
	pub const MigrationSignedDepositBase: Balance = 2_000 * CENTS;
	pub const MigrationMaxKeyLen: u32 = 512;
}

impl pallet_state_trie_migration::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type SignedDepositPerItem = MigrationSignedDepositPerItem;
	type SignedDepositBase = MigrationSignedDepositBase;
	// An origin that can control the whole pallet: Should be a Fellowship member or the controller
	// of the migration.
	type ControlOrigin = EitherOfDiverse<
		EnsureXcm<IsVoiceOfBody<FellowshipLocation, FellowsBodyId>>,
		EnsureSignedBy<MigControllerRoot, AccountId>,
	>;
	type SignedFilter = EnsureSignedBy<MigController, AccountId>;

	// Replace this with weight based on your runtime.
	type WeightInfo = pallet_state_trie_migration::weights::SubstrateWeight<Runtime>;

	type MaxKeyLen = MigrationMaxKeyLen;
}
// Statemint State Migration Controller account controlled by parity.io. Can trigger migration.
// See bot code https://github.com/paritytech/polkadot-scripts/blob/master/src/services/state_trie_migration.ts
ord_parameter_types! {
	pub const MigController: AccountId = AccountId::from(hex_literal::hex!("8458ed39dc4b6f6c7255f7bc42be50c2967db126357c999d44e12ca7ac80dc52"));
	pub const MigControllerRoot: AccountId = AccountId::from(hex_literal::hex!("8458ed39dc4b6f6c7255f7bc42be50c2967db126357c999d44e12ca7ac80dc52"));
}

#[cfg(test)]
mod tests {
	use super::*;
	use sp_runtime::traits::Zero;
	use sp_weights::WeightToFee;
	use system_parachains_constants::kusama::fee;

	/// We can fit at least 1000 transfers in a block.
	#[test]
	fn sane_block_weight() {
		use pallet_balances::WeightInfo;
		let block = RuntimeBlockWeights::get().max_block;
		let base = RuntimeBlockWeights::get().get(DispatchClass::Normal).base_extrinsic;
		let transfer =
			base + weights::pallet_balances::WeightInfo::<Runtime>::transfer_allow_death();

		let fit = block.checked_div_per_component(&transfer).unwrap_or_default();
		assert!(fit >= 1000, "{} should be at least 1000", fit);
	}

	/// The fee for one transfer is at most 1 CENT.
	#[test]
	fn sane_transfer_fee() {
		use pallet_balances::WeightInfo;
		let base = RuntimeBlockWeights::get().get(DispatchClass::Normal).base_extrinsic;
		let transfer =
			base + weights::pallet_balances::WeightInfo::<Runtime>::transfer_allow_death();

		let fee: Balance = fee::WeightToFee::weight_to_fee(&transfer);
		assert!(fee <= CENTS, "{} MILLICENTS should be at most 1000", fee / MILLICENTS);
	}

	/// Weight is being charged for both dimensions.
	#[test]
	fn weight_charged_for_both_components() {
		let fee: Balance = fee::WeightToFee::weight_to_fee(&Weight::from_parts(10_000, 0));
		assert!(!fee.is_zero(), "Charges for ref time");

		let fee: Balance = fee::WeightToFee::weight_to_fee(&Weight::from_parts(0, 10_000));
		assert_eq!(fee, CENTS, "10kb maps to CENT");
	}

	/// Filling up a block by proof size is at most 30 times more expensive than ref time.
	///
	/// This is just a sanity check.
	#[test]
	fn full_block_fee_ratio() {
		let block = RuntimeBlockWeights::get().max_block;
		let time_fee: Balance =
			fee::WeightToFee::weight_to_fee(&Weight::from_parts(block.ref_time(), 0));
		let proof_fee: Balance =
			fee::WeightToFee::weight_to_fee(&Weight::from_parts(0, block.proof_size()));

		let proof_o_time = proof_fee.checked_div(time_fee).unwrap_or_default();
		assert!(proof_o_time <= 30, "{} should be at most 30", proof_o_time);
		let time_o_proof = time_fee.checked_div(proof_fee).unwrap_or_default();
		assert!(time_o_proof <= 30, "{} should be at most 30", time_o_proof);
	}

	#[test]
	fn test_transasction_byte_fee_is_one_tenth_of_relay() {
		let relay_tbf = kusama_runtime_constants::fee::TRANSACTION_BYTE_FEE;
		let parachain_tbf = TransactionByteFee::get();
		assert_eq!(relay_tbf / 10, parachain_tbf);
	}

	#[test]
	fn create_foreign_asset_deposit_is_equal_to_asset_hub_foreign_asset_pallet_deposit() {
		assert_eq!(
			bp_asset_hub_kusama::CreateForeignAssetDeposit::get(),
			ForeignAssetsAssetDeposit::get()
		);
	}

	#[test]
	fn ensure_key_ss58() {
		use frame_support::traits::SortedMembers;
		use sp_core::crypto::Ss58Codec;
		let acc =
			AccountId::from_ss58check("5F4EbSkZz18X36xhbsjvDNs6NuZ82HyYtq5UiJ1h9SBHJXZD").unwrap();
		assert_eq!(acc, MigController::sorted_members()[0]);
	}
}
