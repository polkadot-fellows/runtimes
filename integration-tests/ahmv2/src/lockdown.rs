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

//! Snapshot test for the AHM v2 lockdown: the call filters, XCM barrier and teleport trust on
//! both chains, checked at every stage of a real run.

use crate::mock::*;
use codec::Encode;
use core::{cell::RefCell, mem::discriminant};
use cumulus_primitives_core::ParaId;
use frame_support::{
	assert_noop, assert_ok,
	dispatch::PostDispatchInfo,
	hypothetically,
	traits::{fungible::Mutate, ServiceQueues},
};
use network::constants::{currency::UNITS, system_parachain, time::MINUTES};
use sp_core::H256;
use sp_io::hashing::blake2_256;
use sp_runtime::{
	traits::{AccountIdConversion, Dispatchable},
	AccountId32, DispatchError,
};
use std::collections::HashSet;
use xcm::{latest::prelude::*, VersionedXcm};

/// A para that is not a system chain.
const OUTSIDER_PARA: u32 = 4242;

/// Accounts the lockdown probes use. None of them exists on either snapshot.
const ALICE: AccountId32 = AccountId32::new([0xa1; 32]); // signs every call
const BOB: AccountId32 = AccountId32::new([0xb0; 32]); // receives; the proxied account on Coretime
const CAROL: AccountId32 = AccountId32::new([0xca; 32]); // receives through the proxy

/// The lockdown at every stage of a real run, through both chains' real call filters and XCM
/// configuration.
///
/// From the relay chain's start signal on: a signed transfer is filtered, the manager still drives
/// the migrator, only the Coretime chain and Asset Hub get past the barrier, and a teleport from
/// Asset Hub mints nothing. None of that reopens at the end. On the Coretime chain, adding a proxy
/// is refused exactly while its migration runs, and a proxy already in place works throughout.
#[tokio::test(flavor = "multi_thread")]
async fn the_lockdown_holds_at_every_stage() {
	let (mut rc, mut ct) = tokio::join!(load(Chain::Relay), load(CoretimePara::CHAIN));

	// GIVEN no backlog from the snapshot, which would be processed alongside the probes' messages
	// and move the issuance they measure.
	rc.execute_with(drain_all_inbound_queues);

	// Each chain's stages the probes ran at, by variant, so the end of the test can check that none
	// was skipped.
	let rc_stages = RefCell::new(HashSet::new());
	let ct_stages = RefCell::new(HashSet::new());

	// WHEN the migration runs from `Pending` to `MigrationDone`. THEN every probe holds at every
	// stage it stops at.
	walk_migration(
		&mut rc,
		&mut ct,
		// One block per data stage; 30 is room to spare over the 15 there are.
		30,
		&mut [],
		|| {
			rc_stages.borrow_mut().insert(discriminant(&rc_stage()));
			probe_rc_lockdown();
		},
		|| {
			ct_stages.borrow_mut().insert(discriminant(&ct_stage()));
			probe_ct_lockdown();
		},
	);

	// Every variant of the relay chain's `MigrationStage` was probed, and all three of Coretime's.
	assert_eq!(rc_stages.borrow().len(), 21);
	assert_eq!(ct_stages.borrow().len(), 3);
}

/// The relay chain's lockdown at its current stage: open before the start, closed from the start
/// on, including after `MigrationDone`.
fn probe_rc_lockdown() {
	type Rc = network::relay::Runtime;
	let stage = rc_stage();
	let started = stage.has_started();
	let filtered: DispatchError = frame_system::Error::<Rc>::CallFiltered.into();

	// A signed transfer.
	hypothetically!({
		// GIVEN Alice has funds.
		assert_ok!(pallet_balances::Pallet::<Rc>::mint_into(&ALICE, 100 * UNITS));

		// WHEN she transfers to Bob. THEN it is filtered from the start on.
		let transfer =
			network::relay::RuntimeCall::Balances(pallet_balances::Call::transfer_keep_alive {
				dest: BOB.into(),
				value: 10 * UNITS,
			});
		if started {
			assert_noop!(dispatch_signed(&ALICE, transfer), filtered);
		} else {
			assert_ok!(dispatch_signed(&ALICE, transfer));
		}
		assert_eq!(
			pallet_balances::Pallet::<Rc>::free_balance(&BOB),
			if started { 0 } else { 10 * UNITS },
			"the transfer's recipient at {stage:?}"
		);
	});

	// The manager drives the migration with signed calls, which the filter lets through.
	hypothetically!({
		// GIVEN Alice is the manager.
		assert_ok!(pallet_rc2_migrator::Pallet::<Rc>::set_manager(
			network::relay::RuntimeOrigin::root(),
			Some(ALICE),
		));

		// WHEN she pauses the migration. THEN it pauses while the migration runs, and is refused
		// by the migrator, not the filter, otherwise.
		let pause =
			network::relay::RuntimeCall::Rc2Migrator(pallet_rc2_migrator::Call::pause_migration {});
		if stage.is_ongoing() {
			assert_ok!(dispatch_signed(&ALICE, pause));
		} else {
			assert_noop!(
				dispatch_signed(&ALICE, pause),
				pallet_rc2_migrator::Error::<Rc>::NotRunning
			);
		}
	});

	// Unpaid messages from system chains: from the start, only Coretime and Asset Hub get in.
	for (para, admitted_once_started) in [
		(system_parachain::BROKER_ID, true),
		(system_parachain::ASSET_HUB_ID, true),
		(system_parachain::BRIDGE_HUB_ID, false),
	] {
		assert_eq!(
			hypothetically!(deliver_ump(para, unpaid_noop())),
			if started && !admitted_once_started { Delivery::Refused } else { Delivery::Executed },
			"an unpaid message from para {para} at {stage:?}"
		);
	}

	// An ordinary para paying from its sovereign account here.
	hypothetically!({
		// GIVEN the para's sovereign account here has funds.
		let sovereign: AccountId32 = ParaId::from(OUTSIDER_PARA).into_account_truncating();
		assert_ok!(pallet_balances::Pallet::<Rc>::mint_into(&sovereign, 100 * UNITS));

		// WHEN it sends a paid message. THEN the barrier refuses it from the start on.
		assert_eq!(
			deliver_ump(OUTSIDER_PARA, paid_transfer_to(&BOB)),
			if started { Delivery::Refused } else { Delivery::Executed },
			"a paid message from para {OUTSIDER_PARA} at {stage:?}"
		);
	});

	// A teleport from Asset Hub mints here until the start, and nothing from then on.
	hypothetically!({
		let issuance = pallet_balances::Pallet::<Rc>::total_issuance();
		let landed = if started { 0 } else { 10 * UNITS };
		assert_eq!(
			deliver_ump(system_parachain::ASSET_HUB_ID, teleport_to(&BOB, 10 * UNITS)),
			if started {
				Delivery::Failed(XcmError::UntrustedTeleportLocation)
			} else {
				Delivery::Executed
			},
			"a teleport from Asset Hub at {stage:?}"
		);
		assert_eq!(pallet_balances::Pallet::<Rc>::free_balance(&BOB), landed, "at {stage:?}");
		assert_eq!(
			pallet_balances::Pallet::<Rc>::total_issuance(),
			issuance + landed,
			"issuance after a teleport at {stage:?}"
		);
	});
}

/// The Coretime chain's lockdown at its current stage: changes to the proxy map closed only while
/// the migration runs, announcements open throughout.
fn probe_ct_lockdown() {
	type Ct = network::ct::Runtime;
	let stage = ct_stage();
	let filtered: DispatchError = frame_system::Error::<Ct>::CallFiltered.into();

	// Adding a proxy.
	hypothetically!({
		assert_ok!(pallet_balances::Pallet::<Ct>::mint_into(&ALICE, 100 * UNITS));
		let add = network::ct::RuntimeCall::Proxy(pallet_proxy::Call::add_proxy {
			delegate: BOB.into(),
			proxy_type: network::ct::ProxyType::Any,
			delay: 0,
		});
		if stage.is_ongoing() {
			assert_noop!(dispatch_signed(&ALICE, add), filtered);
		} else {
			assert_ok!(dispatch_signed(&ALICE, add));
		}
	});

	// Using a proxy, here to make a transfer.
	hypothetically!({
		// GIVEN Alice is already Bob's proxy.
		assert_ok!(pallet_balances::Pallet::<Ct>::mint_into(&BOB, 100 * UNITS));
		assert_ok!(pallet_proxy::Pallet::<Ct>::add_proxy_delegate(
			&BOB,
			ALICE,
			network::ct::ProxyType::Any,
			0,
		));

		// WHEN Alice transfers from Bob's account through the proxy.
		let transfer =
			network::ct::RuntimeCall::Balances(pallet_balances::Call::transfer_keep_alive {
				dest: CAROL.into(),
				value: 10 * UNITS,
			});
		let proxy = network::ct::RuntimeCall::Proxy(pallet_proxy::Call::proxy {
			real: BOB.into(),
			force_proxy_type: None,
			call: Box::new(transfer),
		});
		assert_ok!(dispatch_signed(&ALICE, proxy));

		// THEN the transfer lands. `proxy` succeeds even when the inner call fails, so check what
		// the inner call did.
		assert_eq!(
			pallet_balances::Pallet::<Ct>::free_balance(&CAROL),
			10 * UNITS,
			"the proxied transfer at {stage:?}"
		);
	});

	// Announcing through a time-delayed proxy, and the owner cancelling it.
	hypothetically!({
		// GIVEN Alice is already Bob's proxy, with a delay.
		assert_ok!(pallet_balances::Pallet::<Ct>::mint_into(&ALICE, 100 * UNITS));
		assert_ok!(pallet_balances::Pallet::<Ct>::mint_into(&BOB, 100 * UNITS));
		assert_ok!(pallet_proxy::Pallet::<Ct>::add_proxy_delegate(
			&BOB,
			ALICE,
			network::ct::ProxyType::Any,
			10,
		));

		// WHEN Alice announces a call. THEN the announcement is recorded.
		let call_hash = H256::repeat_byte(1);
		let announce = network::ct::RuntimeCall::Proxy(pallet_proxy::Call::announce {
			real: BOB.into(),
			call_hash,
		});
		assert_ok!(dispatch_signed(&ALICE, announce));
		assert_eq!(pallet_proxy::Announcements::<Ct>::get(&ALICE).0.len(), 1);

		// WHEN Bob rejects it. THEN it is gone and Alice's deposit is released.
		let reject = network::ct::RuntimeCall::Proxy(pallet_proxy::Call::reject_announcement {
			delegate: ALICE.into(),
			call_hash,
		});
		assert_ok!(dispatch_signed(&BOB, reject));
		assert!(pallet_proxy::Announcements::<Ct>::get(&ALICE).0.is_empty(), "at {stage:?}");
		assert_eq!(
			pallet_balances::Pallet::<Ct>::reserved_balance(&ALICE),
			0,
			"the announcement deposit is released at {stage:?}"
		);
	});

	// TODO(ahm-v2): probe `RegistrarPara` and `HrmpPara` once they are in the runtime: refused
	// until `MigrationDone`, open from then on.
}

/// Dispatch `call` from `who`, through the runtime's call filter.
fn dispatch_signed<C>(who: &AccountId32, call: C) -> Result<(), DispatchError>
where
	C: Dispatchable<PostInfo = PostDispatchInfo>,
	C::RuntimeOrigin: From<frame_system::RawOrigin<AccountId32>>,
{
	call.dispatch(frame_system::RawOrigin::Signed(who.clone()).into())
		.map(|_| ())
		.map_err(|e| e.error)
}

/// Run relay-chain blocks until no inbound queue holds an unprocessed message.
fn drain_all_inbound_queues() {
	for _ in 0..5 * MINUTES {
		if pallet_message_queue::ServiceHead::<network::relay::Runtime>::get().is_none() {
			return;
		}
		next_block_rc_unchecked();
	}
	panic!("the relay chain's inbound queues did not drain");
}

/// What became of one inbound message.
#[derive(Debug, PartialEq, Eq)]
enum Delivery {
	/// Admitted, and every instruction succeeded.
	Executed,
	/// Admitted, and an instruction failed with this error.
	Failed(XcmError),
	/// Turned away by the barrier before any instruction ran.
	Refused,
}

/// Send `message` up from `para` and service the relay chain's message queue.
///
/// `message` must not end in a `SetTopic`: the queue reports an admitted message under that topic
/// rather than under the hash of its bytes.
fn deliver_ump(para: u32, message: Xcm<()>) -> Delivery {
	let encoded = VersionedXcm::from(message).encode();
	let hash = blake2_256(&encoded);
	enqueue_ump(para.into(), vec![encoded]);

	frame_system::Pallet::<network::relay::Runtime>::reset_events();
	<network::relay::MessageQueue as ServiceQueues>::service_queues(Weight::MAX);

	let events = frame_system::Pallet::<network::relay::Runtime>::events();
	let success =
		events.iter().find_map(|record| match record.event {
			network::relay::RuntimeEvent::MessageQueue(
				pallet_message_queue::Event::Processed { id, success, .. },
			) if id.0 == hash => Some(success),
			_ => None,
		});
	// The executor reports a failed instruction with its error. A barrier refusal is reported only
	// as an unsuccessful `Processed`.
	let failure = events.iter().find_map(|record| match &record.event {
		network::relay::RuntimeEvent::XcmPallet(pallet_xcm::Event::ProcessXcmError {
			error,
			message_id,
			..
		}) if *message_id == hash => Some(*error),
		_ => None,
	});
	match (success, failure) {
		(Some(true), None) => Delivery::Executed,
		(Some(false), Some(error)) => Delivery::Failed(error),
		(Some(false), None) => Delivery::Refused,
		(None, None) => panic!("the relay chain never processed the message from para {para}"),
		outcome => panic!("inconsistent report for the message from para {para}: {outcome:?}"),
	}
}

/// A message that asks for free execution and does nothing else, so whether it runs is the
/// barrier's verdict alone.
fn unpaid_noop() -> Xcm<()> {
	Xcm(vec![UnpaidExecution { weight_limit: Unlimited, check_origin: None }, ClearOrigin])
}

/// A message that pays for itself from the sender's account here and deposits the rest with `to`.
fn paid_transfer_to(to: &AccountId32) -> Xcm<()> {
	Xcm(vec![
		// Take 10 units out of the sender's account here.
		WithdrawAsset((Here, 10 * UNITS).into()),
		// Pay for this message's execution out of it, at most 1 unit.
		BuyExecution { fees: (Here, UNITS).into(), weight_limit: Unlimited },
		// Put the rest in `to`'s account. Withdrawing 10 keeps the rest above the existential
		// deposit, so the deposit cannot fail for that reason.
		DepositAsset { assets: AllCounted(1).into(), beneficiary: account(to) },
	])
}

/// A teleport of `amount` to `to`, unpaid as system chains send it.
fn teleport_to(to: &AccountId32, amount: u128) -> Xcm<()> {
	Xcm(vec![
		UnpaidExecution { weight_limit: Unlimited, check_origin: None },
		ReceiveTeleportedAsset((Here, amount).into()),
		DepositAsset { assets: AllCounted(1).into(), beneficiary: account(to) },
	])
}

fn account(who: &AccountId32) -> Location {
	Junction::AccountId32 { network: None, id: who.clone().into() }.into()
}
