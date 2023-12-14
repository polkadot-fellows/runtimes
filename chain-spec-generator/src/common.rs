use crate::{
	relay_chain_specs::{KusamaChainSpec, PolkadotChainSpec},
	system_parachains_specs::{
		AssetHubKusamaChainSpec, AssetHubPolkadotChainSpec, BridgeHubKusamaChainSpec,
		BridgeHubPolkadotChainSpec, CollectivesPolkadotChainSpec,
	},
	ChainSpec,
};
use polkadot_primitives::{AccountId, AccountPublic};
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::IdentifyAccount;

pub fn testnet_accounts() -> Vec<AccountId> {
	vec![
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		get_account_id_from_seed::<sr25519::Public>("Bob"),
		get_account_id_from_seed::<sr25519::Public>("Charlie"),
		get_account_id_from_seed::<sr25519::Public>("Dave"),
		get_account_id_from_seed::<sr25519::Public>("Eve"),
		get_account_id_from_seed::<sr25519::Public>("Ferdie"),
		get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
		get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
		get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
		get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
		get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
		get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
	]
}

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

#[derive(Debug, serde::Deserialize)]
struct EmptyChainSpecWithId {
	id: String,
}

pub fn from_json_file(filepath: &str, supported: String) -> Result<Box<dyn ChainSpec>, String> {
	let path = std::path::PathBuf::from(&filepath);
	let file = std::fs::File::open(&filepath).expect("Failed to open file");
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
		_ => Err(format!("Unknown chain 'id' in json file. Only supported: {supported}'")),
	}
}
