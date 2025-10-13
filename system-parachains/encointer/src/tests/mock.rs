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
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

use crate::{tests::xcm_mock::TestMessageSender, xcm_config::KsmLocation};
use codec::Encode;
use frame_support::{
	construct_runtime, derive_impl, parameter_types,
	traits::{
		tokens::imbalance::ResolveTo, AsEnsureOriginWithArg, ConstU32, Disabled, Everything,
		IsInVec, Nothing,
	},
	weights::WeightToFee,
};
use frame_system::{EnsureRoot, EnsureSigned};
use parachains_common::xcm_config::ParentRelayOrSiblingParachains;
use polkadot_primitives::{AccountIndex, BlakeTwo256, Signature};
use sp_runtime::{generic, traits::MaybeEquivalence, AccountId32, BuildStorage};
use system_parachains_constants::kusama::fee::WeightToFee as KusamaWeightToFee;
use xcm::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AllowSubscriptionsFrom, AllowTopLevelPaidExecutionFrom,
	AllowUnpaidExecutionFrom, ConvertedConcreteId, DescribeAllTerminal, DescribeFamily,
	EnsureXcmOrigin, FixedWeightBounds, FungiblesAdapter, HashedDescription, IsConcrete,
	NoChecking, SignedAccountId32AsNative, SignedToAccountId32, TakeWeightCredit, UsingComponents,
	WithComputedOrigin,
};
use xcm_executor::{
	traits::{ConvertLocation, JustTry, WeightTrader},
	AssetsInHolding, XcmExecutor,
};

pub type TxExtension = (
	frame_system::CheckNonZeroSender<Test>,
	frame_system::CheckSpecVersion<Test>,
	frame_system::CheckTxVersion<Test>,
	frame_system::CheckGenesis<Test>,
	frame_system::CheckMortality<Test>,
	frame_system::CheckNonce<Test>,
	frame_system::CheckWeight<Test>,
);
pub type Address = sp_runtime::MultiAddress<AccountId, AccountIndex>;
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, TxExtension>;
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

pub type BlockNumber = u32;
pub type AccountId = AccountId32;

construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		XcmPallet: pallet_xcm,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountData = pallet_balances::AccountData<Balance>;
	type AccountId = AccountId;
	type Lookup = sp_runtime::traits::IdentityLookup<AccountId>;
}

pub type Balance = u128;

parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
}

impl pallet_balances::Config for Test {
	type MaxLocks = ConstU32<0>;
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = ();
	type MaxFreezes = ConstU32<0>;
	type DoneSlashHandler = ();
}

parameter_types! {
	pub const AssetDeposit: u128 = 1_000_000;
	pub const MetadataDepositBase: u128 = 1_000_000;
	pub const MetadataDepositPerByte: u128 = 100_000;
	pub const AssetAccountDeposit: u128 = 1_000_000;
	pub const ApprovalDeposit: u128 = 1_000_000;
	pub const AssetsStringLimit: u32 = 50;
	pub const RemoveItemsLimit: u32 = 50;
}

impl pallet_assets::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = AssetIdForAssets;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type AssetAccountDeposit = AssetAccountDeposit;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = AssetsStringLimit;
	type Freezer = ();
	type Extra = ();
	type WeightInfo = ();
	type RemoveItemsLimit = RemoveItemsLimit;
	type AssetIdParameter = AssetIdForAssets;
	type CallbackHandle = ();
	type Holder = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

parameter_types! {
	pub const RelayLocation: Location = Here.into_location();
	pub const AnyNetwork: Option<NetworkId> = None;
	pub UniversalLocation: InteriorLocation = (ByGenesis([0; 32]), Parachain(42)).into();
	pub UnitWeightCost: u64 = 1_000;
	pub const BaseXcmWeight: Weight = Weight::from_parts(1_000, 1_000);
	pub CurrencyPerSecondPerByte: (AssetId, u128, u128) = (AssetId(RelayLocation::get()), 1, 1);
	pub TrustedAssets: (AssetFilter, Location) = (All.into(), Here.into());
	pub const MaxInstructions: u32 = 100;
	pub const MaxAssetsIntoHolding: u32 = 64;
	pub CheckingAccount: AccountId = XcmPallet::check_account();
}

type AssetIdForAssets = u128;

pub struct FromLocationToAsset<Location, AssetId>(core::marker::PhantomData<(Location, AssetId)>);
impl MaybeEquivalence<Location, AssetIdForAssets>
	for FromLocationToAsset<Location, AssetIdForAssets>
{
	fn convert(value: &Location) -> Option<AssetIdForAssets> {
		match value.unpack() {
			(0, []) => Some(0 as AssetIdForAssets),
			(1, []) => Some(1 as AssetIdForAssets),
			(0, [PalletInstance(1), GeneralIndex(index)]) if ![0, 1].contains(index) =>
				Some(*index as AssetIdForAssets),
			_ => None,
		}
	}

	fn convert_back(value: &AssetIdForAssets) -> Option<Location> {
		match value {
			0u128 => Some(Location { parents: 1, interior: Here }),
			1u128 => Some(Location { parents: 0, interior: Here }),
			para_id @ 1..=1000 =>
				Some(Location { parents: 1, interior: [Parachain(*para_id as u32)].into() }),
			_ => None,
		}
	}
}

/// Converts a local signed origin into an XCM location. Forms the basis for local origins
/// sending/executing XCMs.
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, AnyNetwork>;
pub type LocalAssetsTransactor = FungiblesAdapter<
	Assets,
	ConvertedConcreteId<
		AssetIdForAssets,
		Balance,
		FromLocationToAsset<Location, AssetIdForAssets>,
		JustTry,
	>,
	SovereignAccountOf,
	AccountId,
	NoChecking,
	CheckingAccount,
>;

type OriginConverter = (
	pallet_xcm::XcmPassthrough<RuntimeOrigin>,
	SignedAccountId32AsNative<AnyNetwork, RuntimeOrigin>,
);

parameter_types! {
	pub static ParaFortyTwo: Location = Location::new(1, [Parachain(42)]);
	pub static AllowExplicitUnpaidFrom: Vec<Location> = vec![];
	pub static AllowUnpaidFrom: Vec<Location> = vec![];
	pub static AllowPaidFrom: Vec<Location> = vec![ParaFortyTwo::get()];
	pub static AllowSubsFrom: Vec<Location> = vec![];
	// 1_000_000_000_000 => 1 unit of asset for 1 unit of ref time weight.
	// 1024 * 1024 => 1 unit of asset for 1 unit of proof size weight.
	pub static WeightPrice: (AssetId, u128, u128) =
		(From::from(Here), 1_000_000_000_000, 1024 * 1024);
}

pub type Barrier = (
	TakeWeightCredit,
	// AllowKnownQueryResponses<TestResponseHandler>,
	WithComputedOrigin<
		(
			// If the message is one that immediately attempts to pay for execution, then
			// allow it.
			AllowTopLevelPaidExecutionFrom<Everything>,
			// Subscriptions for version tracking are OK.
			AllowSubscriptionsFrom<ParentRelayOrSiblingParachains>,
		),
		UniversalLocation,
		ConstU32<8>,
	>,
	// AllowExplicitUnpaidExecutionFrom<IsInVec<AllowExplicitUnpaidFrom>>,
	AllowUnpaidExecutionFrom<IsInVec<AllowUnpaidFrom>>,
	// AllowSubscriptionsFrom<IsInVec<AllowSubsFrom>>,
);

#[derive(Clone)]
pub struct TestTrader {
	weight_bought_so_far: Weight,
}
impl WeightTrader for TestTrader {
	fn new() -> Self {
		Self { weight_bought_so_far: Weight::zero() }
	}

	fn buy_weight(
		&mut self,
		weight: Weight,
		payment: AssetsInHolding,
		_context: &XcmContext,
	) -> Result<AssetsInHolding, XcmError> {
		let amount = KusamaWeightToFee::weight_to_fee(&weight);
		let required: Asset = (Here, amount).into();
		let unused = payment.checked_sub(required).map_err(|_| XcmError::TooExpensive)?;
		self.weight_bought_so_far.saturating_add(weight);
		Ok(unused)
	}

	fn refund_weight(&mut self, weight: Weight, _context: &XcmContext) -> Option<Asset> {
		let weight = weight.min(self.weight_bought_so_far);
		let amount = KusamaWeightToFee::weight_to_fee(&weight);
		self.weight_bought_so_far -= weight;
		if amount > 0 {
			Some((Here, amount).into())
		} else {
			None
		}
	}
}

parameter_types! {
	pub XcmFeePot: AccountId = AccountId32::new([0u8; 32]);
}

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = TestMessageSender;
	type AssetTransactor = LocalAssetsTransactor;
	type OriginConverter = OriginConverter;
	type IsReserve = ();
	type IsTeleporter = ();
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<BaseXcmWeight, RuntimeCall, MaxInstructions>;
	type Trader = UsingComponents<
		system_parachains_constants::kusama::fee::WeightToFee,
		KsmLocation,
		AccountId,
		Balances,
		ResolveTo<XcmFeePot, Balances>,
	>;
	type ResponseHandler = XcmPallet;
	type AssetTrap = XcmPallet;
	type AssetLocker = ();
	type AssetExchanger = ();
	type AssetClaims = XcmPallet;
	type SubscriptionService = XcmPallet;
	type PalletInstancesInfo = ();
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	type FeeManager = ();
	type MessageExporter = ();
	type UniversalAliases = Nothing;
	type CallDispatcher = RuntimeCall;
	type SafeCallFilter = Everything;
	type Aliasers = Nothing;
	type TransactionalProcessor = ();
	type HrmpNewChannelOpenRequestHandler = ();
	type HrmpChannelAcceptedHandler = ();
	type HrmpChannelClosingHandler = ();
	type XcmRecorder = XcmPallet;
	type XcmEventEmitter = XcmPallet;
}

parameter_types! {
	pub TreasuryAccountId: AccountId = AccountId::new([42u8; 32]);
}

pub struct TreasuryToAccount;
impl ConvertLocation<AccountId> for TreasuryToAccount {
	fn convert_location(location: &Location) -> Option<AccountId> {
		match location.unpack() {
			(1, [Parachain(42), Plurality { id: BodyId::Treasury, part: BodyPart::Voice }]) =>
				Some(TreasuryAccountId::get()), // Hardcoded test treasury account id
			_ => None,
		}
	}
}

pub(crate) type SovereignAccountOf = (
	AccountId32Aliases<AnyNetwork, AccountId>,
	TreasuryToAccount,
	HashedDescription<AccountId, DescribeFamily<DescribeAllTerminal>>,
);

impl pallet_xcm::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmRouter = TestMessageSender;
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Everything;
	type XcmReserveTransferFilter = Everything;
	type Weigher = FixedWeightBounds<BaseXcmWeight, RuntimeCall, MaxInstructions>;
	type UniversalLocation = UniversalLocation;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
	type TrustedLockers = ();
	type SovereignAccountOf = SovereignAccountOf;
	type Currency = Balances;
	type CurrencyMatcher = IsConcrete<RelayLocation>;
	type MaxLockers = frame_support::traits::ConstU32<8>;
	type MaxRemoteLockConsumers = frame_support::traits::ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
	type WeightInfo = pallet_xcm::TestWeightInfo;
	type AdminOrigin = EnsureRoot<AccountId>;
	type AuthorizedAliasConsideration = Disabled;
}

pub const UNITS: Balance = 1_000_000_000_000;
pub const INITIAL_BALANCE: Balance = 100 * UNITS;
pub const MINIMUM_BALANCE: Balance = UNITS;

pub fn sibling_chain_account_id(para_id: u32, account: [u8; 32]) -> AccountId {
	let location: Location =
		(Parent, Parachain(para_id), Junction::AccountId32 { id: account, network: None }).into();
	SovereignAccountOf::convert_location(&location).unwrap()
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	let admin_account: AccountId = AccountId::new([0u8; 32]);
	pallet_assets::GenesisConfig::<Test> {
		assets: vec![
			(0, admin_account.clone(), true, MINIMUM_BALANCE),
			(1, admin_account.clone(), true, MINIMUM_BALANCE),
			(100, admin_account.clone(), true, MINIMUM_BALANCE),
		],
		metadata: vec![
			(0, "Native token".encode(), "NTV".encode(), 12),
			(1, "Relay token".encode(), "RLY".encode(), 12),
			(100, "Test token".encode(), "TST".encode(), 12),
		],
		accounts: vec![
			(0, sibling_chain_account_id(42, [3u8; 32]), INITIAL_BALANCE),
			(1, TreasuryAccountId::get(), INITIAL_BALANCE),
			(100, TreasuryAccountId::get(), INITIAL_BALANCE),
		],
		next_asset_id: None,
	}
	.assimilate_storage(&mut t)
	.unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub fn next_block() {
	System::set_block_number(System::block_number() + 1);
}

pub fn run_to(block_number: BlockNumber) {
	while System::block_number() < block_number {
		next_block();
	}
}
