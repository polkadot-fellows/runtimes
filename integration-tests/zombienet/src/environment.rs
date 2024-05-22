//! Helpers functions to get configuration (e.g. Provider and images) from the env vars
use std::{env, future::Future, pin::Pin};

use zombienet_sdk::{LocalFileSystem, Network, NetworkConfig, NetworkConfigExt, OrchestratorError};

const DEFAULT_POLKADOT_IMAGE: &str = "docker.io/parity/polkadot:latest";
const DEFAULT_CUMULUS_IMAGE: &str = "docker.io/parity/polkadot-parachain:latest";

#[derive(Debug, Default)]
pub struct Images {
	pub polkadot: String,
	pub cumulus: String,
}

pub enum Provider {
	Native,
	K8s,
	Docker,
}

// Use `docker` as default provider
impl From<String> for Provider {
	fn from(value: String) -> Self {
		match value.to_ascii_lowercase().as_ref() {
			"native" => Provider::Native,
			"k8s" => Provider::K8s,
			_ => Provider::Docker, // default provider
		}
	}
}

pub fn get_images_from_env() -> Images {
	let polkadot = env::var("POLKADOT_IMAGE").unwrap_or(DEFAULT_POLKADOT_IMAGE.into());
	let cumulus = env::var("CUMULUS_IMAGE").unwrap_or(DEFAULT_CUMULUS_IMAGE.into());
	Images { polkadot, cumulus }
}

pub fn get_provider_from_env() -> Provider {
	env::var("ZOMBIE_PROVIDER").unwrap_or_default().into()
}

type SpawnResult = Result<Network<LocalFileSystem>, OrchestratorError>;
pub fn get_spawn_fn() -> fn(NetworkConfig) -> Pin<Box<dyn Future<Output = SpawnResult> + Send>> {
	let provider = get_provider_from_env();

	match provider {
		Provider::Native => zombienet_sdk::NetworkConfig::spawn_native,
		Provider::K8s => zombienet_sdk::NetworkConfig::spawn_k8s,
		Provider::Docker => zombienet_sdk::NetworkConfig::spawn_docker,
	}
}
