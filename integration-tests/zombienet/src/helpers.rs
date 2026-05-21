//! Throughput helper used by the elastic scaling integration test.
//!
//! Mirrors `cumulus-zombienet-sdk-helpers::assert_para_throughput` from polkadot-sdk;
//! kept local because pulling that crate as a git dep drags in polkadot-sdk's whole
//! workspace, whose pinned crate versions conflict with this repo's.

use std::{collections::HashMap, ops::Range};

use anyhow::anyhow;
use codec::Decode;
use polkadot_primitives::{CandidateReceiptV2, Id as ParaId};
use tokio::join;
use zombienet_sdk::{
	subxt::{events::Events, utils::H256, OnlineClient, PolkadotConfig},
	LocalFileSystem, Network,
};

/// Wait until every named validator has prepared at least `min_prepared` PVF
/// artifacts (i.e. `polkadot_pvf_prepare_concluded >= min_prepared`).
pub async fn wait_for_pvf_prepared(
	network: &Network<LocalFileSystem>,
	validators: &[&str],
	min_prepared: u64,
	timeout_secs: u64,
) -> Result<(), anyhow::Error> {
	let threshold = min_prepared as f64;
	for v in validators {
		let node = network.get_node(*v)?;
		log::info!("Waiting for {v} to finish PVF preparation...");
		node.wait_metric_with_timeout(
			"polkadot_pvf_prepare_concluded",
			|c| c >= threshold,
			timeout_secs,
		)
		.await
		.map_err(|e| anyhow!("{v}: PVF prepare did not conclude within timeout: {e}"))?;
	}
	log::info!("All validators have prepared at least {min_prepared} PVF artifact(s)");
	Ok(())
}

/// Assert that the relay's finality lag (best − finalized) is within `maximum_lag`
/// blocks at the moment of the call. Mirrors `cumulus-zombienet-sdk-helpers`.
pub async fn assert_finality_lag(
	client: &OnlineClient<PolkadotConfig>,
	maximum_lag: u32,
) -> Result<(), anyhow::Error> {
	let mut best_stream = client.blocks().subscribe_best().await?;
	let mut fin_stream = client.blocks().subscribe_finalized().await?;
	let (Some(Ok(best)), Some(Ok(finalized))) = join!(best_stream.next(), fin_stream.next()) else {
		return Err(anyhow!("unable to fetch best and finalized blocks"));
	};
	let lag = best.number().saturating_sub(finalized.number());
	log::info!("Finality lag: {lag} blocks (max allowed: {maximum_lag})");
	if lag > maximum_lag {
		return Err(anyhow!(
			"finality lag {lag} exceeds maximum {maximum_lag}: best #{}, finalized #{}",
			best.number(),
			finalized.number(),
		));
	}
	Ok(())
}

fn is_session_change(events: &Events<PolkadotConfig>) -> bool {
	events.iter().any(|e| {
		e.as_ref()
			.is_ok_and(|e| e.pallet_name() == "Session" && e.variant_name() == "NewSession")
	})
}

fn count_backed_for(events: &Events<PolkadotConfig>, para_id: u32) -> Result<u32, anyhow::Error> {
	let mut count = 0u32;
	for ev in events.iter() {
		let ev = ev?;
		if ev.pallet_name() == "ParaInclusion" && ev.variant_name() == "CandidateBacked" {
			let receipt = CandidateReceiptV2::<H256>::decode(&mut &ev.field_bytes()[..])?;
			if u32::from(receipt.descriptor.para_id()) == para_id {
				count += 1;
			}
		}
	}
	Ok(count)
}

/// Count `ParaInclusion::CandidateBacked` events per `ParaId` over `stop_after`
/// finalized relay blocks, starting after the first session change. Session-change
/// blocks are skipped (they never carry backed candidates).
pub async fn assert_para_throughput(
	relay_client: &OnlineClient<PolkadotConfig>,
	stop_after: u32,
	expected: HashMap<ParaId, Range<u32>>,
) -> Result<(), anyhow::Error> {
	let mut blocks = relay_client.blocks().subscribe_finalized().await?;
	let mut counts: HashMap<ParaId, u32> = HashMap::new();

	log::info!("Waiting for the first session change");
	loop {
		let block = blocks
			.next()
			.await
			.ok_or_else(|| anyhow!("relay block stream ended before first session change"))??;
		let events = block.events().await?;
		if is_session_change(&events) {
			log::info!("First session change at relay block #{}", block.number());
			break;
		}
	}

	log::info!("Counting backed candidates over {stop_after} finalized relay blocks");
	let mut seen = 0u32;
	while let Some(block) = blocks.next().await {
		let events = block?.events().await?;
		if is_session_change(&events) {
			continue;
		}
		seen += 1;
		for para_id in expected.keys() {
			*counts.entry(*para_id).or_default() += count_backed_for(&events, u32::from(*para_id))?;
		}
		if seen >= stop_after {
			break;
		}
	}

	log::info!("Throughput per para: {counts:?}");
	for (para_id, range) in expected {
		let actual = counts.get(&para_id).copied().unwrap_or(0);
		if !range.contains(&actual) {
			return Err(anyhow!(
				"para_id {para_id}: {actual} backed candidates not in expected range {range:?}"
			));
		}
	}
	Ok(())
}
