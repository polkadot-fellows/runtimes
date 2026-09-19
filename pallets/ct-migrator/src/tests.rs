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

//! Unit tests for `pallet-ct-migrator`.
//!
//! The pallet's contract, in the abstract: it receives portable payloads from a trusted (Root)
//! origin, turns them into local state through the chain's regular APIs, and never invents
//! balance — every re-attribution is capped by what actually arrived and every gap is parked
//! loudly. The tests pin that contract with exact values.

use crate::{mock::*, *};
use frame_support::{assert_noop, assert_ok, hypothetically, traits::fungible::Mutate};
use sp_runtime::{traits::BadOrigin, AccountId32};

fn root() -> RuntimeOrigin {
	RuntimeOrigin::root()
}

#[test]
fn receive_accounts_mints_free_and_holds_exactly() {
	new_test_ext().execute_with(|| {
		let alice = acc(1); // regular account: liquid + migrated reserve
		let charlie = acc(3); // already exists locally, receives on top

		// GIVEN charlie already has a local balance.
		<Balances as Mutate<AccountId32>>::mint_into(&charlie, 100).unwrap();
		let ti_before = total_issuance();

		// WHEN a batch arrives with one fresh and one pre-existing account.
		assert_ok!(CtMigrator::receive_accounts(
			root(),
			vec![
				portable_account(&alice, 50, vec![(PortableHoldReason::UnnamedReserve, 500)]),
				portable_account(&charlie, 30, vec![(PortableHoldReason::ProxyDeposit, 70)]),
			],
		));

		// THEN each account holds exactly what was sent, split free vs held per reason.
		assert_eq!(free(&alice), 50);
		assert_eq!(held(HoldReason::RcMigratedReserve, &alice), 500);
		assert_eq!(free(&charlie), 100 + 30);
		assert_eq!(held(HoldReason::ProxyDeposit, &charlie), 70);

		// AND issuance grew by exactly the minted total, which is also tracked for the final
		// reconciliation.
		assert_eq!(total_issuance(), ti_before + 550 + 100);
		assert_eq!(CtMintedTotal::<Test>::get(), 650);

		// AND the first batch moves the stage machine out of Pending.
		assert_eq!(CtMigrationStage::<Test>::get(), MigrationStage::DataMigrationOngoing);
		let events = migrator_events();
		assert!(events.contains(&Event::StageTransition {
			old: MigrationStage::Pending,
			new: MigrationStage::DataMigrationOngoing,
		}));
		assert!(events.contains(&Event::AccountsReceived { count_good: 2, count_bad: 0 }));
	});
}

#[test]
fn sub_ed_free_survives_hold_placement_and_reattribution() {
	new_test_ext().execute_with(|| {
		let bob = acc(2); // deposit holder whose liquid dust followed the deposit (free < ED)

		// GIVEN nothing; bob does not exist. WHEN his free part cannot provide the ED.
		assert_ok!(CtMigrator::receive_accounts(
			root(),
			vec![portable_account(&bob, 2, vec![(PortableHoldReason::UnnamedReserve, 40)])],
		));

		// THEN the account exists (provider reference), the hold landed and the dust was NOT
		// silently burned mid-hold (balances dusts a sub-ED free remainder whenever the reserve
		// passes through zero; the integration path must never expose that window).
		assert_eq!(free(&bob), 2);
		assert_eq!(held(HoldReason::RcMigratedReserve, &bob), 40);
		assert_eq!(frame_system::Pallet::<Test>::providers(&bob), 1);
		assert_eq!(CtMintedTotal::<Test>::get(), 42);
		assert_eq!(total_issuance(), 42);

		// AND WHEN the deposit is later re-attributed (registrar record arrives), the dust
		// survives the hold flip too.
		assert_ok!(CtMigrator::receive_registrar(
			root(),
			vec![PortableParaInfo {
				para_id: 2000,
				manager: bob.clone(),
				deposit: 40,
				locked: None,
				registered: true,
				head_len: 32,
			}],
			None,
		));
		// The deposit is released to free so the registrar pallet can take its own at this
		// chain's rates — and crucially the sub-ED dust survives the release. Releasing the naive
		// way would take the hold through zero while free was still below ED, and
		// pallet-balances would burn the remainder; `release_hold` credits free first.
		assert_eq!(free(&bob), 42);
		assert_eq!(held(HoldReason::RcMigratedReserve, &bob), 0);
		assert_eq!(total_issuance(), 42, "no dust may be burned by the hand-over");
	});
}

#[test]
fn sub_ed_free_survives_proxy_deposit_resize() {
	new_test_ext().execute_with(|| {
		let pure = acc(10); // keyless delegator with sub-ED liquid dust and a proxy deposit
		assert_ok!(CtMigrator::receive_accounts(
			root(),
			vec![portable_account(&pure, 3, vec![(PortableHoldReason::ProxyDeposit, 400)])],
		));

		let proxies = PortableProxy {
			delegator: pure.clone(),
			delegates: vec![migrator_types::PortableProxyDelegate {
				delegate: acc(11),
				proxy_type: PortableProxyType::Any,
				delay: 0,
			}]
			.try_into()
			.unwrap(),
		};
		assert_ok!(CtMigrator::receive_proxies(root(), vec![proxies]));

		// The resize releases the whole migrated hold and re-reserves 120 at local rates; the
		// 3-planck dust must ride along, not burn while the hold is momentarily empty.
		assert_eq!(pallet_balances::Pallet::<Test>::reserved_balance(&pure), 120);
		assert_eq!(free(&pure), 3 + 400 - 120);
		assert_eq!(total_issuance(), 403);
	});
}

#[test]
fn receive_registrar_releases_the_deposit_and_hands_the_para_over() {
	new_test_ext().execute_with(|| {
		let alice = acc(1); // parachain manager
		give_hold(&alice, HoldReason::RcMigratedReserve, 500);
		let para = PortableParaInfo {
			para_id: 2000,
			manager: alice.clone(),
			deposit: 300,
			locked: None,
			registered: true,
			head_len: 32,
		};

		let ti_before = total_issuance();
		assert_ok!(CtMigrator::receive_registrar(root(), vec![para.clone()], Some(3000)));

		// THEN the recorded deposit is *released* rather than re-labelled: the registrar pallet
		// holds its deposits as `Consideration` tickets, which can only be minted by taking
		// funds, and it prices them at this chain's rates. Anything beyond the recorded amount
		// stays parked under the generic migrated reason.
		assert_eq!(held(HoldReason::RcMigratedReserve, &alice), 200);
		assert_eq!(ReattributedDeposits::<Test>::get(), 300);
		assert!(ParkedDepositShortfalls::<Test>::iter().next().is_none());

		// AND the para itself was handed to the registrar pallet, carrying the state and lock the
		// relay chain recorded.
		let received = ReceivedParas::get();
		assert_eq!(received.len(), 1);
		assert_eq!(received[0].para_id, 2000);
		assert_eq!(received[0].manager, alice);
		assert_eq!(received[0].locked, None);
		assert_eq!(
			received[0].state,
			registrar_primitives::MigratedParaState::Registered { head_len: 32 }
		);
		assert_eq!(ReceivedNextFreeParaId::get(), Some(3000));
		// AND releasing never mints.
		assert_eq!(total_issuance(), ti_before);
		assert!(
			migrator_events().contains(&Event::RegistrarReceived { count_good: 1, count_bad: 0 })
		);
	});
}

#[test]
fn registrar_shortfall_is_parked_never_minted() {
	new_test_ext().execute_with(|| {
		let alice = acc(1); // manager whose recorded deposit exceeds what arrived (RC anomaly)
		give_hold(&alice, HoldReason::RcMigratedReserve, 100);

		let para = PortableParaInfo {
			para_id: 2000,
			manager: alice.clone(),
			deposit: 250,
			locked: None,
			registered: false,
			head_len: 0,
		};
		let ti_before = total_issuance();
		assert_ok!(CtMigrator::receive_registrar(root(), vec![para.clone()], None));

		assert_eq!(held(HoldReason::RcMigratedReserve, &alice), 0);
		assert_eq!(ParkedDepositShortfalls::<Test>::get(2000), Some(150));
		assert_eq!(total_issuance(), ti_before);
		// The para still lands: a shortfall is an accounting gap, not a failed record. It is a
		// merely reserved id here, so it arrives as `Reserved` rather than `Registered`.
		let received = ReceivedParas::get();
		assert_eq!(received.len(), 1);
		assert_eq!(received[0].para_id, 2000);
		assert_eq!(received[0].state, registrar_primitives::MigratedParaState::Reserved);
		assert!(migrator_events()
			.contains(&Event::DepositShortfallParked { para_id: 2000, shortfall: 150 }));
	});
}

#[test]
fn multi_para_manager_attribution_is_capped_by_what_arrived() {
	new_test_ext().execute_with(|| {
		let alice = acc(1); // manager of two paras, arrived hold covers only 500 of 600
		give_hold(&alice, HoldReason::RcMigratedReserve, 500);
		let para = |id| PortableParaInfo {
			para_id: id,
			manager: alice.clone(),
			deposit: 300,
			locked: None,
			registered: true,
			head_len: 32,
		};

		assert_ok!(CtMigrator::receive_registrar(root(), vec![para(2100), para(2200)], None));

		// The first para's deposit is released in full, the second gets only the remainder, and
		// the gap parks under it. What arrived caps what can be handed over, however many paras
		// share a manager.
		assert_eq!(held(HoldReason::RcMigratedReserve, &alice), 0);
		assert_eq!(ReattributedDeposits::<Test>::get(), 500);
		assert_eq!(ParkedDepositShortfalls::<Test>::get(2100), None);
		assert_eq!(ParkedDepositShortfalls::<Test>::get(2200), Some(100));
		// Both paras still reach the registrar pallet; a shortfall is an accounting gap.
		assert_eq!(
			ReceivedParas::get().iter().map(|p| p.para_id).collect::<Vec<_>>(),
			vec![2100, 2200]
		);
	});
}

#[test]
fn receive_hrmp_reattributes_both_sides_on_sibling_sovereigns() {
	new_test_ext().execute_with(|| {
		let sov_sender: AccountId32 = sibling_account(2000);
		let sov_recipient: AccountId32 = sibling_account(2001);
		give_hold(&sov_sender, HoldReason::RcMigratedReserve, 100);
		give_hold(&sov_recipient, HoldReason::RcMigratedReserve, 50);

		let channel = PortableHrmpChannel {
			sender: 2000,
			recipient: 2001,
			max_capacity: 8,
			max_total_size: 4096,
			max_message_size: 1024,
			sender_deposit: 100,
			recipient_deposit: 50,
		};
		assert_ok!(CtMigrator::receive_hrmp(root(), vec![channel.clone()]));

		// Released rather than re-labelled, for the same reason as the registrar's: the HRMP
		// pallet mints its own `Consideration` tickets at this chain's rates.
		assert_eq!(held(HoldReason::RcMigratedReserve, &sov_sender), 0);
		assert_eq!(ReattributedHrmpDeposits::<Test>::get(), 150);
		assert!(ParkedHrmpShortfalls::<Test>::iter().next().is_none());

		// A channel that exists on the relay chain arrives confirmed, so the receiving pallet
		// takes both ends' deposits.
		assert_eq!(
			ReceivedChannels::get(),
			vec![hrmp_primitives::MigratedChannel {
				channel: hrmp_primitives::ChannelId { sender: 2000, recipient: 2001 },
				confirmed: true,
			}]
		);
		assert!(migrator_events().contains(&Event::HrmpReceived { count_good: 1, count_bad: 0 }));

		// Hypothetically, had the recipient deposit not (fully) arrived, the gap parks under the
		// (sender, recipient, side) key.
		hypothetically!({
			let channel2 = PortableHrmpChannel {
				sender: 2000,
				recipient: 2002,
				recipient_deposit: 80,
				sender_deposit: 0,
				max_capacity: 8,
				max_total_size: 4096,
				max_message_size: 1024,
			};
			assert_ok!(CtMigrator::receive_hrmp(root(), vec![channel2]));
			assert_eq!(ParkedHrmpShortfalls::<Test>::get((2000, 2002, false)), Some(80));
		});
	});
}

#[test]
fn receive_hrmp_requests_relabels_and_always_stores() {
	new_test_ext().execute_with(|| {
		let sov: AccountId32 = sibling_account(2000);
		give_hold(&sov, HoldReason::RcMigratedReserve, 60);

		let request = |recipient, deposit| PortableHrmpRequest {
			sender: 2000,
			recipient,
			confirmed: true,
			sender_deposit: deposit,
			max_message_size: 1024,
			max_capacity: 8,
			max_total_size: 4096,
		};

		// WHEN two requests arrive but the sovereign's hold only covers the first.
		assert_ok!(CtMigrator::receive_hrmp_requests(
			root(),
			vec![request(2001, 60), request(2002, 40)],
		));

		// THEN the covered deposit is released, the uncovered one parks — and BOTH records are
		// still handed over, because a shortfall is an accounting gap and not a failed record.
		assert_eq!(ParkedHrmpShortfalls::<Test>::get((2000, 2002, true)), Some(40));
		let handed: Vec<(u32, u32)> = ReceivedChannels::get()
			.into_iter()
			.map(|c| (c.channel.sender, c.channel.recipient))
			.collect();
		assert_eq!(handed, vec![(2000, 2001), (2000, 2002)]);
		assert!(migrator_events().contains(&Event::HrmpRequestsReceived { count: 2 }));
	});
}

#[test]
fn receive_proxies_recreates_defs_and_resizes_deposit_to_local_rates() {
	new_test_ext().execute_with(|| {
		let pure = acc(10); // keyless delegator; its deposit arrived as a ProxyDeposit hold
		let delegate = acc(11); // its controller
		give_hold(&pure, HoldReason::ProxyDeposit, 400); // also mints ED free

		let proxies = PortableProxy {
			delegator: pure.clone(),
			delegates: vec![migrator_types::PortableProxyDelegate {
				delegate: delegate.clone(),
				proxy_type: PortableProxyType::Any,
				delay: 4,
			}]
			.try_into()
			.unwrap(),
		};
		assert_ok!(CtMigrator::receive_proxies(root(), vec![proxies]));

		// THEN the definition exists in the real proxy pallet with the delay converted to this
		// chain's block time (4 sender blocks -> 2 local blocks).
		let (defs, deposit) = pallet_proxy::Proxies::<Test>::get(&pure);
		assert_eq!(defs.len(), 1);
		assert_eq!(defs[0].delegate, delegate);
		assert_eq!(defs[0].proxy_type, ProxyType::Any);
		assert_eq!(defs[0].delay, 2);

		// AND the migrated deposit was resized to local rates: base 100 + factor 20 reserved,
		// the remainder released to the delegator as free balance.
		assert_eq!(deposit, 120);
		assert_eq!(pallet_balances::Pallet::<Test>::reserved_balance(&pure), 120);
		assert_eq!(held(HoldReason::ProxyDeposit, &pure), 0);
		assert_eq!(free(&pure), ED + 400 - 120);
		assert!(migrator_events().contains(&Event::ProxiesReceived { count_good: 1, count_bad: 0 }));
	});
}

#[test]
fn proxy_delay_conversion_rounds_down() {
	new_test_ext().execute_with(|| {
		let delegator = acc(10);
		let make = |delegate: AccountId32, delay| migrator_types::PortableProxyDelegate {
			delegate,
			proxy_type: PortableProxyType::Any,
			delay,
		};
		let proxies = PortableProxy {
			delegator: delegator.clone(),
			delegates: vec![make(acc(11), 3), make(acc(12), 1)].try_into().unwrap(),
		};
		assert_ok!(CtMigrator::receive_proxies(root(), vec![proxies]));

		let (defs, _) = pallet_proxy::Proxies::<Test>::get(&delegator);
		// 3 / 2 = 1, and 1 / 2 = 0: a 1-block delay disappears entirely. Pinned so a change of
		// rounding policy shows up as a test change, not silently.
		assert_eq!(defs[0].delay, 1);
		assert_eq!(defs[1].delay, 0);
	});
}

#[test]
fn receive_proxies_merges_with_existing_local_defs_and_dedups() {
	new_test_ext().execute_with(|| {
		let dan = acc(10); // delegator with a pre-existing local proxy
		let local = acc(11); // local delegate, added before the migration reaches this chain
		let migrated = acc(12); // delegate arriving from the sender chain
		<Balances as Mutate<AccountId32>>::mint_into(&dan, 1_000).unwrap();
		assert_ok!(Proxy::add_proxy(
			RuntimeOrigin::signed(dan.clone()),
			local.clone(),
			ProxyType::Any,
			0
		));
		assert_eq!(pallet_balances::Pallet::<Test>::reserved_balance(&dan), 120);

		let delegates = |list: Vec<AccountId32>| PortableProxy {
			delegator: dan.clone(),
			delegates: list
				.into_iter()
				.map(|d| migrator_types::PortableProxyDelegate {
					delegate: d,
					proxy_type: PortableProxyType::Any,
					delay: 0,
				})
				.collect::<Vec<_>>()
				.try_into()
				.unwrap(),
		};

		// WHEN a migrated set arrives containing a new delegate AND a duplicate of the local one.
		assert_ok!(CtMigrator::receive_proxies(
			root(),
			vec![delegates(vec![migrated.clone(), local.clone()])],
		));

		// THEN the duplicate is not re-added and the deposit tops up to the 2-def requirement.
		let (defs, deposit) = pallet_proxy::Proxies::<Test>::get(&dan);
		assert_eq!(defs.len(), 2);
		assert_eq!(deposit, 100 + 2 * 20);
		assert_eq!(pallet_balances::Pallet::<Test>::reserved_balance(&dan), 140);
	});
}

#[test]
fn receive_proxies_writes_entry_even_when_deposit_cannot_be_reserved() {
	new_test_ext().execute_with(|| {
		let broke = acc(10); // delegator that arrived with no balance at all
		let delegate = acc(11);

		let proxies = PortableProxy {
			delegator: broke.clone(),
			delegates: vec![migrator_types::PortableProxyDelegate {
				delegate: delegate.clone(),
				proxy_type: PortableProxyType::NonTransfer,
				delay: 0,
			}]
			.try_into()
			.unwrap(),
		};
		assert_ok!(CtMigrator::receive_proxies(root(), vec![proxies]));

		// Access outranks the deposit: the entry exists, under-backed, and nothing failed.
		let (defs, deposit) = pallet_proxy::Proxies::<Test>::get(&broke);
		assert_eq!(defs.len(), 1);
		assert_eq!(deposit, 0);
		assert!(migrator_events().contains(&Event::ProxiesReceived { count_good: 1, count_bad: 0 }));
	});
}

#[test]
fn receive_proxies_overflowing_merged_set_is_parked_and_rolled_back() {
	new_test_ext().execute_with(|| {
		let max = acc(10); // delegator already at MaxProxies (= 4 in this mock)
		<Balances as Mutate<AccountId32>>::mint_into(&max, 10_000).unwrap();
		for i in 41..45u8 {
			assert_ok!(Proxy::add_proxy(
				RuntimeOrigin::signed(max.clone()),
				acc(i),
				ProxyType::Any,
				0
			));
		}
		give_hold(&max, HoldReason::ProxyDeposit, 400);
		let (defs_before, deposit_before) = pallet_proxy::Proxies::<Test>::get(&max);

		let overflowing = PortableProxy {
			delegator: max.clone(),
			delegates: vec![migrator_types::PortableProxyDelegate {
				delegate: acc(45),
				proxy_type: PortableProxyType::Any,
				delay: 0,
			}]
			.try_into()
			.unwrap(),
		};
		assert_ok!(CtMigrator::receive_proxies(root(), vec![overflowing.clone()]));

		// The whole item is rolled back — including the hold release — and parked for recovery.
		assert_eq!(FailedProxies::<Test>::get(&max), Some(overflowing));
		assert_eq!(held(HoldReason::ProxyDeposit, &max), 400);
		let (defs_after, deposit_after) = pallet_proxy::Proxies::<Test>::get(&max);
		assert_eq!(defs_after, defs_before);
		assert_eq!(deposit_after, deposit_before);
		assert!(migrator_events().contains(&Event::ProxiesReceived { count_good: 0, count_bad: 1 }));
	});
}

#[test]
fn reconcile_balances_reconciles_and_holds_the_lockdown() {
	new_test_ext().execute_with(|| {
		let alice = acc(1);
		// GIVEN some minted total from the accounts stage.
		assert_ok!(CtMigrator::receive_accounts(
			root(),
			vec![portable_account(&alice, 100, vec![])],
		));
		migrator_events();

		// WHEN the sender signals completion (with a mismatching burn total: reporting must not
		// block completion).
		assert_ok!(CtMigrator::reconcile_balances(root(), 0, 150));

		// THEN this chain is reconciled but still locked down: the relay chain's verification
		// window has not closed yet.
		assert_eq!(CtMigrationStage::<Test>::get(), MigrationStage::CoolOff);
		assert!(CtMigrationStage::<Test>::get().is_ongoing());
		let events = migrator_events();
		assert!(events.contains(&Event::MigrationFinished {
			rc_kept: 0,
			rc_migrated: 150,
			ct_minted: 100,
		}));
		assert!(events.contains(&Event::StageTransition {
			old: MigrationStage::DataMigrationOngoing,
			new: MigrationStage::CoolOff,
		}));

		// WHEN the relay chain closes the window. THEN the lockdown ends.
		assert_ok!(CtMigrator::end_lockdown(root()));
		assert_eq!(CtMigrationStage::<Test>::get(), MigrationStage::MigrationDone);

		// WHEN the same signal arrives again. THEN it is a no-op: a resent message is not worth
		// failing an XCM over.
		assert_ok!(CtMigrator::end_lockdown(root()));
		assert_eq!(CtMigrationStage::<Test>::get(), MigrationStage::MigrationDone);
	});
}

#[test]
fn the_handshake_answers_the_relay_chain_and_opens_the_migration() {
	new_test_ext().execute_with(|| {
		// WHEN the relay chain asks whether this chain is ready.
		assert_ok!(CtMigrator::start_migration(root()));

		// THEN the migration is open here and exactly one answer went upwards.
		assert_eq!(CtMigrationStage::<Test>::get(), MigrationStage::DataMigrationOngoing);
		assert_eq!(sent().len(), 1);
		assert_eq!(sent()[0].0, Location::parent());

		// WHEN the same ask arrives again. THEN it is answered again without reopening, so a
		// relay chain rewound to `Scheduled` can redo the handshake.
		assert_ok!(CtMigrator::start_migration(root()));
		assert_eq!(CtMigrationStage::<Test>::get(), MigrationStage::DataMigrationOngoing);
		assert_eq!(sent().len(), 2);

		// WHEN the migration has finished. THEN a start is refused rather than reopening a chain
		// that is already serving its new control plane.
		assert_ok!(CtMigrator::reconcile_balances(root(), 0, 0));
		assert_noop!(CtMigrator::start_migration(root()), Error::<Test>::AlreadyFinished);
	});
}

#[test]
fn an_unfinished_migration_cannot_be_completed() {
	new_test_ext().execute_with(|| {
		// WHEN the relay chain closes a window that never opened. THEN it is refused, so a stray
		// or reordered message cannot unlock this chain mid-migration.
		assert_noop!(CtMigrator::end_lockdown(root()), Error::<Test>::NotStarted);
		assert_ok!(CtMigrator::start_migration(root()));
		assert_noop!(CtMigrator::end_lockdown(root()), Error::<Test>::NotReconciled);
		assert_eq!(CtMigrationStage::<Test>::get(), MigrationStage::DataMigrationOngoing);
	});
}

#[test]
fn the_manager_drives_the_migration_but_cannot_appoint_one() {
	new_test_ext().execute_with(|| {
		let alice = acc(1); // appointed manager
		let signed = RuntimeOrigin::signed(alice.clone());

		// WHEN a signed account appoints itself. THEN it is refused.
		assert_noop!(CtMigrator::set_manager(signed.clone(), Some(alice.clone())), BadOrigin);

		// GIVEN Alice appointed manager by the admin origin.
		assert_ok!(CtMigrator::set_manager(root(), Some(alice.clone())));
		assert_eq!(Manager::<Test>::get(), Some(alice.clone()));
		assert!(
			migrator_events().contains(&Event::ManagerSet { old: None, new: Some(alice.clone()) })
		);

		// WHEN the manager sends the relay chain's two signals and forces a stage. THEN all
		// three are accepted: the manager stands in for a signal that never arrived.
		assert_ok!(CtMigrator::start_migration(signed.clone()));
		assert_eq!(CtMigrationStage::<Test>::get(), MigrationStage::DataMigrationOngoing);
		assert_eq!(sent().len(), 1);
		assert_ok!(CtMigrator::force_set_stage(signed.clone(), MigrationStage::CoolOff));
		assert_ok!(CtMigrator::end_lockdown(signed.clone()));
		assert_eq!(CtMigrationStage::<Test>::get(), MigrationStage::MigrationDone);

		// WHEN the manager tries a data batch, or to appoint a manager. THEN both are refused:
		// batches come from the relay chain alone, and appointments stay with the admin origin.
		assert_noop!(CtMigrator::receive_accounts(signed.clone(), vec![]), BadOrigin);
		assert_noop!(CtMigrator::set_manager(signed.clone(), None), BadOrigin);

		// WHEN the admin origin removes the manager. THEN Alice loses the powers.
		assert_ok!(CtMigrator::set_manager(root(), None));
		assert_eq!(Manager::<Test>::get(), None);
		assert!(migrator_events().contains(&Event::ManagerSet { old: Some(alice), new: None }));
		assert_noop!(CtMigrator::start_migration(signed), BadOrigin);
	});
}

#[test]
fn all_receive_calls_require_root() {
	new_test_ext().execute_with(|| {
		let signed = RuntimeOrigin::signed(acc(1));
		assert_noop!(CtMigrator::receive_accounts(signed.clone(), vec![]), BadOrigin);
		assert_noop!(CtMigrator::receive_registrar(signed.clone(), vec![], None), BadOrigin);
		assert_noop!(CtMigrator::receive_hrmp(signed.clone(), vec![]), BadOrigin);
		assert_noop!(CtMigrator::receive_proxies(signed.clone(), vec![]), BadOrigin);
		assert_noop!(CtMigrator::receive_hrmp_requests(signed.clone(), vec![]), BadOrigin);
		assert_noop!(CtMigrator::reconcile_balances(signed.clone(), 0, 0), BadOrigin);
		assert_noop!(CtMigrator::start_migration(signed.clone()), BadOrigin);
		assert_noop!(CtMigrator::end_lockdown(signed.clone()), BadOrigin);
		assert_noop!(
			CtMigrator::force_set_stage(signed.clone(), MigrationStage::MigrationDone),
			BadOrigin
		);
		assert_noop!(CtMigrator::set_manager(signed, None), BadOrigin);
	});
}

#[test]
fn the_relay_chains_queue_goes_first_on_a_duty_cycle_while_the_migration_runs() {
	new_test_ext().execute_with(|| {
		// The mock's pattern: three blocks of priority, one of round robin.
		let prioritised_blocks = |from: u64, to: u64| -> Vec<AggregateMessageOrigin> {
			(from..=to)
				.filter(|n| n % 4 < 3)
				.map(|_| AggregateMessageOrigin::Parent)
				.collect()
		};

		// GIVEN no migration. WHEN blocks pass. THEN no queue is forced.
		run_blocks(8);
		assert_eq!(ForcedHeads::get(), vec![]);

		// GIVEN an open migration. WHEN blocks pass. THEN the relay chain's queue goes first on
		// three blocks in four.
		assert_ok!(CtMigrator::start_migration(root()));
		let from = System::block_number() + 1;
		run_blocks(8);
		assert_eq!(ForcedHeads::get(), prioritised_blocks(from, from + 7));

		// WHEN the priority is disabled. THEN nothing is forced.
		assert_ok!(CtMigrator::set_dmp_queue_priority(root(), QueuePriority::Disabled));
		ForcedHeads::set(vec![]);
		run_blocks(4);
		assert_eq!(ForcedHeads::get(), vec![]);

		// WHEN the same configuration is set again, or one that never prioritises. THEN refused.
		assert_noop!(
			CtMigrator::set_dmp_queue_priority(root(), QueuePriority::Disabled),
			Error::<Test>::QueuePriorityAlreadySet
		);
		assert_noop!(
			CtMigrator::set_dmp_queue_priority(root(), QueuePriority::OverrideConfig(0, 5)),
			Error::<Test>::ZeroPriorityBlocks
		);
		assert_noop!(
			CtMigrator::set_dmp_queue_priority(
				RuntimeOrigin::signed(acc(1)),
				QueuePriority::Config
			),
			BadOrigin
		);

		// WHEN the lockdown ends. THEN the queue takes its turn like every other.
		assert_ok!(CtMigrator::set_dmp_queue_priority(root(), QueuePriority::Config));
		assert_ok!(CtMigrator::reconcile_balances(root(), 0, 0));
		assert_ok!(CtMigrator::end_lockdown(root()));
		ForcedHeads::set(vec![]);
		run_blocks(4);
		assert_eq!(ForcedHeads::get(), vec![]);
	});
}
