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

/// Asserts that funds teleported into a chain's accumulation account are forwarded back to Asset
/// Hub and burned there, leaving Asset Hub's checking account and the chain's issuance in step.
///
/// For Kusama-like chains, which burn. Polkadot chains forward to the DAP staging account instead
/// and are covered by the SDK's `dap_helpers::test_accumulate_forward_transfers_to_asset_hub`.
#[macro_export]
macro_rules! test_accumulated_funds_are_burnt_on_asset_hub {
	( $chain:ident, $asset_hub:ident, $chain_ed:expr, $asset_hub_ed:expr $(,)? ) => {
		#[test]
		fn accumulated_funds_are_burnt_on_asset_hub() {
			use $crate::{
				frame_support::traits::{fungible::Inspect as _, Hooks as _},
				pallet_accumulate_and_forward, Assets, Chain, Junction, Location, Weight,
				WeightLimit,
			};

			type ChainRuntime = <$chain as Chain>::Runtime;
			type ChainEvent = <$chain as Chain>::RuntimeEvent;
			type AssetHubRuntime = <$asset_hub as Chain>::Runtime;
			type AssetHubEvent = <$asset_hub as Chain>::RuntimeEvent;
			type ChainBalances = $crate::pallet_balances::Pallet<ChainRuntime>;
			type AssetHubBalances = $crate::pallet_balances::Pallet<AssetHubRuntime>;

			let accumulation_account = $chain::execute_with(|| {
				pallet_accumulate_and_forward::Pallet::<ChainRuntime>::accumulation_account()
			});
			let check_account =
				$asset_hub::execute_with($crate::pallet_xcm::Pallet::<AssetHubRuntime>::check_account);
			// Enough that the forward still clears `MinTransferAmount` after arrival fees.
			let teleported = 10 * $chain::execute_with(|| {
				<ChainRuntime as pallet_accumulate_and_forward::Config>::MinTransferAmount::get()
			});

			let (asset_hub_issuance_before, check_balance_before) = $asset_hub::execute_with(|| {
				(AssetHubBalances::total_issuance(), AssetHubBalances::balance(&check_account))
			});
			let chain_issuance_before =
				$chain::execute_with(|| ChainBalances::total_issuance());

			// GIVEN a real teleport out of Asset Hub into the accumulation account. The KSM moves
			// into Asset Hub's checking account, which is how it comes to sit on this chain at all.
			let sender = $crate::paste::paste! { [<$asset_hub Sender>]::get() };

			$asset_hub::execute_with(|| {
				let dest = <$asset_hub as Para>::sibling_location_of(<$chain as Para>::para_id());
				let beneficiary: Location = Junction::AccountId32 {
					network: None,
					id: accumulation_account.clone().into(),
				}
				.into();
				let assets: Assets = (Location::parent(), teleported).into();

				assert_ok!($crate::pallet_xcm::Pallet::<AssetHubRuntime>::limited_teleport_assets(
					<$asset_hub as Chain>::RuntimeOrigin::signed(sender.clone()),
					bx!(dest.into()),
					bx!(beneficiary.into()),
					bx!(assets.into()),
					0,
					WeightLimit::Unlimited,
				));
			});

			let accumulated = $chain::execute_with(|| {
				assert_eq!(
					ChainBalances::total_issuance(),
					chain_issuance_before + teleported,
					"the whole teleported amount should land on this chain"
				);
				ChainBalances::balance(&accumulation_account)
			});
			let forwarded = accumulated - $chain_ed;

			// WHEN the forward runs.
			$chain::execute_with(|| {
				let period =
					<ChainRuntime as pallet_accumulate_and_forward::Config>::TransferPeriod::get();
				$crate::frame_system::Pallet::<ChainRuntime>::set_block_number(period);
				pallet_accumulate_and_forward::Pallet::<ChainRuntime>::on_idle(period, Weight::MAX);

				assert_expected_events!(
					$chain,
					vec![ChainEvent::AccumulateForward(
						pallet_accumulate_and_forward::Event::ForwardSucceeded { .. }
					) => {},]
				);
				// Emptied down to the ED; the KSM has left this chain.
				assert_eq!(ChainBalances::balance(&accumulation_account), $chain_ed);
			});

			// THEN Asset Hub burns it.
			$asset_hub::execute_with(|| {
				assert_expected_events!(
					$asset_hub,
					vec![AssetHubEvent::MessageQueue(
						$crate::pallet_message_queue::Event::Processed { success: true, .. }
					) => {},]
				);
				assert_eq!(
					AssetHubBalances::total_issuance(),
					asset_hub_issuance_before - forwarded,
					"the burn should move Asset Hub's total issuance"
				);
			});

			// AND the checking account still matches what this chain holds: both moved by the
			// teleport in minus the burn, which is the invariant the whole change exists for.
			let check_balance_after =
				$asset_hub::execute_with(|| AssetHubBalances::balance(&check_account));
			let chain_issuance_after = $chain::execute_with(|| ChainBalances::total_issuance());
			assert_eq!(
				check_balance_after - check_balance_before,
				chain_issuance_after - chain_issuance_before,
				"Asset Hub's checking account must track this chain's issuance"
			);
			assert_eq!(check_balance_after - check_balance_before, teleported - forwarded);
		}
	};
}
