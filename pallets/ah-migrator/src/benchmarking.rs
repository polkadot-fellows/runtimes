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
use frame_support::{
	traits::{tokens::IdAmount, Currency, EnqueueMessage},
	weights::WeightMeter,
};
use frame_system::RawOrigin;
use pallet_rc_migrator::types::PalletMigration;
use xcm::VersionedXcm;

pub const UNITS: u128 = 10_000_000_000;

pub trait ParametersFactory<RcMultisig, RcAccount> {
	fn create_multisig(n: u8) -> RcMultisig;
	fn create_account(n: u8) -> RcAccount;
}

pub struct BenchmarkFactory<T: Config>(PhantomData<T>);
impl<T: Config>
	ParametersFactory<
		RcMultisig<AccountId32, u128>,
		RcAccount<AccountId32, u128, T::RcHoldReason, T::RcFreezeReason>,
	> for BenchmarkFactory<T>
where
	T::AccountId: From<AccountId32>,
	<<T as pallet_multisig::Config>::Currency as Currency<T::AccountId>>::Balance: From<u128>,
{
	fn create_multisig(n: u8) -> RcMultisig<AccountId32, u128> {
		let creator: AccountId32 = [n; 32].into();
		let deposit: u128 = UNITS;
		let _ = <T as pallet_multisig::Config>::Currency::deposit_creating(
			&creator,
			(deposit * 10).into(),
		);
		let _ =
			<T as pallet_multisig::Config>::Currency::reserve(&creator, deposit.into()).unwrap();

		RcMultisig { creator, deposit, details: Some([2u8; 32].into()) }
	}

	fn create_account(n: u8) -> RcAccount<AccountId32, u128, T::RcHoldReason, T::RcFreezeReason> {
		let who: AccountId32 = [n; 32].into();

		let hold_amount = UNITS;
		// TODO: abstract this
		// 10 - preimage pallet, 0 - just first variant
		let hold_reason: T::RcHoldReason = Decode::decode(&mut &[10, 0][..]).unwrap();
		let holds = vec![IdAmount { id: hold_reason, amount: hold_amount }];

		let freeze_amount = 2 * UNITS;
		// TODO: abstract this
		// 39 - nomination pools pallet, 0 - just first variant
		let freeze_reason: T::RcFreezeReason = Decode::decode(&mut &[39, 0][..]).unwrap();
		let freezes = vec![IdAmount { id: freeze_reason, amount: freeze_amount }];

		let lock_amount = 3 * UNITS;
		let locks = vec![pallet_balances::BalanceLock::<u128> {
			id: [1u8; 8],
			amount: lock_amount,
			reasons: pallet_balances::Reasons::All,
		}];

		let unnamed_reserve = 4 * UNITS;

		let free = UNITS + hold_amount + freeze_amount + lock_amount + unnamed_reserve;
		let reserved = hold_amount + unnamed_reserve;
		let frozen = freeze_amount + lock_amount;

		RcAccount {
			who,
			free,
			reserved,
			frozen,
			holds,
			freezes,
			locks,
			unnamed_reserve,
			consumers: 1,
			providers: 1,
		}
	}
}

#[benchmarks(where T: pallet_balances::Config)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn receive_multisigs_from_snap(n: Linear<0, 100>) {
		verify_snapshot::<T>();
		let (mut messages, _cursor) = relay_snapshot(|| {
			unwrap_no_debug(pallet_rc_migrator::multisig::MultisigMigrator::<T>::migrate_out_many(
				None,
				&mut WeightMeter::new(),
			))
		});

		// TODO: unreserve fails since accounts should migrate first to make it successful. we will
		// have a similar issue with the other calls benchmarks.
		// TODO: possible we can truncate to n to have weights based on the number of messages
		// TODO: for calls that have messages with `m` number of variants, we perhaps need to have
		// `m` parameters like `n` parameter in this function. and we filter the returned by
		// `migrate_out_many` `messages` or we pass these parameters to `migrate_out_many`.
		messages.truncate(n as usize);

		#[extrinsic_call]
		receive_multisigs(RawOrigin::Root, messages);

		for event in frame_system::Pallet::<T>::events() {
			let encoded = event.encode();
			log::info!("Event of pallet: {} and event: {}", encoded[0], encoded[1]);
		}
	}

	#[benchmark]
	fn receive_nom_pools_messages_from_snap() {
		verify_snapshot::<T>();
		let (messages, _cursor) = relay_snapshot(|| {
			unwrap_no_debug(
				pallet_rc_migrator::staking::nom_pools::NomPoolsMigrator::<T>::migrate_many(
					None,
					&mut WeightMeter::new(),
				),
			)
		});

		#[extrinsic_call]
		receive_nom_pools_messages(RawOrigin::Root, messages);

		// TODO assert event
	}

	// #[benchmark]
	// fn receive_accounts_from_snap(n: Linear<0, 100>) {
	// 	verify_snapshot::<T>();
	// 	let (mut accounts, _cursor) = relay_snapshot(|| {
	// 		unwrap_no_debug(pallet_rc_migrator::account::AccountMigrator::<T>::migrate_out_many(
	// 			// TODO: we can have different shift for cursor, but we never know if we was able
	// 			// to fetch the worth cases.
	// 			None,
	// 			&mut WeightMeter::new(),
	// 		))
	// 	});
	// 	accounts.truncate(n as usize);

	// 	#[extrinsic_call]
	// 	receive_accounts(RawOrigin::Root, accounts);
	// }

	#[benchmark]
	fn receive_multisigs(n: Linear<0, 100>) {
		let messages = (0..n)
			.map(|i| <<T as Config>::BenchmarkHelper>::create_multisig(i.try_into().unwrap()))
			.collect::<Vec<_>>();

		#[extrinsic_call]
		_(RawOrigin::Root, messages)
	}

	#[benchmark]
	fn receive_accounts(n: Linear<0, 100>) {
		let messages = (0..n)
			.map(|i| <<T as Config>::BenchmarkHelper>::create_account(i.try_into().unwrap()))
			.collect::<Vec<_>>();

		#[extrinsic_call]
		_(RawOrigin::Root, messages)

		// TODO assert event
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
	})
	.unwrap();

	if raw_acc.free == 0 {
		panic!("No or broken snapshot: account does not have any balance");
	}
}

/// Read something from the relay chain snapshot instead of the asset hub one.
fn relay_snapshot<R, F: FnOnce() -> R>(f: F) -> R {
	sp_io::storage::get(b"relay_chain_enable");
	let result = f();
	sp_io::storage::get(b"relay_chain_disable");
	result
}
