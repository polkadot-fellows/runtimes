// Copyright (C) Parity Technologies (UK) Ltd.
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

//! Helper imports to make it easy to run the AHM integration tests for different runtimes.

#![cfg(test)]

#[cfg(not(any(feature = "polkadot", feature = "paseo", feature = "kusama")))]
compile_error!("Asset Hub migration requires the `polkadot`, `paseo` or `kusama` feature");

pub mod accounts_translation_works;
pub mod balances_test;
pub mod bench_ah;
pub mod bench_ops;
pub mod bench_rc;
pub mod call_filter_asset_hub;
pub mod call_filter_relay;
pub mod checks;
pub mod mock;
pub mod multisig_still_work;
pub mod multisig_test;
pub mod proxy;
pub mod queues_priority;
pub mod tests;
pub mod xcm_route;

/// Imports for the AHM tests that can be reused for other chains.
pub mod porting_prelude {
	#[cfg(any(feature = "polkadot", feature = "paseo"))]
	pub mod import_alias {
		pub use polkadot_runtime_constants::DOLLARS as RC_DOLLARS;
	}
	#[cfg(feature = "kusama")]
	pub mod import_alias {
		pub use asset_hub_kusama_runtime as asset_hub_polkadot_runtime;
		pub use kusama_runtime as polkadot_runtime;
		pub use kusama_runtime_constants as polkadot_runtime_constants;

		pub use kusama_runtime_constants::currency::UNITS as RC_DOLLARS;
	}
	pub use import_alias::*;

	// Convenience aliases:
	pub use asset_hub_polkadot_runtime::{
		Runtime as AhRuntime, RuntimeCall as AhRuntimeCall, RuntimeEvent as AhRuntimeEvent,
		RuntimeOrigin as AhRuntimeOrigin,
	};
	pub use polkadot_runtime::{
		Runtime as RcRuntime, RuntimeCall as RcRuntimeCall, RuntimeEvent as RcRuntimeEvent,
		RuntimeOrigin as RcRuntimeOrigin,
	};

	pub use polkadot_runtime_constants::proxy as rc_proxy_definition;
}
