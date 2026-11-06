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
use frame_support::traits::Nothing;
use sp_runtime::FixedU128;

parameter_types! {
	pub const PoolsPalletId: PalletId = PalletId(*b"py/nopls");
	// Allow pools that got slashed up to 90% to remain operational.
	pub const MaxPointsToBalance: u8 = 10;
	/// Bound for `pallet_nomination_pools::SubPools::with_era`, kept at its historical maximum of
	/// `BondingDuration + 4 = 32`.
	///
	/// Since polkadot-sdk#12323 the pallet takes this bound directly instead of deriving it from
	/// `nominator_bonding_duration()`, so it no longer shrinks (32 -> 6) when `AreNominatorsSlashable`
	/// is set to `false` (bugbounty report #90). After that flip the merge window is `32 - 2 = 30`
	/// eras, long enough for deferred pre-flip slashes (`SlashDeferDuration = 27`) to still apply.
	pub MaxUnbondingPools: u32 = BondingDuration::get().saturating_add(4);
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
	type MaxUnbondingPools = MaxUnbondingPools;
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
