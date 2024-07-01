use anyhow::anyhow;
use subxt::{OnlineClient, PolkadotConfig};
use zombienet_sdk::{NetworkConfig, NetworkConfigBuilder, NetworkNode};

pub mod environment;

pub type Error = Box<dyn std::error::Error>;

// Chain generator command template
const CMD_TPL: &str = "chain-spec-generator {{chainName}}";

// Relaychain nodes
const ALICE: &str = "alice";
const BOB: &str = "bob";
// Collator
const COLLATOR: &str = "collator";

pub fn small_network() -> Result<NetworkConfig, Error> {
	let images = environment::get_images_from_env();
	let config = NetworkConfigBuilder::new()
		.with_relaychain(|r| {
			r.with_chain("polkadot-local")
				.with_default_command("polkadot")
				.with_default_image(images.polkadot.as_str())
				.with_chain_spec_command(CMD_TPL)
				.chain_spec_command_is_local(true)
				.with_node(|node| node.with_name(ALICE))
				.with_node(|node| node.with_name(BOB))
		})
		.with_parachain(|p| {
			p.with_id(2000).cumulus_based(true).with_collator(|n| {
				n.with_name(COLLATOR)
					.with_command("polkadot-parachain")
					.with_image(images.cumulus.as_str())
			})
		})
		.build()
		.map_err(|errs| {
			let e = errs.iter().fold("".to_string(), |memo, err| format!("{memo} \n {err}"));
			anyhow!(e)
		})?;

	Ok(config)
}

pub async fn wait_subxt_client(
	node: &NetworkNode,
) -> Result<OnlineClient<PolkadotConfig>, anyhow::Error> {
	log::info!("trying to connect to: {}", node.ws_uri());
	loop {
		match node.client::<PolkadotConfig>().await {
			Ok(cli) => {
				log::info!("returning client for: {}", node.ws_uri());
				return Ok(cli);
			},
			Err(e) => {
				log::trace!("{e:?}");
				if let subxt::Error::Rpc(subxt::error::RpcError::ClientError(ref inner)) = e {
					log::trace!("inner: {inner}");
					if inner.to_string().contains("i/o error") {
						// The node is not ready to accept connections yet
						tokio::time::sleep(std::time::Duration::from_secs(1)).await;
						continue;
					}
				}
				return Err(anyhow!("Cannot connect to node : {e:?}"));
			},
		};
	}
}
