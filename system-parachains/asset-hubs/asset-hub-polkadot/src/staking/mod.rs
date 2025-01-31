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

//! Staking related config of the Asset Hub.
//!
//! The large pallets have their config in a sub-module, the smaller ones are defined here.

pub mod bags_thresholds;
pub mod nom_pools;

use crate::*;

parameter_types! {
	// 1% of the Relay Chain's deposit
	pub const FastUnstakeDeposit: Balance = UNITS / 100;
}

impl pallet_fast_unstake::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BatchSize = frame_support::traits::ConstU32<16>;
	type Deposit = FastUnstakeDeposit;
	type ControlOrigin = EnsureRoot<AccountId>;
	type Staking = nom_pools::StakingMock;
	type MaxErasToCheckPerBlock = ConstU32<1>;
	type WeightInfo = (); // TODO weights::pallet_fast_unstake::WeightInfo<Runtime>;
}

parameter_types! {
	pub const BagThresholds: &'static [u64] = &bags_thresholds::THRESHOLDS;
}

type VoterBagsListInstance = pallet_bags_list::Instance1;
impl pallet_bags_list::Config<VoterBagsListInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ScoreProvider = nom_pools::StakingMock;
	type WeightInfo = (); // TODO weights::pallet_bags_list::WeightInfo<Runtime>;
	type BagThresholds = BagThresholds;
	type Score = sp_npos_elections::VoteWeight;
}
