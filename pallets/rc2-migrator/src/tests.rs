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

//! Unit tests for `pallet-rc2-migrator`.
//!
//! The pallet's contract, in the abstract: drain every account and record on this chain into
//! portable payloads, burn exactly what is shipped, and keep the conservation ledger
//! (`RcMigratedBalance`) exact at every step — a failed send must roll back to a retryable
//! state, never leave balances burned-but-unsent. The tests pin that contract with exact values;
//! the counterpart chains appear only as captured XCM messages.

use crate::{accounts::AccountsMigrator, mock::*, *};
use frame_support::{
	assert_noop, assert_ok, hypothetically,
	traits::{
		LockableCurrency, OnInitialize, OnRuntimeUpgrade, ReservableCurrency, WithdrawReasons,
	},
	weights::Weight,
};
use migrator_types::{PortableProxyDelegate, PortableProxyType};
use runtime_parachains::{
	hrmp as parachains_hrmp,
	inclusion::{AggregateMessageOrigin, UmpQueueId},
};
use sp_core::{sr25519, Pair, H256};
use sp_runtime::{traits::BadOrigin, AccountId32, MultiSignature, MultiSigner};

type Stage = MigrationStageOf<Test>;

fn root() -> RuntimeOrigin {
	RuntimeOrigin::root()
}

/// Execute the next block's `on_initialize` of the migrator, then let a healthy Coretime chain
/// confirm whatever it sent — the stage machine holds until each batch is answered.
fn run_block() {
	let now = System::block_number() + 1;
	System::set_block_number(now);
	<Rc2Migrator as OnInitialize<u32>>::on_initialize(now);
	confirm_pending_batches();
}

/// Seed the conservation tracker the way `AccountsInit` does, for tests that drive a stage
/// function directly instead of walking the machine.
fn seed_tracker() {
	RcMigratedBalance::<Test>::put(MigratedBalances {
		kept: total_issuance(),
		..Default::default()
	});
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
		add_proxy(&bob, &delegate, ProxyType::Any); // 44 reserved
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
		)); // 25 + 6 = 31 reserved

		// WHEN the index is built.
		let records = AccountsMigrator::<Test>::build_expected_reserves();

		// THEN every source is classified: registrar + HRMP (+ requests) are Coretime-bound,
		// portable proxy deposits travel under their own reason, everything whose purpose ends
		// with this chain is refunded.
		assert_eq!(records, 8, "para + channel + request + 3 proxies + multisig + announcement");
		assert_eq!(ExpectedReserves::<Test>::get(&alice).ct, 300);
		assert_eq!(ExpectedReserves::<Test>::get(child_sov(2000)).ct, 70 + 25);
		assert_eq!(ExpectedReserves::<Test>::get(child_sov(2001)).ct, 30);
		assert_eq!(ExpectedReserves::<Test>::get(&bob).proxy, 44);
		assert_eq!(ExpectedReserves::<Test>::get(&frank).proxy, 44);
		assert_eq!(ExpectedReserves::<Test>::get(&carol).refund, 44);
		assert_eq!(ExpectedReserves::<Test>::get(&dave).refund, 40);
		assert_eq!(ExpectedReserves::<Test>::get(&eve).refund, 31);
	});
}

// ---------------------------------------------------------------------------
// Single-account withdrawal: the split rule
// ---------------------------------------------------------------------------

fn withdraw(who: &AccountId32) -> Option<accounts::Withdrawal> {
	let info = frame_system::Account::<Test>::get(who);
	AccountsMigrator::<Test>::withdraw_account(who, info).expect("withdrawal must not error")
}

fn ct_holds(w: &accounts::Withdrawal) -> Vec<(PortableHoldReason, u128)> {
	w.ct.as_ref()
		.map(|a| a.holds.iter().map(|h| (h.reason, h.amount)).collect())
		.unwrap_or_default()
}

#[test]
fn withdraw_splits_deposit_buffer_and_teleport() {
	new_test_ext().execute_with(|| {
		let alice = acc(1); // parachain manager, cleanly migrating
		fund(&alice, 1_000);
		register_para(2000, &alice); // free 700, reserved 300
		AccountsMigrator::<Test>::build_expected_reserves();
		let ti_before = total_issuance();

		let w = withdraw(&alice).expect("migrates");

		// Deposit -> CT hold, one buffer of free follows it, the rest teleports to AH.
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
		<Balances as ReservableCurrency<AccountId32>>::reserve(&bob, 200).unwrap();
		AccountsMigrator::<Test>::build_expected_reserves();

		let w = withdraw(&bob).expect("migrates");

		assert_eq!(ct_holds(&w), vec![(PortableHoldReason::UnattributedReserve, 200)]);
		assert_eq!(w.ct.as_ref().unwrap().free, 100);
		assert_eq!(w.ah, Some((bob.clone(), 300)));
		assert!(migrator_events()
			.contains(&Event::UnattributedReserve { who: bob.clone(), amount: 200 }));
	});
}

#[test]
fn withdraw_refunds_deposits_whose_purpose_ends_here() {
	new_test_ext().execute_with(|| {
		let carol = acc(3); // delegator with only a Staking def: nothing travels, deposit refunds
		fund(&carol, 500);
		add_proxy(&carol, &acc(7), ProxyType::Staking); // free 456, reserved 44
		AccountsMigrator::<Test>::build_expected_reserves();

		let w = withdraw(&carol).expect("migrates");

		// The refund joins the liquid balance; with no CT-bound hold there is no buffer either.
		assert!(w.ct.is_none());
		assert_eq!(w.ah, Some((carol.clone(), 500)));
		assert!(
			migrator_events().contains(&Event::DepositRefunded { who: carol.clone(), amount: 44 })
		);
	});
}

#[test]
fn withdraw_attributes_shortfall_in_priority_order() {
	new_test_ext().execute_with(|| {
		let dave = acc(4); // account whose live reserve under-covers the recorded deposits
		fund(&dave, 200);
		<Balances as ReservableCurrency<AccountId32>>::reserve(&dave, 100).unwrap();
		// Recorded expectations exceed the live 100: CT-bound deposits are made whole first,
		// proxy deposits second, refunds last. (Set directly: only the split math is under test.)
		ExpectedReserves::<Test>::insert(&dave, ExpectedReserve { ct: 50, proxy: 30, refund: 40 });

		let w = withdraw(&dave).expect("migrates");

		assert_eq!(
			ct_holds(&w),
			vec![(PortableHoldReason::UnnamedReserve, 50), (PortableHoldReason::ProxyDeposit, 30),]
		);
		// Of the refundable 40 only 20 reserve was left; it becomes liquid.
		assert!(
			migrator_events().contains(&Event::DepositRefunded { who: dave.clone(), amount: 20 })
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
		AccountsMigrator::<Test>::build_expected_reserves();

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
			AccountsMigrator::<Test>::build_expected_reserves();
			let w = withdraw(&multisigish).expect("migrates");
			assert_eq!(w.ct.as_ref().unwrap().free, 100);
			assert_eq!(w.ah, Some((multisigish, 400)));
		});
	});
}

#[test]
fn withdraw_keeps_sub_ah_ed_dust_with_the_deposit() {
	new_test_ext().execute_with(|| {
		let heidi = acc(8); // deposit holder whose teleport remainder would be below AH's ED
		fund(&heidi, 404);
		<Balances as ReservableCurrency<AccountId32>>::reserve(&heidi, 300).unwrap();
		ExpectedReserves::<Test>::insert(&heidi, ExpectedReserve { ct: 300, ..Default::default() });

		let w = withdraw(&heidi).expect("migrates");

		// liquid 104: buffer 100 + remainder 4 < AH ED (5) -> the dust follows the deposit.
		assert_eq!(w.ct.as_ref().unwrap().free, 104);
		assert_eq!(w.ah, None);
	});
}

#[test]
fn can_migrate_keeps_module_below_ed_and_locked_accounts() {
	new_test_ext().execute_with(|| {
		// Module accounts stay for the sweep stage.
		fund(&pot(), 500);
		assert_eq!(withdraw(&pot()), None);
		assert!(frame_system::Account::<Test>::contains_key(&pot()));

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
		<Balances as ReservableCurrency<AccountId32>>::reserve(&ida, 300).unwrap();
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
		let info = frame_system::Account::<Test>::get(&ida);
		assert_eq!(info.data.free + info.data.reserved, 0);
		assert_eq!(info.consumers, 1);
		assert_eq!(total_issuance(), ti_before - 1_000);
		assert!(migrator_events()
			.contains(&Event::AccountShellDrained { who: ida.clone(), amount: 1_000 }));
	});
}

#[test]
fn withdraw_translates_child_sovereigns_to_sibling_addresses() {
	new_test_ext().execute_with(|| {
		open_channel(2000, 2001, 70, 30); // funds + reserves on the child sovereigns
		AccountsMigrator::<Test>::build_expected_reserves();

		let w = withdraw(&child_sov(2000)).expect("migrates");

		let ct = w.ct.as_ref().unwrap();
		assert_eq!(ct.who, migrator_types::sibling_account::<AccountId32>(2000));
		assert_eq!(ct_holds(&w), vec![(PortableHoldReason::UnnamedReserve, 70)]);
	});
}

// ---------------------------------------------------------------------------
// Accounts stage: batching, tracker, rollback
// ---------------------------------------------------------------------------

#[test]
fn accounts_stage_tracks_and_sends_exactly_what_it_burns() {
	new_test_ext().execute_with(|| {
		let alice = acc(1); // parachain manager
		fund(&alice, 1_000);
		register_para(2000, &alice);
		let ti_before = total_issuance();

		assert_ok!(Rc2Migrator::force_set_stage(root(), Stage::AccountsInit));
		run_block(); // AccountsInit: seeds tracker, builds the index
		run_block(); // AccountsOngoing: migrates everything and finishes

		assert_eq!(RcMigrationStage::<Test>::get(), Stage::AccountsDone);
		let tracker = RcMigratedBalance::<Test>::get();
		assert_eq!(tracker.ct_reserved, 300);
		assert_eq!(tracker.ct_free, 100);
		assert_eq!(tracker.ah_free, 600);
		assert_eq!(tracker.kept, ti_before - 1_000);
		assert_eq!(total_issuance(), tracker.kept);

		// The messages carry exactly the burned pieces.
		let sent = take_sent_xcm();
		let ct_calls = decode_ct_calls(&sent);
		assert_eq!(
			ct_calls,
			vec![CtMigratorCall::ReceiveAccounts {
				accounts: vec![migrator_types::PortableAccount {
					who: alice.clone(),
					free: 100,
					holds: vec![migrator_types::PortableHold {
						reason: PortableHoldReason::UnnamedReserve,
						amount: 300,
					}]
					.try_into()
					.unwrap(),
				}],
			}]
		);
		assert_eq!(decode_teleports(&sent), vec![vec![(alice, 600)]]);
	});
}

#[test]
fn accounts_stage_rolls_back_whole_block_when_a_send_fails() {
	new_test_ext().execute_with(|| {
		let alice = acc(1);
		fund(&alice, 1_000);
		register_para(2000, &alice);
		AccountsMigrator::<Test>::build_expected_reserves();
		seed_tracker();
		let tracker_before = RcMigratedBalance::<Test>::get();
		let ti_before = total_issuance();

		// WHEN every send fails, the block's work must roll back whole: nothing burned, nothing
		// sent, cursor unchanged — the same range is retried next block.
		FailSends::set(true);
		let result = migrator_types::with_rollback(|| AccountsMigrator::<Test>::migrate_many(None));
		assert!(matches!(result, Err(Error::<Test>::XcmSendFailed)));
		assert_eq!(free(&alice), 700);
		assert_eq!(reserved(&alice), 300);
		assert_eq!(total_issuance(), ti_before);
		assert_eq!(RcMigratedBalance::<Test>::get(), tracker_before);
		assert!(sent_xcm().is_empty(), "a rolled-back block must not leave messages behind");

		// AND the retry succeeds once sending recovers.
		FailSends::set(false);
		let result = migrator_types::with_rollback(|| AccountsMigrator::<Test>::migrate_many(None));
		assert!(matches!(result, Ok(None)));
		assert!(!frame_system::Account::<Test>::contains_key(&alice));
		assert_eq!(decode_ct_calls(&take_sent_xcm()).len(), 1);
	});
}

#[test]
fn accounts_stage_stops_at_the_per_block_limit_and_resumes_from_the_cursor() {
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
		seed_tracker();

		let cursor = migrator_types::with_rollback(|| AccountsMigrator::<Test>::migrate_many(None))
			.expect("first block succeeds");
		let cursor = cursor.expect("more accounts remain than the per-block limit");

		let done =
			migrator_types::with_rollback(|| AccountsMigrator::<Test>::migrate_many(Some(cursor)))
				.expect("second block succeeds");
		assert_eq!(done, None, "two blocks cover everything");

		// Every account is gone and the ledger is exact: all free balance teleported.
		assert_eq!(frame_system::Account::<Test>::iter().count(), 0);
		let tracker = RcMigratedBalance::<Test>::get();
		assert_eq!(tracker.ah_free, ti_before);
		assert_eq!(tracker.kept, 0);
		assert_eq!(total_issuance(), 0);
	});
}

// ---------------------------------------------------------------------------
// Proxy stage
// ---------------------------------------------------------------------------

#[test]
fn proxy_stage_sends_portable_defs_and_deletes_migrated_delegators() {
	new_test_ext().execute_with(|| {
		let bob = acc(2); // migrated delegator with one portable and one untranslatable def
		let d1 = acc(21);
		let d2 = acc(22);
		fund(&bob, 548);
		add_proxy(&bob, &d1, ProxyType::Any);
		add_proxy(&bob, &d2, ProxyType::Staking);
		AccountsMigrator::<Test>::build_expected_reserves();
		seed_tracker();
		migrator_types::with_rollback(|| AccountsMigrator::<Test>::migrate_many(None)).unwrap();
		assert!(!frame_system::Account::<Test>::contains_key(&bob));
		take_sent_xcm();

		let done = proxy::ProxyMigrator::<Test>::migrate_many(None).unwrap();
		assert_eq!(done, None);

		// The portable definition travelled; the whole entry is deleted — the delegator's account
		// is gone, so a record here could only claim money that left.
		assert!(!pallet_proxy::Proxies::<Test>::contains_key(&bob));
		assert_eq!(
			decode_ct_calls(&take_sent_xcm()),
			vec![CtMigratorCall::ReceiveProxies {
				proxies: vec![migrator_types::PortableProxy {
					delegator: bob,
					delegates: vec![PortableProxyDelegate {
						delegate: d1,
						proxy_type: PortableProxyType::Any,
						delay: 0,
					}]
					.try_into()
					.unwrap(),
				}],
			}]
		);
	});
}

#[test]
fn proxy_stage_clamps_entries_of_accounts_that_stay() {
	new_test_ext().execute_with(|| {
		let carol = acc(3); // shell-drained delegator (session keys): record stays, money left
		let d1 = acc(21);
		let d2 = acc(22);
		fund(&carol, 548);
		add_proxy(&carol, &d1, ProxyType::Any);
		add_proxy(&carol, &d2, ProxyType::Staking);
		frame_system::Pallet::<Test>::inc_consumers(&carol).unwrap();
		AccountsMigrator::<Test>::build_expected_reserves();
		seed_tracker();
		migrator_types::with_rollback(|| AccountsMigrator::<Test>::migrate_many(None)).unwrap();
		assert_eq!(reserved(&carol), 0, "shell-drained");

		proxy::ProxyMigrator::<Test>::migrate_many(None).unwrap();

		// The untranslatable def stays, but the recorded deposit is clamped to the (zero) reserve
		// so the entry never claims money that is gone.
		let (defs, deposit) = pallet_proxy::Proxies::<Test>::get(&carol);
		assert_eq!(defs.len(), 1);
		assert_eq!(defs[0].proxy_type, ProxyType::Staking);
		assert_eq!(deposit, 0);
	});
}

#[test]
fn proxy_stage_deletes_fundless_husk_entries() {
	new_test_ext().execute_with(|| {
		let husk = acc(12); // v1 leftover: proxy entry, no account behind it
		let d1 = acc(21);
		let def = pallet_proxy::ProxyDefinition {
			delegate: d1.clone(),
			proxy_type: ProxyType::Any,
			delay: 0u32,
		};
		pallet_proxy::Proxies::<Test>::insert(
			&husk,
			(frame_support::BoundedVec::truncate_from(vec![def]), 0u128),
		);

		proxy::ProxyMigrator::<Test>::migrate_many(None).unwrap();

		// The record is cleaned up; its (manager-linked) definition still travels.
		assert!(!pallet_proxy::Proxies::<Test>::contains_key(&husk));
		assert_eq!(decode_ct_calls(&take_sent_xcm()).len(), 1);
	});
}

#[test]
fn announcement_records_of_migrated_announcers_are_dropped() {
	new_test_ext().execute_with(|| {
		let frank = acc(6); // delegator
		let eve = acc(5); // announcer whose account migrates away
		let ada = acc(13); // announcer who stays (locked account)
		fund(&frank, 500);
		add_proxy(&frank, &eve, ProxyType::Any);
		add_proxy(&frank, &ada, ProxyType::Any);
		fund(&eve, 500);
		fund(&ada, 500);
		assert_ok!(Proxy::announce(
			RuntimeOrigin::signed(eve.clone()),
			frank.clone(),
			H256::zero()
		));
		assert_ok!(Proxy::announce(
			RuntimeOrigin::signed(ada.clone()),
			frank.clone(),
			H256::zero()
		));
		<Balances as LockableCurrency<AccountId32>>::set_lock(
			*b"testlock",
			&ada,
			100,
			WithdrawReasons::all(),
		);
		AccountsMigrator::<Test>::build_expected_reserves();
		// Announcement deposits are refunds: they teleport to AH with the announcer's balance.
		assert_eq!(ExpectedReserves::<Test>::get(&eve).refund, 31);
		seed_tracker();
		migrator_types::with_rollback(|| AccountsMigrator::<Test>::migrate_many(None)).unwrap();
		assert!(!frame_system::Account::<Test>::contains_key(&eve));

		proxy::ProxyMigrator::<Test>::drain_announcements().unwrap();

		// Migrated announcer: record dropped (deposit was refunded). Kept announcer: record and
		// reserve intact (the proxy deposit itself sits on frank, the delegator).
		assert!(!pallet_proxy::Announcements::<Test>::contains_key(&eve));
		let (_, deposit) = pallet_proxy::Announcements::<Test>::get(&ada);
		assert_eq!(deposit, 31);
		assert_eq!(reserved(&ada), 31);
	});
}

// ---------------------------------------------------------------------------
// Registrar stage
// ---------------------------------------------------------------------------

#[test]
fn registrar_stage_moves_next_free_id_and_drains_records() {
	new_test_ext().execute_with(|| {
		let alice = acc(1);
		fund(&alice, 1_000);
		register_para(2000, &alice); // bumps NextFreeParaId to 2001

		registrar::RegistrarMigrator::<Test>::migrate_init().unwrap();
		assert_eq!(
			decode_ct_calls(&take_sent_xcm()),
			vec![CtMigratorCall::ReceiveRegistrar { paras: vec![], next_free_para_id: Some(2001) }]
		);
		assert_eq!(paras_registrar::NextFreeParaId::<Test>::get(), 0.into(), "killed on this side");

		let done = registrar::RegistrarMigrator::<Test>::migrate_many(None).unwrap();
		assert_eq!(done, None);
		assert!(paras_registrar::Paras::<Test>::iter().next().is_none());
		assert_eq!(
			decode_ct_calls(&take_sent_xcm()),
			vec![CtMigratorCall::ReceiveRegistrar {
				paras: vec![migrator_types::PortableParaInfo {
					para_id: 2000,
					manager: alice,
					deposit: 300,
					// Never locked, and it travels as such: the relay chain only sets this at a
					// para's first head, and this one has never produced one.
					locked: None,
					// The test para has a registrar record but was never onboarded, so it has no
					// lifecycle and no head data — it travels as a reserved id.
					registered: false,
					head_len: 0,
				}],
				next_free_para_id: None,
			}]
		);
	});
}

// ---------------------------------------------------------------------------
// HRMP stage
// ---------------------------------------------------------------------------

/// The HRMP stage copies records to Coretime and keeps the relay chain's own, because the relay
/// chain still routes every message through `HrmpChannels` and still completes handshakes from
/// `HrmpOpenChannelRequests` at a session boundary. Only the deposits move.
#[test]
fn hrmp_stage_copies_requests_and_channels_and_keeps_them_deposit_free() {
	new_test_ext().execute_with(|| {
		// GIVEN one open channel with deposits at both ends, and one unconfirmed request.
		open_channel(2000, 2001, 70, 30);
		open_request(2000, 2002, 25);

		// WHEN the requests are copied.
		hrmp::HrmpMigrator::<Test>::copy_open_requests().unwrap();

		// THEN Coretime is told the deposit that was taken...
		assert_eq!(
			decode_ct_calls(&take_sent_xcm()),
			vec![CtMigratorCall::ReceiveHrmpRequests {
				requests: vec![migrator_types::PortableHrmpRequest {
					sender: 2000,
					recipient: 2002,
					confirmed: false,
					sender_deposit: 25,
					max_message_size: 1024,
					max_capacity: 8,
					max_total_size: 4096,
				}],
			}]
		);
		assert!(migrator_events().contains(&Event::HrmpRequestsSent { count: 1 }));

		// ...and the request stays here so the session boundary can still promote it, with the
		// deposit zeroed so a cancellation does not refund money that has left the chain. The
		// counts stay too: the relay chain still bounds requests per para.
		let request_id =
			HrmpChannelId { sender: ParaId::from(2000), recipient: ParaId::from(2002) };
		let request = parachains_hrmp::HrmpOpenChannelRequests::<Test>::get(&request_id)
			.expect("the open request must stay on the relay chain");
		assert_eq!(request.sender_deposit, 0);
		assert_eq!(parachains_hrmp::HrmpOpenChannelRequestsList::<Test>::get(), vec![request_id]);
		assert_eq!(
			parachains_hrmp::HrmpOpenChannelRequestCount::<Test>::get(ParaId::from(2000)),
			1
		);

		// WHEN the channels are copied.
		let done = hrmp::HrmpMigrator::<Test>::migrate_many(None).unwrap();
		assert_eq!(done, None);

		// THEN Coretime is told both deposits...
		assert_eq!(
			decode_ct_calls(&take_sent_xcm()),
			vec![CtMigratorCall::ReceiveHrmp {
				channels: vec![migrator_types::PortableHrmpChannel {
					sender: 2000,
					recipient: 2001,
					max_capacity: 8,
					max_total_size: 4096,
					max_message_size: 1024,
					sender_deposit: 70,
					recipient_deposit: 30,
				}],
			}]
		);

		// ...and the channel stays here, deposit-free, so messages still route.
		let channel_id =
			HrmpChannelId { sender: ParaId::from(2000), recipient: ParaId::from(2001) };
		let channel = parachains_hrmp::HrmpChannels::<Test>::get(&channel_id)
			.expect("the channel must stay on the relay chain: it is what routes messages");
		assert_eq!(channel.sender_deposit, 0);
		assert_eq!(channel.recipient_deposit, 0);
		assert_eq!(channel.max_capacity, 8);
		assert_eq!(channel.max_message_size, 1024);
	});
}

// ---------------------------------------------------------------------------
// Sweep and TI correction
// ---------------------------------------------------------------------------

#[test]
fn sweep_empties_pots_reaps_dust_and_teleports_to_the_beneficiary() {
	new_test_ext().execute_with(|| {
		let dusty = acc(9); // reapable below-ED account
		let backed_dust = acc(14); // below-ED with a broken reserve (holds a consumer ref)
		let husk = acc(15); // zero balance, alive only via a stale provider ref
		let modl_dust = {
			let mut bytes = [0u8; 32];
			bytes[..8].copy_from_slice(b"modlxyz\0");
			AccountId32::new(bytes)
		};
		fund(&pot(), 500);
		// The treasury pot's balance is book-kept as inactive; the sweep must reactivate it.
		pallet_balances::InactiveIssuance::<Test>::put(500);
		force_anomalous_account(&dusty, 4, 0, 0);
		force_anomalous_account(&backed_dust, 2, 3, 1);
		force_anomalous_account(&husk, 0, 0, 0);
		force_anomalous_account(&modl_dust, 4, 0, 0);
		seed_tracker();

		assert_ok!(Rc2Migrator::force_set_stage(root(), Stage::Sweep));
		migrator_events();
		// Pots and dust are separate stages: one block empties the pots, the next pages the
		// dust (everything here fits one page), and the machine lands on TiCorrection.
		run_block();
		assert_eq!(RcMigrationStage::<Test>::get(), Stage::SweepDust { last_key: None });
		run_block();

		assert_eq!(RcMigrationStage::<Test>::get(), Stage::TiCorrection);
		assert!(!frame_system::Account::<Test>::contains_key(&pot()));
		assert_eq!(pallet_balances::InactiveIssuance::<Test>::get(), 0);
		assert!(!frame_system::Account::<Test>::contains_key(&dusty));
		assert!(!frame_system::Account::<Test>::contains_key(&backed_dust));
		assert!(!frame_system::Account::<Test>::contains_key(&husk));
		// Module accounts are never dust-reaped; the sweep only empties the configured pots.
		assert_eq!(free(&modl_dust), 4);

		let events = migrator_events();
		assert!(events.contains(&Event::AccountSwept { who: pot(), amount: 500 }));
		assert!(events.contains(&Event::DustSwept { count: 2, amount: 4 + 5 }));
		assert!(events.contains(&Event::HusksReaped { count: 1 }));

		// Each stage teleports its own proceeds — the pots (500), then the dust page (4 + 5) —
		// and the ledger moves with them.
		assert_eq!(
			decode_teleports(&take_sent_xcm()),
			vec![vec![(acc(200), 500)], vec![(acc(200), 9)]]
		);
		let tracker = RcMigratedBalance::<Test>::get();
		assert_eq!(tracker.ah_free, 509);
		assert_eq!(total_issuance(), tracker.kept);
	});
}

#[test]
fn ti_correction_burns_the_audited_phantom_and_signals_finish() {
	new_test_ext().execute_with(|| {
		// GIVEN issuance that no account holds (the audited anomaly) and nothing else.
		pallet_balances::TotalIssuance::<Test>::put(50);
		TiCorrection::set(50);
		seed_tracker();

		assert_ok!(Rc2Migrator::force_set_stage(root(), Stage::TiCorrection));
		migrator_events();
		run_block();

		assert!(matches!(RcMigrationStage::<Test>::get(), Stage::CoolOff { .. }));
		assert_eq!(total_issuance(), 0);
		let tracker = RcMigratedBalance::<Test>::get();
		assert_eq!(tracker.ti_corrected, 50);
		assert_eq!(tracker.kept, 0);
		let events = migrator_events();
		assert!(events.contains(&Event::TiCorrected { expected: 50, unaccounted: 50, burned: 50 }));
		assert_eq!(
			decode_ct_calls(&take_sent_xcm()),
			vec![CtMigratorCall::ReconcileBalances { rc_kept: 0, rc_migrated: 0 }]
		);
	});
}

#[test]
fn ti_correction_never_burns_more_than_measured_and_reports_anomalies() {
	new_test_ext().execute_with(|| {
		// Measured phantom (30) below the audited expectation (50): burn the 30, report loudly.
		pallet_balances::TotalIssuance::<Test>::put(30);
		TiCorrection::set(50);
		seed_tracker();

		assert_ok!(Rc2Migrator::force_set_stage(root(), Stage::TiCorrection));
		migrator_events();
		run_block();

		assert_eq!(total_issuance(), 0);
		let events = migrator_events();
		assert!(events.contains(&Event::TiCorrectionAnomaly { expected: 50, unaccounted: 30 }));
		assert!(events.contains(&Event::TiCorrected { expected: 50, unaccounted: 30, burned: 30 }));

		// Hypothetically, with MORE unaccounted than audited, only the audited amount burns; the
		// excess stays on the books for investigation.
		hypothetically!({
			pallet_balances::TotalIssuance::<Test>::put(80);
			seed_tracker();
			assert_ok!(Rc2Migrator::force_set_stage(root(), Stage::TiCorrection));
			run_block();
			assert_eq!(total_issuance(), 30);
			assert!(migrator_events().contains(&Event::TiCorrected {
				expected: 50,
				unaccounted: 80,
				burned: 50,
			}));
		});
	});
}

// ---------------------------------------------------------------------------
// Scheduling and the control plane
// ---------------------------------------------------------------------------

#[test]
fn only_the_admin_origin_or_manager_drives_the_machine() {
	new_test_ext().execute_with(|| {
		let alice = acc(1); // appointed manager
		let start = now_ms() + 5 * BLOCK_TIME_MS;

		// WHEN a signed account with no appointment drives anything. THEN it is refused.
		let signed = RuntimeOrigin::signed(alice.clone());
		assert_noop!(
			Rc2Migrator::schedule_migration(signed.clone(), start, WARM_UP, COOL_OFF),
			BadOrigin
		);
		assert_noop!(Rc2Migrator::cancel_migration(signed.clone()), BadOrigin);
		assert_noop!(Rc2Migrator::force_set_stage(signed.clone(), Stage::Paused), BadOrigin);
		assert_noop!(Rc2Migrator::set_manager(signed.clone(), Some(alice.clone())), BadOrigin);

		// WHEN the admin origin appoints it manager. THEN it drives the machine, but still
		// cannot appoint a manager itself.
		assert_ok!(Rc2Migrator::set_manager(root(), Some(alice.clone())));
		assert!(
			migrator_events().contains(&Event::ManagerSet { old: None, new: Some(alice.clone()) })
		);
		assert_ok!(Rc2Migrator::schedule_migration(signed.clone(), start, WARM_UP, COOL_OFF));
		assert_eq!(RcMigrationStage::<Test>::get(), Stage::Scheduled { start });
		assert_noop!(Rc2Migrator::set_manager(signed.clone(), None), BadOrigin);

		// WHEN the admin origin removes it. THEN it loses the powers.
		assert_ok!(Rc2Migrator::set_manager(root(), None));
		assert!(migrator_events().contains(&Event::ManagerSet { old: Some(alice), new: None }));
		assert_noop!(Rc2Migrator::cancel_migration(signed), BadOrigin);
	});
}

#[test]
fn a_scheduled_migration_can_be_cancelled_and_rescheduled() {
	new_test_ext().execute_with(|| {
		// WHEN the start is now or earlier. THEN it is refused: a start already past would begin
		// the migration on the very next block.
		assert_noop!(
			Rc2Migrator::schedule_migration(root(), now_ms(), WARM_UP, COOL_OFF),
			Error::<Test>::StartInPast
		);

		// GIVEN a scheduled migration.
		let start = now_ms() + 5 * BLOCK_TIME_MS;
		assert_ok!(Rc2Migrator::schedule_migration(root(), start, WARM_UP, COOL_OFF));

		// WHEN it is scheduled again. THEN it is refused, so a second call cannot silently move
		// a start date that is already committed.
		assert_noop!(
			Rc2Migrator::schedule_migration(root(), start + 1, WARM_UP, COOL_OFF),
			Error::<Test>::AlreadyScheduled
		);

		// WHEN it is cancelled. THEN the machine is pending again and can take a new start.
		assert_ok!(Rc2Migrator::cancel_migration(root()));
		assert_eq!(RcMigrationStage::<Test>::get(), Stage::Pending);
		assert_ok!(Rc2Migrator::schedule_migration(root(), start, WARM_UP, COOL_OFF));

		// WHEN the handshake has begun. THEN cancelling is refused: the Coretime chain has been
		// told something this call cannot take back.
		run_blocks(6);
		assert_eq!(RcMigrationStage::<Test>::get(), Stage::WaitingForCt);
		assert_noop!(Rc2Migrator::cancel_migration(root()), Error::<Test>::NotScheduled);
	});
}

#[test]
fn nothing_moves_until_the_coretime_chain_confirms() {
	new_test_ext().execute_with(|| {
		// GIVEN a machine that has sent its handshake.
		let start = now_ms() + BLOCK_TIME_MS;
		assert_ok!(Rc2Migrator::schedule_migration(root(), start, WARM_UP, COOL_OFF));
		run_blocks(2);
		assert_eq!(RcMigrationStage::<Test>::get(), Stage::WaitingForCt);

		// WHEN many blocks pass with no answer. THEN the machine holds and sends nothing more:
		// a Coretime chain that never confirms must not be sent data anyway.
		let sent_so_far = sent_xcm().len();
		run_blocks(20);
		assert_eq!(RcMigrationStage::<Test>::get(), Stage::WaitingForCt);
		assert_eq!(sent_xcm().len(), sent_so_far);

		// WHEN anyone other than the Coretime chain confirms, root included. THEN it is refused.
		assert_noop!(Rc2Migrator::ct_ready(RuntimeOrigin::signed(acc(1))), BadOrigin);
		assert_noop!(Rc2Migrator::ct_ready(root()), BadOrigin);
		assert_eq!(RcMigrationStage::<Test>::get(), Stage::WaitingForCt);

		// WHEN the Coretime chain confirms. THEN the warm-up runs for the scheduled window, and
		// only then does the first data stage begin.
		let at = System::block_number();
		assert_ok!(Rc2Migrator::ct_ready(RuntimeOrigin::signed(coretime())));
		assert_eq!(RcMigrationStage::<Test>::get(), Stage::WarmUp { end_at: at + WARM_UP });

		// WHEN it confirms again, as it does after a re-run handshake. THEN nothing changes.
		assert_ok!(Rc2Migrator::ct_ready(RuntimeOrigin::signed(coretime())));
		assert_eq!(RcMigrationStage::<Test>::get(), Stage::WarmUp { end_at: at + WARM_UP });
		run_blocks(WARM_UP - 1);
		assert_eq!(RcMigrationStage::<Test>::get(), Stage::WarmUp { end_at: at + WARM_UP });
		run_blocks(1);
		assert_eq!(RcMigrationStage::<Test>::get(), Stage::AccountsInit);
	});
}

/// A multisig member: the keypair that signs, and the account the pallet knows it by.
fn member(seed: u8) -> (sr25519::Pair, AccountId32) {
	let pair = sr25519::Pair::from_seed(&[seed; 32]);
	let who = MultiSigner::Sr25519(pair.public()).into_account();
	(pair, who)
}

/// One member's signed vote for `call` in the current round, ready to submit.
fn vote(
	pair: &sr25519::Pair,
	call: RuntimeCall,
) -> (Box<ManagerMultisigVote<Test>>, MultiSignature) {
	let payload = ManagerMultisigVote::<Test>::new(
		MultiSigner::Sr25519(pair.public()),
		call,
		ManagerMultisigRound::<Test>::get(),
	);
	let sig = MultiSignature::Sr25519(pair.sign(&payload.encode_with_bytes_wrapper()));
	(Box::new(payload), sig)
}

#[test]
fn the_manager_multisig_drives_the_machine_once_its_threshold_is_met() {
	new_test_ext().execute_with(|| {
		let (alice, alice_id) = member(1); // multisig member
		let (bob, bob_id) = member(2); // multisig member
		let (_carol, carol_id) = member(3); // multisig member who never votes
		MultisigMembers::set(vec![alice_id.clone(), bob_id.clone(), carol_id]);

		// GIVEN the runtime upgrade seeded this network's round, and a scheduled migration.
		<Rc2Migrator as OnRuntimeUpgrade>::on_runtime_upgrade();
		assert_eq!(ManagerMultisigRound::<Test>::get(), MultisigStartRound::get());
		let start = now_ms() + 5 * BLOCK_TIME_MS;
		assert_ok!(Rc2Migrator::schedule_migration(root(), start, WARM_UP, COOL_OFF));
		let cancel = RuntimeCall::Rc2Migrator(crate::Call::<Test>::cancel_migration {});

		// WHEN one member votes to cancel. THEN the vote is recorded and nothing is dispatched.
		let (payload, sig) = vote(&alice, cancel.clone());
		assert_ok!(Rc2Migrator::vote_manager_multisig(RuntimeOrigin::none(), payload, sig));
		assert_eq!(RcMigrationStage::<Test>::get(), Stage::Scheduled { start });
		assert!(migrator_events().contains(&Event::ManagerMultisigVoted { votes: 1 }));

		// WHEN the same member votes again for the same call. THEN it is refused.
		let (payload, sig) = vote(&alice, cancel.clone());
		assert_noop!(
			Rc2Migrator::vote_manager_multisig(RuntimeOrigin::none(), payload, sig),
			Error::<Test>::DuplicateVote
		);

		// WHEN a second member votes. THEN the threshold is met, the call is dispatched as the
		// multisig's account, and the round advances so the votes cannot be replayed.
		let (payload, sig) = vote(&bob, cancel.clone());
		assert_ok!(Rc2Migrator::vote_manager_multisig(RuntimeOrigin::none(), payload, sig));
		assert_eq!(RcMigrationStage::<Test>::get(), Stage::Pending);
		assert!(migrator_events().contains(&Event::ManagerMultisigDispatched { res: Ok(()) }));
		assert_eq!(ManagerMultisigRound::<Test>::get(), MultisigStartRound::get() + 1);
		assert_eq!(ManagerMultisigs::<Test>::iter().count(), 0);
		assert_eq!(ManagerVotesInCurrentRound::<Test>::iter().count(), 0);

		// WHEN a vote from the previous round arrives. THEN it is refused as stale.
		let stale = ManagerMultisigVote::<Test>::new(
			MultiSigner::Sr25519(alice.public()),
			cancel.clone(),
			MultisigStartRound::get(),
		);
		let sig = MultiSignature::Sr25519(alice.sign(&stale.encode_with_bytes_wrapper()));
		assert_noop!(
			Rc2Migrator::vote_manager_multisig(RuntimeOrigin::none(), Box::new(stale), sig),
			Error::<Test>::UnsignedValidationFailed
		);
	});
}

#[test]
fn only_members_with_a_valid_signature_may_vote() {
	new_test_ext().execute_with(|| {
		let (alice, alice_id) = member(1); // multisig member
		let (mallory, _) = member(9); // not a member
		MultisigMembers::set(vec![alice_id]);
		let cancel = RuntimeCall::Rc2Migrator(crate::Call::<Test>::cancel_migration {});

		// WHEN a non-member votes. THEN it is refused.
		let (payload, sig) = vote(&mallory, cancel.clone());
		assert_noop!(
			Rc2Migrator::vote_manager_multisig(RuntimeOrigin::none(), payload, sig),
			Error::<Test>::UnsignedValidationFailed
		);

		// WHEN a member's vote carries somebody else's signature. THEN it is refused.
		let (payload, _) = vote(&alice, cancel.clone());
		let forged = MultiSignature::Sr25519(mallory.sign(&payload.encode_with_bytes_wrapper()));
		assert_noop!(
			Rc2Migrator::vote_manager_multisig(RuntimeOrigin::none(), payload, forged),
			Error::<Test>::UnsignedValidationFailed
		);

		// WHEN a vote is submitted signed rather than as an inherent. THEN it is refused: the
		// point of the unsigned path is that members need no funded account here.
		let (payload, sig) = vote(&alice, cancel);
		assert_noop!(
			Rc2Migrator::vote_manager_multisig(RuntimeOrigin::signed(acc(1)), payload, sig),
			BadOrigin
		);
	});
}

#[test]
fn the_manager_stays_funded_until_the_end_and_is_then_reaped() {
	new_test_ext().execute_with(|| {
		let manager = acc(40); // governance-appointed manager
		fund(&manager, 1_000);
		assert_ok!(Rc2Migrator::set_manager(root(), Some(manager.clone())));

		// GIVEN a migration the manager scheduled and the Coretime chain confirmed.
		let start = now_ms() + BLOCK_TIME_MS;
		assert_ok!(Rc2Migrator::schedule_migration(
			RuntimeOrigin::signed(manager.clone()),
			start,
			WARM_UP,
			COOL_OFF
		));
		run_blocks(2);
		assert_ok!(Rc2Migrator::ct_ready(RuntimeOrigin::signed(coretime())));

		// WHEN the accounts stage has run. THEN the manager still has its balance: it is the one
		// account the drain leaves alone, so it can keep paying for the calls that drive this.
		for _ in 0..40 {
			if RcMigrationStage::<Test>::get() == Stage::AccountsDone {
				break;
			}
			run_blocks(1);
		}
		assert_eq!(RcMigrationStage::<Test>::get(), Stage::AccountsDone);
		assert_eq!(Balances::free_balance(&manager), 1_000);

		// WHEN the migration ends. THEN the appointment is over and the balance has left for the
		// same account on Asset Hub.
		for _ in 0..60 {
			if RcMigrationStage::<Test>::get().is_finished() {
				break;
			}
			run_blocks(1);
		}
		assert_eq!(RcMigrationStage::<Test>::get(), Stage::MigrationDone);
		assert_eq!(Manager::<Test>::get(), None);
		assert_eq!(Balances::free_balance(&manager), 0);
		assert!(migrator_events()
			.contains(&Event::ManagerReaped { who: manager.clone(), amount: 1_000 }));
		assert!(decode_teleports(&sent_xcm())
			.iter()
			.any(|batch| batch.contains(&(manager.clone(), 1_000))));
	});
}

#[test]
fn the_coretime_queue_goes_first_on_a_duty_cycle_while_the_migration_runs() {
	new_test_ext().execute_with(|| {
		let ct_queue = AggregateMessageOrigin::Ump(UmpQueueId::Para(CT_PARA_ID.into()));
		// The mock's pattern: three blocks of priority, one of round robin.
		let prioritised_blocks = |from: u32, to: u32| -> Vec<AggregateMessageOrigin> {
			(from..=to).filter(|n| n % 4 < 3).map(|_| ct_queue.clone()).collect()
		};

		// GIVEN a pending machine. WHEN blocks pass. THEN no queue is forced: there is nothing to
		// protect yet.
		run_blocks(8);
		assert_eq!(ForcedHeads::get(), vec![]);

		// GIVEN an ongoing migration. WHEN blocks pass. THEN the Coretime queue goes first on
		// three blocks in four.
		assert_ok!(Rc2Migrator::force_set_stage(root(), Stage::Paused));
		let from = System::block_number() + 1;
		run_blocks(8);
		assert_eq!(ForcedHeads::get(), prioritised_blocks(from, from + 7));
		assert_eq!(
			migrator_events()
				.iter()
				.filter(|e| matches!(e, Event::CtUmpQueuePrioritised { cycle_period: 4, .. }))
				.count(),
			6
		);

		// WHEN the priority is disabled. THEN nothing is forced.
		assert_ok!(Rc2Migrator::set_ct_ump_queue_priority(root(), QueuePriority::Disabled));
		ForcedHeads::set(vec![]);
		run_blocks(4);
		assert_eq!(ForcedHeads::get(), vec![]);

		// WHEN the pattern is overridden to one block in two. THEN that is the cycle.
		assert_ok!(Rc2Migrator::set_ct_ump_queue_priority(
			root(),
			QueuePriority::OverrideConfig(1, 1)
		));
		let from = System::block_number() + 1;
		run_blocks(4);
		assert_eq!(ForcedHeads::get().len(), (from..from + 4).filter(|n| n % 2 == 0).count());

		// WHEN the same configuration is set again, or one that never prioritises. THEN refused.
		assert_noop!(
			Rc2Migrator::set_ct_ump_queue_priority(root(), QueuePriority::OverrideConfig(1, 1)),
			Error::<Test>::QueuePriorityAlreadySet
		);
		assert_noop!(
			Rc2Migrator::set_ct_ump_queue_priority(root(), QueuePriority::OverrideConfig(0, 5)),
			Error::<Test>::ZeroPriorityBlocks
		);

		// WHEN the migration is done. THEN the queue takes its turn like every other.
		assert_ok!(Rc2Migrator::force_set_stage(root(), Stage::MigrationDone));
		ForcedHeads::set(vec![]);
		run_blocks(4);
		assert_eq!(ForcedHeads::get(), vec![]);
	});
}

// ---------------------------------------------------------------------------
// The whole machine
// ---------------------------------------------------------------------------

#[test]
fn full_stage_machine_drains_the_chain_to_zero() {
	new_test_ext().execute_with(|| {
		let alice = acc(1); // parachain manager
		let pure = acc(30); // keyless pure delegator
		let delegate = acc(31);
		let dusty = acc(9); // reapable dust
		fund(&alice, 1_000);
		register_para(2000, &alice);
		open_channel(2000, 2001, 70, 30);
		open_request(2000, 2002, 25);
		fund(&pure, 544);
		add_proxy(&pure, &delegate, ProxyType::Any);
		fund(&pot(), 500);
		force_anomalous_account(&dusty, 4, 0, 0);
		// The audited phantom issuance.
		pallet_balances::TotalIssuance::<Test>::mutate(|ti| *ti += 50);
		TiCorrection::set(50);
		let ti_start = total_issuance();

		// WHEN the migration is scheduled and the clock passes its start.
		let start = now_ms() + BLOCK_TIME_MS;
		assert_ok!(Rc2Migrator::schedule_migration(root(), start, WARM_UP, COOL_OFF));
		run_blocks(2);

		// THEN nothing has moved: the relay chain is waiting for the Coretime chain to confirm.
		assert_eq!(RcMigrationStage::<Test>::get(), Stage::WaitingForCt);
		assert_eq!(
			decode_ct_calls(&sent_xcm()),
			vec![CtMigratorCall::StartMigration],
			"the first thing sent is the handshake, before any data"
		);

		// WHEN the Coretime chain confirms, and the machine runs to Done one stage per block.
		let handshake_at = System::block_number();
		assert_ok!(Rc2Migrator::ct_ready(RuntimeOrigin::signed(coretime())));
		assert_eq!(
			RcMigrationStage::<Test>::get(),
			Stage::WarmUp { end_at: handshake_at + WARM_UP }
		);
		for _ in 0..60 {
			if RcMigrationStage::<Test>::get().is_finished() {
				break;
			}
			run_blocks(1);
		}

		// THEN it finishes on schedule, measured from the handshake: the warm-up, then 15 working
		// blocks (the dust pass is its own stage), then the cool-off window.
		assert_eq!(RcMigrationStage::<Test>::get(), Stage::MigrationDone);
		assert_eq!(System::block_number(), handshake_at + WARM_UP + 15 + COOL_OFF);

		// The records Coretime now owns outright are drained...
		assert!(paras_registrar::Paras::<Test>::iter().next().is_none());
		assert!(pallet_proxy::Proxies::<Test>::iter().next().is_none());
		// ...while the HRMP records the relay chain still reads stay, deposit-free. It routes
		// every message through `HrmpChannels` and promotes handshakes from
		// `HrmpOpenChannelRequests` at a session boundary; draining either would stop parachains
		// talking to each other.
		for (_, channel) in parachains_hrmp::HrmpChannels::<Test>::iter() {
			assert_eq!(channel.sender_deposit, 0);
			assert_eq!(channel.recipient_deposit, 0);
		}
		for (_, request) in parachains_hrmp::HrmpOpenChannelRequests::<Test>::iter() {
			assert_eq!(request.sender_deposit, 0);
		}
		// ...every account is gone...
		assert_eq!(frame_system::Account::<Test>::iter().count(), 0);
		// ...and the ledger balances to zero, exactly.
		let tracker = RcMigratedBalance::<Test>::get();
		assert_eq!(
			tracker.kept +
				tracker.ct_reserved +
				tracker.ct_free +
				tracker.ah_free +
				tracker.ti_corrected,
			ti_start,
			"conservation is exact"
		);
		assert_eq!(tracker.kept, 0, "the relay chain drains to exactly zero");
		assert_eq!(total_issuance(), 0);
		assert_eq!(tracker.ti_corrected, 50);
	});
}

#[test]
fn force_set_stage_requires_root() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Rc2Migrator::force_set_stage(RuntimeOrigin::signed(acc(1)), Stage::Paused),
			BadOrigin
		);
		assert_ok!(Rc2Migrator::force_set_stage(root(), Stage::Paused));
		assert_eq!(RcMigrationStage::<Test>::get(), Stage::Paused);
		// Paused halts the machine: blocks pass, nothing moves.
		run_block();
		assert_eq!(RcMigrationStage::<Test>::get(), Stage::Paused);
	});
}

// ---------------------------------------------------------------------------
// Batch confirmation: the relay chain does not run ahead of the Coretime chain
// ---------------------------------------------------------------------------

#[test]
fn data_extraction_pauses_once_too_many_batches_are_outstanding() {
	new_test_ext().execute_with(|| {
		// Allow nothing in flight, so a single unacknowledged batch trips the gate. The mock's
		// fixture is far smaller than a real chain's, so this is how the threshold is reached.
		assert_eq!(Rc2Migrator::unprocessed_msg_buffer(), 1);
		assert_ok!(Rc2Migrator::set_unprocessed_msg_buffer(root(), Some(0)));
		assert_eq!(Rc2Migrator::unprocessed_msg_buffer(), 0);
		let alice = acc(1); // manager; its registrar deposit is what travels to Coretime
		fund(&alice, 10_000);
		register_para(2000, &alice);

		assert_ok!(Rc2Migrator::force_set_stage(root(), Stage::AccountsInit));
		run_blocks_without_ct(2);
		let outstanding = UnconfirmedBatchCount::<Test>::get();
		assert_eq!(outstanding, 1, "the accounts stage sent one batch");
		let stalled_at = RcMigrationStage::<Test>::get();

		// WHEN nothing is acknowledged, THEN extraction stops where it is: the relay chain
		// destroys what it sends, so it must not run arbitrarily far ahead of what landed.
		run_blocks_without_ct(5);
		assert_eq!(RcMigrationStage::<Test>::get(), stalled_at);
		assert_eq!(UnconfirmedBatchCount::<Test>::get(), outstanding, "no new batch was sent");

		// WHEN the Coretime chain catches up, THEN the machine moves again.
		confirm_pending_batches();
		assert_eq!(UnconfirmedBatchCount::<Test>::get(), 0);
		run_blocks_without_ct(1);
		assert_ne!(RcMigrationStage::<Test>::get(), stalled_at);
	});
}

#[test]
fn a_rejected_batch_is_reported_but_never_halts_the_migration() {
	new_test_ext().execute_with(|| {
		let alice = acc(1); // manager; its registrar deposit is what travels to Coretime
		fund(&alice, 10_000);
		register_para(2000, &alice);

		assert_ok!(Rc2Migrator::force_set_stage(root(), Stage::AccountsInit));
		run_blocks_without_ct(2);
		let (query_id, batch) = UnconfirmedBatches::<Test>::iter().next().unwrap();
		let sent_in = batch.stage;

		// WHEN the Coretime chain reports that the batch failed to dispatch.
		assert_ok!(Rc2Migrator::receive_query_response(
			RuntimeOrigin::signed(coretime()),
			query_id,
			Response::DispatchResult(MaybeErrorCode::Error(vec![1, 2].try_into().unwrap())),
		));

		// THEN it is recorded and the batch stays retryable — but the machine does NOT stop.
		// Halting mid-run would leave both chains locked down with no way forward; the gap is
		// settled after the migration, not during it.
		assert_ne!(RcMigrationStage::<Test>::get(), Stage::Paused);
		assert!(UnconfirmedBatches::<Test>::contains_key(query_id));
		assert!(migrator_events().iter().any(|e| matches!(
			e,
			Event::BatchFailed { query_id: q, stage, .. } if *q == query_id && *stage == sent_in
		)));
	});
}

#[test]
fn an_unanswered_batch_is_reported_once_and_does_not_halt_the_migration() {
	new_test_ext().execute_with(|| {
		let alice = acc(1); // manager; its registrar deposit is what travels to Coretime
		fund(&alice, 10_000);
		register_para(2000, &alice);
		assert_ok!(Rc2Migrator::force_set_stage(root(), Stage::AccountsInit));
		run_blocks_without_ct(2);
		let (query_id, _) = UnconfirmedBatches::<Test>::iter().next().unwrap();

		// WHEN the response never arrives, THEN it is reported exactly once — the entry stays
		// outstanding, so a per-block report would fire forever — and the stage is untouched.
		run_blocks_without_ct(XcmResponseTimeout::get() + 3);
		assert_ne!(RcMigrationStage::<Test>::get(), Stage::Paused);
		let timeouts = migrator_events()
			.iter()
			.filter(|e| matches!(e, Event::BatchTimedOut { query_id: q, .. } if *q == query_id))
			.count();
		assert_eq!(timeouts, 1, "the timeout is reported once, not every block");
	});
}

#[test]
fn only_the_coretime_chain_may_answer_and_only_for_a_known_batch() {
	new_test_ext().execute_with(|| {
		let alice = acc(1); // manager; its registrar deposit is what travels to Coretime
		fund(&alice, 10_000);
		register_para(2000, &alice);
		assert_ok!(Rc2Migrator::force_set_stage(root(), Stage::AccountsInit));
		run_blocks_without_ct(2);
		let (query_id, _) = UnconfirmedBatches::<Test>::iter().next().unwrap();

		let ok = Response::DispatchResult(MaybeErrorCode::Success);
		assert_noop!(
			Rc2Migrator::receive_query_response(
				RuntimeOrigin::signed(acc(9)),
				query_id,
				ok.clone(),
			),
			BadOrigin
		);
		assert_noop!(
			Rc2Migrator::receive_query_response(
				RuntimeOrigin::signed(coretime()),
				query_id + 1,
				ok,
			),
			Error::<Test>::UnknownQuery
		);
		// A response that is not a dispatch result says nothing about the batch.
		assert_noop!(
			Rc2Migrator::receive_query_response(
				RuntimeOrigin::signed(coretime()),
				query_id,
				Response::Null,
			),
			Error::<Test>::UnexpectedResponse
		);
	});
}

#[test]
fn every_batch_carries_a_report_appendix_that_survives_a_failed_transact() {
	new_test_ext().execute_with(|| {
		let alice = acc(1); // manager; its registrar deposit is what travels to Coretime
		fund(&alice, 10_000);
		register_para(2000, &alice);
		assert_ok!(Rc2Migrator::force_set_stage(root(), Stage::AccountsInit));
		take_sent_xcm();
		run_blocks_without_ct(2);

		// The appendix must be *set before* the Transact it reports on: an appendix runs whether
		// or not the body succeeded, which is what lets `ExpectTransactStatus` abort the body and
		// still have the failure reported back.
		let sent = take_sent_xcm();
		let (_, msg) = sent
			.iter()
			.find(|(_, m)| m.0.iter().any(|i| matches!(i, Transact { .. })))
			.expect("the accounts stage sends a batch to the Coretime chain");
		let appendix = msg
			.0
			.iter()
			.position(|i| matches!(i, SetAppendix(_)))
			.expect("every batch reports its dispatch result");
		let transact = msg.0.iter().position(|i| matches!(i, Transact { .. })).unwrap();
		assert!(appendix < transact, "the appendix must be set before the Transact");
		assert!(msg.0.iter().any(|i| matches!(i, ExpectTransactStatus(_))));
	});
}

#[test]
fn a_failed_batch_can_be_retried_under_a_fresh_query() {
	new_test_ext().execute_with(|| {
		let alice = acc(1); // manager; its registrar deposit is what travels to Coretime
		fund(&alice, 10_000);
		register_para(2000, &alice);
		assert_ok!(Rc2Migrator::force_set_stage(root(), Stage::AccountsInit));
		run_blocks_without_ct(2);
		let (query_id, batch) = UnconfirmedBatches::<Test>::iter().next().unwrap();
		let payload = batch.call.clone();
		let sent_in = batch.stage;

		// GIVEN the Coretime chain rejected the batch.
		assert_ok!(Rc2Migrator::receive_query_response(
			RuntimeOrigin::signed(coretime()),
			query_id,
			Response::DispatchResult(MaybeErrorCode::Error(vec![1].try_into().unwrap())),
		));
		take_sent_xcm();

		// WHEN it is retried.
		assert_ok!(Rc2Migrator::retry_batch(root(), query_id));

		// THEN the same payload goes out again under a fresh query, and the old id is forgotten
		// so a late answer to it cannot settle anything.
		assert_eq!(decode_ct_calls(&take_sent_xcm()), vec![payload]);
		assert!(!UnconfirmedBatches::<Test>::contains_key(query_id));
		let (new_id, new_batch) = UnconfirmedBatches::<Test>::iter().next().unwrap();
		assert_ne!(new_id, query_id);
		assert_eq!(new_batch.stage, sent_in, "a retry keeps the stage that built the payload");
		assert_eq!(UnconfirmedBatchCount::<Test>::get(), 1);
		assert!(migrator_events().iter().any(|e| matches!(
			e,
			Event::BatchRetried { old_query_id, new_query_id, .. }
				if *old_query_id == query_id && *new_query_id == new_id
		)));

		// AND confirming the retry clears it.
		assert_ok!(Rc2Migrator::receive_query_response(
			RuntimeOrigin::signed(coretime()),
			new_id,
			Response::DispatchResult(MaybeErrorCode::Success),
		));
		assert_eq!(UnconfirmedBatchCount::<Test>::get(), 0);
	});
}

#[test]
fn only_a_known_batch_can_be_recovered_and_abandoning_is_root_only() {
	new_test_ext().execute_with(|| {
		let alice = acc(1); // manager; its registrar deposit is what travels to Coretime
		fund(&alice, 10_000);
		register_para(2000, &alice);
		assert_ok!(Rc2Migrator::force_set_stage(root(), Stage::AccountsInit));
		run_blocks_without_ct(2);
		let (query_id, _) = UnconfirmedBatches::<Test>::iter().next().unwrap();

		assert_noop!(Rc2Migrator::retry_batch(root(), query_id + 99), Error::<Test>::UnknownQuery);
		assert_noop!(
			Rc2Migrator::retry_batch(RuntimeOrigin::signed(acc(9)), query_id),
			BadOrigin
		);
		// Abandoning loses data, so it is root's alone — not the manager's.
		assert_noop!(
			Rc2Migrator::abandon_batch(RuntimeOrigin::signed(acc(9)), query_id),
			BadOrigin
		);
	});
}

#[test]
fn abandoning_a_batch_drops_it_and_records_the_loss() {
	new_test_ext().execute_with(|| {
		let alice = acc(1); // manager; its registrar deposit is what travels to Coretime
		fund(&alice, 10_000);
		register_para(2000, &alice);
		assert_ok!(Rc2Migrator::force_set_stage(root(), Stage::AccountsInit));
		run_blocks_without_ct(2);
		let (query_id, batch) = UnconfirmedBatches::<Test>::iter().next().unwrap();
		let sent_in = batch.stage;
		take_sent_xcm();

		assert_ok!(Rc2Migrator::abandon_batch(root(), query_id));

		// The batch is gone and nothing was re-sent: its contents are lost, deliberately, and the
		// event is the only record that they existed.
		assert!(UnconfirmedBatches::<Test>::iter().next().is_none());
		assert_eq!(UnconfirmedBatchCount::<Test>::get(), 0);
		assert!(take_sent_xcm().is_empty());
		assert!(migrator_events()
			.contains(&Event::BatchAbandoned { query_id, stage: sent_in }));
	});
}
