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

//! Config for the nomination pools.

use crate::*;
use pallet_nomination_pools::{adapter::*, BondType};
use sp_runtime::{DispatchError, DispatchResult, FixedU128};
use sp_staking::{EraIndex, Stake};

parameter_types! {
	pub const PoolsPalletId: PalletId = PalletId(*b"py/nopls");
	// Allow pools that got slashed up to 90% to remain operational.
	pub const MaxPointsToBalance: u8 = 10;
}

impl pallet_nomination_pools::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type RewardCounter = FixedU128;
	type BalanceToU256 = polkadot_runtime_common::BalanceToU256;
	type U256ToBalance = polkadot_runtime_common::U256ToBalance;
	type StakeAdapter = MockStakeAdapter; // FAIL-CI pallet_nomination_pools::adapter::DelegateStake<Self, Staking, DelegatedStaking>;
	type PostUnbondingPoolsWindow = frame_support::traits::ConstU32<4>;
	type MaxMetadataLen = frame_support::traits::ConstU32<256>;
	// we use the same number of allowed unlocking chunks as with staking.
	type MaxUnbonding = ConstU32<10>; // FAIL-CI <Self as pallet_staking::Config>::MaxUnlockingChunks;
	type PalletId = PoolsPalletId;
	type MaxPointsToBalance = MaxPointsToBalance;
	type WeightInfo = (); // FAIL-CI weights::pallet_nomination_pools::WeightInfo<Self>;
	type AdminOrigin = EnsureRoot<AccountId>; // FAIL-CI EitherOf<EnsureRoot<AccountId>, StakingAdmin>;
}

pub struct MockStakeAdapter;
impl StakeStrategy for MockStakeAdapter {
	type Balance = crate::Balance;
	type AccountId = <Runtime as frame_system::Config>::AccountId;
	type CoreStaking = StakingMock;

	fn strategy_type() -> StakeStrategyType {
		unimplemented!()
	}

	fn transferable_balance(
		pool_account: Pool<Self::AccountId>,
		member_account: Member<Self::AccountId>,
	) -> Self::Balance {
		unimplemented!()
	}

	fn total_balance(pool_account: Pool<Self::AccountId>) -> Option<Self::Balance> {
		unimplemented!()
	}

	fn member_delegation_balance(member_account: Member<Self::AccountId>) -> Option<Self::Balance> {
		unimplemented!()
	}

	fn pledge_bond(
		who: Member<Self::AccountId>,
		pool_account: Pool<Self::AccountId>,
		reward_account: &Self::AccountId,
		amount: Self::Balance,
		bond_type: BondType,
	) -> DispatchResult {
		unimplemented!()
	}

	fn member_withdraw(
		who: Member<Self::AccountId>,
		pool_account: Pool<Self::AccountId>,
		amount: Self::Balance,
		num_slashing_spans: u32,
	) -> DispatchResult {
		unimplemented!()
	}

	fn dissolve(pool_account: Pool<Self::AccountId>) -> DispatchResult {
		unimplemented!()
	}

	fn pending_slash(pool_account: Pool<Self::AccountId>) -> Self::Balance {
		unimplemented!()
	}

	fn member_slash(
		who: Member<Self::AccountId>,
		pool_account: Pool<Self::AccountId>,
		amount: Self::Balance,
		maybe_reporter: Option<Self::AccountId>,
	) -> DispatchResult {
		unimplemented!()
	}

	fn migrate_nominator_to_agent(
		agent: Pool<Self::AccountId>,
		reward_account: &Self::AccountId,
	) -> DispatchResult {
		unimplemented!()
	}

	fn migrate_delegation(
		agent: Pool<Self::AccountId>,
		delegator: Member<Self::AccountId>,
		value: Self::Balance,
	) -> DispatchResult {
		unimplemented!()
	}
}

pub struct StakingMock;

impl sp_staking::StakingInterface for StakingMock {
	type Balance = crate::Balance;
	type AccountId = <Runtime as frame_system::Config>::AccountId;
	type CurrencyToVote = sp_staking::currency_to_vote::U128CurrencyToVote;

	fn minimum_nominator_bond() -> Self::Balance {
		unimplemented!()
	}
	fn minimum_validator_bond() -> Self::Balance {
		unimplemented!()
	}

	fn desired_validator_count() -> u32 {
		unimplemented!()
	}

	fn current_era() -> EraIndex {
		unimplemented!()
	}

	fn bonding_duration() -> EraIndex {
		unimplemented!()
	}

	fn status(
		_: &Self::AccountId,
	) -> Result<sp_staking::StakerStatus<Self::AccountId>, DispatchError> {
		unimplemented!()
	}

	fn is_virtual_staker(who: &Self::AccountId) -> bool {
		unimplemented!()
	}

	fn bond_extra(who: &Self::AccountId, extra: Self::Balance) -> DispatchResult {
		unimplemented!()
	}

	fn unbond(who: &Self::AccountId, amount: Self::Balance) -> DispatchResult {
		unimplemented!()
	}

	fn set_payee(_stash: &Self::AccountId, _reward_acc: &Self::AccountId) -> DispatchResult {
		unimplemented!()
	}

	fn chill(_: &Self::AccountId) -> sp_runtime::DispatchResult {
		unimplemented!()
	}

	fn withdraw_unbonded(who: Self::AccountId, _: u32) -> Result<bool, DispatchError> {
		unimplemented!()
	}

	fn bond(stash: &Self::AccountId, value: Self::Balance, _: &Self::AccountId) -> DispatchResult {
		unimplemented!()
	}

	fn nominate(_: &Self::AccountId, nominations: Vec<Self::AccountId>) -> DispatchResult {
		unimplemented!()
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn nominations(_: &Self::AccountId) -> Option<Vec<Self::AccountId>> {
		unimplemented!()
	}

	fn stash_by_ctrl(_controller: &Self::AccountId) -> Result<Self::AccountId, DispatchError> {
		unimplemented!()
	}

	fn stake(who: &Self::AccountId) -> Result<Stake<Balance>, DispatchError> {
		unimplemented!()
	}

	fn election_ongoing() -> bool {
		unimplemented!()
	}

	fn force_unstake(_who: Self::AccountId) -> sp_runtime::DispatchResult {
		unimplemented!()
	}

	fn is_exposed_in_era(_who: &Self::AccountId, _era: &EraIndex) -> bool {
		unimplemented!()
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn add_era_stakers(
		_current_era: &EraIndex,
		_stash: &Self::AccountId,
		_exposures: Vec<(Self::AccountId, Self::Balance)>,
	) {
		unimplemented!()
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn set_current_era(_era: EraIndex) {
		unimplemented!()
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn max_exposure_page_size() -> sp_staking::Page {
		unimplemented!()
	}

	fn slash_reward_fraction() -> Perbill {
		unimplemented!()
	}
}
