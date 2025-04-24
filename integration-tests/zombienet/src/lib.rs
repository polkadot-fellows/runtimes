use anyhow::anyhow;
use zombienet_sdk::{NetworkConfig, NetworkConfigBuilder};

// Chain generator command template
const CMD_TPL: &str = "chain-spec-generator {{chainName}}";

// Relaychain nodes
const ALICE: &str = "alice";
const BOB: &str = "bob";
// Collator
const COLLATOR: &str = "collator";

pub fn small_network() -> Result<NetworkConfig, anyhow::Error> {
	let images = zombienet_sdk::environment::get_images_from_env();
	let config = NetworkConfigBuilder::new()
		.with_relaychain(|r| {
			r.with_chain("polkadot-local")
				.with_default_command("polkadot")
				.with_default_image(images.polkadot.as_str())
				.with_default_args(vec!["-lparachain=debug".into()])
				.with_chain_spec_command(CMD_TPL)
				.with_default_args(vec!["-lparachain=debug,runtime=debug".into()])
				.chain_spec_command_is_local(true)
				.with_node(|node| node.with_name(ALICE))
				.with_node(|node| node.with_name(BOB))
		})
		.with_parachain(|p| {
			p.with_id(1005)
				.with_default_command("polkadot-parachain")
				.with_default_image(images.cumulus.as_str())
				.with_default_args(vec!["-lparachain=debug".into()])
				.with_chain_spec_command(CMD_TPL)
				.chain_spec_command_is_local(true)
				.with_chain("coretime-polkadot-local")
				.with_collator(|n| n.with_name(COLLATOR))
		})
		.build()
		.map_err(|errs| {
			let e = errs.iter().fold("".to_string(), |memo, err| format!("{memo} \n {err}"));
			anyhow!(e)
		})?;

	Ok(config)
}
