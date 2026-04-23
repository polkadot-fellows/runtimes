use anyhow::anyhow;
use serde_json::json;
use zombienet_sdk::{NetworkConfig, NetworkConfigBuilder};

pub mod environment;
pub mod helpers;

pub type Error = Box<dyn std::error::Error>;

// Chain generator command template
const CMD_TPL: &str = "chain-spec-generator {{chainName}}";

// Relaychain nodes
const ALICE: &str = "alice";
const BOB: &str = "bob";
const CHARLIE: &str = "charlie";
// Collator
pub const COLLATOR_1005: &str = "collator_1005";
pub const COLLATOR_1010_1: &str = "collator_1010_1";
pub const COLLATOR_1010_2: &str = "collator_1010_2";

// Para IDs for chains configured with elastic scaling in this workspace
// (BLOCK_PROCESSING_VELOCITY = 3, 12s slot). See
// `system-parachains/constants/src/polkadot.rs::consensus::elastic_scaling`.
pub const ASSET_HUB_POLKADOT_PARA_ID: u32 = 1000;
pub const PEOPLE_POLKADOT_PARA_ID: u32 = 1004;

/// Name of the first relay validator — used by tests to obtain an RPC client.
pub const ELASTIC_VALIDATOR_0: &str = "validator-0";

/// Describes a single elastic-scaling network to spawn. One per integration test.
pub struct ElasticNetwork<'a> {
	/// Chain name understood by `chain-spec-generator` (e.g. `asset-hub-polkadot-local`).
	pub chain: &'a str,
	/// Para ID matching the `chain`'s chain-spec extensions.
	pub para_id: u32,
	/// Names for the parachain collators. The slice length controls the collator count.
	pub collators: &'a [&'a str],
}

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
				.with_validator(|node| node.with_name(CHARLIE))
		})
		.with_parachain(|p| {
			p.with_id(1005)
				.with_default_command("polkadot-omni-node")
				.with_default_image(images.cumulus.as_str())
				.with_chain_spec_command(CMD_TPL)
				.chain_spec_command_is_local(true)
				.with_chain("coretime-polkadot-local")
				.with_collator(|n| n.with_name(COLLATOR_1005))
		})
		.with_parachain(|p| {
			p.with_id(1010)
				.with_default_command("polkadot-omni-node")
				.with_default_image(images.cumulus.as_str())
				.with_chain_spec_command(CMD_TPL)
				.chain_spec_command_is_local(true)
				.with_chain("bulletin-polkadot-local")
				.with_collator(|n| n.with_name(COLLATOR_1010_1))
				.with_collator(|n| n.with_name(COLLATOR_1010_2))
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

/// Build a zombienet network configuration for exercising elastic scaling on a single
/// parachain.
///
/// The relay chain (`polkadot-local`) is configured with 3 bulk cores and 5 validators
/// (1 validator per core); the parachain is registered with its collators running the
/// `slot-based` authoring backend — required to author multiple blocks per relay slot.
/// The test driver is expected to assign the two extra bulk cores to `net.para_id`
/// via `Coretime::assign_core` before measuring throughput.
pub fn elastic_scaling_network(net: ElasticNetwork<'_>) -> Result<NetworkConfig, Error> {
	let images = environment::get_images_from_env();
	let ElasticNetwork { chain, para_id, collators } = net;
	let collators: Vec<String> = collators.iter().map(|s| (*s).to_owned()).collect();

	let config = NetworkConfigBuilder::new()
		.with_relaychain(|r| {
			let r = r
				.with_chain("polkadot-local")
				.with_default_command("polkadot")
				.with_default_image(images.polkadot.as_str())
				.with_chain_spec_command(CMD_TPL)
				.chain_spec_command_is_local(true)
				.with_default_args(vec!["-lparachain=debug,runtime=info".into()])
				// Provision 3 bulk cores so the parachain can scale up to 3 cores total
				// (the default core plus 2 that we assign at runtime).
				.with_genesis_overrides(json!({
					"configuration": {
						"config": {
							"scheduler_params": {
								"num_cores": 3,
								"max_validators_per_core": 1,
							},
						}
					}
				}))
				.with_validator(|n| n.with_name(ELASTIC_VALIDATOR_0));
			(1..5).fold(r, |acc, i| acc.with_validator(|n| n.with_name(&format!("validator-{i}"))))
		})
		.with_parachain(|p| {
			let (first, rest) = collators.split_first().expect("at least one collator required");
			let p = p
				.with_id(para_id)
				.with_default_command("polkadot-omni-node")
				.with_default_image(images.cumulus.as_str())
				.with_chain_spec_command(CMD_TPL)
				.chain_spec_command_is_local(true)
				.with_chain(chain)
				// Enable slot-based authoring so the collator set can fill multiple cores.
				.with_default_args(vec![
					"-laura=debug,runtime=info,cumulus-consensus=debug,parachain::collation-generation=debug,parachain::collator-protocol=debug,parachain=debug".into(),
					"--force-authoring".into(),
					("--authoring", "slot-based").into(),
				])
				.with_collator(|n| n.with_name(first));
			rest.iter().fold(p, |acc, name| acc.with_collator(|n| n.with_name(name)))
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
