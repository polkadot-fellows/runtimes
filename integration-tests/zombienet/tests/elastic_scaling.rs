//! Elastic scaling integration tests for the system parachains.
//!
//! Each test asserts ≈3 backed candidates per 6s relay block (~60 over 20 RCBs).
//!
//! The two cases (Asset Hub Polkadot and People Polkadot) live as **separate**
//! `#[tokio::test]` functions and are serialised via `#[serial]` so that the
//! second case starts on a host that has fully released the first case's
//! validator/collator processes (zombienet's `Network` has no `Drop` impl, so we
//! call `network.destroy().await` explicitly at the end of each case).

use std::collections::HashMap;

use anyhow::anyhow;
use polkadot_primitives::Id as ParaId;
use serial_test::serial;
use zombienet_sdk::subxt::{OnlineClient, PolkadotConfig};
use zombienet_sdk_tests::{
	elastic_scaling_network,
	environment::{get_provider_from_env, get_spawn_fn},
	helpers::{assert_para_throughput, wait_for_pvf_prepared},
	ElasticNetwork, ASSET_HUB_POLKADOT_PARA_ID, ELASTIC_VALIDATOR_0, ELASTIC_VALIDATORS,
	PEOPLE_POLKADOT_PARA_ID,
};

fn init_tracing() {
	// `try_init` so the second test in the same process doesn't panic.
	let _ = tracing_subscriber::fmt()
		.with_env_filter(
			tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
		)
		.try_init();
}

async fn run(
	chain: &'static str,
	para_id: u32,
	collators: &'static [&'static str],
	expected: std::ops::Range<u32>,
) -> Result<(), anyhow::Error> {
	init_tracing();
	log::info!("Using zombienet provider: {:?}", get_provider_from_env());
	log::info!("running elastic scaling test for chain '{chain}' (para_id {para_id})");

	let config = elastic_scaling_network(ElasticNetwork { chain, para_id, collators })
		.map_err(|e| anyhow!("{e}"))?;
	let network = (get_spawn_fn())(config).await?;

	let relay_client: OnlineClient<PolkadotConfig> =
		network.get_node(ELASTIC_VALIDATOR_0)?.wait_client().await?;

	let first_collator = network.get_node(collators[0])?;
	assert!(
		first_collator.wait_until_is_up(120u64).await.is_ok(),
		"collator {} failed to come up",
		collators[0]
	);

	wait_for_pvf_prepared(&network, ELASTIC_VALIDATORS, 1, 300).await?;

	let measurement_result = assert_para_throughput(
		&relay_client,
		20,
		HashMap::from([(ParaId::from(para_id), expected.clone())]),
	)
	.await;

	// Explicitly tear down — `Network` has no `Drop` impl, so the spawned
	// validator/collator processes would otherwise leak and the next test would
	// inherit the host load.
	if let Err(e) = network.destroy().await {
		log::warn!("network.destroy() failed for chain '{chain}': {e}");
	}

	measurement_result?;
	log::info!("🚀 elastic scaling test passed for chain '{chain}'");
	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn elastic_scaling_asset_hub_polkadot() -> Result<(), anyhow::Error> {
	run(
		"asset-hub-polkadot-local",
		ASSET_HUB_POLKADOT_PARA_ID,
		&["asset-hub-collator-0", "asset-hub-collator-1", "asset-hub-collator-2"],
		45..61,
	)
	.await
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn elastic_scaling_people_polkadot() -> Result<(), anyhow::Error> {
	run(
		"people-polkadot-local",
		PEOPLE_POLKADOT_PARA_ID,
		&["people-collator-0", "people-collator-1", "people-collator-2"],
		45..61,
	)
	.await
}
