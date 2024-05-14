use std::time::Instant;
use subxt::ext::futures::StreamExt;
use zombienet_sdk_tests::{environment::get_spawn_fn, small_network, wait_subxt_client};

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

	// wait 3 blocks
	let mut blocks = client.blocks().subscribe_finalized().await.unwrap().take(10);

	while let Some(block) = blocks.next().await {
		println!("Block #{}", block.unwrap().header().number);
	}
}
