// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! To run these benchmarks, you will need a modified version of `frame-omni-bencher` that can load
//! snapshots of the relay and asset hub. You can find it on branch `oty-ahm-omni-bencher` of the
//! SDK. Install it with
//! `cargo install --path substrate/utils/frame/omni-bencher --profile production`
//!
//! ```bash
//! frame-omni-bencher v1 benchmark pallet --runtime=target/release/wbuild/asset-hub-polkadot-runtime/asset_hub_polkadot_runtime.wasm --pallet "pallet-ah-migrator" --extrinsic "" --snap=ah-polkadot.snap --rc-snap=polkadot.snap
//! ```

use crate::*;
use core::str::FromStr;
use cumulus_primitives_core::{AggregateMessageOrigin, InboundDownwardMessage};
use frame_benchmarking::v2::*;
use frame_support::{traits::EnqueueMessage, weights::WeightMeter};
use frame_system::RawOrigin;
use pallet_rc_migrator::types::PalletMigration;
use xcm::VersionedXcm;

#[benchmarks(where T: pallet_balances::Config)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn receive_multisigs() {
		verify_snapshot::<T>();
		let (messages, _cursor) = relay_snapshot(|| {
			unwrap_no_debug(pallet_rc_migrator::multisig::MultisigMigrator::<T>::migrate_out_many(
				None,
				&mut WeightMeter::new(),
			))
		});

		#[extrinsic_call]
		_(RawOrigin::Root, messages);

		for event in frame_system::Pallet::<T>::events() {
			let encoded = event.encode();
			log::info!("Event of pallet: {} and event: {}", encoded[0], encoded[1]);
		}
	}
}

/// Unwrap something that does not implement Debug. Otherwise we would need to require
/// `pallet_rc_migrator::Config` on out runtime `T`.
pub fn unwrap_no_debug<T, E>(result: Result<T, E>) -> T {
	match result {
		Ok(t) => t,
		Err(_) => panic!("unwrap_no_debug"),
	}
}

/// Check that Oliver's account has some balance on AH and Relay.
///
/// This serves as sanity check that the snapshots were loaded correctly.
fn verify_snapshot<T: Config>() {
	let raw_acc: [u8; 32] =
		hex::decode("6c9e3102dd2c24274667d416e07570ebce6f20ab80ee3fc9917bf4a7568b8fd2")
			.unwrap()
			.try_into()
			.unwrap();
	let acc = AccountId32::from(raw_acc);
	frame_system::Pallet::<T>::reset_events();

	// Sanity check that this is the right account
	let ah_acc = frame_system::Account::<T>::get(&acc);
	if ah_acc.data.free == 0 {
		panic!("No or broken snapshot: account does not have any balance");
	}

	let key = frame_system::Account::<T>::hashed_key_for(&acc);
	let raw_acc = relay_snapshot(|| {
		frame_support::storage::unhashed::get::<
			pallet_balances::AccountData<<T as pallet_balances::Config>::Balance>,
		>(key.as_ref())
	}).unwrap();

	if raw_acc.free == 0 {
		panic!("No or broken snapshot: account does not have any balance");
	}
}

fn relay_snapshot<R, F: FnOnce() -> R>(f: F) -> R {
	// Enable the relay chain snapshot
	sp_io::storage::get(b"relay_chain_enable");
	let result = f();
	sp_io::storage::get(b"relay_chain_disable");
	result
}
