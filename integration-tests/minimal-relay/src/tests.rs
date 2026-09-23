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

//! Tests for the Minimal Relay migration

use crate::mock::*;
use codec::{Decode, Encode};
use frame_support::{
	assert_ok,
	traits::{fungible::Inspect, OnInitialize},
};
use migrator_types::PortableProxyType;
use pallet_rc2_migrator::{RcMigratedBalance, RcMigrationStage};
use polkadot_runtime_common::paras_registrar;
use runtime_parachains::hrmp::HrmpChannels;
use sp_core::crypto::{Ss58AddressFormat, Ss58Codec};
use sp_runtime::{
	traits::{AccountIdConversion, Convert, Dispatchable},
	AccountId32,
};
use std::collections::{BTreeMap, BTreeSet};
use xcm::latest::prelude::*;

/// An XCM program that executes `call` on the destination with the sender's sovereign-account
/// origin. Both the RC and the system parachains grant each other unpaid execution, so no fee
/// payment is needed.
fn unpaid_transact<Call: Encode>(call: Call) -> Xcm<()> {
	Xcm(vec![
		UnpaidExecution { weight_limit: Unlimited, check_origin: None },
		Transact {
			origin_kind: OriginKind::SovereignAccount,
			fallback_max_weight: None,
			call: call.encode().into(),
		},
	])
}

/// An XCM program that executes `call` on the destination **as the sender itself** — the shape a
/// parachain uses to dispatch a call with its own origin, as opposed to as its sovereign account.
fn unpaid_transact_native<Call: Encode>(call: Call) -> Vec<u8> {
	xcm::VersionedXcm::<()>::from(Xcm(vec![
		UnpaidExecution { weight_limit: Unlimited, check_origin: None },
		Transact {
			origin_kind: OriginKind::Native,
			fallback_max_weight: None,
			call: call.encode().into(),
		},
	]))
	.encode()
}

/// Mirror of the accounts-stage split rule, for assertions: how much of an account's free
/// balance goes to the Coretime chain (working buffer) versus Asset Hub (teleport). Valid for
/// keyed (`nonce > 0`) accounts whose reserve is fully CT-bound (which is what
/// [`find_clean_manager`] selects); never-signed `Any`-delegators route everything to CT instead.
fn expected_split(free: u128, reserved: u128) -> (u128, u128) {
	let buffer: u128 = crate::mock::network::relay::CtFreeBuffer::get();
	let ah_ed: u128 = crate::mock::network::relay::AhExistentialDeposit::get();
	let mut ct_free = if reserved == 0 { 0 } else { free.min(buffer) };
	let mut ah_free = free - ct_free;
	if ah_free > 0 && ah_free < ah_ed && reserved > 0 {
		ct_free += ah_free;
		ah_free = 0;
	}
	(ct_free, ah_free)
}

/// Polkadot-format SS58 of an account, for census output.
/// SS58 prefix of the network under test, so printed addresses match what a block explorer shows.
const SS58_PREFIX: u16 = if cfg!(feature = "kusama") { 2 } else { 0 };

/// Token symbol and decimals differ between the networks (DOT 10, KSM 12); printing the wrong one
/// silently misreports every figure by a factor of 100.
const TOKEN: &str = if cfg!(feature = "kusama") { "KSM" } else { "DOT" };
const TOKEN_UNIT: f64 = if cfg!(feature = "kusama") { 1e12 } else { 1e10 };

fn ss58(a: &AccountId32) -> String {
	a.to_ss58check_with_version(Ss58AddressFormat::custom(SS58_PREFIX))
}

/// Where an account's balance, deposits and records continue on the destination chains.
///
/// Mirrors `rc2-migrator`'s rule rather than calling it: a child sovereign (`para…`) is the same
/// parachain seen from a sibling (`sibl…`), and everyone else keeps their address. Stated here so
/// the test asserts the intended behaviour instead of whatever the migrator happens to do.
fn translate_destination(who: &AccountId32) -> AccountId32 {
	let bytes: &[u8] = who.as_ref();
	if let Some(rest) = bytes.strip_prefix(b"para") {
		let para_id = u32::from(rest[0]) | (u32::from(rest[1]) << 8);
		return migrator_types::sibling_account(para_id);
	}
	who.clone()
}

fn dot(v: u128) -> f64 {
	v as f64 / TOKEN_UNIT
}

/// Decompose every account on the RC by why the migration leaves it behind, with per-account
/// lines for everything that is not aggregate dust.
///
/// With `only_referenced`, regular accounts are printed only when extra consumer references
/// would block their withdrawal — the pre-migration prediction, where printing the millions of
/// migratable accounts would be noise. Without it, every remaining account is printed: the
/// post-migration measurement of the "RC → 0" gap.
fn print_remaining_on_rc(only_referenced: bool) {
	type Rc = crate::mock::network::relay::Runtime;
	let ed = pallet_balances::Pallet::<Rc>::minimum_balance();

	let (mut dust_n, mut dust_amt) = (0u32, 0u128);
	// Why each dust account is not reapable by the sweep stage (a reapable one would be).
	let mut dust_blockers = BTreeMap::<&str, (u32, u128)>::new();
	let (mut accounts, mut total_sum) = (0u32, 0u128);
	for (who, info) in frame_system::Account::<Rc>::iter() {
		let d = &info.data;
		let total = d.free + d.reserved;
		accounts += 1;
		total_sum += total;
		let bytes: &[u8] = who.as_ref();
		if bytes.starts_with(b"modl") {
			let name = String::from_utf8_lossy(&bytes[4..12]);
			println!(
				"module `{}`: {} | free {:.4} reserved {:.4}",
				name.trim_end_matches('\0'),
				ss58(&who),
				dot(d.free),
				dot(d.reserved),
			);
		// Child sovereigns migrate (translated to sibl); pre-migration they are not leftovers,
		// so only the post-migration mode prints the ones that stayed (held back / anomalies).
		} else if bytes.starts_with(b"sibl") || (!only_referenced && bytes.starts_with(b"para")) {
			let kind = if bytes.starts_with(b"sibl") { "sibl" } else { "para (child)" };
			let para = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
			println!(
				"{kind} sovereign of para {para}: {} | free {:.4} reserved {:.4}",
				ss58(&who),
				dot(d.free),
				dot(d.reserved),
			);
		} else if total < ed {
			dust_n += 1;
			dust_amt += total;
			let blocker = if !pallet_balances::Holds::<Rc>::get(&who).is_empty() {
				"holds"
			} else if info.consumers != 0 {
				"consumer refs"
			} else if d.reserved != 0 {
				"reserved only"
			} else if d.free == 0 {
				"zero balance (provider-ref husk)"
			} else {
				"none (sweep reaps it)"
			};
			let e = dust_blockers.entry(blocker).or_default();
			e.0 += 1;
			e.1 += total;
		} else {
			// Extra consumer refs (beyond the one a reserve accounts for) mean some pallet
			// still references the account and withdrawal would fail — session keys being the
			// known case.
			let expected = u32::from(d.reserved > 0);
			let referenced = info.consumers > expected;
			if referenced || !only_referenced {
				let keys = pallet_session::NextKeys::<Rc>::get(&who).is_some();
				println!(
					"{}: {} | free {:.4} reserved {:.4} consumers {} session-keys {}",
					if referenced { "referenced" } else { "held back" },
					ss58(&who),
					dot(d.free),
					dot(d.reserved),
					info.consumers,
					keys,
				);
			}
		}
	}
	println!("below-ED dust: {dust_n} accounts, {:.4} {TOKEN}", dot(dust_amt));
	for (blocker, (n, amt)) in &dust_blockers {
		println!("  dust blocked by {blocker}: {n} accounts, {:.4} {TOKEN}", dot(*amt));
	}
	println!("all accounts on the RC: {accounts}, {:.4} {TOKEN}", dot(total_sum));
}

/// Find a parachain manager that migrates cleanly: a live registrar deposit that fully accounts
/// for the account's reserve, a signing key (so the buffer split applies, not the pure-proxy
/// routing), and nothing else attached. Returns `(manager, free, reserved)`.
/// Must run inside the RC externalities.
fn find_clean_manager() -> (AccountId32, u128, u128) {
	type Rc = crate::mock::network::relay::Runtime;
	let mut recorded = BTreeMap::<AccountId32, u128>::new();
	for (_, info) in paras_registrar::Paras::<Rc>::iter() {
		*recorded.entry(info.manager).or_default() += info.deposit;
	}
	paras_registrar::Paras::<Rc>::iter()
		.find_map(|(_, info)| {
			let account = frame_system::Account::<Rc>::get(&info.manager);
			(account.nonce > 0 &&
				account.data.reserved > 0 &&
				account.data.reserved <= *recorded.get(&info.manager).unwrap_or(&0) &&
				account.data.frozen == 0 &&
				pallet_balances::Holds::<Rc>::get(&info.manager).is_empty() &&
				pallet_balances::Locks::<Rc>::get(&info.manager).is_empty())
			.then(|| (info.manager, account.data.free, account.data.reserved))
		})
		.expect("live RC snapshot has a cleanly migrating parachain manager")
}

// Multi-thread runtimes so snapshot loading/hydration actually runs in parallel; the default
// `#[tokio::test]` runtime is current-thread and would serialize the CPU-bound loads.
#[tokio::test(flavor = "multi_thread")]
async fn three_chains_produce_blocks() {
	let (mut rc, mut ah, mut ct) = load_externalities().await;
	// 10 blocks is enough for the message queues to drain whatever the live snapshot carries;
	// `next_block_*` asserts on every block that nothing fails processing and that the weight
	// stays under 80% of the block limit.
	rc.execute_with(|| {
		for _ in 0..10 {
			next_block_rc();
		}
	});
	ah.execute_with(|| {
		for _ in 0..10 {
			next_block_para::<AssetHubPara>();
		}
	});
	ct.execute_with(|| {
		for _ in 0..10 {
			next_block_para::<CoretimePara>();
		}
	});
}

#[tokio::test(flavor = "multi_thread")]
async fn rc_and_asset_hub_exchange_messages() {
	message_round_trip::<AssetHubPara>().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn rc_and_coretime_exchange_messages() {
	message_round_trip::<CoretimePara>().await;
}

/// Assert that a `System::Remarked` event was emitted on runtime `T`.
fn assert_remarked<T: frame_system::Config>(chain: &str)
where
	T::RuntimeEvent: TryInto<frame_system::Event<T>>,
{
	assert!(
		frame_system::Pallet::<T>::events().into_iter().any(|record| matches!(
			record.event.try_into(),
			Ok(frame_system::Event::<T>::Remarked { .. })
		)),
		"remark did not execute on {chain}"
	);
}

/// Sends a `System::remark_with_event` from the RC to `P` and back, asserting on the destination
/// that the remark actually executed.
async fn message_round_trip<P: Para>()
where
	RuntimeCallFor<P>: From<frame_system::Call<P::Runtime>>,
{
	let (mut rc, mut para) = tokio::join!(load(Chain::Relay), load(P::CHAIN));

	// RC -> para.
	let dmp = rc.execute_with(|| {
		let call: RuntimeCallFor<P> = frame_system::Call::<P::Runtime>::remark_with_event {
			remark: b"minimal-relay dmp".to_vec(),
		}
		.into();
		send_dmp(P::PARA_ID.into(), unpaid_transact(call));
		next_block_rc();
		take_dmp(P::PARA_ID.into())
	});
	rc.commit_all().unwrap();
	// The live snapshot may have queued unrelated messages for this para, so only assert that
	// ours is among them.
	assert!(!dmp.is_empty(), "RC queued no DMP message for {}", P::NAME);

	para.execute_with(|| {
		enqueue_dmp::<P>(dmp);
		next_block_para::<P>();
		assert_remarked::<P::Runtime>(P::NAME);
	});
	para.commit_all().unwrap();

	// para -> RC.
	let ump = para.execute_with(|| {
		let call: crate::mock::network::relay::RuntimeCall =
			frame_system::Call::remark_with_event { remark: b"minimal-relay ump".to_vec() }.into();
		send_ump::<P>(unpaid_transact(call));
		take_ump::<P>()
	});
	para.commit_all().unwrap();
	assert!(!ump.is_empty(), "{} queued no UMP message for the RC", P::NAME);

	rc.execute_with(|| {
		enqueue_ump(P::PARA_ID.into(), ump);
		next_block_rc();
		assert_remarked::<crate::mock::network::relay::Runtime>("the RC");
	});
}

/// One row of the balance census: how many accounts have this kind of balance, and how much.
#[derive(Default)]
struct CensusRow {
	accounts: u32,
	total: u128,
}

impl CensusRow {
	fn add(&mut self, amount: u128) {
		self.accounts += 1;
		self.total += amount;
	}
}

/// Aggregate and print every kind of balance on the chain: free, holds per reason, named and
/// unnamed reserves, locks per id and freezes per id. Reporting only, no assertions — this is
/// the input for deciding what the migration must do with each class of balance.
fn print_balance_census<T>(name: &str)
where
	T: frame_system::Config<AccountData = pallet_balances::AccountData<u128>>
		+ pallet_balances::Config<Balance = u128>,
	<T as pallet_balances::Config>::RuntimeHoldReason: std::fmt::Debug,
	<T as pallet_balances::Config>::FreezeIdentifier: std::fmt::Debug,
	<T as pallet_balances::Config>::ReserveIdentifier: std::fmt::Debug,
{
	let mut accounts = 0u32;
	let (mut free, mut reserved, mut frozen, mut unnamed_reserve) =
		(CensusRow::default(), CensusRow::default(), CensusRow::default(), CensusRow::default());
	let mut holds = BTreeMap::<String, CensusRow>::new();
	let mut named_reserves = BTreeMap::<String, CensusRow>::new();
	let mut locks = BTreeMap::<String, CensusRow>::new();
	let mut freezes = BTreeMap::<String, CensusRow>::new();

	for (who, info) in frame_system::Account::<T>::iter() {
		accounts += 1;
		let data = info.data;
		if data.free > 0 {
			free.add(data.free);
		}
		if data.frozen > 0 {
			frozen.add(data.frozen);
		}

		// Reserved splits into holds (attributed in balances itself), named reserves (id but no
		// reason) and the unnamed remainder (old `reserve()` API — attributable only by
		// cross-referencing the pallets that placed the deposits).
		let mut attributed = 0u128;
		for hold in pallet_balances::Holds::<T>::get(&who) {
			holds.entry(format!("hold {:?}", hold.id)).or_default().add(hold.amount);
			attributed += hold.amount;
		}
		for reserve in pallet_balances::Reserves::<T>::get(&who) {
			named_reserves
				.entry(format!("named reserve {:?}", reserve.id))
				.or_default()
				.add(reserve.amount);
			attributed += reserve.amount;
		}
		if data.reserved > 0 {
			reserved.add(data.reserved);
			let unnamed = data.reserved.saturating_sub(attributed);
			if unnamed > 0 {
				unnamed_reserve.add(unnamed);
			}
		}

		for lock in pallet_balances::Locks::<T>::get(&who) {
			locks
				.entry(format!("lock {:?}", String::from_utf8_lossy(&lock.id)))
				.or_default()
				.add(lock.amount);
		}
		for freeze in pallet_balances::Freezes::<T>::get(&who) {
			freezes.entry(format!("freeze {:?}", freeze.id)).or_default().add(freeze.amount);
		}
	}

	let row = |label: &str, r: &CensusRow| {
		println!("| {label} | {} | {:.4} |", r.accounts, dot(r.total));
	};
	println!("### Balance census: {name}");
	println!();
	println!(
		"{accounts} accounts, total issuance {:.4} {TOKEN} (inactive: {:.4})",
		dot(pallet_balances::TotalIssuance::<T>::get()),
		dot(pallet_balances::InactiveIssuance::<T>::get()),
	);
	println!();
	println!("| Balance kind | Accounts | {TOKEN} |");
	println!("|---|---:|---:|");
	row("free", &free);
	row("reserved (holds + named + unnamed)", &reserved);
	for (label, r) in holds.iter().chain(named_reserves.iter()) {
		row(label, r);
	}
	row("unnamed reserve", &unnamed_reserve);
	// `frozen` is the max of overlapping locks/freezes per account, so rows are not additive.
	row("frozen (not additive with rows below)", &frozen);
	for (label, r) in locks.iter().chain(freezes.iter()) {
		row(label, r);
	}

	// Issuance that no account holds: free + reserved across all accounts versus TotalIssuance.
	// The exact planck value is the source of truth for `TiCorrection` in the RC runtime config.
	let in_accounts = free.total + reserved.total;
	let phantom = pallet_balances::TotalIssuance::<T>::get().saturating_sub(in_accounts);
	println!("| issuance not held by any account | — | {:.4} |", dot(phantom));
	println!("phantom issuance, exact planck: {phantom}");
}

/// Print the balance census of the Relay Chain snapshot. Run with `--nocapture` to see it.
#[tokio::test(flavor = "multi_thread")]
async fn balance_census() {
	let mut rc = remote_ext(Chain::Relay).await;
	rc.execute_with(|| {
		print_balance_census::<crate::mock::network::relay::Runtime>("Polkadot Relay Chain");

		// Decompose which accounts the accounts stage keeps on the RC, with amounts — the
		// direct answer to "where does the kept balance sit".
		{
			type Rc = crate::mock::network::relay::Runtime;
			let ed = pallet_balances::Pallet::<Rc>::minimum_balance();
			let mut cats: BTreeMap<&str, (u32, u128)> = BTreeMap::new();
			for (who, info) in frame_system::Account::<Rc>::iter() {
				let d = &info.data;
				let total = d.free + d.reserved;
				let bytes: &[u8] = who.as_ref();
				let cat = if bytes.starts_with(b"para") {
					"para sovereign"
				} else if bytes.starts_with(b"sibl") {
					"sibl sovereign"
				} else if bytes.starts_with(b"modl") {
					"module account"
				} else if total < ed {
					"below-ED"
				} else if d.frozen > 0 ||
					!pallet_balances::Locks::<Rc>::get(&who).is_empty() ||
					!pallet_balances::Freezes::<Rc>::get(&who).is_empty() ||
					!pallet_balances::Holds::<Rc>::get(&who).is_empty()
				{
					"locks/freezes/holds"
				} else {
					continue
				};
				let e = cats.entry(cat).or_default();
				e.0 += 1;
				e.1 += total;
			}
			println!("\n### kept-account decomposition (accounts the migrator skips)");
			let mut sum = 0u128;
			for (cat, (n, amt)) in &cats {
				println!("{cat}: {n} accounts, {:.4} {TOKEN}", dot(*amt));
				sum += amt;
			}
			println!("skipped-account total: {:.4} {TOKEN}", dot(sum));
		}

		// Registrar reconciliation census: for every manager, compare the recorded deposits
		// against the live reserve. Classifies the deposit-shortfall causes: `zero`/`partial`
		// are genuine on-chain anomalies (reserve reduced out-of-band, record never updated);
		// `over` are accounts with additional unattributable reserves (proxy deposits) that the
		// migration holds back whole.
		{
			type Rc = crate::mock::network::relay::Runtime;
			let mut recorded = BTreeMap::<AccountId32, u128>::new();
			let mut paras_of = BTreeMap::<AccountId32, Vec<(u32, u128, Option<bool>)>>::new();
			for (id, info) in paras_registrar::Paras::<Rc>::iter() {
				*recorded.entry(info.manager.clone()).or_default() += info.deposit;
				paras_of.entry(info.manager).or_default().push((
					id.into(),
					info.deposit,
					info.locked,
				));
			}
			let mut cls = BTreeMap::<&str, (u32, u32, u128)>::new(); // managers, paras, recorded
			println!("\n### registrar deposit anomalies (recorded vs live reserve, per manager)");
			for (manager, expected) in &recorded {
				let reserved = frame_system::Account::<Rc>::get(manager).data.reserved;
				let cat = if reserved == *expected {
					"exact"
				} else if reserved == 0 {
					"zero reserve (anomaly)"
				} else if reserved < *expected {
					"partial reserve (anomaly)"
				} else {
					"over-reserved (held back: proxy etc.)"
				};
				if cat != "exact" {
					println!(
						"{cat}: {} | paras {:?} | recorded {:.4} {TOKEN}, live reserve {:.4} {TOKEN}, gap {:.4} {TOKEN}",
						ss58(manager),
						paras_of[manager],
						dot(*expected),
						dot(reserved),
						dot(expected.saturating_sub(reserved)),
					);
				}
				let e = cls.entry(cat).or_default();
				e.0 += 1;
				e.1 += paras_of[manager].len() as u32;
				e.2 += expected;
			}
			println!("\n### registrar reconciliation summary");
			for (cat, (managers, paras, dep)) in &cls {
				println!(
					"{cat}: {managers} managers / {paras} paras, recorded {:.4} {TOKEN}",
					dot(*dep)
				);
			}

			// HRMP: per (child) sovereign, channel deposits vs live reserve — including pending
			// open-channel-request deposits, which are reserved but attached to no channel yet.
			let mut channel_dep = BTreeMap::<AccountId32, (u128, Vec<u32>)>::new();
			for (id, ch) in HrmpChannels::<Rc>::iter() {
				let s = channel_dep
					.entry(id.sender.into_account_truncating())
					.or_insert((0, vec![u32::from(id.sender)]));
				s.0 += ch.sender_deposit;
				let r = channel_dep
					.entry(id.recipient.into_account_truncating())
					.or_insert((0, vec![u32::from(id.recipient)]));
				r.0 += ch.recipient_deposit;
			}
			let mut request_dep = BTreeMap::<AccountId32, u128>::new();
			for (id, req) in runtime_parachains::hrmp::HrmpOpenChannelRequests::<Rc>::iter() {
				*request_dep.entry(id.sender.into_account_truncating()).or_default() +=
					req.sender_deposit;
			}
			println!(
				"\n### hrmp sovereign reconciliation (channel + request deposits vs live reserve)"
			);
			let (mut exact, mut short, mut over) = (0u32, 0u32, 0u32);
			for (sov, (chan, paras)) in &channel_dep {
				let requests = request_dep.get(sov).copied().unwrap_or(0);
				let reserved = frame_system::Account::<Rc>::get(sov).data.reserved;
				let full = chan + requests;
				if reserved == full {
					exact += 1;
				} else if reserved < full {
					short += 1;
					println!(
						"short: para {:?} | channels {:.4} + requests {:.4} {TOKEN} recorded, live reserve {:.4} {TOKEN}",
						paras, dot(*chan), dot(requests), dot(reserved),
					);
				} else {
					over += 1;
					println!(
						"over: para {:?} | channels {:.4} + requests {:.4} {TOKEN} recorded, live reserve {:.4} {TOKEN}",
						paras, dot(*chan), dot(requests), dot(reserved),
					);
				}
			}
			println!(
				"hrmp sovereigns: {exact} exact, {short} short (anomaly), {over} over-reserved"
			);
		}

		// Who would remain on the RC after the migration and why — the "RC → 0" gap list.
		{
			println!("\n### remaining-on-RC gap list (accounts the migration cannot move)");
			print_remaining_on_rc(true);
		}

		// The Balances pallet's own storage keys: how many are empty leftovers, and how many
		// belong to accounts that no longer exist (v1 reaped the account, the key survived).
		{
			type Rc = crate::mock::network::relay::Runtime;
			let report = |name: &str, entries: Vec<(AccountId32, bool)>| {
				let n = entries.len();
				let empty = entries.iter().filter(|(_, e)| *e).count();
				let orphan = entries
					.iter()
					.filter(|(who, _)| !frame_system::Account::<Rc>::contains_key(who))
					.count();
				println!(
					"{name}: {n} keys, {empty} empty-value, {orphan} for nonexistent accounts"
				);
			};
			report(
				"Balances::Locks",
				pallet_balances::Locks::<Rc>::iter().map(|(w, v)| (w, v.is_empty())).collect(),
			);
			report(
				"Balances::Reserves",
				pallet_balances::Reserves::<Rc>::iter()
					.map(|(w, v)| (w, v.is_empty()))
					.collect(),
			);
			report(
				"Balances::Freezes",
				pallet_balances::Freezes::<Rc>::iter().map(|(w, v)| (w, v.is_empty())).collect(),
			);
			report(
				"Balances::Holds",
				pallet_balances::Holds::<Rc>::iter().map(|(w, v)| (w, v.is_empty())).collect(),
			);
		}

		// Unclaimed pre-genesis claims are part of total issuance but sit in no account — the
		// prime suspect for the "not held by any account" row above.
		let unclaimed = polkadot_runtime_common::claims::Total::<crate::mock::network::relay::Runtime>::get();
		println!();
		println!(
			"claims::Total (unclaimed pre-genesis claims): {:.4} {TOKEN}",
			unclaimed as f64 / 1e10
		);

		// AHM v1's final balance bookkeeping, for provenance of the issuance numbers. Read via
		// raw key to avoid depending on pallet-rc-migrator just for one probe.
		let key = [
			sp_io::hashing::twox_128(b"RcMigrator"),
			sp_io::hashing::twox_128(b"RcMigratedBalanceArchive"),
		]
		.concat();
		if let Some(raw) = frame_support::storage::unhashed::get_raw(&key) {
			if let Ok((kept, migrated)) = <(u128, u128)>::decode(&mut &raw[..]) {
				println!(
					"v1 RcMigratedBalanceArchive: kept {:.4} {TOKEN}, migrated {:.4} {TOKEN}",
					kept as f64 / 1e10,
					migrated as f64 / 1e10,
				);
			}
		} else {
			println!("v1 RcMigratedBalanceArchive: not found");
		}

		// The XCM teleport checking account (tracked teleported-out DOT before v1 moved that
		// role to Asset Hub).
		let check = pallet_xcm::Pallet::<crate::mock::network::relay::Runtime>::check_account();
		let check_data = frame_system::Account::<crate::mock::network::relay::Runtime>::get(&check).data;
		println!(
			"XCM checking account {check:?}: free {:.4} {TOKEN}, reserved {:.4} {TOKEN}",
			check_data.free as f64 / 1e10,
			check_data.reserved as f64 / 1e10,
		);
	});
}

/// Dump every proxy entry on the Relay Chain as JSON lines, cross-referenced against registrar
/// managers and para sovereigns — the data source for the proxy migration report
/// (`ahm-phase-2/simulation/proxy-report.html`).
///
/// Writes to the file named by `AHM_PROXY_DUMP`; without it, prints only aggregates.
#[tokio::test(flavor = "multi_thread")]
async fn proxy_census() {
	type Rc = crate::mock::network::relay::Runtime;

	let mut rc = remote_ext(Chain::Relay).await;
	rc.execute_with(|| {
		// Registrar managers and what they manage.
		let mut manages = BTreeMap::<AccountId32, Vec<u32>>::new();
		for (id, info) in paras_registrar::Paras::<Rc>::iter() {
			manages.entry(info.manager).or_default().push(id.into());
		}

		let mut out = Vec::new();
		let (mut entries, mut deposit_total) = (0u32, 0u128);
		for (who, (defs, deposit)) in pallet_proxy::Proxies::<Rc>::iter() {
			entries += 1;
			deposit_total += deposit;
			let account = frame_system::Account::<Rc>::get(&who);
			let bytes: &[u8] = who.as_ref();
			let sovereign = bytes.starts_with(b"para") || bytes.starts_with(b"sibl");
			let delegates: Vec<String> = defs
				.iter()
				.map(|d| {
					format!(
						r#"{{"delegate":"{}","type":"{:?}","delay":{},"delegate_manages":{:?}}}"#,
						ss58(&d.delegate),
						d.proxy_type,
						d.delay,
						manages.get(&d.delegate).cloned().unwrap_or_default(),
					)
				})
				.collect();
			// `consumers` decides the delegator's fate in the migration: withdrawal fails when
			// consumer references remain after the reserve is released (session references
			// being the known case), keeping the entry on the RC. Session keys themselves are
			// never touched either way — they are live session state managed from AH via XCM,
			// and a zero-balance key-holder is the intended end state.
			out.push(format!(
				r#"{{"who":"{}","deposit":"{}","free":"{}","reserved":"{}","nonce":{},"consumers":{},"sovereign":{},"session_keys":{},"exists":{},"manages":{:?},"delegates":[{}]}}"#,
				ss58(&who),
				deposit,
				account.data.free,
				account.data.reserved,
				account.nonce,
				account.consumers,
				sovereign,
				pallet_session::NextKeys::<Rc>::get(&who).is_some(),
				frame_system::Account::<Rc>::contains_key(&who),
				manages.get(&who).cloned().unwrap_or_default(),
				delegates.join(","),
			));
		}
		println!("proxy census: {entries} delegators, {:.2} {TOKEN} deposits", deposit_total as f64 / 1e10);
		if let Ok(path) = std::env::var("AHM_PROXY_DUMP") {
			std::fs::write(&path, out.join("\n")).expect("can write proxy dump");
			println!("wrote {path}");
		}
	});
}

/// Account balances migrate RC -> Coretime.
///
/// Drives the accounts stage of `pallet-rc2-migrator` to completion against the real snapshots,
/// shuttles the resulting DMP messages to the Coretime chain and verifies balance conservation on
/// both ends plus the exact balances of one named account: a parachain manager with a live
/// registrar deposit.
///
/// On the relay chain that deposit is an anonymous reserve (old `Currency::reserve` API, no
/// record of why). The migration must not recreate anonymous money: the amount has to arrive on
/// Coretime as a hold under an explicit reason — `CtMigrator(RcMigratedReserve)` until the pallet
/// owning the deposit migrates and re-attributes it — which is what the hold assertions below
/// pin down.
#[tokio::test(flavor = "multi_thread")]
async fn accounts_migrate_rc_to_ct() {
	type Rc = crate::mock::network::relay::Runtime;
	type Ct = crate::mock::network::ct::Runtime;
	type RcStage = pallet_rc2_migrator::MigrationStageOf<Rc>;

	let (mut rc, mut ct) = tokio::join!(load(Chain::Relay), load(Chain::Coretime));

	// GIVEN a parachain manager holding a live registrar deposit (reserved balance) on the RC.
	let (manager, rc_free, rc_reserved, rc_issuance_before) = rc.execute_with(|| {
		let (manager, free, reserved) = find_clean_manager();
		(manager, free, reserved, pallet_balances::TotalIssuance::<Rc>::get())
	});
	// What the split rule should do with this manager.
	let (ct_free_exp, _ah_free_exp) = expected_split(rc_free, rc_reserved);

	// WHEN the accounts stage runs to completion.
	//
	// It has to be shuttled rather than run to the end on the relay chain alone: each batch is
	// only confirmed once the Coretime chain has integrated it and its `ReportTransactStatus`
	// answer has travelled back, and the stage machine holds until then.
	let ct_account_before = ct.execute_with(|| {
		assert!(
			pallet_balances::Holds::<Ct>::get(&manager).is_empty(),
			"manager unexpectedly already has holds on Coretime"
		);
		frame_system::Account::<Ct>::get(&manager)
	});
	let ct_issuance_before = ct.execute_with(pallet_balances::TotalIssuance::<Ct>::get);

	rc.execute_with(|| {
		// Written directly: `force_set_stage` needs a paused, running machine, and this one has
		// not been scheduled.
		pallet_rc2_migrator::RcMigrationStage::<Rc>::put(RcStage::AccountsInit);
	});
	rc.commit_all().unwrap();

	let mut ct_ump: Vec<polkadot_primitives::UpwardMessage> = Vec::new();
	let mut sent_any = false;
	let mut rounds = 0;
	let migrated = loop {
		rounds += 1;
		assert!(rounds <= 20, "accounts stage must finish within 20 shuttle rounds");

		let (dmp, stage, migrated) = rc.execute_with(|| {
			enqueue_ump(CoretimePara::PARA_ID.into(), core::mem::take(&mut ct_ump));
			for _ in 0..3 {
				next_block_rc();
			}
			(
				take_dmp(CoretimePara::PARA_ID.into()),
				RcMigrationStage::<Rc>::get(),
				RcMigratedBalance::<Rc>::get(),
			)
		});
		rc.commit_all().unwrap();
		sent_any |= !dmp.is_empty();

		ct.execute_with(|| {
			enqueue_dmp::<CoretimePara>(dmp);
			for _ in 0..3 {
				next_block_para::<CoretimePara>();
			}
			ct_ump = take_ump::<CoretimePara>();
		});
		ct.commit_all().unwrap();

		// `AccountsDone` is a one-block checkpoint and this loop samples every third block, so
		// wait for the machine to have left the accounts stages rather than for that exact stage.
		if !matches!(stage, RcStage::AccountsInit | RcStage::AccountsOngoing { .. }) {
			break migrated;
		}
	};
	assert!(sent_any, "the accounts stage queued no DMP messages for Coretime");

	// Deliver the final round's reports. Every batch the *accounts* stage sent must be confirmed
	// by now — the relay chain burned what each one carried, so an unconfirmed accounts batch is
	// balance that went nowhere. Later stages are already sending by this point (the machine does
	// not stop at `AccountsDone`), so only the accounts batches are in scope here.
	rc.execute_with(|| {
		enqueue_ump(CoretimePara::PARA_ID.into(), core::mem::take(&mut ct_ump));
		for _ in 0..3 {
			next_block_rc();
		}
		let unconfirmed_accounts: Vec<_> = pallet_rc2_migrator::UnconfirmedBatches::<Rc>::iter()
			.filter(|(_, batch)| {
				matches!(
					batch.stage,
					RcStage::AccountsInit | RcStage::AccountsOngoing { .. } | RcStage::AccountsDone
				)
			})
			.map(|(query_id, _)| query_id)
			.collect();
		assert!(
			unconfirmed_accounts.is_empty(),
			"the Coretime chain never confirmed accounts batches {unconfirmed_accounts:?}",
		);
	});
	rc.commit_all().unwrap();

	// THEN the manager is reaped on the RC and issuance dropped by exactly what migrated.
	let total_migrated = migrated.ct_reserved + migrated.ct_free + migrated.ah_free;
	rc.execute_with(|| {
		assert!(
			!frame_system::Account::<Rc>::contains_key(&manager),
			"the manager account must be reaped on the RC"
		);
		let rc_issuance = pallet_balances::TotalIssuance::<Rc>::get();
		assert_eq!(rc_issuance, rc_issuance_before - total_migrated);
		assert_eq!(rc_issuance, migrated.kept);
	});

	// AND the Coretime chain integrates every CT-bound piece: the registrar deposit arrives as a
	// hold, the working buffer arrives free, and issuance grows by exactly the CT-bound burn.
	// (The teleported remainder is asserted end-to-end in `full_migration_rc_to_ct`.)
	ct.execute_with(|| {
		assert_eq!(
			pallet_ct_migrator::CtMigrationStage::<Ct>::get(),
			pallet_ct_migrator::MigrationStage::DataMigrationOngoing,
		);

		let ct_account = frame_system::Account::<Ct>::get(&manager);
		assert_eq!(ct_account.data.free, ct_account_before.data.free + ct_free_exp);
		assert_eq!(ct_account.data.reserved, ct_account_before.data.reserved + rc_reserved);

		let holds = pallet_balances::Holds::<Ct>::get(&manager);
		assert_eq!(holds.len(), 1, "the registrar deposit must arrive as exactly one hold");
		assert_eq!(
			holds[0].id,
			crate::mock::network::ct::RuntimeHoldReason::CtMigrator(
				pallet_ct_migrator::HoldReason::RcMigratedReserve
			)
		);
		assert_eq!(holds[0].amount, rc_reserved);

		assert_eq!(
			pallet_balances::TotalIssuance::<Ct>::get(),
			ct_issuance_before + migrated.ct_reserved + migrated.ct_free,
			"Coretime must mint exactly the CT-bound burn"
		);
	});
}

/// The full migration pipeline RC -> CT: accounts, registrar, HRMP, cool-off, done.
///
/// The lockdown windows this suite schedules with: long enough to be visible as their own stages,
/// short enough not to pad every run.
const WARM_UP: u32 = 5;
const COOL_OFF: u32 = 5;

/// Drives `pallet-rc2-migrator` from `Scheduled` to `MigrationDone` against the real snapshots,
/// shuttling DMP after every burst of RC blocks, and asserts the end state on both chains:
/// registrar and HRMP state drained from the RC and landed on Coretime, deposits re-attributed
/// to `RegistrarDeposit` holds under the `min(recorded, held)` rule, and balance conservation
/// exact.
///
/// With `AHM_EVENTS=<path>` set, the run also writes the JSONL event stream that the migration
/// monitor frontend replays.
#[tokio::test(flavor = "multi_thread")]
async fn full_migration_rc_to_ct() {
	type Rc = crate::mock::network::relay::Runtime;
	type Ct = crate::mock::network::ct::Runtime;
	type RcStage = pallet_rc2_migrator::MigrationStageOf<Rc>;
	type Ah = crate::mock::network::ah::Runtime;

	let (mut rc, mut ct, mut ah) =
		tokio::join!(load(Chain::Relay), load(Chain::Coretime), load(Chain::AssetHub));

	// GIVEN the live registrar and HRMP state of the snapshot, and one cleanly migrating
	// manager to spot-check the AH teleport leg with.
	// `paras_before_detail` and `requests_before_detail` carry the facts only this chain can
	// see — the lifecycle that tells Reserved from Registered, the lock, and whether a request
	// was confirmed. Coretime cannot rederive any of them, so they are what the post-migration
	// assertions compare against.
	let (
		paras_before,
		paras_before_detail,
		hrmp_before,
		requests_before,
		requests_before_detail,
		rc_ti_before,
		sample,
	) = rc.execute_with(|| {
		crate::events::emit_rc_census("before");
		crate::events::emit_pre_facts();
		let paras: Vec<(u32, u128)> = paras_registrar::Paras::<Rc>::iter()
			.map(|(id, info)| (id.into(), info.deposit))
			.collect();
		// The lock travels whole, not collapsed: Coretime has to tell "never locked"
		// (still eligible for its own automatic lock) from "unlocked on purpose".
		let detail: Vec<(u32, bool, Option<bool>)> = paras_registrar::Paras::<Rc>::iter()
			.map(|(id, info)| {
				(
					id.into(),
					runtime_parachains::paras::Pallet::<Rc>::lifecycle(id).is_some(),
					info.locked,
				)
			})
			.collect();
		let channels: Vec<(_, u128, u128)> = HrmpChannels::<Rc>::iter()
			.map(|(id, ch)| (id, ch.sender_deposit, ch.recipient_deposit))
			.collect();
		let requests: Vec<u128> = runtime_parachains::hrmp::HrmpOpenChannelRequests::<Rc>::iter()
			.map(|(_, r)| r.sender_deposit)
			.collect();
		let requests_detail: Vec<(u32, u32, bool)> =
			runtime_parachains::hrmp::HrmpOpenChannelRequests::<Rc>::iter()
				.map(|(id, r)| (id.sender.into(), id.recipient.into(), r.confirmed))
				.collect();

		(
			paras,
			detail,
			channels,
			requests,
			requests_detail,
			pallet_balances::TotalIssuance::<Rc>::get(),
			find_clean_manager(),
		)
	});

	// Pre-migration sanity: the snapshot must actually contain the shapes this test claims to
	// exercise, or a green run would prove nothing.
	assert!(
		paras_before_detail.iter().any(|(_, registered, _)| *registered),
		"the snapshot must contain at least one onboarded para"
	);
	assert!(
		paras_before_detail.iter().any(|(_, _, locked)| *locked == Some(true)),
		"the snapshot must contain at least one locked para, or the lock carry is untested"
	);
	assert!(
		paras_before_detail.iter().any(|(_, _, locked)| locked.is_none()),
		"the snapshot must contain at least one never-locked para, or the None carry is untested"
	);

	// The sweep stage's inputs — the configured pots plus reapable below-ED dust, all landing
	// on the sweep beneficiary — and the sibl-format sovereigns, whose balances migrate
	// untranslated to the same bytes on AH.
	let (treasury, sweep_pots, sweep_dust, sibl_before) = rc.execute_with(|| {
		let treasury: AccountId32 =
			crate::mock::network::relay::TreasuryPalletId::get().into_account_truncating();
		let pots: Vec<(AccountId32, u128)> = crate::mock::network::relay::SweepAccounts::get()
			.into_iter()
			.map(|who| {
				let amount = frame_system::Account::<Rc>::get(&who).data.free;
				(who, amount)
			})
			.collect();
		let ed = pallet_balances::Pallet::<Rc>::minimum_balance();
		let mut dust = 0u128;
		for (_, info) in frame_system::Account::<Rc>::iter() {
			let total = info.data.free + info.data.reserved;
			if total > 0 && total < ed {
				dust += total;
			}
		}
		// A sibl account's expected AH arrival includes its para's CHILD sovereign: the child
		// migrates translated to the same sibl bytes, its AH-bound free landing on one address.
		let sibl: Vec<(AccountId32, u128)> = frame_system::Account::<Rc>::iter()
			.filter(|(who, _)| AsRef::<[u8]>::as_ref(who).starts_with(b"sibl"))
			.map(|(who, info)| {
				let bytes: &[u8] = who.as_ref();
				let para = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
				let child: AccountId32 =
					polkadot_primitives::Id::from(para).into_account_truncating();
				let child_data = frame_system::Account::<Rc>::get(&child).data;
				let (_, child_ah) = expected_split(child_data.free, child_data.reserved);
				(who, info.data.free + info.data.reserved + child_ah)
			})
			.collect();
		(treasury, pots, dust, sibl)
	});
	assert!(sweep_pots.iter().any(|(_, a)| *a > 0), "the snapshot has pots to sweep");
	assert!(!sibl_before.is_empty(), "the snapshot has sibl-format sovereigns");
	assert!(!paras_before.is_empty(), "live RC snapshot has registered paras");
	assert!(!hrmp_before.is_empty(), "live RC snapshot has HRMP channels");
	let recorded_deposits: u128 = paras_before.iter().map(|(_, d)| d).sum();
	let recorded_hrmp: u128 = hrmp_before.iter().map(|(_, s, r)| s + r).sum::<u128>() +
		requests_before.iter().sum::<u128>();

	// Proxy state before: the delegators with portable definitions (which travel to CT), one
	// delegator with both an Any and a ParaRegistration delegate for the dispatch checks, and
	// every never-signed delegator for the funds-follow-control invariant.
	let (ct_bound_proxies, nonce0_delegators, proxy_dispatch) = rc.execute_with(|| {
		// The portable type a definition translates to, if the CT chain represents it.
		let portable = |t: &<Rc as pallet_proxy::Config>::ProxyType| {
			PortableProxyType::try_from(t.clone()).ok()
		};
		let mut ct_bound = Vec::new();
		let mut nonce0 = Vec::new();
		let mut dispatch = None;
		for (who, (defs, _)) in pallet_proxy::Proxies::<Rc>::iter() {
			let account = frame_system::Account::<Rc>::get(&who);
			let had_any =
				defs.iter().any(|d| portable(&d.proxy_type) == Some(PortableProxyType::Any));
			if account.nonce == 0 {
				// A never-signed delegator. With an `Any` def it is (or must be treated as) a
				// keyless pure whose whole balance follows the recreated definitions to CT. A
				// nonce-0 delegator WITHOUT `Any` defs necessarily created its entry via
				// `as_multi` — no other dispatch path exists at nonce 0 with no prior defs and
				// no post-v1 `create_pure` (verified by events and the spawner-reserve
				// signature) — so it is a multisig, controllable on every chain by its members.
				let existed = frame_system::Account::<Rc>::contains_key(&who);
				nonce0.push((
					who.clone(),
					had_any,
					existed,
					account.data.free + account.data.reserved,
				));
			}
			if defs.iter().any(|d| portable(&d.proxy_type).is_some()) {
				ct_bound.push(who.clone());
			}
			let any = defs.iter().find(|d| portable(&d.proxy_type) == Some(PortableProxyType::Any));
			let reg = defs
				.iter()
				.find(|d| portable(&d.proxy_type) == Some(PortableProxyType::ParaRegistration));
			if let (Some(any), Some(reg), None) = (any, reg, dispatch.as_ref()) {
				dispatch = Some((who.clone(), any.delegate.clone(), reg.delegate.clone()));
			}
		}
		// Not every network has one: `ParaRegistration` proxies exist on Polkadot but nobody has
		// created one on Kusama. Absence is allowed, but it has to be *because* the chain has
		// none — not because the lookup above is broken — so prove that before letting the
		// dispatch checks be skipped.
		if dispatch.is_none() {
			let para_reg = pallet_proxy::Proxies::<Rc>::iter()
				.flat_map(|(_, (defs, _))| defs.into_inner())
				.filter(|d| portable(&d.proxy_type) == Some(PortableProxyType::ParaRegistration))
				.count();
			assert_eq!(
				para_reg, 0,
				"the chain has {para_reg} ParaRegistration proxies but none paired with an Any \
				 delegate — the dispatch fixture is broken, not absent"
			);
		}
		(ct_bound, nonce0, dispatch)
	});
	assert!(!ct_bound_proxies.is_empty(), "live snapshot has portable proxies");

	let (ct_ti_before, ct_pures_before) = ct.execute_with(|| {
		crate::events::emit_ct_census("before");
		// Pre-migration CT balances of the possible pures, so their arrival asserts exactly.
		let pures: BTreeMap<AccountId32, u128> = nonce0_delegators
			.iter()
			.filter(|(_, had_any, existed, _)| *had_any && *existed)
			.map(|(who, ..)| {
				// Keyed by the relay-chain account, but read at the address it continues under
				// here — a para sovereign already has a Coretime balance under its `sibl…` name,
				// and missing it would look like the migration invented money.
				let d = frame_system::Account::<Ct>::get(translate_destination(who)).data;
				(who.clone(), d.free + d.reserved)
			})
			.collect();
		(pallet_balances::TotalIssuance::<Ct>::get(), pures)
	});
	let (ah_ti_before, ah_checking_before, ah_sample_before) = ah.execute_with(|| {
		// Baseline probe so the event stream carries the checking account's pre-migration
		// value; the monitor measures the teleport receipts as the drain from this baseline.
		crate::events::emit_para_block(Chain::AssetHub);

		let checking = pallet_xcm::Pallet::<Ah>::check_account();
		(
			pallet_balances::TotalIssuance::<Ah>::get(),
			frame_system::Account::<Ah>::get(&checking).data.free,
			frame_system::Account::<Ah>::get(&sample.0).data.free,
		)
	});
	// Pre-migration AH balances of every address the sweep and sibl legs pay into.
	let ah_arrivals_before: BTreeMap<AccountId32, u128> = ah.execute_with(|| {
		sweep_pots
			.iter()
			.map(|(who, _)| who)
			.chain(sibl_before.iter().map(|(who, _)| who))
			.map(|who| (who.clone(), frame_system::Account::<Ah>::get(who).data.free))
			.collect()
	});

	// The relay chain's existential deposit: below it, accounts are reaped rather than migrated.
	let rc_existential_deposit: u128 =
		<crate::mock::network::relay::Runtime as pallet_balances::Config>::ExistentialDeposit::get(
		);

	// The preimages that must not survive: `can_migrate` refuses any account holding a named
	// hold, so one preimage deposit strands that account's whole balance. The migration releases
	// them itself in `AccountsInit`; this only records what was there so the post-check can prove
	// it happened.
	let preimages_before =
		rc.execute_with(|| pallet_preimage::RequestStatusFor::<Rc>::iter().count());

	// WHEN the whole migration runs, DMP shuttled after every burst of RC blocks.
	rc.execute_with(|| {
		let start = now_ms_rc() + RC_BLOCK_TIME_MS;
		pallet_rc2_migrator::Pallet::<Rc>::schedule_migration(
			crate::mock::network::relay::RuntimeOrigin::root(),
			start,
			WARM_UP,
			COOL_OFF,
		)
		.expect("root may schedule the migration");
	});
	rc.commit_all().unwrap();

	// The Coretime chain's answer to the handshake, carried up on the next round.
	let mut ct_ump: Vec<polkadot_primitives::UpwardMessage> = Vec::new();
	// Every stage the machine was seen in, so the run proves it passed through the handshake and
	// the warm-up rather than only that it reached the end.
	let mut seen: Vec<RcStage> = Vec::new();

	let mut rounds = 0;
	loop {
		rounds += 1;
		assert!(rounds <= 40, "migration must finish within 40 shuttle rounds");

		let (ct_dmp, ah_dmp, rc_stage) = rc.execute_with(|| {
			enqueue_ump(CoretimePara::PARA_ID.into(), core::mem::take(&mut ct_ump));
			for _ in 0..3 {
				next_block_rc();
			}
			(
				take_dmp(CoretimePara::PARA_ID.into()),
				take_dmp(AssetHubPara::PARA_ID.into()),
				RcMigrationStage::<Rc>::get(),
			)
		});
		rc.commit_all().unwrap();

		ct.execute_with(|| {
			enqueue_dmp::<CoretimePara>(ct_dmp);
			for _ in 0..3 {
				next_block_para::<CoretimePara>();
			}
			ct_ump = take_ump::<CoretimePara>();
		});
		ct.commit_all().unwrap();

		ah.execute_with(|| {
			enqueue_dmp::<AssetHubPara>(ah_dmp);
			for _ in 0..3 {
				next_block_para::<AssetHubPara>();
				// A trapped asset means a teleport failed half-way; that must never pass.
				assert!(
					!frame_system::Pallet::<Ah>::events().into_iter().any(|r| matches!(
						r.event,
						crate::mock::network::ah::RuntimeEvent::PolkadotXcm(
							pallet_xcm::Event::AssetsTrapped { .. }
						)
					)),
					"assets were trapped on Asset Hub"
				);
			}
		});
		ah.commit_all().unwrap();

		// --- during-migration sanity -------------------------------------------------------
		// Checked every round rather than only at the end, so a transient breach is caught
		// where it happens rather than being papered over by the final state.
		let ct_has_para: BTreeMap<u32, bool> = ct.execute_with(|| {
			// The receiving pallets' invariants must hold at every intermediate step, not just
			// once the machine stops. A migration that passes through a state where a channel
			// holds the wrong deposits has a bug, even if it tidies up afterwards.
			pallet_hrmp_para::Pallet::<Ct>::do_try_state()
				.expect("HRMP invariants broke mid-migration");
			pallet_registrar_para::Pallet::<Ct>::do_try_state()
				.expect("registrar invariants broke mid-migration");

			// Nothing may be handed over twice. Records arrive in batches across many blocks,
			// and a retried batch that re-inserted would double-charge a deposit.
			let paras = pallet_registrar_para::Paras::<Ct>::iter().count();
			assert!(
				paras <= paras_before.len(),
				"more paras on CT ({paras}) than the relay chain ever had"
			);
			let channels = pallet_hrmp_para::Channels::<Ct>::iter_keys().count();
			assert!(
				channels <= hrmp_before.len() + requests_before_detail.len(),
				"more migrated channels on CT ({channels}) than the relay chain ever had"
			);

			paras_before_detail
				.iter()
				.map(|(id, _, _)| (*id, pallet_registrar_para::Paras::<Ct>::contains_key(*id)))
				.collect()
		});
		ct.commit_all().unwrap();

		// Cores keep getting assigned, from the PRD's during-migration list. The claim queue is
		// the collator-facing answer to "which para may build on which core", and it is refilled by
		// the scheduler at session boundaries — so this is only a real check because the harness
		// can now cross one mid-migration. If the migration ever disturbed the assigner or the
		// scheduler, this is where it would show, while the machine is still running rather than
		// after it has stopped.
		rc.execute_with(|| {
			let before = runtime_parachains::runtime_api_impl::v13::claim_queue::<Rc>();
			assert!(
				!before.is_empty(),
				"cores must be assigned mid-migration, at stage {rc_stage:?}"
			);

			rotate_session_rc();

			let after = runtime_parachains::runtime_api_impl::v13::claim_queue::<Rc>();
			assert!(
				!after.is_empty(),
				"cores must still be assigned after a session boundary mid-migration, at stage \
				 {rc_stage:?}"
			);
			// Not merely non-empty — the same cores, still carrying work. A queue that emptied and
			// refilled with nothing would satisfy a bare "is_empty" check.
			assert_eq!(
				before.keys().collect::<BTreeSet<_>>(),
				after.keys().collect::<BTreeSet<_>>(),
				"the set of scheduled cores changed across a session boundary mid-migration"
			);
		});
		rc.commit_all().unwrap();

		// A record that reaches CT must have left the relay chain: the two sides must never
		// both claim the same para, or there are two control planes for it at once.
		rc.execute_with(|| {
			for (para_id, _, _) in &paras_before_detail {
				let on_rc = paras_registrar::Paras::<Rc>::contains_key(
					polkadot_primitives::Id::from(*para_id),
				);
				let on_ct = ct_has_para.get(para_id).copied().unwrap_or(false);
				assert!(!(on_rc && on_ct), "para {para_id} is claimed by both chains at once");
			}
		});
		rc.commit_all().unwrap();

		seen.push(rc_stage.clone());
		if rc_stage == RcStage::MigrationDone {
			break;
		}
	}

	assert!(
		seen.contains(&RcStage::WaitingForCt),
		"the migration must wait for the Coretime chain before moving data: {seen:?}"
	);
	assert!(
		seen.iter().any(|stage| matches!(stage, RcStage::WarmUp { .. })),
		"the migration must warm up before moving data: {seen:?}"
	);

	// Drain both directions until quiet, so anything either side queued during the migration is
	// delivered before the after-state is asserted on. The queues are serviced a batch at a time,
	// so one round is not enough.
	for _ in 0..12 {
		let ump = ct.execute_with(|| {
			next_block_para::<CoretimePara>();
			take_ump::<CoretimePara>()
		});
		ct.commit_all().unwrap();

		let dmp = rc.execute_with(|| {
			enqueue_ump(CoretimePara::PARA_ID.into(), ump);
			next_block_rc();
			take_dmp(CoretimePara::PARA_ID.into())
		});
		rc.commit_all().unwrap();

		ct.execute_with(|| {
			enqueue_dmp::<CoretimePara>(dmp);
			next_block_para::<CoretimePara>();
		});
		ct.commit_all().unwrap();
	}

	// THEN the RC has given up the registry, kept what it still routes on, and reduced issuance by
	// exactly the burn.
	let (tracker, migrated_nonce0) = rc.execute_with(|| {
		crate::events::emit_rc_census("after");
		assert!(
			paras_registrar::Paras::<Rc>::iter().next().is_none(),
			"all registrar records must be drained from the RC"
		);

		// The HRMP records stay. The relay chain refuses any candidate whose outbound channel it
		// cannot find (`check_outbound_hrmp`), and it is the only thing that promotes an open
		// request to a channel, at a session boundary. What leaves is the money: Coretime holds
		// every deposit now, so a figure left behind here would be a refund against an account the
		// accounts stage has emptied.
		assert!(
			HrmpChannels::<Rc>::iter().next().is_some(),
			"the RC must keep its HRMP channels: it routes every message through them"
		);
		for (id, channel) in HrmpChannels::<Rc>::iter() {
			assert_eq!(
				(channel.sender_deposit, channel.recipient_deposit),
				(0, 0),
				"channel {id:?} kept a deposit the RC can no longer refund"
			);
		}
		for (id, request) in runtime_parachains::hrmp::HrmpOpenChannelRequests::<Rc>::iter() {
			assert_eq!(
				request.sender_deposit, 0,
				"open request {id:?} kept a deposit the RC can no longer refund"
			);
		}

		// The relay chain's own HRMP invariant, which nothing here used to check. The
		// ingress/egress indexes must describe exactly the set of channels in `HrmpChannels`;
		// upstream asserts this in `assert_storage_consistency_exhaustive`, which is private and
		// `test`-gated so it cannot be called from here. The indexes are maintained incrementally
		// and never rebuilt, so any stage that removes a channel without touching them leaves the
		// relay chain permanently inconsistent — this is what makes that self-catching.
		let from_ingress: BTreeSet<(u32, u32)> =
			runtime_parachains::hrmp::HrmpIngressChannelsIndex::<Rc>::iter()
				.flat_map(|(recipient, senders)| {
					senders
						.into_iter()
						.map(move |sender| (sender.into(), recipient.into()))
						.collect::<Vec<_>>()
				})
				.collect();
		let from_egress: BTreeSet<(u32, u32)> =
			runtime_parachains::hrmp::HrmpEgressChannelsIndex::<Rc>::iter()
				.flat_map(|(sender, recipients)| {
					recipients
						.into_iter()
						.map(move |recipient| (sender.into(), recipient.into()))
						.collect::<Vec<_>>()
				})
				.collect();
		let ground_truth: BTreeSet<(u32, u32)> = HrmpChannels::<Rc>::iter_keys()
			.map(|id| (id.sender.into(), id.recipient.into()))
			.collect();
		assert_eq!(from_ingress, ground_truth, "HRMP ingress index diverged from HrmpChannels");
		assert_eq!(from_egress, ground_truth, "HRMP egress index diverged from HrmpChannels");

		// And the relay chain still *tells* paras about those channels. `HrmpChannels` holding a
		// row is necessary but not the property that matters: what decides whether a parachain may
		// actually send is `backing_constraints`, the runtime API a collator asks. It reads the
		// same map, so a drain leaves `hrmp_channels_out` empty and every para silently believes
		// it has nowhere to send — with no error anywhere, because nothing was refused, nothing
		// was ever attempted.
		let mut egress = BTreeMap::<u32, BTreeSet<u32>>::new();
		for id in HrmpChannels::<Rc>::iter_keys() {
			egress.entry(id.sender.into()).or_default().insert(id.recipient.into());
		}
		let mut checked = 0usize;
		for (sender, recipients) in &egress {
			let Some(constraints) = runtime_parachains::runtime_api_impl::v13::backing_constraints::<
				Rc,
			>(polkadot_primitives::Id::from(*sender))
			else {
				// Not a fully live para (no head or no current code), so it has no constraints to
				// report. Nothing to assert for it.
				continue;
			};
			let reported: BTreeSet<u32> = constraints
				.hrmp_channels_out
				.iter()
				.map(|(para, _)| u32::from(*para))
				.collect();
			assert_eq!(
				&reported, recipients,
				"para {sender} is told a different set of outbound channels than the RC holds"
			);
			checked += 1;
		}
		assert!(
			checked > 10,
			"only {checked} paras had reportable constraints; the assertion above is not covering 			 the migrated set"
		);

		// No ghost proxy records: every remaining entry's recorded deposit is backed by an
		// actual reserve, and the dispatch-check manager's translatable defs left for CT.
		for (who, (_, deposit)) in pallet_proxy::Proxies::<Rc>::iter() {
			assert!(
				deposit <= frame_system::Account::<Rc>::get(&who).data.reserved,
				"proxy entry of {who:?} claims a deposit that is not reserved"
			);
			assert!(
				frame_system::Account::<Rc>::contains_key(&who),
				"fund-less proxy entries (v1 husks) must be deleted, found {who:?}"
			);
		}
		if let Some((manager, ..)) = &proxy_dispatch {
			let residual = pallet_proxy::Proxies::<Rc>::get(manager).0;
			assert!(
				residual
					.iter()
					.all(|d| PortableProxyType::try_from(d.proxy_type.clone()).is_err()),
				"the manager's translatable defs must have left the RC"
			);
		}

		// Every funded pot is gone; its balance teleported to the same address on AH.
		for (who, amount) in &sweep_pots {
			if *amount > 0 {
				assert!(
					!frame_system::Account::<Rc>::contains_key(who),
					"pot {who:?} must be swept"
				);
			}
		}

		// No preimage deposit survived. The migration releases them in `AccountsInit`, because
		// `can_migrate` refuses any account holding a named hold — on Kusama one preimage was
		// holding back a para manager with 539 KSM of registrar deposits.
		//
		// A *requested* preimage keeps its blob and loses only its ticket, so this asserts on the
		// deposits, not on the preimages themselves.
		let deposits_left = pallet_preimage::RequestStatusFor::<Rc>::iter()
			.filter(|(_, status)| match status {
				pallet_preimage::RequestStatus::Unrequested { .. } => true,
				pallet_preimage::RequestStatus::Requested { maybe_ticket, .. } =>
					maybe_ticket.is_some(),
			})
			.count();
		assert_eq!(
			deposits_left, 0,
			"every preimage deposit must be released ({preimages_before} existed before)"
		);

		// The RC end state: not a single planck remains anywhere, and every surviving record
		// earns its place — something still references it (session key-holders being the known
		// case) or it is a module account. Unreferenced husks are reaped.
		for (who, info) in frame_system::Account::<Rc>::iter() {
			assert_eq!(
				info.data.free + info.data.reserved,
				0,
				"only zero-balance shells may stay on the RC, found {who:?}"
			);
			let bytes: &[u8] = who.as_ref();
			assert!(
				info.consumers > 0 || bytes.starts_with(b"modl"),
				"unreferenced husk {who:?} must have been reaped"
			);
		}

		// Never-signed delegators whose accounts migrated away (fund-less husks never had an
		// account to migrate): their CT-side control is asserted below.
		let migrated_nonce0: Vec<_> = nonce0_delegators
			.iter()
			.filter(|(who, _, existed, _)| {
				*existed && !frame_system::Account::<Rc>::contains_key(who)
			})
			.cloned()
			.collect();
		// The measured counterpart of `balance_census`'s pre-migration gap list: every account
		// still here, per line. Visible with `--nocapture`.
		println!("\n### remaining on RC after the migration");
		print_remaining_on_rc(false);

		// The proxy entries that survive the proxy stage, with why their delegator is still
		// here (session keys / unattributable reserve).
		println!("\n### proxy entries remaining on RC");
		for (who, (defs, deposit)) in pallet_proxy::Proxies::<Rc>::iter() {
			let account = frame_system::Account::<Rc>::get(&who);
			let types: Vec<String> =
				defs.iter().map(|d| format!("{:?}/{}", d.proxy_type, d.delay)).collect();
			println!(
				"{} | defs [{}] deposit {:.4} | free {:.4} reserved {:.4} nonce {} \
				 session-keys {}",
				ss58(&who),
				types.join(", "),
				dot(deposit),
				dot(account.data.free),
				dot(account.data.reserved),
				account.nonce,
				pallet_session::NextKeys::<Rc>::get(&who).is_some(),
			);
		}

		let tracker = RcMigratedBalance::<Rc>::get();
		assert_eq!(
			tracker.kept +
				tracker.ct_reserved +
				tracker.ct_free +
				tracker.ah_free +
				tracker.ti_corrected,
			rc_ti_before,
			"balance bookkeeping is exact"
		);
		assert_eq!(pallet_balances::TotalIssuance::<Rc>::get(), tracker.kept);
		// Where the relay chain's issuance went. Printed on every run because it is the headline
		// number anyone reviewing a migration wants, and it differs per network.
		println!("\n### the relay chain drained {:.4} {TOKEN}", dot(rc_ti_before));
		println!("  to Coretime, held:  {:.4}", dot(tracker.ct_reserved));
		println!("  to Coretime, free:  {:.4}", dot(tracker.ct_free));
		println!("  to Asset Hub:       {:.4}", dot(tracker.ah_free));
		println!("  burned (TI corr.):  {:.4}", dot(tracker.ti_corrected));
		println!("  kept behind:        {:.4}", dot(tracker.kept));
		// The headline: the relay chain ends the migration with ZERO issuance.
		assert_eq!(tracker.kept, 0, "the RC must drain to exactly zero issuance");
		// The audited phantom issuance was burned in full: the runtime constant equals the
		// measured unaccounted issuance on this snapshot, so nothing remains and no anomaly.
		assert_eq!(
			tracker.ti_corrected,
			crate::mock::network::relay::TiCorrection::get(),
			"TI correction burned exactly the audited amount"
		);
		(tracker, migrated_nonce0)
	});
	let migrated_ct = tracker.migrated_ct();

	// AND Coretime holds every record, every deposit is re-attributed or parked, and issuance
	// grew by exactly what the RC burned.
	ct.execute_with(|| {
		use pallet_ct_migrator::*;
		crate::events::emit_ct_census("after");

		use pallet_hrmp_para::ChannelState;
		use pallet_registrar_para::RegistrationState;

		assert_eq!(CtMigrationStage::<Ct>::get(), MigrationStage::MigrationDone);
		let failed_paras: Vec<u32> = FailedParas::<Ct>::iter_keys().collect();
		assert!(
			failed_paras.is_empty(),
			"{} of {} paras failed to integrate: {failed_paras:?}",
			failed_paras.len(),
			paras_before.len(),
		);
		let failed_channels: Vec<_> = FailedHrmpChannels::<Ct>::iter_keys().collect();
		assert!(
			failed_channels.is_empty(),
			"{} channels failed to integrate: {failed_channels:?}",
			failed_channels.len(),
		);
		// Nothing may still be in flight once the migration reports itself done: the relay chain
		// burned whatever an unconfirmed batch carries.
		let unconfirmed: Vec<u64> =
			pallet_rc2_migrator::UnconfirmedBatches::<Rc>::iter_keys().collect();
		assert!(
			unconfirmed.is_empty(),
			"{} batch(es) were never confirmed: {unconfirmed:?}",
			unconfirmed.len(),
		);

		// --- the registrar pallet actually owns the paras now -----------------------------
		// System chains are deliberately not among them: they are not registered through this
		// pallet, hold no deposit here, and its own `do_try_state` rejects ids below the floor.
		let floor: u32 = <Ct as pallet_registrar_para::Config>::FirstPublicParaId::get();
		let expected: Vec<_> =
			paras_before_detail.iter().filter(|(id, _, _)| *id >= floor).collect();
		assert!(
			expected.len() < paras_before_detail.len(),
			"the snapshot must contain system paras, or the skip rule is untested"
		);
		assert_eq!(
			pallet_registrar_para::Paras::<Ct>::iter().count(),
			expected.len(),
			"every public para landed in the registrar pallet, and no system para did"
		);
		assert!(pallet_registrar_para::NextFreeParaId::<Ct>::get() > 0, "the id counter migrated");
		for (para_id, registered, locked) in &expected {
			let info = pallet_registrar_para::Paras::<Ct>::get(*para_id)
				.unwrap_or_else(|| panic!("para {para_id} must land"));

			// A live para must arrive locked. Miss this and its manager silently regains the
			// control the relay chain's lock existed to remove — the whole point of carrying
			// the flag across.
			assert_eq!(
				info.locked, *locked,
				"para {para_id} must arrive with the relay chain's lock"
			);

			// Reserved and Registered are told apart by `paras::ParaLifecycles`, which stays on
			// the relay chain. If that classification were lost, a merely reserved id would
			// arrive holding a registration deposit it never paid.
			match (registered, &info.state) {
				(true, RegistrationState::Registered { .. }) => {},
				(false, RegistrationState::Reserved) => {},
				(_, other) =>
					panic!("para {para_id}: registered={registered} but arrived as {other:?}"),
			}
		}

		// --- and the HRMP pallet owns the channels ----------------------------------------
		// Exactly the migrated channels and requests — nothing more. There is deliberately no
		// CT↔para control channel per registered para any more: the control link is relayed
		// through the relay chain, which keeps its para-facing calls and forwards them, so no
		// per-para channel exists to be capped, refused while onboarding, or collided with.
		// (Kusama is what forced this: the relay chain caps how many channels one para may hold,
		// the old design wanted one per para it managed, and 156 of 214 directions were refused
		// `LimitExceeded` — see B25 and `decisions.md` 2026-08-31.)
		let want: std::collections::BTreeSet<_> = hrmp_before
			.iter()
			.map(|(id, _, _)| (u32::from(id.sender), u32::from(id.recipient)))
			.chain(requests_before_detail.iter().map(|(s, r, _)| (*s, *r)))
			.collect();
		let have: std::collections::BTreeSet<_> = pallet_hrmp_para::Channels::<Ct>::iter_keys()
			.map(|c| (c.sender, c.recipient))
			.collect();
		assert_eq!(have, want, "the HRMP pallet holds exactly the migrated channels and requests");
		for (id, _, _) in &hrmp_before {
			let key = hrmp_primitives::ChannelId {
				sender: id.sender.into(),
				recipient: id.recipient.into(),
			};
			let info = pallet_hrmp_para::Channels::<Ct>::get(key)
				.unwrap_or_else(|| panic!("channel {key:?} must land"));
			// An existing channel means both ends paid, so it arrives fully open.
			assert_eq!(info.state, ChannelState::Open, "channel {key:?} must arrive open");
		}
		for (sender, recipient, confirmed) in &requests_before_detail {
			let key = hrmp_primitives::ChannelId { sender: *sender, recipient: *recipient };
			let info = pallet_hrmp_para::Channels::<Ct>::get(key)
				.unwrap_or_else(|| panic!("request {key:?} must land"));
			// An unconfirmed request is the sender's deposit alone, which is exactly what
			// `Pending` means here. Getting this wrong would hold money nobody paid.
			let want = if *confirmed { ChannelState::Open } else { ChannelState::Pending };
			assert_eq!(info.state, want, "request {key:?} must arrive in the right state");
		}

		// --- the id counter cannot hand out something already taken ----------------------
		// After the drain the relay chain's counter is gone, so Coretime's is the only one. If
		// it arrived below a live id, the next `reserve` would collide with a real parachain.
		let highest = paras_before_detail.iter().map(|(id, _, _)| *id).max().unwrap_or(0);
		assert!(
			pallet_registrar_para::NextFreeParaId::<Ct>::get() > highest,
			"the id counter must sit above every migrated para"
		);

		// --- migrated state must be usable, not merely present ---------------------------
		// The strongest thing this test can say: pick real migrated paras out of the snapshot
		// and drive them through the pallet as their manager would.
		let locked_para = paras_before_detail.iter().find(|(_, _, locked)| *locked == Some(true));
		if let Some((para_id, _, _)) = locked_para {
			let manager = pallet_registrar_para::Paras::<Ct>::get(*para_id).unwrap().manager;
			// A locked para's manager must be shut out. This is the whole reason the lock is
			// carried across; if it silently failed, the manager would regain control of a live
			// parachain.
			let refused = pallet_registrar_para::Pallet::<Ct>::deregister(
				crate::mock::network::ct::RuntimeOrigin::signed(manager.clone()),
				*para_id,
			);
			assert!(refused.is_err(), "a locked migrated para must refuse its manager");

			// And the manager may still lock further: locking is never blocked by a lock.
			assert_ok!(pallet_registrar_para::Pallet::<Ct>::add_lock(
				crate::mock::network::ct::RuntimeOrigin::signed(manager),
				*para_id,
			));
		}

		let reserved_para = paras_before_detail
			.iter()
			.find(|(_, registered, locked)| !*registered && *locked != Some(true));
		if let Some((para_id, _, _)) = reserved_para {
			let info = pallet_registrar_para::Paras::<Ct>::get(*para_id).unwrap();
			let before = pallet_balances::Pallet::<Ct>::free_balance(&info.manager);
			// A reserved id is dropped locally, deposit returned, without ever touching the
			// relay chain — so it works even though nothing is listening up there any more.
			assert_ok!(pallet_registrar_para::Pallet::<Ct>::deregister(
				crate::mock::network::ct::RuntimeOrigin::signed(info.manager.clone()),
				*para_id,
			));
			assert!(pallet_registrar_para::Paras::<Ct>::get(*para_id).is_none());
			assert!(
				pallet_balances::Pallet::<Ct>::free_balance(&info.manager) > before,
				"dropping a reserved id must return its deposit"
			);
		}

		// --- the pallets' own invariants hold across the whole live snapshot --------------
		// This is the strongest check in the test: it says every channel holds exactly the
		// deposits its state claims, for real migrated data rather than a fixture.
		assert_ok!(pallet_hrmp_para::Pallet::<Ct>::do_try_state());
		assert_ok!(pallet_registrar_para::Pallet::<Ct>::do_try_state());

		// Reconciliation: re-attributed + parked shortfalls == the recorded totals, for both
		// deposit kinds. Nothing is invented and nothing is silently dropped.
		let reattributed = ReattributedDeposits::<Ct>::get();
		let parked: u128 = ParkedDepositShortfalls::<Ct>::iter().map(|(_, v)| v).sum();
		assert_eq!(reattributed + parked, recorded_deposits, "deposit reconciliation is exact");
		assert!(reattributed > 0, "at least some deposits must re-attribute");
		let reattributed_hrmp = ReattributedHrmpDeposits::<Ct>::get();
		let parked_hrmp: u128 = ParkedHrmpShortfalls::<Ct>::iter().map(|(_, v)| v).sum();
		assert_eq!(
			reattributed_hrmp + parked_hrmp,
			recorded_hrmp,
			"HRMP deposit reconciliation is exact"
		);

		// The migrator releases every migrated registrar and HRMP deposit for the owning pallet
		// to take its own `Consideration` at this chain's rates, so the only holds it may still
		// carry are proxy deposits awaiting their definitions and the parked unattributed reserve.
		let proxy_id =
			crate::mock::network::ct::RuntimeHoldReason::CtMigrator(HoldReason::ProxyDeposit);
		let unattributed_id = crate::mock::network::ct::RuntimeHoldReason::CtMigrator(
			HoldReason::UnattributedReserve,
		);
		let (mut proxy_held, mut unattributed_held) = (0u128, 0u128);
		let mut proxy_stuck: Vec<(AccountId32, u128)> = Vec::new();
		for who in frame_system::Account::<Ct>::iter_keys() {
			for hold in pallet_balances::Holds::<Ct>::get(&who) {
				if hold.id == proxy_id {
					proxy_held += hold.amount;
					proxy_stuck.push((who.clone(), hold.amount));
				} else if hold.id == unattributed_id {
					unattributed_held += hold.amount;
				}
			}
		}

		// What the pallets took instead. Priced at this chain's rates, so it is not comparable
		// to the relay chain's recorded amounts — only its existence and attribution are.
		let mut pallet_held = 0u128;
		for who in frame_system::Account::<Ct>::iter_keys() {
			for hold in pallet_balances::Holds::<Ct>::get(&who) {
				let is_control_plane = matches!(
					hold.id,
					crate::mock::network::ct::RuntimeHoldReason::RegistrarPara(_) |
						crate::mock::network::ct::RuntimeHoldReason::HrmpPara(_)
				);
				if is_control_plane {
					pallet_held += hold.amount;
				}
			}
		}
		assert!(pallet_held > 0, "the control-plane pallets must hold their own deposits");
		println!("control-plane deposits held on CT: {:.4} {TOKEN}", dot(pallet_held));
		// Every migrated proxy deposit is resized (released and re-reserved at this chain's
		// rates) when its definitions arrive; a remaining hold would mean defs never followed.
		// No para sovereign may still be holding a *migrator* hold on Coretime. This is the
		// invariant both halves of the sovereign-translation bug violated: the accounts stage
		// sends a child sovereign's balance to the `sibl…` address, so every stage that names an
		// account must send the same one. A `para…` key arriving here means some stage did not
		// translate, and the money lands on an address nothing will ever resize or refund.
		for (who, holds) in frame_system::Account::<Ct>::iter_keys()
			.map(|who| (who.clone(), pallet_balances::Holds::<Ct>::get(&who)))
			.filter(|(_, h)| !h.is_empty())
		{
			let bytes: &[u8] = who.as_ref();
			assert!(
				!bytes.starts_with(b"para"),
				"{} is a relay-chain child sovereign holding {:?} on Coretime; some stage sent an \
				 untranslated account id",
				ss58(&who),
				holds.iter().map(|h| format!("{:?}", h.id)).collect::<Vec<_>>(),
			);
		}

		if proxy_held != 0 {
			// Name them: a bare total says nothing about whether this is one odd account or a
			// systematic gap in which definitions travel.
			for (who, amount) in &proxy_stuck {
				let defs = pallet_proxy::Proxies::<Ct>::get(who).0;
				println!(
					"stuck proxy deposit: {} {:.4} {TOKEN} | defs on CT {:?}",
					ss58(who),
					dot(*amount),
					defs.iter().map(|d| format!("{:?}", d.proxy_type)).collect::<Vec<_>>(),
				);
			}
		}
		assert_eq!(
			proxy_held,
			0,
			"no unresized proxy deposit may remain ({} account(s), see the list above)",
			proxy_stuck.len(),
		);
		// The RC's anomalous reserves (backed by no deposit record) must arrive parked rather
		// than stay behind; the snapshot is known to carry some.
		assert!(unattributed_held > 0, "unattributed reserves must arrive parked on CT");
		println!("unattributed reserves parked on CT: {:.4} {TOKEN}", dot(unattributed_held));

		assert_eq!(CtMintedTotal::<Ct>::get(), migrated_ct, "CT minted exactly the CT-bound burn");
		assert_eq!(pallet_balances::TotalIssuance::<Ct>::get(), ct_ti_before + migrated_ct);

		// AND every portable definition was recreated in the REAL proxy pallet, so keyless
		// (pure) delegators can dispatch here from day one.
		assert!(FailedProxies::<Ct>::iter().next().is_none(), "no proxy set may fail");
		let failed_accounts: Vec<_> = FailedAccounts::<Ct>::iter_keys().map(|w| ss58(&w)).collect();
		assert!(
			failed_accounts.is_empty(),
			"{} account(s) failed to integrate: {failed_accounts:?}",
			failed_accounts.len(),
		);
		for who in &ct_bound_proxies {
			// Under the address the account continues at here: a child sovereign's balance, its
			// deposit and its definitions all arrive at the `sibl…` address, so the delegator is
			// looked up there and not under its relay-chain key.
			let here = translate_destination(who);
			assert!(
				!pallet_proxy::Proxies::<Ct>::get(&here).0.is_empty(),
				"delegator {} must have proxies on CT (looked up as {})",
				ss58(who),
				ss58(&here),
			);
		}

		// Funds-follow-control invariant: every never-signed delegator with an `Any` definition
		// moved WHOLE to this chain — its balance and an `Any` def — so the delegate keeps full
		// control. A violation strands money nobody can use.
		//
		// No size exemption. A pure proxy's funds are reachable only through its delegate, so a
		// below-ED one left behind is not "dust that does not matter" — it is money separated
		// from its only key, and the amount has nothing to do with it. `can_migrate` exempts
		// these from its below-ED rule for exactly this reason.
		for (who, had_any, _, rc_total) in &migrated_nonce0 {
			if !*had_any {
				continue;
			}
			// Below-ED delegators are reaped rather than migrated — a deliberate exception, not a
			// gap: their definitions still reach this chain so the delegate keeps the para, and a
			// sub-ED remainder is not worth a migration path of its own. Printed, so the exception
			// is visible in every run rather than implied by the assertion's absence.
			if *rc_total < rc_existential_deposit {
				println!(
					"below-ED delegator {} reaped, {:.6} {TOKEN} not carried",
					ss58(who),
					dot(*rc_total),
				);
				continue;
			}
			// A para sovereign also reads as never-signed with an `Any` definition, and it is the
			// one kind of delegator that does not keep its address: look it up where it lands.
			let here = translate_destination(who);
			let d = frame_system::Account::<Ct>::get(&here).data;
			let before = ct_pures_before.get(who).copied().unwrap_or_default();
			assert_eq!(
				d.free + d.reserved,
				before + rc_total,
				"possible pure {}'s whole RC balance must arrive on CT (as {})",
				ss58(who),
				ss58(&here),
			);
			assert!(
				pallet_proxy::Proxies::<Ct>::get(&here)
					.0
					.iter()
					.any(|def| def.proxy_type == crate::mock::network::ct::ProxyType::Any),
				"possible pure {} must have an Any definition on CT (as {})",
				ss58(who),
				ss58(&here),
			);
		}

		// Dispatch checks through the recreated definitions: the Any delegate acts for the
		// manager, and the ParaRegistration delegate is recognised but confined to its scope.
		//
		// Skipped where the snapshot has no such delegator — Kusama has no `ParaRegistration`
		// proxies at all. The fixture asserts *why* it is absent, so this cannot quietly stop
		// testing anything on a chain that does have them.
		if let Some((manager, any_delegate, reg_delegate)) = proxy_dispatch.clone() {
			let remark = || {
				Box::new(crate::mock::network::ct::RuntimeCall::System(
					frame_system::Call::remark_with_event { remark: b"via proxy".to_vec() },
				))
			};
			let proxy_call = |force, delegate: &AccountId32| {
				crate::mock::network::ct::RuntimeCall::Proxy(pallet_proxy::Call::proxy {
					real: sp_runtime::MultiAddress::Id(manager.clone()),
					force_proxy_type: force,
					call: remark(),
				})
				.dispatch(crate::mock::network::ct::RuntimeOrigin::signed(delegate.clone()))
			};
			let executed = |records: &[frame_system::EventRecord<_, _>]| {
				records.iter().rev().find_map(|r| match &r.event {
					crate::mock::network::ct::RuntimeEvent::Proxy(
						pallet_proxy::Event::ProxyExecuted { result },
					) => Some(result.clone()),
					_ => None,
				})
			};

			proxy_call(Some(crate::mock::network::ct::ProxyType::Any), &any_delegate)
				.expect("Any delegate may dispatch for the pure manager");
			assert_eq!(
				executed(&frame_system::Pallet::<Ct>::events()),
				Some(Ok(())),
				"the Any-proxied call must execute"
			);

			proxy_call(Some(crate::mock::network::ct::ProxyType::ParaRegistration), &reg_delegate)
				.expect("ParaRegistration delegate is recognised");
			assert!(
				matches!(executed(&frame_system::Pallet::<Ct>::events()), Some(Err(_))),
				"a ParaRegistration proxy must not be able to dispatch an arbitrary call"
			);
		} else {
			println!("no Any+ParaRegistration delegator in this snapshot; dispatch checks skipped");
		}
	});

	// AND Asset Hub received the teleports: issuance unchanged, the checking account (the "{TOKEN}
	// out on the RC" ledger) drained by exactly the teleported total, the sample manager's free
	// balance arrived, and nothing was trapped (asserted per block in the loop above).
	let (_, ah_free_exp) = expected_split(sample.1, sample.2);
	ah.execute_with(|| {
		assert_eq!(
			pallet_balances::TotalIssuance::<Ah>::get(),
			ah_ti_before,
			"teleports must not change AH issuance"
		);
		let checking = pallet_xcm::Pallet::<Ah>::check_account();
		let checking_now = frame_system::Account::<Ah>::get(&checking).data.free;
		assert_eq!(
			ah_checking_before - checking_now,
			tracker.ah_free,
			"AH checking account drained by exactly the teleported total"
		);
		assert_eq!(
			frame_system::Account::<Ah>::get(&sample.0).data.free,
			ah_sample_before + ah_free_exp,
			"the manager's free balance arrived on AH"
		);

		// The swept pots and reaped dust landed on the sweep beneficiary, exactly.
		let pots_total: u128 = sweep_pots.iter().map(|(_, amount)| amount).sum();
		assert_eq!(
			frame_system::Account::<Ah>::get(&treasury).data.free,
			ah_arrivals_before[&treasury] + pots_total + sweep_dust,
			"pots and dust must arrive on the sweep beneficiary"
		);

		// The sibl-format sovereigns' balances arrived on the same bytes — on AH those ARE the
		// paras' sovereign accounts, so the paras regain control. The expectation includes the
		// child sovereign's AH leg, which translates to the same address.
		for (who, total) in &sibl_before {
			assert_eq!(
				frame_system::Account::<Ah>::get(who).data.free,
				ah_arrivals_before[who] + total,
				"sibl sovereign {who:?}'s balance must arrive on its AH sovereign account"
			);
		}

		// Never-signed delegators with `Any` defs went whole to CT (asserted there); the ones
		// without are multisigs, controllable on every chain by their members, so their teleport
		// here needs no per-account control check.
	});

	// --- post-migration: normal operations resume -------------------------------------------
	//
	// The PRD's post-migration verification list, for the parts that are this stream's: HRMP
	// messages keep flowing, and a parachain can still manage its own channels. Both are checked
	// against the state the migration actually produced rather than a reconstruction of it, which
	// is the whole reason they live at the end of this test.

	// The relay chain still tells every para which channels it may send on. `HrmpChannels` holding
	// rows is necessary; this is the property that matters, because it is what a collator reads.
	rc.execute_with(|| {
		let mut usable = 0usize;
		for id in HrmpChannels::<Rc>::iter_keys() {
			let Some(constraints) =
				runtime_parachains::runtime_api_impl::v13::backing_constraints::<Rc>(id.sender)
			else {
				continue;
			};
			let Some((_, limits)) =
				constraints.hrmp_channels_out.iter().find(|(para, _)| *para == id.recipient)
			else {
				panic!("{id:?} is in HrmpChannels but not reported to its sender");
			};
			// Not merely present — reported with room. A channel whose capacity read as zero would
			// be a channel a para is told it cannot send on, which is the same outage by a
			// different route.
			assert!(
				limits.messages_remaining > 0 && limits.bytes_remaining > 0,
				"{id:?} is reported with no room: {limits:?}"
			);
			usable += 1;
		}
		assert!(usable > 20, "only {usable} channels are usable; HRMP has not survived");
	});
	rc.commit_all().unwrap();

	// And a parachain can still open a channel *by dispatching exactly the call it dispatches
	// today*. This is the whole forwarding design end to end: the para's `Transact` lands on the
	// relay chain, whose body forwards it to Coretime, which takes the deposit and drives the relay
	// chain back — and the channel materialises at the session boundary like any other.
	let (opener, target) = rc.execute_with(|| {
		// Two live paras that have no channel between them yet, so the request is a real one.
		let existing: BTreeSet<(u32, u32)> = HrmpChannels::<Rc>::iter_keys()
			.map(|id| (id.sender.into(), id.recipient.into()))
			.collect();
		let live: Vec<u32> = paras_before_detail
			.iter()
			.map(|(id, _, _)| *id)
			.filter(|id| {
				runtime_parachains::paras::Pallet::<Rc>::is_valid_para(
					polkadot_primitives::Id::from(*id),
				)
			})
			.collect();
		live.iter()
			.flat_map(|a| live.iter().map(move |b| (*a, *b)))
			.find(|(a, b)| a != b && !existing.contains(&(*a, *b)))
			.expect("the snapshot must have two live paras without a channel between them")
	});
	rc.commit_all().unwrap();

	// Coretime takes the channel deposit from the opener's sovereign account there, so it has to be
	// able to pay. Funding it is legitimate setup: what is under test is the control plane, not
	// whether this particular para happens to be solvent.
	ct.execute_with(|| {
		let sovereign = <<Ct as pallet_hrmp_para::Config>::SovereignAccountOf as Convert<
			u32,
			AccountId32,
		>>::convert(opener);
		let _ =
			<pallet_balances::Pallet<Ct> as frame_support::traits::fungible::Mutate<_>>::mint_into(
				&sovereign,
				10_000_000_000_000,
			);
	});
	ct.commit_all().unwrap();

	let request = crate::mock::network::relay::RuntimeCall::Hrmp(runtime_parachains::hrmp::Call::<
		Rc,
	>::hrmp_init_open_channel {
		recipient: target.into(),
		proposed_max_capacity: 8,
		proposed_max_message_size: 1024,
	});

	// The para asks the relay chain with **exactly the message it sends today**: an unpaid
	// `Transact` of its own call, delivered as real UMP. Post-migration the barrier admits this
	// shape from any child para (closing B26), the origin converter hands it the narrow
	// control-plane origin, and the call's body forwards — so this drives the whole loop:
	// barrier, origin, seam, Coretime.
	let to_ct = rc.execute_with(|| {
		let _ = take_dmp(CoretimePara::PARA_ID.into());
		frame_system::Pallet::<Rc>::reset_events();
		enqueue_ump(polkadot_primitives::Id::from(opener), vec![unpaid_transact_native(request)]);
		next_block_rc();
		assert!(
			frame_system::Pallet::<Rc>::events().into_iter().any(|record| matches!(
				record.event,
				crate::mock::network::relay::RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				)
			)),
			"the para's unpaid control-plane message must be admitted and dispatched"
		);
		let forwarded = take_dmp(CoretimePara::PARA_ID.into());
		assert!(
			!forwarded.is_empty(),
			"the relay chain must forward para {opener}'s request to Coretime, not act on it"
		);
		// And it must NOT have acted locally: the request belongs to Coretime now.
		assert!(
			runtime_parachains::hrmp::HrmpOpenChannelRequests::<Rc>::get(
				&polkadot_primitives::HrmpChannelId {
					sender: opener.into(),
					recipient: target.into()
				}
			)
			.is_none(),
			"the relay chain must not have recorded the request itself"
		);
		forwarded
	});
	rc.commit_all().unwrap();

	// Coretime records it, takes the deposit, and asks the relay chain.
	let to_rc = ct.execute_with(|| {
		enqueue_dmp::<CoretimePara>(to_ct);
		next_block_para::<CoretimePara>();
		let channel = hrmp_primitives::ChannelId { sender: opener, recipient: target };
		let info = pallet_hrmp_para::Channels::<Ct>::get(channel)
			.expect("Coretime must hold the channel the para asked for");
		assert!(
			matches!(info.state, pallet_hrmp_para::ChannelState::Opening { .. }),
			"expected Opening, got {:?}",
			info.state
		);
		assert!(info.sender_ticket.is_some(), "Coretime must hold the sender's deposit");
		take_ump::<CoretimePara>()
	});
	ct.commit_all().unwrap();

	// The relay chain records the request, and the session boundary is what turns it into a
	// channel — as it is for any HRMP channel, migrated or not.
	rc.execute_with(|| {
		assert!(!to_rc.is_empty(), "Coretime must ask the relay chain to record the request");
		enqueue_ump(CoretimePara::PARA_ID.into(), to_rc);
		next_block_rc();

		let id =
			polkadot_primitives::HrmpChannelId { sender: opener.into(), recipient: target.into() };
		let request = runtime_parachains::hrmp::HrmpOpenChannelRequests::<Rc>::get(&id)
			.expect("the relay chain must now hold the request");
		assert!(!request.confirmed, "the recipient has not accepted yet");
		assert_eq!(
			request.sender_deposit, 0,
			"the relay chain takes no deposit: Coretime holds the money"
		);
	});
	rc.commit_all().unwrap();
}

/// The hand-written cross-chain call encodings must match the runtimes that receive them.
///
/// Each chain encodes the other's calls by hand, as a pallet index followed by a call index and the
/// SCALE payload (`para_control.rs` on both sides). The compiler checks none of it. Reorder a
/// `construct_runtime!`, renumber a `#[pallet::call_index]`, or change a message's shape, and every
/// message becomes an undecodable blob at the destination — which surfaces as XCM that silently
/// does nothing, not as a build failure.
///
/// So decode what each chain would actually send, using the real `RuntimeCall` of the chain that
/// receives it. These are pure encoding tests and need no snapshot.
mod call_encoding {
	use codec::{Decode, Encode};
	use coretime_polkadot_runtime::para_control::{
		HrmpRelayCalls, RegistrarRelayCalls, RelayRuntimePallets,
	};
	use hrmp_primitives::{ChannelId, MessageToRelayV1 as HrmpToRelay};
	use polkadot_runtime::para_control::{
		CoretimeRuntimePallets, HrmpParaCalls, RegistrarParaCalls,
	};
	use registrar_primitives::MessageToRelayV1 as RegistrarToRelay;
	use sp_core::H256;

	type RelayCall = polkadot_runtime::RuntimeCall;
	type CoretimeCall = coretime_polkadot_runtime::RuntimeCall;
	type AccountId = sp_runtime::AccountId32;

	const PARA: u32 = 2000;
	const OTHER: u32 = 2001;
	const MSG_ID: u64 = 7;

	fn channel() -> ChannelId {
		ChannelId { sender: PARA, recipient: OTHER }
	}

	fn manager() -> AccountId {
		AccountId::new([1u8; 32])
	}

	/// Every registrar message Coretime can send must decode on the relay chain as `receive`
	/// carrying the identical payload. Payload equality is the whole check: with one entry point
	/// there is no per-call routing left to drift, only the pallet index and the message bytes.
	#[test]
	fn coretime_registrar_calls_decode_on_the_relay_chain() {
		let messages = vec![
			RegistrarToRelay::Register {
				para_id: PARA,
				message_id: MSG_ID,
				manager: manager(),
				genesis_head: vec![1, 2, 3],
				code_hash: H256::repeat_byte(9),
				code_len: 42,
			},
			RegistrarToRelay::CancelRegistration { para_id: PARA, message_id: MSG_ID },
			RegistrarToRelay::Deregister { para_id: PARA, message_id: MSG_ID },
			RegistrarToRelay::CancelDeregistration { para_id: PARA, message_id: MSG_ID },
			RegistrarToRelay::AuthorizeCodeUpgrade {
				para_id: PARA,
				message_id: MSG_ID,
				code_hash: H256::repeat_byte(3),
				code_len: 11,
			},
			RegistrarToRelay::SetCurrentHead {
				para_id: PARA,
				message_id: MSG_ID,
				head: vec![4, 5],
			},
			RegistrarToRelay::RemoveUpgradeCooldown { para_id: PARA, message_id: MSG_ID },
		];

		for message in messages {
			let sent = registrar_primitives::MessageToRelay::V1(message);
			let encoded =
				RelayRuntimePallets::RegistrarRelay(RegistrarRelayCalls::Receive(sent.clone()))
					.encode();
			match RelayCall::decode(&mut &encoded[..])
				.unwrap_or_else(|e| panic!("{sent:?} does not decode on the relay chain: {e:?}"))
			{
				RelayCall::RegistrarRelay(pallet_registrar_relay::Call::receive { message }) =>
					assert_eq!(message, sent),
				other => panic!("{sent:?} decoded as {other:?}"),
			}
		}
	}

	/// Every HRMP message Coretime can send, same check.
	#[test]
	fn coretime_hrmp_calls_decode_on_the_relay_chain() {
		let messages = vec![
			HrmpToRelay::InitOpenChannel {
				channel: channel(),
				message_id: MSG_ID,
				max_capacity: 8,
				max_message_size: 1024,
			},
			HrmpToRelay::AcceptOpenChannel { channel: channel(), message_id: MSG_ID },
			HrmpToRelay::CloseChannel { channel: channel(), message_id: MSG_ID, initiator: PARA },
			HrmpToRelay::CancelOpenRequest { channel: channel(), message_id: MSG_ID },
			HrmpToRelay::EstablishSystemChannel { channel: channel(), message_id: MSG_ID },
		];

		for message in messages {
			let sent = hrmp_primitives::MessageToRelay::V1(message);
			let encoded =
				RelayRuntimePallets::HrmpRelay(HrmpRelayCalls::Receive(sent.clone())).encode();
			match RelayCall::decode(&mut &encoded[..])
				.unwrap_or_else(|e| panic!("{sent:?} does not decode on the relay chain: {e:?}"))
			{
				RelayCall::HrmpRelay(pallet_hrmp_relay::Call::receive { message }) =>
					assert_eq!(message, sent),
				other => panic!("{sent:?} decoded as {other:?}"),
			}
		}
	}

	/// The relay chain's replies. Both para pallets take every response through a single
	/// `receive`, so what matters here is that the pallet and call indices land, and that the
	/// payload survives.
	#[test]
	fn relay_reports_decode_on_coretime() {
		let registrar_report = registrar_primitives::MessageToPara::V1(
			registrar_primitives::MessageToParaV1::RegisterResponse {
				para_id: PARA,
				message_id: MSG_ID,
				outcome: Err(registrar_primitives::FailureReason::CannotUpgrade),
			},
		);
		let encoded = CoretimeRuntimePallets::RegistrarPara(RegistrarParaCalls::Receive(
			registrar_report.clone(),
		))
		.encode();
		match CoretimeCall::decode(&mut &encoded[..]).expect("registrar report does not decode") {
			CoretimeCall::RegistrarPara(pallet_registrar_para::Call::receive { message }) =>
				assert_eq!(message, registrar_report),
			other => panic!("registrar report decoded as {other:?}"),
		}

		let hrmp_report =
			hrmp_primitives::MessageToPara::V1(hrmp_primitives::MessageToParaV1::OpenResponse {
				channel: channel(),
				message_id: MSG_ID,
				outcome: Ok(()),
			});
		let encoded =
			CoretimeRuntimePallets::HrmpPara(HrmpParaCalls::Receive(hrmp_report.clone())).encode();
		match CoretimeCall::decode(&mut &encoded[..]).expect("HRMP report does not decode") {
			CoretimeCall::HrmpPara(pallet_hrmp_para::Call::receive { message }) =>
				assert_eq!(message, hrmp_report),
			other => panic!("HRMP report decoded as {other:?}"),
		}
	}

	/// Every call the relay chain forwards on a para's behalf must decode on Coretime as the
	/// matching extrinsic with the same arguments. `full_migration_rc_to_ct` drives only
	/// `OpenChannel` end to end, so the other eight of these hand-written indices are pinned here
	/// or nowhere.
	#[test]
	fn forwarded_para_calls_decode_on_coretime() {
		let decode = |encoded: Vec<u8>| {
			CoretimeCall::decode(&mut &encoded[..]).expect("forwarded call does not decode")
		};
		let head = vec![9u8; 16];

		use pallet_registrar_para::Call as RegistrarCall;
		let registrar_cases = vec![
			(
				RegistrarParaCalls::Deregister(PARA),
				CoretimeCall::RegistrarPara(RegistrarCall::deregister { para_id: PARA }),
			),
			(
				RegistrarParaCalls::AddLock(PARA),
				CoretimeCall::RegistrarPara(RegistrarCall::add_lock { para_id: PARA }),
			),
			(
				RegistrarParaCalls::RemoveLock(PARA),
				CoretimeCall::RegistrarPara(RegistrarCall::remove_lock { para_id: PARA }),
			),
			(
				RegistrarParaCalls::SetCurrentHead(PARA, head.clone()),
				CoretimeCall::RegistrarPara(RegistrarCall::set_current_head {
					para_id: PARA,
					head,
				}),
			),
		];
		for (sent, expected) in registrar_cases {
			assert_eq!(decode(CoretimeRuntimePallets::RegistrarPara(sent).encode()), expected);
		}

		use pallet_hrmp_para::Call as HrmpCall;
		let hrmp_cases = vec![
			(
				HrmpParaCalls::OpenChannel(PARA, OTHER, 8, 1024),
				CoretimeCall::HrmpPara(HrmpCall::open_channel {
					sender: PARA,
					recipient: OTHER,
					max_capacity: 8,
					max_message_size: 1024,
				}),
			),
			(
				HrmpParaCalls::AcceptOpenChannel(PARA, OTHER),
				CoretimeCall::HrmpPara(HrmpCall::accept_open_channel {
					sender: PARA,
					recipient: OTHER,
				}),
			),
			(
				HrmpParaCalls::CloseChannel(PARA, OTHER, PARA),
				CoretimeCall::HrmpPara(HrmpCall::close_channel {
					sender: PARA,
					recipient: OTHER,
					initiator: PARA,
				}),
			),
			(
				HrmpParaCalls::CancelOpenRequest(PARA, OTHER),
				CoretimeCall::HrmpPara(HrmpCall::cancel_open_request {
					sender: PARA,
					recipient: OTHER,
				}),
			),
			(
				HrmpParaCalls::EstablishSystemChannel(PARA, OTHER),
				CoretimeCall::HrmpPara(HrmpCall::establish_system_channel {
					sender: PARA,
					recipient: OTHER,
				}),
			),
		];
		for (sent, expected) in hrmp_cases {
			assert_eq!(decode(CoretimeRuntimePallets::HrmpPara(sent).encode()), expected);
		}
	}
}

/// The relay chain answers Coretime, and nobody else.
///
/// Two gates stand between a parachain and the relay-side control plane, and neither can do the
/// other's job: `SystemChildParachainAsNative` decides whether a para gets an origin of its own at
/// all, and `PostAhmFilter` decides which calls that origin may reach. This drives a real
/// `Transact` through both, because the failure mode they guard against is invisible — a call the
/// relay chain refuses inside XCM produces no error anyone sees, just a `Transact` that did
/// nothing.
///
/// The observable is the reply: every relay-side control-plane call reports its verdict back to
/// Coretime, success or failure. A queued DMP means the call really dispatched.
#[tokio::test]
async fn only_a_system_para_can_drive_the_relay_control_plane() {
	let mut rc = load(Chain::Relay).await;
	type Rc = crate::mock::network::relay::Runtime;
	let coretime: polkadot_primitives::Id = CoretimePara::PARA_ID.into();

	// A deregistration request for a para that does not exist. The verdict does not matter —
	// what matters is that a verdict is produced at all, which only happens if the call ran.
	let call = crate::mock::network::relay::RuntimeCall::RegistrarRelay(
		pallet_registrar_relay::Call::receive {
			message: registrar_primitives::MessageToRelay::V1(
				registrar_primitives::MessageToRelayV1::Deregister {
					para_id: 4_999,
					message_id: 7,
				},
			),
		},
	);
	let message = xcm::VersionedXcm::<()>::from(Xcm(vec![
		UnpaidExecution { weight_limit: Unlimited, check_origin: None },
		Transact {
			// How Coretime really sends: as itself, not as a sovereign account.
			origin_kind: OriginKind::Native,
			fallback_max_weight: None,
			call: call.encode().into(),
		},
	]))
	.encode();

	rc.execute_with(|| {
		// The live snapshot has unrelated traffic queued for Coretime; clear it so the assertions
		// below are about this message alone.
		let _ = take_dmp(coretime);

		// WHEN Coretime asks
		enqueue_ump(coretime, vec![message.clone()]);
		next_block_rc();
		assert!(
			!take_dmp(coretime).is_empty(),
			"Coretime's request must reach the relay chain and be answered"
		);

		// WHEN an ordinary parachain sends the identical message. Serviced by hand rather than
		// through `next_block_rc`, which asserts that nothing fails — here a failure is the
		// point, and asserting on it is stronger than asserting on a missing reply: it proves the
		// message was delivered and refused, not merely lost somewhere in the harness.
		enqueue_ump(2_000.into(), vec![message]);
		let now = frame_system::Pallet::<Rc>::block_number() + 1;
		frame_system::Pallet::<Rc>::set_block_number(now);
		frame_system::Pallet::<Rc>::reset_events();
		<crate::mock::network::relay::MessageQueue as OnInitialize<_>>::on_initialize(now);

		let refused = frame_system::Pallet::<Rc>::events().into_iter().any(|record| {
			matches!(
				record.event,
				crate::mock::network::relay::RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: false, .. }
				)
			)
		});
		assert!(refused, "an ordinary parachain's request must be refused, not executed");
		assert!(
			take_dmp(coretime).is_empty(),
			"and it must not produce a verdict, which would mean the call had run"
		);
	});
}

/// Probe: can the harness cross a Relay Chain session boundary at all against a real snapshot?
///
/// Everything the parachain machinery does lazily happens at a session boundary, and until this
/// existed the suite could not produce one. Kept as a test rather than folded into the helper so a
/// snapshot or runtime change that breaks rotation is reported here, not as a confusing failure in
/// whichever test happens to rotate first.
#[tokio::test]
async fn rc_can_cross_a_session_boundary() {
	let mut rc = load(Chain::Relay).await;

	rc.execute_with(|| {
		let before = session_index_rc();
		rotate_session_rc();
		assert_eq!(session_index_rc(), before + 1, "the parachains session index must advance");

		rotate_session_rc();
		assert_eq!(session_index_rc(), before + 2);
	});
}

/// A freshly registered parachain, driven the way Coretime really drives it, across the session
/// boundaries that registration actually takes — and the relayed control route working for it the
/// moment it is live.
///
/// This is the gap between the suite's two halves: the SDK's cross-chain tests use mocks, so they
/// have no `paras` lifecycle and no sessions, and `full_migration_rc_to_ct` exercises *migrated*
/// paras, which are already live. Nothing covered a *new* registration against the real lifecycle —
/// and `paras` takes `SESSION_DELAY` (2) boundaries to onboard one, so any test that does not
/// rotate cannot tell a working registration from one that silently went nowhere.
///
/// There is deliberately **no channel to wait for**. The old design opened a CT↔para control
/// channel at registration, which the relay chain refused while the para was still onboarding —
/// a whole class of invisible failure (B22), and a per-para channel budget Kusama could not fit
/// (B25). The control link is relayed through this chain instead: the para keeps sending exactly
/// the call it sends today, unpaid, and this chain forwards it to the control plane at the para's
/// expense over there. So what registration must deliver is *nothing but the registration* — and
/// what must work afterwards, with no further setup, is the forwarded route.
#[tokio::test]
async fn a_fresh_registration_across_sessions_and_the_relayed_control_route() {
	use runtime_parachains::hrmp::{HrmpChannels, HrmpOpenChannelRequests};
	use sp_runtime::traits::{BlakeTwo256, Hash};

	type Rc = crate::mock::network::relay::Runtime;
	let mut rc = load(Chain::Relay).await;

	let coretime: polkadot_primitives::Id = CoretimePara::PARA_ID.into();
	// An id the live snapshot does not know, so the registration is a genuinely new one.
	let fresh_id: u32 = 4_999;
	let fresh: polkadot_primitives::Id = fresh_id.into();
	let manager = AccountId32::new([7u8; 32]);
	// Above `MIN_CODE_SIZE`; the relay chain takes no deposit on this path, so the manager needs
	// no balance here — Coretime holds the money.
	let code = vec![1u8; 32];
	let code_hash = BlakeTwo256::hash(&code);
	let genesis_head = vec![2u8; 16];

	// Coretime asks the relay chain to accept the registration, exactly as `register` does.
	let register = crate::mock::network::relay::RuntimeCall::RegistrarRelay(
		pallet_registrar_relay::Call::receive {
			message: registrar_primitives::MessageToRelay::V1(
				registrar_primitives::MessageToRelayV1::Register {
					para_id: fresh_id,
					message_id: 1,
					manager: manager.clone(),
					genesis_head: genesis_head.clone(),
					code_hash,
					code_len: code.len() as u32,
				},
			),
		},
	);

	let as_coretime = |call: crate::mock::network::relay::RuntimeCall| {
		xcm::VersionedXcm::<()>::from(Xcm(vec![
			UnpaidExecution { weight_limit: Unlimited, check_origin: None },
			Transact {
				origin_kind: OriginKind::Native,
				fallback_max_weight: None,
				call: call.encode().into(),
			},
		]))
		.encode()
	};

	rc.execute_with(|| {
		// GIVEN a chain whose control plane has moved — a fresh CT-driven registration only
		// happens in that world — and nothing on it knows this para id.
		RcMigrationStage::<Rc>::put(pallet_rc2_migrator::MigrationStage::MigrationDone);
		assert!(runtime_parachains::paras::Pallet::<Rc>::lifecycle(fresh).is_none());
		let _ = take_dmp(coretime);

		// WHEN Coretime requests the registration.
		enqueue_ump(coretime, vec![as_coretime(register)]);
		next_block_rc();
		assert!(
			pallet_registrar_relay::PendingRegistrations::<Rc>::contains_key(fresh_id),
			"the relay chain must be holding the authorization"
		);

		// Mark the code trusted first, which is what skips PVF pre-checking. Without this the
		// code needs a validator vote that no test can produce, and the *para* pays for it: at the
		// next session `groom_ongoing_pvf_votes` rejects the unconcluded vote and offboards it, so
		// the lifecycle goes to `None` rather than `Parathread`. Worth knowing beyond this test —
		// on the real chain pre-checking adds its own sessions before a para onboards.
		assert_ok!(crate::mock::network::relay::RuntimeCall::Paras(
			runtime_parachains::paras::Call::add_trusted_validation_code {
				validation_code: polkadot_primitives::ValidationCode(code.clone()),
			}
		)
		.dispatch(frame_system::RawOrigin::Root.into()));

		// WHEN the validation code is uploaded. Unsigned and feeless: the pending entry already
		// pins the exact bytes.
		assert_ok!(crate::mock::network::relay::RuntimeCall::RegistrarRelay(
			pallet_registrar_relay::Call::apply_authorized_code {
				para_id: fresh_id,
				validation_code: code.clone(),
			}
		)
		.dispatch(frame_system::RawOrigin::Authorized.into()));

		// THEN the para is registered and `Onboarding`, the registration is reported back, and —
		// the point — nothing has asked for any channel on its behalf.
		assert_eq!(
			runtime_parachains::paras::Pallet::<Rc>::lifecycle(fresh),
			Some(runtime_parachains::paras::ParaLifecycle::Onboarding),
		);
		assert!(!runtime_parachains::paras::Pallet::<Rc>::is_valid_para(fresh));
		assert!(
			!take_dmp(coretime).is_empty(),
			"the relay chain must report the registration back to Coretime"
		);
		let out = polkadot_primitives::HrmpChannelId { sender: coretime, recipient: fresh };
		let back = polkadot_primitives::HrmpChannelId { sender: fresh, recipient: coretime };
		for id in [&out, &back] {
			assert!(
				HrmpOpenChannelRequests::<Rc>::get(id).is_none(),
				"no control channel may be requested for a fresh registration"
			);
		}

		// WHEN the two session boundaries registration actually takes go by.
		let before = session_index_rc();
		rotate_session_rc();
		rotate_session_rc();
		assert_eq!(session_index_rc(), before + 2);

		// THEN the para is live, still with no channel anywhere — and none needed.
		assert!(
			runtime_parachains::paras::Pallet::<Rc>::is_valid_para(fresh),
			"the para must be live after SESSION_DELAY boundaries"
		);
		for id in [&out, &back] {
			assert!(HrmpChannels::<Rc>::get(id).is_none());
			assert!(HrmpOpenChannelRequests::<Rc>::get(id).is_none());
		}

		// WHEN the new para sends the exact message any para sends today — an unpaid `Transact`
		// of its own call — twice in one block.
		let ask = crate::mock::network::relay::RuntimeCall::Hrmp(runtime_parachains::hrmp::Call::<
			Rc,
		>::hrmp_init_open_channel {
			recipient: 2000.into(),
			proposed_max_capacity: 8,
			proposed_max_message_size: 1024,
		});
		let _ = take_dmp(coretime);
		enqueue_ump(fresh, vec![as_coretime(ask.clone()), as_coretime(ask)]);
		next_block_rc();

		// THEN exactly one request is forwarded to the control plane: the route works with no
		// further setup, and the second ask fell to the per-para per-block forward budget.
		assert_eq!(
			take_dmp(coretime).len(),
			1,
			"one forward per para per block: the first must go through, the second must not"
		);
		assert!(
			HrmpOpenChannelRequests::<Rc>::iter_keys().all(|id| id.sender != fresh),
			"the relay chain must forward the para's request, not act on it"
		);
	});
}

/// Probe: the relay chain's per-para channel caps, and who is closest to them.
///
/// Context for B25. The cap is one flat number applied to every para — HRMP's `is_system()` checks
/// only ever waive *deposits*, never the count — so a para whose channel count scales with the size
/// of the network has nowhere to grow.
#[tokio::test]
async fn channel_caps_and_who_is_near_them() {
	type Rc = crate::mock::network::relay::Runtime;
	let mut rc = load(Chain::Relay).await;

	rc.execute_with(|| {
		let config = runtime_parachains::configuration::ActiveConfig::<Rc>::get();
		println!(
			"{}: outbound cap {}, inbound cap {}",
			crate::mock::network::NAME,
			config.hrmp_max_parachain_outbound_channels,
			config.hrmp_max_parachain_inbound_channels,
		);

		let mut out = BTreeMap::<u32, usize>::new();
		let mut inn = BTreeMap::<u32, usize>::new();
		for id in HrmpChannels::<Rc>::iter_keys() {
			*out.entry(id.sender.into()).or_default() += 1;
			*inn.entry(id.recipient.into()).or_default() += 1;
		}

		let mut top: Vec<_> = out.iter().map(|(p, n)| (*n, *p)).collect();
		top.sort_unstable_by(|a, b| b.cmp(a));
		println!("busiest senders (para, outbound):");
		for (n, para) in top.iter().take(8) {
			println!("  {para}: {n} out, {} in", inn.get(para).copied().unwrap_or(0));
		}

		let total_paras = out.keys().chain(inn.keys()).collect::<BTreeSet<_>>().len();
		let coretime: u32 = CoretimePara::PARA_ID;
		println!(
			"paras with any channel: {total_paras}; coretime({coretime}) has {} out / {} in",
			out.get(&coretime).copied().unwrap_or(0),
			inn.get(&coretime).copied().unwrap_or(0),
		);

		// The caps stay a probe, not a constraint. The control plane holds **no** channel per
		// para — its link to each para is relayed through this chain (see `decisions.md`
		// 2026-08-31) — so its budget here is zero and no cap can price it out. This is what
		// closed B25: on this very snapshot Kusama could fit only 30 of the 107 channels the old
		// per-para design needed. The numbers keep printing so a future design that *does* want
		// per-para channels knows what it is up against.
		#[cfg(not(feature = "kusama"))]
		let cap = 128u32;
		#[cfg(feature = "kusama")]
		let cap = 30u32;
		assert_eq!(config.hrmp_max_parachain_outbound_channels, cap);
		assert_eq!(config.hrmp_max_parachain_inbound_channels, cap);
	});
}

/// Which XCM doors are open to a **non-system** parachain on the relay chain, before the
/// migration: unpaid is shut — including the control-plane shape, whose barrier only opens at
/// `MigrationDone` — and paid works out of the para's own sovereign account here.
///
/// This is the pre-migration half of B26. Post-migration the sovereign account is empty by design
/// ("Zero DOT on RC"), and what opens instead is `AllowUnpaidParaControlFrom`: exactly
/// `UnpaidExecution` + one `Transact { Native }` from a child para, rate-limited per para per
/// block, with the forwarded work priced on the Coretime chain. That half is driven end to end in
/// `full_migration_rc_to_ct` and in the fresh-registration test above. What this test pins is that
/// **nothing changes early**: on a snapshot that predates the migration, the new barrier arm must
/// refuse the very shape it will later admit.
#[tokio::test]
async fn a_non_system_para_can_only_reach_the_relay_chain_by_paying() {
	type Rc = crate::mock::network::relay::Runtime;
	let mut rc = load(Chain::Relay).await;

	let para: u32 = 2000;
	let call = crate::mock::network::relay::RuntimeCall::Hrmp(
		runtime_parachains::hrmp::Call::<Rc>::hrmp_init_open_channel {
			recipient: 2001.into(),
			proposed_max_capacity: 8,
			proposed_max_message_size: 1024,
		},
	);

	/// Service one UMP message and say whether the XCM executed at all.
	fn deliver(para: u32, msg: Vec<u8>) -> bool {
		enqueue_ump(para.into(), vec![msg]);
		let now = frame_system::Pallet::<Rc>::block_number() + 1;
		frame_system::Pallet::<Rc>::set_block_number(now);
		frame_system::Pallet::<Rc>::reset_events();
		<crate::mock::network::relay::MessageQueue as OnInitialize<_>>::on_initialize(now);
		frame_system::Pallet::<Rc>::events().into_iter().any(|record| {
			matches!(
				record.event,
				crate::mock::network::relay::RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				)
			)
		})
	}

	rc.execute_with(|| {
		let sovereign: AccountId32 = polkadot_primitives::Id::from(para).into_account_truncating();
		let funds = frame_system::Account::<Rc>::get(&sovereign).data.free;
		assert!(
			funds > 0,
			"this snapshot must predate the migration for the paid door to mean anything"
		);

		// The unpaid door is shut. Not "the call is refused" — the program never runs.
		assert!(
			!deliver(para, unpaid_transact_native(call.clone())),
			"a non-system para must not get unpaid execution here"
		);

		// The paid door is open, out of the para's own sovereign account on this chain — which is
		// exactly the balance the migration takes away.
		let fee: u128 = 1_000_000_000;
		let paid = xcm::VersionedXcm::<()>::from(Xcm(vec![
			WithdrawAsset((Location::here(), fee).into()),
			BuyExecution { fees: (Location::here(), fee).into(), weight_limit: Unlimited },
			Transact {
				origin_kind: OriginKind::Native,
				fallback_max_weight: None,
				call: call.encode().into(),
			},
		]))
		.encode();
		assert!(deliver(para, paid), "a non-system para must be able to pay its way in");
		// And it really was its own money.
		assert!(
			frame_system::Account::<Rc>::get(&sovereign).data.free < funds,
			"the fee must come out of the para's sovereign account here"
		);
	});
}

/// Before the migration, the relay chain serves HRMP itself — the forwarding seam is inert.
///
/// The PRD's Phase 1 rule for this chain is *"before-migrate-v2 — same as existing"*, and this is
/// what makes that concrete: the seam we added to `pallet_hrmp` reads the migration stage, so at
/// `Pending` it must take the local path and write local state, exactly as it always has. Nothing
/// should reach Coretime, whose pallets hold no state yet and would record a channel for a para it
/// has never heard of.
///
/// The complementary half — that the same call *forwards* once the migration is done — is asserted
/// at the end of `full_migration_rc_to_ct`, against the state the migration really produced.
#[tokio::test]
async fn before_the_migration_the_relay_chain_still_serves_hrmp_itself() {
	type Rc = crate::mock::network::relay::Runtime;
	let mut rc = load(Chain::Relay).await;

	rc.execute_with(|| {
		assert_eq!(
			RcMigrationStage::<Rc>::get(),
			pallet_rc2_migrator::MigrationStage::Pending,
			"the snapshot must predate the migration"
		);

		// Two live paras with no channel *and no pending request* between them, so the request is
		// a real one and not refused as a duplicate.
		let existing: BTreeSet<(u32, u32)> = HrmpChannels::<Rc>::iter_keys()
			.map(|id| (u32::from(id.sender), u32::from(id.recipient)))
			.chain(
				runtime_parachains::hrmp::HrmpOpenChannelRequests::<Rc>::iter_keys()
					.map(|id| (u32::from(id.sender), u32::from(id.recipient))),
			)
			.collect();
		// Non-system paras on both ends, so the deposit path is actually exercised: HRMP waives the
		// deposit whenever either end is a system chain.
		let live: Vec<u32> = HrmpChannels::<Rc>::iter_keys()
			.map(|id| u32::from(id.sender))
			.collect::<BTreeSet<_>>()
			.into_iter()
			.filter(|id| {
				*id >= 2000 &&
					runtime_parachains::paras::Pallet::<Rc>::is_valid_para(
						polkadot_primitives::Id::from(*id),
					)
			})
			.collect();
		let (sender, recipient) = live
			.iter()
			.flat_map(|a| live.iter().map(move |b| (*a, *b)))
			.find(|(a, b)| a != b && !existing.contains(&(*a, *b)))
			.expect("two live paras without a channel between them");

		// The sender pays the relay chain's own deposit on this path, so it has to be able to.
		// Funding it is setup: what is under test is which code path runs, not solvency.
		let sovereign: AccountId32 =
			polkadot_primitives::Id::from(sender).into_account_truncating();
		let _ =
			<pallet_balances::Pallet<Rc> as frame_support::traits::fungible::Mutate<_>>::mint_into(
				&sovereign,
				1_000_000_000_000_000,
			);
		let before = frame_system::Account::<Rc>::get(&sovereign).data.reserved;
		let _ = take_dmp(CoretimePara::PARA_ID.into());

		// The para asks, with the origin the XCM converter hands it.
		assert_ok!(crate::mock::network::relay::RuntimeCall::Hrmp(
			runtime_parachains::hrmp::Call::<Rc>::hrmp_init_open_channel {
				recipient: recipient.into(),
				proposed_max_capacity: 8,
				proposed_max_message_size: 1024,
			}
		)
		.dispatch(pallet_registrar_relay::Origin::Para(sender).into()));

		// THEN this chain recorded it, took the deposit, and told Coretime nothing.
		let id = polkadot_primitives::HrmpChannelId {
			sender: sender.into(),
			recipient: recipient.into(),
		};
		let request = runtime_parachains::hrmp::HrmpOpenChannelRequests::<Rc>::get(&id)
			.expect("the relay chain must record the request itself before the migration");
		// The relay chain's own configured deposit, taken on the relay chain — which is exactly
		// what stops being true once the control plane moves.
		let expected =
			runtime_parachains::configuration::ActiveConfig::<Rc>::get().hrmp_sender_deposit;
		assert_eq!(request.sender_deposit, expected);
		assert_eq!(
			frame_system::Account::<Rc>::get(&sovereign).data.reserved,
			before + expected,
			"the deposit must be reserved on the sender's sovereign account here"
		);
		assert!(
			take_dmp(CoretimePara::PARA_ID.into()).is_empty(),
			"nothing may be forwarded to Coretime before the migration"
		);
	});
}
