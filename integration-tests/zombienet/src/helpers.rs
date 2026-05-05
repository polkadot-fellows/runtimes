//! Throughput helper used by the elastic scaling integration test.
//!
//! Mirrors `cumulus-zombienet-sdk-helpers::assert_para_throughput` from polkadot-sdk;
//! kept local because pulling that crate as a git dep drags in polkadot-sdk's whole
//! workspace, whose pinned crate versions conflict with this repo's.

use std::{collections::HashMap, ops::Range};

use anyhow::anyhow;
use codec::Decode;
use polkadot_primitives::{CandidateReceiptV2, Id as ParaId};
use zombienet_sdk::subxt::{events::Events, utils::H256, OnlineClient, PolkadotConfig};

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
