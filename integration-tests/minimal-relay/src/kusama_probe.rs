// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

//! What is on Kusama's relay chain that Polkadot's does not have.
//!
//! Kusama carries `pallet-society`, which reserves user funds for bids and vouches. No migrator
//! touches it and the post-AHM call filter closes every Society call, so anything it holds would
//! be frozen with no way to release it. This measures the exposure against a real snapshot.
//!
//! Run with `SNAP_RC` pointed at the Kusama relay snapshot:
//! `SNAP_RC=snapshots/snap_rc_ksm.snap cargo test -p polkadot-integration-tests-minimal-relay
//!  kusama_probe -- --nocapture`

use crate::mock::{load, Chain};

type Ksm = crate::mock::network::relay::Runtime;

/// Format a balance in whole tokens. The two networks differ in decimals (KSM 12, DOT 10), so
/// this reads them from the runtime rather than assuming either.
fn ksm(v: u128) -> String {
	#[cfg(feature = "kusama")]
	let (unit, sym) = (1_000_000_000_000f64, "KSM");
	#[cfg(not(feature = "kusama"))]
	let (unit, sym) = (10_000_000_000f64, "DOT");
	format!("{:.4} {sym}", v as f64 / unit)
}

#[cfg(feature = "kusama")]
#[tokio::test]
async fn society_exposure_on_kusama() {
	let mut rc = load(Chain::Relay).await;

	rc.execute_with(|| {
		let members = pallet_society::Members::<Ksm>::iter().count();
		let member_count = pallet_society::MemberCount::<Ksm>::get();
		let candidates = pallet_society::Candidates::<Ksm>::iter().count();
		let bids = pallet_society::Bids::<Ksm>::get();
		let pot = pallet_society::Pot::<Ksm>::get();
		let founder = pallet_society::Founder::<Ksm>::get();

		println!("\n### pallet-society on the Kusama relay chain");
		println!("founder:      {founder:?}");
		println!("members:      {members} (MemberCount says {member_count})");
		println!("candidates:   {candidates}");
		println!("bids:         {}", bids.len());
		println!("pot:          {}", ksm(pot));

		// Payouts are what members are owed; the pallet reserves them on the member's account
		// until claimed.
		let mut payout_accounts = 0usize;
		let mut payout_total = 0u128;
		for (_who, record) in pallet_society::Payouts::<Ksm>::iter() {
			if record.paid > 0 || !record.payouts.is_empty() {
				payout_accounts += 1;
				payout_total += record.payouts.iter().map(|(_, v)| *v).sum::<u128>();
			}
		}
		println!("payouts:      {payout_accounts} accounts owed {}", ksm(payout_total));

		// The pot is an ordinary account; its free balance is what a sweep would move.
		use sp_runtime::traits::AccountIdConversion;
		let pot_account: sp_runtime::AccountId32 =
			kusama_runtime::SocietyPalletId::get().into_account_truncating();
		let pot_free = pallet_balances::Pallet::<Ksm>::free_balance(&pot_account);
		println!("pot account:  {} free", ksm(pot_free));

		println!(
			"\nVERDICT: {}",
			if members == 0 && bids.is_empty() && candidates == 0 && payout_accounts == 0 {
				"Society holds no user state. Only the pot matters, and sweeping it is enough."
			} else {
				"Society holds live user state. Reserves here are frozen by the post-AHM filter \
				 unless the migration handles them."
			}
		);
	});
}

/// What phase 2 would actually have to move off Kusama's relay chain.
///
/// The Polkadot equivalents are asserted by the migration test; Kusama has never been run, so this
/// is a census rather than a check. It exists to size the work and to surface anything Polkadot
/// does not have.
#[tokio::test]
async fn what_kusama_has_to_migrate() {
	let mut rc = load(Chain::Relay).await;

	rc.execute_with(|| {
		use polkadot_runtime_common::paras_registrar;
		use runtime_parachains::hrmp;

		println!("\n### registrar");
		let mut paras = 0usize;
		let mut registered = 0usize;
		let mut locked = 0usize;
		let mut deposits = 0u128;
		let mut unbacked = 0usize;
		let mut unbacked_value = 0u128;
		let mut owed: std::collections::BTreeMap<sp_runtime::AccountId32, u128> =
			Default::default();
		for (id, info) in paras_registrar::Paras::<Ksm>::iter() {
			paras += 1;
			deposits += info.deposit;
			if runtime_parachains::paras::Pallet::<Ksm>::lifecycle(id).is_some() {
				registered += 1;
			}
			if info.locked.unwrap_or(false) {
				locked += 1;
			}
			// Managers often hold several paras, so the backing check has to be per manager
			// against the sum, not per para.
			*owed.entry(info.manager.clone()).or_default() += info.deposit;
		}
		for (manager, want) in &owed {
			let acc = frame_system::Account::<Ksm>::get(manager);
			if *want > acc.data.reserved {
				unbacked += 1;
				unbacked_value += want.saturating_sub(acc.data.reserved);
			}
		}
		println!("paras:        {paras} ({registered} onboarded, {locked} locked)");
		println!("deposits:     {}", ksm(deposits));
		println!(
			"managers:     {} total, {unbacked} whose reserve cannot cover what the registrar \
			 records ({} short)",
			owed.len(),
			ksm(unbacked_value),
		);

		println!("\n### hrmp");
		let channels = hrmp::HrmpChannels::<Ksm>::iter().count();
		let requests = hrmp::HrmpOpenChannelRequests::<Ksm>::iter().count();
		let hrmp_deposits: u128 = hrmp::HrmpChannels::<Ksm>::iter()
			.map(|(_, c)| c.sender_deposit + c.recipient_deposit)
			.sum();
		println!("channels:     {channels}");
		println!("requests:     {requests}");
		println!("deposits:     {}", ksm(hrmp_deposits));

		println!("\n### proxies, by type");
		use std::collections::BTreeMap;
		let mut by_type: BTreeMap<String, usize> = BTreeMap::new();
		let mut proxy_deposits = 0u128;
		let mut delegators = 0usize;
		for (_who, (defs, deposit)) in pallet_proxy::Proxies::<Ksm>::iter() {
			delegators += 1;
			proxy_deposits += deposit;
			for d in defs.iter() {
				*by_type.entry(format!("{:?}", d.proxy_type)).or_default() += 1;
			}
		}
		println!("delegators:   {delegators}, deposits {}", ksm(proxy_deposits));
		for (t, n) in &by_type {
			println!("  {t:<40} {n}");
		}

		println!("\n### balances");
		let mut accounts = 0usize;
		let mut free = 0u128;
		let mut reserved = 0u128;
		for (_who, acc) in frame_system::Account::<Ksm>::iter() {
			accounts += 1;
			free += acc.data.free;
			reserved += acc.data.reserved;
		}
		println!("accounts:     {accounts}");
		println!("free:         {}", ksm(free));
		println!("reserved:     {}", ksm(reserved));
		println!("issuance:     {}", ksm(pallet_balances::TotalIssuance::<Ksm>::get()));
	});
}

/// Identify the account the Kusama migration leaves funded on the relay chain.
#[cfg(feature = "kusama")]
#[tokio::test]
async fn who_is_left_holding_ksm() {
	let mut rc = load(Chain::Relay).await;

	rc.execute_with(|| {
		use sp_core::crypto::{Ss58AddressFormat, Ss58Codec};

		let target: sp_runtime::AccountId32 = [
			94u8, 205, 77, 159, 2, 85, 237, 61, 60, 90, 193, 22, 10, 150, 95, 14, 167, 67, 183, 69,
			51, 3, 111, 30, 77, 63, 75, 252, 67, 249, 240, 97,
		]
		.into();

		let acc = frame_system::Account::<Ksm>::get(&target);
		println!("\naccount:   {}", target.to_ss58check_with_version(Ss58AddressFormat::custom(2)));
		println!("free:      {}", ksm(acc.data.free));
		println!("reserved:  {}", ksm(acc.data.reserved));
		println!("frozen:    {}", ksm(acc.data.frozen));
		println!(
			"providers: {}  consumers: {}  nonce: {}",
			acc.providers, acc.consumers, acc.nonce
		);

		println!("\nfreezes:   {:?}", pallet_balances::Freezes::<Ksm>::get(&target).len());
		for h in pallet_balances::Holds::<Ksm>::get(&target).iter() {
			println!("hold:      {:?} = {}", h.id, ksm(h.amount));
		}
		let locks = pallet_balances::Locks::<Ksm>::get(&target);
		println!("locks:     {}", locks.len());
		for l in locks.iter() {
			println!("  id {:?} amount {}", core::str::from_utf8(&l.id), ksm(l.amount));
		}
		println!("proxies:   {}", pallet_proxy::Proxies::<Ksm>::get(&target).0.len());
		println!(
			"is a para manager: {}",
			polkadot_runtime_common::paras_registrar::Paras::<Ksm>::iter()
				.any(|(_, i)| i.manager == target)
		);
		println!("session keys: {}", pallet_session::NextKeys::<Ksm>::get(&target).is_some());
	});
}

/// Every hold reason in use on the Kusama relay chain, and how much sits under each.
#[tokio::test]
async fn what_holds_exist_on_kusama() {
	let mut rc = load(Chain::Relay).await;

	rc.execute_with(|| {
		use std::collections::BTreeMap;
		let mut by_reason: BTreeMap<String, (usize, u128)> = BTreeMap::new();
		let mut unnamed_reserve_total = 0u128;
		let mut held_total = 0u128;

		for (who, acc) in frame_system::Account::<Ksm>::iter() {
			let holds = pallet_balances::Holds::<Ksm>::get(&who);
			let this_held: u128 = holds.iter().map(|h| h.amount).sum();
			held_total += this_held;
			unnamed_reserve_total += acc.data.reserved.saturating_sub(this_held);
			for h in holds.iter() {
				let e = by_reason.entry(format!("{:?}", h.id)).or_default();
				e.0 += 1;
				e.1 += h.amount;
			}
		}

		println!("\n### holds on the Kusama relay chain");
		for (reason, (n, total)) in &by_reason {
			println!("  {reason:<50} {n:>4} accounts  {}", ksm(*total));
		}
		println!("\ntotal under named holds:   {}", ksm(held_total));
		println!("total unnamed reserve:     {}", ksm(unnamed_reserve_total));
	});
}

/// What the preimage that strands a para manager actually is, and whether anything still needs it.
#[cfg(feature = "kusama")]
#[tokio::test]
async fn what_is_the_preimage() {
	let mut rc = load(Chain::Relay).await;

	rc.execute_with(|| {
		use pallet_preimage::RequestStatus;
		use sp_core::crypto::{Ss58AddressFormat, Ss58Codec};

		println!("\n### preimages on the Kusama relay chain");
		for (hash, status) in pallet_preimage::RequestStatusFor::<Ksm>::iter() {
			match status {
				RequestStatus::Unrequested { ticket, len } => println!(
					"UNREQUESTED {hash:?}\n  depositor {} len {len} bytes\n  -> nothing has \
					 asked for this; the depositor can `unnote_preimage` and take the deposit back",
					ticket.0.to_ss58check_with_version(Ss58AddressFormat::custom(2)),
				),
				RequestStatus::Requested { maybe_ticket, count, maybe_len } => println!(
					"REQUESTED   {hash:?}\n  outstanding requests {count} len {maybe_len:?}\n  \
					 depositor {:?}\n  -> the chain still needs it; it cannot simply be dropped",
					maybe_ticket
						.map(|(a, _)| a.to_ss58check_with_version(Ss58AddressFormat::custom(2))),
				),
			}
		}

		// Who would still be pointing at one: governance tracks that stay on the Kusama relay.
		println!(
			"\nreferenda (fellowship): {}",
			pallet_referenda::ReferendumInfoFor::<Ksm, pallet_referenda::Instance2>::iter().count()
		);
		println!("scheduled agenda slots: {}", pallet_scheduler::Agenda::<Ksm>::iter().count());
	});
}

/// The `DelegatedStaking` holds that outlive AHM phase 1 on both relays.
#[tokio::test]
async fn what_are_the_delegated_staking_holds() {
	let mut rc = load(Chain::Relay).await;

	rc.execute_with(|| {
		use sp_core::crypto::{Ss58AddressFormat, Ss58Codec};

		println!("\n### delegated-staking residue");
		println!("delegators: {}", pallet_delegated_staking::Delegators::<Ksm>::iter().count());
		println!("agents:     {}", pallet_delegated_staking::Agents::<Ksm>::iter().count());

		for (who, _) in frame_system::Account::<Ksm>::iter() {
			let holds = pallet_balances::Holds::<Ksm>::get(&who);
			if holds.is_empty() {
				continue;
			}
			let acc = frame_system::Account::<Ksm>::get(&who);
			println!(
				"\n{}\n  free {} reserved {} (held {})",
				who.to_ss58check_with_version(Ss58AddressFormat::custom(2)),
				ksm(acc.data.free),
				ksm(acc.data.reserved),
				ksm(holds.iter().map(|h| h.amount).sum::<u128>()),
			);
			println!(
				"  providers {} consumers {} nonce {}",
				acc.providers, acc.consumers, acc.nonce
			);
			println!(
				"  is delegator: {}  is agent: {}",
				pallet_delegated_staking::Delegators::<Ksm>::contains_key(&who),
				pallet_delegated_staking::Agents::<Ksm>::contains_key(&who),
			);
			if let Some(d) = pallet_delegated_staking::Delegators::<Ksm>::get(&who) {
				println!(
					"  delegation: {} to agent {}",
					ksm(d.amount),
					d.agent.to_ss58check_with_version(Ss58AddressFormat::custom(2))
				);
			}
		}
	});
}

/// The delegator whose proxy deposit does not get resized on Coretime.
#[cfg(feature = "kusama")]
#[tokio::test]
async fn the_unresized_delegator() {
	use crate::mock::network::ct;
	let mut ct_ext = load(Chain::Coretime).await;
	let mut rc = load(Chain::Relay).await;

	let who: sp_runtime::AccountId32 = {
		use sp_core::crypto::Ss58Codec;
		sp_runtime::AccountId32::from_ss58check("FBeL7DePfD8RbGPjt96g2VdKmbkkRMt5UudRwA9GxjTXCU8")
			.expect("valid address")
	};

	rc.execute_with(|| {
		let (defs, deposit) = pallet_proxy::Proxies::<Ksm>::get(&who);
		println!("\nON THE RELAY CHAIN");
		println!("  deposit {}", ksm(deposit));
		for d in defs.iter() {
			println!("  def {:?} delay {:?}", d.proxy_type, d.delay);
		}
		let acc = frame_system::Account::<Ksm>::get(&who);
		println!("  free {} reserved {}", ksm(acc.data.free), ksm(acc.data.reserved));
	});

	ct_ext.execute_with(|| {
		let (defs, deposit) = pallet_proxy::Proxies::<ct::Runtime>::get(&who);
		println!("\nON CORETIME, BEFORE THE MIGRATION");
		println!("  deposit {}", ksm(deposit));
		println!("  existing defs: {}", defs.len());
		for d in defs.iter() {
			println!("  def {:?} delay {:?}", d.proxy_type, d.delay);
		}
		let acc = frame_system::Account::<ct::Runtime>::get(&who);
		println!("  free {} reserved {}", ksm(acc.data.free), ksm(acc.data.reserved));
		println!(
			"  MaxProxies on CT: {}",
			<ct::Runtime as pallet_proxy::Config>::MaxProxies::get()
		);
	});
}

/// Who on the relay chain carries the proxy deposit that ends up stuck on Coretime?
#[cfg(feature = "kusama")]
#[tokio::test]
async fn find_the_stuck_proxy_deposit() {
	let mut rc = load(Chain::Relay).await;
	const STUCK: u128 = 668_033_331_300;

	rc.execute_with(|| {
		use sp_core::crypto::{Ss58AddressFormat, Ss58Codec};

		let target = sp_runtime::AccountId32::from_ss58check(
			"FBeL7DePfD8RbGPjt96g2VdKmbkkRMt5UudRwA9GxjTXCU8",
		)
		.expect("valid");
		println!("\ntarget raw bytes: {:?}", <[u8; 32]>::from(target.clone()));

		println!("\naccounts on the relay chain with a proxy deposit of {STUCK}:");
		for (who, (defs, deposit)) in pallet_proxy::Proxies::<Ksm>::iter() {
			if deposit == STUCK {
				println!(
					"  {} == target? {}",
					who.to_ss58check_with_version(Ss58AddressFormat::custom(2)),
					who == target
				);
				for d in defs.iter() {
					println!("    def {:?}", d.proxy_type);
				}
			}
		}

		println!("\nthe target's own relay entry:");
		let (defs, deposit) = pallet_proxy::Proxies::<Ksm>::get(&target);
		println!("  deposit {deposit} defs {}", defs.len());
		println!(
			"  exists in Proxies map: {}",
			pallet_proxy::Proxies::<Ksm>::contains_key(&target)
		);
		println!(
			"  system account exists: {}",
			frame_system::Account::<Ksm>::contains_key(&target)
		);
	});
}

/// Does para 2105's *child* sovereign on the relay chain hold the proxy deposit?
#[cfg(feature = "kusama")]
#[tokio::test]
async fn para_2105_child_sovereign() {
	let mut rc = load(Chain::Relay).await;

	rc.execute_with(|| {
		use sp_core::crypto::{Ss58AddressFormat, Ss58Codec};
		let mut bytes = [0u8; 32];
		bytes[..4].copy_from_slice(b"para");
		bytes[4..6].copy_from_slice(&2105u32.to_le_bytes()[..2]);
		let child: sp_runtime::AccountId32 = bytes.into();

		println!("\npara 2105 child sovereign on the RC:");
		println!("  {}", child.to_ss58check_with_version(Ss58AddressFormat::custom(2)));
		let (defs, deposit) = pallet_proxy::Proxies::<Ksm>::get(&child);
		println!("  proxy deposit {}", ksm(deposit));
		for d in defs.iter() {
			println!("  def {:?}", d.proxy_type);
		}
		let acc = frame_system::Account::<Ksm>::get(&child);
		println!("  free {} reserved {}", ksm(acc.data.free), ksm(acc.data.reserved));
	});
}

/// Which relay-chain records are keyed by a para sovereign account? Those are the ones the
/// untranslated stages get wrong.
#[tokio::test]
async fn records_keyed_by_a_para_sovereign() {
	let mut rc = load(Chain::Relay).await;

	rc.execute_with(|| {
		let is_sovereign = |a: &sp_runtime::AccountId32| {
			let b: &[u8] = a.as_ref();
			b.starts_with(b"para") || b.starts_with(b"sibl")
		};

		let proxies = pallet_proxy::Proxies::<Ksm>::iter()
			.filter(|(who, _)| is_sovereign(who))
			.count();
		let managers = polkadot_runtime_common::paras_registrar::Paras::<Ksm>::iter()
			.filter(|(_, i)| is_sovereign(&i.manager))
			.count();
		let multisigs = pallet_multisig::Multisigs::<Ksm>::iter()
			.filter(|(who, _, _)| is_sovereign(who))
			.count();

		println!("\nrelay-chain records keyed by a para sovereign:");
		println!("  proxy delegators:  {proxies}");
		println!("  registrar managers:{managers}");
		println!("  multisig holders:  {multisigs}");
	});
}

/// Why is this below-ED pure-like delegator not migrating?
#[cfg(feature = "kusama")]
#[tokio::test]
async fn the_below_ed_pure() {
	let mut rc = load(Chain::Relay).await;

	rc.execute_with(|| {
		use sp_core::crypto::Ss58Codec;
		let who = sp_runtime::AccountId32::from_ss58check(
			"HXYacFPwGfZn2n41sGQ6v9tbxx2NXL1vF5YoRJwVLL1aEPy",
		)
		.expect("valid");

		let info = frame_system::Account::<Ksm>::get(&who);
		println!(
			"\nnonce {} providers {} consumers {}",
			info.nonce, info.providers, info.consumers
		);
		println!(
			"free {} reserved {} frozen {}",
			ksm(info.data.free),
			ksm(info.data.reserved),
			ksm(info.data.frozen)
		);
		println!("relay ED: {}", ksm(<Ksm as pallet_balances::Config>::ExistentialDeposit::get()));
		println!(
			"locks {} freezes {} holds {}",
			pallet_balances::Locks::<Ksm>::get(&who).len(),
			pallet_balances::Freezes::<Ksm>::get(&who).len(),
			pallet_balances::Holds::<Ksm>::get(&who).len()
		);
		let (defs, deposit) = pallet_proxy::Proxies::<Ksm>::get(&who);
		println!("proxy deposit {} defs:", ksm(deposit));
		for d in defs.iter() {
			println!("  {:?}", d.proxy_type);
		}
		println!(
			"is_pure_like: {}",
			pallet_rc2_migrator::accounts::AccountsMigrator::<Ksm>::is_pure_like(&who, &info)
		);
		println!(
			"can_migrate:  {}",
			pallet_rc2_migrator::accounts::AccountsMigrator::<Ksm>::can_migrate(&who, &info, None)
		);
	});
}

/// Every module account on the relay chain that still holds a balance.
///
/// `can_migrate` refuses anything `modl`-prefixed, so a funded pot is stranded unless it is in
/// `SweepAccounts`. This is the list that config has to cover — measured, not guessed.
#[tokio::test]
async fn module_accounts_that_need_sweeping() {
	let mut rc = load(Chain::Relay).await;

	rc.execute_with(|| {
		use sp_core::crypto::{Ss58AddressFormat, Ss58Codec};
		let prefix: u16 = if cfg!(feature = "kusama") { 2 } else { 0 };

		let mut total = 0u128;
		println!("\n### funded module accounts on the relay chain");
		for (who, acc) in frame_system::Account::<Ksm>::iter() {
			let bytes: &[u8] = who.as_ref();
			if !bytes.starts_with(b"modl") {
				continue;
			}
			let sum = acc.data.free + acc.data.reserved;
			if sum == 0 {
				continue;
			}
			total += sum;
			// `modl` + the PalletId is the readable part of the derivation.
			let tag = core::str::from_utf8(&bytes[4..12]).unwrap_or("????????");
			println!(
				"  {:<16} {:<50} {}",
				tag,
				who.to_ss58check_with_version(Ss58AddressFormat::custom(prefix)),
				ksm(sum),
			);
		}
		println!("  total in funded pots: {}", ksm(total));
	});
}
