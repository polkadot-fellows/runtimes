// Copyright (C) Polkadot Fellows.
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

use super::*;
use crate::{
	mock::*, BlockLimits, CtMigratorCall, CtRuntimeCall, Error, Event, ExpectedReserves, Limits,
	MigratedBalances, RcMigratedBalance, RcMigrationStage,
};
use frame_support::{assert_noop, assert_ok, traits::ReservableCurrency};
use migrator_types::{sibling_account, PortableHoldReason};
use sp_runtime::{
	traits::{BlakeTwo256, Hash},
	AccountId32,
	DispatchError::BadOrigin,
};
use xcm::prelude::*;

type Migrator = AccountsMigrator<Test>;

// Paras and their deposits. The sovereign of a para is `child_sov(id)` here and
// `sibling_account(id)` on Asset Hub and the Coretime chain.
const FRANKS_PARA: u32 = 2000; // registered by Frank
const CHANNEL_SENDER: u32 = 2001; // the two ends of an open channel
const CHANNEL_RECIPIENT: u32 = 2002;
const SENDER_DEPOSIT: u128 = 50;
const RECIPIENT_DEPOSIT: u128 = 40;
const REQUEST_SENDER: u32 = 2003; // a channel request the recipient accepted
const REQUEST_RECIPIENT: u32 = 2004;
const REQUEST_DEPOSIT: u128 = 30;
/// What one proxy definition costs on the relay chain.
const PROXY_DEPOSIT: u128 = PROXY_DEPOSIT_BASE + PROXY_DEPOSIT_FACTOR;
/// What one announcement costs.
const ANNOUNCEMENT_DEPOSIT: u128 = ANNOUNCEMENT_DEPOSIT_BASE + ANNOUNCEMENT_DEPOSIT_FACTOR;
/// What a 2-of-2 multisig operation costs.
const MULTISIG_DEPOSIT: u128 = MULTISIG_DEPOSIT_BASE + 2 * MULTISIG_DEPOSIT_FACTOR;

fn reserved(who: &AccountId) -> u128 {
	pallet_balances::Pallet::<Test>::reserved_balance(who)
}

/// `(reason, amount)` of each hold of an account sent to the Coretime chain.
fn holds(account: &PortableAccount<AccountId, u128>) -> Vec<(PortableHoldReason, u128)> {
	account.holds.iter().map(|h| (h.reason, h.amount)).collect()
}

/// What a block sends to the Coretime chain for `who`.
fn to_ct<'a>(out: &'a BlockWithdrawals, who: &AccountId) -> &'a PortableAccount<AccountId, u128> {
	out.to_ct
		.iter()
		.find(|a| a.who == *who)
		.unwrap_or_else(|| panic!("{who:?} goes to Coretime"))
}

/// The instruction that credits `who` on Asset Hub, as `send_teleport` builds it.
fn deposit(who: AccountId, amount: u128) -> Instruction<()> {
	DepositAsset {
		assets: AssetFilter::Definite((Location::parent(), amount).into()),
		beneficiary: Location::new(0, [Junction::AccountId32 { network: None, id: who.into() }]),
	}
}

/// How many accounts each teleport sent so far credits: one `DepositAsset` per account.
fn accounts_per_message() -> Vec<usize> {
	sent()
		.iter()
		.map(|(dest, message)| {
			assert_eq!(*dest, ah_dest());
			message.0.iter().filter(|i| matches!(i, DepositAsset { .. })).count()
		})
		.collect()
}

/// `(count, amount)` of every `AccountsTeleported` event so far.
fn teleported_events() -> Vec<(u32, u128)> {
	System::events()
		.into_iter()
		.filter_map(|record| match record.event {
			RuntimeEvent::Rc2Migrator(Event::AccountsTeleported { count, amount }) =>
				Some((count, amount)),
			_ => None,
		})
		.collect()
}

/// The `i`th of many accounts.
fn account(i: u32) -> AccountId {
	let mut id = [0u8; 32];
	id[..4].copy_from_slice(&i.to_le_bytes());
	AccountId32::new(id)
}

/// One account per row of the table in `accounts.md`.
#[test]
fn migrate_one_account_per_case() {
	new_test_ext().execute_with(|| {
		// GIVEN one account per case.
		fund(&ALICE, 100); // free balance
		let pot = pallet_account(); // module account
		fund(&pot, 100);
		force_below_ed(&CHARLIE, ED - 1); // below the existential deposit
		fund(&DAVE, 100); // the manager
		let para = child_sov(FRANKS_PARA); // para sovereign
		fund(&para, 100);
		let sibl: AccountId = sibling_account(3000); // sibling-format account
		fund(&sibl, 100);
		fund(&EVE, 100); // referenced by another pallet
		add_consumer(&EVE);
		fund(&FRANK, 1_000); // para manager with a registrar deposit
		register_para(FRANKS_PARA, &FRANK);
		assert_eq!((free(&FRANK), reserved(&FRANK)), (700, PARA_DEPOSIT));
		open_channel(CHANNEL_SENDER, CHANNEL_RECIPIENT, SENDER_DEPOSIT, RECIPIENT_DEPOSIT);
		open_request(REQUEST_SENDER, REQUEST_RECIPIENT, REQUEST_DEPOSIT);
		accept_request(REQUEST_SENDER, REQUEST_RECIPIENT);
		fund(&GRACE, 500); // delegator with a portable proxy
		add_proxy(&GRACE, &JUDY, ProxyType::NonTransfer);
		fund(&JUDY, 500); // her delegate, with an announcement pending
		announce(&JUDY, &GRACE);
		fund(&LEO, 500); // depositor of a multisig operation
		start_multisig(&LEO, &ALICE);
		assert_eq!((reserved(&JUDY), reserved(&LEO)), (ANNOUNCEMENT_DEPOSIT, MULTISIG_DEPOSIT));
		fund(&BOB, 500); // reserve that no record explains
		reserve(&BOB, 40);
		fund(&HEIDI, 500); // delegator with no portable proxy
		add_proxy(&HEIDI, &ALICE, ProxyType::Staking);
		fund(&MALLORY, 500); // noted a preimage
		note_preimage(&MALLORY, vec![1, 2, 3]);
		fund(&NIAJ, 500); // a named hold no pallet accounts for
		stray_hold(&NIAJ, 7);
		assert_eq!((held(&MALLORY), held(&NIAJ)), (4, 7));
		fund(&IVAN, 500); // created a pure proxy, holds its deposit
		let pure = create_pure(&IVAN); // the pure proxy, never signed
		fund(&pure, 200);
		assert_eq!((reserved(&IVAN), reserved(&pure)), (PROXY_DEPOSIT, 0));

		// WHEN the stage starts and one block withdraws them all.
		Migrator::init();
		let out = Migrator::migrate_many(None, Some(&DAVE));
		assert_eq!(out.last_key, None);
		let mut to_ah = out.to_ah.clone();
		// sorting just to make it easier to assert
		to_ah.sort();

		// THEN each lands where its row says.
		let mut expected = vec![
			(ALICE.clone(), 100),                // free balance
			(sibling_account(FRANKS_PARA), 100), // para sovereign, translated
			(sibl.clone(), 100),                 // sibling-format, as is
			(EVE.clone(), 100),                  // referenced, drained to a shell
			(FRANK.clone(), 600),                // what is left after the deposit and the buffer
			(GRACE.clone(), 500 - PROXY_DEPOSIT - CT_FREE_BUFFER),
			(HEIDI.clone(), 500), // the deposit was refunded
			(IVAN.clone(), 500 - PROXY_DEPOSIT - CT_FREE_BUFFER),
			(JUDY.clone(), 500), // the deposit was refunded
			(LEO.clone(), 500),  // the deposit was refunded
			(BOB.clone(), 500 - 40 - CT_FREE_BUFFER),
			(MALLORY.clone(), 500), // the preimage deposit was released
			(NIAJ.clone(), 500),    // the stray hold was released
		];
		expected.sort();
		assert_eq!(to_ah, expected);
		// deposit holders: the deposit as a hold, the free part (here only the ED) with it
		assert_eq!(out.to_ct.len(), 9);
		let frank_ct = to_ct(&out, &FRANK);
		assert_eq!(frank_ct.free, CT_FREE_BUFFER);
		assert_eq!(holds(frank_ct), vec![(PortableHoldReason::RegistrarDeposit, PARA_DEPOSIT)]);
		for (para, deposit) in [
			(CHANNEL_SENDER, SENDER_DEPOSIT),
			(CHANNEL_RECIPIENT, RECIPIENT_DEPOSIT),
			(REQUEST_SENDER, REQUEST_DEPOSIT),
			(REQUEST_RECIPIENT, HRMP_RECIPIENT_DEPOSIT),
		] {
			let sov = to_ct(&out, &sibling_account(para));
			assert_eq!(sov.free, ED, "para {para}");
			assert_eq!(holds(sov), vec![(PortableHoldReason::HrmpDeposit, deposit)], "para {para}");
		}
		for who in [&GRACE, &IVAN] {
			let delegator = to_ct(&out, who);
			assert_eq!(delegator.free, CT_FREE_BUFFER, "{who:?}");
			assert_eq!(
				holds(delegator),
				vec![(PortableHoldReason::ProxyDeposit, PROXY_DEPOSIT)],
				"{who:?}"
			);
		}
		for (who, amount) in
			[(HEIDI, PROXY_DEPOSIT), (JUDY, ANNOUNCEMENT_DEPOSIT), (LEO, MULTISIG_DEPOSIT)]
		{
			System::assert_has_event(Event::DepositRefunded { who, amount }.into());
		}
		// what no record explains is parked on Coretime under its own reason
		let bob_ct = to_ct(&out, &BOB);
		assert_eq!(bob_ct.free, CT_FREE_BUFFER);
		assert_eq!(holds(bob_ct), vec![(PortableHoldReason::UnattributedReserve, 40)]);
		System::assert_has_event(Event::UnattributedReserve { who: BOB, amount: 40 }.into());
		// the pure goes to Coretime whole, where its delegate keeps control
		let pure_ct = to_ct(&out, &pure);
		assert_eq!((pure_ct.free, holds(pure_ct)), (200, vec![]));
		for who in [&ALICE, &para, &sibl, &FRANK] {
			assert!(!frame_system::Account::<Test>::contains_key(who), "{who:?} is reaped");
		}
		let shell = frame_system::Account::<Test>::get(&EVE);
		assert_eq!((shell.data.free, shell.consumers), (0, 1), "the shell stays, empty");
		System::assert_has_event(Event::AccountShellDrained { who: EVE, amount: 100 }.into());

		// AND only what a later stage handles is left: the pot, Charlie and the manager.
		assert_eq!(free(&pot), 100, "module account");
		assert_eq!(free(&CHARLIE), ED - 1, "below the existential deposit");
		assert_eq!(free(&DAVE), 100, "the manager");
		assert_eq!(total_issuance(), 100 + (ED - 1) + 100);
	});
}

#[test]
fn migrate_many_resumes_from_cursor() {
	new_test_ext().execute_with(|| {
		// GIVEN a limit of 3 accounts a block, and 4 accounts.
		Limits::<Test>::put(BlockLimits { items_per_xcm: 3, xcm_per_block: 1 });
		for i in 0..4 {
			fund(&account(i), 100);
		}
		assert_eq!(total_issuance(), 400);

		// WHEN the first block runs. THEN it withdraws 3 and leaves a cursor.
		let first = Migrator::migrate_many(None, None);
		assert_eq!(first.to_ah.len(), 3);
		assert_eq!(total_issuance(), 100);
		let last_key = first.last_key.expect("one account is left");

		// WHEN the next block continues after the cursor. THEN it withdraws the rest.
		let second = Migrator::migrate_many(Some(last_key), None);
		assert_eq!(second.to_ah.len(), 1);
		assert_eq!(second.last_key, None);
		assert_eq!(total_issuance(), 0);
	});
}

#[test]
fn withdraw_releases_a_lock() {
	new_test_ext().execute_with(|| {
		// GIVEN an account with a lock.
		fund(&BOB, 100);
		lock(&BOB, 40);

		// WHEN withdrawn. THEN the lock is gone and the whole balance leaves as free.
		let out = Migrator::migrate_many(None, None);
		assert_eq!(out.to_ah, vec![(BOB.clone(), 100)]);
		assert!(out.to_ct.is_empty());
		assert!(pallet_balances::Locks::<Test>::get(&BOB).is_empty());
		assert!(!frame_system::Account::<Test>::contains_key(&BOB));
		assert_eq!(total_issuance(), 0);
	});
}

#[test]
fn init_indexes_registrar_deposits() {
	new_test_ext().execute_with(|| {
		// GIVEN a manager of two paras.
		fund(&FRANK, 1_000);
		register_para(FRANKS_PARA, &FRANK);
		register_para(FRANKS_PARA + 1, &FRANK);

		// WHEN the index is built, twice. THEN both deposits are indexed, once.
		assert_eq!(Migrator::init(), 2);
		assert_eq!(Migrator::init(), 2);
		assert_eq!(ExpectedReserves::<Test>::get(&FRANK).registrar, 2 * PARA_DEPOSIT);
		assert_eq!(ExpectedReserves::<Test>::iter().count(), 1);
	});
}

#[test]
fn init_indexes_hrmp_deposits() {
	new_test_ext().execute_with(|| {
		// GIVEN a channel, a request the recipient accepted, and a request not yet accepted.
		let pending_sender = 2005;
		let pending_recipient = 2006;
		open_channel(CHANNEL_SENDER, CHANNEL_RECIPIENT, SENDER_DEPOSIT, RECIPIENT_DEPOSIT);
		open_request(REQUEST_SENDER, REQUEST_RECIPIENT, REQUEST_DEPOSIT);
		accept_request(REQUEST_SENDER, REQUEST_RECIPIENT);
		open_request(pending_sender, pending_recipient, REQUEST_DEPOSIT);

		// WHEN the index is built. THEN every deposit is under its sovereign as HRMP.
		assert_eq!(Migrator::init(), 3);
		let hrmp = |para: u32| ExpectedReserves::<Test>::get(child_sov(para)).hrmp;
		assert_eq!(
			(hrmp(CHANNEL_SENDER), hrmp(CHANNEL_RECIPIENT)),
			(SENDER_DEPOSIT, RECIPIENT_DEPOSIT)
		);
		assert_eq!(
			(hrmp(REQUEST_SENDER), hrmp(REQUEST_RECIPIENT)),
			(REQUEST_DEPOSIT, HRMP_RECIPIENT_DEPOSIT)
		);
		assert_eq!((hrmp(pending_sender), hrmp(pending_recipient)), (REQUEST_DEPOSIT, 0));
	});
}

#[test]
fn init_indexes_proxy_deposits() {
	new_test_ext().execute_with(|| {
		// GIVEN a portable delegation, a non-portable one, a mixed one, and a pure proxy.
		for who in [&GRACE, &HEIDI, &IVAN, &BOB] {
			fund(who, 500);
		}
		add_proxy(&GRACE, &ALICE, ProxyType::NonTransfer);
		add_proxy(&HEIDI, &ALICE, ProxyType::Staking);
		add_proxy(&BOB, &ALICE, ProxyType::Staking);
		add_proxy(&BOB, &ALICE, ProxyType::Any);
		let pure = create_pure(&IVAN);

		// WHEN the index is built. THEN a portable or mixed set is a proxy deposit, a
		// non-portable one a refund, and the pure's is under its creator.
		assert_eq!(Migrator::init(), 4);
		let expected = |who: &AccountId| {
			let e = ExpectedReserves::<Test>::get(who);
			(e.proxy, e.refund)
		};
		assert_eq!(expected(&GRACE), (PROXY_DEPOSIT, 0));
		assert_eq!(expected(&HEIDI), (0, PROXY_DEPOSIT));
		assert_eq!(expected(&BOB), (PROXY_DEPOSIT + PROXY_DEPOSIT_FACTOR, 0));
		assert_eq!(expected(&IVAN), (PROXY_DEPOSIT, 0));
		assert_eq!(expected(&pure), (0, 0));
	});
}

#[test]
fn init_indexes_refunded_deposits() {
	new_test_ext().execute_with(|| {
		// GIVEN an announcement and a multisig operation.
		for who in [&GRACE, &JUDY, &LEO] {
			fund(who, 500);
		}
		add_proxy(&GRACE, &JUDY, ProxyType::NonTransfer);
		announce(&JUDY, &GRACE);
		start_multisig(&LEO, &ALICE);

		// WHEN the index is built. THEN both deposits are refunds, under who reserved them.
		assert_eq!(Migrator::init(), 3);
		assert_eq!(ExpectedReserves::<Test>::get(&JUDY).refund, ANNOUNCEMENT_DEPOSIT);
		assert_eq!(ExpectedReserves::<Test>::get(&LEO).refund, MULTISIG_DEPOSIT);
	});
}

#[test]
fn init_releases_preimage_deposits() {
	new_test_ext().execute_with(|| {
		// GIVEN an unrequested preimage, a requested one with its owner's deposit, and a request
		// nobody has noted a blob for.
		fund(&MALLORY, 500);
		let unrequested = note_preimage(&MALLORY, vec![1]);
		let requested = note_preimage(&MALLORY, vec![2, 2]);
		assert_ok!(Preimage::request_preimage(RuntimeOrigin::root(), requested));
		let unnoted = BlakeTwo256::hash(&[3]);
		assert_ok!(Preimage::request_preimage(RuntimeOrigin::root(), unnoted));
		assert_eq!(held(&MALLORY), 2 + 3);

		// WHEN the stage starts. THEN both deposits are released; the unrequested blob is gone,
		// the requested one stays without a ticket, the unnoted request is untouched.
		Migrator::init();
		assert_eq!(held(&MALLORY), 0);
		assert!(pallet_preimage::RequestStatusFor::<Test>::get(unrequested).is_none());
		assert!(matches!(
			pallet_preimage::RequestStatusFor::<Test>::get(requested),
			Some(pallet_preimage::RequestStatus::Requested {
				maybe_ticket: None,
				maybe_len: Some(2),
				..
			})
		));
		assert!(matches!(
			pallet_preimage::RequestStatusFor::<Test>::get(unnoted),
			Some(pallet_preimage::RequestStatus::Requested {
				maybe_ticket: None,
				maybe_len: None,
				..
			})
		));
	});
}

#[test]
fn withdraw_refunds_the_deposits_of_a_closing_channel() {
	new_test_ext().execute_with(|| {
		// GIVEN a channel with a pending close request.
		open_channel(CHANNEL_SENDER, CHANNEL_RECIPIENT, SENDER_DEPOSIT, RECIPIENT_DEPOSIT);
		request_close(CHANNEL_SENDER, CHANNEL_RECIPIENT);

		// WHEN withdrawn. THEN the deposits go to Asset Hub as free balance, no hold.
		Migrator::init();
		let out = Migrator::migrate_many(None, None);
		assert!(out.to_ct.is_empty());
		let mut to_ah = out.to_ah;
		to_ah.sort();
		let mut expected = vec![
			(sibling_account(CHANNEL_SENDER), SENDER_DEPOSIT + ED),
			(sibling_account(CHANNEL_RECIPIENT), RECIPIENT_DEPOSIT + ED),
		];
		expected.sort();
		assert_eq!(to_ah, expected);
	});
}

#[test]
fn withdraw_attributes_up_to_the_reserve() {
	new_test_ext().execute_with(|| {
		// GIVEN a manager whose record says 300 but who has only 200 reserved.
		fund(&FRANK, 1_000);
		register_para(FRANKS_PARA, &FRANK);
		<Balances as ReservableCurrency<AccountId>>::unreserve(&FRANK, 100);
		assert_eq!((free(&FRANK), reserved(&FRANK)), (800, 200));

		// WHEN withdrawn. THEN the hold is what was reserved, and the rest is free.
		Migrator::init();
		let out = Migrator::migrate_many(None, None);
		assert_eq!(holds(&out.to_ct[0]), vec![(PortableHoldReason::RegistrarDeposit, 200)]);
		assert_eq!(out.to_ct[0].free, CT_FREE_BUFFER);
		assert_eq!(out.to_ah, vec![(FRANK, 700)]);
		assert_eq!(total_issuance(), 0);
	});
}

#[test]
fn withdraw_rounds_dust_to_coretime() {
	new_test_ext().execute_with(|| {
		// GIVEN a deposit holder whose remainder after the buffer is below the ED.
		fund(&FRANK, PARA_DEPOSIT + CT_FREE_BUFFER + ED - 1);
		register_para(FRANKS_PARA, &FRANK);

		// WHEN withdrawn. THEN the remainder follows the deposit instead of being teleported.
		Migrator::init();
		let out = Migrator::migrate_many(None, None);
		assert_eq!(out.to_ct[0].free, CT_FREE_BUFFER + ED - 1);
		assert!(out.to_ah.is_empty());
		assert_eq!(total_issuance(), 0);
	});
}

#[test]
fn accounts_stage_sends_deposit_holders_to_coretime() {
	new_test_ext().execute_with(|| {
		// GIVEN a manager with a deposit, and the stage about to start.
		fund(&FRANK, 1_000);
		register_para(FRANKS_PARA, &FRANK);
		RcMigrationStage::<Test>::put(Stage::AccountsInit);

		// WHEN the stage runs to the end: a block to build the index, one to withdraw. THEN the
		// Coretime chain gets the account with its hold and buffer, Asset Hub the rest, and the
		// ledger says the same.
		run_blocks(2);
		assert_stage(Stage::AccountsDone);
		let sent = sent();
		assert_eq!(sent.len(), 2);
		assert_eq!(sent[0].0, ct_dest());
		let CtRuntimeCall::CtMigrator(CtMigratorCall::ReceiveAccounts { accounts }) = sent_call(0)
		else {
			panic!("the first message carries accounts for Coretime");
		};
		assert_eq!(accounts.len(), 1);
		assert_eq!(accounts[0].who, FRANK);
		assert_eq!(accounts[0].free, CT_FREE_BUFFER);
		assert_eq!(holds(&accounts[0]), vec![(PortableHoldReason::RegistrarDeposit, PARA_DEPOSIT)]);
		let (dest, Xcm(message)) = &sent[1];
		assert_eq!(*dest, ah_dest());
		assert!(message.contains(&deposit(FRANK, 600)));
		System::assert_has_event(Event::AccountsBatchSent { count: 1 }.into());
		assert_eq!(
			RcMigratedBalance::<Test>::get(),
			MigratedBalances {
				kept: 0,
				ct_reserved: PARA_DEPOSIT,
				ct_free: CT_FREE_BUFFER,
				ah_free: 600,
				ti_corrected: 0
			}
		);
		assert_eq!(ExpectedReserves::<Test>::iter().count(), 0);
	});
}

#[test]
fn accounts_stage_teleports_to_asset_hub() {
	new_test_ext().execute_with(|| {
		// GIVEN two plain accounts and the stage about to start.
		fund(&ALICE, 100);
		fund(&BOB, 250);
		RcMigrationStage::<Test>::put(Stage::AccountsInit);

		// WHEN the stage initialises. THEN the ledger holds the issuance and nothing is sent.
		run_blocks(1);
		assert_stage(Stage::AccountsOngoing { last_key: None });
		assert_eq!(
			RcMigratedBalance::<Test>::get(),
			MigratedBalances { kept: 350, ..Default::default() }
		);
		assert!(sent().is_empty());

		// WHEN the block of withdrawals runs. THEN one teleport carries both, everything is
		// booked as moved to Asset Hub, and the stage is done.
		run_blocks(1);
		assert_stage(Stage::AccountsDone);
		let sent = sent();
		assert_eq!(sent.len(), 1);
		let (dest, Xcm(message)) = &sent[0];
		assert_eq!(*dest, ah_dest());
		// the total is teleported in, then one deposit per account (in storage order)
		assert_eq!(
			message[..2],
			[
				UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
				ReceiveTeleportedAsset((Location::parent(), 350u128).into()),
			]
		);
		assert_eq!(message.len(), 4);
		assert!(message.contains(&deposit(ALICE, 100)));
		assert!(message.contains(&deposit(BOB, 250)));
		assert_eq!(teleported_events(), vec![(2, 350)]);
		assert_eq!(
			RcMigratedBalance::<Test>::get(),
			MigratedBalances { kept: 0, ah_free: 350, ..Default::default() }
		);
		assert_eq!(total_issuance(), 0);
	});
}

#[test]
fn accounts_stage_sends_in_chunks_and_resumes() {
	new_test_ext().execute_with(|| {
		// GIVEN 7 accounts of 100, limits of 2 accounts a message and 2 messages a block, and the
		// stage initialised.
		Limits::<Test>::put(BlockLimits { items_per_xcm: 2, xcm_per_block: 2 });
		for i in 0..7 {
			fund(&account(i), 100);
		}
		assert_eq!(total_issuance(), 700);
		RcMigrationStage::<Test>::put(Stage::AccountsInit);
		run_blocks(1);

		// WHEN the first block runs. THEN the 2 messages carry 4 accounts and the stage carries a
		// cursor.
		run_blocks(1);
		assert!(matches!(
			RcMigrationStage::<Test>::get(),
			Stage::AccountsOngoing { last_key: Some(_) }
		));
		assert_eq!(accounts_per_message(), vec![2, 2]);
		assert_eq!(total_issuance(), 300);

		// WHEN the next block runs. THEN the last 3 leave in a full and a half message, and
		// the stage is done.
		run_blocks(1);
		assert_stage(Stage::AccountsDone);
		assert_eq!(accounts_per_message(), vec![2, 2, 2, 1]);
		assert_eq!(total_issuance(), 0);
		assert_eq!(
			RcMigratedBalance::<Test>::get(),
			MigratedBalances { kept: 0, ah_free: 700, ..Default::default() }
		);
	});
}

/// A rewind to `AccountsInit` runs the init and the whole withdraw loop again over state that
/// is already half migrated. Nothing may leave twice, and the books must still add up.
#[test]
fn accounts_stage_rewound_to_init_moves_nothing_twice() {
	new_test_ext().execute_with(|| {
		// GIVEN 8 accounts, 4 of which leave per block: 7 plain ones, and a para manager with a
		// registrar deposit, so both destinations and the deposit index are involved.
		Limits::<Test>::put(BlockLimits { items_per_xcm: 2, xcm_per_block: 2 });
		for i in 0..7 {
			fund(&account(i), 100);
		}
		fund(&FRANK, 1_000);
		register_para(FRANKS_PARA, &FRANK);
		let total = 7 * 100 + 1_000;
		let ah_free = total - PARA_DEPOSIT - CT_FREE_BUFFER;
		assert_eq!(total_issuance(), total);
		assert_eq!(frame_system::Account::<Test>::iter().count(), 8);

		// WHEN the stage initialises and runs one block. THEN 4 accounts are gone.
		RcMigrationStage::<Test>::put(Stage::AccountsInit);
		run_blocks(2);
		assert_eq!(frame_system::Account::<Test>::iter().count(), 4);
		assert_eq!(RcMigratedBalance::<Test>::get().kept, total_issuance());

		// WHEN the stage is set back to its init, as `force_set_stage` does, and runs to the
		// end. THEN the other 4 leave and the chain is empty.
		RcMigrationStage::<Test>::put(Stage::AccountsInit);
		run_blocks(3);
		assert_stage(Stage::AccountsDone);
		assert_eq!(frame_system::Account::<Test>::iter().count(), 0);
		assert_eq!(total_issuance(), 0);

		// AND each of the 8 accounts was teleported once, for the free balance of one pass.
		let (teleported, amount) = teleported_events()
			.iter()
			.fold((0, 0), |(n, a), (count, amount)| (n + count, a + amount));
		assert_eq!((teleported, amount), (8, ah_free));

		// AND the books hold one pass over every account, and the index is cleared.
		assert_eq!(
			RcMigratedBalance::<Test>::get(),
			MigratedBalances {
				kept: 0,
				ct_reserved: PARA_DEPOSIT,
				ct_free: CT_FREE_BUFFER,
				ah_free,
				..Default::default()
			}
		);
		assert_eq!(ExpectedReserves::<Test>::iter().count(), 0);
	});
}

#[test]
fn accounts_stage_retries_a_block_whose_send_failed() {
	new_test_ext().execute_with(|| {
		// GIVEN the stage initialised with one plain account.
		fund(&ALICE, 100);
		RcMigrationStage::<Test>::put(Stage::AccountsInit);
		run_blocks(1);
		let migrated = RcMigratedBalance::<Test>::get();

		// WHEN a block runs and its send fails. THEN nothing moved.
		SendFails::set(true);
		run_blocks(1);
		assert_stage(Stage::AccountsOngoing { last_key: None });
		assert_eq!(free(&ALICE), 100);
		assert_eq!(total_issuance(), 100);
		assert_eq!(RcMigratedBalance::<Test>::get(), migrated);
		assert!(sent().is_empty());

		// WHEN the next block runs with a working router. THEN the same range leaves.
		SendFails::set(false);
		run_blocks(1);
		assert_stage(Stage::AccountsDone);
		assert_eq!(sent().len(), 1);
		assert_eq!(total_issuance(), 0);
	});
}

#[test]
fn set_limits_checks_origin_and_range() {
	new_test_ext().execute_with(|| {
		let limits = BlockLimits { items_per_xcm: 2, xcm_per_block: 2 };
		assert_noop!(Rc2Migrator::set_limits(RuntimeOrigin::signed(ALICE), limits), BadOrigin);
		for bad in [
			BlockLimits { items_per_xcm: 0, ..limits },
			BlockLimits { xcm_per_block: 0, ..limits },
			BlockLimits { items_per_xcm: BlockLimits::MAX_ITEMS_PER_XCM + 1, ..limits },
			// 98 * 17 = 1666 items, above the block limit
			BlockLimits { items_per_xcm: BlockLimits::MAX_ITEMS_PER_XCM, xcm_per_block: 17 },
		] {
			assert_noop!(
				Rc2Migrator::set_limits(RuntimeOrigin::signed(ADMIN), bad),
				Error::<Test>::BadLimits
			);
		}
		assert_ok!(Rc2Migrator::set_limits(RuntimeOrigin::signed(ADMIN), limits));
		assert_eq!(Limits::<Test>::get(), limits);
	});
}
