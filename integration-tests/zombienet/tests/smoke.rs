use std::time::Instant;
use subxt::{ext::futures::StreamExt, OnlineClient, PolkadotConfig};
use zombienet_sdk_tests::small_network;

fn dump_provider_and_versions() {
	use zombienet_sdk::environment::Provider;
	let provider = zombienet_sdk::environment::get_provider_from_env();
	log::info!(
		"Using zombienet provider: {:?}",
		match provider {
			Provider::Native => "Native",
			Provider::K8s => "K8s",
			Provider::Docker => "Docker",
		}
	);

	if let Provider::Docker = provider {
		let images = zombienet_sdk::environment::get_images_from_env();
		log::info!("Using Docker images: {:?}", images);

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
				log::error!("Error: {}", stderr);
			}
		}
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn smoke() -> Result<(), anyhow::Error> {
	let _ = env_logger::try_init_from_env(
		env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
	);

	// config and env
	dump_provider_and_versions();
	let spawn_fn = zombienet_sdk::environment::get_spawn_fn();
	let config = small_network()?;

	// spawn
	let now = Instant::now();
	let network = spawn_fn(config).await?;
	let elapsed = now.elapsed();
	log::info!("🚀🚀🚀🚀 network deployed in {:.2?}", elapsed);

	let alice = network.get_node("alice")?;
	// wait until the subxt client is ready
	let alice_client: OnlineClient<PolkadotConfig> = alice.wait_client().await?;

	// wait 10 blocks
	let mut blocks = alice_client.blocks().subscribe_finalized().await.unwrap().take(10);

	while let Some(block) = blocks.next().await {
		log::info!("Block #{}", block.unwrap().header().number);
	}

	// wait 10 blocks on the parachain
	let collator = network.get_node("collator")?;
	let collator_client: OnlineClient<PolkadotConfig> = collator.wait_client().await?;

	let mut blocks = collator_client.blocks().subscribe_finalized().await.unwrap().take(10);

	while let Some(block) = blocks.next().await {
		log::info!("Parachain Block #{}", block.unwrap().header().number);
	}

	Ok(())
}
