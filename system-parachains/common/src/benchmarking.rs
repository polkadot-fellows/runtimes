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
// along with Polkadot. If not, see <http://www.gnu.org/licenses/>.

//! Setup helpers shared by the XCM benchmarks of the system-parachain runtimes.

use alloc::boxed::Box;
use codec::{Encode, MaxEncodedLen};
use frame_support::{
	assert_ok,
	traits::{Currency, Get},
};
use frame_system::RawOrigin;
use xcm::latest::{Junction::AccountId32, Location, NetworkId};

/// Returns a maximum-encoded-length aliaser. Aliasers differ only at the final junction so failed
/// comparisons traverse the entire location. Using 255 parents also avoids the cheaper alias
/// filters configured before `AuthorizedAliasers` in system parachains.
fn max_sized_aliaser(discriminator: [u8; 32]) -> Location {
	let network = Some(NetworkId::ByFork { block_number: u64::MAX, block_hash: [0xFF; 32] });
	let shared = AccountId32 { network, id: [0xFF; 32] };
	let last = AccountId32 { network, id: discriminator };
	let aliaser =
		Location::new(255, [shared, shared, shared, shared, shared, shared, shared, last]);

	assert_eq!(
		aliaser.encoded_size(),
		Location::max_encoded_len(),
		"aliaser is not the largest encodable `Location`",
	);

	aliaser
}

/// Sets up a worst-case `(origin, target)` for `AliasOrigin` in system parachain runtimes.
///
/// The target authorizes the maximum number of maximum-sized aliasers. The matching aliaser is
/// last and has an expiry, forcing the full list to be decoded, converted, and searched.
///
/// Panics on setup failure because the benchmark maps errors to `BenchmarkError::Skip`.
pub fn set_up_worst_case_authorized_alias<Runtime>() -> (Location, Location)
where
	Runtime: pallet_xcm::Config + pallet_balances::Config,
	<Runtime as frame_system::Config>::AccountId: From<[u8; 32]>,
{
	let target_id = [42u8; 32];
	let target_account: <Runtime as frame_system::Config>::AccountId = target_id.into();
	let target = Location::new(0, [AccountId32 { id: target_id, network: None }]);

	let balance =
		<Runtime as pallet_balances::Config>::ExistentialDeposit::get() * 1_000_000u32.into();
	let _ = <pallet_balances::Pallet<Runtime> as Currency<_>>::make_free_balance_be(
		&target_account,
		balance,
	);
	let target_origin: <Runtime as frame_system::Config>::RuntimeOrigin =
		RawOrigin::Signed(target_account).into();

	let origin = max_sized_aliaser([170u8; 32]);

	for index in 1..pallet_xcm::MaxAuthorizedAliases::get() {
		let mut id = [0u8; 32];
		id[..4].copy_from_slice(&index.to_le_bytes());
		let filler = max_sized_aliaser(id);
		assert_ok!(pallet_xcm::Pallet::<Runtime>::add_authorized_alias(
			target_origin.clone(),
			Box::new(filler.into()),
			Some(u64::MAX),
		));
	}
	assert_ok!(pallet_xcm::Pallet::<Runtime>::add_authorized_alias(
		target_origin,
		Box::new(origin.clone().into()),
		Some(u64::MAX),
	));

	(origin, target)
}
