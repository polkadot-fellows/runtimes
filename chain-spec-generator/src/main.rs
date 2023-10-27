use clap::Parser;
use sc_chain_spec::ChainSpec;
use std::collections::HashMap;

mod relay_chain_specs;

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
		]);

	if let Some(function) = supported_chains.get(&*cli.chain) {
		let chain_spec = (*function)()?.as_json(cli.raw)?;
		print!("{chain_spec}");
		Ok(())
	} else {
		if cli.chain.ends_with(".json") {
			let chain_spec = relay_chain_specs::from_json_file(&cli.chain)?.as_json(cli.raw)?;
			print!("{chain_spec}");
			Ok(())
		} else {
			let supported = supported_chains.keys().enumerate().fold(String::new(), |c, (n, k)| {
				let extra = (n + 1 < supported_chains.len()).then(|| ", ").unwrap_or("");
				format!("{c}{k}{extra}")
			});
			Err(format!("Unknown chain, only supported: {supported} or a json file"))
		}
	}
}
