// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Cumulus.
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

use crate as pallet_ah_ops;
use crate::*;
use frame_election_provider_support::BoundedSupportsOf;
use frame_support::derive_impl;
use frame_system::{EnsureRoot, EnsureSigned};
use pallet_election_provider_multi_block::PageIndex;
use sp_core::H256;
use sp_runtime::traits::{parameter_types, BlakeTwo256, IdentityLookup};

type Block = frame_system::mocking::MockBlock<Runtime>;

// For testing the pallet, we construct a mock runtime.
frame_support::construct_runtime!(
	pub enum Runtime {
		System: frame_system,
		Assets: pallet_assets,
		Balances: pallet_balances,
		AhOps: pallet_ah_ops,
		Timestamp: pallet_timestamp,
		Staking: pallet_staking_async,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Runtime {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type Nonce = u64;
	type Hash = H256;
	type RuntimeCall = RuntimeCall;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u128>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Runtime {
	type Balance = u128;
	type AccountStore = System;
}

#[derive_impl(pallet_assets::config_preludes::TestDefaultConfig)]
impl pallet_assets::Config for Runtime {
	type Balance = u128;
	type Currency = Balances;
	type CreateOrigin = EnsureSigned<Self::AccountId>;
	type ForceOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type Holder = ();
	type Freezer = ();
}

impl pallet_timestamp::Config for Runtime {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ();
	type WeightInfo = ();
}

impl pallet_staking_async::Config for Runtime {
	type Filter = ();
	type OldCurrency = Balances;
	type Currency = Balances;
	type CurrencyBalance = u128;
	type RuntimeHoldReason = RuntimeHoldReason;
	type CurrencyToVote = sp_staking::currency_to_vote::SaturatingCurrencyToVote;
	type RewardRemainder = ();
	type Slash = ();
	type Reward = ();
	type SessionsPerEra = ();
	type BondingDuration = ();
	type SlashDeferDuration = ();
	type AdminOrigin = frame_system::EnsureNone<Self::AccountId>;
	type EraPayout = ();
	type MaxExposurePageSize = ();
	type ElectionProvider = Self;
	type VoterList = pallet_staking_async::UseNominatorsAndValidatorsMap<Self>;
	type TargetList = Self;
	type MaxValidatorSet = ();
	type NominationsQuota = pallet_staking_async::FixedNominationsQuota<100>;
	type MaxUnlockingChunks = ();
	type HistoryDepth = ();
	type MaxControllersInDeprecationBatch = ();
	type EventListeners = ();
	type MaxInvulnerables = ();
	type PlanningEraOffset = ();
	type RcClientInterface = Self;
	type MaxEraDuration = ();
	type MaxPruningItems = ConstU32<100>;
	type WeightInfo = ();
}

impl pallet_staking_async_rc_client::RcClientInterface for Runtime {
	type AccountId = AccountId32;

	fn validator_set(new_validator_set: Vec<Self::AccountId>, id: u32, _prune_up_tp: Option<u32>) {
		unimplemented!()
	}
}

impl frame_election_provider_support::SortedListProvider<AccountId32> for Runtime {
	type Error = &'static str;
	type Score = u128;

	fn iter() -> Box<dyn Iterator<Item = AccountId32>> {
		unimplemented!()
	}

	fn lock() {
		unimplemented!()
	}

	fn unlock() {
		unimplemented!()
	}

	fn iter_from(
		_start: &AccountId32,
	) -> Result<Box<dyn Iterator<Item = AccountId32>>, Self::Error> {
		unimplemented!()
	}

	fn count() -> u32 {
		unimplemented!()
	}

	fn contains(_id: &AccountId32) -> bool {
		unimplemented!()
	}

	fn on_insert(_id: AccountId32, _score: u128) -> Result<(), Self::Error> {
		unimplemented!()
	}

	fn on_update(_id: &AccountId32, _score: u128) -> Result<(), Self::Error> {
		unimplemented!()
	}

	fn get_score(_id: &AccountId32) -> Result<u128, Self::Error> {
		unimplemented!()
	}

	fn on_increase(_id: &AccountId32, _additional: u128) -> Result<(), Self::Error> {
		unimplemented!()
	}

	fn on_remove(_id: &AccountId32) -> Result<(), Self::Error> {
		unimplemented!()
	}

	fn unsafe_regenerate(
		_all: impl IntoIterator<Item = AccountId32>,
		_score_of: Box<dyn Fn(&AccountId32) -> Option<u128>>,
	) -> u32 {
		unimplemented!()
	}

	fn unsafe_clear() {
		unimplemented!()
	}

	#[cfg(feature = "try-runtime")]
	fn try_state() -> Result<(), sp_runtime::TryRuntimeError> {
		unimplemented!()
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn score_update_worst_case(_who: &AccountId32, _is_increase: bool) -> Self::Score {
		unimplemented!()
	}
}

impl frame_election_provider_support::ElectionProvider for Runtime {
	type AccountId = AccountId32;
	type BlockNumber = u64;
	type Error = &'static str;
	type MaxWinnersPerPage = ();
	type MaxBackersPerWinner = ();
	type MaxBackersPerWinnerFinal = ();
	type Pages = ConstU32<1>;
	type DataProvider = Staking;

	fn elect(_remaining: PageIndex) -> Result<BoundedSupportsOf<Self>, Self::Error> {
		unimplemented!()
	}

	fn duration() -> Self::BlockNumber {
		0
	}

	fn start() -> Result<(), Self::Error> {
		Ok(())
	}

	fn status() -> Result<bool, ()> {
		Ok(true)
	}
}

parameter_types! {
	pub const MigrationCompletion: bool = true;
	pub TreasuryPreMigrationAccount: AccountId32 = AccountId32::from([1; 32]);
	pub TreasuryPostMigrationAccount: AccountId32 = AccountId32::from([2; 32]);
}

impl Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type Fungibles = Assets;
	type RcBlockNumberProvider = System; // Wrong but unused
	type MigrateOrigin = EnsureRoot<AccountId32>;
	type RelevantAssets = ();
	type AssetId = u32;
	type WeightInfo = ();
	type MigrationCompletion = MigrationCompletion;
	type TreasuryPreMigrationAccount = TreasuryPreMigrationAccount;
	type TreasuryPostMigrationAccount = TreasuryPostMigrationAccount;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	use sp_runtime::BuildStorage;
	let t = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();
	sp_io::TestExternalities::new(t)
}

pub fn assert_last_event<T: Config>(generic_event: impl Into<<T as Config>::RuntimeEvent>) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into().into());
}
