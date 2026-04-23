//! Elastic scaling integration test for **Asset Hub Polkadot** (para_id 1000).
//!
//! Spawns a `polkadot-local` relay chain with 3 bulk cores plus `asset-hub-polkadot-local`
//! running slot-based authoring with 3 collators, then asserts that assigning two extra
//! cores via `Coretime::assign_core` increases backed-candidate throughput by 3x.
use std::collections::HashMap;

use anyhow::anyhow;
use polkadot_primitives::Id as ParaId;
use zombienet_sdk::subxt::{OnlineClient, PolkadotConfig};
use zombienet_sdk_tests::{
	elastic_scaling_network,
	environment::{get_provider_from_env, get_spawn_fn},
	helpers::{assert_para_throughput, assign_cores},
	ElasticNetwork, ASSET_HUB_POLKADOT_PARA_ID, ELASTIC_VALIDATOR_0,
};

const COLLATORS: &[&str] =
	&["asset-hub-collator-0", "asset-hub-collator-1", "asset-hub-collator-2"];

#[tokio::test(flavor = "multi_thread")]
async fn elastic_scaling_asset_hub_polkadot() -> Result<(), anyhow::Error> {
	tracing_subscriber::fmt()
		.with_env_filter(
			tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
		)
		.init();

	log::info!("Using zombienet provider: {:?}", get_provider_from_env());

	let spawn_fn = get_spawn_fn();
	let config = elastic_scaling_network(ElasticNetwork {
		chain: "asset-hub-polkadot-local",
		para_id: ASSET_HUB_POLKADOT_PARA_ID,
		collators: COLLATORS,
	})
	.map_err(|e| anyhow!("{e}"))?;
	let network = spawn_fn(config).await?;
	network.detach().await;

	let relay_node = network.get_node(ELASTIC_VALIDATOR_0)?;
	let relay_client: OnlineClient<PolkadotConfig> = relay_node.wait_client().await?;

	let first_collator = network.get_node(COLLATORS[0])?;
	assert!(
		first_collator.wait_until_is_up(120u64).await.is_ok(),
		"elastic collator failed to come up"
	);

	// ~1 candidate per 6s relay block, so over 10 RCBs we expect ~10 total.
	log::info!("Measuring baseline throughput with the default single core");
	assert_para_throughput(
		&relay_client,
		10,
		HashMap::from([(ParaId::from(ASSET_HUB_POLKADOT_PARA_ID), 9..11)]),
	)
	.await?;

	// Scale up to 3 cores.
	assign_cores(&relay_client, ASSET_HUB_POLKADOT_PARA_ID, vec![0, 1]).await?;

	// With 3 cores, expect close to 3 candidates per 6s relay slot — ~60
	// over 20 relay blocks.
	log::info!("Measuring throughput after scaling up to 3 cores");
	assert_para_throughput(
		&relay_client,
		20,
		HashMap::from([(ParaId::from(ASSET_HUB_POLKADOT_PARA_ID), 45..61)]),
	)
	.await?;

	log::info!("🚀 asset-hub-polkadot elastic scaling test passed");
	Ok(())
}
