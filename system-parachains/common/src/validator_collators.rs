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

//! Helpers for the chains where validators collate through `pallet-validator-collators`.

use core::marker::PhantomData;
use frame_support::traits::Get;
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_validator_collators::MaxCollators;

/// The `KickThreshold` of `pallet-collator-selection` on a chain where validators collate.
///
/// With the cap `None` it is `SessionLength`, so collator selection kicks as it did before
/// validators could collate. With a cap, it is `parachains_common::impls::KickThreshold` for
/// [`CollatorCap`] extra collators, so it grows with the cap without a runtime upgrade.
///
/// Governance is expected to set `UpTo(n)` with n at most the active validator set. A stored set
/// smaller than the cap only lengthens the threshold, which is safe. Lowering the cap shortens it
/// at once, while the previously drawn validators still take their turns for up to two sessions.
pub struct KickThreshold<Runtime, BlocksPerAuraSlot, SessionLength>(
	PhantomData<(Runtime, BlocksPerAuraSlot, SessionLength)>,
);

impl<Runtime, BlocksPerAuraSlot, SessionLength> Get<BlockNumberFor<Runtime>>
	for KickThreshold<Runtime, BlocksPerAuraSlot, SessionLength>
where
	Runtime: pallet_validator_collators::Config + pallet_collator_selection::Config,
	BlocksPerAuraSlot: Get<u32>,
	SessionLength: Get<BlockNumberFor<Runtime>>,
{
	fn get() -> BlockNumberFor<Runtime> {
		match CollatorCap::<Runtime>::get() {
			0 => SessionLength::get(),
			_ => parachains_common::impls::KickThreshold::<
				Runtime,
				CollatorCap<Runtime>,
				BlocksPerAuraSlot,
				SessionLength,
			>::get(),
		}
	}
}

/// The cap stored in `pallet-validator-collators`, at most
/// [`pallet_validator_collators::Config::MaxValidators`].
pub struct CollatorCap<Runtime>(PhantomData<Runtime>);

impl<Runtime: pallet_validator_collators::Config> Get<u32> for CollatorCap<Runtime> {
	fn get() -> u32 {
		MaxCollators::<Runtime>::get()
			.limit()
			.min(<Runtime as pallet_validator_collators::Config>::MaxValidators::get())
	}
}
