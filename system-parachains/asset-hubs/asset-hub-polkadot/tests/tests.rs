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
		CheckingAccount, DotLocation, GovernanceLocation, LocationToAccountId,
		RelayTreasuryLocation, RelayTreasuryPalletAccount, StakingPot,
		TrustBackedAssetsPalletLocation, XcmConfig,
	},
	AllPalletsWithoutSystem, AssetDeposit, Assets, Balances, Block, ExistentialDeposit,
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
use codec::{Decode, Encode};
use frame_support::{
	assert_err, assert_ok,
	traits::{fungibles::InspectEnumerable, ContainsPair},
};
use parachains_common::{
	AccountId, AssetHubPolkadotAuraId as AuraId, AssetIdForTrustBackedAssets, Balance,
};
use sp_consensus_aura::SlotDuration;
use sp_core::crypto::Ss58Codec;
use sp_runtime::{traits::MaybeEquivalence, Either, TryRuntimeError};
use system_parachains_constants::polkadot::{
	consensus::RELAY_CHAIN_SLOT_DURATION_MILLIS, currency::UNITS, fee::WeightToFee,
};
use xcm::latest::{
	prelude::{Assets as XcmAssets, *},
	WESTEND_GENESIS_HASH,
};
use xcm_builder::WithLatestLocationConverter;
use xcm_executor::traits::ConvertLocation;
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
	// TODO: after AHM change this from `()` to `CheckingAccount`
	(),
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
		Some(XcmBridgeHubRouterFeeAssetId::get()),
		Some(RelayTreasuryPalletAccount::get()),
	)
}

#[test]
fn receive_reserve_asset_deposited_ksm_from_asset_hub_kusama_fees_paid_by_pool_swap_works() {
	const BLOCK_AUTHOR_ACCOUNT: [u8; 32] = [13; 32];
	let block_author_account = AccountId::from(BLOCK_AUTHOR_ACCOUNT);
	let staking_pot = StakingPot::get();

	let foreign_asset_id_location_v5 = Location::new(2, [GlobalConsensus(NetworkId::Kusama)]);
	let foreign_asset_id_minimum_balance = 1_000_000_000;
	// sovereign account as foreign asset owner (can be whoever for this scenario)
	let foreign_asset_owner = LocationToAccountId::convert_location(&Location::parent()).unwrap();
	let foreign_asset_create_params = (
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
		foreign_asset_create_params.clone(),
		1000000000000,
		|| {
			// setup pool for paying fees to touch `SwapFirstAssetTrader`
			asset_test_utils::test_cases::setup_pool_for_paying_fees_with_foreign_assets::<
				Runtime,
				RuntimeOrigin,
			>(ExistentialDeposit::get(), foreign_asset_create_params);
			// staking pot account for collecting local native fees from `BuyExecution`
			let _ = Balances::force_set_balance(
				RuntimeOrigin::root(),
				StakingPot::get().into(),
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
			// check staking pot for ED
			assert_eq!(Balances::free_balance(&staking_pot), ExistentialDeposit::get());
			// check now foreign asset for staking pot
			assert_eq!(
				ForeignAssets::balance(foreign_asset_id_location_v5.clone(), &staking_pot),
				0
			);
		},
		|| {
			// `SwapFirstAssetTrader` - staking pot receives xcm fees in KSMs
			assert!(Balances::free_balance(&staking_pot) > ExistentialDeposit::get());
			// staking pot receives no foreign assets
			assert_eq!(
				ForeignAssets::balance(foreign_asset_id_location_v5.clone(), &staking_pot),
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
		GovernanceOrigin::Location(GovernanceLocation::get()),
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
		GovernanceOrigin::Location(GovernanceLocation::get()),
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
		GovernanceOrigin::Location(GovernanceLocation::get()),
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

	// no - random para
	assert_err!(
		parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::Location(Location::new(1, Parachain(12334)))),
		Either::Right(InstructionError { index: 0, error: XcmError::Barrier })
	);
	// no - AssetHub
	assert_err!(
		parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::Location(Location::new(1, Parachain(ASSET_HUB_ID)))),
		Either::Right(InstructionError { index: 0, error: XcmError::Barrier })
	);
	// no - Collectives
	assert_err!(
		parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::Location(Location::new(1, Parachain(COLLECTIVES_ID)))),
		Either::Right(InstructionError { index: 0, error: XcmError::Barrier })
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
	>(GovernanceOrigin::Location(Location::parent())));
	assert_ok!(parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
		Runtime,
		RuntimeOrigin,
	>(GovernanceOrigin::Location(GovernanceLocation::get())));
}
