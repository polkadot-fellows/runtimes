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

//! Tests for the BEEFY ParaHeadsRootProvider and whitelisted parathreads.

use beefy_primitives::mmr::BeefyDataProvider;
use polkadot_primitives::{HeadData, Id as ParaId};
use polkadot_runtime::{BuildStorage, ParaHeadsRootProvider, Runtime};
use runtime_parachains::paras as parachains_paras;

#[test]
fn para_heads_root_provider_includes_whitelisted_parathreads() {
	let t = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();
	sp_io::TestExternalities::new(t).execute_with(|| {
		// Insert heads for paras 1, 2, 3367 (whitelisted parathread), 4000.
		let head = |i: u32| HeadData(vec![i as u8; 32]);
		for id in [1u32, 2, 3367, 4000] {
			parachains_paras::Heads::<Runtime>::insert(ParaId::from(id), head(id));
		}

		// Register 1, 2, 4000 as active parachains – 3367 is NOT in Parachains.
		parachains_paras::Parachains::<Runtime>::put(vec![
			ParaId::from(1u32),
			ParaId::from(2u32),
			ParaId::from(4000u32),
		]);
		let root_without_3367_in_parachains = ParaHeadsRootProvider::extra_data();

		// Now also add 3367 to the active parachains list (reordered to verify BTreeMap sorts).
		parachains_paras::Parachains::<Runtime>::put(vec![
			ParaId::from(4000u32),
			ParaId::from(3367u32),
			ParaId::from(2u32),
			ParaId::from(1u32),
		]);
		let root_with_3367_in_parachains = ParaHeadsRootProvider::extra_data();

		// Both roots must be identical: 3367 is included via the whitelist regardless
		// of whether it appears in the active parachains list. The BTreeMap deduplicates.
		assert_eq!(root_without_3367_in_parachains, root_with_3367_in_parachains);
	});
}
