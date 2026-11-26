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
		bridging, CheckingAccount, DotLocation, LocationToAccountId, RelayChainLocation,
		TrustBackedAssetsPalletLocation, XcmConfig,
	},
	AllPalletsWithoutSystem, AssetDeposit, Assets, Balances, Block, Dap, ExistentialDeposit,
	ForeignAssets, ForeignAssetsInstance, MetadataDepositBase, MetadataDepositPerByte,
	ParachainSystem, PolkadotXcm, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin, SessionKeys,
	ToKusamaXcmRouterInstance, TrustBackedAssetsInstance, XcmpQueue, SLOT_DURATION,
};
use asset_test_utils::{
	include_create_and_manage_foreign_assets_for_local_consensus_parachain_assets_works,
	include_teleports_for_foreign_assets_works,
	test_cases_over_bridge::{
		receive_reserve_asset_deposited_from_different_consensus_works, TestBridgingConfig,
	},
	CollatorSessionKey, CollatorSessionKeys, ExtBuilder, GovernanceOrigin, SlotDurations,
};
use assets_common::local_and_foreign_assets::ForeignAssetReserveData;
use codec::{Decode, Encode};
use frame_support::{
	assert_err, assert_ok,
	traits::{fungibles::InspectEnumerable, ContainsPair},
	weights::Weight,
};
use parachains_common::{
	AccountId, AssetHubPolkadotAuraId as AuraId, AssetIdForTrustBackedAssets, Balance,
};
use sp_consensus_aura::SlotDuration;
use sp_core::crypto::Ss58Codec;
use sp_runtime::{traits::MaybeEquivalence, Either, TryRuntimeError};
use system_parachains_constants::polkadot::{
	consensus::RELAY_CHAIN_SLOT_DURATION_MILLIS, currency::UNITS,
	fee::WeightToFee as DotWeightToFee,
};
use xcm::latest::{
	prelude::{Assets as XcmAssets, *},
	WESTEND_GENESIS_HASH,
};
use xcm_builder::WithLatestLocationConverter;
use xcm_executor::traits::ConvertLocation;
use xcm_runtime_apis::conversions::LocationToAccountHelper;

const ALICE: [u8; 32] = [1u8; 32];
const BOB: [u8; 32] = [2u8; 32];
const SOME_ASSET_ADMIN: [u8; 32] = [5u8; 32];

frame_support::parameter_types! {
	// Local OpenGov
	pub Governance: GovernanceOrigin<RuntimeOrigin> = GovernanceOrigin::Origin(RuntimeOrigin::root());
}

type AssetIdForTrustBackedAssetsConvertLatest =
	assets_common::AssetIdForTrustBackedAssetsConvert<TrustBackedAssetsPalletLocation>;
type RuntimeHelper = asset_test_utils::RuntimeHelper<Runtime, AllPalletsWithoutSystem>;
type WeightToFee = DotWeightToFee<Runtime>;

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

#[test]
fn test_ed_is_one_hundredth_of_relay() {
	ExtBuilder::<Runtime>::default()
		.with_tracing()
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
		.with_tracing()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::ed25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			let local_asset_id = 1;
			let foreign_asset_id_location =
				Location::new(1, [Parachain(1234), GeneralIndex(12345)]);

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
	LocationToAccountId,
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
	Location,
	WithLatestLocationConverter<Location>,
	collator_session_keys(),
	ExistentialDeposit::get(),
	Location::new(1, [Parachain(1313), GeneralIndex(12345)]),
	Box::new(|| {
		assert!(Assets::asset_ids().collect::<Vec<_>>().is_empty());
	}),
	Box::new(|| {
		assert!(Assets::asset_ids().collect::<Vec<_>>().is_empty());
	})
);

include_create_and_manage_foreign_assets_for_local_consensus_parachain_assets_works!(
	Runtime,
	XcmConfig,
	WeightToFee,
	LocationToAccountId,
	ForeignAssetsInstance,
	Location,
	WithLatestLocationConverter<Location>,
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
		None,
		Some(pallet_dap::Pallet::<Runtime>::staging_account()),
	)
}

#[test]
fn receive_reserve_asset_deposited_ksm_from_asset_hub_kusama_fees_paid_by_pool_swap_works() {
	const BLOCK_AUTHOR_ACCOUNT: [u8; 32] = [13; 32];
	let block_author_account = AccountId::from(BLOCK_AUTHOR_ACCOUNT);
	let dap_staging_account = pallet_dap::Pallet::<Runtime>::staging_account();

	let foreign_asset_id_location_v5 = Location::new(2, [GlobalConsensus(NetworkId::Kusama)]);
	let reserve_location = Location::new(2, [GlobalConsensus(NetworkId::Kusama), Parachain(1000)]);
	let foreign_asset_reserve_data =
		ForeignAssetReserveData { reserve: reserve_location, teleportable: false };
	let foreign_asset_id_minimum_balance = 1_000_000_000;
	// sovereign account as foreign asset owner (can be whoever for this scenario)
	let foreign_asset_owner = LocationToAccountId::convert_location(&Location::parent()).unwrap();
	let foreign_asset_create_params = (
		foreign_asset_owner.clone(),
		foreign_asset_id_location_v5.clone(),
		foreign_asset_reserve_data,
		foreign_asset_id_minimum_balance,
	);
	let pool_params = (
		foreign_asset_owner,
		foreign_asset_id_location_v5.clone(),
		foreign_asset_id_minimum_balance,
	);

	receive_reserve_asset_deposited_from_different_consensus_works::<
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
		|| {
			// setup pool for paying fees to touch `SwapFirstAssetTrader`
			asset_test_utils::test_cases::setup_pool_for_paying_fees_with_foreign_assets::<
				Runtime,
				RuntimeOrigin,
			>(ExistentialDeposit::get(), pool_params);
			// DAP staging account for collecting local native fees from `BuyExecution`
			let _ = Balances::force_set_balance(
				RuntimeOrigin::root(),
				pallet_dap::Pallet::<Runtime>::staging_account().into(),
				ExistentialDeposit::get(),
			);
			// prepare bridge configuration
			bridging_to_asset_hub_kusama()
		},
		(
			[PalletInstance(
				bp_bridge_hub_polkadot::WITH_BRIDGE_POLKADOT_TO_KUSAMA_MESSAGES_PALLET_INDEX,
			)]
			.into(),
			GlobalConsensus(Kusama),
			[Parachain(1000)].into(),
		),
		|| {
			// check DAP staging account for ED
			assert_eq!(Balances::free_balance(&dap_staging_account), ExistentialDeposit::get());
			// check now foreign asset for DAP staging account
			assert_eq!(
				ForeignAssets::balance(foreign_asset_id_location_v5.clone(), &dap_staging_account),
				0
			);
		},
		|| {
			// `SwapFirstAssetTrader` - DAP staging account receives xcm fees in KSMs (swapped
			// to DOT)
			assert!(Balances::free_balance(&dap_staging_account) > ExistentialDeposit::get());
			// DAP staging account receives no foreign assets
			assert_eq!(
				ForeignAssets::balance(foreign_asset_id_location_v5.clone(), &dap_staging_account),
				0
			);
		},
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
		|| bp_asset_hub_polkadot::build_congestion_message(Default::default(), true).into(),
		|| bp_asset_hub_polkadot::build_congestion_message(Default::default(), false).into(),
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
		"max_weight: {max_weight:?} should be adjusted to actual {actual:?}"
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
		Governance::get(),
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
		Governance::get(),
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
fn change_xcm_bridge_hub_ethereum_base_fee_by_governance_works() {
	asset_test_utils::test_cases::change_storage_constant_by_governance_works::<
		Runtime,
		bridging::to_ethereum::BridgeHubEthereumBaseFee,
		Balance,
	>(
		collator_session_keys(),
		1000,
		Governance::get(),
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

#[test]
fn xcm_payment_api_works() {
	parachains_runtimes_test_utils::test_cases::xcm_payment_api_with_native_token_works::<
		Runtime,
		RuntimeCall,
		RuntimeOrigin,
		Block,
		WeightToFee,
	>();
	asset_test_utils::test_cases::xcm_payment_api_with_pools_works::<
		Runtime,
		RuntimeCall,
		RuntimeOrigin,
		Block,
		WeightToFee,
	>();
	asset_test_utils::test_cases::xcm_payment_api_foreign_asset_pool_works::<
		Runtime,
		RuntimeCall,
		RuntimeOrigin,
		LocationToAccountId,
		Block,
		WeightToFee,
	>(ExistentialDeposit::get(), WESTEND_GENESIS_HASH);
}

#[test]
fn test_xcm_v4_to_v5_works() {
	// Test some common XCM location patterns to ensure V4 -> V5 compatibility
	let test_locations_v4 = vec![
		// Relay chain
		xcm::v4::Location::new(1, xcm::v4::Junctions::Here),
		// Sibling parachain
		xcm::v4::Location::new(1, [xcm::v4::Junction::Parachain(1000)]),
		// Asset on sibling parachain
		xcm::v4::Location::new(
			1,
			[
				xcm::v4::Junction::Parachain(1000),
				xcm::v4::Junction::PalletInstance(50),
				xcm::v4::Junction::GeneralIndex(1984),
			],
		),
		// Global consensus location
		xcm::v4::Location::new(
			1,
			[xcm::v4::Junction::GlobalConsensus(xcm::v4::NetworkId::Polkadot)],
		),
	];

	for v4_location in test_locations_v4 {
		// Test V4 -> V5 conversion
		let v5_location = xcm::v5::Location::try_from(v4_location.clone())
			.map_err(|_| TryRuntimeError::Other("Failed to convert V4 location to V5"))
			.unwrap();

		// Test that we can encode/decode V5 location
		let encoded = v5_location.encode();
		let decoded = xcm::v5::Location::decode(&mut &encoded[..])
			.map_err(|_| TryRuntimeError::Other("Failed to decode V5 location"))
			.unwrap();

		assert_eq!(v5_location, decoded, "V5 location encode/decode round-trip failed");

		// Test V4 encoded -> V5 decoded compatibility
		let encoded_v4 = v4_location.encode();
		let decoded_v5 = xcm::v5::Location::decode(&mut &encoded_v4[..])
			.map_err(|_| TryRuntimeError::Other("Failed to decode V4 encoded location as V5"))
			.unwrap();

		// try-from is compatible
		assert_eq!(
			decoded_v5, v5_location,
			"V4 encoded -> V5 decoded should match try_from conversion"
		);

		// encode/decode is compatible
		assert_eq!(encoded_v4, decoded_v5.encode(), "V4 encoded should match V5 re-encoded");
	}
}

#[test]
fn authorized_aliases_work() {
	ExtBuilder::<Runtime>::default()
		.with_tracing()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::ed25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			use frame_support::traits::fungible::Mutate;
			let alice: AccountId = ALICE.into();
			let local_alice = Location::new(0, AccountId32 { network: None, id: ALICE });
			let alice_on_sibling_para =
				Location::new(1, [Parachain(42), AccountId32 { network: None, id: ALICE }]);
			let alice_on_relay = Location::new(1, AccountId32 { network: None, id: ALICE });
			let bob_on_relay = Location::new(1, AccountId32 { network: None, id: [42_u8; 32] });

			assert_ok!(Balances::mint_into(&alice, 2 * UNITS));

			// neither `alice_on_sibling_para`, `alice_on_relay`, `bob_on_relay` are allowed to
			// alias into `local_alice`
			for aliaser in [&alice_on_sibling_para, &alice_on_relay, &bob_on_relay] {
				assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(
					aliaser,
					&local_alice
				));
			}

			// Alice explicitly authorizes `alice_on_sibling_para` to alias her local account
			assert_ok!(PolkadotXcm::add_authorized_alias(
				RuntimeHelper::origin_of(alice.clone()),
				Box::new(alice_on_sibling_para.clone().into()),
				None
			));

			// `alice_on_sibling_para` now explicitly allowed to alias into `local_alice`
			assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(
				&alice_on_sibling_para,
				&local_alice
			));
			// as expected, `alice_on_relay` and `bob_on_relay` still can't alias into `local_alice`
			for aliaser in [&alice_on_relay, &bob_on_relay] {
				assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(
					aliaser,
					&local_alice
				));
			}

			// Alice explicitly authorizes `alice_on_relay` to alias her local account
			assert_ok!(PolkadotXcm::add_authorized_alias(
				RuntimeHelper::origin_of(alice.clone()),
				Box::new(alice_on_relay.clone().into()),
				None
			));
			// Now both `alice_on_relay` and `alice_on_sibling_para` can alias into her local
			// account
			for aliaser in [&alice_on_relay, &alice_on_sibling_para] {
				assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(
					aliaser,
					&local_alice
				));
			}

			// Alice removes authorization for `alice_on_relay` to alias her local account
			assert_ok!(PolkadotXcm::remove_authorized_alias(
				RuntimeHelper::origin_of(alice.clone()),
				Box::new(alice_on_relay.clone().into())
			));

			// `alice_on_relay` no longer allowed to alias into `local_alice`
			assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(
				&alice_on_relay,
				&local_alice
			));

			// `alice_on_sibling_para` still allowed to alias into `local_alice`
			assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(
				&alice_on_sibling_para,
				&local_alice
			));
		})
}

#[test]
fn governance_authorize_upgrade_works() {
	use polkadot_runtime_constants::system_parachain::{ASSET_HUB_ID, COLLECTIVES_ID};

	// no - random non-system para
	assert_err!(
		parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::Location(Location::new(1, Parachain(12334)))),
		Either::Right(InstructionError { index: 0, error: XcmError::Barrier })
	);
	// no - random system para
	assert_err!(
		parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::Location(Location::new(1, Parachain(1765)))),
		Either::Right(InstructionError { index: 1, error: XcmError::BadOrigin })
	);
	// no - AssetHub
	assert_err!(
		parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::Location(Location::new(1, Parachain(ASSET_HUB_ID)))),
		Either::Right(InstructionError { index: 1, error: XcmError::BadOrigin })
	);
	// no - Collectives
	assert_err!(
		parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::Location(Location::new(1, Parachain(COLLECTIVES_ID)))),
		Either::Right(InstructionError { index: 1, error: XcmError::BadOrigin })
	);
	// no - Collectives Voice of Fellows plurality
	assert_err!(
		parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::LocationAndDescendOrigin(
			Location::new(1, Parachain(COLLECTIVES_ID)),
			Plurality { id: BodyId::Technical, part: BodyPart::Voice }.into()
		)),
		Either::Right(InstructionError { index: 2, error: XcmError::BadOrigin })
	);

	// ok - relaychain
	assert_ok!(parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
		Runtime,
		RuntimeOrigin,
	>(GovernanceOrigin::Location(RelayChainLocation::get())));
}

/// A Staking proxy can add/remove a StakingOperator proxy for the account it is proxying.
#[test]
fn staking_proxy_can_manage_staking_operator() {
	use asset_hub_polkadot_runtime::{Proxy, ProxyType};
	use frame_support::traits::fungible::Mutate;
	use sp_runtime::traits::StaticLookup;

	ExtBuilder::<Runtime>::default()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::ed25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			// Given: Alice, Bob, and Carol with sufficient balance
			let alice: AccountId = ALICE.into();
			let bob: AccountId = [2u8; 32].into();
			let carol: AccountId = [3u8; 32].into();

			Balances::mint_into(&alice, 100 * UNITS).unwrap();
			Balances::mint_into(&bob, 100 * UNITS).unwrap();
			Balances::mint_into(&carol, 100 * UNITS).unwrap();

			// Given: Alice has Bob as her Staking proxy
			assert_ok!(Proxy::add_proxy(
				RuntimeOrigin::signed(alice.clone()),
				<Runtime as frame_system::Config>::Lookup::unlookup(bob.clone()),
				ProxyType::Staking,
				0
			));

			// When: Bob (via proxy) adds Carol as StakingOperator for Alice
			let add_call = RuntimeCall::Proxy(pallet_proxy::Call::add_proxy {
				delegate: <Runtime as frame_system::Config>::Lookup::unlookup(carol.clone()),
				proxy_type: ProxyType::StakingOperator,
				delay: 0,
			});
			assert_ok!(Proxy::proxy(
				RuntimeOrigin::signed(bob.clone()),
				<Runtime as frame_system::Config>::Lookup::unlookup(alice.clone()),
				None,
				Box::new(add_call)
			));

			// Then: Carol is Alice's StakingOperator proxy
			let alice_proxies = pallet_proxy::Proxies::<Runtime>::get(&alice);
			assert!(
				alice_proxies
					.0
					.iter()
					.any(|p| p.delegate == carol && p.proxy_type == ProxyType::StakingOperator),
				"Carol should be Alice's StakingOperator proxy"
			);

			// When: Bob tries to add an Any proxy for Alice
			let add_any_call = RuntimeCall::Proxy(pallet_proxy::Call::add_proxy {
				delegate: <Runtime as frame_system::Config>::Lookup::unlookup(carol.clone()),
				proxy_type: ProxyType::Any,
				delay: 0,
			});
			// proxy() returns Ok(()) but inner call result is in ProxyExecuted event
			assert_ok!(Proxy::proxy(
				RuntimeOrigin::signed(bob.clone()),
				<Runtime as frame_system::Config>::Lookup::unlookup(alice.clone()),
				None,
				Box::new(add_any_call),
			));

			// Then: The ProxyExecuted event should contain CallFiltered error
			let events = frame_system::Pallet::<Runtime>::events();
			let proxy_executed = events.iter().rev().find_map(|record| {
				if let RuntimeEvent::Proxy(pallet_proxy::Event::ProxyExecuted { result }) =
					&record.event
				{
					Some(*result)
				} else {
					None
				}
			});
			assert_eq!(
				proxy_executed,
				Some(Err(frame_system::Error::<Runtime>::CallFiltered.into())),
				"Inner call should fail with CallFiltered"
			);

			// And: Carol was NOT added as Any proxy
			let alice_proxies = pallet_proxy::Proxies::<Runtime>::get(&alice);
			assert!(
				!alice_proxies
					.0
					.iter()
					.any(|p| p.delegate == carol && p.proxy_type == ProxyType::Any),
				"Carol should NOT be Alice's Any proxy - Staking proxy cannot add Any"
			);

			// When: Bob (via proxy) removes Carol as StakingOperator for Alice
			let remove_call = RuntimeCall::Proxy(pallet_proxy::Call::remove_proxy {
				delegate: <Runtime as frame_system::Config>::Lookup::unlookup(carol.clone()),
				proxy_type: ProxyType::StakingOperator,
				delay: 0,
			});
			assert_ok!(Proxy::proxy(
				RuntimeOrigin::signed(bob.clone()),
				<Runtime as frame_system::Config>::Lookup::unlookup(alice.clone()),
				None,
				Box::new(remove_call)
			));

			// Then: Carol is no longer Alice's StakingOperator proxy
			let alice_proxies = pallet_proxy::Proxies::<Runtime>::get(&alice);
			assert!(
				!alice_proxies
					.0
					.iter()
					.any(|p| p.delegate == carol && p.proxy_type == ProxyType::StakingOperator),
				"Carol should no longer be Alice's StakingOperator proxy"
			);
		});
}

/// Verifies StakingOperator filter allows validator operations and session key management,
/// but forbids fund management.
#[test]
fn staking_operator_filter_allows_validator_ops_and_session_keys() {
	use asset_hub_polkadot_runtime::ProxyType;
	use frame_support::traits::InstanceFilter;
	use pallet_staking_async::{Call as StakingCall, RewardDestination, ValidatorPrefs};
	use pallet_staking_async_rc_client::Call as RcClientCall;

	let operator = ProxyType::StakingOperator;

	// StakingOperator can perform validator operations
	assert!(operator
		.filter(&RuntimeCall::Staking(StakingCall::validate { prefs: ValidatorPrefs::default() })));
	assert!(operator.filter(&RuntimeCall::Staking(StakingCall::chill {})));
	assert!(operator.filter(&RuntimeCall::Staking(StakingCall::kick { who: vec![] })));

	// StakingOperator can manage session keys
	assert!(operator.filter(&RuntimeCall::StakingRcClient(RcClientCall::set_keys {
		keys: Default::default(),
		proof: Default::default(),
		max_delivery_and_remote_execution_fee: None,
	})));
	assert!(operator.filter(&RuntimeCall::StakingRcClient(RcClientCall::purge_keys {
		max_delivery_and_remote_execution_fee: None,
	})));

	// StakingOperator can batch operations
	assert!(operator.filter(&RuntimeCall::Utility(pallet_utility::Call::batch { calls: vec![] })));
	assert!(
		operator.filter(&RuntimeCall::Utility(pallet_utility::Call::batch_all { calls: vec![] }))
	);
	assert!(
		operator.filter(&RuntimeCall::Utility(pallet_utility::Call::force_batch { calls: vec![] }))
	);

	// StakingOperator cannot use other utility calls
	assert!(!operator.filter(&RuntimeCall::Utility(pallet_utility::Call::as_derivative {
		index: 0,
		call: Box::new(RuntimeCall::System(frame_system::Call::remark { remark: vec![] })),
	})));
	assert!(!operator.filter(&RuntimeCall::Utility(pallet_utility::Call::dispatch_as {
		as_origin: Box::new(asset_hub_polkadot_runtime::OriginCaller::system(
			frame_system::RawOrigin::Root,
		)),
		call: Box::new(RuntimeCall::System(frame_system::Call::remark { remark: vec![] })),
	})));
	assert!(!operator.filter(&RuntimeCall::Utility(pallet_utility::Call::with_weight {
		call: Box::new(RuntimeCall::System(frame_system::Call::remark { remark: vec![] })),
		weight: Default::default(),
	})));

	// StakingOperator cannot manage funds or nominations
	assert!(!operator.filter(&RuntimeCall::Staking(StakingCall::bond {
		value: 100,
		payee: RewardDestination::Staked
	})));
	assert!(!operator.filter(&RuntimeCall::Staking(StakingCall::unbond { value: 100 })));
	assert!(!operator.filter(&RuntimeCall::Staking(StakingCall::nominate { targets: vec![] })));
	assert!(!operator
		.filter(&RuntimeCall::Staking(StakingCall::update_payee { controller: [0u8; 32].into() })));
}

/// Test that a pure proxy stash can delegate to a StakingOperator
/// who can then call validate, chill, and manage session keys.
#[test]
fn pure_proxy_stash_can_delegate_to_staking_operator() {
	use asset_hub_polkadot_runtime::ProxyType;

	let controller: AccountId = ALICE.into();
	let operator: AccountId = [2u8; 32].into();

	ExtBuilder::<Runtime>::default()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::ed25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			use frame_support::traits::fungible::Mutate;

			// GIVEN: fund controller and operator
			assert_ok!(Balances::mint_into(&controller, 100 * UNITS));
			assert_ok!(Balances::mint_into(&operator, 100 * UNITS));

			// WHEN: controller creates a pure proxy stash with Staking proxy type
			assert_ok!(asset_hub_polkadot_runtime::Proxy::create_pure(
				RuntimeOrigin::signed(controller.clone()),
				ProxyType::Staking,
				0,
				0
			));
			let pure_stash = asset_hub_polkadot_runtime::Proxy::pure_account(
				&controller,
				&ProxyType::Staking,
				0,
				None,
			);

			// Fund the pure proxy stash
			assert_ok!(Balances::mint_into(&pure_stash, 100 * UNITS));

			// WHEN: controller (via Staking proxy) adds StakingOperator proxy for the operator
			let add_operator_call = RuntimeCall::Proxy(pallet_proxy::Call::add_proxy {
				delegate: operator.clone().into(),
				proxy_type: ProxyType::StakingOperator,
				delay: 0,
			});
			assert_ok!(asset_hub_polkadot_runtime::Proxy::proxy(
				RuntimeOrigin::signed(controller.clone()),
				pure_stash.clone().into(),
				None,
				Box::new(add_operator_call),
			));

			// THEN: operator can call chill on behalf of pure proxy stash
			let chill_call = RuntimeCall::Staking(pallet_staking_async::Call::chill {});
			assert_ok!(asset_hub_polkadot_runtime::Proxy::proxy(
				RuntimeOrigin::signed(operator.clone()),
				pure_stash.clone().into(),
				None,
				Box::new(chill_call),
			));

			// THEN: operator can call validate on behalf of pure proxy stash
			let validate_call = RuntimeCall::Staking(pallet_staking_async::Call::validate {
				prefs: Default::default(),
			});
			assert_ok!(asset_hub_polkadot_runtime::Proxy::proxy(
				RuntimeOrigin::signed(operator.clone()),
				pure_stash.clone().into(),
				None,
				Box::new(validate_call),
			));

			// THEN: operator can call purge_keys (session key management on AssetHub)
			let purge_keys_call =
				RuntimeCall::StakingRcClient(pallet_staking_async_rc_client::Call::purge_keys {
					max_delivery_and_remote_execution_fee: None,
				});
			assert_ok!(asset_hub_polkadot_runtime::Proxy::proxy(
				RuntimeOrigin::signed(operator.clone()),
				pure_stash.clone().into(),
				None,
				Box::new(purge_keys_call),
			));

			// THEN: operator CANNOT call bond (fund management is forbidden)
			let bond_call = RuntimeCall::Staking(pallet_staking_async::Call::bond {
				value: 10 * UNITS,
				payee: pallet_staking_async::RewardDestination::Staked,
			});
			assert_ok!(asset_hub_polkadot_runtime::Proxy::proxy(
				RuntimeOrigin::signed(operator.clone()),
				pure_stash.clone().into(),
				None,
				Box::new(bond_call),
			));
			// Check that the proxied call failed due to filter (CallFiltered error)
			frame_system::Pallet::<Runtime>::assert_last_event(
				pallet_proxy::Event::ProxyExecuted {
					result: Err(frame_system::Error::<Runtime>::CallFiltered.into()),
				}
				.into(),
			);
		});
}

#[test]
fn slash_goes_to_dap_buffer_account() {
	use frame_support::traits::{
		fungible::{Balanced, Inspect},
		Hooks, OnUnbalanced,
	};
	use sp_runtime::BuildStorage;

	let dap_buffer = pallet_dap::Pallet::<Runtime>::buffer_account();
	let dap_staging = pallet_dap::Pallet::<Runtime>::staging_account();
	let ed = ExistentialDeposit::get();

	let mut t = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();
	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![
			(AccountId::from(ALICE), 1_000 * UNITS),
			(dap_buffer.clone(), ed),
			(dap_staging.clone(), ed),
		],
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();

	sp_io::TestExternalities::from(t).execute_with(|| {
		let dap_buffer_before = <Balances as Inspect<_>>::balance(&dap_buffer);
		let dap_staging_before = <Balances as Inspect<_>>::balance(&dap_staging);

		// When: a slash occurs (simulating staking slash via OnUnbalanced)
		let slash_amount = 100 * UNITS;
		let credit = <Balances as Balanced<AccountId>>::issue(slash_amount);
		Dap::on_unbalanced(credit);

		// Slash lands in staging first, not directly in the buffer.
		assert_eq!(
			<Balances as Inspect<_>>::balance(&dap_staging),
			dap_staging_before + slash_amount
		);
		assert_eq!(<Balances as Inspect<_>>::balance(&dap_buffer), dap_buffer_before);

		// on_idle drains staging into buffer and deactivates.
		pallet_dap::Pallet::<Runtime>::on_idle(1, Weight::MAX);
		assert_eq!(<Balances as Inspect<_>>::balance(&dap_staging), dap_staging_before);
		assert_eq!(
			<Balances as Inspect<_>>::balance(&dap_buffer),
			dap_buffer_before + slash_amount
		);

		// When: another slash occurs
		let slash_amount_2 = 50 * UNITS;
		let credit2 = <Balances as Balanced<AccountId>>::issue(slash_amount_2);
		Dap::on_unbalanced(credit2);

		// Lands in staging again.
		assert_eq!(
			<Balances as Inspect<_>>::balance(&dap_staging),
			dap_staging_before + slash_amount_2
		);
		assert_eq!(
			<Balances as Inspect<_>>::balance(&dap_buffer),
			dap_buffer_before + slash_amount
		);

		// After on_idle: staging drains into buffer and deactivates.
		pallet_dap::Pallet::<Runtime>::on_idle(2, Weight::MAX);
		assert_eq!(<Balances as Inspect<_>>::balance(&dap_staging), dap_staging_before);
		assert_eq!(
			<Balances as Inspect<_>>::balance(&dap_buffer),
			dap_buffer_before + slash_amount + slash_amount_2
		);
	});
}

#[test]
fn session_keys_are_compatible_between_ah_and_rc() {
	use asset_hub_polkadot_runtime::staking::RelayChainSessionKeys;
	use sp_runtime::traits::OpaqueKeys;

	// Verify the key type IDs match in order.
	// This ensures that when keys are encoded on AssetHub and decoded on Polkadot (or vice versa),
	// they map to the correct key types.
	assert_eq!(
		RelayChainSessionKeys::key_ids(),
		polkadot_runtime::SessionKeys::key_ids(),
		"Session key type IDs must match between AssetHub and Polkadot"
	);
}

/// Dust reaches the DAP instead of being burned, and is deactivated in the buffer.
#[test]
fn dust_goes_to_dap_and_is_deactivated() {
	use frame_support::traits::{
		fungible::{Inspect, Mutate},
		tokens::Preservation,
		Hooks,
	};
	use sp_runtime::BuildStorage;

	let dap_buffer = pallet_dap::Pallet::<Runtime>::buffer_account();
	let dap_staging = pallet_dap::Pallet::<Runtime>::staging_account();
	let ed = ExistentialDeposit::get();

	let mut t = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();
	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![
			(AccountId::from(ALICE), ed),
			(AccountId::from(BOB), ed),
			(dap_buffer.clone(), ed),
			(dap_staging.clone(), ed),
		],
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();

	sp_io::TestExternalities::from(t).execute_with(|| {
		let issuance_before = Balances::total_issuance();
		let inactive_before = Balances::inactive_issuance();
		let staging_before = <Balances as Inspect<_>>::balance(&dap_staging);
		let buffer_before = <Balances as Inspect<_>>::balance(&dap_buffer);

		// Reap Alice, leaving dust behind.
		let dust = ed / 2;
		assert_ok!(<Balances as Mutate<_>>::transfer(
			&AccountId::from(ALICE),
			&AccountId::from(BOB),
			ed - dust,
			Preservation::Expendable,
		));

		assert_eq!(<Balances as Inspect<_>>::balance(&AccountId::from(ALICE)), 0);
		assert_eq!(<Balances as Inspect<_>>::balance(&dap_staging), staging_before + dust);
		assert_eq!(Balances::total_issuance(), issuance_before);

		// The drain deactivates it: still issued, no longer active.
		pallet_dap::Pallet::<Runtime>::on_idle(1, Weight::MAX);

		assert_eq!(<Balances as Inspect<_>>::balance(&dap_staging), staging_before);
		assert_eq!(<Balances as Inspect<_>>::balance(&dap_buffer), buffer_before + dust);
		assert_eq!(Balances::total_issuance(), issuance_before);
		assert_eq!(Balances::inactive_issuance(), inactive_before + dust);
	});
}

/// `Preserve` keeps staging at exactly its ED, so it is never reapable and the chain in
/// https://github.com/paritytech/polkadot-sdk/issues/12130 never starts.
#[test]
fn dap_staging_account_keeps_ed_through_drain() {
	use frame_support::traits::{
		fungible::{Inspect, Mutate},
		tokens::{DepositConsequence, Preservation, Provenance},
		Hooks,
	};
	use sp_runtime::BuildStorage;

	let dap_buffer = pallet_dap::Pallet::<Runtime>::buffer_account();
	let dap_staging = pallet_dap::Pallet::<Runtime>::staging_account();
	let ed = ExistentialDeposit::get();

	let mut t = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();
	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![
			(AccountId::from(ALICE), ed),
			(AccountId::from(BOB), ed),
			(dap_buffer.clone(), ed),
			(dap_staging.clone(), ed),
		],
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();

	sp_io::TestExternalities::from(t).execute_with(|| {
		// Draining an empty staging account leaves the ED.
		pallet_dap::Pallet::<Runtime>::on_idle(1, Weight::MAX);
		assert_eq!(<Balances as Inspect<_>>::balance(&dap_staging), ed);

		// Dust arrives and drains in one pass, leaving the ED, not reaped.
		let dust = ed / 2;
		assert_ok!(<Balances as Mutate<_>>::transfer(
			&AccountId::from(ALICE),
			&AccountId::from(BOB),
			ed - dust,
			Preservation::Expendable,
		));
		assert_eq!(<Balances as Inspect<_>>::balance(&dap_staging), ed + dust);

		pallet_dap::Pallet::<Runtime>::on_idle(2, Weight::MAX);
		assert_eq!(<Balances as Inspect<_>>::balance(&dap_staging), ed);
		assert!(frame_system::Pallet::<Runtime>::account_exists(&dap_staging));

		// Routed dust cannot bounce back out: a sub-ED deposit is refused.
		assert_eq!(
			<Balances as Inspect<_>>::can_deposit(
				&AccountId::from(ALICE),
				dust,
				Provenance::Extant
			),
			DepositConsequence::BelowMinimum
		);
	});
}

/// Even if that chain were to start, it stops after one call: the deposit back into the emptied
/// staging account fails `BelowMinimum`, so no fresh dust appears. Recursion would blow the stack.
#[test]
#[should_panic(expected = "Failed to deposit slash to DAP staging account")]
fn dust_removal_terminates_when_staging_is_dusted() {
	use frame_support::traits::{fungible::Mutate, tokens::Preservation};
	use sp_runtime::BuildStorage;

	let dap_staging = pallet_dap::Pallet::<Runtime>::staging_account();
	let ed = ExistentialDeposit::get();

	let mut t = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();
	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![(AccountId::from(BOB), ed), (dap_staging.clone(), ed)],
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();

	sp_io::TestExternalities::from(t).execute_with(|| {
		// Reap staging itself, so its dust routes back into it.
		let dust = ed / 2;
		let _ = <Balances as Mutate<_>>::transfer(
			&dap_staging,
			&AccountId::from(BOB),
			ed - dust,
			Preservation::Expendable,
		);
	});
}

/// Regression test for the dotUSD supply drift fixed by #1334.
///
/// In `staging-xcm-executor` < 28.1.0 a failing `WithdrawAsset` rolled back the storage of the
/// assets already withdrawn by the same instruction, but dropped their credit only afterwards. The
/// credit drop decreased the asset's `supply` while the account balances stayed unchanged.
#[test]
fn failed_withdraw_asset_does_not_decrease_trust_backed_asset_supply() {
	use frame_support::traits::fungibles::{Inspect, Mutate};
	use xcm_executor::XcmExecutor;

	const DOT_USD: AssetIdForTrustBackedAssets = 7873;
	const AMOUNT: Balance = 1_000 * 1_000_000;

	ExtBuilder::<Runtime>::default().build().execute_with(|| {
		let alice = AccountId::from(ALICE);
		assert_ok!(Assets::force_create(
			RuntimeHelper::root_origin(),
			DOT_USD.into(),
			AccountId::from(SOME_ASSET_ADMIN).into(),
			true,
			1,
		));
		assert_ok!(Assets::mint_into(DOT_USD, &alice, AMOUNT));
		assert_eq!(Assets::total_issuance(DOT_USD), AMOUNT);

		let dot_usd = AssetIdForTrustBackedAssetsConvertLatest::convert_back(&DOT_USD).unwrap();
		// dotUSD sorts before DOT, so it is withdrawn first and the DOT withdrawal (Alice has
		// none) fails afterwards.
		let xcm = Xcm::<RuntimeCall>(vec![WithdrawAsset(
			vec![(dot_usd, AMOUNT).into(), (DotLocation::get(), UNITS).into()].into(),
		)]);

		let mut hash = xcm.using_encoded(sp_io::hashing::blake2_256);
		let outcome = XcmExecutor::<XcmConfig>::prepare_and_execute(
			Location::new(0, [AccountId32 { network: None, id: ALICE }]),
			xcm,
			&mut hash,
			Weight::MAX,
			Weight::MAX,
		);
		assert!(matches!(outcome, Outcome::Incomplete { .. }), "unexpected outcome: {outcome:?}");

		assert_eq!(Assets::balance(DOT_USD, &alice), AMOUNT);
		assert_eq!(Assets::total_issuance(DOT_USD), AMOUNT);
	});
}

#[test]
fn session_key_deposit_works() {
	system_parachains_test_utils::session_key_deposit_works::<Runtime>(|owner| {
		let generated = SessionKeys::generate(&owner.encode(), None);
		(generated.keys, generated.proof.encode())
	});
}

/// The addresses that emitted an ERC-20 `Transfer` log in the current block.
fn erc20_transfer_log_emitters() -> Vec<sp_core::H160> {
	let transfer = sp_core::H256(sp_io::hashing::keccak_256(b"Transfer(address,address,uint256)"));
	asset_hub_polkadot_runtime::System::events()
		.into_iter()
		.filter_map(|record| match record.event {
			RuntimeEvent::Revive(pallet_revive::Event::ContractEmitted {
				contract,
				topics,
				..
			}) if topics.first() == Some(&transfer) => Some(contract),
			_ => None,
		})
		.collect()
}

/// The address of the `ERC20` precompile with the given `prefix` for the asset with `index`.
fn erc20_precompile_address(index: u32, prefix: u16) -> sp_core::H160 {
	let mut address = [0u8; 20];
	address[..4].copy_from_slice(&index.to_be_bytes());
	address[16..18].copy_from_slice(&prefix.to_be_bytes());
	sp_core::H160(address)
}

/// Every `pallet-assets` instance mirrors its balance changes as ERC-20 `Transfer` logs, emitted
/// from the address of the instance's `ERC20` precompile.
///
/// The `ERC20` precompile itself emits no `Transfer` log for a nonzero transfer. It relies on
/// `Erc20TransferLogsCallback` being the `CallbackHandle` of the instance, with the address prefix
/// of the instance's precompile.
///
/// A missing callback leaves the list of emitters empty, and a callback with another prefix fails
/// the address check. The expected prefixes are literals because changing a prefix moves the
/// address of every live token. The test does not call the precompiles, so it does not catch a
/// precompile registered with the wrong prefix or instance.
#[test]
fn asset_balance_changes_are_mirrored_as_erc20_transfer_logs() {
	use asset_hub_polkadot_runtime::{PoolAssets, System};

	ExtBuilder::<Runtime>::default()
		.with_collators(collator_session_keys().collators())
		.with_session_keys(collator_session_keys().session_keys())
		.build()
		.execute_with(|| {
			let owner = AccountId::from(ALICE);
			let recipient = AccountId::from(SOME_ASSET_ADMIN);
			let root = RuntimeHelper::root_origin;
			let signed = || RuntimeHelper::origin_of(owner.clone());

			// Trust-backed assets: A mint and a transfer are each mirrored once.
			let trust_backed_id: AssetIdForTrustBackedAssets = 11;
			assert_ok!(Assets::force_create(
				root(),
				trust_backed_id.into(),
				owner.clone().into(),
				true,
				1
			));
			System::reset_events();
			assert_ok!(Assets::mint(signed(), trust_backed_id.into(), owner.clone().into(), 1_000));
			assert_ok!(Assets::transfer(
				signed(),
				trust_backed_id.into(),
				recipient.clone().into(),
				100
			));
			assert_eq!(
				erc20_transfer_log_emitters(),
				vec![erc20_precompile_address(trust_backed_id, 0x0120); 2]
			);

			// Pool assets.
			let pool_id = 12;
			assert_ok!(PoolAssets::force_create(root(), pool_id, owner.clone().into(), true, 1));
			System::reset_events();
			assert_ok!(PoolAssets::mint(signed(), pool_id, owner.clone().into(), 1_000));
			assert_ok!(PoolAssets::transfer(signed(), pool_id, recipient.clone().into(), 100));
			assert_eq!(
				erc20_transfer_log_emitters(),
				vec![erc20_precompile_address(pool_id, 0x0320); 2]
			);

			// Foreign assets: The index is not the asset ID but is assigned on creation. Create
			// another asset first so that the one under test does not get index zero.
			let other_foreign_id = Location::new(1, [Parachain(2000)]);
			let foreign_id = Location::new(1, [Parachain(2001)]);
			for id in [&other_foreign_id, &foreign_id] {
				assert_ok!(ForeignAssets::force_create(
					root(),
					id.clone(),
					owner.clone().into(),
					true,
					1
				));
			}
			let foreign_index =
				pallet_assets_precompiles::pallet::Pallet::<Runtime>::asset_index_of(&foreign_id)
					.expect("creating a foreign asset assigns it an index");
			System::reset_events();
			assert_ok!(ForeignAssets::mint(
				signed(),
				foreign_id.clone(),
				owner.clone().into(),
				1_000
			));
			assert_ok!(ForeignAssets::transfer(
				signed(),
				foreign_id,
				recipient.clone().into(),
				100
			));
			assert_eq!(
				erc20_transfer_log_emitters(),
				vec![erc20_precompile_address(foreign_index, 0x0220); 2]
			);
		});
}

mod validator_collators {
	use super::*;
	use asset_hub_polkadot_runtime::{
		staking::{
			self, AnnounceValidatorSet, EnsureReportingSiblings, ValidatorSetToSystemChains,
		},
		AllPalletsWithoutSystem, ParachainInfo, StakingPointsConverter, ValidatorCollatorChains,
		ValidatorCollators,
	};
	use codec::DecodeLimit;
	use cumulus_primitives_core::{XcmpMessageFormat, XcmpMessageSource};
	use frame_support::traits::{EnsureOrigin, Get, Hooks};
	use pallet_staking_async::OnEraStart;
	use polkadot_runtime_constants::system_parachain::{BULLETIN_ID, PEOPLE_ID};
	use sp_staking::ServiceKind;
	use xcm::{VersionedXcm, MAX_XCM_DECODE_DEPTH};

	fn events() -> Vec<RuntimeEvent> {
		frame_system::Pallet::<Runtime>::events().into_iter().map(|r| r.event).collect()
	}

	/// Era 1 is active with `who` exposed, so credited points reach `who`.
	fn activate_era_one_with(who: &AccountId) {
		pallet_staking_async::ActiveEra::<Runtime>::put(pallet_staking_async::ActiveEraInfo {
			index: 1,
			start: Some(0),
		});
		pallet_staking_async::session_rotation::Eras::<Runtime>::upsert_exposure(
			1,
			who,
			pallet_staking_async::Exposure { total: 1, own: 1, others: vec![] },
		);
	}

	/// Governance turns on validator collation, through the parameter's admin origin.
	fn enable_validator_collation() {
		use asset_hub_polkadot_runtime::dynamic_params::validator_collation;
		assert_ok!(asset_hub_polkadot_runtime::Parameters::set_parameter(
			RuntimeOrigin::root(),
			asset_hub_polkadot_runtime::RuntimeParameters::ValidatorCollation(
				validator_collation::Parameters::Enabled(validator_collation::Enabled, Some(true)),
			),
		));
	}

	/// Era 0 is active and era 1 is planned with `validators` elected. The relay chain then
	/// reports the session that activates era 1, so staking starts it and calls its era-start
	/// hook.
	fn start_era_one_through_staking(validators: Vec<AccountId>) {
		pallet_staking_async::ActiveEra::<Runtime>::put(pallet_staking_async::ActiveEraInfo {
			index: 0,
			start: Some(0),
		});
		pallet_staking_async::CurrentEra::<Runtime>::put(1);
		pallet_staking_async::ForceEra::<Runtime>::put(pallet_staking_async::Forcing::ForceNone);
		pallet_staking_async::NextEraValidators::<Runtime>::put((
			1,
			frame_support::BoundedVec::truncate_from(validators),
		));
		frame_system::Pallet::<Runtime>::reset_events();
		assert_ok!(asset_hub_polkadot_runtime::StakingRcClient::relay_session_report(
			RuntimeOrigin::root(),
			pallet_staking_async_rc_client::SessionReport {
				end_index: 0,
				validator_points: vec![],
				activation_timestamp: Some((1_000, 1)),
				leftover: false,
			},
		));
		assert_eq!(pallet_staking_async::ActiveEra::<Runtime>::get().map(|a| a.index), Some(1));
	}

	fn announcements() -> Vec<RuntimeEvent> {
		events()
			.into_iter()
			.filter(|event| matches!(event, RuntimeEvent::ValidatorSetAnnouncer(_)))
			.collect()
	}

	fn collation_points(era: u32) -> Vec<(AccountId, u32)> {
		pallet_staking_async::ErasRewardPointsByService::<Runtime>::get(era, ServiceKind::Collation)
			.individual
			.into_iter()
			.collect()
	}

	#[test]
	fn validators_collate_only_after_governance_raises_the_cap() {
		system_parachains_test_utils::validator_collators::validators_collate_only_after_governance_raises_the_cap::<
			Runtime,
			AllPalletsWithoutSystem,
		>(
			|owner| {
				let generated = SessionKeys::generate(&owner.encode(), None);
				(generated.keys, generated.proof.encode())
			},
			|era, validators| {
				enable_validator_collation();
				AnnounceValidatorSet::on_era_start(era, &validators)
			},
		);
	}

	#[test]
	fn no_genesis_preset_gives_points_per_service() {
		use asset_hub_polkadot_runtime::{genesis_config_presets, RuntimeGenesisConfig};
		// GIVEN the default genesis and every genesis preset of the runtime
		// WHEN the chain starts from one of them
		// THEN no source earns points until governance sets them
		// The presets read storage, so they run in externalities.
		sp_io::TestExternalities::new_empty().execute_with(|| {
			assert!(RuntimeGenesisConfig::default()
				.staking_points_converter
				.points_per_service
				.is_empty());
			for id in genesis_config_presets::preset_names() {
				let preset: serde_json::Value = serde_json::from_slice(
					&genesis_config_presets::get_preset(&id).expect("listed presets exist"),
				)
				.expect("presets are JSON");
				assert!(
					preset.get("stakingPointsConverter").is_none(),
					"preset {id:?} configures the points converter"
				);
			}
		});
		// AND the collating chains are pinned. Add a chain here once it starts collating. Asset
		// Hub credits its own blocks locally, so it is never listed.
		assert_eq!(ValidatorCollatorChains::get(), vec![PEOPLE_ID.into(), BULLETIN_ID.into()]);
	}

	/// Opens a channel to every chain in `ValidatorCollatorChains`.
	fn open_channels_to_collating_chains() {
		let mut destinations = ValidatorCollatorChains::get();
		// Parachain system binary searches the channels by para id.
		destinations.sort();
		ParachainSystem::open_outbound_hrmp_channel_for_benchmarks_or_tests(destinations[0]);
		cumulus_pallet_parachain_system::RelevantMessagingState::<Runtime>::mutate(|state| {
			let state = state.as_mut().expect("a channel was just opened");
			let channel = state.egress_channels[0].1.clone();
			state.egress_channels =
				destinations.into_iter().map(|id| (id, channel.clone())).collect();
		});
	}

	#[test]
	fn era_start_sends_the_set_to_each_collating_chain() {
		let alice = AccountId::from(ALICE);
		let bob = AccountId::from(BOB);
		ExtBuilder::<Runtime>::default()
			.with_safe_xcm_version(xcm::latest::VERSION)
			.build()
			.execute_with(|| {
				// GIVEN open channels to the collating chains
				open_channels_to_collating_chains();

				// WHEN governance has turned validator collation on and era 1 starts with Alice
				// and Bob
				enable_validator_collation();
				AnnounceValidatorSet::on_era_start(1, &[alice.clone(), bob.clone()]);

				// THEN the set is stored here and sent to each collating chain
				let events = events();
				assert!(events.contains(&RuntimeEvent::ValidatorCollators(
					pallet_validator_collators::Event::ValidatorSetReceived { era: 1, count: 2 }
				)));
				let mut destinations = ValidatorCollatorChains::get();
				for destination in &destinations {
					assert!(
						events.contains(&RuntimeEvent::ValidatorSetAnnouncer(
							pallet_validator_set_announcer::Event::AnnouncementSent {
								destination: *destination,
								era: 1,
							},
						)),
						"no announcement to {destination:?}",
					);
				}

				// AND each chain gets an unpaid `Transact` of `set_validators` with Asset Hub's
				// location as an XCM origin
				let expected_call =
					ValidatorSetToSystemChains::encoded_call(1, &[alice.clone(), bob.clone()]);
				let mut messages = XcmpQueue::take_outbound_messages(usize::MAX, &[]);
				messages.sort_by_key(|(destination, _)| *destination);
				destinations.sort();
				assert_eq!(
					messages.iter().map(|(destination, _)| *destination).collect::<Vec<_>>(),
					destinations
				);
				for (_, page) in messages {
					let mut data = &page[..];
					assert_eq!(
						XcmpMessageFormat::decode(&mut data),
						Ok(XcmpMessageFormat::ConcatenatedVersionedXcm)
					);
					let xcm: Xcm<()> = VersionedXcm::<()>::decode_with_depth_limit(
						MAX_XCM_DECODE_DEPTH,
						&mut data,
					)
					.unwrap()
					.try_into()
					.unwrap();
					assert!(matches!(
						xcm.0.as_slice(),
						[
							UnpaidExecution { weight_limit: Unlimited, check_origin: None },
							Transact { origin_kind: OriginKind::Xcm, call, .. },
							SetTopic(_),
						] if call.encoded() == expected_call.as_slice()
					));
				}
			});
	}

	#[test]
	fn staking_era_start_stores_and_sends_nothing_while_collation_is_off() {
		let alice = AccountId::from(ALICE);
		let bob = AccountId::from(BOB);
		ExtBuilder::<Runtime>::default()
			.with_safe_xcm_version(xcm::latest::VERSION)
			.build()
			.execute_with(|| {
				// GIVEN validator collation is off, as by default, and channels to the collating
				// chains are open
				open_channels_to_collating_chains();

				// WHEN staking starts era 1 with Alice and Bob
				start_era_one_through_staking(vec![alice, bob]);

				// THEN no set is stored, so no session rotation is forced, and nothing is sent
				assert!(!pallet_validator_collators::ExternalValidatorSet::<Runtime>::exists());
				assert_eq!(
					pallet_validator_collators::PendingRotation::<Runtime>::get(),
					pallet_validator_collators::RotationState::Idle
				);
				assert_eq!(announcements(), vec![]);
				assert_eq!(XcmpQueue::take_outbound_messages(usize::MAX, &[]), vec![]);
			});
	}

	#[test]
	fn staking_era_start_announces_the_elected_set_once_collation_is_on() {
		let alice = AccountId::from(ALICE);
		let bob = AccountId::from(BOB);
		ExtBuilder::<Runtime>::default().build().execute_with(|| {
			// GIVEN governance set the cap first, which forces no rotation while no set is stored
			assert_ok!(ValidatorCollators::set_max_collators(
				RuntimeOrigin::root(),
				pallet_validator_collators::CollatorDraw::UpTo(2),
			));
			assert_eq!(
				pallet_validator_collators::PendingRotation::<Runtime>::get(),
				pallet_validator_collators::RotationState::Idle
			);
			// AND then turned validator collation on
			enable_validator_collation();

			// WHEN staking starts era 1 with Alice and Bob
			start_era_one_through_staking(vec![alice, bob]);

			// THEN the hook stores the set here and announces it to each collating chain
			assert_eq!(
				pallet_validator_collators::ExternalValidatorSet::<Runtime>::get()
					.map(|set| (set.era, set.validators.len())),
				Some((1, 2))
			);
			assert_eq!(
				pallet_validator_collators::PendingRotation::<Runtime>::get(),
				pallet_validator_collators::RotationState::AwaitingQueue
			);
			let announcements = announcements();
			for id in ValidatorCollatorChains::get() {
				assert!(
					announcements.iter().any(|event| matches!(
						event,
						RuntimeEvent::ValidatorSetAnnouncer(
							pallet_validator_set_announcer::Event::AnnouncementSent { destination, era: 1 } |
							pallet_validator_set_announcer::Event::AnnouncementFailed { destination, era: 1 }
						) if *destination == id
					)),
					"no announcement to {id:?}",
				);
			}
		});
	}

	#[test]
	fn era_start_weight_follows_the_switch() {
		use asset_hub_polkadot_runtime::ValidatorSetAnnouncer;
		ExtBuilder::<Runtime>::default().build().execute_with(|| {
			let read = <Runtime as frame_system::Config>::DbWeight::get().reads(1);
			let hook_weight = |n| <AnnounceValidatorSet as OnEraStart<AccountId>>::weight(n);
			for n in [0, 320, 1000] {
				// GIVEN validator collation is off
				// WHEN staking weighs the era-start hook
				// THEN it charges only the read of the switch, as the hook does nothing else
				assert_eq!(hook_weight(n), read);
			}

			// GIVEN governance turned validator collation on
			enable_validator_collation();
			for n in [0, 320, 1000] {
				// THEN it charges the read of the switch and announcing `n` validators, so the
				// session report that starts an era is weighed for the announcement
				assert_eq!(
					hook_weight(n),
					read.saturating_add(ValidatorSetAnnouncer::announce_weight(n))
				);
			}
		});
	}

	#[test]
	fn only_the_collator_selection_origin_switches_validator_collation() {
		use asset_hub_polkadot_runtime::{
			dynamic_params::validator_collation::Enabled, governance::pallet_custom_origins,
			DynamicParameterOrigin, RuntimeParametersKey,
		};
		use frame_support::traits::EnsureOriginWithArg;
		let key = RuntimeParametersKey::ValidatorCollation(Enabled.into());
		// GIVEN the switch's key
		// WHEN root or the StakingAdmin sets it THEN it is accepted
		for origin in [RuntimeOrigin::root(), pallet_custom_origins::Origin::StakingAdmin.into()] {
			assert!(DynamicParameterOrigin::try_origin(origin, &key).is_ok());
		}
		// WHEN another origin sets it THEN it is rejected
		for origin in [
			RuntimeOrigin::signed(AccountId::from(ALICE)),
			pallet_custom_origins::Origin::WhitelistedCaller.into(),
			pallet_custom_origins::Origin::GeneralAdmin.into(),
		] {
			assert!(DynamicParameterOrigin::try_origin(origin, &key).is_err());
		}
	}

	#[test]
	fn only_root_and_the_staking_admin_set_the_cap_and_the_points() {
		use asset_hub_polkadot_runtime::governance::pallet_custom_origins;
		use frame_support::assert_noop;
		use pallet_staking_async_points_converter::PointsPerService;
		use pallet_validator_collators::{CollatorDraw, MaxCollators};
		use sp_runtime::DispatchError::BadOrigin;
		// The referenda that enable validator collation run on the StakingAdmin track.
		ExtBuilder::<Runtime>::default().build().execute_with(|| {
			let own = ParachainInfo::parachain_id();
			// GIVEN the default cap and no points for this chain's blocks
			assert_eq!(MaxCollators::<Runtime>::get(), CollatorDraw::None);
			assert_eq!(PointsPerService::<Runtime>::get(own, ServiceKind::Collation), 0);

			// WHEN a signed account, GeneralAdmin or WhitelistedCaller sets them
			// THEN both calls are rejected
			for origin in [
				RuntimeOrigin::signed(AccountId::from(ALICE)),
				pallet_custom_origins::Origin::GeneralAdmin.into(),
				pallet_custom_origins::Origin::WhitelistedCaller.into(),
			] {
				assert_noop!(
					ValidatorCollators::set_max_collators(origin.clone(), CollatorDraw::UpTo(1)),
					BadOrigin
				);
				assert_noop!(
					StakingPointsConverter::set_points_per_service(
						origin,
						own,
						ServiceKind::Collation,
						1
					),
					BadOrigin
				);
			}

			// WHEN Root or the StakingAdmin sets them THEN both calls take effect
			for (value, origin) in [
				(1, RuntimeOrigin::root()),
				(2, pallet_custom_origins::Origin::StakingAdmin.into()),
			] {
				assert_ok!(ValidatorCollators::set_max_collators(
					origin.clone(),
					CollatorDraw::UpTo(value)
				));
				assert_ok!(StakingPointsConverter::set_points_per_service(
					origin,
					own,
					ServiceKind::Collation,
					value
				));
				assert_eq!(MaxCollators::<Runtime>::get(), CollatorDraw::UpTo(value));
				assert_eq!(PointsPerService::<Runtime>::get(own, ServiceKind::Collation), value);
			}
		});
	}

	#[test]
	fn no_origin_sets_the_validator_set_of_asset_hub() {
		// Asset Hub stores the set staking elects, from the era start hook. A call that sets it
		// would let one origin choose who collates on every chain that draws from the set.
		ExtBuilder::<Runtime>::default().build().execute_with(|| {
			// GIVEN a set of one validator
			let validators = [AccountId::from(ALICE)]
				.into_iter()
				.collect::<std::collections::BTreeSet<_>>()
				.try_into()
				.unwrap();
			// WHEN Root calls `set_validators` with it
			// THEN it is rejected and no set is stored
			frame_support::assert_noop!(
				ValidatorCollators::set_validators(RuntimeOrigin::root(), 1, validators),
				sp_runtime::DispatchError::BadOrigin
			);
		});
	}

	#[test]
	fn own_blocks_earn_points_only_after_governance_sets_them() {
		ExtBuilder::<Runtime>::default().build().execute_with(|| {
			let alice = AccountId::from(ALICE);
			let own = ParachainInfo::parachain_id();
			let report = || {
				// Alice authored 3 blocks here and the report block comes.
				for _ in 0..3 {
					<ValidatorCollators as pallet_authorship::EventHandler<_, _>>::note_author(
						alice.clone(),
					);
				}
				let period = pallet_validator_collators::ReportPeriod::<Runtime>::get();
				frame_system::Pallet::<Runtime>::reset_events();
				ValidatorCollators::on_poll(
					period,
					&mut frame_support::weights::WeightMeter::new(),
				);
			};
			// GIVEN era 1 is active with Alice exposed and the stored set of Alice
			activate_era_one_with(&alice);
			assert_ok!(ValidatorCollators::receive_validator_set(1, [alice.clone()]));

			// WHEN this chain reports Alice's blocks with the default points per service
			report();
			// THEN they earn no points
			assert!(events().contains(&RuntimeEvent::StakingPointsConverter(
				pallet_staking_async_points_converter::Event::BlocksIgnored {
					source: own,
					blocks: 3
				}
			)));
			assert_eq!(collation_points(1), vec![]);

			// WHEN governance sets 5 points per block of this chain and Alice's next blocks are
			// reported
			assert_ok!(StakingPointsConverter::set_points_per_service(
				RuntimeOrigin::root(),
				own,
				ServiceKind::Collation,
				5
			));
			report();
			// THEN they reach staking as collation points of this chain, with no message sent
			assert!(events().contains(&RuntimeEvent::StakingPointsConverter(
				pallet_staking_async_points_converter::Event::BlocksCredited {
					source: own,
					blocks: 3
				}
			)));
			assert_eq!(collation_points(1), vec![(alice, 15)]);
		});
	}

	#[test]
	fn only_listed_chains_report_blocks_and_only_as_themselves() {
		type EnsureReportingSibling = EnsureReportingSiblings<ValidatorCollatorChains>;
		let sibling =
			|id: u32| RuntimeOrigin::from(cumulus_pallet_xcm::Origin::SiblingParachain(id.into()));
		let chains = ValidatorCollatorChains::get();
		// GIVEN a collating chain as a sibling parachain origin
		// WHEN it reports THEN it is accepted under its own id
		for id in &chains {
			assert_eq!(EnsureReportingSibling::try_origin(sibling((*id).into())).ok(), Some(*id));
		}
		// GIVEN another system or non-system sibling, the relay chain, a collating chain's
		// location as an XCM origin, its sovereign account, or root
		let account_of = |location| LocationToAccountId::convert_location(&location).unwrap();
		let others = [1000, 1001, 1005, 2000]
			.into_iter()
			.filter(|id| !chains.contains(&(*id).into()))
			.map(sibling);
		let as_collating_chains = chains.iter().flat_map(|id| {
			let location = Location::new(1, Parachain((*id).into()));
			[
				RuntimeOrigin::from(pallet_xcm::Origin::Xcm(location.clone())),
				RuntimeOrigin::signed(account_of(location)),
			]
		});
		for origin in others
			.chain(as_collating_chains)
			.chain([RuntimeOrigin::from(cumulus_pallet_xcm::Origin::Relay), RuntimeOrigin::root()])
		{
			// WHEN it reports THEN it is rejected
			assert!(EnsureReportingSibling::try_origin(origin).is_err());
		}
	}

	#[test]
	fn report_messages_execute_unpaid_and_earn_points_only_once_set() {
		ExtBuilder::<Runtime>::default().build().execute_with(|| {
			let alice = AccountId::from(ALICE);
			let report = || {
				RuntimeCall::StakingPointsConverter(
					pallet_staking_async_points_converter::Call::report_authored_blocks {
						blocks: vec![(alice.clone(), 3)],
					},
				)
			};
			// GIVEN era 1 is active with Alice exposed
			activate_era_one_with(&alice);

			let chains: Vec<u32> =
				ValidatorCollatorChains::get().into_iter().map(Into::into).collect();
			for &id in &chains {
				// WHEN a non-system sibling, or an account on the chain, sends the message the
				// chain sends to report its blocks
				// THEN the barrier rejects it, since only system chains execute without paying
				for origin in [
					Location::new(1, Parachain(2000)),
					Location::new(1, [Parachain(id), AccountId32 { network: None, id: ALICE }]),
				] {
					assert_eq!(
						RuntimeHelper::execute_as_origin(
							(origin, OriginKind::Native),
							report(),
							None
						)
						.ensure_complete()
						.map_err(|error| error.error),
						Err(XcmError::Barrier)
					);
				}

				// WHEN the chain sends it, an unpaid `Transact` with its native origin
				// THEN it executes, and with the default points per service earns nothing
				frame_system::Pallet::<Runtime>::reset_events();
				assert_ok!(RuntimeHelper::execute_as_origin(
					(Location::new(1, Parachain(id)), OriginKind::Native),
					report(),
					None,
				)
				.ensure_complete());
				assert!(events().contains(&RuntimeEvent::StakingPointsConverter(
					pallet_staking_async_points_converter::Event::BlocksIgnored {
						source: id.into(),
						blocks: 3
					}
				)));
			}
			assert_eq!(collation_points(1), vec![]);

			for (reported, &id) in (1..).zip(&chains) {
				// WHEN governance sets 5 points per block of the chain and the chain reports again
				assert_ok!(StakingPointsConverter::set_points_per_service(
					RuntimeOrigin::root(),
					id.into(),
					ServiceKind::Collation,
					5
				));
				assert_ok!(RuntimeHelper::execute_as_origin(
					(Location::new(1, Parachain(id)), OriginKind::Native),
					report(),
					None,
				)
				.ensure_complete());
				// THEN Alice's blocks are credited as the chain's collation points
				assert_eq!(collation_points(1), vec![(alice.clone(), 15 * reported)]);
			}
		});
	}

	#[test]
	fn full_report_fits_the_message_queue() {
		ExtBuilder::<Runtime>::default().build().execute_with(|| {
			// GIVEN the largest report a chain sends, all credited
			let entries = <staking::MaxValidatorSet as Get<u32>>::get();
			let message = StakingPointsConverter::report_authored_blocks_weight(entries);
			// WHEN Asset Hub processes it from its message queue
			// THEN it fits the service weight, so it is not set aside as overweight
			let service_weight =
				asset_hub_polkadot_runtime::dynamic_params::message_queue::MaxOnInitWeight::get()
					.expect("the message queue has a service weight");
			assert!(
				service_weight.all_gte(message),
				"a full report of {entries} entries weighs {message:?}",
			);
		});
	}

	#[test]
	fn kick_threshold_follows_the_cap() {
		use pallet_validator_collators::CollatorDraw::{All, UpTo};
		system_parachains_test_utils::validator_collators::kick_threshold_follows_the_cap::<Runtime>(
			asset_hub_polkadot_runtime::Period::get(),
			&[(UpTo(1000), 28_800), (UpTo(320), 10_800), (UpTo(500), 18_000), (All, 28_800)],
		);
	}

	#[test]
	fn regular_rotation_keeps_its_period() {
		system_parachains_test_utils::validator_collators::regular_rotation_keeps_its_period::<
			Runtime,
		>(asset_hub_polkadot_runtime::Period::get(), asset_hub_polkadot_runtime::Offset::get());
	}

	#[test]
	fn aura_authorities_fit_the_relay_head_data() {
		// The relay chain's `max_head_data_size`.
		const MAX_HEAD_DATA_SIZE: u32 = 20_480;
		// The seal, the collator's pre-runtime digests, the relay-parent storage root digest and a
		// four byte block number, about 200 bytes, which the test block lacks.
		const ALLOWANCE: u32 = 256;
		system_parachains_test_utils::validator_collators::aura_authorities_fit_the_relay_head_data::<
			Runtime,
		>(MAX_HEAD_DATA_SIZE, ALLOWANCE);
	}

	#[test]
	fn relay_proof_request_includes_the_validator_collators_randomness_key() {
		use cumulus_primitives_core::{
			relay_chain::well_known_keys::ONE_EPOCH_AGO_RANDOMNESS,
			runtime_decl_for_key_to_include_in_relay_proof::KeyToIncludeInRelayProofV1,
			RelayStorageKey,
		};
		// GIVEN the runtime wires pallet-validator-collators, which reads the relay chain epoch
		// randomness from every relay chain state proof
		// WHEN a collator asks the runtime which relay chain keys to prove
		let keys = Runtime::keys_to_prove().keys;
		// THEN the randomness key is requested
		assert!(keys.contains(&RelayStorageKey::Top(ONE_EPOCH_AGO_RANDOMNESS.to_vec())));
	}

	// `fast-runtime` shortens the era, not the expected era duration.
	#[cfg(not(feature = "fast-runtime"))]
	#[test]
	fn expected_era_duration_matches_the_staking_era() {
		use system_parachains_constants::polkadot::staking::EXPECTED_ERA_DURATION_MILLIS;
		// GIVEN the era length of Asset Hub's staking
		let era = staking::SessionsPerEra::get() as u64 *
			staking::RelaySessionDuration::get() as u64 *
			RELAY_CHAIN_SLOT_DURATION_MILLIS as u64;
		// WHEN system chains expect the end of an era after the arrival of its set
		// THEN they expect it after one era, so they report shortly before the rotation
		assert_eq!(
			EXPECTED_ERA_DURATION_MILLIS, era,
			"update `system_parachains_constants::polkadot::staking::EXPECTED_ERA_DURATION_MILLIS` \
			to the era length of {era} ms",
		);
	}
}
