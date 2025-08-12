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

use crate::{
	relay_chain_specs::{KusamaChainSpec, PolkadotChainSpec},
	system_parachains_specs::{
		AssetHubKusamaChainSpec, AssetHubPolkadotChainSpec, BridgeHubKusamaChainSpec,
		BridgeHubPolkadotChainSpec, CollectivesPolkadotChainSpec, CoretimeKusamaChainSpec,
		CoretimePolkadotChainSpec, /* EncointerKusamaChainSpec, */ GluttonKusamaChainSpec,
		PeopleKusamaChainSpec, PeoplePolkadotChainSpec,
	},
	ChainSpec,
};

#[derive(Debug, serde::Deserialize)]
struct EmptyChainSpecWithId {
	id: String,
}

pub fn from_json_file(filepath: &str, supported: String) -> Result<Box<dyn ChainSpec>, String> {
	let path = std::path::PathBuf::from(&filepath);
	let file = std::fs::File::open(filepath).expect("Failed to open file");
	let reader = std::io::BufReader::new(file);
	let chain_spec: EmptyChainSpecWithId = serde_json::from_reader(reader)
		.expect("Failed to read 'json' file with ChainSpec configuration");
	match &chain_spec.id {
		x if x.starts_with("polkadot") | x.starts_with("dot") =>
			Ok(Box::new(PolkadotChainSpec::from_json_file(path)?)),
		x if x.starts_with("kusama") | x.starts_with("ksm") =>
			Ok(Box::new(KusamaChainSpec::from_json_file(path)?)),
		x if x.starts_with("asset-hub-polkadot") =>
			Ok(Box::new(AssetHubPolkadotChainSpec::from_json_file(path)?)),
		x if x.starts_with("asset-hub-kusama") =>
			Ok(Box::new(AssetHubKusamaChainSpec::from_json_file(path)?)),
		x if x.starts_with("collectives-polkadot") =>
			Ok(Box::new(CollectivesPolkadotChainSpec::from_json_file(path)?)),
		x if x.starts_with("bridge-hub-polkadot") =>
			Ok(Box::new(BridgeHubPolkadotChainSpec::from_json_file(path)?)),
		x if x.starts_with("bridge-hub-kusama") =>
			Ok(Box::new(BridgeHubKusamaChainSpec::from_json_file(path)?)),
		x if x.starts_with("coretime-kusama") =>
			Ok(Box::new(CoretimeKusamaChainSpec::from_json_file(path)?)),
		x if x.starts_with("coretime-polkadot") =>
			Ok(Box::new(CoretimePolkadotChainSpec::from_json_file(path)?)),
		x if x.starts_with("glutton-kusama") =>
			Ok(Box::new(GluttonKusamaChainSpec::from_json_file(path)?)),
		// x if x.starts_with("encointer-kusama") =>
		// 	Ok(Box::new(EncointerKusamaChainSpec::from_json_file(path)?)),
		x if x.starts_with("people-kusama") =>
			Ok(Box::new(PeopleKusamaChainSpec::from_json_file(path)?)),
		x if x.starts_with("people-polkadot") =>
			Ok(Box::new(PeoplePolkadotChainSpec::from_json_file(path)?)),
		_ => Err(format!("Unknown chain 'id' in json file. Only supported: {supported}'")),
	}
}
