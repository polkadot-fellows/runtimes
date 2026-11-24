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
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::{
	derive_impl, ord_parameter_types, parameter_types,
	traits::{
		tokens::imbalance::ResolveTo, ConstU32, ConstU64, Contains, EnsureOrigin,
		EnsureOriginWithArg, Everything, GetCallMetadata, InsideBoth, NeverEnsureOrigin,
	},
};
use frame_system::{EnsureRoot, EnsureSignedBy};
use pallet_tx_pause::RuntimeCallNameOf;
use scale_info::TypeInfo;
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
pub const COOLDOWN: u64 = 20;
pub const MAX_DEPOSITS: u32 = 3;
pub const MAX_CALLS: u32 = 2;
pub const CALL_DEPOSIT: u64 = 50;
/// Seven groups, so two slots are left for single calls.
pub const MAX_ACTIVE_PAUSES: u32 = 9;

/// The groups the tests can pause.
#[derive(
	Clone,
	Copy,
	PartialEq,
	Eq,
	Debug,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	MaxEncodedLen,
)]
pub enum Group {
	Transfer,
	ForceTransfer,
	TransferAll,
	Burn,
	ForceUnreserve,
	/// Shares its calls with `Transfer` and `Burn`.
	Money,
	/// Has no deposit, so only Root can pause it.
	RootOnly,
	/// Not one of the groups.
	Unknown,
	/// A call tx-pause refuses to pause.
	Whitelisted,
	/// The kill switch itself.
	KillSwitchCall,
}

pub struct Groups;
impl PauseGroups<Group, RuntimeCallNameOf<Test>, u64> for Groups {
	fn all() -> Vec<Group> {
		use Group::*;
		vec![Transfer, ForceTransfer, TransferAll, Burn, ForceUnreserve, Money, RootOnly]
	}

	fn calls(group: &Group) -> Vec<RuntimeCallNameOf<Test>> {
		let balances =
			|calls: &[&str]| calls.iter().map(|call| full_name("Balances", call)).collect();
		match group {
			Group::Transfer => balances(&["transfer_allow_death"]),
			Group::ForceTransfer => balances(&["force_transfer"]),
			Group::TransferAll => balances(&["transfer_all"]),
			Group::Burn => balances(&["burn"]),
			Group::ForceUnreserve => balances(&["force_unreserve"]),
			// A runtime upgrade can change what a group pauses.
			Group::Money if MoneyUpgraded::get() =>
				balances(&["transfer_allow_death", "force_transfer"]),
			Group::Money => balances(&["transfer_allow_death", "burn"]),
			Group::RootOnly => balances(&["force_set_balance"]),
			Group::Unknown => vec![],
			Group::Whitelisted => balances(&["transfer_keep_alive"]),
			Group::KillSwitchCall => vec![full_name("KillSwitch", "unpause")],
		}
	}

	fn deposit(group: &Group) -> Option<u64> {
		(*group != Group::RootOnly).then_some(GroupDeposit::get())
	}
}

/// Every call that exists in the runtime.
pub struct ExistingCall;
impl Contains<RuntimeCallNameOf<Test>> for ExistingCall {
	fn contains((pallet, call): &RuntimeCallNameOf<Test>) -> bool {
		let (Ok(pallet), Ok(call)) = (core::str::from_utf8(pallet), core::str::from_utf8(call))
		else {
			return false;
		};
		// `get_call_names` panics for a pallet that doesn't exist.
		RuntimeCall::get_module_names().contains(&pallet) &&
			RuntimeCall::get_call_names(pallet).contains(&call)
	}
}

parameter_types! {
	pub static RelayBlock: u64 = 1;
	// Like parameters, governance can change these or set them to zero.
	pub static GroupDeposit: u64 = DEPOSIT;
	pub static CallDepositAmount: u64 = CALL_DEPOSIT;
	pub static PauseLength: u64 = PAUSE_DURATION;
	pub static CooldownLength: u64 = COOLDOWN;
	pub static MoneyUpgraded: bool = false;
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

/// Root can do everything, the Fellowship everything but force pausing.
pub struct AdminOrigin;
impl EnsureOriginWithArg<RuntimeOrigin, AdminAction<TargetOf<Test>>> for AdminOrigin {
	type Success = ();

	fn try_origin(
		o: RuntimeOrigin,
		action: &AdminAction<TargetOf<Test>>,
	) -> Result<(), RuntimeOrigin> {
		let o = match <EnsureRoot<u64> as EnsureOrigin<_>>::try_origin(o) {
			Ok(_) => return Ok(()),
			Err(o) => o,
		};
		match action {
			AdminAction::ForcePause(_) => Err(o),
			AdminAction::Unpause(_) | AdminAction::Extend(_) | AdminAction::Settle(_) =>
				<EnsureSignedBy<Fellowship, u64> as EnsureOrigin<_>>::try_origin(o).map(|_| ()),
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin(_: &AdminAction<TargetOf<Test>>) -> Result<RuntimeOrigin, ()> {
		Ok(RuntimeOrigin::root())
	}
}

#[cfg(feature = "runtime-benchmarks")]
pub struct BenchmarkHelper;
#[cfg(feature = "runtime-benchmarks")]
impl crate::BenchmarkHelper<Group> for BenchmarkHelper {
	fn group() -> Group {
		Group::Money
	}
}

impl Config for Test {
	type Group = Group;
	type CallName = RuntimeCallNameOf<Test>;
	type Groups = Groups;
	type ValidCall = ExistingCall;
	type CallDeposit = CallDepositAmount;
	type TransactionPause = TxPause;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type OnSlash = ResolveTo<ConstU64<TREASURY>, Balances>;
	type AdminOrigin = AdminOrigin;
	type BlockNumberProvider = MockRelay;
	type PauseDuration = PauseLength;
	type Cooldown = CooldownLength;
	type MaxCallsPerTarget = ConstU32<MAX_CALLS>;
	type MaxDepositsPerPause = ConstU32<MAX_DEPOSITS>;
	type MaxActivePauses = ConstU32<MAX_ACTIVE_PAUSES>;
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
