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
		StakingPot, TrustBackedAssetsPalletLocation, XcmConfig,
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

frame_support::parameter_types! {
	// Local OpenGov
	pub Governance: GovernanceOrigin<RuntimeOrigin> = GovernanceOrigin::Origin(RuntimeOrigin::root());
}

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

/* // FIXME @karol FAIL-CI
#[test]
fn limited_reserve_transfer_assets_for_native_asset_to_asset_hub_kusama_works() {
	use sp_runtime::traits::Get;

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
		Some(TreasuryAccount::get()),
	)
} */

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

#[cfg(test)]
mod inflation_tests {
	use crate::{Balance, Runtime, UNITS};
	use approx::assert_relative_eq;
	use asset_hub_polkadot_runtime::staking;
	use asset_test_utils::ExtBuilder;
	use cumulus_pallet_parachain_system::pallet::ValidationData;
	use cumulus_primitives_core::{
		relay_chain::BlockNumber as RC_BlockNumber, PersistedValidationData,
	};
	use pallet_staking_async::EraPayout;
	use polkadot_runtime_constants::time::YEARS as RC_YEARS;
	use sp_runtime::Perbill;
	use staking::March2026TI;

	const MILLISECONDS_PER_DAY: u64 = 24 * 60 * 60 * 1000;

	#[test]
	fn pre_march_2026_formula_staking_inflation_correct_single_era() {
		ExtBuilder::<Runtime>::default().build().execute_with(|| {
			let (to_stakers, to_treasury) = <staking::EraPayout as EraPayout<Balance>>::era_payout(
				123, // ignored
				456, // ignored
				MILLISECONDS_PER_DAY,
			);

			// Values are within 0.1%
			assert_relative_eq!(to_stakers as f64, (279_477 * UNITS) as f64, max_relative = 0.001);
			assert_relative_eq!(to_treasury as f64, (49_320 * UNITS) as f64, max_relative = 0.001);
			// Total per day is ~328,797 DOT
			assert_relative_eq!(
				(to_stakers as f64 + to_treasury as f64),
				(328_797 * UNITS) as f64,
				max_relative = 0.001
			);
		});
	}

	#[test]
	fn pre_march_2026_formula_staking_inflation_correct_longer_era() {
		ExtBuilder::<Runtime>::default().build().execute_with(|| {
			// Twice the era duration means twice the emission:
			let (to_stakers, to_treasury) = <staking::EraPayout as EraPayout<Balance>>::era_payout(
				123, // ignored
				456, // ignored
				2 * MILLISECONDS_PER_DAY,
			);

			assert_relative_eq!(
				to_stakers as f64,
				(279_477 * UNITS) as f64 * 2.0,
				max_relative = 0.001
			);
			assert_relative_eq!(
				to_treasury as f64,
				(49_320 * UNITS) as f64 * 2.0,
				max_relative = 0.001
			);
		});
	}

	#[test]
	fn pre_march_2026_formula_staking_inflation_correct_whole_year() {
		ExtBuilder::<Runtime>::default().build().execute_with(|| {
			let (to_stakers, to_treasury) = <staking::EraPayout as EraPayout<Balance>>::era_payout(
				123,                                  // ignored
				456,                                  // ignored
				(36525 * MILLISECONDS_PER_DAY) / 100, // 1 year
			);

			// Our yearly emissions is about 120M DOT:
			let yearly_emission = 120_093_259 * UNITS;
			assert_relative_eq!(
				to_stakers as f64 + to_treasury as f64,
				yearly_emission as f64,
				max_relative = 0.001
			);

			assert_relative_eq!(
				to_stakers as f64,
				yearly_emission as f64 * 0.85,
				max_relative = 0.001
			);
			assert_relative_eq!(
				to_treasury as f64,
				yearly_emission as f64 * 0.15,
				max_relative = 0.001
			);
		});
	}

	// 10 years into the future, our values do not overflow.
	#[test]
	fn pre_march_2026_formula_staking_inflation_correct_not_overflow() {
		ExtBuilder::<Runtime>::default().build().execute_with(|| {
			let (to_stakers, to_treasury) = <staking::EraPayout as EraPayout<Balance>>::era_payout(
				123,                                 // ignored
				456,                                 // ignored
				(36525 * MILLISECONDS_PER_DAY) / 10, // 10 years
			);
			let initial_ti: i128 = 15_011_657_390_566_252_333;
			let projected_total_issuance = (to_stakers as i128 + to_treasury as i128) + initial_ti;

			// In 2034, there will be about 2.7 billion DOT in existence.
			assert_relative_eq!(
				projected_total_issuance as f64,
				(2_700_000_000 * UNITS) as f64,
				max_relative = 0.001
			);
		});
	}

	// Sets the view of the relay chain block number.
	fn set_relay_number(n: RC_BlockNumber) {
		ValidationData::<Runtime>::set(Some(PersistedValidationData {
			parent_head: vec![].into(),
			relay_parent_number: n,
			max_pov_size: Default::default(),
			relay_parent_storage_root: Default::default(),
		}));
	}

	const MARCH_14_2026: RC_BlockNumber = 30_367_108;
	// The March 14, 2026 TI used for calculations in [Ref 1710](https://polkadot.subsquare.io/referenda/1710).
	const MARCH_TI: u128 = 1_676_733_867 * UNITS;
	const TARGET_TI: u128 = 2_100_000_000 * UNITS;

	#[test]
	fn storing_ti_works() {
		ExtBuilder::<Runtime>::default().build().execute_with(|| {
			// Pre-march.
			pallet_balances::pallet::TotalIssuance::<Runtime, ()>::set(MARCH_TI);
			<staking::EraPayout as EraPayout<Balance>>::era_payout(0, 0, MILLISECONDS_PER_DAY);
			assert!(March2026TI::get() == None);

			// Post-march.
			set_relay_number(MARCH_14_2026);
			<staking::EraPayout as EraPayout<Balance>>::era_payout(0, 0, MILLISECONDS_PER_DAY);
			assert!(March2026TI::get() == Some(MARCH_TI));

			// No change on subsequent call.
			set_relay_number(MARCH_14_2026 + 2 * RC_YEARS);
			pallet_balances::pallet::TotalIssuance::<Runtime, ()>::set(MARCH_TI + 1);
			<staking::EraPayout as EraPayout<Balance>>::era_payout(0, 0, MILLISECONDS_PER_DAY);
			assert!(March2026TI::get() == Some(MARCH_TI));
		});
	}

	// The transition from set emission to stepped emission works.
	#[test]
	fn set_to_stepped_inflation_transition_works() {
		ExtBuilder::<Runtime>::default().build().execute_with(|| {
			// Check before transition date.
			let (to_stakers, to_treasury) = <staking::EraPayout as EraPayout<Balance>>::era_payout(
				123, // ignored
				456, // ignored
				MILLISECONDS_PER_DAY,
			);
			assert_relative_eq!(to_stakers as f64, (279_477 * UNITS) as f64, max_relative = 0.001);
			assert_relative_eq!(to_treasury as f64, (49_320 * UNITS) as f64, max_relative = 0.001);
			assert_relative_eq!(
				(to_stakers as f64 + to_treasury as f64),
				(328_797 * UNITS) as f64,
				max_relative = 0.001
			);

			pallet_balances::pallet::TotalIssuance::<Runtime, ()>::set(MARCH_TI);

			// Check after transition date.
			set_relay_number(MARCH_14_2026);
			let (to_stakers, to_treasury) = <staking::EraPayout as EraPayout<Balance>>::era_payout(
				123, // ignored
				456, // ignored
				MILLISECONDS_PER_DAY,
			);
			let two_year_rate = Perbill::from_rational(2_628u32, 10_000u32);
			let era_rate = two_year_rate *
				Perbill::from_rational(1u32, 2u32) *
				Perbill::from_rational(100u32, 36525u32);
			let assumed_payout = era_rate * (TARGET_TI - MARCH_TI);
			assert_relative_eq!(
				(to_stakers as f64 + to_treasury as f64),
				assumed_payout as f64,
				max_relative = 0.00001
			);
		});
	}

	// The emission values for the two year periods are as expected.
	#[test]
	fn stepped_inflation_two_year_values_correct() {
		ExtBuilder::<Runtime>::default()
		.build()
		.execute_with(|| {
			let two_years: RC_BlockNumber = RC_YEARS * 2;
			pallet_balances::pallet::TotalIssuance::<Runtime, ()>::set(MARCH_TI);

			// First period - March 14, 2026 -> March 14, 2028.
			set_relay_number(MARCH_14_2026);
			let (to_stakers, to_treasury) = <staking::EraPayout as EraPayout<Balance>>::era_payout(
				123, // ignored
				456, // ignored
				MILLISECONDS_PER_DAY,
			);
			let two_year_rate = Perbill::from_rational(2_628u32, 10_000u32);
			let first_period_emission = two_year_rate * (TARGET_TI - MARCH_TI);
			assert_relative_eq!(
				(to_stakers as f64 + to_treasury as f64) * 365.25 * 2.0,
				first_period_emission as f64,
				max_relative = 0.00001
			);
			// Visual checks.
			assert_relative_eq!(
				to_stakers as f64 + to_treasury as f64,
				(152_271 * UNITS) as f64,
				max_relative = 0.00001,
			);
			assert_relative_eq!(
				(to_stakers as f64 + to_treasury as f64) * 365.25, // full year
				(55_617_170 * UNITS) as f64, // https://docs.google.com/spreadsheets/d/1pW6fVESnkenJkqIzRk2Pv4cp5KNzVYSupUI6EA-jeR8/edit?gid=0#gid=0&range=E2.
				max_relative = 0.00001,
			);

			// Second period - March 14, 2028 -> March 14, 2030.
			let march_14_2028 = MARCH_14_2026 + two_years;
			set_relay_number(march_14_2028);
			let (to_stakers, to_treasury) = <staking::EraPayout as EraPayout<Balance>>::era_payout(
				123, // ignored
				456, // ignored
				MILLISECONDS_PER_DAY,
			);
			let ti_at_2028 = MARCH_TI + first_period_emission;
			let second_period_emission = two_year_rate * (TARGET_TI - ti_at_2028);
			assert_relative_eq!(
				(to_stakers as f64 + to_treasury as f64) * 365.25 * 2.0,
				second_period_emission as f64,
				max_relative = 0.00001
			);
			// Visual checks.
			assert_relative_eq!(
				to_stakers as f64 + to_treasury as f64,
				(112_254 * UNITS) as f64,
				max_relative = 0.00001,
			);
			assert_relative_eq!(
				(to_stakers as f64 + to_treasury as f64) * 365.25, // full year
				(41_000_978 * UNITS) as f64, // https://docs.google.com/spreadsheets/d/1pW6fVESnkenJkqIzRk2Pv4cp5KNzVYSupUI6EA-jeR8/edit?gid=0#gid=0&range=E3.
				max_relative = 0.00001,
			);

			// Third period - March 14, 2030 -> March 14, 2032.
			let march_14_2030 = march_14_2028 + two_years;
			set_relay_number(march_14_2030);
			let (to_stakers, to_treasury) = <staking::EraPayout as EraPayout<Balance>>::era_payout(
				123, // ignored
				456, // ignored
				MILLISECONDS_PER_DAY,
			);
			let ti_at_2030 = ti_at_2028 + second_period_emission;
			let third_period_emission = two_year_rate * (TARGET_TI - ti_at_2030);
			assert_relative_eq!(
				(to_stakers as f64 + to_treasury as f64) * 365.25 * 2.0,
				third_period_emission as f64,
				max_relative = 0.00001
			);
			// Visual checks.
			assert_relative_eq!(
				to_stakers as f64 + to_treasury as f64,
				(82_754 * UNITS) as f64,
				max_relative = 0.00001,
			);
			assert_relative_eq!(
				(to_stakers as f64 + to_treasury as f64) * 365.25, // full year
				(30_225_921 * UNITS) as f64, // https://docs.google.com/spreadsheets/d/1pW6fVESnkenJkqIzRk2Pv4cp5KNzVYSupUI6EA-jeR8/edit?gid=0#gid=0&range=E4.
				max_relative = 0.00001,
			);
		});
	}

	// Emission value does not change mid period.
	#[test]
	fn emission_value_static_throughout_period() {
		ExtBuilder::<Runtime>::default().build().execute_with(|| {
			let two_years: RC_BlockNumber = RC_YEARS * 2;
			pallet_balances::pallet::TotalIssuance::<Runtime, ()>::set(MARCH_TI);

			// Get payout at the beginning of the first stepped period.
			set_relay_number(MARCH_14_2026);
			let (to_stakers_start, to_treasury_start) =
				<staking::EraPayout as EraPayout<Balance>>::era_payout(
					123, // ignored
					456, // ignored
					MILLISECONDS_PER_DAY,
				);

			// Get payout just before the end of the first stepped period.
			let almost_two_years_later: RC_BlockNumber = MARCH_14_2026 + two_years - 1;
			set_relay_number(almost_two_years_later);
			let (to_stakers_end, to_treasury_end) =
				<staking::EraPayout as EraPayout<Balance>>::era_payout(
					123, // ignored
					456, // ignored
					MILLISECONDS_PER_DAY,
				);

			// Payout identical.
			assert_eq!(to_stakers_start + to_treasury_start, to_stakers_end + to_treasury_end);
		});
	}

	// The emission is eventually zero.
	#[test]
	fn emission_eventually_zero() {
		ExtBuilder::<Runtime>::default().build().execute_with(|| {
			pallet_balances::pallet::TotalIssuance::<Runtime, ()>::set(MARCH_TI);

			let forseeable_future: RC_BlockNumber = MARCH_14_2026 + (RC_YEARS * 80);
			set_relay_number(forseeable_future);
			let (to_stakers, to_treasury) = <staking::EraPayout as EraPayout<Balance>>::era_payout(
				123, // ignored
				456, // ignored
				MILLISECONDS_PER_DAY,
			);

			// Payout is less than 1 UNIT after 41 steps.
			assert!(to_stakers + to_treasury < 1 * UNITS);

			let far_future: RC_BlockNumber = MARCH_14_2026 + (RC_YEARS * 500);
			set_relay_number(far_future);
			let (to_stakers, to_treasury) = <staking::EraPayout as EraPayout<Balance>>::era_payout(
				123, // ignored
				456, // ignored
				MILLISECONDS_PER_DAY,
			);

			// TI has converged on asymptote. Payout is zero.
			assert_eq!(to_stakers + to_treasury, 0);
		});
	}

	// TI stays <= 2.1B.
	#[test]
	fn ti_is_asymptotic_to_desired_value() {
		ExtBuilder::<Runtime>::default().build().execute_with(|| {
			pallet_balances::pallet::TotalIssuance::<Runtime, ()>::set(MARCH_TI);

			let mut current_ti = MARCH_TI;
			let mut current_bn = MARCH_14_2026;

			// Run for 250 periods (500 years) and check TI and emissions.
			// We know from `emission_eventually_zero` that at this point era emissions are 0.
			set_relay_number(current_bn);
			for _ in 0..250 {
				let (to_stakers, to_treasury) =
					<staking::EraPayout as EraPayout<Balance>>::era_payout(
						123,                                      // ignored
						456,                                      // ignored
						(MILLISECONDS_PER_DAY * 36525 * 2) / 100, // two year era
					);
				current_ti += to_stakers + to_treasury;
				current_bn += RC_YEARS * 2;
				set_relay_number(current_bn);
			}

			// TI has hit asymptote.
			assert!(current_ti > TARGET_TI - 1 * UNITS);
			assert!(current_ti < TARGET_TI);
		});
	}
}
