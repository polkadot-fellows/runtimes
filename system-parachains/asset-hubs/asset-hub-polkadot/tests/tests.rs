// This file is part of Cumulus.

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

//! Tests for the Polkadot Asset Hub (previously known as Statemint) chain.

use asset_hub_polkadot_runtime::{
	xcm_config::{
		bridging::{self, XcmBridgeHubRouterFeeAssetId},
		CheckingAccount, DotLocation, ForeignCreatorsSovereignAccountOf, LocationToAccountId,
		RelayTreasuryLocation, RelayTreasuryPalletAccount, StakingPot,
		TrustBackedAssetsPalletLocation, XcmConfig,
	},
	AllPalletsWithoutSystem, AssetConversion, AssetDeposit, Assets, Balances, ExistentialDeposit,
	ForeignAssets, ForeignAssetsInstance, MetadataDepositBase, MetadataDepositPerByte,
	ParachainSystem, PolkadotXcm, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin, SessionKeys,
	ToKusamaXcmRouterInstance, TrustBackedAssetsInstance, XcmpQueue, SLOT_DURATION,
};
use asset_test_utils::{
	test_cases_over_bridge::TestBridgingConfig, CollatorSessionKey, CollatorSessionKeys, ExtBuilder,
};
use codec::{Decode, Encode};
use frame_support::{assert_ok, traits::fungibles::InspectEnumerable};
use parachains_common::{
	AccountId, AssetHubPolkadotAuraId as AuraId, AssetIdForTrustBackedAssets, Balance,
};
use parachains_runtimes_test_utils::SlotDurations;
use sp_consensus_aura::SlotDuration;
use sp_runtime::traits::MaybeEquivalence;
use sp_std::ops::Mul;
use system_parachains_constants::{
	kusama::consensus::RELAY_CHAIN_SLOT_DURATION_MILLIS, polkadot::fee::WeightToFee,
};
use xcm::latest::prelude::{Assets as XcmAssets, *};
use xcm_builder::V4V3LocationConverter;
use xcm_executor::traits::{ConvertLocation, JustTry};

const ALICE: [u8; 32] = [1u8; 32];
const SOME_ASSET_ADMIN: [u8; 32] = [5u8; 32];

type AssetIdForTrustBackedAssetsConvertLatest =
	assets_common::AssetIdForTrustBackedAssetsConvertLatest<TrustBackedAssetsPalletLocation>;

type RuntimeHelper = asset_test_utils::RuntimeHelper<Runtime, AllPalletsWithoutSystem>;

fn collator_session_key(account: [u8; 32]) -> CollatorSessionKey<Runtime> {
	CollatorSessionKey::new(
		AccountId::from(account),
		AccountId::from(account),
		SessionKeys { aura: AuraId::from(sp_core::ed25519::Public::from_raw(account)) },
	)
}

fn collator_session_keys() -> CollatorSessionKeys<Runtime> {
	CollatorSessionKeys::default().add(collator_session_key(ALICE))
}

fn slot_durations() -> SlotDurations {
	SlotDurations {
		relay: SlotDuration::from_millis(RELAY_CHAIN_SLOT_DURATION_MILLIS.into()),
		para: SlotDuration::from_millis(SLOT_DURATION),
	}
}

fn setup_pool_for_paying_fees_with_foreign_assets(
	(foreign_asset_owner, foreign_asset_id_location, foreign_asset_id_minimum_balance): (
		AccountId,
		xcm::v3::Location,
		Balance,
	),
) {
	let existential_deposit = ExistentialDeposit::get();

	// setup a pool to pay fees with `foreign_asset_id_location` tokens
	let pool_owner: AccountId = [14u8; 32].into();
	let native_asset = xcm::v3::Location::parent();
	let pool_liquidity: Balance =
		existential_deposit.max(foreign_asset_id_minimum_balance).mul(100_000);

	let _ = Balances::force_set_balance(
		RuntimeOrigin::root(),
		pool_owner.clone().into(),
		(existential_deposit + pool_liquidity).mul(2),
	);

	assert_ok!(ForeignAssets::mint(
		RuntimeOrigin::signed(foreign_asset_owner),
		foreign_asset_id_location,
		pool_owner.clone().into(),
		(foreign_asset_id_minimum_balance + pool_liquidity).mul(2),
	));

	assert_ok!(AssetConversion::create_pool(
		RuntimeOrigin::signed(pool_owner.clone()),
		Box::new(native_asset),
		Box::new(foreign_asset_id_location)
	));

	assert_ok!(AssetConversion::add_liquidity(
		RuntimeOrigin::signed(pool_owner.clone()),
		Box::new(native_asset),
		Box::new(foreign_asset_id_location),
		pool_liquidity,
		pool_liquidity,
		1,
		1,
		pool_owner,
	));
}

#[test]
fn test_ed_is_one_hundredth_of_relay() {
	ExtBuilder::<Runtime>::default()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::ed25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			let relay_ed = polkadot_runtime_constants::currency::EXISTENTIAL_DEPOSIT;
			let asset_hub_ed = ExistentialDeposit::get();
			assert_eq!(relay_ed / 100, asset_hub_ed);
		});
}

#[test]
fn test_assets_balances_api_works() {
	use assets_common::runtime_api::runtime_decl_for_fungibles_api::FungiblesApi;

	ExtBuilder::<Runtime>::default()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::ed25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			let local_asset_id = 1;
			let foreign_asset_id_location = xcm::v3::Location::new(
				1,
				[xcm::v3::Junction::Parachain(1234), xcm::v3::Junction::GeneralIndex(12345)],
			);

			// check before
			assert_eq!(Assets::balance(local_asset_id, AccountId::from(ALICE)), 0);
			assert_eq!(
				ForeignAssets::balance(foreign_asset_id_location, AccountId::from(ALICE)),
				0
			);
			assert_eq!(Balances::free_balance(AccountId::from(ALICE)), 0);
			assert!(Runtime::query_account_balances(AccountId::from(ALICE))
				.unwrap()
				.try_as::<XcmAssets>()
				.unwrap()
				.is_none());

			// Drip some balance
			use frame_support::traits::fungible::Mutate;
			let some_currency = ExistentialDeposit::get();
			Balances::mint_into(&AccountId::from(ALICE), some_currency).unwrap();

			// We need root origin to create a sufficient asset
			let minimum_asset_balance = 3333333_u128;
			assert_ok!(Assets::force_create(
				RuntimeHelper::root_origin(),
				local_asset_id.into(),
				AccountId::from(ALICE).into(),
				true,
				minimum_asset_balance
			));

			// We first mint enough asset for the account to exist for assets
			assert_ok!(Assets::mint(
				RuntimeHelper::origin_of(AccountId::from(ALICE)),
				local_asset_id.into(),
				AccountId::from(ALICE).into(),
				minimum_asset_balance
			));

			// create foreign asset
			let foreign_asset_minimum_asset_balance = 3333333_u128;
			assert_ok!(ForeignAssets::force_create(
				RuntimeHelper::root_origin(),
				foreign_asset_id_location,
				AccountId::from(SOME_ASSET_ADMIN).into(),
				false,
				foreign_asset_minimum_asset_balance
			));

			// We first mint enough asset for the account to exist for assets
			assert_ok!(ForeignAssets::mint(
				RuntimeHelper::origin_of(AccountId::from(SOME_ASSET_ADMIN)),
				foreign_asset_id_location,
				AccountId::from(ALICE).into(),
				6 * foreign_asset_minimum_asset_balance
			));

			// check after
			assert_eq!(
				Assets::balance(local_asset_id, AccountId::from(ALICE)),
				minimum_asset_balance
			);
			assert_eq!(
				ForeignAssets::balance(foreign_asset_id_location, AccountId::from(ALICE)),
				6 * minimum_asset_balance
			);
			assert_eq!(Balances::free_balance(AccountId::from(ALICE)), some_currency);

			let result: XcmAssets = Runtime::query_account_balances(AccountId::from(ALICE))
				.unwrap()
				.try_into()
				.unwrap();
			assert_eq!(result.len(), 3);

			// check currency
			assert!(result.inner().iter().any(|asset| asset.eq(
				&assets_common::fungible_conversion::convert_balance::<DotLocation, Balance>(
					some_currency
				)
				.unwrap()
			)));
			// check trusted asset
			assert!(result.inner().iter().any(|asset| asset.eq(&(
				AssetIdForTrustBackedAssetsConvertLatest::convert_back(&local_asset_id).unwrap(),
				minimum_asset_balance
			)
				.into())));
			// check foreign asset
			assert!(result.inner().iter().any(|asset| asset.eq(&(
				V4V3LocationConverter::convert_back(&foreign_asset_id_location).unwrap(),
				6 * foreign_asset_minimum_asset_balance
			)
				.into())));
		});
}

asset_test_utils::include_teleports_for_native_asset_works!(
	Runtime,
	AllPalletsWithoutSystem,
	XcmConfig,
	CheckingAccount,
	WeightToFee,
	ParachainSystem,
	collator_session_keys(),
	slot_durations(),
	ExistentialDeposit::get(),
	Box::new(|runtime_event_encoded: Vec<u8>| {
		match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
			Ok(RuntimeEvent::PolkadotXcm(event)) => Some(event),
			_ => None,
		}
	}),
	1000
);

asset_test_utils::include_teleports_for_foreign_assets_works!(
	Runtime,
	AllPalletsWithoutSystem,
	XcmConfig,
	CheckingAccount,
	WeightToFee,
	ParachainSystem,
	ForeignCreatorsSovereignAccountOf,
	ForeignAssetsInstance,
	collator_session_keys(),
	slot_durations(),
	ExistentialDeposit::get(),
	Box::new(|runtime_event_encoded: Vec<u8>| {
		match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
			Ok(RuntimeEvent::PolkadotXcm(event)) => Some(event),
			_ => None,
		}
	}),
	Box::new(|runtime_event_encoded: Vec<u8>| {
		match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
			Ok(RuntimeEvent::XcmpQueue(event)) => Some(event),
			_ => None,
		}
	})
);

asset_test_utils::include_asset_transactor_transfer_with_local_consensus_currency_works!(
	Runtime,
	XcmConfig,
	collator_session_keys(),
	ExistentialDeposit::get(),
	Box::new(|| {
		assert!(Assets::asset_ids().collect::<Vec<_>>().is_empty());
		assert!(ForeignAssets::asset_ids().collect::<Vec<_>>().is_empty());
	}),
	Box::new(|| {
		assert!(Assets::asset_ids().collect::<Vec<_>>().is_empty());
		assert!(ForeignAssets::asset_ids().collect::<Vec<_>>().is_empty());
	})
);

asset_test_utils::include_asset_transactor_transfer_with_pallet_assets_instance_works!(
	asset_transactor_transfer_with_trust_backed_assets_works,
	Runtime,
	XcmConfig,
	TrustBackedAssetsInstance,
	AssetIdForTrustBackedAssets,
	AssetIdForTrustBackedAssetsConvertLatest,
	collator_session_keys(),
	ExistentialDeposit::get(),
	12345,
	Box::new(|| {
		assert!(ForeignAssets::asset_ids().collect::<Vec<_>>().is_empty());
	}),
	Box::new(|| {
		assert!(ForeignAssets::asset_ids().collect::<Vec<_>>().is_empty());
	})
);

asset_test_utils::include_asset_transactor_transfer_with_pallet_assets_instance_works!(
	asset_transactor_transfer_with_foreign_assets_works,
	Runtime,
	XcmConfig,
	ForeignAssetsInstance,
	xcm::v3::Location,
	JustTry,
	collator_session_keys(),
	ExistentialDeposit::get(),
	xcm::v3::Location::new(
		1,
		[xcm::v3::Junction::Parachain(1313), xcm::v3::Junction::GeneralIndex(12345)]
	),
	Box::new(|| {
		assert!(Assets::asset_ids().collect::<Vec<_>>().is_empty());
	}),
	Box::new(|| {
		assert!(Assets::asset_ids().collect::<Vec<_>>().is_empty());
	})
);

asset_test_utils::include_create_and_manage_foreign_assets_for_local_consensus_parachain_assets_works!(
	Runtime,
	XcmConfig,
	WeightToFee,
	ForeignCreatorsSovereignAccountOf,
	ForeignAssetsInstance,
	xcm::v3::Location,
	V4V3LocationConverter,
	collator_session_keys(),
	ExistentialDeposit::get(),
	AssetDeposit::get(),
	MetadataDepositBase::get(),
	MetadataDepositPerByte::get(),
	Box::new(|pallet_asset_call| RuntimeCall::ForeignAssets(pallet_asset_call).encode()),
	Box::new(|runtime_event_encoded: Vec<u8>| {
		match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
			Ok(RuntimeEvent::ForeignAssets(pallet_asset_event)) => Some(pallet_asset_event),
			_ => None,
		}
	}),
	Box::new(|| {
		assert!(Assets::asset_ids().collect::<Vec<_>>().is_empty());
		assert!(ForeignAssets::asset_ids().collect::<Vec<_>>().is_empty());
	}),
	Box::new(|| {
		assert!(Assets::asset_ids().collect::<Vec<_>>().is_empty());
		assert_eq!(ForeignAssets::asset_ids().collect::<Vec<_>>().len(), 1);
	})
);

fn bridging_to_asset_hub_kusama() -> TestBridgingConfig {
	PolkadotXcm::force_xcm_version(
		RuntimeOrigin::root(),
		Box::new(bridging::to_kusama::AssetHubKusama::get()),
		XCM_VERSION,
	)
	.expect("version saved!");
	TestBridgingConfig {
		bridged_network: bridging::to_kusama::KusamaNetwork::get(),
		local_bridge_hub_para_id: bridging::SiblingBridgeHubParaId::get(),
		local_bridge_hub_location: bridging::SiblingBridgeHub::get(),
		bridged_target_location: bridging::to_kusama::AssetHubKusama::get(),
	}
}

#[test]
fn limited_reserve_transfer_assets_for_native_asset_to_asset_hub_kusama_works() {
	asset_test_utils::test_cases_over_bridge::limited_reserve_transfer_assets_for_native_asset_works::<
		Runtime,
		AllPalletsWithoutSystem,
		XcmConfig,
		ParachainSystem,
		XcmpQueue,
		LocationToAccountId,
	>(
		collator_session_keys(),
		slot_durations(),
		ExistentialDeposit::get(),
		AccountId::from(ALICE),
		Box::new(|runtime_event_encoded: Vec<u8>| {
			match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
				Ok(RuntimeEvent::PolkadotXcm(event)) => Some(event),
				_ => None,
			}
		}),
		Box::new(|runtime_event_encoded: Vec<u8>| {
			match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
				Ok(RuntimeEvent::XcmpQueue(event)) => Some(event),
				_ => None,
			}
		}),
		bridging_to_asset_hub_kusama,
		WeightLimit::Unlimited,
		Some(XcmBridgeHubRouterFeeAssetId::get()),
		Some(RelayTreasuryPalletAccount::get()),
	)
}

#[test]
fn receive_reserve_asset_deposited_ksm_from_asset_hub_kusama_fees_paid_by_pool_swap_works() {
	const BLOCK_AUTHOR_ACCOUNT: [u8; 32] = [13; 32];
	let block_author_account = AccountId::from(BLOCK_AUTHOR_ACCOUNT);
	let staking_pot = StakingPot::get();

	let foreign_asset_id_location =
		xcm::v3::Location::new(2, [xcm::v3::Junction::GlobalConsensus(xcm::v3::NetworkId::Kusama)]);
	let foreign_asset_id_minimum_balance = 1_000_000_000;
	// sovereign account as foreign asset owner (can be whoever for this scenario)
	let foreign_asset_owner = LocationToAccountId::convert_location(&Location::parent()).unwrap();
	let foreign_asset_create_params =
		(foreign_asset_owner, foreign_asset_id_location, foreign_asset_id_minimum_balance);

	asset_test_utils::test_cases_over_bridge::receive_reserve_asset_deposited_from_different_consensus_works::<
            Runtime,
            AllPalletsWithoutSystem,
            XcmConfig,
            ForeignAssetsInstance,
        >(
            collator_session_keys().add(collator_session_key(BLOCK_AUTHOR_ACCOUNT)),
            ExistentialDeposit::get(),
            AccountId::from([73; 32]),
            block_author_account.clone(),
            // receiving KSMs
            foreign_asset_create_params.clone(),
            1000000000000,
            || {
                // setup pool for paying fees to touch `SwapFirstAssetTrader`
                setup_pool_for_paying_fees_with_foreign_assets(foreign_asset_create_params);
                // staking pot account for collecting local native fees from `BuyExecution`
                let _ = Balances::force_set_balance(RuntimeOrigin::root(), StakingPot::get().into(), ExistentialDeposit::get());
                // prepare bridge configuration
                bridging_to_asset_hub_kusama()
            },
            (
                [PalletInstance(bp_bridge_hub_polkadot::WITH_BRIDGE_POLKADOT_TO_KUSAMA_MESSAGES_PALLET_INDEX)].into(),
                GlobalConsensus(Kusama),
                [Parachain(1000)].into()
            ),
            || {
                // check staking pot for ED
                assert_eq!(Balances::free_balance(&staking_pot), ExistentialDeposit::get());
                // check now foreign asset for staking pot
                assert_eq!(
                    ForeignAssets::balance(
                        foreign_asset_id_location,
                        &staking_pot
                    ),
                    0
                );
            },
            || {
                // `SwapFirstAssetTrader` - staking pot receives xcm fees in KSMs
                assert!(
                    Balances::free_balance(&staking_pot) > ExistentialDeposit::get()
                );
                // staking pot receives no foreign assets
                assert_eq!(
                    ForeignAssets::balance(
                        foreign_asset_id_location,
                        &staking_pot
                    ),
                    0
                );
            }
        )
}

#[test]
fn receive_reserve_asset_deposited_ksm_from_asset_hub_kusama_fees_paid_by_sufficient_asset_works() {
	const BLOCK_AUTHOR_ACCOUNT: [u8; 32] = [13; 32];
	let block_author_account = AccountId::from(BLOCK_AUTHOR_ACCOUNT);
	let staking_pot = <pallet_collator_selection::Pallet<Runtime>>::account_id();

	let foreign_asset_id_location =
		xcm::v3::Location::new(2, [xcm::v3::Junction::GlobalConsensus(xcm::v3::NetworkId::Kusama)]);
	let foreign_asset_id_minimum_balance = 1_000_000_000;
	// sovereign account as foreign asset owner (can be whoever for this scenario)
	let foreign_asset_owner = LocationToAccountId::convert_location(&Location::parent()).unwrap();
	let foreign_asset_create_params =
		(foreign_asset_owner, foreign_asset_id_location, foreign_asset_id_minimum_balance);

	asset_test_utils::test_cases_over_bridge::receive_reserve_asset_deposited_from_different_consensus_works::<
			Runtime,
			AllPalletsWithoutSystem,
			XcmConfig,
			ForeignAssetsInstance,
		>(
			collator_session_keys().add(collator_session_key(BLOCK_AUTHOR_ACCOUNT)),
			ExistentialDeposit::get(),
			AccountId::from([73; 32]),
			block_author_account.clone(),
			// receiving KSMs
			foreign_asset_create_params,
			1000000000000,
			bridging_to_asset_hub_kusama,
			(
				PalletInstance(bp_bridge_hub_polkadot::WITH_BRIDGE_POLKADOT_TO_KUSAMA_MESSAGES_PALLET_INDEX).into(),
				GlobalConsensus(Kusama),
				Parachain(1000).into()
			),
			|| {
				// check block author before
				assert_eq!(
					ForeignAssets::balance(
						foreign_asset_id_location,
						&block_author_account
					),
					0
				);
			},
			|| {
				// `TakeFirstAssetTrader` puts fees to the block author
				assert!(
					ForeignAssets::balance(
						foreign_asset_id_location,
						&block_author_account
					) > 0
				);
				// nothing adds fees to stakting_pot (e.g. `SwapFirstAssetTrader`, ...)
				assert_eq!(Balances::free_balance(&staking_pot), 0);
			}
		)
}

#[test]
fn reserve_transfer_native_asset_to_non_teleport_para_works() {
	asset_test_utils::test_cases::reserve_transfer_native_asset_to_non_teleport_para_works::<
		Runtime,
		AllPalletsWithoutSystem,
		XcmConfig,
		ParachainSystem,
		XcmpQueue,
		LocationToAccountId,
	>(
		collator_session_keys(),
		slot_durations(),
		ExistentialDeposit::get(),
		AccountId::from(ALICE),
		Box::new(|runtime_event_encoded: Vec<u8>| {
			match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
				Ok(RuntimeEvent::PolkadotXcm(event)) => Some(event),
				_ => None,
			}
		}),
		Box::new(|runtime_event_encoded: Vec<u8>| {
			match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
				Ok(RuntimeEvent::XcmpQueue(event)) => Some(event),
				_ => None,
			}
		}),
		WeightLimit::Unlimited,
	);
}

#[test]
fn report_bridge_status_from_xcm_bridge_router_for_kusama_works() {
	asset_test_utils::test_cases_over_bridge::report_bridge_status_from_xcm_bridge_router_works::<
		Runtime,
		AllPalletsWithoutSystem,
		XcmConfig,
		LocationToAccountId,
		ToKusamaXcmRouterInstance,
	>(
		collator_session_keys(),
		bridging_to_asset_hub_kusama,
		|| {
			Decode::decode(&mut &bp_asset_hub_polkadot::CongestedMessage::get().encode()[..])
				.unwrap()
		},
		|| {
			Decode::decode(&mut &bp_asset_hub_polkadot::UncongestedMessage::get().encode()[..])
				.unwrap()
		},
	)
}

#[test]
fn test_report_bridge_status_call_compatibility() {
	// if this test fails, make sure `bp_asset_hub_kusama` has valid encoding
	assert_eq!(
		RuntimeCall::ToKusamaXcmRouter(pallet_xcm_bridge_hub_router::Call::report_bridge_status {
			bridge_id: Default::default(),
			is_congested: true,
		})
		.encode(),
		bp_asset_hub_polkadot::Call::ToKusamaXcmRouter(
			bp_asset_hub_polkadot::XcmBridgeHubRouterCall::report_bridge_status {
				bridge_id: Default::default(),
				is_congested: true,
			}
		)
		.encode()
	)
}

#[test]
fn check_sane_weight_report_bridge_status() {
	use pallet_xcm_bridge_hub_router::WeightInfo;
	let actual = <Runtime as pallet_xcm_bridge_hub_router::Config<
			ToKusamaXcmRouterInstance,
		>>::WeightInfo::report_bridge_status();
	let max_weight = bp_asset_hub_polkadot::XcmBridgeHubRouterTransactCallMaxWeight::get();
	assert!(
		actual.all_lte(max_weight),
		"max_weight: {:?} should be adjusted to actual {:?}",
		max_weight,
		actual
	);
}

#[test]
fn change_xcm_bridge_hub_router_base_fee_by_governance_works() {
	asset_test_utils::test_cases::change_storage_constant_by_governance_works::<
		Runtime,
		bridging::XcmBridgeHubRouterBaseFee,
		Balance,
	>(
		collator_session_keys(),
		1000,
		Box::new(|call| RuntimeCall::System(call).encode()),
		|| {
			log::error!(
				target: "bridges::estimate",
				"`bridging::XcmBridgeHubRouterBaseFee` actual value: {} for runtime: {}",
				bridging::XcmBridgeHubRouterBaseFee::get(),
				<Runtime as frame_system::Config>::Version::get(),
			);
			(
				bridging::XcmBridgeHubRouterBaseFee::key().to_vec(),
				bridging::XcmBridgeHubRouterBaseFee::get(),
			)
		},
		|old_value| {
			if let Some(new_value) = old_value.checked_add(1) {
				new_value
			} else {
				old_value.checked_sub(1).unwrap()
			}
		},
	)
}

#[test]
fn change_xcm_bridge_hub_router_byte_fee_by_governance_works() {
	asset_test_utils::test_cases::change_storage_constant_by_governance_works::<
		Runtime,
		bridging::XcmBridgeHubRouterByteFee,
		Balance,
	>(
		collator_session_keys(),
		1000,
		Box::new(|call| RuntimeCall::System(call).encode()),
		|| {
			(
				bridging::XcmBridgeHubRouterByteFee::key().to_vec(),
				bridging::XcmBridgeHubRouterByteFee::get(),
			)
		},
		|old_value| {
			if let Some(new_value) = old_value.checked_add(1) {
				new_value
			} else {
				old_value.checked_sub(1).unwrap()
			}
		},
	)
}

#[test]
fn treasury_pallet_account_not_none() {
	assert_eq!(
		RelayTreasuryPalletAccount::get(),
		LocationToAccountId::convert_location(&RelayTreasuryLocation::get()).unwrap()
	)
}

#[test]
fn change_xcm_bridge_hub_ethereum_base_fee_by_governance_works() {
	asset_test_utils::test_cases::change_storage_constant_by_governance_works::<
		Runtime,
		bridging::to_ethereum::BridgeHubEthereumBaseFee,
		Balance,
	>(
		collator_session_keys(),
		1000,
		Box::new(|call| RuntimeCall::System(call).encode()),
		|| {
			(
				bridging::to_ethereum::BridgeHubEthereumBaseFee::key().to_vec(),
				bridging::to_ethereum::BridgeHubEthereumBaseFee::get(),
			)
		},
		|old_value| {
			if let Some(new_value) = old_value.checked_add(1) {
				new_value
			} else {
				old_value.checked_sub(1).unwrap()
			}
		},
	)
}
