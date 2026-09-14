// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

pub mod snowbridge {
	use hex_literal::hex;
	use xcm::latest::prelude::*;
	use xcm_emulator::parameter_types;

	// Weth (Wrapped Ether) contract address on Ethereum mainnet.
	pub const WETH: [u8; 20] = hex!("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");
	// The minimum Ether balance required for an account to exist. Matches value on Polkadot
	// mainnet.
	pub const MIN_ETHER_BALANCE: u128 = 15_000_000_000_000;

	parameter_types! {
		pub EthereumNetwork: NetworkId = Ethereum { chain_id: 1 };
		pub WethLocation: Location =  Location::new(2, [GlobalConsensus(EthereumNetwork::get()), AccountKey20 { network: None, key: WETH }]);
		pub EthLocation: Location =  Location::new(2, [GlobalConsensus(EthereumNetwork::get())]);
	}
}

/// Asserts that a chain's accumulated funds are teleported to Asset Hub and burned there.
#[macro_export]
macro_rules! test_accumulated_funds_are_burnt_on_asset_hub {
	( $chain:ident, $asset_hub:ident, $chain_ed:expr, $asset_hub_ed:expr $(,)? ) => {
		#[test]
		fn accumulated_funds_are_burnt_on_asset_hub() {
			use $crate::{
				frame_support::traits::{
					fungible::{Inspect as _, Mutate as _},
					Hooks as _,
				},
				pallet_accumulate_and_forward, Chain, Weight,
			};

			type ChainRuntime = <$chain as Chain>::Runtime;
			type ChainEvent = <$chain as Chain>::RuntimeEvent;
			type AssetHubRuntime = <$asset_hub as Chain>::Runtime;
			type AssetHubEvent = <$asset_hub as Chain>::RuntimeEvent;

			let accumulation_account = $chain::execute_with(|| {
				pallet_accumulate_and_forward::Pallet::<ChainRuntime>::accumulation_account()
			});
			let amount = 2 * $chain::execute_with(|| {
				<ChainRuntime as pallet_accumulate_and_forward::Config>::MinTransferAmount::get()
			});
			$chain::fund_accounts(vec![(accumulation_account.clone(), amount)]);

			// The emulated chain minted that KSM itself, so top up the checking account.
			let check_account =
				$asset_hub::execute_with($crate::pallet_xcm::Pallet::<AssetHubRuntime>::check_account);
			$asset_hub::execute_with(|| {
				assert!($crate::pallet_balances::Pallet::<AssetHubRuntime>::mint_into(
					&check_account,
					amount + $asset_hub_ed,
				)
				.is_ok());
			});
			let (asset_hub_issuance_before, check_balance_before) = $asset_hub::execute_with(|| {
				(
					$crate::pallet_balances::Pallet::<AssetHubRuntime>::total_issuance(),
					$crate::pallet_balances::Pallet::<AssetHubRuntime>::balance(&check_account),
				)
			});

			let forwarded = amount - $chain_ed;

			$chain::execute_with(|| {
				let issuance_before =
					$crate::pallet_balances::Pallet::<ChainRuntime>::total_issuance();
				let period =
					<ChainRuntime as pallet_accumulate_and_forward::Config>::TransferPeriod::get();

				$crate::frame_system::Pallet::<ChainRuntime>::set_block_number(period);
				pallet_accumulate_and_forward::Pallet::<ChainRuntime>::on_idle(period, Weight::MAX);

				// Emptied down to the ED; the KSM has left this chain.
				assert_expected_events!(
					$chain,
					vec![ChainEvent::AccumulateForward(
						pallet_accumulate_and_forward::Event::ForwardSucceeded { .. }
					) => {},]
				);
				assert_eq!(
					$crate::pallet_balances::Pallet::<ChainRuntime>::balance(&accumulation_account),
					$chain_ed
				);
				assert_eq!(
					$crate::pallet_balances::Pallet::<ChainRuntime>::total_issuance(),
					issuance_before - forwarded
				);
			});

			// Asset Hub burns it: issuance and checking account drop alike.
			$asset_hub::execute_with(|| {
				assert_expected_events!(
					$asset_hub,
					vec![AssetHubEvent::MessageQueue(
						$crate::pallet_message_queue::Event::Processed { success: true, .. }
					) => {},]
				);
				assert_eq!(
					$crate::pallet_balances::Pallet::<AssetHubRuntime>::total_issuance(),
					asset_hub_issuance_before - forwarded
				);
				assert_eq!(
					$crate::pallet_balances::Pallet::<AssetHubRuntime>::balance(&check_account),
					check_balance_before - forwarded
				);
			});
		}
	};
}
