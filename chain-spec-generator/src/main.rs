use clap::Parser;
use sc_chain_spec::ChainSpec;
use std::collections::HashMap;

mod relay_chain_specs;
mod system_parachains_specs;
mod common;

#[derive(Parser)]
struct Cli {
	/// The chain spec to generate.
	chain: String,

	/// Generate the chain spec as raw?
	#[arg(long)]
	raw: bool,
}

fn main() -> Result<(), String> {
	let cli = Cli::parse();

	let supported_chains =
		HashMap::<_, Box<dyn Fn() -> Result<Box<dyn ChainSpec>, String>>>::from([
			(
				"polkadot-dev",
				Box::new(|| relay_chain_specs::polkadot_development_config()) as Box<_>,
			),
			(
				"polkadot-local",
				Box::new(|| relay_chain_specs::polkadot_local_testnet_config()) as Box<_>,
			),
			("kusama-dev", Box::new(|| relay_chain_specs::kusama_development_config()) as Box<_>),
			(
				"kusama-local",
				Box::new(|| relay_chain_specs::kusama_local_testnet_config()) as Box<_>,
			),
            (
				"asset-hub-kusama-local",
				Box::new(|| system_parachains_specs::asset_hub_kusama_local_testnet_config()) as Box<_>,
			),
            (
				"asset-hub-polkadot-local",
				Box::new(|| system_parachains_specs::asset_hub_polkadot_local_testnet_config()) as Box<_>,
			),
            (
				"collectives-polkadot-local",
				Box::new(|| system_parachains_specs::collectives_polkadot_local_testnet_config()) as Box<_>,
			),
            (
				"bridge-hub-polkadot-local",
				Box::new(|| system_parachains_specs::bridge_hub_polkadot_local_testnet_config()) as Box<_>,
			),
            (
				"bridge-hub-kusama-local",
				Box::new(|| system_parachains_specs::bridge_hub_kusama_local_testnet_config()) as Box<_>,
			),
		]);

	if let Some(function) = supported_chains.get(&*cli.chain) {
		let chain_spec = (*function)()?.as_json(cli.raw)?;
		print!("{chain_spec}");
		Ok(())
	} else {
        let supported = supported_chains.keys().enumerate().fold(String::new(), |c, (n, k)| {
            let extra = (n + 1 < supported_chains.len()).then(|| ", ").unwrap_or("");
            format!("{c}{k}{extra}")
        });
		if cli.chain.ends_with(".json") {
			let chain_spec = common::from_json_file(&cli.chain, supported)?.as_json(cli.raw)?;
			print!("{chain_spec}");
			Ok(())
		} else {
			Err(format!("Unknown chain, only supported: {supported} or a json file"))
		}
	}
}
