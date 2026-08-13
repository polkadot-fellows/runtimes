// Copyright (C) Parity Technologies and the various Polkadot contributors, see Contributions.md
// for a list of specific contributors.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{
	xcm_config::{AssetHubLocation, LocationToAccountId, RelayChainLocation},
	Balance, Block, DotWeightToFee as WeightToFee, Runtime, RuntimeCall, RuntimeOrigin,
};
use cumulus_primitives_core::relay_chain::AccountId;
use sp_core::crypto::Ss58Codec;
use xcm::prelude::*;
use xcm_runtime_apis::conversions::LocationToAccountHelper;

use frame_support::{assert_err, assert_ok};
use parachains_runtimes_test_utils::GovernanceOrigin;
use sp_runtime::Either;

const ALICE: [u8; 32] = [1u8; 32];

#[test]
fn location_conversion_works() {
	let alice_32 = AccountId32 { network: None, id: AccountId::from(ALICE).into() };
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
		let expected =
			AccountId::from_string(tc.expected_account_id_str).expect("Invalid AccountId string");

		let got = LocationToAccountHelper::<AccountId, LocationToAccountId>::convert_location(
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
		WeightToFee<Runtime>,
	>();
}

/// Test externalities holding the relay chain's limits, which quoting the delivery fees of an
/// upward message needs and which only the parachain inherent brings in.
fn new_test_ext() -> sp_io::TestExternalities {
	use crate::RuntimeGenesisConfig;
	use cumulus_primitives_core::{relay_chain::AsyncBackingParams, AbridgedHostConfiguration};
	use sp_runtime::BuildStorage;

	let mut ext = sp_io::TestExternalities::new(
		RuntimeGenesisConfig::default().build_storage().expect("runtime genesis builds"),
	);
	ext.execute_with(|| {
		cumulus_pallet_parachain_system::HostConfiguration::<Runtime>::put(
			AbridgedHostConfiguration {
				max_code_size: 3 * 1024 * 1024,
				max_head_data_size: 20 * 1024,
				max_upward_queue_count: 10,
				max_upward_queue_size: 51_200,
				max_upward_message_size: 51_200,
				max_upward_message_num_per_candidate: 10,
				hrmp_max_message_num_per_candidate: 10,
				validation_upgrade_cooldown: 6,
				validation_upgrade_delay: 6,
				async_backing_params: AsyncBackingParams {
					allowed_ancestry_len: 0,
					max_candidate_depth: 0,
				},
			},
		);
	});
	ext
}

/// What the routers charge, in `asset`, to deliver a trivial message to the relay chain.
fn delivery_fees_in(asset: Location) -> Result<Assets, ()> {
	use crate::{xcm_config::XcmConfig, PolkadotXcm};

	type AssetExchanger = <XcmConfig as xcm_executor::Config>::AssetExchanger;
	PolkadotXcm::query_delivery_fees::<AssetExchanger>(
		VersionedLocation::from(Location::parent()),
		VersionedXcm::from(Xcm::<()>::builder_unsafe().clear_origin().build()),
		AssetId(asset).into(),
	)
	.map(|fees| Assets::try_from(fees).expect("fees are in the latest version"))
	.map_err(|_| ())
}

/// Registers `asset` and opens an `AssetConversion` pool holding `dot` DOT against `amount` of it,
/// funded by `provider`.
fn create_pool_with(asset: Location, provider: AccountId, dot: Balance, amount: Balance) {
	use crate::{xcm_config::RelayLocation, AssetConversion, Assets as AssetsPallet, Balances};
	use frame_support::traits::{fungible::Mutate as _, fungibles::Mutate as _};

	assert_ok!(AssetsPallet::force_create(
		RuntimeOrigin::root(),
		asset.clone(),
		provider.clone().into(),
		true, // sufficient, so no DOT is needed to hold it
		1,
	));
	// Enough over the liquidity to cover the pool setup fee and the existential deposits.
	assert_ok!(Balances::mint_into(&provider, dot * 2));
	assert_ok!(AssetsPallet::mint_into(asset.clone(), &provider, amount * 2));

	assert_ok!(AssetConversion::create_pool(
		RuntimeOrigin::signed(provider.clone()),
		Box::new(RelayLocation::get()),
		Box::new(asset.clone()),
	));
	assert_ok!(AssetConversion::add_liquidity(
		RuntimeOrigin::signed(provider.clone()),
		Box::new(RelayLocation::get()),
		Box::new(asset),
		dot,    // DOT desired
		amount, // asset desired
		1,      // DOT min
		1,      // asset min
		provider,
	));
}

/// XCM execution and delivery fees can both be paid in any asset that has a pool against DOT, by
/// swapping just enough of it for the DOT the fee is priced in. No governance action is needed to
/// enable an asset — opening a pool is permissionless.
#[test]
fn xcm_fees_can_be_paid_in_any_asset_with_a_pool() {
	use crate::{
		xcm_config::{RelayLocation, XcmConfig},
		AssetConversion, PolkadotXcm, UNITS,
	};
	use frame_support::weights::WeightToFee as WeightToFeeT;

	// An asset with a pool holding four of it to the DOT, and one with no pool at all.
	let pooled = Location::new(1, [Parachain(2034), GeneralIndex(222)]);
	let poolless = Location::new(1, [Parachain(2034), GeneralIndex(333)]);
	let provider = AccountId::from(ALICE);

	new_test_ext().execute_with(|| {
		create_pool_with(pooled.clone(), provider, 10_000 * UNITS, 40_000 * UNITS);

		// What the pool charges, in the asset, to buy exactly `native` DOT.
		let cost_of_dot = |native: Balance| {
			AssetConversion::quote_price_tokens_for_exact_tokens(
				pooled.clone(),
				RelayLocation::get(),
				native,
				true,
			)
			.expect("the pool is funded")
		};
		// What `native` DOT is worth in the asset, going the other way through the pool.
		let value_of_dot = |native: Balance| {
			AssetConversion::quote_price_exact_tokens_for_tokens(
				RelayLocation::get(),
				pooled.clone(),
				native,
				true,
			)
			.expect("the pool is funded")
		};

		// Execution fees are priced in DOT and settled by swapping the asset for that much DOT.
		let weight = Weight::from_parts(1_000_000_000, 10_000);
		let native_fee = WeightToFee::<Runtime>::weight_to_fee(&weight);
		type Trader = <XcmConfig as xcm_executor::Config>::Trader;
		let charged = PolkadotXcm::query_weight_to_asset_fee::<Trader>(
			weight,
			AssetId(pooled.clone()).into(),
		)
		.unwrap();
		assert_eq!(charged, cost_of_dot(native_fee));
		// Four to the DOT, so a little over four times the DOT price once the pool takes its cut.
		assert!(charged > native_fee * 4, "{charged} should exceed {} ", native_fee * 4);

		// An asset with neither a pool nor a registered rate buys no execution.
		assert!(PolkadotXcm::query_weight_to_asset_fee::<Trader>(
			weight,
			AssetId(poolless.clone()).into()
		)
		.is_err());

		// Delivery fees are quoted by the routers in DOT, and settled through the same pool.
		let in_dot = delivery_fees_in(RelayLocation::get()).expect("the relay chain is routable");
		let Some(Asset { fun: Fungible(dot_fee), .. }) = in_dot.get(0).cloned() else {
			panic!("delivery fees are a single fungible asset: {in_dot:?}");
		};
		// Note that `XcmPaymentApi::query_delivery_fees` quotes *maximally*, i.e. what the DOT the
		// routers ask for is worth in the asset, not what buying that DOT costs. The executor
		// itself quotes minimally when it settles the fee, so the amount actually taken is
		// `cost_of_dot(dot_fee)`, a little more than this. Callers should leave room for that.
		assert_eq!(
			delivery_fees_in(pooled.clone()).unwrap(),
			(pooled.clone(), value_of_dot(dot_fee)).into()
		);
		assert!(cost_of_dot(dot_fee) > value_of_dot(dot_fee));
		// And an asset with no pool cannot pay for delivery either.
		assert!(delivery_fees_in(poolless).is_err());
	});
}

/// Runs the configured transaction-fee charger over a full withdraw-then-correct cycle, the way
/// the `ChargeAssetTxPayment` extension does, and returns what it took in `asset`.
fn charge_transaction_fee_in(asset: Location, payer: &AccountId, fee: Balance) -> Balance {
	use frame_support::dispatch::GetDispatchInfo;
	use pallet_asset_conversion_tx_payment::OnChargeAssetTransaction;

	// `OnChargeAssetTransaction` is generic over the runtime, so the impl is named explicitly.
	type Charger =
		<Runtime as pallet_asset_conversion_tx_payment::Config>::OnChargeAssetTransaction;

	let call = RuntimeCall::System(frame_system::Call::remark { remark: Vec::new() });
	let info = call.get_dispatch_info();

	assert_ok!(<Charger as OnChargeAssetTransaction<Runtime>>::can_withdraw_fee(
		payer,
		asset.clone(),
		fee
	));
	let withdrawn = <Charger as OnChargeAssetTransaction<Runtime>>::withdraw_fee(
		payer,
		&call,
		&info,
		asset.clone(),
		fee,
		0,
	)
	.expect("the payer can cover the fee");
	<Charger as OnChargeAssetTransaction<Runtime>>::correct_and_deposit_fee(
		payer,
		&info,
		&Default::default(),
		fee,
		0,
		asset,
		withdrawn,
	)
	.expect("the fee settles")
}

/// Transaction fees can be paid in any asset with a pool: the signer needs no DOT at all, and the
/// staking pot is still paid in DOT.
#[test]
fn transaction_fees_can_be_paid_in_any_asset_with_a_pool() {
	use crate::{xcm_config::StakingPot, Assets as AssetsPallet, Balances, UNITS};
	use frame_support::traits::{fungible::Inspect as _, fungibles::Mutate as _};

	let pooled = Location::new(1, [Parachain(2034), GeneralIndex(222)]);
	let provider = AccountId::from(ALICE);
	let payer = AccountId::from([2u8; 32]);

	new_test_ext().execute_with(|| {
		create_pool_with(pooled.clone(), provider, 10_000 * UNITS, 40_000 * UNITS);

		// The payer holds the asset and nothing else. The asset is sufficient, so it needs no DOT
		// to keep the account alive either.
		assert_ok!(AssetsPallet::mint_into(pooled.clone(), &payer, 100 * UNITS));
		assert_eq!(Balances::balance(&payer), 0);

		let staking_pot = StakingPot::get();
		let pot_dot_before = Balances::balance(&staking_pot);
		let fee = UNITS / 100;

		let paid = charge_transaction_fee_in(pooled.clone(), &payer, fee);

		// The fee came out of the payer's asset balance, and only out of it.
		assert_eq!(AssetsPallet::balance(pooled.clone(), &payer), 100 * UNITS - paid);
		assert_eq!(Balances::balance(&payer), 0);
		// The staking pot was paid the DOT the fee is denominated in, not the asset.
		assert_eq!(Balances::balance(&staking_pot), pot_dot_before + fee);
		assert_eq!(AssetsPallet::balance(pooled, &staking_pot), 0);
		// Which cost a little over four times as much of the asset, at the pool's price.
		assert!(paid > fee * 4, "{paid} should exceed {}", fee * 4);
	});
}

/// Transaction fees fall back to the rate governance registered for an asset that has no pool, and
/// are then taken in kind at exactly that rate — the same deal the XCM fee paths offer.
#[test]
fn transaction_fees_fall_back_to_a_registered_rate_when_there_is_no_pool() {
	use crate::{xcm_config::StakingPot, AssetRate, Assets as AssetsPallet, Balances, UNITS};
	use frame_support::traits::{fungible::Inspect as _, fungibles::Mutate as _};
	use sp_runtime::FixedU128;

	// An asset worth 4 DOT apiece, with no pool anywhere.
	let rated = Location::new(1, [Parachain(2034), GeneralIndex(222)]);
	let payer = AccountId::from([2u8; 32]);

	new_test_ext().execute_with(|| {
		assert_ok!(AssetsPallet::force_create(
			RuntimeOrigin::root(),
			rated.clone(),
			payer.clone().into(),
			true,
			1,
		));
		assert_ok!(AssetRate::create(
			RuntimeOrigin::root(),
			Box::new(rated.clone()),
			FixedU128::from_u32(4)
		));
		assert_ok!(AssetsPallet::mint_into(rated.clone(), &payer, 100 * UNITS));
		assert_eq!(Balances::balance(&payer), 0);

		let staking_pot = StakingPot::get();
		let pot_dot_before = Balances::balance(&staking_pot);
		let fee = UNITS / 100;

		let paid = charge_transaction_fee_in(rated.clone(), &payer, fee);

		// Exactly the registered rate, with no pool fee or slippage on top.
		assert_eq!(paid, fee / 4);
		assert_eq!(AssetsPallet::balance(rated.clone(), &payer), 100 * UNITS - paid);
		assert_eq!(Balances::balance(&payer), 0);
		// And with no pool to swap against, the staking pot is paid in the asset itself.
		assert_eq!(AssetsPallet::balance(rated, &staking_pot), paid);
		assert_eq!(Balances::balance(&staking_pot), pot_dot_before);
	});
}

/// An asset that has both a pool and a registered rate pays through the pool: the fallback is only
/// for assets the pool cannot price.
#[test]
fn transaction_fees_prefer_the_pool_over_a_registered_rate() {
	use crate::{xcm_config::StakingPot, AssetRate, Assets as AssetsPallet, Balances, UNITS};
	use frame_support::traits::{fungible::Inspect as _, fungibles::Mutate as _};
	use sp_runtime::FixedU128;

	let both = Location::new(1, [Parachain(2034), GeneralIndex(222)]);
	let provider = AccountId::from(ALICE);
	let payer = AccountId::from([2u8; 32]);

	new_test_ext().execute_with(|| {
		create_pool_with(both.clone(), provider, 10_000 * UNITS, 40_000 * UNITS);
		assert_ok!(AssetRate::create(
			RuntimeOrigin::root(),
			Box::new(both.clone()),
			FixedU128::from_u32(4)
		));
		assert_ok!(AssetsPallet::mint_into(both.clone(), &payer, 100 * UNITS));

		let staking_pot = StakingPot::get();
		let pot_dot_before = Balances::balance(&staking_pot);
		let fee = UNITS / 100;

		let paid = charge_transaction_fee_in(both.clone(), &payer, fee);

		// Paid through the pool: the staking pot got DOT rather than the asset, and the amount
		// carries the pool's cut instead of being the flat `fee / 4` the rate would have charged.
		assert_eq!(Balances::balance(&staking_pot), pot_dot_before + fee);
		assert_eq!(AssetsPallet::balance(both, &staking_pot), 0);
		assert!(paid > fee * 4, "{paid} should exceed the rate's {}", fee / 4);
	});
}

/// Assets with no pool fall back to the rate governance registered for them in
/// `pallet-asset-rate`, for both execution and delivery fees, and are taken in kind at that rate.
#[test]
fn xcm_fees_fall_back_to_a_registered_rate_when_there_is_no_pool() {
	use crate::{
		xcm_config::{FeesAtAssetRate, RelayLocation, XcmConfig},
		AssetRate, Assets as AssetsPallet, PolkadotXcm,
	};
	use frame_support::weights::WeightToFee as WeightToFeeT;
	use sp_runtime::FixedU128;
	use xcm_executor::traits::AssetExchange;

	// An asset worth 4 DOT apiece, and one governance never registered a rate for.
	let rated = Location::new(1, [Parachain(2034), GeneralIndex(222)]);
	let unrated = Location::new(1, [Parachain(2034), GeneralIndex(333)]);
	let rate = FixedU128::from_u32(4);

	new_test_ext().execute_with(|| {
		assert_ok!(AssetsPallet::force_create(
			RuntimeOrigin::root(),
			rated.clone(),
			AccountId::from(ALICE).into(),
			true,
			1,
		));
		assert_ok!(AssetRate::create(RuntimeOrigin::root(), Box::new(rated.clone()), rate));

		// Execution fees are priced in DOT, then converted at the registered rate.
		let weight = Weight::from_parts(1_000_000_000, 10_000);
		let native_fee = WeightToFee::<Runtime>::weight_to_fee(&weight);
		type Trader = <XcmConfig as xcm_executor::Config>::Trader;
		assert_eq!(
			PolkadotXcm::query_weight_to_asset_fee::<Trader>(weight, AssetId(rated.clone()).into())
				.unwrap(),
			native_fee / 4,
		);
		// An asset with neither a pool nor a rate buys no execution.
		assert!(PolkadotXcm::query_weight_to_asset_fee::<Trader>(
			weight,
			AssetId(unrated.clone()).into()
		)
		.is_err());

		// Delivery fees are quoted by the router in DOT, and settled in the asset at the same rate.
		let in_dot = delivery_fees_in(RelayLocation::get()).expect("the relay chain is routable");
		let Some(Asset { fun: Fungible(dot_fee), .. }) = in_dot.get(0).cloned() else {
			panic!("delivery fees are a single fungible asset: {in_dot:?}");
		};
		assert_eq!(delivery_fees_in(rated.clone()).unwrap(), (rated.clone(), dot_fee / 4).into());
		// And an asset without a rate cannot pay for delivery either.
		assert!(delivery_fees_in(unrated.clone()).is_err());

		// The same rate prices what the executor asks for, in both directions.
		let dot_asked: Assets = (RelayLocation::get(), 400u128).into();
		assert_eq!(
			FeesAtAssetRate::quote_exchange_price(
				&Asset { id: AssetId(rated.clone()), fun: Fungible(0) }.into(),
				&dot_asked,
				false,
			),
			Some((rated.clone(), 100u128).into()),
		);
		assert_eq!(
			FeesAtAssetRate::quote_exchange_price(
				&(rated.clone(), 100u128).into(),
				&(RelayLocation::get(), 1u128).into(),
				true,
			),
			Some((RelayLocation::get(), 400u128).into()),
		);
		assert_eq!(
			FeesAtAssetRate::quote_exchange_price(
				&Asset { id: AssetId(unrated), fun: Fungible(0) }.into(),
				&dot_asked,
				false,
			),
			None,
		);
	});
}

#[test]
fn governance_authorize_upgrade_works() {
	use polkadot_runtime_constants::system_parachain::COLLECTIVES_ID;

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
	>(GovernanceOrigin::Location(RelayChainLocation::get())));

	// ok - AssetHub
	assert_ok!(parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
		Runtime,
		RuntimeOrigin,
	>(GovernanceOrigin::Location(AssetHubLocation::get())));
}
