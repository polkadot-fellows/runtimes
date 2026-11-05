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

use crate::{
	staking::{BondingDuration, StakingAdmin},
	*,
};
use frame_support::traits::{Get, Nothing};
use sp_runtime::FixedU128;

parameter_types! {
	pub const PoolsPalletId: PalletId = PalletId(*b"py/nopls");
	// Allow pools that got slashed up to 90% to remain operational.
	pub const MaxPointsToBalance: u8 = 10;
}

/// Bound for `pallet_nomination_pools::SubPools::with_era`, frozen at its historical maximum so it
/// does not shrink when the nominator bonding duration drops.
///
/// The pallet derives its `TotalUnbondingPools` bound as `StakeAdapter::bonding_duration() +
/// PostUnbondingPoolsWindow`, and `StakeAdapter::bonding_duration()` resolves to
/// `StakingInterface::nominator_bonding_duration()`, which drops from `BondingDuration` (28) to
/// `NominatorFastUnbondDuration` (2) the moment `AreNominatorsSlashable` is set from `true` to
/// `false`. With a fixed window the bound would shrink 32 -> 6, making any oversized historical
/// `with_era` map undecodable; the next `unbond` then overwrites it with an empty `Default`,
/// destroying per-era unbonding accounting (bugbounty report #90).
///
/// We pin the bound at `BondingDuration + 4 = 32` by computing the window as `MAX -
/// nominator_bonding_duration()`. Subtracting the same value the pallet adds keeps
/// `TotalUnbondingPools == MAX` by construction, regardless of the flag. Post-flip this also widens
/// the effective merge window to `32 - 2 = 30` eras, keeping each per-era pool on its correct
/// points-to-balance ratio long enough for deferred pre-flip slashes (`SlashDeferDuration = 27`) to
/// still be applied.
///
/// TODO: this is an interim runtime fix, meant to be superseded by the in-pallet decoupling in
/// polkadot-sdk.
pub struct PostUnbondingPoolsWindow;
impl Get<u32> for PostUnbondingPoolsWindow {
	fn get() -> u32 {
		// Historical maximum number of unbonding sub-pools: the validator bonding duration plus the
		// legacy 4-era post-unbonding window. `BondingDuration` is the upper bound of
		// `nominator_bonding_duration()`, so the subtraction below never saturates.
		let max_unbonding_pools = BondingDuration::get().saturating_add(4);
		max_unbonding_pools
			.saturating_sub(<Staking as sp_staking::StakingInterface>::nominator_bonding_duration())
	}
}

impl pallet_nomination_pools::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type RewardCounter = FixedU128;
	type BalanceToU256 = polkadot_runtime_common::BalanceToU256;
	type U256ToBalance = polkadot_runtime_common::U256ToBalance;
	type StakeAdapter =
		pallet_nomination_pools::adapter::DelegateStake<Self, Staking, DelegatedStaking>;
	type PostUnbondingPoolsWindow = PostUnbondingPoolsWindow;
	type MaxMetadataLen = frame_support::traits::ConstU32<256>;
	// we use the same number of allowed unlocking chunks as with staking.
	type MaxUnbonding = <Self as pallet_staking_async::Config>::MaxUnlockingChunks;
	type PalletId = PoolsPalletId;
	type MaxPointsToBalance = MaxPointsToBalance;
	type WeightInfo = (); // Have to use stock weights since nom-pools is not benchmarkable with pallet-staking-async.
	type AdminOrigin = EitherOf<EnsureRoot<AccountId>, StakingAdmin>;
	type Filter = Nothing;
	type BlockNumberProvider = RelaychainDataProvider<Runtime>;
}
