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
	assets::hollar::HOLLAR_UNITS,
	xcm_config::{AssetHubLocation, LocationToAccountId, RelayChainLocation},
	Balance, Block, DotWeightToFee as WeightToFee, PeopleAirdrops, Runtime, RuntimeCall,
	RuntimeOrigin, UNITS,
};
use cumulus_primitives_core::relay_chain::AccountId;
use sp_core::crypto::Ss58Codec;
use xcm::prelude::*;
use xcm_runtime_apis::conversions::LocationToAccountHelper;

use frame_support::{assert_err, assert_noop, assert_ok};
use parachains_runtimes_test_utils::GovernanceOrigin;
use sp_runtime::{Either, MultiAddress};

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

/// Everything the fee tests need: an asset registered in `Assets`, optionally priced by a pool,
/// optionally priced by a governance rate, and a payer holding some of it.
#[cfg(test)]
mod fee_assets {
	use super::*;
	use crate::{
		AssetConversion, AssetRate, Assets as AssetsPallet, Balance, Balances, RuntimeGenesisConfig,
	};
	use frame_support::traits::{
		fungible::Mutate as _, fungibles::Mutate as _, tokens::ConversionToAssetBalance,
	};
	use sp_runtime::{BuildStorage, FixedU128};

	pub use crate::xcm_config::RelayLocation;

	/// An asset issued by Asset Hub's trust backed `Assets` pallet, as seen from here.
	///
	/// Deliberately *not* HOLLAR: HOLLAR is `(1, Parachain(2034), GeneralIndex(222))`, and pinning
	/// these tests to it would leave the generic paths untested. `asset_locations_are_not_hollar`
	/// keeps them apart.
	pub fn foreign_asset(id: u128) -> Location {
		Location::new(1, [Parachain(1000), PalletInstance(50), GeneralIndex(id)])
	}

	pub fn new_test_ext() -> sp_io::TestExternalities {
		sp_io::TestExternalities::new(
			RuntimeGenesisConfig::default().build_storage().expect("runtime genesis builds"),
		)
	}

	/// Registers `asset` as a sufficient asset and gives `ALICE` DOT and plenty of it.
	pub fn register(asset: &Location) {
		let alice = AccountId::from(ALICE);
		assert_ok!(AssetsPallet::force_create(
			RuntimeOrigin::root(),
			asset.clone(),
			alice.clone().into(),
			true, // is_sufficient
			1,    // min_balance
		));
		assert_ok!(Balances::mint_into(&alice, 100_000 * UNITS));
		assert_ok!(AssetsPallet::mint_into(asset.clone(), &alice, 100_000 * UNITS));
	}

	/// Opens a DOT/`asset` pool and seeds it at one for one. Anyone can do this; no governance
	/// origin is involved.
	pub fn open_pool(asset: &Location) {
		open_pool_with(asset, 1_000 * UNITS, 1_000 * UNITS);
	}

	/// Opens a DOT/`asset` pool holding `dot` on the DOT side and `amount` of `asset` on the
	/// other, which is what sets the price the pool quotes.
	pub fn open_pool_with(asset: &Location, dot: Balance, amount: Balance) {
		let alice = AccountId::from(ALICE);
		assert_ok!(AssetConversion::create_pool(
			RuntimeOrigin::signed(alice.clone()),
			Box::new(RelayLocation::get()),
			Box::new(asset.clone()),
		));
		assert_ok!(AssetConversion::add_liquidity(
			RuntimeOrigin::signed(alice.clone()),
			Box::new(RelayLocation::get()),
			Box::new(asset.clone()),
			dot,
			amount,
			1,
			1,
			alice,
		));
	}

	/// One asset per pricing path, all registered and held by `ALICE`.
	pub struct PricedAssets {
		/// A pool against DOT and no rate.
		pub pooled: Location,
		/// A governance rate of 4 DOT per unit and no pool.
		pub rated: Location,
		/// Both, with the pool quoting the lower price.
		pub pool_cheaper: Location,
		/// Both, with the registered rate quoting the lower price.
		pub rate_cheaper: Location,
		/// Neither.
		pub unpriced: Location,
	}

	/// Registers one asset per pricing path. `ALICE` holds all of them.
	pub fn priced_assets() -> PricedAssets {
		let assets = PricedAssets {
			pooled: foreign_asset(111),
			rated: foreign_asset(222),
			pool_cheaper: foreign_asset(333),
			rate_cheaper: foreign_asset(444),
			unpriced: foreign_asset(555),
		};
		for asset in [
			&assets.pooled,
			&assets.rated,
			&assets.pool_cheaper,
			&assets.rate_cheaper,
			&assets.unpriced,
		] {
			register(asset);
		}

		open_pool(&assets.pooled);
		set_rate(&assets.rated, 4);

		// One unit is worth 10 DOT in the pool but only 1 by the registered rate, so the pool
		// asks for a tenth of what the rate does.
		open_pool_with(&assets.pool_cheaper, 1_000 * UNITS, 100 * UNITS);
		set_rate(&assets.pool_cheaper, 1);

		// One unit is worth 1 DOT in the pool but 4 by the registered rate, so the rate asks for
		// a quarter of what the pool does.
		open_pool(&assets.rate_cheaper);
		set_rate(&assets.rate_cheaper, 4);

		assets
	}

	/// Registers a governance rate saying one unit of `asset` is worth `dot_per_unit` DOT.
	pub fn set_rate(asset: &Location, dot_per_unit: u32) {
		assert_ok!(AssetRate::create(
			RuntimeOrigin::root(),
			Box::new(asset.clone()),
			FixedU128::from_u32(dot_per_unit),
		));
	}

	/// What the pool charges in `asset` for exactly `native` DOT.
	pub fn pool_price_of(asset: &Location, native: Balance) -> Balance {
		AssetConversion::quote_price_tokens_for_exact_tokens(
			asset.clone(),
			RelayLocation::get(),
			native,
			true,
		)
		.expect("the asset is in a pool with DOT")
	}

	/// What the governance rate charges in `asset` for `native` DOT.
	pub fn rate_price_of(asset: &Location, native: Balance) -> Balance {
		AssetRate::to_asset_balance(native, asset.clone()).expect("the asset has a rate")
	}

	/// What the governance rate charges in `asset`, or `None` if it has no rate.
	pub fn rate_price_if_any(asset: &Location, native: Balance) -> Option<Balance> {
		AssetRate::to_asset_balance(native, asset.clone()).ok()
	}

	/// The cheaper of the two oracles for `asset` — what the runtime must actually charge.
	///
	/// Panics if neither prices it, so a test that expects a price cannot silently pass on an
	/// asset nothing can price.
	pub fn cheapest_price_of(asset: &Location, native: Balance) -> Balance {
		let pool = AssetConversion::quote_price_tokens_for_exact_tokens(
			asset.clone(),
			RelayLocation::get(),
			native,
			true,
		);
		let rate = rate_price_if_any(asset, native);
		match (pool, rate) {
			(Some(pool), Some(rate)) => pool.min(rate),
			(Some(pool), None) => pool,
			(None, Some(rate)) => rate,
			(None, None) => panic!("{asset:?} is priced by neither oracle"),
		}
	}
}

/// Only assets Asset Hub itself issues may be reserve transferred here from Asset Hub.
///
/// Accepting an asset from a chain that is not its real reserve gives it two reserves, and
/// `ReserveAssetDeposited` mints locally — so the impostor reserve can credit this chain with
/// holdings nobody is backing. The cases below are the ones that would actually hurt.
#[test]
fn only_asset_hub_native_assets_are_reserve_accepted_from_asset_hub() {
	use crate::{
		assets::hollar::HollarLocation,
		xcm_config::{AssetHubLocation, TrustedReserves},
	};
	use frame_support::traits::ContainsPair;

	let asset_hub = AssetHubLocation::get();
	let hydration = Location::new(1, [Parachain(2034)]);
	let accepted = |asset: Location, from: &Location| {
		TrustedReserves::contains(&Asset { id: AssetId(asset), fun: Fungible(1) }, from)
	};

	// Asset Hub is the reserve for the assets it issues: trust backed assets and pool tokens.
	assert!(accepted(
		Location::new(1, [Parachain(1000), PalletInstance(50), GeneralIndex(4242)]),
		&asset_hub,
	));
	assert!(accepted(
		Location::new(1, [Parachain(1000), PalletInstance(54), GeneralIndex(7)]),
		&asset_hub,
	));

	// DOT must never arrive as a reserve asset: `FungibleTransactor` has no checking account, so
	// depositing it mints into `Balances`. DOT only ever arrives by teleport.
	assert!(!accepted(Location::parent(), &asset_hub));

	// HOLLAR has a reserve already — Hydration — and must not gain a second one.
	assert!(accepted(HollarLocation::get(), &hydration), "Hydration still backs HOLLAR");
	assert!(!accepted(HollarLocation::get(), &asset_hub));

	// Neither may Asset Hub vouch for assets it merely custodies.
	assert!(!accepted(
		Location::new(2, [GlobalConsensus(NetworkId::Ethereum { chain_id: 1 })]),
		&asset_hub,
	));
	assert!(!accepted(Location::new(2, [GlobalConsensus(NetworkId::Kusama)]), &asset_hub));
	assert!(!accepted(Location::new(1, [Parachain(2000), GeneralIndex(1)]), &asset_hub));

	// And an Asset Hub asset is only accepted *from* Asset Hub.
	let ah_asset = Location::new(1, [Parachain(1000), PalletInstance(50), GeneralIndex(4242)]);
	assert!(!accepted(ah_asset.clone(), &hydration));
	assert!(!accepted(ah_asset, &Location::parent()));
}

/// None of the fee tests may use HOLLAR: it is the one asset that already worked before pools, so
/// testing with it would not show that an arbitrary asset is payable.
#[test]
fn fee_test_assets_are_not_hollar() {
	use crate::assets::hollar::HollarLocation;
	use fee_assets::*;

	let hollar = HollarLocation::get();
	for id in [111, 222, 333, 444, 555] {
		assert_ne!(foreign_asset(id), hollar, "fee tests must not be pinned to HOLLAR");
	}
}

/// XCM execution fees are charged through whichever of the pool and the governance-registered
/// rate asks the payer for less.
#[test]
fn xcm_execution_fees_charge_the_cheaper_of_pool_and_rate() {
	use crate::{xcm_config::XcmConfig, PolkadotXcm};
	use fee_assets::*;
	use frame_support::weights::WeightToFee as WeightToFeeT;

	new_test_ext().execute_with(|| {
		let a = priced_assets();

		let weight = Weight::from_parts(1_000_000_000, 10_000);
		let native_fee = WeightToFee::<Runtime>::weight_to_fee(&weight);
		type Trader = <XcmConfig as xcm_executor::Config>::Trader;
		let quote = |asset: &Location| {
			PolkadotXcm::query_weight_to_asset_fee::<Trader>(weight, AssetId(asset.clone()).into())
		};

		// DOT itself is priced by the plain `UsingComponents` trader.
		assert_eq!(quote(&RelayLocation::get()).unwrap(), native_fee);
		// A pool-only asset is swapped for exactly `native_fee` worth of DOT.
		assert_eq!(quote(&a.pooled).unwrap(), pool_price_of(&a.pooled, native_fee));
		// A rate-only asset is taken in kind at that rate.
		assert_eq!(quote(&a.rated).unwrap(), native_fee / 4);
		// Every priced asset is charged the *minimum* of the two oracles, whichever that is.
		for asset in [&a.pooled, &a.rated, &a.pool_cheaper, &a.rate_cheaper] {
			assert_eq!(
				quote(asset).unwrap(),
				cheapest_price_of(asset, native_fee),
				"{asset:?} must be charged the cheaper of pool and rate",
			);
		}
		// And the two really do disagree, in both directions, so the minimum is not a no-op.
		assert!(
			pool_price_of(&a.pool_cheaper, native_fee) < rate_price_of(&a.pool_cheaper, native_fee)
		);
		assert!(
			rate_price_of(&a.rate_cheaper, native_fee) < pool_price_of(&a.rate_cheaper, native_fee)
		);
		// And an asset nobody priced buys no execution.
		assert!(quote(&a.unpriced).is_err());
	});
}

/// The same rule applies to delivery fees, which the routers always quote in DOT.
#[test]
fn xcm_delivery_fees_charge_the_cheaper_of_pool_and_rate() {
	use crate::{xcm_config::XcmConfig, AssetConversion, ParachainSystem, PolkadotXcm};
	use cumulus_primitives_core::UpwardMessageSender;
	use fee_assets::*;
	use sp_runtime::traits::Zero;

	new_test_ext().execute_with(|| {
		let a = priced_assets();

		// Sending upwards needs the relay chain's limits, which only the inherent brings in.
		<ParachainSystem as UpwardMessageSender>::ensure_successful_delivery();

		type AssetExchanger = <XcmConfig as xcm_executor::Config>::AssetExchanger;
		let message = Xcm::<()>::builder_unsafe().clear_origin().build();
		let quote = |asset: &Location| -> Result<Assets, ()> {
			PolkadotXcm::query_delivery_fees::<AssetExchanger>(
				VersionedLocation::from(Location::parent()),
				VersionedXcm::from(message.clone()),
				AssetId(asset.clone()).into(),
			)
			.map(|fees| Assets::try_from(fees).expect("fees are in the latest version"))
			.map_err(|_| ())
		};

		let in_dot = quote(&RelayLocation::get()).expect("the relay chain is routable");
		let Some(Asset { fun: Fungible(dot_fee), .. }) = in_dot.get(0).cloned() else {
			panic!("delivery fees are a single fungible asset: {in_dot:?}");
		};
		assert!(!dot_fee.is_zero());

		let through_pool = |asset: &Location| {
			AssetConversion::quote_price_exact_tokens_for_tokens(
				RelayLocation::get(),
				asset.clone(),
				dot_fee,
				true,
			)
			.expect("the asset is in a pool with DOT")
		};

		// A pool-only asset is sold for the DOT; a rate-only asset is taken in kind at its rate.
		assert_eq!(quote(&a.pooled).unwrap(), (a.pooled.clone(), through_pool(&a.pooled)).into());
		assert_eq!(quote(&a.rated).unwrap(), (a.rated.clone(), dot_fee / 4).into());
		// Every priced asset is quoted the *minimum* of the two oracles. Delivery fees are priced
		// in the other direction from execution fees — "what does this DOT buy" rather than "what
		// does this cost" — so the pool leg is quoted with `exact_tokens_for_tokens`.
		for asset in [&a.pooled, &a.rated, &a.pool_cheaper, &a.rate_cheaper] {
			let pool = AssetConversion::quote_price_exact_tokens_for_tokens(
				RelayLocation::get(),
				asset.clone(),
				dot_fee,
				true,
			);
			let rate = rate_price_if_any(asset, dot_fee);
			let cheapest = match (pool, rate) {
				(Some(pool), Some(rate)) => pool.min(rate),
				(Some(pool), None) => pool,
				(None, Some(rate)) => rate,
				(None, None) => panic!("{asset:?} is priced by neither oracle"),
			};
			assert_eq!(
				quote(asset).unwrap(),
				(asset.clone(), cheapest).into(),
				"{asset:?} must be quoted the cheaper of pool and rate",
			);
		}
		// And the two really do disagree, in both directions.
		assert!(through_pool(&a.pool_cheaper) < dot_fee);
		assert!(dot_fee / 4 < through_pool(&a.rate_cheaper));
		// And an asset nobody priced cannot pay for delivery either.
		assert!(quote(&a.unpriced).is_err());
	});
}

/// A rate high enough that the fee rounds down to nothing must still cost one unit of the asset,
/// for delivery fees as for the transaction and execution fees, or the fee would be waived.
#[test]
fn rated_delivery_fee_is_never_rounded_down_to_nothing() {
	use crate::{xcm_config::XcmConfig, AssetRate, ParachainSystem, PolkadotXcm};
	use cumulus_primitives_core::UpwardMessageSender;
	use fee_assets::*;
	use frame_support::traits::tokens::ConversionToAssetBalance;

	new_test_ext().execute_with(|| {
		<ParachainSystem as UpwardMessageSender>::ensure_successful_delivery();

		type AssetExchanger = <XcmConfig as xcm_executor::Config>::AssetExchanger;
		let message = Xcm::<()>::builder_unsafe().clear_origin().build();
		let quote = |asset: &Location| -> Assets {
			PolkadotXcm::query_delivery_fees::<AssetExchanger>(
				VersionedLocation::from(Location::parent()),
				VersionedXcm::from(message.clone()),
				AssetId(asset.clone()).into(),
			)
			.map(|fees| Assets::try_from(fees).expect("fees are in the latest version"))
			.expect("the relay chain is routable")
		};
		let Some(Asset { fun: Fungible(dot_fee), .. }) =
			quote(&RelayLocation::get()).get(0).cloned()
		else {
			panic!("delivery fees are a single fungible asset");
		};

		// One unit of the asset is worth more DOT than the whole fee, so the rate alone would
		// price the fee at zero units.
		let asset = foreign_asset(666);
		register(&asset);
		set_rate(&asset, u32::MAX);
		assert_eq!(AssetRate::to_asset_balance(dot_fee, asset.clone()).ok(), Some(0));

		assert_eq!(quote(&asset), (asset.clone(), 1u128).into());
	});
}

/// Transaction fees take the same rule, and settle in the asset the winning path implies: DOT
/// into the staking pot for the pool, the asset itself for the rate.
#[test]
fn transaction_fees_charge_the_cheaper_of_pool_and_rate() {
	use crate::{xcm_config::StakingPot, Assets as AssetsPallet, Balance, Balances, RuntimeCall};
	use fee_assets::*;
	use frame_support::{dispatch::GetDispatchInfo, traits::fungible::Inspect as _};
	use pallet_asset_conversion_tx_payment::OnChargeAssetTransaction;
	use sp_runtime::traits::Dispatchable;

	type Charger =
		<Runtime as pallet_asset_conversion_tx_payment::Config>::OnChargeAssetTransaction;

	new_test_ext().execute_with(|| {
		let a = priced_assets();

		let payer = AccountId::from(ALICE);
		let pot = StakingPot::get();
		let call: RuntimeCall = frame_system::Call::remark { remark: alloc::vec![] }.into();
		let info = call.get_dispatch_info();
		let post_info = <RuntimeCall as Dispatchable>::PostInfo::default();
		let fee: Balance = UNITS / 100;

		// Charges `fee` in `asset` and returns what the payer was charged.
		let charge = |asset: &Location| -> Balance {
			assert_ok!(Charger::can_withdraw_fee(&payer, asset.clone(), fee));
			let before = AssetsPallet::balance(asset.clone(), &payer);
			let paid = Charger::withdraw_fee(&payer, &call, &info, asset.clone(), fee, 0)
				.expect("the asset is priced");
			let charged = Charger::correct_and_deposit_fee(
				&payer,
				&info,
				&post_info,
				fee,
				0,
				asset.clone(),
				paid,
			)
			.unwrap();
			assert_eq!(AssetsPallet::balance(asset.clone(), &payer), before - charged);
			charged
		};

		// An asset nobody priced cannot pay for a transaction.
		assert!(Charger::can_withdraw_fee(&payer, a.unpriced.clone(), fee).is_err());

		// A pool-only asset is swapped, so the staking pot is paid in DOT. The expected price is
		// read before charging, because the swap itself moves the pool.
		let pot_dot_before = Balances::balance(&pot);
		let expected = pool_price_of(&a.pooled, fee);
		assert_eq!(charge(&a.pooled), expected);
		assert_eq!(Balances::balance(&pot), pot_dot_before + fee);

		// A rate-only asset is taken in kind, so the pot is paid in the asset and no DOT moves.
		let pot_asset_before = AssetsPallet::balance(a.rated.clone(), &pot);
		let pot_dot_before = Balances::balance(&pot);
		let expected = rate_price_of(&a.rated, fee);
		assert_eq!(expected, fee / 4);
		assert_eq!(charge(&a.rated), expected);
		assert_eq!(AssetsPallet::balance(a.rated.clone(), &pot), pot_asset_before + expected);
		assert_eq!(Balances::balance(&pot), pot_dot_before);

		// Whichever oracle is cheaper is the one that charges, and the settlement asset follows it.
		// `charge` moves the pools, so each expectation is read immediately before its charge.
		let pot_dot_before = Balances::balance(&pot);
		let expected = pool_price_of(&a.pool_cheaper, fee);
		assert!(expected < rate_price_of(&a.pool_cheaper, fee));
		assert_eq!(charge(&a.pool_cheaper), expected);
		assert_eq!(Balances::balance(&pot), pot_dot_before + fee);

		let pot_asset_before = AssetsPallet::balance(a.rate_cheaper.clone(), &pot);
		let pot_dot_before = Balances::balance(&pot);
		let expected = rate_price_of(&a.rate_cheaper, fee);
		assert!(expected < pool_price_of(&a.rate_cheaper, fee));
		assert_eq!(charge(&a.rate_cheaper), expected);
		assert_eq!(
			AssetsPallet::balance(a.rate_cheaper.clone(), &pot),
			pot_asset_before + expected,
		);
		assert_eq!(Balances::balance(&pot), pot_dot_before);
	});
}

/// A pool so thin that a single fee moves its price must never make an asset more expensive than
/// the rate governance registered for it.
///
/// Anyone can open a pool for any registered asset, and slippage — unlike a mispricing — is not
/// something arbitrage repairs: a correctly priced 1 DOT pool still doubles the cost of a 0.5 DOT
/// fee. Taking the cheaper of the two oracles is what makes that unprofitable to attempt.
#[test]
fn a_thin_pool_never_overcharges_against_the_registered_rate() {
	use crate::{Assets as AssetsPallet, Balance};
	use fee_assets::*;
	use pallet_asset_conversion_tx_payment::OnChargeAssetTransaction;

	type Charger =
		<Runtime as pallet_asset_conversion_tx_payment::Config>::OnChargeAssetTransaction;

	let asset = foreign_asset(777);
	new_test_ext().execute_with(|| {
		register(&asset);
		// One unit of the asset is worth one DOT, per governance.
		set_rate(&asset, 1);
		// A griefer opens the thinnest pool that will take a fee: 1 DOT a side.
		open_pool_with(&asset, UNITS, UNITS);

		let payer = AccountId::from(ALICE);
		let call: crate::RuntimeCall = frame_system::Call::remark { remark: alloc::vec![] }.into();
		let info = frame_support::dispatch::GetDispatchInfo::get_dispatch_info(&call);

		// Half the pool's DOT side: on the constant-product curve this costs about twice the
		// rate price, so the pool must lose.
		let fee: Balance = UNITS / 2;
		assert!(pool_price_of(&asset, fee) > rate_price_of(&asset, fee) * 3 / 2);

		let before = AssetsPallet::balance(asset.clone(), &payer);
		let paid = Charger::withdraw_fee(&payer, &call, &info, asset.clone(), fee, 0)
			.expect("the rate prices the asset");
		let charged = Charger::correct_and_deposit_fee(
			&payer,
			&info,
			&<crate::RuntimeCall as sp_runtime::traits::Dispatchable>::PostInfo::default(),
			fee,
			0,
			asset.clone(),
			paid,
		)
		.unwrap();
		assert_eq!(charged, rate_price_of(&asset, fee), "the rate should have won");
		assert_eq!(AssetsPallet::balance(asset, &payer), before - charged);
	});
}

/// The XCM payment API advertises DOT plus everything the oracles can price: pooled assets and
/// rated assets, each listed once.
#[test]
fn xcm_payment_api_lists_every_payable_asset() {
	use fee_assets::*;
	use xcm_runtime_apis::fees::runtime_decl_for_xcm_payment_api::XcmPaymentApiV2;

	new_test_ext().execute_with(|| {
		let a = priced_assets();

		let acceptable = Runtime::query_acceptable_payment_assets(xcm::latest::VERSION)
			.expect("the assets are all in the latest version")
			.into_iter()
			.map(|asset| AssetId::try_from(asset).expect("latest version"))
			.collect::<alloc::vec::Vec<_>>();

		for expected in
			[&RelayLocation::get(), &a.pooled, &a.rated, &a.pool_cheaper, &a.rate_cheaper]
		{
			assert_eq!(
				acceptable.iter().filter(|id| id.0 == *expected).count(),
				1,
				"{expected:?} should be advertised exactly once, got {acceptable:?}",
			);
		}
		// Nothing prices this one, so it is not advertised.
		assert!(!acceptable.iter().any(|id| id.0 == a.unpriced));
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

/// The transaction extension pipeline is versioned: version 0 is the pipeline that predates the
/// Individuality deployment and must stay frozen so already-built signers keep working, while
/// version 1 carries the Individuality origin modifiers.
///
/// This pins both: the identifiers of version 0 in order, and the fact that version 1 exists and is
/// version 0 plus the Individuality extensions. Any reordering of version 0 breaks live signers, so
/// it should only ever change together with `transaction_version`.
#[test]
fn transaction_extension_versions_are_stable() {
	use codec::Encode;
	use sp_runtime::traits::{Pipeline, PipelineMetadataBuilder, TransactionExtension};

	// `ChargeAssetTxPayment` is served by `pallet-asset-conversion-tx-payment` rather than
	// `pallet-asset-tx-payment`, which is only allowed because the two encode identically:
	// a compact tip followed by an optional asset `Location`. Pin those bytes, because version 0
	// of the pipeline is frozen and every already-built signer targets it.
	type ChargeAssetTxPayment = pallet_asset_conversion_tx_payment::ChargeAssetTxPayment<Runtime>;
	assert_eq!(ChargeAssetTxPayment::from(0, None).encode(), alloc::vec![0x00, 0x00]);
	assert_eq!(
		ChargeAssetTxPayment::from(1, Some(Location::parent())).encode(),
		alloc::vec![0x04, 0x01, 0x01, 0x00],
	);

	let v0: Vec<&str> = <crate::TxExtensionV0 as TransactionExtension<RuntimeCall>>::metadata()
		.into_iter()
		.map(|m| m.identifier)
		.collect();
	assert_eq!(
		v0,
		vec![
			"AuthorizeCall",
			"CheckNonZeroSender",
			"CheckSpecVersion",
			"CheckTxVersion",
			"CheckGenesis",
			"CheckMortality",
			"CheckNonce",
			"CheckWeight",
			"ChargeAssetTxPayment",
			"CheckMetadataHash",
			"StorageWeightReclaim",
		],
	);

	// Version 1 must be advertised in the metadata, otherwise no wallet can construct it.
	let mut builder = PipelineMetadataBuilder::new();
	<crate::TxExtensionOtherVersions as Pipeline<RuntimeCall>>::build_metadata(&mut builder);
	let v1_indices = builder.by_version.get(&1).expect("extension version 1 must be advertised");
	let v1: Vec<&str> =
		v1_indices.iter().map(|i| builder.in_versions[*i as usize].identifier).collect();
	assert_eq!(builder.by_version.len(), 1, "only version 1 lives outside version 0");

	// Version 1 is version 0 plus the Individuality pipeline: same non-Individuality identifiers,
	// in the same relative order.
	let indiv = [
		"UnitTransactionExtension",
		"VerifyMultiSignature",
		"AsPerson",
		"ScoreAsParticipant",
		"GameAsInvited",
		"PeopleLiteAuth",
		"AsMember",
		"AsCoinage",
		"AsResources",
		"HonourAuth",
		"RestrictOrigins",
	];
	let v1_without_indiv: Vec<&str> = v1.iter().copied().filter(|id| !indiv.contains(id)).collect();
	assert_eq!(v1_without_indiv, v0, "version 1 must extend version 0, not reshuffle it");
	for id in indiv {
		assert!(v1.contains(&id), "version 1 must carry `{id}`");
	}
}

#[test]
fn individuality_storage_parameters_are_governance_mutable() {
	use crate::{
		parameters::{dynamic_params, RuntimeParameters, StatementAllowanceParameter},
		Parameters, RuntimeGenesisConfig,
	};
	use frame_support::traits::Get;
	use polkadot_runtime_constants::{
		fellowship::FELLOWS_RANK,
		system_parachain::{ASSET_HUB_ID, COLLECTIVES_ID},
		xcm::body::TECHNICAL_MAINTENANCE_INDEX,
	};
	use sp_runtime::BuildStorage;

	let mut ext = sp_io::TestExternalities::new(
		RuntimeGenesisConfig::default().build_storage().expect("runtime genesis builds"),
	);
	ext.execute_with(|| {
		assert_eq!(
			<<Runtime as indiv_pallet_resources::Config>::StmtStoreGraceWindow as Get<u32>>::get(),
			24 * 60 * 60,
		);
		assert_noop!(
			Parameters::set_parameter(
				RuntimeOrigin::signed(ALICE.into()),
				RuntimeParameters::StatementStorage(
					dynamic_params::statement_storage::Parameters::StmtStoreGraceWindow(
						dynamic_params::statement_storage::StmtStoreGraceWindow,
						Some(60 * 60),
					),
				),
			),
			sp_runtime::DispatchError::BadOrigin,
		);
		assert_ok!(Parameters::set_parameter(
			RuntimeOrigin::root(),
			RuntimeParameters::StatementStorage(
				dynamic_params::statement_storage::Parameters::StmtStoreGraceWindow(
					dynamic_params::statement_storage::StmtStoreGraceWindow,
					Some(60 * 60),
				),
			),
		));
		assert_eq!(
			<<Runtime as indiv_pallet_resources::Config>::StmtStoreGraceWindow as Get<u32>>::get(),
			60 * 60,
		);

		assert_ok!(Parameters::set_parameter(
			RuntimeOrigin::root(),
			RuntimeParameters::StatementStorage(
				dynamic_params::statement_storage::Parameters::PersonStatementLimit(
					dynamic_params::statement_storage::PersonStatementLimit,
					Some(StatementAllowanceParameter { max_size: 42, max_count: 3 }),
				),
			),
		));
		assert_eq!(
			<<Runtime as indiv_pallet_resources::Config>::PersonStatementLimit as Get<
				sp_statement_store::StatementAllowance,
			>>::get(),
			sp_statement_store::StatementAllowance { max_size: 42, max_count: 3 },
		);

		assert_ok!(Parameters::set_parameter(
			RuntimeOrigin::from(pallet_xcm::Origin::Xcm(Location::new(
				1,
				[
					Parachain(COLLECTIVES_ID),
					Plurality { id: BodyId::Technical, part: BodyPart::Voice },
					GeneralIndex(FELLOWS_RANK),
				],
			))),
			RuntimeParameters::BulletinStorage(
				dynamic_params::bulletin_storage::Parameters::LongTermStorageGraceWindow(
					dynamic_params::bulletin_storage::LongTermStorageGraceWindow,
					Some(2 * 60 * 60),
				),
			),
		));
		assert_eq!(
			<<Runtime as indiv_pallet_resources::Config>::LongTermStorageGraceWindow as Get<u32>>::get(),
			2 * 60 * 60,
		);

		assert_ok!(Parameters::set_parameter(
			RuntimeOrigin::from(pallet_xcm::Origin::Xcm(Location::new(
				1,
				[
					Parachain(ASSET_HUB_ID),
					Plurality {
						id: BodyId::Index(TECHNICAL_MAINTENANCE_INDEX),
						part: BodyPart::Voice,
					},
				],
			))),
			RuntimeParameters::StatementStorage(
				dynamic_params::statement_storage::Parameters::StmtStoreGraceWindow(
					dynamic_params::statement_storage::StmtStoreGraceWindow,
					Some(30 * 60),
				),
			),
		));
		assert_eq!(
			<<Runtime as indiv_pallet_resources::Config>::StmtStoreGraceWindow as Get<u32>>::get(),
			30 * 60,
		);

		assert_noop!(
			Parameters::set_parameter(
				RuntimeOrigin::from(pallet_xcm::Origin::Xcm(Location::new(
					1,
					[
						Parachain(ASSET_HUB_ID),
						Plurality {
							id: BodyId::Index(TECHNICAL_MAINTENANCE_INDEX + 1),
							part: BodyPart::Voice,
						},
					],
				))),
				RuntimeParameters::StatementStorage(
					dynamic_params::statement_storage::Parameters::StmtStoreGraceWindow(
						dynamic_params::statement_storage::StmtStoreGraceWindow,
						Some(15 * 60),
					),
				),
			),
			sp_runtime::DispatchError::BadOrigin,
		);

		assert_ok!(Parameters::set_parameter(
			RuntimeOrigin::root(),
			RuntimeParameters::PeopleAirdrops(
				dynamic_params::people_airdrops::Parameters::PrizeSource(
					dynamic_params::people_airdrops::PrizeSource,
					Some(sp_runtime::AccountId32::new([42u8; 32])),
				),
			),
		));
		assert_eq!(
			dynamic_params::people_airdrops::PrizeSource::get(),
			sp_runtime::AccountId32::new([42u8; 32]),
		);
		assert_ok!(Parameters::set_parameter(
			RuntimeOrigin::root(),
			RuntimeParameters::LitePersonhood(
				dynamic_params::lite_personhood::Parameters::RegistrationFee(
					dynamic_params::lite_personhood::RegistrationFee,
					Some(75 * UNITS),
				),
			),
		));
		assert_eq!(
			<<Runtime as indiv_pallet_people_lite::Config>::RegistrationFee as Get<Balance>>::get(),
			75 * UNITS,
		);
	});
}

#[test]
fn individuality_cross_runtime_pallet_indices_are_pinned() {
	use crate::MembersNotifier;
	use asset_hub_polkadot_runtime::individuality::RingRootsNotifierEndpoint;
	use frame_support::traits::PalletInfoAccess;

	assert_eq!(MembersNotifier::index(), 69);
	assert_eq!(RingRootsNotifierEndpoint::get().pallet_index, MembersNotifier::index() as u8,);
	assert_eq!(asset_hub_polkadot_runtime::MembersSubscriber::index(), 97);
	assert_eq!(PeopleAirdrops::index(), 74);
}

#[test]
fn individuality_deployment_order_guards_are_enforced() {
	use crate::{
		individuality::StableAssetLocation, Assets, ChunksManager, Coinage, RuntimeGenesisConfig,
	};
	use indiv_support::traits::RingExponent;
	use sp_runtime::{transaction_validity::TransactionValidityError, BuildStorage};

	let mut ext = sp_io::TestExternalities::new(
		RuntimeGenesisConfig::default().build_storage().expect("runtime genesis builds"),
	);
	ext.execute_with(|| {
		// `add_chunks` is authorized only after its page hash is committed.
		assert!(matches!(
			ChunksManager::authorize_add_chunks(&RingExponent::R2e9, &0, &[]),
			Err(TransactionValidityError::Invalid(
				sp_runtime::transaction_validity::InvalidTransaction::Call
			))
		));

		// Coinage refuses an unregistered backing asset. Once governance funds the pallet account's
		// minimum balance, it can create sufficient instances for that backing asset.
		let stable = StableAssetLocation::get();
		assert_noop!(
			Coinage::create_sufficient_instance(
				RuntimeOrigin::root(),
				stable.clone(),
				HOLLAR_UNITS / 100,
			),
			indiv_pallet_coinage::Error::<Runtime>::UnknownAsset,
		);
		assert_ok!(Assets::force_create(
			RuntimeOrigin::root(),
			stable.clone(),
			AccountId::from(ALICE).into(),
			true,
			1,
		));
		assert_ok!(Assets::mint(
			RuntimeOrigin::signed(AccountId::from(ALICE)),
			stable.clone(),
			MultiAddress::Id(Coinage::pallet_account()),
			1,
		));
		assert_ok!(Coinage::create_sufficient_instance(
			RuntimeOrigin::root(),
			stable.clone(),
			HOLLAR_UNITS / 100,
		));
		assert_ok!(Coinage::create_sufficient_instance(
			RuntimeOrigin::root(),
			stable.clone(),
			HOLLAR_UNITS / 100,
		));
		let mut instances = Coinage::get_instance_ids(stable);
		instances.sort();
		assert_eq!(instances, vec![0, 1]);

		// No schedule means no game state or score round is active.
		assert!(indiv_pallet_game::Game::<Runtime>::get().is_none());
		assert!(indiv_pallet_game::GameSchedules::<Runtime>::get().is_empty());
		assert!(indiv_pallet_score::RoundPlanning::<Runtime>::get().is_none());
		assert!(indiv_pallet_score::Participants::<Runtime>::iter().next().is_none());
	});
}

#[test]
fn bulletin_destination_is_governable_but_must_remain_a_sibling_parachain() {
	use crate::{
		individuality::BulletinDataStore,
		parameters::{dynamic_params, RuntimeParameters},
		Parameters, RuntimeGenesisConfig,
	};
	use sp_runtime::BuildStorage;

	let mut ext = sp_io::TestExternalities::new(
		RuntimeGenesisConfig::default().build_storage().expect("runtime genesis builds"),
	);
	ext.execute_with(|| {
		assert_ok!(Parameters::set_parameter(
			RuntimeOrigin::root(),
			RuntimeParameters::BulletinStorage(
				dynamic_params::bulletin_storage::Parameters::BulletinChainLocation(
					dynamic_params::bulletin_storage::BulletinChainLocation,
					Some(Location::parent()),
				),
			),
		));
		assert_eq!(
			BulletinDataStore::bulletin_chain_location(),
			Err(sp_runtime::DispatchError::Other(
				"Bulletin destination must be a sibling parachain"
			)),
		);
	});
}

#[test]
fn individuality_dynamic_parameter_extremes_do_not_brick_parameter_updates() {
	use crate::{
		individuality::BulletinDataStore,
		parameters::{dynamic_params, RuntimeParameters, StatementAllowanceParameter},
		ExistentialDeposit, Parameters, RuntimeGenesisConfig,
	};
	use frame_support::traits::Get;
	use indiv_pallet_resources::types::LongTermStorageAllocation;
	use sp_runtime::BuildStorage;

	macro_rules! statement_parameter {
		($name:ident, $value:expr) => {
			assert_ok!(Parameters::set_parameter(
				RuntimeOrigin::root(),
				RuntimeParameters::StatementStorage(
					dynamic_params::statement_storage::Parameters::$name(
						dynamic_params::statement_storage::$name,
						Some($value),
					),
				),
			));
		};
	}
	macro_rules! bulletin_parameter {
		($name:ident, $value:expr) => {
			assert_ok!(Parameters::set_parameter(
				RuntimeOrigin::root(),
				RuntimeParameters::BulletinStorage(
					dynamic_params::bulletin_storage::Parameters::$name(
						dynamic_params::bulletin_storage::$name,
						Some($value),
					),
				),
			));
		};
	}

	let mut ext = sp_io::TestExternalities::new(
		RuntimeGenesisConfig::default().build_storage().expect("runtime genesis builds"),
	);
	ext.execute_with(|| {
		let zero_allowance = StatementAllowanceParameter { max_size: 0, max_count: 0 };
		let max_allowance = StatementAllowanceParameter { max_size: u32::MAX, max_count: u32::MAX };
		statement_parameter!(AccountsApiAllowance, zero_allowance.clone());
		statement_parameter!(AccountsApiAllowance, max_allowance.clone());
		statement_parameter!(StmtStoreSlotsPerPeriod, 0u32);
		statement_parameter!(StmtStoreSlotsPerPeriod, u32::MAX);
		statement_parameter!(LiteStmtStoreSlotsPerPeriod, 0u32);
		statement_parameter!(LiteStmtStoreSlotsPerPeriod, u32::MAX);
		statement_parameter!(StmtStoreCleanupLimit, 0u32);
		statement_parameter!(StmtStoreCleanupLimit, u32::MAX);
		statement_parameter!(StmtStoreReplacementCooldown, 0u32);
		statement_parameter!(StmtStoreReplacementCooldown, u32::MAX);
		statement_parameter!(StmtStoreGraceWindow, 0u32);
		statement_parameter!(StmtStoreGraceWindow, u32::MAX);
		statement_parameter!(NotificationAllowance, zero_allowance.clone());
		statement_parameter!(NotificationAllowance, max_allowance.clone());
		statement_parameter!(NotificationSlotsPerPeriod, 0u8);
		statement_parameter!(NotificationSlotsPerPeriod, u8::MAX);
		statement_parameter!(LiteNotificationSlotsPerPeriod, 0u8);
		statement_parameter!(LiteNotificationSlotsPerPeriod, u8::MAX);
		statement_parameter!(NotificationPeriodDuration, 0u32);
		statement_parameter!(NotificationPeriodDuration, u32::MAX);
		statement_parameter!(LitePersonStatementLimit, zero_allowance.clone());
		statement_parameter!(LitePersonStatementLimit, max_allowance.clone());
		statement_parameter!(PersonStatementLimit, zero_allowance);
		statement_parameter!(PersonStatementLimit, max_allowance);

		let zero_allocation = LongTermStorageAllocation { transactions: 0, bytes: 0 };
		let max_allocation = LongTermStorageAllocation { transactions: u32::MAX, bytes: u64::MAX };
		bulletin_parameter!(LongTermStoragePeriodDuration, 0u32);
		bulletin_parameter!(LongTermStoragePeriodDuration, u32::MAX);
		bulletin_parameter!(LongTermStorageGraceWindow, 0u32);
		bulletin_parameter!(LongTermStorageGraceWindow, u32::MAX);
		bulletin_parameter!(LongTermStorageClaimsPerPeriod, 0u8);
		bulletin_parameter!(LongTermStorageClaimsPerPeriod, u8::MAX);
		bulletin_parameter!(LongTermStorageCleanupLimit, 0u32);
		bulletin_parameter!(LongTermStorageCleanupLimit, u32::MAX);
		bulletin_parameter!(LongTermStorageAllowanceForPeople, zero_allocation);
		bulletin_parameter!(LongTermStorageAllowanceForPeople, max_allocation);
		bulletin_parameter!(LongTermStorageAllowanceForLitePeople, zero_allocation);
		bulletin_parameter!(LongTermStorageAllowanceForLitePeople, max_allocation);
		let zero_para = Location::new(1, [Parachain(0)]);
		bulletin_parameter!(BulletinChainLocation, zero_para.clone());
		assert_eq!(BulletinDataStore::bulletin_chain_location(), Ok(zero_para));
		let max_para = Location::new(1, [Parachain(u32::MAX)]);
		bulletin_parameter!(BulletinChainLocation, max_para.clone());
		assert_eq!(BulletinDataStore::bulletin_chain_location(), Ok(max_para));
		bulletin_parameter!(BulletinTransactionStoragePalletIndex, 0u8);
		bulletin_parameter!(BulletinTransactionStoragePalletIndex, u8::MAX);

		// `pallet_parameters` accepts every SCALE-decodable value. The runtime consumes the
		// guarded aliases from `indiv_support::parameters`, so malicious or accidental extremes
		// cannot violate the resources pallet's invariants or exceed its benchmarked cleanup
		// bounds.
		statement_parameter!(StmtStoreSlotsPerPeriod, 0u32);
		assert_eq!(
			<<Runtime as indiv_pallet_resources::Config>::StmtStoreSlotsPerPeriod as Get<u32>>::get(
			),
			1,
		);
		statement_parameter!(LiteStmtStoreSlotsPerPeriod, u32::MAX);
		assert_eq!(
			<<Runtime as indiv_pallet_resources::Config>::LiteStmtStoreSlotsPerPeriod as Get<
				u32,
			>>::get(),
			1,
		);
		statement_parameter!(StmtStoreCleanupLimit, u32::MAX);
		assert_eq!(
			<<Runtime as indiv_pallet_resources::Config>::StmtStoreCleanupLimit as Get<u32>>::get(),
			50,
		);
		statement_parameter!(StmtStoreReplacementCooldown, u32::MAX);
		assert_eq!(
			<<Runtime as indiv_pallet_resources::Config>::StmtStoreReplacementCooldown as Get<
				u32,
			>>::get(),
			24 * 60 * 60,
		);
		statement_parameter!(StmtStoreGraceWindow, 0u32);
		assert_eq!(
			<<Runtime as indiv_pallet_resources::Config>::StmtStoreGraceWindow as Get<u32>>::get(),
			1,
		);
		statement_parameter!(NotificationSlotsPerPeriod, 0u8);
		statement_parameter!(LiteNotificationSlotsPerPeriod, u8::MAX);
		assert_eq!(
			<<Runtime as indiv_pallet_resources::Config>::LiteNotificationSlotsPerPeriod as Get<
				u8,
			>>::get(),
			0,
		);
		bulletin_parameter!(LongTermStoragePeriodDuration, 0u32);
		assert_eq!(
			<<Runtime as indiv_pallet_resources::Config>::LongTermStoragePeriodDuration as Get<
				u32,
			>>::get(),
			1,
		);
		bulletin_parameter!(LongTermStorageGraceWindow, u32::MAX);
		assert_eq!(
			<<Runtime as indiv_pallet_resources::Config>::LongTermStorageGraceWindow as Get<u32>>::get(),
			0,
		);
		bulletin_parameter!(LongTermStorageClaimsPerPeriod, 0u8);
		assert_eq!(
			<<Runtime as indiv_pallet_resources::Config>::LongTermStorageClaimsPerPeriod as Get<
				u8,
			>>::get(),
			1,
		);
		bulletin_parameter!(LongTermStorageCleanupLimit, u32::MAX);
		assert_eq!(
			<<Runtime as indiv_pallet_resources::Config>::LongTermStorageCleanupLimit as Get<
				u32,
			>>::get(),
			20,
		);
		assert_ok!(Parameters::set_parameter(
			RuntimeOrigin::root(),
			RuntimeParameters::PeopleAirdrops(
				dynamic_params::people_airdrops::Parameters::PrizeSource(
					dynamic_params::people_airdrops::PrizeSource,
					Some(sp_runtime::AccountId32::new([0u8; 32])),
				),
			),
		));
		assert_ok!(Parameters::set_parameter(
			RuntimeOrigin::root(),
			RuntimeParameters::LitePersonhood(
				dynamic_params::lite_personhood::Parameters::RegistrationFee(
					dynamic_params::lite_personhood::RegistrationFee,
					Some(Balance::MAX),
				),
			),
		));
		assert_ok!(Parameters::set_parameter(
			RuntimeOrigin::root(),
			RuntimeParameters::LitePersonhood(
				dynamic_params::lite_personhood::Parameters::RegistrationFee(
					dynamic_params::lite_personhood::RegistrationFee,
					Some(0),
				),
			),
		));
		assert_eq!(
			<<Runtime as indiv_pallet_people_lite::Config>::RegistrationFee as Get<Balance>>::get(),
			ExistentialDeposit::get(),
		);
	});
}
