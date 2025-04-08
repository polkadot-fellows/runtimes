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

mod claim_assets;
mod hybrid_transfers;
mod reserve_transfer;
mod send;
mod set_xcm_versions;
mod swap;
mod teleport;
mod treasury;
mod xcm_fee_estimation;

#[macro_export]
macro_rules! foreign_balance_on {
	( $chain:ident, $id:expr, $who:expr ) => {
		emulated_integration_tests_common::impls::paste::paste! {
			<$chain>::execute_with(|| {
				type ForeignAssets = <$chain as [<$chain Pallet>]>::ForeignAssets;
				<ForeignAssets as Inspect<_>>::balance($id.try_into().unwrap(), $who)
			})
		}
	};
}

#[macro_export]
macro_rules! create_pool_with_ksm_on {
	( $chain:ident, $asset_id:expr, $is_foreign:expr, $asset_owner:expr ) => {
		emulated_integration_tests_common::impls::paste::paste! {
			<$chain>::execute_with(|| {
				type RuntimeEvent = <$chain as Chain>::RuntimeEvent;
				let owner = $asset_owner;
				let signed_owner = <$chain as Chain>::RuntimeOrigin::signed(owner.clone());
				// AssetHubKusama has v4 asset ids but penpal has v5 asset ids.
				let ksm_location: xcm::v4::Location = xcm::v4::Parent.into();
				if $is_foreign {
					assert_ok!(<$chain as [<$chain Pallet>]>::ForeignAssets::mint(
						signed_owner.clone(),
						$asset_id.clone().try_into().unwrap(),
						owner.clone().into(),
						10_000_000_000_000, // For it to have more than enough.
					));
				} else {
					let asset_id = match $asset_id.interior.last() {
						Some(xcm::v4::Junction::GeneralIndex(id)) => *id as u32,
						_ => unreachable!(),
					};
					assert_ok!(<$chain as [<$chain Pallet>]>::Assets::mint(
						signed_owner.clone(),
						asset_id.into(),
						owner.clone().into(),
						10_000_000_000_000, // For it to have more than enough.
					));
				}

				assert_ok!(<$chain as [<$chain Pallet>]>::AssetConversion::create_pool(
					signed_owner.clone(),
					Box::new(ksm_location.clone().try_into().unwrap()),
					Box::new($asset_id.clone().try_into().unwrap()),
				));

				assert_expected_events!(
					$chain,
					vec![
						RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::PoolCreated { .. }) => {},
					]
				);

				assert_ok!(<$chain as [<$chain Pallet>]>::AssetConversion::add_liquidity(
					signed_owner,
					Box::new(ksm_location.try_into().unwrap()),
					Box::new($asset_id.try_into().unwrap()),
					1_000_000_000_000,
					2_000_000_000_000, // $asset_id is worth half of ksm
					0,
					0,
					owner.into()
				));

				assert_expected_events!(
					$chain,
					vec![
						RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::LiquidityAdded { .. }) => {},
					]
				);
			});
		}
	};
}
