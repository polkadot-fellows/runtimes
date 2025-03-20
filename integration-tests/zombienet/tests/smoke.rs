use std::time::Instant;
use subxt::ext::futures::StreamExt;
use zombienet_sdk_tests::{
	environment::{
		get_images_from_env, get_spawn_fn
	},
	small_network, wait_subxt_client
};

#[test]
fn dump_docker_images() {
	let output = std::process::Command::new("docker")
		.arg("images")
		.output()
		.expect("Failed to execute command");

	if output.status.success() {
		let stdout = String::from_utf8_lossy(&output.stdout);
		eprintln!("Docker Images:\n{}", stdout);
		log::info!("Docker Images:\n{}", stdout);
	} else {
		let stderr = String::from_utf8_lossy(&output.stderr);
		eprintln!("Error:\n{}", stderr);
		log::error!("Error:\n{}", stderr);
	}
}

#[test]
fn dump_docker_binary_versions() {
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
			eprintln!("{} binary version: {}", image, stdout);
			log::info!("{} binary version: {}", image, stdout);
		} else {
			let stderr = String::from_utf8_lossy(&output.stderr);
			eprintln!("Error: {}", stderr);
			log::error!("Error: {}", stderr);
		}
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn smoke() {
	tracing_subscriber::fmt::init();

	// config and env
	let spawn_fn = get_spawn_fn();
	let config = small_network().unwrap();

	// spawn
	let now = Instant::now();
	let network = spawn_fn(config).await.unwrap();
	let elapsed = now.elapsed();
	log::info!("🚀🚀🚀🚀 network deployed in {:.2?}", elapsed);

	let alice = network.get_node("alice").unwrap();
	// wait until the subxt client is ready
	let client = wait_subxt_client(alice).await.unwrap();

	// wait 10 blocks
	let mut blocks = client.blocks().subscribe_finalized().await.unwrap().take(10);

	while let Some(block) = blocks.next().await {
		log::info!("Block #{}", block.unwrap().header().number);
	}

	// wait 10 blocks on the parachain
	let collator = network.get_node("collator").unwrap();
	let collator_client = wait_subxt_client(collator).await.unwrap();

	let mut blocks = collator_client.blocks().subscribe_finalized().await.unwrap().take(10);

	while let Some(block) = blocks.next().await {
		log::info!("Parachain Block #{}", block.unwrap().header().number);
	}
}
