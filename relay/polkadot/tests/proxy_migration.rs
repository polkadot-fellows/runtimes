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

//! Tests for the legacy `Proxy.Proxies` storage repair migration.

use codec::Encode;
use frame_support::{
	storage::unhashed,
	traits::{Currency, OnRuntimeUpgrade, ReservableCurrency},
};
use hex_literal::hex;
use polkadot_primitives::AccountId;
use polkadot_runtime::{Balances, Runtime};
use relay_common::proxy::MigrateLegacyProxies;

type Migration = MigrateLegacyProxies<Runtime>;
type Proxies = pallet_proxy::Proxies<Runtime>;

/// Writes `raw` as the raw, undecoded `Proxies` value for `who`.
fn put_raw_proxies(who: &AccountId, raw: &[u8]) {
	let key = Proxies::hashed_key_for(who);
	unhashed::put_raw(&key, raw);
}

/// The undecodable entry from the issue, legacy-encoded: two proxies to `0x98c8..2223` (`Any` and
/// removed `SudoBalances`) plus a deposit.
const ISSUE_RAW_VALUE: [u8; 83] = hex!(
	"0898c8a3d01da9877b7b30877c717ae8f2a7e726cefa176c5dfcdcebc9b6a12223\
	 0098c8a3d01da9877b7b30877c717ae8f2a7e726cefa176c5dfcdcebc9b6a12223\
	 04005109bd2e0000000000000000000000"
);

const DELEGATE: [u8; 32] = hex!("98c8a3d01da9877b7b30877c717ae8f2a7e726cefa176c5dfcdcebc9b6a12223");

const ISSUE_DEPOSIT: u128 = 200_740_000_000;

/// The reported entry decodes after the migration: the `Any` proxy is preserved with `delay = 0`,
/// the removed `SudoBalances` proxy is dropped and the deposit is untouched.
#[test]
fn repairs_reported_entry() {
	sp_io::TestExternalities::default().execute_with(|| {
		// Arbitrary owner; the map key is irrelevant to decoding.
		let who = AccountId::from([1u8; 32]);
		put_raw_proxies(&who, &ISSUE_RAW_VALUE);

		assert!(Proxies::try_get(&who).is_err(), "entry should be undecodable before migration");

		Migration::on_runtime_upgrade();

		let (proxies, deposit) = Proxies::try_get(&who).expect("entry decodable after migration");
		assert_eq!(proxies.len(), 1, "the removed-type proxy must be dropped");
		assert_eq!(deposit, ISSUE_DEPOSIT, "the deposit must be preserved");

		let proxy = &proxies[0];
		assert_eq!(proxy.delegate, AccountId::from(DELEGATE));
		assert_eq!(proxy.delay, 0);
		// `Any` encodes to discriminant `0`.
		assert_eq!(proxy.proxy_type.encode(), vec![0u8]);
	});
}

/// A current-format entry is left untouched, so re-running the migration is a no-op.
#[test]
fn leaves_valid_entry_untouched() {
	sp_io::TestExternalities::default().execute_with(|| {
		let who = AccountId::from([2u8; 32]);
		put_raw_proxies(&who, &ISSUE_RAW_VALUE);

		// Repair once, then re-run: the bytes must not change.
		Migration::on_runtime_upgrade();
		let key = Proxies::hashed_key_for(&who);
		let repaired = unhashed::get_raw(&key).expect("entry exists");

		Migration::on_runtime_upgrade();
		assert_eq!(unhashed::get_raw(&key).expect("entry exists"), repaired);
	});
}

/// When every proxy has a removed type, the entry is removed and the deposit unreserved.
#[test]
fn removes_entry_with_only_removed_types_and_refunds() {
	sp_io::TestExternalities::default().execute_with(|| {
		let who = AccountId::from([3u8; 32]);
		let deposit: u128 = 5_000_000_000;

		// Mirror the on-chain reserve for this entry.
		let _ = Balances::make_free_balance_be(&who, 1_000_000_000_000);
		Balances::reserve(&who, deposit).expect("can reserve");
		assert_eq!(Balances::reserved_balance(&who), deposit);

		// Legacy value holding only a removed-type proxy.
		let mut raw = Vec::new();
		raw.push(0x04u8); // compact(1)
		raw.extend_from_slice(&DELEGATE);
		raw.push(0x04u8); // proxy_type = 4 (removed)
		raw.extend_from_slice(&deposit.encode());
		put_raw_proxies(&who, &raw);

		assert!(Proxies::try_get(&who).is_err());

		Migration::on_runtime_upgrade();

		assert!(!pallet_proxy::Proxies::<Runtime>::contains_key(&who));
		assert_eq!(Balances::reserved_balance(&who), 0);
	});
}
