// Copyright (C) Parity Technologies and the various Polkadot contributors, see Contributions.md
// for a list of specific contributors.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{self as pallet_kill_switch, *};
use frame_support::{
	derive_impl, ord_parameter_types, parameter_types,
	traits::{
		tokens::imbalance::ResolveTo, ConstU32, ConstU64, Contains, Everything, InsideBoth,
		NeverEnsureOrigin,
	},
};
use frame_system::{EnsureRoot, EnsureSignedBy};
use pallet_tx_pause::RuntimeCallNameOf;
use sp_runtime::BuildStorage;

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		TxPause: pallet_tx_pause,
		KillSwitch: pallet_kill_switch,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type BaseCallFilter = InsideBoth<Everything, TxPause>;
	type Block = Block;
	type AccountData = pallet_balances::AccountData<u64>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type AccountStore = System;
}

/// Calls tx-pause refuses to pause even if they have a price.
pub struct NeverPausable;
impl Contains<RuntimeCallNameOf<Test>> for NeverPausable {
	fn contains(name: &RuntimeCallNameOf<Test>) -> bool {
		matches!(
			(name.0.as_slice(), name.1.as_slice()),
			(b"System", _) | (b"KillSwitch", _) | (b"Balances", b"transfer_keep_alive")
		)
	}
}

impl pallet_tx_pause::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	// Only the kill switch touches tx-pause.
	type PauseOrigin = NeverEnsureOrigin<()>;
	type UnpauseOrigin = NeverEnsureOrigin<()>;
	type WhitelistedCalls = NeverPausable;
	type MaxNameLen = ConstU32<256>;
	type WeightInfo = ();
}

pub const DEPOSIT: u64 = 100;
pub const TREASURY: u64 = 1000;
pub const PAUSE_DURATION: u64 = 10;
pub const RELEASE_DELAY: u64 = 100;
pub const COOLDOWN: u64 = 20;
pub const MAX_FELLOWSHIP_EXTENSION: u64 = 15;
pub const MAX_DEPOSITS: u32 = 3;
pub const MAX_POLL_STEPS: u32 = 5;

/// `Balances` calls cost [`DEPOSIT`], everything else can't be paused.
pub struct DepositFor;
impl Convert<RuntimeCallNameOf<Test>, Option<u64>> for DepositFor {
	fn convert(name: RuntimeCallNameOf<Test>) -> Option<u64> {
		(name.0.as_slice() == b"Balances").then_some(DEPOSIT)
	}
}

parameter_types! {
	pub static RelayBlock: u64 = 1;
}

/// Stands in for the relay chain block number.
pub struct MockRelay;
impl BlockNumberProvider for MockRelay {
	type BlockNumber = u64;

	fn current_block_number() -> u64 {
		RelayBlock::get()
	}

	fn set_block_number(n: u64) {
		RelayBlock::set(n);
	}
}

ord_parameter_types! {
	pub const Fellowship: u64 = 99;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct BenchmarkHelper;
#[cfg(feature = "runtime-benchmarks")]
impl crate::BenchmarkHelper<RuntimeCallNameOf<Test>> for BenchmarkHelper {
	fn target() -> RuntimeCallNameOf<Test> {
		full_name("Balances", "transfer_allow_death")
	}
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Target = RuntimeCallNameOf<Test>;
	type Pauser = TxPause;
	type DepositFor = DepositFor;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type OnSlash = ResolveTo<ConstU64<TREASURY>, Balances>;
	type FellowshipOrigin = EnsureSignedBy<Fellowship, u64>;
	type ForceOrigin = EnsureRoot<u64>;
	type BlockNumberProvider = MockRelay;
	type PauseDuration = ConstU64<PAUSE_DURATION>;
	type ReleaseDelay = ConstU64<RELEASE_DELAY>;
	type Cooldown = ConstU64<COOLDOWN>;
	type MaxFellowshipExtension = ConstU64<MAX_FELLOWSHIP_EXTENSION>;
	type MaxDepositsPerPause = ConstU32<MAX_DEPOSITS>;
	type MaxPollSteps = ConstU32<MAX_POLL_STEPS>;
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = BenchmarkHelper;
}

pub fn full_name(pallet: &str, call: &str) -> RuntimeCallNameOf<Test> {
	(pallet.as_bytes().to_vec().try_into().unwrap(), call.as_bytes().to_vec().try_into().unwrap())
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pallet_balances::GenesisConfig::<Test> {
		balances: (1..=5).map(|who| (who, 1_000)).collect(),
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
