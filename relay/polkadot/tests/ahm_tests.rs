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

//! Asset hub migration tests.

use frame_support::traits::tokens::ConversionFromAssetBalance;
use polkadot_runtime::AssetRateWithNative;
use polkadot_runtime_common::impls::VersionedLocatableAsset;
use xcm::prelude::*;
use remote_externalities::OfflineConfig;
use remote_externalities::Builder;
use polkadot_runtime::Block;
use remote_externalities::RemoteExternalities;
use remote_externalities::Mode; 
use polkadot_runtime::Runtime as T;
use polkadot_runtime::AhmController;
use polkadot_runtime::System;
use frame_support::sp_runtime::traits::Dispatchable;

#[tokio::test]
async fn ahm_indices_out() {
	let Some(mut ext) = remote_ext_test_setup().await else { return; };
	let mut calls = Vec::new();

	ext.execute_with(|| {
		frame_system::Pallet::<T>::set_block_number(1);
		let ti = pallet_balances::TotalIssuance::<T>::get();

		loop {
			let Some((call, weight)) = pallet_indices::Pallet::<T>::migrate_next(1000) else {
				break;
			};
			calls.push(call);

			/*log::error!("Number of events: {:?}", System::events().len());
			for event in System::events() {
				log::error!("Event: {:?}", event);
			}
			System::reset_events();*/
		}

		for call in calls {
			let runtime_call: polkadot_runtime::RuntimeCall = call.into();
			runtime_call.dispatch(frame_system::RawOrigin::Root.into()).unwrap();
		}

		let ti2 = pallet_balances::TotalIssuance::<T>::get();
		assert_eq!(ti, ti2, "Total issuance must be the same after migration");
	});	
}

async fn remote_ext_test_setup() -> Option<RemoteExternalities<Block>> {
	sp_tracing::try_init_simple();
	let Some(snap) = std::env::var("SNAP").ok() else{
		return None;
	};
	let abs = std::path::absolute(snap.clone());

	let ext = Builder::<Block>::default()
		.mode(Mode::Offline(
				OfflineConfig { state_snapshot: snap.clone().into() },
			))
		.build()
		.await
		.map_err(|e| {
			eprintln!("Could not load from snapshot: {:?}: {:?}", abs, e);
		})
		.unwrap();
	
	Some(ext)
}
