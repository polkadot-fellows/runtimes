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

//! Tests for the Kusama Asset Hub (previously known as Statemine) chain.

use asset_hub_kusama_runtime::{
	xcm_config::{
		bridging::{self, XcmBridgeHubRouterFeeAssetId},
		CheckingAccount, ForeignCreatorsSovereignAccountOf, KsmLocation, LocationToAccountId,
		RelayTreasuryLocation, RelayTreasuryPalletAccount, StakingPot,
		TrustBackedAssetsPalletLocation, XcmConfig,
	},
	AllPalletsWithoutSystem, AssetConversion, AssetDeposit, Assets, Balances, ExistentialDeposit,
	ForeignAssets, ForeignAssetsInstance, MetadataDepositBase, MetadataDepositPerByte,
	ParachainSystem, PolkadotXcm, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin, SessionKeys,
	ToPolkadotXcmRouterInstance, TrustBackedAssetsInstance, XcmpQueue, SLOT_DURATION,
};
use asset_test_utils::{
	test_cases_over_bridge::TestBridgingConfig, CollatorSessionKey, CollatorSessionKeys, ExtBuilder,
};
use codec::{Decode, Encode};
use frame_support::{assert_ok, traits::fungibles::InspectEnumerable};
use parachains_common::{AccountId, AssetIdForTrustBackedAssets, AuraId, Balance};
use parachains_runtimes_test_utils::SlotDurations;
use sp_consensus_aura::SlotDuration;
use sp_core::crypto::Ss58Codec;
use sp_runtime::traits::MaybeEquivalence;
use sp_std::ops::Mul;
use system_parachains_constants::kusama::{
	consensus::RELAY_CHAIN_SLOT_DURATION_MILLIS, fee::WeightToFee,
};
use xcm::latest::prelude::{Assets as XcmAssets, *};
use xcm_builder::WithLatestLocationConverter;
use xcm_executor::traits::{ConvertLocation, JustTry};
use xcm_runtime_apis::conversions::LocationToAccountHelper;

const ALICE: [u8; 32] = [1u8; 32];
const SOME_ASSET_ADMIN: [u8; 32] = [5u8; 32];

type AssetIdForTrustBackedAssetsConvertLatest =
	assets_common::AssetIdForTrustBackedAssetsConvert<TrustBackedAssetsPalletLocation>;

type RuntimeHelper = asset_test_utils::RuntimeHelper<Runtime, AllPalletsWithoutSystem>;

fn collator_session_key(account: [u8; 32]) -> CollatorSessionKey<Runtime> {
	CollatorSessionKey::new(
		AccountId::from(account),
		AccountId::from(account),
		SessionKeys { aura: AuraId::from(sp_core::sr25519::Public::from_raw(account)) },
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
		xcm::v4::Location,
		Balance,
	),
) {
	let existential_deposit = ExistentialDeposit::get();

	// setup a pool to pay fees with `foreign_asset_id_location` tokens
	let pool_owner: AccountId = [14u8; 32].into();
	let native_asset = xcm::v4::Location::parent();
	let pool_liquidity: Balance =
		existential_deposit.max(foreign_asset_id_minimum_balance).mul(100_000);

	let _ = Balances::force_set_balance(
		RuntimeOrigin::root(),
		pool_owner.clone().into(),
		(existential_deposit + pool_liquidity).mul(2),
	);

	assert_ok!(ForeignAssets::mint(
		RuntimeOrigin::signed(foreign_asset_owner),
		foreign_asset_id_location.clone(),
		pool_owner.clone().into(),
		(foreign_asset_id_minimum_balance + pool_liquidity).mul(2),
	));

	assert_ok!(AssetConversion::create_pool(
		RuntimeOrigin::signed(pool_owner.clone()),
		Box::new(native_asset.clone()),
		Box::new(foreign_asset_id_location.clone())
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
			SessionKeys { aura: AuraId::from(sp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			let relay_ed = kusama_runtime_constants::currency::EXISTENTIAL_DEPOSIT;
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
			SessionKeys { aura: AuraId::from(sp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			let local_asset_id = 1;
			let foreign_asset_id_location = xcm::v4::Location::new(
				1,
				[xcm::v4::Junction::Parachain(1234), xcm::v4::Junction::GeneralIndex(12345)],
			);

			// check before
			assert_eq!(Assets::balance(local_asset_id, AccountId::from(ALICE)), 0);
			assert_eq!(
				ForeignAssets::balance(foreign_asset_id_location.clone(), AccountId::from(ALICE)),
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
				foreign_asset_id_location.clone(),
				AccountId::from(SOME_ASSET_ADMIN).into(),
				false,
				foreign_asset_minimum_asset_balance
			));

			// We first mint enough asset for the account to exist for assets
			assert_ok!(ForeignAssets::mint(
				RuntimeHelper::origin_of(AccountId::from(SOME_ASSET_ADMIN)),
				foreign_asset_id_location.clone(),
				AccountId::from(ALICE).into(),
				6 * foreign_asset_minimum_asset_balance
			));

			// check after
			assert_eq!(
				Assets::balance(local_asset_id, AccountId::from(ALICE)),
				minimum_asset_balance
			);
			assert_eq!(
				ForeignAssets::balance(foreign_asset_id_location.clone(), AccountId::from(ALICE)),
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
				&assets_common::fungible_conversion::convert_balance::<KsmLocation, Balance>(
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
				WithLatestLocationConverter::convert_back(&foreign_asset_id_location).unwrap(),
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

include_teleports_for_foreign_assets_works!(
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
	xcm::v4::Location,
	JustTry,
	collator_session_keys(),
	ExistentialDeposit::get(),
	xcm::v4::Location::new(
		1,
		[xcm::v4::Junction::Parachain(1313), xcm::v4::Junction::GeneralIndex(12345)]
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
	xcm::v4::Location,
	WithLatestLocationConverter<xcm::v4::Location>,
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

fn bridging_to_asset_hub_polkadot() -> TestBridgingConfig {
	PolkadotXcm::force_xcm_version(
		RuntimeOrigin::root(),
		Box::new(bridging::to_polkadot::AssetHubPolkadot::get()),
		XCM_VERSION,
	)
	.expect("version saved!");
	TestBridgingConfig {
		bridged_network: bridging::to_polkadot::PolkadotNetwork::get(),
		local_bridge_hub_para_id: bridging::SiblingBridgeHubParaId::get(),
		local_bridge_hub_location: bridging::SiblingBridgeHub::get(),
		bridged_target_location: bridging::to_polkadot::AssetHubPolkadot::get(),
	}
}

#[test]
fn limited_reserve_transfer_assets_for_native_asset_to_asset_hub_polkadot_works() {
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
		bridging_to_asset_hub_polkadot,
		WeightLimit::Unlimited,
		Some(XcmBridgeHubRouterFeeAssetId::get()),
		Some(RelayTreasuryPalletAccount::get()),
	)
}

#[test]
fn receive_reserve_asset_deposited_dot_from_asset_hub_polkadot_fees_paid_by_pool_swap_works() {
	const BLOCK_AUTHOR_ACCOUNT: [u8; 32] = [13; 32];
	let block_author_account = AccountId::from(BLOCK_AUTHOR_ACCOUNT);
	let staking_pot = StakingPot::get();

	let foreign_asset_id_location = xcm::v4::Location::new(
		2,
		[xcm::v4::Junction::GlobalConsensus(xcm::v4::NetworkId::Polkadot)],
	);
	let foreign_asset_id_minimum_balance = 1_000_000_000;
	// sovereign account as foreign asset owner (can be whoever for this scenario)
	let foreign_asset_owner = LocationToAccountId::convert_location(&Location::parent()).unwrap();
	let foreign_asset_create_params =
		(foreign_asset_owner, foreign_asset_id_location.clone(), foreign_asset_id_minimum_balance);

	remove_when_updated_to_stable2409::receive_reserve_asset_deposited_from_different_consensus_works::<
		Runtime,
		AllPalletsWithoutSystem,
		XcmConfig,
		ForeignAssetsInstance,
	>(
		collator_session_keys().add(collator_session_key(BLOCK_AUTHOR_ACCOUNT)),
		ExistentialDeposit::get(),
		AccountId::from([73; 32]),
		block_author_account,
		// receiving DOTs
		foreign_asset_create_params.clone(),
		1000000000000,
		|| {
			// setup pool for paying fees to touch `SwapFirstAssetTrader`
			setup_pool_for_paying_fees_with_foreign_assets(foreign_asset_create_params);
			// staking pot account for collecting local native fees from `BuyExecution`
			let _ = Balances::force_set_balance(RuntimeOrigin::root(), StakingPot::get().into(), ExistentialDeposit::get());
			// prepare bridge configuration
			bridging_to_asset_hub_polkadot()
		},
		(
			[PalletInstance(bp_bridge_hub_kusama::WITH_BRIDGE_KUSAMA_TO_POLKADOT_MESSAGES_PALLET_INDEX)].into(),
			GlobalConsensus(Polkadot),
			[Parachain(1000)].into()
		),
		|| {
			// check staking pot for ED
			assert_eq!(Balances::free_balance(&staking_pot), ExistentialDeposit::get());
			// check now foreign asset for staking pot
			assert_eq!(
				ForeignAssets::balance(
					foreign_asset_id_location.clone(),
					&staking_pot
				),
				0
			);
		},
		|| {
			// `SwapFirstAssetTrader` - staking pot receives xcm fees in DOTs
			assert!(
				Balances::free_balance(&staking_pot) > ExistentialDeposit::get()
			);
			// staking pot receives no foreign assets
			assert_eq!(
				ForeignAssets::balance(
					foreign_asset_id_location.clone(),
					&staking_pot
				),
				0
			);
		}
	)
}

#[test]
fn receive_reserve_asset_deposited_dot_from_asset_hub_polkadot_fees_paid_by_sufficient_asset_works()
{
	const BLOCK_AUTHOR_ACCOUNT: [u8; 32] = [13; 32];
	let block_author_account = AccountId::from(BLOCK_AUTHOR_ACCOUNT);
	let staking_pot = <pallet_collator_selection::Pallet<Runtime>>::account_id();

	let foreign_asset_id_location = xcm::v4::Location::new(
		2,
		[xcm::v4::Junction::GlobalConsensus(xcm::v4::NetworkId::Polkadot)],
	);
	let foreign_asset_id_minimum_balance = 1_000_000_000;
	// sovereign account as foreign asset owner (can be whoever for this scenario)
	let foreign_asset_owner = LocationToAccountId::convert_location(&Location::parent()).unwrap();
	let foreign_asset_create_params =
		(foreign_asset_owner, foreign_asset_id_location.clone(), foreign_asset_id_minimum_balance);

	remove_when_updated_to_stable2409::receive_reserve_asset_deposited_from_different_consensus_works::<
			Runtime,
			AllPalletsWithoutSystem,
			XcmConfig,
			ForeignAssetsInstance,
		>(
			collator_session_keys().add(collator_session_key(BLOCK_AUTHOR_ACCOUNT)),
			ExistentialDeposit::get(),
			AccountId::from([73; 32]),
			block_author_account.clone(),
			// receiving DOTs
			foreign_asset_create_params,
			1000000000000,
			bridging_to_asset_hub_polkadot,
			(
				PalletInstance(bp_bridge_hub_kusama::WITH_BRIDGE_KUSAMA_TO_POLKADOT_MESSAGES_PALLET_INDEX).into(),
				GlobalConsensus(Polkadot),
				Parachain(1000).into()
			),
			|| {
				// check block author before
				assert_eq!(
					ForeignAssets::balance(
						foreign_asset_id_location.clone(),
						&block_author_account
					),
					0
				);
			},
			|| {
				// `TakeFirstAssetTrader` puts fees to the block author
				assert!(
					ForeignAssets::balance(
						foreign_asset_id_location.clone(),
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
fn report_bridge_status_from_xcm_bridge_router_for_polkadot_works() {
	asset_test_utils::test_cases_over_bridge::report_bridge_status_from_xcm_bridge_router_works::<
		Runtime,
		AllPalletsWithoutSystem,
		XcmConfig,
		LocationToAccountId,
		ToPolkadotXcmRouterInstance,
	>(
		collator_session_keys(),
		bridging_to_asset_hub_polkadot,
		|| bp_asset_hub_kusama::build_congestion_message(Default::default(), true).into(),
		|| bp_asset_hub_kusama::build_congestion_message(Default::default(), false).into(),
	)
}

#[test]
fn test_report_bridge_status_call_compatibility() {
	// if this test fails, make sure `bp_asset_hub_polkadot` has valid encoding
	assert_eq!(
		RuntimeCall::ToPolkadotXcmRouter(
			pallet_xcm_bridge_hub_router::Call::report_bridge_status {
				bridge_id: Default::default(),
				is_congested: true,
			}
		)
		.encode(),
		bp_asset_hub_kusama::Call::ToPolkadotXcmRouter(
			bp_asset_hub_kusama::XcmBridgeHubRouterCall::report_bridge_status {
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
		ToPolkadotXcmRouterInstance,
	>>::WeightInfo::report_bridge_status();
	let max_weight = bp_asset_hub_kusama::XcmBridgeHubRouterTransactCallMaxWeight::get();
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

#[allow(clippy::too_many_arguments)]
pub mod remove_when_updated_to_stable2409 {
	use crate::{AccountId, Balance, TestBridgingConfig};

	use asset_test_utils::*;
	use codec::Encode;
	use core::fmt::Debug;
	use frame_support::{
		assert_ok,
		traits::{
			fungible::Mutate, Currency, OnFinalize, OnInitialize, OriginTrait, ProcessMessageError,
		},
	};
	use frame_system::pallet_prelude::BlockNumberFor;
	use sp_core::Get;
	use sp_runtime::traits::StaticLookup;
	use xcm::prelude::*;
	use xcm_builder::{CreateMatcher, MatchXcm};
	use xcm_executor::{traits::ConvertLocation, XcmExecutor};

	#[macro_export]
	macro_rules! include_teleports_for_foreign_assets_works(
		(
			$runtime:path,
			$all_pallets_without_system:path,
			$xcm_config:path,
			$checking_account:path,
			$weight_to_fee:path,
			$hrmp_channel_opener:path,
			$sovereign_account_of:path,
			$assets_pallet_instance:path,
			$collator_session_key:expr,
			$slot_durations:expr,
			$existential_deposit:expr,
			$unwrap_pallet_xcm_event:expr,
			$unwrap_xcmp_queue_event:expr
		) => {
			#[test]
			fn teleports_for_foreign_assets_works() {
				const BOB: [u8; 32] = [2u8; 32];
				let target_account = parachains_common::AccountId::from(BOB);
				const SOME_ASSET_OWNER: [u8; 32] = [5u8; 32];
				let asset_owner = parachains_common::AccountId::from(SOME_ASSET_OWNER);

				$crate::remove_when_updated_to_stable2409::teleports_for_foreign_assets_works::<
					$runtime,
					$all_pallets_without_system,
					$xcm_config,
					$checking_account,
					$weight_to_fee,
					$hrmp_channel_opener,
					$sovereign_account_of,
					$assets_pallet_instance
				>(
					$collator_session_key,
					$slot_durations,
					target_account,
					$existential_deposit,
					asset_owner,
					$unwrap_pallet_xcm_event,
					$unwrap_xcmp_queue_event
				)
			}
		}
	);

	/// Test-case makes sure that `Runtime` can receive teleported assets from sibling parachain,
	/// and can teleport it back
	pub fn teleports_for_foreign_assets_works<
		Runtime,
		AllPalletsWithoutSystem,
		XcmConfig,
		CheckingAccount,
		WeightToFee,
		HrmpChannelOpener,
		SovereignAccountOf,
		ForeignAssetsPalletInstance,
	>(
		collator_session_keys: CollatorSessionKeys<Runtime>,
		slot_durations: SlotDurations,
		target_account: AccountIdOf<Runtime>,
		existential_deposit: BalanceOf<Runtime>,
		asset_owner: AccountIdOf<Runtime>,
		unwrap_pallet_xcm_event: Box<dyn Fn(Vec<u8>) -> Option<pallet_xcm::Event<Runtime>>>,
		unwrap_xcmp_queue_event: Box<
			dyn Fn(Vec<u8>) -> Option<cumulus_pallet_xcmp_queue::Event<Runtime>>,
		>,
	) where
		Runtime: frame_system::Config
			+ pallet_balances::Config
			+ pallet_session::Config
			+ pallet_xcm::Config
			+ parachain_info::Config
			+ pallet_collator_selection::Config
			+ cumulus_pallet_parachain_system::Config
			+ cumulus_pallet_xcmp_queue::Config
			+ pallet_assets::Config<ForeignAssetsPalletInstance>
			+ pallet_timestamp::Config,
		AllPalletsWithoutSystem:
			OnInitialize<BlockNumberFor<Runtime>> + OnFinalize<BlockNumberFor<Runtime>>,
		AccountIdOf<Runtime>: Into<[u8; 32]>,
		ValidatorIdOf<Runtime>: From<AccountIdOf<Runtime>>,
		BalanceOf<Runtime>: From<Balance>,
		XcmConfig: xcm_executor::Config,
		CheckingAccount: Get<AccountIdOf<Runtime>>,
		HrmpChannelOpener: frame_support::inherent::ProvideInherent<
			Call = cumulus_pallet_parachain_system::Call<Runtime>,
		>,
		WeightToFee: frame_support::weights::WeightToFee<Balance = Balance>,
		<WeightToFee as frame_support::weights::WeightToFee>::Balance: From<u128> + Into<u128>,
		SovereignAccountOf: ConvertLocation<AccountIdOf<Runtime>>,
		<Runtime as pallet_assets::Config<ForeignAssetsPalletInstance>>::AssetId:
			From<xcm::v4::Location> + Into<xcm::v4::Location>,
		<Runtime as pallet_assets::Config<ForeignAssetsPalletInstance>>::AssetIdParameter:
			From<xcm::v4::Location> + Into<xcm::v4::Location>,
		<Runtime as pallet_assets::Config<ForeignAssetsPalletInstance>>::Balance:
			From<Balance> + Into<u128>,
		<Runtime as frame_system::Config>::AccountId:
			Into<<<Runtime as frame_system::Config>::RuntimeOrigin as OriginTrait>::AccountId>,
		<<Runtime as frame_system::Config>::Lookup as StaticLookup>::Source:
			From<<Runtime as frame_system::Config>::AccountId>,
		<Runtime as frame_system::Config>::AccountId: From<AccountId>,
		ForeignAssetsPalletInstance: 'static,
	{
		// foreign parachain with the same consensus currency as asset
		let foreign_para_id = 2222;
		let foreign_asset_id_location = xcm::v4::Location {
			parents: 1,
			interior: [
				xcm::v4::Junction::Parachain(foreign_para_id),
				xcm::v4::Junction::GeneralIndex(1234567),
			]
			.into(),
		};

		// foreign creator, which can be sibling parachain to match ForeignCreators
		let foreign_creator =
			Location { parents: 1, interior: [Parachain(foreign_para_id)].into() };
		let foreign_creator_as_account_id =
			SovereignAccountOf::convert_location(&foreign_creator).expect("");

		// we want to buy execution with local relay chain currency
		let buy_execution_fee_amount =
			WeightToFee::weight_to_fee(&Weight::from_parts(90_000_000_000, 0));
		let buy_execution_fee =
			Asset { id: AssetId(Location::parent()), fun: Fungible(buy_execution_fee_amount) };

		let teleported_foreign_asset_amount = 10_000_000_000_000;
		let runtime_para_id = 1000;
		ExtBuilder::<Runtime>::default()
			.with_collators(collator_session_keys.collators())
			.with_session_keys(collator_session_keys.session_keys())
			.with_balances(vec![
				(
					foreign_creator_as_account_id,
					existential_deposit + (buy_execution_fee_amount * 2).into(),
				),
				(target_account.clone(), existential_deposit),
				(CheckingAccount::get(), existential_deposit),
			])
			.with_safe_xcm_version(XCM_VERSION)
			.with_para_id(runtime_para_id.into())
			.with_tracing()
			.build()
			.execute_with(|| {
				let mut alice = [0u8; 32];
				alice[0] = 1;

				let included_head = RuntimeHelper::<Runtime, AllPalletsWithoutSystem>::run_to_block(
					2,
					AccountId::from(alice).into(),
				);
				// checks target_account before
				assert_eq!(
					<pallet_balances::Pallet<Runtime>>::free_balance(&target_account),
					existential_deposit
				);
				// check `CheckingAccount` before
				assert_eq!(
					<pallet_balances::Pallet<Runtime>>::free_balance(CheckingAccount::get()),
					existential_deposit
				);
				assert_eq!(
					<pallet_assets::Pallet<Runtime, ForeignAssetsPalletInstance>>::balance(
						foreign_asset_id_location.clone().into(),
						&target_account
					),
					0.into()
				);
				assert_eq!(
					<pallet_assets::Pallet<Runtime, ForeignAssetsPalletInstance>>::balance(
						foreign_asset_id_location.clone().into(),
						CheckingAccount::get()
					),
					0.into()
				);
				// check totals before
				assert_total::<
					pallet_assets::Pallet<Runtime, ForeignAssetsPalletInstance>,
					AccountIdOf<Runtime>,
				>(foreign_asset_id_location.clone(), 0, 0);

				// create foreign asset (0 total issuance)
				let asset_minimum_asset_balance = 3333333_u128;
				assert_ok!(
					<pallet_assets::Pallet<Runtime, ForeignAssetsPalletInstance>>::force_create(
						RuntimeHelper::<Runtime, ()>::root_origin(),
						foreign_asset_id_location.clone().into(),
						asset_owner.into(),
						false,
						asset_minimum_asset_balance.into()
					)
				);
				assert_total::<
					pallet_assets::Pallet<Runtime, ForeignAssetsPalletInstance>,
					AccountIdOf<Runtime>,
				>(foreign_asset_id_location.clone(), 0, 0);
				assert!(teleported_foreign_asset_amount > asset_minimum_asset_balance);

				// 1. process received teleported assets from sibling parachain (foreign_para_id)
				let xcm = Xcm(vec![
					// BuyExecution with relaychain native token
					WithdrawAsset(buy_execution_fee.clone().into()),
					BuyExecution {
						fees: Asset {
							id: AssetId(Location::parent()),
							fun: Fungible(buy_execution_fee_amount),
						},
						weight_limit: Limited(Weight::from_parts(403531000, 65536)),
					},
					// Process teleported asset
					ReceiveTeleportedAsset(Assets::from(vec![Asset {
						id: AssetId(foreign_asset_id_location.clone()),
						fun: Fungible(teleported_foreign_asset_amount),
					}])),
					DepositAsset {
						assets: Wild(AllOf {
							id: AssetId(foreign_asset_id_location.clone()),
							fun: WildFungibility::Fungible,
						}),
						beneficiary: Location {
							parents: 0,
							interior: [AccountId32 {
								network: None,
								id: target_account.clone().into(),
							}]
							.into(),
						},
					},
					ExpectTransactStatus(MaybeErrorCode::Success),
				]);
				let mut hash = xcm.using_encoded(sp_io::hashing::blake2_256);

				let outcome = XcmExecutor::<XcmConfig>::prepare_and_execute(
					foreign_creator,
					xcm,
					&mut hash,
					RuntimeHelper::<Runtime, ()>::xcm_max_weight(XcmReceivedFrom::Sibling),
					Weight::zero(),
				);
				assert_ok!(outcome.ensure_complete());

				// checks target_account after
				assert_eq!(
					<pallet_balances::Pallet<Runtime>>::free_balance(&target_account),
					existential_deposit
				);
				assert_eq!(
					<pallet_assets::Pallet<Runtime, ForeignAssetsPalletInstance>>::balance(
						foreign_asset_id_location.clone().into(),
						&target_account
					),
					teleported_foreign_asset_amount.into()
				);
				// checks `CheckingAccount` after
				assert_eq!(
					<pallet_balances::Pallet<Runtime>>::free_balance(CheckingAccount::get()),
					existential_deposit
				);
				assert_eq!(
					<pallet_assets::Pallet<Runtime, ForeignAssetsPalletInstance>>::balance(
						foreign_asset_id_location.clone().into(),
						CheckingAccount::get()
					),
					0.into()
				);
				// check total after (twice: target_account + CheckingAccount)
				assert_total::<
					pallet_assets::Pallet<Runtime, ForeignAssetsPalletInstance>,
					AccountIdOf<Runtime>,
				>(
					foreign_asset_id_location.clone(),
					teleported_foreign_asset_amount,
					teleported_foreign_asset_amount,
				);

				// 2. try to teleport asset back to source parachain (foreign_para_id)
				{
					let dest = Location::new(1, [Parachain(foreign_para_id)]);
					let mut dest_beneficiary = Location::new(1, [Parachain(foreign_para_id)])
						.appended_with(AccountId32 {
							network: None,
							id: sp_runtime::AccountId32::new([3; 32]).into(),
						})
						.unwrap();
					dest_beneficiary.reanchor(&dest, &XcmConfig::UniversalLocation::get()).unwrap();

					let target_account_balance_before_teleport =
						<pallet_assets::Pallet<Runtime, ForeignAssetsPalletInstance>>::balance(
							foreign_asset_id_location.clone().into(),
							&target_account,
						);
					let asset_to_teleport_away = asset_minimum_asset_balance * 3;
					assert!(
						asset_to_teleport_away <
							(target_account_balance_before_teleport -
								asset_minimum_asset_balance.into())
							.into()
					);

					// Make sure the target account has enough native asset to pay for delivery fees
					let delivery_fees =
						xcm_helpers::teleport_assets_delivery_fees::<XcmConfig::XcmSender>(
							(foreign_asset_id_location.clone(), asset_to_teleport_away).into(),
							0,
							Unlimited,
							dest_beneficiary.clone(),
							dest.clone(),
						);
					<pallet_balances::Pallet<Runtime>>::mint_into(
						&target_account,
						delivery_fees.into(),
					)
					.unwrap();

					assert_ok!(
						RuntimeHelper::<Runtime, ()>::do_teleport_assets::<HrmpChannelOpener>(
							RuntimeHelper::<Runtime, ()>::origin_of(target_account.clone()),
							dest,
							dest_beneficiary,
							(foreign_asset_id_location.clone(), asset_to_teleport_away),
							Some((runtime_para_id, foreign_para_id)),
							included_head,
							&alice,
							&slot_durations,
						)
					);

					// check balances
					assert_eq!(
						<pallet_assets::Pallet<Runtime, ForeignAssetsPalletInstance>>::balance(
							foreign_asset_id_location.clone().into(),
							&target_account
						),
						(target_account_balance_before_teleport - asset_to_teleport_away.into())
					);
					assert_eq!(
						<pallet_assets::Pallet<Runtime, ForeignAssetsPalletInstance>>::balance(
							foreign_asset_id_location.clone().into(),
							CheckingAccount::get()
						),
						0.into()
					);
					// check total after (twice: target_account + CheckingAccount)
					assert_total::<
						pallet_assets::Pallet<Runtime, ForeignAssetsPalletInstance>,
						AccountIdOf<Runtime>,
					>(
						foreign_asset_id_location.clone(),
						teleported_foreign_asset_amount - asset_to_teleport_away,
						teleported_foreign_asset_amount - asset_to_teleport_away,
					);

					// check events
					RuntimeHelper::<Runtime, ()>::assert_pallet_xcm_event_outcome(
						&unwrap_pallet_xcm_event,
						|outcome| {
							assert_ok!(outcome.ensure_complete());
						},
					);
					assert!(RuntimeHelper::<Runtime, ()>::xcmp_queue_message_sent(
						unwrap_xcmp_queue_event
					)
					.is_some());
				}
			})
	}

	/// Helper function to verify `xcm` contains all relevant instructions expected on destination
	/// chain as part of a reserve-asset-transfer.
	pub(crate) fn assert_matches_reserve_asset_deposited_instructions<RuntimeCall: Debug>(
		xcm: &mut Xcm<RuntimeCall>,
		expected_reserve_assets_deposited: &Assets,
		expected_beneficiary: &Location,
	) {
		let _ = xcm
			.0
			.matcher()
			.skip_inst_while(|inst| !matches!(inst, ReserveAssetDeposited(..)))
			.expect("no instruction ReserveAssetDeposited?")
			.match_next_inst(|instr| match instr {
				ReserveAssetDeposited(reserve_assets) => {
					assert_eq!(reserve_assets.clone(), expected_reserve_assets_deposited.clone());
					Ok(())
				},
				_ => Err(ProcessMessageError::BadFormat),
			})
			.expect("expected instruction ReserveAssetDeposited")
			.match_next_inst(|instr| match instr {
				ClearOrigin => Ok(()),
				_ => Err(ProcessMessageError::BadFormat),
			})
			.expect("expected instruction ClearOrigin")
			.match_next_inst(|instr| match instr {
				BuyExecution { .. } => Ok(()),
				_ => Err(ProcessMessageError::BadFormat),
			})
			.expect("expected instruction BuyExecution")
			.match_next_inst(|instr| match instr {
				DepositAsset { assets: _, beneficiary } if beneficiary == expected_beneficiary =>
					Ok(()),
				_ => Err(ProcessMessageError::BadFormat),
			})
			.expect("expected instruction DepositAsset");
	}

	pub fn receive_reserve_asset_deposited_from_different_consensus_works<
		Runtime,
		AllPalletsWithoutSystem,
		XcmConfig,
		ForeignAssetsPalletInstance,
	>(
		collator_session_keys: CollatorSessionKeys<Runtime>,
		existential_deposit: BalanceOf<Runtime>,
		target_account: AccountIdOf<Runtime>,
		block_author_account: AccountIdOf<Runtime>,
		(foreign_asset_owner, foreign_asset_id_location, foreign_asset_id_minimum_balance): (
			AccountIdOf<Runtime>,
			xcm::v4::Location,
			u128,
		),
		foreign_asset_id_amount_to_transfer: u128,
		prepare_configuration: impl FnOnce() -> TestBridgingConfig,
		(bridge_instance, universal_origin, descend_origin): (Junctions, Junction, Junctions), /* bridge adds origin manipulation on the way */
		additional_checks_before: impl FnOnce(),
		additional_checks_after: impl FnOnce(),
	) where
		Runtime: frame_system::Config
			+ pallet_balances::Config
			+ pallet_session::Config
			+ pallet_xcm::Config
			+ parachain_info::Config
			+ pallet_collator_selection::Config
			+ cumulus_pallet_parachain_system::Config
			+ cumulus_pallet_xcmp_queue::Config
			+ pallet_assets::Config<ForeignAssetsPalletInstance>
			+ pallet_timestamp::Config,
		AllPalletsWithoutSystem:
			OnInitialize<BlockNumberFor<Runtime>> + OnFinalize<BlockNumberFor<Runtime>>,
		AccountIdOf<Runtime>: Into<[u8; 32]> + From<[u8; 32]>,
		ValidatorIdOf<Runtime>: From<AccountIdOf<Runtime>>,
		BalanceOf<Runtime>: From<Balance> + Into<Balance>,
		XcmConfig: xcm_executor::Config,
		<Runtime as pallet_assets::Config<ForeignAssetsPalletInstance>>::AssetId:
			From<xcm::v4::Location> + Into<xcm::v4::Location>,
		<Runtime as pallet_assets::Config<ForeignAssetsPalletInstance>>::AssetIdParameter:
			From<xcm::v4::Location> + Into<xcm::v4::Location>,
		<Runtime as pallet_assets::Config<ForeignAssetsPalletInstance>>::Balance:
			From<Balance> + Into<u128> + From<u128>,
		<Runtime as frame_system::Config>::AccountId: Into<<<Runtime as frame_system::Config>::RuntimeOrigin as OriginTrait>::AccountId>
			+ Into<AccountId>,
		<<Runtime as frame_system::Config>::Lookup as StaticLookup>::Source:
			From<<Runtime as frame_system::Config>::AccountId>,
		ForeignAssetsPalletInstance: 'static,
	{
		ExtBuilder::<Runtime>::default()
			.with_collators(collator_session_keys.collators())
			.with_session_keys(collator_session_keys.session_keys())
			.with_tracing()
			.build()
			.execute_with(|| {
				// Set account as block author, who will receive fees
				RuntimeHelper::<Runtime, AllPalletsWithoutSystem>::run_to_block(
					2,
					block_author_account.clone(),
				);

				// drip 'ED' user target account
				let _ = <pallet_balances::Pallet<Runtime>>::deposit_creating(
					&target_account,
					existential_deposit,
				);

				// create foreign asset for wrapped/derived representation
				assert_ok!(
					<pallet_assets::Pallet<Runtime, ForeignAssetsPalletInstance>>::force_create(
						RuntimeHelper::<Runtime, AllPalletsWithoutSystem>::root_origin(),
						foreign_asset_id_location.clone().into(),
						foreign_asset_owner.into(),
						true, // is_sufficient=true
						foreign_asset_id_minimum_balance.into()
					)
				);

				// prepare bridge config
				let TestBridgingConfig { local_bridge_hub_location, .. } = prepare_configuration();

				// Balances before
				assert_eq!(
					<pallet_balances::Pallet<Runtime>>::free_balance(&target_account),
					existential_deposit.clone()
				);

				// ForeignAssets balances before
				assert_eq!(
					<pallet_assets::Pallet<Runtime, ForeignAssetsPalletInstance>>::balance(
						foreign_asset_id_location.clone().into(),
						&target_account
					),
					0.into()
				);

				// additional check before
				additional_checks_before();

				let expected_assets = Assets::from(vec![Asset {
					id: AssetId(foreign_asset_id_location.clone()),
					fun: Fungible(foreign_asset_id_amount_to_transfer),
				}]);
				let expected_beneficiary = Location::new(
					0,
					[AccountId32 { network: None, id: target_account.clone().into() }],
				);

				// Call received XCM execution
				let xcm = Xcm(vec![
					DescendOrigin(bridge_instance),
					UniversalOrigin(universal_origin),
					DescendOrigin(descend_origin),
					ReserveAssetDeposited(expected_assets.clone()),
					ClearOrigin,
					BuyExecution {
						fees: Asset {
							id: AssetId(foreign_asset_id_location.clone()),
							fun: Fungible(foreign_asset_id_amount_to_transfer),
						},
						weight_limit: Unlimited,
					},
					DepositAsset {
						assets: Wild(AllCounted(1)),
						beneficiary: expected_beneficiary.clone(),
					},
					SetTopic([
						220, 188, 144, 32, 213, 83, 111, 175, 44, 210, 111, 19, 90, 165, 191, 112,
						140, 247, 192, 124, 42, 17, 153, 141, 114, 34, 189, 20, 83, 69, 237, 173,
					]),
				]);
				assert_matches_reserve_asset_deposited_instructions(
					&mut xcm.clone(),
					&expected_assets,
					&expected_beneficiary,
				);

				let mut hash = xcm.using_encoded(sp_io::hashing::blake2_256);

				// execute xcm as XcmpQueue would do
				let outcome = XcmExecutor::<XcmConfig>::prepare_and_execute(
					local_bridge_hub_location,
					xcm,
					&mut hash,
					RuntimeHelper::<Runtime, AllPalletsWithoutSystem>::xcm_max_weight(
						XcmReceivedFrom::Sibling,
					),
					Weight::zero(),
				);
				assert_ok!(outcome.ensure_complete());

				// Balances after
				assert_eq!(
					<pallet_balances::Pallet<Runtime>>::free_balance(&target_account),
					existential_deposit.clone()
				);

				// ForeignAssets balances after
				assert!(
					<pallet_assets::Pallet<Runtime, ForeignAssetsPalletInstance>>::balance(
						foreign_asset_id_location.into(),
						&target_account
					) > 0.into()
				);

				// additional check after
				additional_checks_after();
			})
	}
}

#[test]
fn location_conversion_works() {
	let alice_32 =
		AccountId32 { network: None, id: polkadot_core_primitives::AccountId::from(ALICE).into() };
	let bob_20 = AccountKey20 { network: None, key: [123u8; 20] };

	// the purpose of hardcoded values is to catch an unintended location conversion logic change.
	struct TestCase {
		description: &'static str,
		location: Location,
		expected_account_id_str: &'static str,
	}

	let test_cases = vec![
		// DescribeTerminus
		TestCase {
			description: "DescribeTerminus Parent",
			location: Location::new(1, Here),
			expected_account_id_str: "5Dt6dpkWPwLaH4BBCKJwjiWrFVAGyYk3tLUabvyn4v7KtESG",
		},
		TestCase {
			description: "DescribeTerminus Sibling",
			location: Location::new(1, [Parachain(1111)]),
			expected_account_id_str: "5Eg2fnssmmJnF3z1iZ1NouAuzciDaaDQH7qURAy3w15jULDk",
		},
		// DescribePalletTerminal
		TestCase {
			description: "DescribePalletTerminal Parent",
			location: Location::new(1, [PalletInstance(50)]),
			expected_account_id_str: "5CnwemvaAXkWFVwibiCvf2EjqwiqBi29S5cLLydZLEaEw6jZ",
		},
		TestCase {
			description: "DescribePalletTerminal Sibling",
			location: Location::new(1, [Parachain(1111), PalletInstance(50)]),
			expected_account_id_str: "5GFBgPjpEQPdaxEnFirUoa51u5erVx84twYxJVuBRAT2UP2g",
		},
		// DescribeAccountId32Terminal
		TestCase {
			description: "DescribeAccountId32Terminal Parent",
			location: Location::new(1, [alice_32]),
			expected_account_id_str: "5DN5SGsuUG7PAqFL47J9meViwdnk9AdeSWKFkcHC45hEzVz4",
		},
		TestCase {
			description: "DescribeAccountId32Terminal Sibling",
			location: Location::new(1, [Parachain(1111), alice_32]),
			expected_account_id_str: "5DGRXLYwWGce7wvm14vX1Ms4Vf118FSWQbJkyQigY2pfm6bg",
		},
		// DescribeAccountKey20Terminal
		TestCase {
			description: "DescribeAccountKey20Terminal Parent",
			location: Location::new(1, [bob_20]),
			expected_account_id_str: "5CJeW9bdeos6EmaEofTUiNrvyVobMBfWbdQvhTe6UciGjH2n",
		},
		TestCase {
			description: "DescribeAccountKey20Terminal Sibling",
			location: Location::new(1, [Parachain(1111), bob_20]),
			expected_account_id_str: "5CE6V5AKH8H4rg2aq5KMbvaVUDMumHKVPPQEEDMHPy3GmJQp",
		},
		// DescribeTreasuryVoiceTerminal
		TestCase {
			description: "DescribeTreasuryVoiceTerminal Parent",
			location: Location::new(1, [Plurality { id: BodyId::Treasury, part: BodyPart::Voice }]),
			expected_account_id_str: "5CUjnE2vgcUCuhxPwFoQ5r7p1DkhujgvMNDHaF2bLqRp4D5F",
		},
		TestCase {
			description: "DescribeTreasuryVoiceTerminal Sibling",
			location: Location::new(
				1,
				[Parachain(1111), Plurality { id: BodyId::Treasury, part: BodyPart::Voice }],
			),
			expected_account_id_str: "5G6TDwaVgbWmhqRUKjBhRRnH4ry9L9cjRymUEmiRsLbSE4gB",
		},
		// DescribeBodyTerminal
		TestCase {
			description: "DescribeBodyTerminal Parent",
			location: Location::new(1, [Plurality { id: BodyId::Unit, part: BodyPart::Voice }]),
			expected_account_id_str: "5EBRMTBkDisEXsaN283SRbzx9Xf2PXwUxxFCJohSGo4jYe6B",
		},
		TestCase {
			description: "DescribeBodyTerminal Sibling",
			location: Location::new(
				1,
				[Parachain(1111), Plurality { id: BodyId::Unit, part: BodyPart::Voice }],
			),
			expected_account_id_str: "5DBoExvojy8tYnHgLL97phNH975CyT45PWTZEeGoBZfAyRMH",
		},
	];

	for tc in test_cases {
		let expected = polkadot_core_primitives::AccountId::from_string(tc.expected_account_id_str)
			.expect("Invalid AccountId string");

		let got = LocationToAccountHelper::<polkadot_core_primitives::AccountId, LocationToAccountId>::convert_location(
			tc.location.into(),
		)
			.unwrap();

		assert_eq!(got, expected, "{}", tc.description);
	}
}
