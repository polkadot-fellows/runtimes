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

//! Unit tests for the accounts stage.
//!
//! The stage's contract: every account on this chain is withdrawn whole, its reserve attributed
//! against the owning pallets' records, and the pieces routed to the Coretime chain or Asset Hub
//! with the conservation ledger (`RcMigratedBalance`) exact after every block. The tests pin that
//! contract with exact values; the counterpart chains appear only as the returned payloads.

use super::*;
use crate::{mock::*, ExpectedReserves, RcMigratedBalance};
use frame_support::{
	assert_ok, hypothetically,
	traits::{LockableCurrency, WithdrawReasons},
	weights::Weight,
};
use sp_core::H256;
use sp_runtime::traits::{BlakeTwo256, Hash};

type Migrator = AccountsMigrator<Test>;

fn withdraw(who: &AccountId32) -> Option<Withdrawal> {
	let info = frame_system::Account::<Test>::get(who);
	Migrator::withdraw_account(who, info, None).expect("withdrawal must not error")
}

fn ct_holds(w: &Withdrawal) -> Vec<(PortableHoldReason, u128)> {
	w.ct.as_ref()
		.map(|a| a.holds.iter().map(|h| (h.reason, h.amount)).collect())
		.unwrap_or_default()
}

/// Seed the conservation ledger the way `init` does, for tests that drive `migrate_many`
/// directly.
fn seed_ledger() {
	RcMigratedBalance::<Test>::put(MigratedBalances {
		kept: total_issuance(),
		..Default::default()
	});
}

fn migrate_block(last_key: Option<AccountId32>) -> BlockWithdrawals {
	with_storage_layer(|| Migrator::migrate_many(last_key, None).map_err(DispatchError::from))
		.expect("block succeeds")
}

// ---------------------------------------------------------------------------
// Expected-reserve indexing
// ---------------------------------------------------------------------------

#[test]
fn build_expected_reserves_indexes_every_deposit_source() {
	new_test_ext().execute_with(|| {
		let alice = acc(1); // parachain manager
		let bob = acc(2); // delegator with a portable (Any) proxy def
		let carol = acc(3); // delegator with only a non-portable (Staking) def
		let dave = acc(4); // multisig depositor
		let eve = acc(5); // proxy announcer
		let frank = acc(6); // delegator eve announces for
		let delegate = acc(7);

		// GIVEN one deposit of every kind the relay chain knows.
		fund(&alice, 1_000);
		register_para(2000, &alice); // 300 recorded + reserved
		open_channel(2000, 2001, 70, 30);
		open_request(2000, 2002, 25);
		fund(&bob, 500);
		add_proxy(&bob, &delegate, ProxyType::Any); // 40 base + 4 factor = 44 reserved
		fund(&carol, 500);
		add_proxy(&carol, &delegate, ProxyType::Staking); // 44 reserved
		fund(&dave, 500);
		let call = Box::new(RuntimeCall::System(frame_system::Call::remark { remark: vec![] }));
		assert_ok!(Multisig::as_multi(
			RuntimeOrigin::signed(dave.clone()),
			2,
			vec![eve.clone()],
			None,
			call,
			Weight::zero(),
		)); // 30 base + 2 * 5 factor = 40 reserved
		fund(&frank, 500);
		add_proxy(&frank, &eve, ProxyType::Any);
		fund(&eve, 500);
		assert_ok!(Proxy::announce(
			RuntimeOrigin::signed(eve.clone()),
			frank.clone(),
			H256::zero()
		)); // 25 base + 6 factor = 31 reserved

		// WHEN the index is built.
		let records = Migrator::build_expected_reserves();

		// THEN every source is classified: registrar + HRMP (+ requests) are Coretime-bound,
		// portable proxy deposits travel under their own reason, everything whose purpose ends
		// with this chain is refunded.
		assert_eq!(records, 8, "para + channel + request + 3 proxies + announcement + multisig");
		assert_eq!(
			ExpectedReserves::<Test>::get(&alice),
			ExpectedReserve { ct: 300, ..Default::default() }
		);
		assert_eq!(ExpectedReserves::<Test>::get(child_sov(2000)).ct, 70 + 25);
		assert_eq!(ExpectedReserves::<Test>::get(child_sov(2001)).ct, 30);
		assert_eq!(ExpectedReserves::<Test>::get(&bob).proxy, 44);
		assert_eq!(ExpectedReserves::<Test>::get(&frank).proxy, 44);
		assert_eq!(ExpectedReserves::<Test>::get(&carol).refund, 44);
		assert_eq!(ExpectedReserves::<Test>::get(&dave).refund, 40);
		assert_eq!(ExpectedReserves::<Test>::get(&eve).refund, 31);
	});
}

#[test]
fn init_releases_preimage_deposits_and_seeds_the_ledger() {
	new_test_ext().execute_with(|| {
		let alice = acc(1); // noted two preimages, one of which governance then requested
		fund(&alice, 1_000);
		register_para(2000, &alice);

		// GIVEN an unrequested preimage (alice's deposit), a requested one alice noted (her
		// deposit, governance's request) and one governance requested that nobody noted.
		let unrequested = vec![1u8; 4];
		let requested_noted = vec![2u8; 4];
		let requested_bare = BlakeTwo256::hash(&[3u8; 4]);
		assert_ok!(Preimage::note_preimage(
			RuntimeOrigin::signed(alice.clone()),
			unrequested.clone()
		));
		assert_ok!(Preimage::note_preimage(
			RuntimeOrigin::signed(alice.clone()),
			requested_noted.clone()
		));
		let requested_noted_hash = BlakeTwo256::hash(&requested_noted);
		assert_ok!(Preimage::request_preimage(RuntimeOrigin::root(), requested_noted_hash));
		assert_ok!(Preimage::request_preimage(RuntimeOrigin::root(), requested_bare));
		// Two deposits of 1 base + 4 bytes each, as named holds.
		assert_eq!(pallet_balances::Holds::<Test>::get(&alice).len(), 1);
		assert_eq!(pallet_balances::Holds::<Test>::get(&alice)[0].amount, 10);
		let info = frame_system::Account::<Test>::get(&alice);
		assert!(!Migrator::can_migrate(&alice, &info, None), "a named hold blocks migration");
		let ti = total_issuance();

		// WHEN the stage initialises.
		let records = Migrator::init();

		// THEN every deposit is back with alice, the unrequested blob is gone, the requested
		// blob is kept without a ticket, and the bare request is untouched.
		assert_eq!(records, 1);
		assert!(pallet_balances::Holds::<Test>::get(&alice).is_empty());
		assert_eq!(free(&alice), 700);
		assert!(pallet_preimage::RequestStatusFor::<Test>::get(BlakeTwo256::hash(&unrequested))
			.is_none());
		assert!(matches!(
			pallet_preimage::RequestStatusFor::<Test>::get(requested_noted_hash),
			Some(pallet_preimage::RequestStatus::Requested { maybe_ticket: None, count: 1, .. })
		));
		assert!(pallet_preimage::PreimageFor::<Test>::contains_key((requested_noted_hash, 4)));
		assert!(matches!(
			pallet_preimage::RequestStatusFor::<Test>::get(requested_bare),
			Some(pallet_preimage::RequestStatus::Requested { maybe_ticket: None, count: 1, .. })
		));
		// AND the account migrates now, and the ledger starts from the untouched issuance.
		let info = frame_system::Account::<Test>::get(&alice);
		assert!(Migrator::can_migrate(&alice, &info, None));
		assert_eq!(
			RcMigratedBalance::<Test>::get(),
			MigratedBalances { kept: ti, ..Default::default() }
		);
	});
}

// ---------------------------------------------------------------------------
// Single-account withdrawal: the split rule
// ---------------------------------------------------------------------------

#[test]
fn withdraw_splits_deposit_buffer_and_teleport() {
	new_test_ext().execute_with(|| {
		let alice = acc(1); // parachain manager, cleanly migrating
		fund(&alice, 1_000);
		register_para(2000, &alice); // free 700, reserved 300
		Migrator::build_expected_reserves();
		let ti_before = total_issuance();

		let w = withdraw(&alice).expect("migrates");

		// Deposit -> Coretime hold, one buffer of free follows it, the rest teleports to AH.
		assert_eq!(ct_holds(&w), vec![(PortableHoldReason::UnnamedReserve, 300)]);
		assert_eq!(w.ct.as_ref().unwrap().free, 100);
		assert_eq!(w.ah, Some((alice.clone(), 600)));
		// The account is gone and exactly its total was burned.
		assert!(!frame_system::Account::<Test>::contains_key(&alice));
		assert_eq!(total_issuance(), ti_before - 1_000);
	});
}

#[test]
fn withdraw_parks_unattributed_reserve_under_its_own_reason() {
	new_test_ext().execute_with(|| {
		let bob = acc(2); // account with a reserve no pallet's records explain (on-chain anomaly)
		fund(&bob, 600);
		reserve(&bob, 200);
		Migrator::build_expected_reserves();

		let w = withdraw(&bob).expect("migrates");

		assert_eq!(ct_holds(&w), vec![(PortableHoldReason::UnattributedReserve, 200)]);
		assert_eq!(w.ct.as_ref().unwrap().free, 100);
		assert_eq!(w.ah, Some((bob.clone(), 300)));
		assert_eq!(
			migrator_events(),
			vec![Event::UnattributedReserve { who: bob.clone(), amount: 200 }]
		);
	});
}

#[test]
fn withdraw_refunds_deposits_whose_purpose_ends_here() {
	new_test_ext().execute_with(|| {
		let carol = acc(3); // delegator with only a Staking def: nothing travels, deposit refunds
		fund(&carol, 500);
		add_proxy(&carol, &acc(7), ProxyType::Staking); // free 456, reserved 44
		Migrator::build_expected_reserves();

		let w = withdraw(&carol).expect("migrates");

		// The refund joins the liquid balance; with no Coretime-bound hold there is no buffer.
		assert!(w.ct.is_none());
		assert_eq!(w.ah, Some((carol.clone(), 500)));
		assert_eq!(
			migrator_events(),
			vec![Event::DepositRefunded { who: carol.clone(), amount: 44 }]
		);
	});
}

#[test]
fn withdraw_attributes_shortfall_in_priority_order() {
	new_test_ext().execute_with(|| {
		let dave = acc(4); // account whose live reserve under-covers the recorded deposits
		fund(&dave, 200);
		reserve(&dave, 100);
		// Recorded expectations exceed the live 100: Coretime-bound deposits are made whole
		// first, proxy deposits second, refunds last. (Set directly: only the split math is under
		// test.)
		ExpectedReserves::<Test>::insert(&dave, ExpectedReserve { ct: 50, proxy: 30, refund: 40 });

		let w = withdraw(&dave).expect("migrates");

		assert_eq!(
			ct_holds(&w),
			vec![(PortableHoldReason::UnnamedReserve, 50), (PortableHoldReason::ProxyDeposit, 30)]
		);
		// Of the refundable 40 only 20 reserve was left; it becomes liquid.
		assert_eq!(
			migrator_events(),
			vec![Event::DepositRefunded { who: dave.clone(), amount: 20 }]
		);
		// liquid = 100 free + 20 refunded; buffer 100 stays with the deposit, 20 teleports.
		assert_eq!(w.ct.as_ref().unwrap().free, 100);
		assert_eq!(w.ah, Some((dave.clone(), 20)));
	});
}

#[test]
fn withdraw_routes_never_signed_any_delegators_wholly_to_ct() {
	new_test_ext().execute_with(|| {
		let pure = acc(30); // keyless pure proxy: nonce 0, Any def
		let delegate = acc(31);
		fund(&pure, 544);
		add_proxy(&pure, &delegate, ProxyType::Any); // free 500, reserved 44, nonce still 0
		Migrator::build_expected_reserves();

		let w = withdraw(&pure).expect("migrates");

		// Funds follow control: everything goes where the definitions are recreated.
		assert_eq!(ct_holds(&w), vec![(PortableHoldReason::ProxyDeposit, 44)]);
		assert_eq!(w.ct.as_ref().unwrap().free, 500);
		assert_eq!(w.ah, None);

		// A never-signed delegator WITHOUT an Any def is not a pure (a pure created with less
		// has already lost control by construction): the regular split applies.
		hypothetically!({
			let multisigish = acc(32);
			fund(&multisigish, 544);
			add_proxy(&multisigish, &delegate, ProxyType::NonTransfer);
			Migrator::build_expected_reserves();
			let w = withdraw(&multisigish).expect("migrates");
			assert_eq!(w.ct.as_ref().unwrap().free, 100);
			assert_eq!(w.ah, Some((multisigish, 400)));
		});

		// A signed delegator with an Any def is a regular account that happens to have a proxy.
		hypothetically!({
			let signer = acc(33);
			fund(&signer, 544);
			add_proxy(&signer, &delegate, ProxyType::Any);
			frame_system::Pallet::<Test>::inc_account_nonce(&signer);
			Migrator::build_expected_reserves();
			let w = withdraw(&signer).expect("migrates");
			assert_eq!(w.ct.as_ref().unwrap().free, 100);
			assert_eq!(w.ah, Some((signer, 400)));
		});
	});
}

#[test]
fn withdraw_keeps_sub_ah_ed_dust_with_the_deposit() {
	new_test_ext().execute_with(|| {
		let heidi = acc(8); // deposit holder whose teleport remainder would be below AH's ED
		fund(&heidi, 404);
		reserve(&heidi, 300);
		ExpectedReserves::<Test>::insert(&heidi, ExpectedReserve { ct: 300, ..Default::default() });

		let w = withdraw(&heidi).expect("migrates");

		// liquid 104: buffer 100 + remainder 4 < AH ED (5) -> the dust follows the deposit.
		assert_eq!(w.ct.as_ref().unwrap().free, 104);
		assert_eq!(w.ah, None);
	});
}

#[test]
fn can_migrate_keeps_manager_module_below_ed_and_locked_accounts() {
	new_test_ext().execute_with(|| {
		// The manager stays funded to keep driving the migration.
		let manager = acc(20);
		fund(&manager, 500);
		let info = frame_system::Account::<Test>::get(&manager);
		assert!(!Migrator::can_migrate(&manager, &info, Some(&manager)));
		assert!(Migrator::can_migrate(&manager, &info, None), "only while it is the manager");

		// Module accounts stay for the sweep stage.
		fund(&pot(), 500);
		assert_eq!(withdraw(&pot()), None);
		assert!(frame_system::Account::<Test>::contains_key(pot()));

		// Below-ED accounts only exist via external provider refs; they are not migrated.
		let dusty = acc(9);
		force_anomalous_account(&dusty, 4, 0, 0);
		assert_eq!(withdraw(&dusty), None);

		// Locks cannot be translated; the account stays behind whole.
		let locked = acc(10);
		fund(&locked, 500);
		<Balances as LockableCurrency<AccountId32>>::set_lock(
			*b"testlock",
			&locked,
			100,
			WithdrawReasons::all(),
		);
		assert_eq!(withdraw(&locked), None);
		assert_eq!(free(&locked), 500);
	});
}

#[test]
fn withdraw_drains_consumer_referenced_accounts_to_shells() {
	new_test_ext().execute_with(|| {
		let ida = acc(11); // validator-like account: session keys hold a consumer reference
		fund(&ida, 1_000);
		reserve(&ida, 300);
		ExpectedReserves::<Test>::insert(&ida, ExpectedReserve { ct: 300, ..Default::default() });
		// The extra reference some pallet (session keys in production) holds on the account.
		frame_system::Pallet::<Test>::inc_consumers(&ida).unwrap();
		let ti_before = total_issuance();

		let w = withdraw(&ida).expect("migrates");

		// The money moves like any other account's...
		assert_eq!(ct_holds(&w), vec![(PortableHoldReason::UnnamedReserve, 300)]);
		assert_eq!(w.ct.as_ref().unwrap().free, 100);
		assert_eq!(w.ah, Some((ida.clone(), 600)));
		// ...but the record survives as a zero-balance shell.
		assert_eq!(free(&ida), 0);
		assert_eq!(reserved(&ida), 0);
		assert_eq!(frame_system::Pallet::<Test>::consumers(&ida), 1);
		assert_eq!(total_issuance(), ti_before - 1_000);
		assert_eq!(
			migrator_events(),
			vec![Event::AccountShellDrained { who: ida.clone(), amount: 1_000 }]
		);
	});
}

#[test]
fn withdraw_translates_child_sovereigns_to_sibling_addresses() {
	new_test_ext().execute_with(|| {
		open_channel(2000, 2001, 70, 30); // funds + reserves on the child sovereigns
		Migrator::build_expected_reserves();

		let w = withdraw(&child_sov(2000)).expect("migrates");

		let ct = w.ct.as_ref().unwrap();
		assert_eq!(ct.who, migrator_types::sibling_account::<AccountId32>(2000));
		assert_eq!(ct_holds(&w), vec![(PortableHoldReason::UnnamedReserve, 70)]);
		// The sovereign's free balance (the ED it was funded with) is below the buffer, so it
		// all follows the deposit as working buffer.
		assert_eq!(ct.free, ED);
		assert_eq!(w.ah, None);
	});
}

// ---------------------------------------------------------------------------
// Per-block withdrawal: payloads, ledger, cursor
// ---------------------------------------------------------------------------

#[test]
fn migrate_many_returns_exactly_what_it_burns_and_keeps_the_ledger_exact() {
	new_test_ext().execute_with(|| {
		let alice = acc(1); // parachain manager
		let manager = acc(20); // drives the migration; must stay
		fund(&alice, 1_000);
		fund(&manager, 500);
		register_para(2000, &alice);
		let ti_before = total_issuance();
		Migrator::init();

		let out = with_storage_layer(|| {
			Migrator::migrate_many(None, Some(&manager)).map_err(DispatchError::from)
		})
		.expect("block succeeds");

		// One block covers everything; the payloads carry exactly the burned pieces.
		assert_eq!(out.last_key, None);
		assert_eq!(
			out.ct,
			vec![PortableAccount {
				who: alice.clone(),
				free: 100,
				holds: vec![PortableHold {
					reason: PortableHoldReason::UnnamedReserve,
					amount: 300
				}]
				.try_into()
				.unwrap(),
			}]
		);
		assert_eq!(out.ah, vec![(alice.clone(), 600)]);
		assert_eq!(free(&manager), 500, "the manager is untouched");

		// The ledger sums to the issuance the stage started from, and what is kept is what the
		// chain still holds.
		let ledger = RcMigratedBalance::<Test>::get();
		assert_eq!(
			ledger,
			MigratedBalances {
				kept: ti_before - 1_000,
				ct_reserved: 300,
				ct_free: 100,
				ah_free: 600,
				ti_corrected: 0,
			}
		);
		assert_eq!(total_issuance(), ledger.kept);
	});
}

#[test]
fn migrate_many_stops_at_the_per_block_limit_and_resumes_from_the_cursor() {
	new_test_ext().execute_with(|| {
		// GIVEN more accounts than one block may process.
		let count = MAX_ACCOUNTS_PER_BLOCK + 20;
		for i in 0..count {
			let mut bytes = [0u8; 32];
			bytes[..4].copy_from_slice(&i.to_le_bytes());
			bytes[4] = 0xAA;
			fund(&AccountId32::new(bytes), 1_000);
		}
		let ti_before = total_issuance();
		seed_ledger();

		// WHEN the first block runs, it stops at the limit and hands back a cursor.
		let first = migrate_block(None);
		assert_eq!(first.ah.len(), MAX_ACCOUNTS_PER_BLOCK as usize);
		assert!(first.ct.is_empty(), "plain free balance has no Coretime leg");
		let cursor = first.last_key.expect("more accounts remain than the per-block limit");
		assert_eq!(frame_system::Account::<Test>::iter().count(), 20);

		// AND the second block continues after the cursor and exhausts the account space.
		let second = migrate_block(Some(cursor));
		assert_eq!(second.ah.len(), 20);
		assert_eq!(second.last_key, None);

		// THEN every account is gone and the ledger is exact: all free balance teleported.
		assert_eq!(frame_system::Account::<Test>::iter().count(), 0);
		assert_eq!(
			RcMigratedBalance::<Test>::get(),
			MigratedBalances { kept: 0, ah_free: ti_before, ..Default::default() }
		);
		assert_eq!(total_issuance(), 0);
	});
}

#[test]
fn migrate_many_leaves_kept_accounts_in_place_and_out_of_the_payloads() {
	new_test_ext().execute_with(|| {
		let alice = acc(1); // migrates
		let locked = acc(10); // stays: untranslatable lock
		fund(&alice, 1_000);
		fund(&locked, 500);
		<Balances as LockableCurrency<AccountId32>>::set_lock(
			*b"testlock",
			&locked,
			100,
			WithdrawReasons::all(),
		);
		fund(&pot(), 300); // stays: module account
		seed_ledger();
		let ti_before = total_issuance();

		let out = migrate_block(None);

		assert_eq!(out.ah, vec![(alice, 1_000)]);
		assert_eq!(free(&locked), 500);
		assert_eq!(free(&pot()), 300);
		assert_eq!(total_issuance(), ti_before - 1_000);
		assert_eq!(RcMigratedBalance::<Test>::get().kept, 800);
	});
}
