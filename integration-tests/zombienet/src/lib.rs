use anyhow::anyhow;
use zombienet_sdk::{NetworkConfig, NetworkConfigBuilder};

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
				.with_default_args(vec!["-lparachain=debug,runtime=debug".into()])
				.chain_spec_command_is_local(true)
				.with_validator(|node| node.with_name(ALICE))
				.with_validator(|node| node.with_name(BOB))
		})
		.with_parachain(|p| {
			p.with_id(1005)
				.with_default_command("polkadot-parachain")
				.with_default_image(images.cumulus.as_str())
				.with_chain_spec_command(CMD_TPL)
				.chain_spec_command_is_local(true)
				.with_chain("coretime-polkadot-local")
				.with_collator(|n| n.with_name(COLLATOR))
		});

	let config = if let Ok(local_ip) = std::env::var("ZOMBIE_LOCAL_IP") {
		config.with_global_settings(|s| s.with_local_ip(&local_ip))
	} else {
		config
	};

	let config = config.build().map_err(|errs| {
		let e = errs.iter().fold("".to_string(), |memo, err| format!("{memo} \n {err}"));
		anyhow!(e)
	})?;

	Ok(config)
}
