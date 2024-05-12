// Copyright (C) Parity Technologies and the various Polkadot contributors, see Contributions.md
// for a list of specific contributors.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

use clap::Parser;
use sc_chain_spec::ChainSpec;
use std::{collections::HashMap, path::PathBuf};

mod common;
mod relay_chain_specs;
mod system_parachains_specs;

#[derive(Parser)]
struct Cli {
	/// The chain spec to generate.
	chain: String,

	/// Generate the chain spec as raw?
	#[arg(long)]
	raw: bool,
}

#[derive(Debug, serde::Deserialize)]
struct EmptyChainSpecWithId {
	id: String,
}

macro_rules! chainspec {
	($chain:expr, $csty:ty, $defcon:expr $(,)?) => {
		(
			$chain,
			Box::new(|p| {
				if let Some(p) = p {
					<Result<Box<dyn ChainSpec>, String>>::Ok(Box::new(<$csty>::from_json_file(p)?))
				} else {
					$defcon()
				}
			}) as Box<_>,
		)
	};
}

fn main() -> Result<(), String> {
	let cli = Cli::parse();

	let supported_chains =
		HashMap::<_, Box<dyn Fn(Option<PathBuf>) -> Result<Box<dyn ChainSpec>, String>>>::from([
			chainspec!(
				"polkadot-dev",
				relay_chain_specs::PolkadotChainSpec,
				relay_chain_specs::polkadot_development_config,
			),
			chainspec!(
				"polkadot-local",
				relay_chain_specs::PolkadotChainSpec,
				relay_chain_specs::polkadot_local_testnet_config,
			),
			chainspec!(
				"kusama-dev",
				relay_chain_specs::KusamaChainSpec,
				relay_chain_specs::kusama_development_config,
			),
			chainspec!(
				"kusama-local",
				relay_chain_specs::KusamaChainSpec,
				relay_chain_specs::kusama_local_testnet_config,
			),
			chainspec!(
				"asset-hub-kusama-local",
				system_parachains_specs::AssetHubKusamaChainSpec,
				system_parachains_specs::asset_hub_kusama_local_testnet_config,
			),
			chainspec!(
				"asset-hub-polkadot-local",
				system_parachains_specs::AssetHubPolkadotChainSpec,
				system_parachains_specs::asset_hub_polkadot_local_testnet_config,
			),
			chainspec!(
				"collectives-polkadot-local",
				system_parachains_specs::CollectivesPolkadotChainSpec,
				system_parachains_specs::collectives_polkadot_local_testnet_config,
			),
			chainspec!(
				"bridge-hub-polkadot-local",
				system_parachains_specs::BridgeHubPolkadotChainSpec,
				system_parachains_specs::bridge_hub_polkadot_local_testnet_config,
			),
			chainspec!(
				"bridge-hub-kusama-local",
				system_parachains_specs::BridgeHubKusamaChainSpec,
				system_parachains_specs::bridge_hub_kusama_local_testnet_config,
			),
			chainspec!(
				"glutton-kusama-local",
				system_parachains_specs::GluttonKusamaChainSpec,
				system_parachains_specs::glutton_kusama_local_testnet_config,
			),
			chainspec!(
				"encointer-kusama-local",
				system_parachains_specs::EncointerKusamaChainSpec,
				system_parachains_specs::encointer_kusama_local_testnet_config,
			),
			chainspec!(
				"coretime-kusama-local",
				system_parachains_specs::CoretimeKusamaChainSpec,
				system_parachains_specs::coretime_kusama_local_testnet_config,
			),
			chainspec!(
				"people-kusama-local",
				system_parachains_specs::PeopleKusamaChainSpec,
				system_parachains_specs::people_kusama_local_testnet_config,
			),
		]);

	if let Some(function) = supported_chains.get(&*cli.chain) {
		let chain_spec = (*function)(None)?.as_json(cli.raw)?;
		print!("{chain_spec}");
		return Ok(())
	} else if cli.chain.ends_with(".json") {
		let file = std::fs::File::open(&cli.chain).expect("Failed to open file");
		let reader = std::io::BufReader::new(file);
		let chain_spec: EmptyChainSpecWithId = serde_json::from_reader(reader)
			.expect("Failed to read 'json' file with ChainSpec configuration");

		if let Some(function) = supported_chains.get(&*chain_spec.id) {
			let path = std::path::PathBuf::from(&cli.chain);
			let chain_spec = (*function)(Some(path))?.as_json(cli.raw)?;
			print!("{chain_spec}");
			return Ok(())
		}
	}

	let supported = supported_chains.keys().copied().collect::<Vec<_>>().join(", ");

	Err(format!("Unknown chain, only supported: {supported} or a json file"))
}
