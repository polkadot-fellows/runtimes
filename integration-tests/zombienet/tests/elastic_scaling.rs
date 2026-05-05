//! Elastic scaling integration test for the system parachains.
//! Each `Case` spawns a `polkadot-local` relay plus the target parachain running
//! slot-based authoring with 3 collators. `ParachainConfigBuilder::with_num_cores(3)`
//! seeds 3 cores assigned to the parachain at genesis, sidestepping the runtime
//! `Coretime::assign_core` path (Polkadot relay has no `pallet_sudo`).
//!
//! The test asserts ≈3 backed candidates per 6s relay block (~60 over 20 RCBs).

use std::collections::HashMap;

use anyhow::anyhow;
use polkadot_primitives::Id as ParaId;
use zombienet_sdk::subxt::{OnlineClient, PolkadotConfig};
use zombienet_sdk_tests::{
	elastic_scaling_network,
	environment::{get_provider_from_env, get_spawn_fn},
	helpers::assert_para_throughput,
	ElasticNetwork, ASSET_HUB_POLKADOT_PARA_ID, ELASTIC_VALIDATOR_0, PEOPLE_POLKADOT_PARA_ID,
};

struct Case {
	chain: &'static str,
	para_id: u32,
	collators: &'static [&'static str],
	/// Acceptable count of backed candidates over 20 RCBs with velocity-3 elastic scaling.
	expected: std::ops::Range<u32>,
}

const CASES: &[Case] = &[
	Case {
		chain: "asset-hub-polkadot-local",
		para_id: ASSET_HUB_POLKADOT_PARA_ID,
		collators: &["asset-hub-collator-0", "asset-hub-collator-1", "asset-hub-collator-2"],
		expected: 45..61,
	},
	Case {
		chain: "people-polkadot-local",
		para_id: PEOPLE_POLKADOT_PARA_ID,
		collators: &["people-collator-0", "people-collator-1", "people-collator-2"],
		expected: 45..61,
	},
];

async fn run(case: &Case) -> Result<(), anyhow::Error> {
	log::info!("running elastic scaling test for chain '{}' (para_id {})", case.chain, case.para_id);

	let config = elastic_scaling_network(ElasticNetwork {
		chain: case.chain,
		para_id: case.para_id,
		collators: case.collators,
	})
	.map_err(|e| anyhow!("{e}"))?;
	let network = (get_spawn_fn())(config).await?;

	let relay_client: OnlineClient<PolkadotConfig> =
		network.get_node(ELASTIC_VALIDATOR_0)?.wait_client().await?;

	let first_collator = network.get_node(case.collators[0])?;
	assert!(
		first_collator.wait_until_is_up(120u64).await.is_ok(),
		"collator {} failed to come up",
		case.collators[0]
	);

	assert_para_throughput(
		&relay_client,
		20,
		HashMap::from([(ParaId::from(case.para_id), case.expected.clone())]),
	)
	.await?;

	log::info!("🚀 elastic scaling test passed for chain '{}'", case.chain);
	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn elastic_scaling() -> Result<(), anyhow::Error> {
	tracing_subscriber::fmt()
		.with_env_filter(
			tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
		)
		.init();

	log::info!("Using zombienet provider: {:?}", get_provider_from_env());

	for case in CASES {
		run(case).await?;
	}
	Ok(())
}
