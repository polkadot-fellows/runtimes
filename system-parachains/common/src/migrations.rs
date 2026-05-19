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

//! Shared single-block migrations for system-parachain runtimes.

use frame_support::{traits::OnRuntimeUpgrade, weights::Weight};
use sp_runtime::traits::Get;

/// See: https://github.com/paritytech/polkadot-sdk/pull/10477#discussion_r3262614144
pub struct FixPoVMessagesTracker<DbWeight>(core::marker::PhantomData<DbWeight>);

impl<DbWeight> FixPoVMessagesTracker<DbWeight> {
	/// Storage key of `ParachainSystem::PoVMessagesTracker`.
	pub const fn storage_key() -> [u8; 32] {
		hex_literal::hex!("45323df7cc47150b3930e2666b0aa31322f3096ef79c4c691c3a9210667dbadc")
	}
}

impl<DbWeight: Get<frame_support::weights::RuntimeDbWeight>> OnRuntimeUpgrade
	for FixPoVMessagesTracker<DbWeight>
{
	fn on_runtime_upgrade() -> Weight {
		let key = Self::storage_key();
		if frame_support::storage::unhashed::get_raw(&key).map(|v| v.len()) == Some(42) {
			frame_support::storage::unhashed::kill(&key);
			log::info!(
				target: "runtime::parachain-system",
				"FixPoVMessagesTracker: cleared legacy PoVMessagesTracker value",
			);
		}
		DbWeight::get().reads_writes(1, 1)
	}
}
