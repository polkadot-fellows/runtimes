//! Helper functions shared by the zombienet integration tests.

use std::{collections::HashMap, ops::Range, time::Duration};

use anyhow::anyhow;
use codec::Decode;
use polkadot_primitives::{CandidateReceiptV2, Id as ParaId};
use zombienet_sdk::{
	subxt::{
		config::polkadot::PolkadotExtrinsicParamsBuilder,
		events::Events,
		ext::scale_value::value,
		tx::{DynamicPayload, Signer, TxStatus},
		utils::H256,
		OnlineClient, PolkadotConfig,
	},
	subxt_signer,
};

/// Submit a `Sudo::sudo(Utility::batch(Coretime::assign_core { .. }))` call that
/// assigns each listed core to `para_id` with the full bulk ratio (57600 parts).
pub async fn assign_cores(
	relay_client: &OnlineClient<PolkadotConfig>,
	para_id: u32,
	cores: Vec<u32>,
) -> Result<(), anyhow::Error> {
	log::info!("Assigning cores {cores:?} to para_id {para_id}");

	let assign_calls: Vec<_> = cores
		.into_iter()
		.map(|core| {
			value! {
				Coretime(assign_core {
					core: core,
					begin: 0u32,
					assignment: ((Task(para_id), 57600u16)),
					end_hint: None()
				})
			}
		})
		.collect();

	let call = zombienet_sdk::subxt::tx::dynamic(
		"Sudo",
		"sudo",
		vec![value! { Utility(batch { calls: assign_calls }) }],
	);

	submit_and_wait_finalized(
		relay_client,
		&call,
		&subxt_signer::sr25519::dev::alice(),
		Duration::from_secs(60),
	)
	.await
}

/// Submit `call` signed by `signer` and wait for finalization (up to `timeout`).
pub async fn submit_and_wait_finalized<S>(
	client: &OnlineClient<PolkadotConfig>,
	call: &DynamicPayload,
	signer: &S,
	timeout: Duration,
) -> Result<(), anyhow::Error>
where
	S: Signer<PolkadotConfig>,
{
	let fut = async {
		let extensions = PolkadotExtrinsicParamsBuilder::new().immortal().build();
		let mut tx = client
			.tx()
			.create_signed(call, signer, extensions)
			.await?
			.submit_and_watch()
			.await?;

		while let Some(status) = tx.next().await.transpose()? {
			match status {
				TxStatus::InFinalizedBlock(tx_in_block) => {
					tx_in_block.wait_for_success().await?;
					log::info!("Extrinsic finalized in {:?}", tx_in_block.block_hash());
					return Ok::<(), anyhow::Error>(());
				},
				TxStatus::Error { message }
				| TxStatus::Invalid { message }
				| TxStatus::Dropped { message } => {
					return Err(anyhow!("tx failed: {message}"));
				},
				_ => continue,
			}
		}
		Err(anyhow!("tx stream ended before finalization"))
	};

	tokio::time::timeout(timeout, fut)
		.await
		.map_err(|_| anyhow!("timed out after {:?} waiting for extrinsic finalization", timeout))?
}

fn candidates_backed_for(
	events: &Events<PolkadotConfig>,
	para_id: u32,
) -> Result<u32, anyhow::Error> {
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

fn is_session_change(events: &Events<PolkadotConfig>) -> bool {
	events.iter().any(|e| {
		e.as_ref()
			.is_ok_and(|e| e.pallet_name() == "Session" && e.variant_name() == "NewSession")
	})
}

/// Count backed candidates per ParaId over `stop_after` finalized relay blocks, starting
/// after the first session change (parachains only start producing candidates once a
/// session boundary has been crossed). Session-change blocks themselves are skipped.
pub async fn assert_para_throughput(
	relay_client: &OnlineClient<PolkadotConfig>,
	stop_after: u32,
	expected: HashMap<ParaId, Range<u32>>,
) -> Result<(), anyhow::Error> {
	let mut blocks_sub = relay_client.blocks().subscribe_finalized().await?;
	let mut counts: HashMap<ParaId, u32> = HashMap::new();
	let mut seen_blocks = 0u32;

	log::info!("Waiting for the first session change");
	let mut saw_session_change = false;
	while !saw_session_change {
		let block = blocks_sub
			.next()
			.await
			.ok_or_else(|| anyhow!("relay block stream ended before first session change"))??;
		let events = block.events().await?;
		if is_session_change(&events) {
			log::info!("First session change at relay block #{}", block.number());
			saw_session_change = true;
		}
	}

	log::info!("Counting backed candidates over {stop_after} finalized relay blocks");

	while let Some(block) = blocks_sub.next().await {
		let block = block?;
		let events = block.events().await?;

		if is_session_change(&events) {
			continue;
		}

		seen_blocks += 1;
		for para_id in expected.keys() {
			let c = candidates_backed_for(&events, u32::from(*para_id))?;
			*counts.entry(*para_id).or_default() += c;
		}

		if seen_blocks >= stop_after {
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
