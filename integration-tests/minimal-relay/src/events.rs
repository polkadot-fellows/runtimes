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

//! JSONL event sink for the migration monitor.
//!
//! Opt-in via `AHM_EVENTS=<path>`: every produced block appends one line per interesting
//! runtime event plus one `state` probe line, so a frontend can replay the migration by folding
//! the file. The schema is shared with the (future) live capture daemon:
//!
//! ```json
//! {"seq":0,"chain":"rc","block":123,"kind":"stage","payload":{"old":"Pending","new":"..."}}
//! {"seq":1,"chain":"rc","block":123,"kind":"state","payload":{"stage":"...","ti":"...planck"}}
//! ```
//!
//! Balances are emitted as decimal strings in planck to keep the JSON integer-safe.

use crate::mock::Chain;
use frame_support::traits::{PalletInfoData, PalletsInfoAccess};
use serde_json::{json, Value};
use std::{
	collections::HashMap,
	fs::File,
	io::Write,
	sync::{
		atomic::{AtomicU64, Ordering},
		Mutex, OnceLock,
	},
};

static SINK: OnceLock<Option<Mutex<File>>> = OnceLock::new();
static SEQ: AtomicU64 = AtomicU64::new(0);

fn sink() -> Option<&'static Mutex<File>> {
	SINK.get_or_init(|| {
		let path = std::env::var("AHM_EVENTS").ok()?;
		let file = File::create(&path)
			.unwrap_or_else(|e| panic!("cannot create AHM_EVENTS file {path}: {e}"));
		println!("Writing migration events to {path}");
		Some(Mutex::new(file))
	})
	.as_ref()
}

/// Append one event line. No-op unless `AHM_EVENTS` is set.
pub fn emit(chain: &str, block: u32, kind: &str, payload: Value) {
	let Some(file) = sink() else { return };
	let line = json!({
		"seq": SEQ.fetch_add(1, Ordering::Relaxed),
		"chain": chain,
		"block": block,
		"kind": kind,
		"payload": payload,
	});
	let mut file = file.lock().unwrap();
	writeln!(file, "{line}").expect("can write event line");
}

fn planck(v: u128) -> Value {
	Value::String(v.to_string())
}

/// Debug-formatted stage, e.g. `AccountsOngoing { last_key: Some(...) }`. The frontend takes the
/// variant name from the prefix and the cursor detail is kept for humans.
fn stage_str<S: core::fmt::Debug>(stage: &S) -> String {
	format!("{stage:?}")
}

/// Emit a per-pallet storage census of the relay chain: one `storage_census` line per pallet
/// with its key count and total value bytes. Called before and after the migration so the
/// frontend can diff what each pallet held. Must run inside the RC `TestExternalities`.
pub fn emit_rc_census(phase: &str) {
	if sink().is_none() {
		return;
	}
	let block = frame_system::Pallet::<crate::mock::network::relay::Runtime>::block_number();
	census("rc", phase, block, crate::mock::network::relay::AllPalletsWithSystem::infos());
}

/// Like [`emit_rc_census`] for the Coretime chain.
pub fn emit_ct_census(phase: &str) {
	if sink().is_none() {
		return;
	}
	let block = frame_system::Pallet::<crate::mock::network::ct::Runtime>::block_number();
	census("ct", phase, block, crate::mock::network::ct::AllPalletsWithSystem::infos());
}

/// One pass over the whole top-level trie, bucketed by the 16-byte `twox128(pallet_name)`
/// prefix. Pallet names come from the runtime itself (`AllPalletsWithSystem`), so no list is
/// maintained by hand. Keys under no known pallet prefix (`:code`, retired pallets, …) are
/// aggregated as `(unattributed)`. Pallets with zero keys emit no line; the frontend treats
/// absence as zero.
fn census(chain: &str, phase: &str, block: u32, pallets: Vec<PalletInfoData>) {
	let mut by_prefix: HashMap<[u8; 16], (u64, u64)> = HashMap::new();
	let (mut other_keys, mut other_bytes) = (0u64, 0u64);

	let mut key = Vec::new();
	while let Some(next) = sp_io::storage::next_key(&key) {
		// Reading into an empty buffer returns the value's total length without copying it.
		let len = sp_io::storage::read(&next, &mut [], 0).unwrap_or(0) as u64;
		if next.len() >= 16 {
			let entry = by_prefix.entry(next[..16].try_into().expect("len checked")).or_default();
			entry.0 += 1;
			entry.1 += len;
		} else {
			other_keys += 1;
			other_bytes += len;
		}
		key = next;
	}

	for pallet in pallets {
		let prefix = sp_io::hashing::twox_128(pallet.name.as_bytes());
		if let Some((keys, bytes)) = by_prefix.remove(&prefix) {
			emit(
				chain,
				block,
				"storage_census",
				json!({ "phase": phase, "pallet": pallet.name, "keys": keys, "bytes": bytes }),
			);
		}
	}
	for (_, (keys, bytes)) in by_prefix {
		other_keys += keys;
		other_bytes += bytes;
	}
	if other_keys > 0 {
		emit(
			chain,
			block,
			"storage_census",
			json!({
				"phase": phase, "pallet": "(unattributed)",
				"keys": other_keys, "bytes": other_bytes,
			}),
		);
	}
}

/// Emit `fact` lines describing the RC state at the pre-migration block — one line per
/// measured fact: `{pallet, key, value}`, where DOT amounts are `{"planck": "..."}` objects
/// so the frontend can format them. Everything here is measured from state; grouping the
/// pallets into sections is the frontend's concern. Must run inside the RC
/// `TestExternalities`, at the last block before the migration is scheduled.
pub fn emit_pre_facts() {
	if sink().is_none() {
		return;
	}
	type Rc = crate::mock::network::relay::Runtime;
	let block = frame_system::Pallet::<Rc>::block_number();
	let fact = |pallet: &str, key: &str, value: Value| {
		emit("rc", block, "fact", json!({ "pallet": pallet, "key": key, "value": value }))
	};
	let dot = |v: u128| json!({ "planck": v.to_string() });

	// Balances: account population and the TI ↔ Σ-accounts gap (the phantom issuance).
	let (mut accounts, mut free, mut reserved) = (0u64, 0u128, 0u128);
	for (_, info) in frame_system::Account::<Rc>::iter() {
		accounts += 1;
		free += info.data.free;
		reserved += info.data.reserved;
	}
	let ti = pallet_balances::TotalIssuance::<Rc>::get();
	fact("Balances", "accounts", json!(accounts));
	fact("Balances", "Σ free", dot(free));
	fact("Balances", "Σ reserved", dot(reserved));
	fact("Balances", "TI − Σ accounts", dot(ti.saturating_sub(free + reserved)));
	fact("Balances", "inactive issuance", dot(pallet_balances::InactiveIssuance::<Rc>::get()));

	// Proxy: population and the deposit split.
	let (mut delegators, mut defs, mut deposits, mut zero_dep) = (0u64, 0u64, 0u128, 0u64);
	for (_, (list, deposit)) in pallet_proxy::Proxies::<Rc>::iter() {
		delegators += 1;
		defs += list.len() as u64;
		deposits += deposit;
		if deposit == 0 {
			zero_dep += 1;
		}
	}
	fact("Proxy", "delegators", json!(delegators));
	fact("Proxy", "definitions", json!(defs));
	fact("Proxy", "deposits", dot(deposits));
	fact("Proxy", "zero-deposit entries", json!(zero_dep));
	fact("Proxy", "live-deposit entries", json!(delegators - zero_dep));
	fact("Proxy", "announcements", json!(pallet_proxy::Announcements::<Rc>::iter_keys().count()));

	// Registrar: recorded deposits vs what the managers actually hold in reserve.
	let mut paras = 0u64;
	let mut recorded = 0u128;
	let mut by_manager: HashMap<_, u128> = HashMap::new();
	let (mut ghost_records, mut ghost_amount) = (0u64, 0u128);
	for (_, info) in polkadot_runtime_common::paras_registrar::Paras::<Rc>::iter() {
		paras += 1;
		recorded += info.deposit;
		*by_manager.entry(info.manager.clone()).or_default() += info.deposit;
		if frame_system::Account::<Rc>::get(&info.manager).data.reserved == 0 {
			ghost_records += 1;
			ghost_amount += info.deposit;
		}
	}
	let backed: u128 = by_manager
		.iter()
		.map(|(m, rec)| (*rec).min(frame_system::Account::<Rc>::get(m).data.reserved))
		.sum();
	fact("Registrar", "paras", json!(paras));
	fact("Registrar", "recorded deposits", dot(recorded));
	fact("Registrar", "backed by manager reserves", dot(backed));
	fact("Registrar", "records on zero-reserve managers", json!(ghost_records));
	fact("Registrar", "recorded on zero-reserve managers", dot(ghost_amount));

	// HRMP: channels and pending requests, with their recorded deposits.
	let (mut channels, mut channel_dep) = (0u64, 0u128);
	for (_, ch) in runtime_parachains::hrmp::HrmpChannels::<Rc>::iter() {
		channels += 1;
		channel_dep += ch.sender_deposit + ch.recipient_deposit;
	}
	let (mut requests, mut request_dep) = (0u64, 0u128);
	for (_, req) in runtime_parachains::hrmp::HrmpOpenChannelRequests::<Rc>::iter() {
		requests += 1;
		request_dep += req.sender_deposit;
	}
	fact("Hrmp", "open channels", json!(channels));
	fact("Hrmp", "channel deposits", dot(channel_dep));
	fact("Hrmp", "pending open requests", json!(requests));
	fact("Hrmp", "request deposits", dot(request_dep));

	// Crowdloan: what, if anything, is left to wind down.
	fact(
		"Crowdloan",
		"funds",
		json!(polkadot_runtime_common::crowdloan::Funds::<Rc>::iter_keys().count()),
	);
}

/// Emit the events and state probe of the relay-chain block that was just produced.
/// Must run inside the RC `TestExternalities`.
pub fn emit_rc_block() {
	if sink().is_none() {
		return;
	}
	type Rc = crate::mock::network::relay::Runtime;
	use pallet_rc2_migrator::Event as MigEvent;

	let block = frame_system::Pallet::<Rc>::block_number();
	let e = |kind: &str, payload: Value| emit("rc", block, kind, payload);
	for record in frame_system::Pallet::<Rc>::events() {
		match record.event {
			crate::mock::network::relay::RuntimeEvent::Rc2Migrator(ev) => match ev {
				MigEvent::StageTransition { old, new } =>
					e("stage", json!({ "old": stage_str(&old), "new": stage_str(&new) })),
				MigEvent::AccountsBatchSent { count } =>
					e("accounts_batch_sent", json!({ "count": count })),
				MigEvent::DepositRefunded { who, amount } => e(
					"deposit_refunded",
					json!({ "who": format!("{who:?}"), "amount": planck(amount) }),
				),
				MigEvent::ProxyBatchSent { count } =>
					e("proxy_batch_sent", json!({ "count": count })),
				MigEvent::AccountSwept { who, amount } => e(
					"account_swept",
					json!({ "who": format!("{who:?}"), "amount": planck(amount) }),
				),
				MigEvent::DustSwept { count, amount } =>
					e("dust_swept", json!({ "count": count, "amount": planck(amount) })),
				MigEvent::HrmpRequestsSent { count } =>
					e("hrmp_requests_sent", json!({ "count": count })),
				MigEvent::AccountsTeleported { count, amount } =>
					e("accounts_teleported", json!({ "count": count, "amount": planck(amount) })),
				MigEvent::UnattributedReserve { who, amount } => e(
					"unattributed_reserve",
					json!({ "who": format!("{who:?}"), "amount": planck(amount) }),
				),
				MigEvent::AccountShellDrained { who, amount } => e(
					"account_shell_drained",
					json!({ "who": format!("{who:?}"), "amount": planck(amount) }),
				),
				MigEvent::HusksReaped { count } => e("husks_reaped", json!({ "count": count })),
				MigEvent::TiCorrected { expected, unaccounted, burned } => e(
					"ti_corrected",
					json!({
						"expected": planck(expected),
						"unaccounted": planck(unaccounted),
						"burned": planck(burned),
					}),
				),
				MigEvent::TiCorrectionAnomaly { expected, unaccounted } => e(
					"ti_correction_anomaly",
					json!({ "expected": planck(expected), "unaccounted": planck(unaccounted) }),
				),
				MigEvent::RegistrarBatchSent { count } =>
					e("registrar_batch_sent", json!({ "count": count })),
				MigEvent::HrmpBatchSent { count } =>
					e("hrmp_batch_sent", json!({ "count": count })),
				_ => (),
			},
			crate::mock::network::relay::RuntimeEvent::MessageQueue(
				pallet_message_queue::Event::Processed { success, .. },
			) if !success => e("mq_failed", json!({})),
			_ => (),
		}
	}

	let tracker = pallet_rc2_migrator::RcMigratedBalance::<Rc>::get();
	e(
		"state",
		json!({
			"stage": stage_str(&pallet_rc2_migrator::RcMigrationStage::<Rc>::get()),
			"ti": planck(pallet_balances::TotalIssuance::<Rc>::get()),
			"kept": planck(tracker.kept),
			"ct_reserved": planck(tracker.ct_reserved),
			"ct_free": planck(tracker.ct_free),
			"ah_free": planck(tracker.ah_free),
			"ti_corrected": planck(tracker.ti_corrected),
			"paras": polkadot_runtime_common::paras_registrar::Paras::<Rc>::iter_keys().count(),
			"hrmp_channels": runtime_parachains::hrmp::HrmpChannels::<Rc>::iter_keys().count(),
			"proxies": pallet_proxy::Proxies::<Rc>::iter_keys().count(),
		}),
	);
}

/// Emit the events and state probe of the parachain block that was just produced.
/// Must run inside that chain's `TestExternalities`.
pub fn emit_para_block(chain: Chain) {
	if sink().is_none() {
		return;
	}
	match chain {
		Chain::Coretime => emit_ct_block(),
		// Only a light state probe for AH; it is not a destination of this migration's data.
		Chain::AssetHub => {
			type Ah = crate::mock::network::ah::Runtime;
			let block = frame_system::Pallet::<Ah>::block_number();
			// The checking account is AH's ledger of "DOT out on the relay chain"; teleports
			// from the RC drain it, so its delta is the AH-side receipt confirmation.
			let checking = pallet_xcm::Pallet::<Ah>::check_account();
			let checking_balance = frame_system::Account::<Ah>::get(&checking).data.free;
			emit(
				"ah",
				block,
				"state",
				json!({
					"ti": planck(pallet_balances::TotalIssuance::<Ah>::get()),
					"checking": planck(checking_balance),
				}),
			);
		},
		Chain::Relay => unreachable!("relay blocks go through emit_rc_block"),
	}
}

fn emit_ct_block() {
	type Ct = crate::mock::network::ct::Runtime;
	use pallet_ct_migrator::Event as MigEvent;

	let block = frame_system::Pallet::<Ct>::block_number();
	let e = |kind: &str, payload: Value| emit("ct", block, kind, payload);
	for record in frame_system::Pallet::<Ct>::events() {
		match record.event {
			crate::mock::network::ct::RuntimeEvent::CtMigrator(ev) => match ev {
				MigEvent::StageTransition { old, new } =>
					e("stage", json!({ "old": stage_str(&old), "new": stage_str(&new) })),
				MigEvent::AccountsReceived { count_good, count_bad } =>
					e("accounts_received", json!({ "good": count_good, "bad": count_bad })),
				MigEvent::RegistrarReceived { count_good, count_bad } =>
					e("registrar_received", json!({ "good": count_good, "bad": count_bad })),
				MigEvent::DepositShortfallParked { para_id, shortfall } => e(
					"deposit_shortfall_parked",
					json!({ "para_id": para_id, "shortfall": planck(shortfall) }),
				),
				MigEvent::HrmpReceived { count_good, count_bad } =>
					e("hrmp_received", json!({ "good": count_good, "bad": count_bad })),
				MigEvent::ProxiesReceived { count_good, count_bad } =>
					e("proxies_received", json!({ "good": count_good, "bad": count_bad })),
				MigEvent::HrmpRequestsReceived { count } =>
					e("hrmp_requests_received", json!({ "count": count })),
				MigEvent::HrmpShortfallParked { sender, recipient, shortfall } => e(
					"hrmp_shortfall_parked",
					json!({
						"sender": sender,
						"recipient": recipient,
						"shortfall": planck(shortfall),
					}),
				),
				MigEvent::MigrationFinished { rc_kept, rc_migrated, ct_minted } => e(
					"migration_finished",
					json!({
						"rc_kept": planck(rc_kept),
						"rc_migrated": planck(rc_migrated),
						"ct_minted": planck(ct_minted),
					}),
				),
				// Which queue went first this block is not migration state; the stream carries
				// the stage, not the service ring.
				MigEvent::DmpQueuePrioritised { .. } |
				MigEvent::DmpQueuePriorityConfigSet { .. } => (),
			},
			crate::mock::network::ct::RuntimeEvent::MessageQueue(
				pallet_message_queue::Event::Processed { success, .. },
			) if !success => e("mq_failed", json!({})),
			_ => (),
		}
	}

	e(
		"state",
		json!({
			"stage": stage_str(&pallet_ct_migrator::CtMigrationStage::<Ct>::get()),
			"ti": planck(pallet_balances::TotalIssuance::<Ct>::get()),
			"minted": planck(pallet_ct_migrator::CtMintedTotal::<Ct>::get()),
			"reattributed": planck(pallet_ct_migrator::ReattributedDeposits::<Ct>::get()),
			"reattributed_hrmp": planck(pallet_ct_migrator::ReattributedHrmpDeposits::<Ct>::get()),
			// Read from the pallets that own the state, not from the migrator: it hands records
			// over rather than parking them, so counting its storage would report zero forever.
			// A channel the relay chain already had arrives `Open`; an unconfirmed request
			// arrives `Pending`, which is the same split the relay-chain census reports.
			"paras": pallet_registrar_para::Paras::<Ct>::iter_keys().count(),
			"hrmp_channels": pallet_hrmp_para::Channels::<Ct>::iter_values()
				.filter(|c| matches!(c.state, pallet_hrmp_para::ChannelState::Open))
				.count(),
			"hrmp_requests": pallet_hrmp_para::Channels::<Ct>::iter_values()
				.filter(|c| !matches!(c.state, pallet_hrmp_para::ChannelState::Open))
				.count(),
			"failed_accounts": pallet_ct_migrator::FailedAccounts::<Ct>::iter_keys().count(),
			"failed_paras": pallet_ct_migrator::FailedParas::<Ct>::iter_keys().count(),
			"failed_hrmp": pallet_ct_migrator::FailedHrmpChannels::<Ct>::iter_keys().count(),
			"failed_proxies": pallet_ct_migrator::FailedProxies::<Ct>::iter_keys().count(),
			"parked_shortfalls":
				pallet_ct_migrator::ParkedDepositShortfalls::<Ct>::iter_keys().count(),
			"parked_hrmp_shortfalls":
				pallet_ct_migrator::ParkedHrmpShortfalls::<Ct>::iter_keys().count(),
		}),
	);
}
