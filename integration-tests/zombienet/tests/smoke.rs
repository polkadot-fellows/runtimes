use std::time::Instant;
use zombienet_sdk::subxt::{
	dynamic::{self, At, DecodedValueThunk, Value},
	ext::futures::StreamExt,
	OnlineClient, PolkadotConfig,
};
use zombienet_sdk_tests::{
	environment::{get_images_from_env, get_provider_from_env, get_spawn_fn, Provider},
	small_network, COLLATOR_1005, COLLATOR_1006_1,
};

fn dump_provider_and_versions() {
	let provider = get_provider_from_env();
	log::info!("Using zombienet provider: {provider:?}");

	if let Provider::Docker = provider {
		let images = get_images_from_env();

		for image in [images.polkadot, images.cumulus] {
			let output = std::process::Command::new("docker")
				.arg("run")
				.arg(image.clone())
				.arg("--version")
				.output()
				.expect("Failed to execute command");

			if output.status.success() {
				let stdout = String::from_utf8_lossy(&output.stdout);
				log::info!("{} binary version: {}", image, stdout.trim());
			} else {
				let stderr = String::from_utf8_lossy(&output.stderr);
				log::error!("Error: {stderr}");
			}
		}
	}
}

async fn chain_label(client: &OnlineClient<PolkadotConfig>) -> Result<String, anyhow::Error> {
	let call = dynamic::runtime_api_call("Core", "version", Vec::<Value>::new());
	let result: DecodedValueThunk = client.runtime_api().at_latest().await?.call(call).await?;
	let value = result.to_value()?;
	let spec_name = value.at("spec_name").unwrap().at(0).unwrap().as_str().unwrap();
	let spec_version = value.at("spec_version").unwrap().as_u128().unwrap();
	Ok(format!("{spec_name}@{spec_version}"))
}

#[tokio::test(flavor = "multi_thread")]
async fn smoke() -> Result<(), anyhow::Error> {
	tracing_subscriber::fmt()
		.with_env_filter(
			tracing_subscriber::EnvFilter::try_from_default_env()
				.unwrap_or_else(|_| "info".into()),
		)
		.init();

	// config and env
	dump_provider_and_versions();
	let spawn_fn = get_spawn_fn();
	let config = small_network().unwrap();

	// spawn
	let now = Instant::now();
	let network = spawn_fn(config).await?;
	let elapsed = now.elapsed();
	log::info!("🚀🚀🚀🚀 network deployed in {elapsed:.2?}");

	// prevent delete on drop
	network.detach().await;

	let alice = network.get_node("alice")?;
	// wait until the subxt client is ready
	let alice_client: OnlineClient<PolkadotConfig> = alice.wait_client().await?;

	// wait 10 blocks
	let alice_chain_label = chain_label(&alice_client).await?;
	let mut blocks = alice_client.blocks().subscribe_finalized().await.unwrap().take(10);

	let mut now = Instant::now();
	while let Some(block) = blocks.next().await {
		log::info!(
			"{alice_chain_label} Block #{} in {} seconds",
			block.unwrap().header().number,
			now.elapsed().as_secs()
		);
		now = Instant::now();
	}

	// wait 10 blocks on each parachain
	for node_name in [COLLATOR_1005, COLLATOR_1006_1] {
		let collator = network.get_node(node_name)?;
		let collator_client: OnlineClient<PolkadotConfig> = collator.wait_client().await?;

		let collator_chain_label = chain_label(&collator_client).await?;

		let mut blocks = collator_client.blocks().subscribe_finalized().await.unwrap().take(10);
		let mut now = Instant::now();
		while let Some(block) = blocks.next().await {
			log::info!(
				"{collator_chain_label} Block #{} in {} seconds",
				block.unwrap().header().number,
				now.elapsed().as_secs()
			);
			now = Instant::now();
		}
	}

	Ok(())
}
