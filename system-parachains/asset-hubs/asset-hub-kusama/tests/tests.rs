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

extern crate alloc;

use alloc::boxed::Box;
use asset_hub_kusama_runtime::{
	xcm_config::{
		bridging::{self, XcmBridgeHubRouterFeeAssetId},
		CheckingAccount, KsmLocation, LocationToAccountId, RelayChainLocation,
		RelayTreasuryLocation, RelayTreasuryPalletAccount, StakingPot,
		TrustBackedAssetsPalletLocation, XcmConfig,
	},
	AllPalletsWithoutSystem, AssetDeposit, Assets, Balances, Block, ExistentialDeposit,
	ForeignAssets, ForeignAssetsInstance, MetadataDepositBase, MetadataDepositPerByte,
	ParachainSystem, PolkadotXcm, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin, SessionKeys,
	ToPolkadotXcmRouterInstance, TrustBackedAssetsInstance, XcmpQueue, SLOT_DURATION,
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
	traits::{fungibles::InspectEnumerable, ContainsPair, Get},
};
use parachains_common::{AccountId, AssetIdForTrustBackedAssets, AuraId, Balance};
use sp_consensus_aura::SlotDuration;
use sp_core::crypto::Ss58Codec;
use sp_runtime::{traits::MaybeEquivalence, Either, TryRuntimeError};
use system_parachains_constants::kusama::{
	consensus::RELAY_CHAIN_SLOT_DURATION_MILLIS, currency::UNITS,
	fee::WeightToFee as KsmWeightToFee,
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
type WeightToFee = KsmWeightToFee<Runtime>;

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

#[test]
fn test_ed_is_one_hundredth_of_relay() {
	ExtBuilder::<Runtime>::default()
		.with_tracing()
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
		.with_tracing()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::sr25519::Public::from_raw(ALICE)) },
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
	ExtBuilder::<Runtime>::default()
	.build()
	.execute_with(|| {
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
	});
}

#[test]
fn receive_reserve_asset_deposited_dot_from_asset_hub_polkadot_fees_paid_by_pool_swap_works() {
	const BLOCK_AUTHOR_ACCOUNT: [u8; 32] = [13; 32];
	let block_author_account = AccountId::from(BLOCK_AUTHOR_ACCOUNT);
	let staking_pot = StakingPot::get();

	let foreign_asset_id_location = Location::new(2, [GlobalConsensus(Polkadot)]);
	let reserve_location = Location::new(2, [GlobalConsensus(Polkadot), Parachain(1000)]);
	let foreign_asset_reserve_data =
		ForeignAssetReserveData { reserve: reserve_location, teleportable: false };
	let foreign_asset_id_minimum_balance = 1_000_000_000;
	// sovereign account as foreign asset owner (can be whoever for this scenario)
	let foreign_asset_owner = LocationToAccountId::convert_location(&Location::parent()).unwrap();
	let foreign_asset_create_params = (
		foreign_asset_owner.clone(),
		foreign_asset_id_location.clone(),
		foreign_asset_reserve_data,
		foreign_asset_id_minimum_balance,
	);
	let pool_params =
		(foreign_asset_owner, foreign_asset_id_location.clone(), foreign_asset_id_minimum_balance);

	receive_reserve_asset_deposited_from_different_consensus_works::<
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
		foreign_asset_create_params,
		1000000000000,
		|| {
			// setup pool for paying fees to touch `SwapFirstAssetTrader`
			asset_test_utils::test_cases::setup_pool_for_paying_fees_with_foreign_assets::<
				Runtime,
				RuntimeOrigin,
			>(ExistentialDeposit::get(), pool_params);
			// staking pot account for collecting local native fees from `BuyExecution`
			let _ = Balances::force_set_balance(
				RuntimeOrigin::root(),
				StakingPot::get().into(),
				ExistentialDeposit::get(),
			);
			// prepare bridge configuration
			bridging_to_asset_hub_polkadot()
		},
		(
			[PalletInstance(
				bp_bridge_hub_kusama::WITH_BRIDGE_KUSAMA_TO_POLKADOT_MESSAGES_PALLET_INDEX,
			)]
			.into(),
			GlobalConsensus(Polkadot),
			[Parachain(1000)].into(),
		),
		|| {
			// check staking pot for ED
			assert_eq!(Balances::free_balance(&staking_pot), ExistentialDeposit::get());
			// check now foreign asset for staking pot
			assert_eq!(ForeignAssets::balance(foreign_asset_id_location.clone(), &staking_pot), 0);
		},
		|| {
			// `SwapFirstAssetTrader` - staking pot receives xcm fees in DOTs
			assert!(Balances::free_balance(&staking_pot) > ExistentialDeposit::get());
			// staking pot receives no foreign assets
			assert_eq!(ForeignAssets::balance(foreign_asset_id_location.clone(), &staking_pot), 0);
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
		Unlimited,
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
fn treasury_pallet_account_not_none() {
	ExtBuilder::<Runtime>::default().build().execute_with(|| {
		let relay_treasury_account: AccountId = RelayTreasuryPalletAccount::get();
		assert_eq!(
			relay_treasury_account,
			LocationToAccountId::convert_location(&RelayTreasuryLocation::get()).unwrap()
		)
	});
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
			SessionKeys { aura: AuraId::from(sp_core::sr25519::Public::from_raw(ALICE)) },
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

	// ok - relaychain
	assert_ok!(parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
		Runtime,
		RuntimeOrigin,
	>(GovernanceOrigin::Location(RelayChainLocation::get())));
}

/// A Staking proxy can add/remove a StakingOperator proxy for the account it is proxying.
#[test]
fn staking_proxy_can_manage_staking_operator() {
	use asset_hub_kusama_runtime::{Proxy, ProxyType};
	use frame_support::traits::fungible::Mutate;
	use sp_runtime::traits::StaticLookup;

	ExtBuilder::<Runtime>::default()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::sr25519::Public::from_raw(ALICE)) },
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
	use asset_hub_kusama_runtime::ProxyType;
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
		max_delivery_and_remote_execution_fee: None,
	})));
	assert!(operator.filter(&RuntimeCall::StakingRcClient(RcClientCall::purge_keys {
		max_delivery_and_remote_execution_fee: None,
	})));

	// StakingOperator can batch operations
	assert!(operator.filter(&RuntimeCall::Utility(pallet_utility::Call::batch { calls: vec![] })));

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
	use asset_hub_kusama_runtime::ProxyType;

	let controller: AccountId = ALICE.into();
	let operator: AccountId = [2u8; 32].into();

	ExtBuilder::<Runtime>::default()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			use frame_support::traits::fungible::Mutate;

			// GIVEN: fund controller and operator
			assert_ok!(Balances::mint_into(&controller, 100 * UNITS));
			assert_ok!(Balances::mint_into(&operator, 100 * UNITS));

			// WHEN: controller creates a pure proxy stash with Staking proxy type
			assert_ok!(asset_hub_kusama_runtime::Proxy::create_pure(
				RuntimeOrigin::signed(controller.clone()),
				ProxyType::Staking,
				0,
				0
			));
			let pure_stash = asset_hub_kusama_runtime::Proxy::pure_account(
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
			assert_ok!(asset_hub_kusama_runtime::Proxy::proxy(
				RuntimeOrigin::signed(controller.clone()),
				pure_stash.clone().into(),
				None,
				Box::new(add_operator_call),
			));

			// THEN: operator can call chill on behalf of pure proxy stash
			let chill_call = RuntimeCall::Staking(pallet_staking_async::Call::chill {});
			assert_ok!(asset_hub_kusama_runtime::Proxy::proxy(
				RuntimeOrigin::signed(operator.clone()),
				pure_stash.clone().into(),
				None,
				Box::new(chill_call),
			));

			// THEN: operator can call validate on behalf of pure proxy stash
			let validate_call = RuntimeCall::Staking(pallet_staking_async::Call::validate {
				prefs: Default::default(),
			});
			assert_ok!(asset_hub_kusama_runtime::Proxy::proxy(
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
			assert_ok!(asset_hub_kusama_runtime::Proxy::proxy(
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
			assert_ok!(asset_hub_kusama_runtime::Proxy::proxy(
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
fn session_keys_are_compatible_between_ah_and_rc() {
	use asset_hub_kusama_runtime::staking::RelayChainSessionKeys;
	use sp_runtime::traits::OpaqueKeys;

	// Verify the key type IDs match in order.
	// This ensures that when keys are encoded on AssetHub and decoded on Kusama (or vice versa),
	// they map to the correct key types.
	assert_eq!(
		RelayChainSessionKeys::key_ids(),
		kusama_runtime::SessionKeys::key_ids(),
		"Session key type IDs must match between AssetHub and Kusama"
	);
}
