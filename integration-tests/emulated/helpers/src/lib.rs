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

pub use paste;

// Substrate
pub use pallet_balances;
pub use pallet_message_queue;

// Polkadot
pub use pallet_xcm;
pub use xcm::prelude::{AccountId32, VersionedAssets, Weight, WeightLimit};

// Cumulus
pub use asset_test_utils;
pub use cumulus_pallet_xcmp_queue;
pub use emulated_integration_tests_common::{macros::Dmp, test_chain_can_claim_assets};
pub use xcm_emulator::Chain;

pub mod common;

/// TODO: when bumping to polkadot-sdk stable2506,
/// remove this crate altogether and get the macros from `emulated-integration-tests-common`.
/// note: $asset needs to be prefunded outside this function
#[macro_export]
macro_rules! create_pool_with_native_on {
	( $chain:ident, $asset:expr, $is_foreign:expr, $asset_owner:expr ) => {
		$crate::create_pool_with_native_on!(
			$chain,
			$asset,
			$is_foreign,
			$asset_owner,
			1_000_000_000_000,
			2_000_000_000_000
		);
	};

	( $chain:ident, $asset:expr, $is_foreign:expr, $asset_owner:expr, $native_amount:expr, $asset_amount:expr ) => {
		$crate::paste::paste! {
			<$chain>::execute_with(|| {
				type RuntimeEvent = <$chain as $crate::Chain>::RuntimeEvent;
				let owner = $asset_owner;
				let signed_owner = <$chain as $crate::Chain>::RuntimeOrigin::signed(owner.clone());
				let native_asset = Location::parent();

				if $is_foreign {
					assert_ok!(<$chain as [<$chain Pallet>]>::ForeignAssets::mint(
						signed_owner.clone(),
						$asset.clone().into(),
						owner.clone().into(),
						10_000_000_000_000, // For it to have more than enough.
					));
				} else {
					let asset_id = match $asset.clone().interior.last() {
						Some(GeneralIndex(id)) => *id as u32,
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
					Box::new(native_asset.clone()),
					Box::new($asset.clone().try_into().unwrap()),
				));

				assert_expected_events!(
					$chain,
					vec![
						RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::PoolCreated { .. }) => {},
					]
				);

				assert_ok!(<$chain as [<$chain Pallet>]>::AssetConversion::add_liquidity(
					signed_owner,
					Box::new(native_asset),
					Box::new($asset),
					$native_amount,
					$asset_amount,
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

#[macro_export]
macro_rules! test_relay_is_trusted_teleporter {
	( $sender_relay:ty, vec![$( $receiver_para:ty ),+], ($assets:expr, $amount:expr), $xcm_call:ident ) => {
		$crate::paste::paste! {
			// init Origin variables
			let sender = [<$sender_relay Sender>]::get();
			let mut relay_sender_balance_before =
				<$sender_relay as $crate::Chain>::account_data_of(sender.clone()).free;
			let fee_asset_item = 0;
			let weight_limit = $crate::WeightLimit::Unlimited;

			$(
				{
					// init Destination variables
					let receiver = [<$receiver_para Receiver>]::get();
					let para_receiver_balance_before =
						<$receiver_para as $crate::Chain>::account_data_of(receiver.clone()).free;
					let para_destination =
						<$sender_relay>::child_location_of(<$receiver_para>::para_id());
					let beneficiary: Location =
						$crate::AccountId32 { network: None, id: receiver.clone().into() }.into();

					<$sender_relay>::execute_with(|| {
						$crate::Dmp::<<$sender_relay as $crate::Chain>::Runtime>::make_parachain_reachable(<$receiver_para>::para_id());
					});

					// Dry-run first.
					let call = <$sender_relay as Chain>::RuntimeCall::XcmPallet(pallet_xcm::Call::$xcm_call {
						dest: bx!(para_destination.clone().into()),
						beneficiary: bx!(beneficiary.clone().into()),
						assets: bx!($assets.clone().into()),
						fee_asset_item: fee_asset_item,
						weight_limit: weight_limit.clone(),
					});

					// verify sane weight for a call
					let max_weight_with_margin_for_error = (Weight::MAX.ref_time() / 100) * 90; // assume up to 90% of max weight
					assert!(call.get_dispatch_info().call_weight.ref_time() < max_weight_with_margin_for_error);

					let mut delivery_fees_amount = 0;
					let mut remote_message = VersionedXcm::from(Xcm(Vec::new()));
					<$sender_relay>::execute_with(|| {
						type Runtime = <$sender_relay as Chain>::Runtime;
						type OriginCaller = <$sender_relay as Chain>::OriginCaller;

						let origin = OriginCaller::system(RawOrigin::Signed(sender.clone()));
						let result = Runtime::dry_run_call(origin, call.clone(), xcm::prelude::XCM_VERSION).unwrap();
						// We filter the result to get only the messages we are interested in.
						let (destination_to_query, messages_to_query) = &result
							.forwarded_xcms
							.iter()
							.find(|(destination, _)| {
								*destination == VersionedLocation::from(Location::new(0, [Parachain(<$receiver_para>::para_id().into())]))
							})
							.unwrap();
						assert_eq!(messages_to_query.len(), 1);
						remote_message = messages_to_query[0].clone();
						let delivery_fees =
							Runtime::query_delivery_fees(destination_to_query.clone(), remote_message.clone())
								.unwrap();
						let latest_delivery_fees: Assets = delivery_fees.clone().try_into().unwrap();
						let Fungible(inner_delivery_fees_amount) = latest_delivery_fees.inner()[0].fun else {
							unreachable!("asset is non-fungible");
						};
						delivery_fees_amount = inner_delivery_fees_amount;
					});

					// Reset to send actual message.
					<$sender_relay>::reset_ext();
					<$receiver_para>::reset_ext();

					// Send XCM message from Relay.
					<$sender_relay>::execute_with(|| {
						$crate::Dmp::<<$sender_relay as $crate::Chain>::Runtime>::make_parachain_reachable(<$receiver_para>::para_id());

						let origin = <$sender_relay as Chain>::RuntimeOrigin::signed(sender.clone());
						assert_ok!(call.dispatch(origin));

						type RuntimeEvent = <$sender_relay as $crate::Chain>::RuntimeEvent;

						assert_expected_events!(
							$sender_relay,
							vec![
								RuntimeEvent::XcmPallet(
									$crate::pallet_xcm::Event::Attempted { outcome: Outcome::Complete { .. } }
								) => {},
								RuntimeEvent::Balances(
									$crate::pallet_balances::Event::Burned { who: sender, amount }
								) => {},
								RuntimeEvent::XcmPallet(
									$crate::pallet_xcm::Event::Sent { .. }
								) => {},
							]
						);
					});

					// Receive XCM message in Destination Parachain
					<$receiver_para>::execute_with(|| {
						type RuntimeEvent = <$receiver_para as $crate::Chain>::RuntimeEvent;

						assert_expected_events!(
							$receiver_para,
							vec![
								RuntimeEvent::Balances(
									$crate::pallet_balances::Event::Minted { who: receiver, .. }
								) => {},
								RuntimeEvent::MessageQueue(
									$crate::pallet_message_queue::Event::Processed { success: true, .. }
								) => {},
							]
						);
					});

					// Check if balances are updated accordingly in Origin and Parachain
					let relay_sender_balance_after =
						<$sender_relay as $crate::Chain>::account_data_of(sender.clone()).free;
					let para_receiver_balance_after =
						<$receiver_para as $crate::Chain>::account_data_of(receiver.clone()).free;

					assert_eq!(relay_sender_balance_before - $amount - delivery_fees_amount, relay_sender_balance_after);
					assert!(para_receiver_balance_after > para_receiver_balance_before);

					// Update sender balance
					relay_sender_balance_before = <$sender_relay as $crate::Chain>::account_data_of(sender.clone()).free;
				}
			)+
		}
	};
}

#[macro_export]
macro_rules! test_parachain_is_trusted_teleporter_for_relay {
	( $sender_para:ty, $receiver_relay:ty, $amount:expr, $xcm_call:ident ) => {
		$crate::paste::paste! {
			// init Origin variables
			let sender = [<$sender_para Sender>]::get();
			// Mint assets to `$sender_para` to succeed with teleport.
			<$sender_para>::execute_with(|| {
				assert_ok!(<$sender_para as [<$sender_para Pallet>]>::Balances::mint_into(
					&sender,
					$amount + 10_000_000_000, // Some extra for delivery fees.
				));
			});
			let mut para_sender_balance_before =
				<$sender_para as $crate::Chain>::account_data_of(sender.clone()).free;
			let origin = <$sender_para as $crate::Chain>::RuntimeOrigin::signed(sender.clone());
			let assets: Assets = (Parent, $amount).into();
			let fee_asset_item = 0;
			let weight_limit = $crate::WeightLimit::Unlimited;

			// We need to mint funds into the checking account of `$receiver_relay`
			// for it to accept a teleport from `$sender_para`.
			// Else we'd get a `NotWithdrawable` error since it tries to reduce the check account balance, which
			// would be 0.
			<$receiver_relay>::execute_with(|| {
				let check_account = <$receiver_relay as [<$receiver_relay Pallet>]>::XcmPallet::check_account();
				assert_ok!(<$receiver_relay as [<$receiver_relay Pallet>]>::Balances::mint_into(
					&check_account,
					$amount,
				));
			});

			// Init destination variables.
			let receiver = [<$receiver_relay Receiver>]::get();
			let relay_receiver_balance_before =
				<$receiver_relay as $crate::Chain>::account_data_of(receiver.clone()).free;
			let relay_destination: Location = Parent.into();
			let beneficiary: Location =
				$crate::AccountId32 { network: None, id: receiver.clone().into() }.into();

			// Dry-run first.
			let call = <$sender_para as Chain>::RuntimeCall::PolkadotXcm(pallet_xcm::Call::$xcm_call {
				dest: bx!(relay_destination.clone().into()),
				beneficiary: bx!(beneficiary.clone().into()),
				assets: bx!(assets.clone().into()),
				fee_asset_item: fee_asset_item,
				weight_limit: weight_limit.clone(),
			});

			// verify sane weight for a call
			let max_weight_with_margin_for_error = (Weight::MAX.ref_time() / 100) * 90; // assume up to 90% of max weight
			assert!(call.get_dispatch_info().call_weight.ref_time() < max_weight_with_margin_for_error);

			// These will be filled in the closure.
			let mut delivery_fees_amount = 0;
			let mut remote_message = VersionedXcm::from(Xcm(Vec::new()));
			<$sender_para>::execute_with(|| {
				type Runtime = <$sender_para as Chain>::Runtime;
				type OriginCaller = <$sender_para as Chain>::OriginCaller;

				let origin = OriginCaller::system(RawOrigin::Signed(sender.clone()));
				let result = Runtime::dry_run_call(origin, call.clone(), xcm::prelude::XCM_VERSION).unwrap();
				// We filter the result to get only the messages we are interested in.
				let (destination_to_query, messages_to_query) = &result
					.forwarded_xcms
					.iter()
					.find(|(destination, _)| {
						*destination == VersionedLocation::from(Location::parent())
					})
					.unwrap();
				assert_eq!(messages_to_query.len(), 1);
				remote_message = messages_to_query[0].clone();
				let delivery_fees =
					Runtime::query_delivery_fees(destination_to_query.clone(), remote_message.clone())
						.unwrap();
				let latest_delivery_fees: Assets = delivery_fees.clone().try_into().unwrap();
				delivery_fees_amount = if let Some(first_asset) = latest_delivery_fees.inner().first() {
					let Fungible(inner_delivery_fees_amount) = first_asset.fun else {
						unreachable!("asset is non-fungible");
					};
					inner_delivery_fees_amount
				} else {
					0
				}
			});

			// Reset to send actual message.
			<$sender_para>::reset_ext();
			<$receiver_relay>::reset_ext();

			// Mint assets to `$sender_para` to succeed with teleport.
			<$sender_para>::execute_with(|| {
				assert_ok!(<$sender_para as [<$sender_para Pallet>]>::Balances::mint_into(
					&sender,
					$amount + 10_000_000_000, // Some extra for delivery fees.
				));
			});

			// Since we reset everything, we need to mint funds into the checking account again.
			<$receiver_relay>::execute_with(|| {
				let check_account = <$receiver_relay as [<$receiver_relay Pallet>]>::XcmPallet::check_account();
				assert_ok!(<$receiver_relay as [<$receiver_relay Pallet>]>::Balances::mint_into(
					&check_account,
					$amount,
				));
			});

			// Send XCM message from Parachain.
			<$sender_para>::execute_with(|| {
				let origin = <$sender_para as Chain>::RuntimeOrigin::signed(sender.clone());
				assert_ok!(call.dispatch(origin));

				type RuntimeEvent = <$sender_para as $crate::Chain>::RuntimeEvent;

				assert_expected_events!(
					$sender_para,
					vec![
						RuntimeEvent::PolkadotXcm(
							$crate::pallet_xcm::Event::Attempted { outcome: Outcome::Complete { .. } }
						) => {},
						RuntimeEvent::Balances(
							$crate::pallet_balances::Event::Burned { who: sender, amount }
						) => {},
						RuntimeEvent::PolkadotXcm(
							$crate::pallet_xcm::Event::Sent { .. }
						) => {},
					]
				);
			});

			// Receive XCM message in Destination Parachain
			<$receiver_relay>::execute_with(|| {
				type RuntimeEvent = <$receiver_relay as $crate::Chain>::RuntimeEvent;

				assert_expected_events!(
					$receiver_relay,
					vec![
						RuntimeEvent::Balances(
							$crate::pallet_balances::Event::Minted { who: receiver, .. }
						) => {},
						RuntimeEvent::MessageQueue(
							$crate::pallet_message_queue::Event::Processed { success: true, .. }
						) => {},
					]
				);
			});

			// Check if balances are updated accordingly in Origin and Relay Chain
			let para_sender_balance_after =
				<$sender_para as $crate::Chain>::account_data_of(sender.clone()).free;
			let relay_receiver_balance_after =
				<$receiver_relay as $crate::Chain>::account_data_of(receiver.clone()).free;

			assert_eq!(para_sender_balance_before - $amount - delivery_fees_amount, para_sender_balance_after);
			assert!(relay_receiver_balance_after > relay_receiver_balance_before);

			// Update sender balance
			para_sender_balance_before = <$sender_para as $crate::Chain>::account_data_of(sender.clone()).free;
		}
	};
}

#[macro_export]
macro_rules! test_parachain_is_trusted_teleporter {
	( $sender_para:ty, vec![$( $receiver_para:ty ),+], ($assets:expr, $amount:expr), $xcm_call:ident ) => {
		$crate::paste::paste! {
			// init Origin variables
			let sender = [<$sender_para Sender>]::get();
			let mut para_sender_balance_before =
				<$sender_para as $crate::Chain>::account_data_of(sender.clone()).free;
			let origin = <$sender_para as $crate::Chain>::RuntimeOrigin::signed(sender.clone());
			let fee_asset_item = 0;
			let weight_limit = $crate::WeightLimit::Unlimited;

			$(
				{
					// init Destination variables
					let receiver = [<$receiver_para Receiver>]::get();
					let para_receiver_balance_before =
						<$receiver_para as $crate::Chain>::account_data_of(receiver.clone()).free;
					let para_destination =
						<$sender_para>::sibling_location_of(<$receiver_para>::para_id());
					let beneficiary: Location =
						$crate::AccountId32 { network: None, id: receiver.clone().into() }.into();

					// Dry-run first.
					let call = <$sender_para as Chain>::RuntimeCall::PolkadotXcm(pallet_xcm::Call::$xcm_call {
						dest: bx!(para_destination.clone().into()),
						beneficiary: bx!(beneficiary.clone().into()),
						assets: bx!($assets.clone().into()),
						fee_asset_item: fee_asset_item,
						weight_limit: weight_limit.clone(),
					});

					let max_weight_with_margin_for_error = (Weight::MAX.ref_time() / 100) * 90; // assume up to 90% of max weight
					assert!(call.get_dispatch_info().call_weight.ref_time() < max_weight_with_margin_for_error);

					let mut delivery_fees_amount = 0;
					let mut remote_message = VersionedXcm::from(Xcm(Vec::new()));
					<$sender_para>::execute_with(|| {
						type Runtime = <$sender_para as Chain>::Runtime;
						type OriginCaller = <$sender_para as Chain>::OriginCaller;

						let origin = OriginCaller::system(RawOrigin::Signed(sender.clone()));
						let result = Runtime::dry_run_call(origin, call.clone(), xcm::prelude::XCM_VERSION).unwrap();
						// We filter the result to get only the messages we are interested in.
						let (destination_to_query, messages_to_query) = &result
							.forwarded_xcms
							.iter()
							.find(|(destination, _)| {
								*destination == VersionedLocation::from(Location::new(1, [Parachain(<$receiver_para>::para_id().into())]))
							})
							.unwrap();
						assert_eq!(messages_to_query.len(), 1);
						remote_message = messages_to_query[0].clone();
						let delivery_fees =
							Runtime::query_delivery_fees(destination_to_query.clone(), remote_message.clone())
								.unwrap();
						let latest_delivery_fees: Assets = delivery_fees.clone().try_into().unwrap();
						let Fungible(inner_delivery_fees_amount) = latest_delivery_fees.inner()[0].fun else {
							unreachable!("asset is non-fungible");
						};
						delivery_fees_amount = inner_delivery_fees_amount;
					});

					// Reset to send actual message.
					<$sender_para>::reset_ext();
					<$receiver_para>::reset_ext();

					// TODO: The test fails without the line below, seems like no horizontal message passing is being done
					//       when also using dry_run_call above (it works if there is no dry_run_call)
					//       So this is just workaround, must be investigated
					<$sender_para>::execute_with(|| { });

					// Send XCM message from Origin Parachain
					<$sender_para>::execute_with(|| {
						let origin = <$sender_para as Chain>::RuntimeOrigin::signed(sender.clone());
						assert_ok!(call.dispatch(origin));

						type RuntimeEvent = <$sender_para as $crate::Chain>::RuntimeEvent;

						assert_expected_events!(
							$sender_para,
							vec![
								RuntimeEvent::PolkadotXcm(
									$crate::pallet_xcm::Event::Attempted { outcome: Outcome::Complete { .. } }
								) => {},
								RuntimeEvent::XcmpQueue(
									$crate::cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }
								) => {},
								RuntimeEvent::Balances(
									$crate::pallet_balances::Event::Burned { who: sender, amount }
								) => {},
							]
						);
					});

					// Receive XCM message in Destination Parachain
					<$receiver_para>::execute_with(|| {
						type RuntimeEvent = <$receiver_para as $crate::Chain>::RuntimeEvent;

						assert_expected_events!(
							$receiver_para,
							vec![
								RuntimeEvent::Balances(
									$crate::pallet_balances::Event::Minted { who: receiver, .. }
								) => {},
								RuntimeEvent::MessageQueue(
									$crate::pallet_message_queue::Event::Processed { success: true, .. }
								) => {},
							]
						);
					});

					// Check if balances are updated accordingly in Origin and Destination Parachains
					let para_sender_balance_after =
						<$sender_para as $crate::Chain>::account_data_of(sender.clone()).free;
					let para_receiver_balance_after =
						<$receiver_para as $crate::Chain>::account_data_of(receiver.clone()).free;

					assert_eq!(para_sender_balance_before - $amount - delivery_fees_amount, para_sender_balance_after);
					assert!(para_receiver_balance_after > para_receiver_balance_before);

					// Update sender balance
					para_sender_balance_before = <$sender_para as $crate::Chain>::account_data_of(sender.clone()).free;
				}
			)+
		}
	};
}
