// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot. If not, see <http://www.gnu.org/licenses/>.

//! Tests for the Kusama Runtime Configuration

use crate::*;
use frame_support::{
	dispatch::GetDispatchInfo, traits::WhitelistedStorageKeys, weights::WeightToFee as WeightToFeeT,
};
use polkadot_runtime_common::MinimumMultiplier;
use separator::Separatable;
use sp_core::hexdisplay::HexDisplay;
use sp_keyring::Sr25519Keyring::Charlie;
use std::collections::HashSet;

#[test]
fn remove_keys_weight_is_sensible() {
	use polkadot_runtime_common::crowdloan::WeightInfo;
	let max_weight = <Runtime as crowdloan::Config>::WeightInfo::refund(RemoveKeysLimit::get());
	// Max remove keys limit should be no more than half the total block weight.
	assert!((max_weight * 2).all_lt(BlockWeights::get().max_block));
}

#[test]
fn sample_size_is_sensible() {
	use polkadot_runtime_common::auctions::WeightInfo;
	// Need to clean up all samples at the end of an auction.
	let samples: BlockNumber = EndingPeriod::get() / SampleLength::get();
	let max_weight: Weight = RocksDbWeight::get().reads_writes(samples.into(), samples.into());
	// Max sample cleanup should be no more than half the total block weight.
	assert!((max_weight * 2).all_lt(BlockWeights::get().max_block));
	assert!((<Runtime as auctions::Config>::WeightInfo::on_initialize() * 2)
		.all_lt(BlockWeights::get().max_block));
}

#[test]
fn payout_weight_portion() {
	use pallet_staking::WeightInfo;
	let payout_weight =
		<Runtime as pallet_staking::Config>::WeightInfo::payout_stakers_alive_staked(
			MaxNominators::get(),
		)
		.ref_time() as f64;
	let block_weight = BlockWeights::get().max_block.ref_time() as f64;

	println!(
		"a full payout takes {:.2} of the block weight [{} / {}]",
		payout_weight / block_weight,
		payout_weight,
		block_weight
	);
	assert!(payout_weight * 2f64 < block_weight);
}

#[test]
#[ignore]
fn block_cost() {
	let max_block_weight = BlockWeights::get().max_block;
	let raw_fee = WeightToFee::weight_to_fee(&max_block_weight);

	println!(
		"Full Block weight == {} // WeightToFee(full_block) == {} plank",
		max_block_weight,
		raw_fee.separated_string(),
	);
}

#[test]
#[ignore]
fn transfer_cost_min_multiplier() {
	let min_multiplier = MinimumMultiplier::get();
	let call = pallet_balances::Call::<Runtime>::transfer_keep_alive {
		dest: Charlie.to_account_id().into(),
		value: Default::default(),
	};
	let info = call.get_dispatch_info();
	// convert to outer call.
	let call = RuntimeCall::Balances(call);
	let len = call.using_encoded(|e| e.len()) as u32;

	let mut ext = sp_io::TestExternalities::new_empty();
	let mut test_with_multiplier = |m| {
		ext.execute_with(|| {
			pallet_transaction_payment::NextFeeMultiplier::<Runtime>::put(m);
			let fee = TransactionPayment::compute_fee(len, &info, 0);
			println!(
				"extension_weight = {:?} // call_weight = {:?} // multiplier = {:?} // full transfer fee = {:?}",
				info.extension_weight.ref_time().separated_string(),
				info.call_weight.ref_time().separated_string(),
				pallet_transaction_payment::NextFeeMultiplier::<Runtime>::get(),
				fee.separated_string(),
			);
		});
	};

	test_with_multiplier(min_multiplier);
	test_with_multiplier(Multiplier::saturating_from_rational(1, 1u128));
	test_with_multiplier(Multiplier::saturating_from_rational(1, 1_000u128));
	test_with_multiplier(Multiplier::saturating_from_rational(1, 1_000_000u128));
	test_with_multiplier(Multiplier::saturating_from_rational(1, 1_000_000_000u128));
}

#[test]
fn nominator_limit() {
	use pallet_election_provider_multi_phase::WeightInfo;
	// starting point of the nominators.
	let all_voters: u32 = 10_000;

	// assuming we want around 5k candidates and 1k active validators.
	let all_targets: u32 = 5_000;
	let desired: u32 = 1_000;
	let weight_with = |active| {
		<Runtime as pallet_election_provider_multi_phase::Config>::WeightInfo::submit_unsigned(
			all_voters.max(active),
			all_targets,
			active,
			desired,
		)
	};

	let mut active = 1;
	while weight_with(active).all_lte(OffchainSolutionWeightLimit::get()) || active == all_voters {
		active += 1;
	}

	println!("can support {} nominators to yield a weight of {}", active, weight_with(active));
}

#[test]
fn call_size() {
	RuntimeCall::assert_size_under(256);
}

#[test]
fn check_whitelist() {
	let whitelist: HashSet<String> = AllPalletsWithSystem::whitelisted_storage_keys()
		.iter()
		.map(|e| HexDisplay::from(&e.key).to_string())
		.collect();

	// Block number
	assert!(whitelist.contains("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac"));
	// Total issuance
	assert!(whitelist.contains("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80"));
	// Execution phase
	assert!(whitelist.contains("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a"));
	// Event count
	assert!(whitelist.contains("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850"));
	// System events
	assert!(whitelist.contains("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7"));
	// Configuration ActiveConfig
	assert!(whitelist.contains("06de3d8a54d27e44a9d5ce189618f22db4b49d95320d9021994c850f25b8e385"));
	// XcmPallet VersionDiscoveryQueue
	assert!(whitelist.contains("1405f2411d0af5a7ff397e7c9dc68d194a222ba0333561192e474c59ed8e30e1"));
	// XcmPallet SafeXcmVersion
	assert!(whitelist.contains("1405f2411d0af5a7ff397e7c9dc68d196323ae84c43568be0d1394d5d0d522c4"));
}

#[test]
fn check_treasury_pallet_id() {
	assert_eq!(
		<Treasury as frame_support::traits::PalletInfoAccess>::index() as u8,
		kusama_runtime_constants::TREASURY_PALLET_ID
	);
}

#[test]
fn staking_operator_proxy_filter_works() {
	use frame_support::traits::InstanceFilter;

	let proxy = TransparentProxyType(ProxyType::StakingOperator);

	// StakingOperator ALLOWS these calls on relay chain:
	// - Session::set_keys
	let keys = SessionKeys {
		grandpa: GrandpaId::from(sp_core::ed25519::Public::from_raw([0u8; 32])),
		babe: pallet_babe::AuthorityId::from(sp_core::sr25519::Public::from_raw([0u8; 32])),
		para_validator: ValidatorId::from(sp_core::sr25519::Public::from_raw([0u8; 32])),
		para_assignment: polkadot_primitives::AssignmentId::from(
			sp_core::sr25519::Public::from_raw([0u8; 32]),
		),
		authority_discovery: AuthorityDiscoveryId::from(sp_core::sr25519::Public::from_raw(
			[0u8; 32],
		)),
		beefy: BeefyId::from(sp_core::ecdsa::Public::from_raw([0u8; 33])),
	};
	assert!(
		proxy.filter(&RuntimeCall::Session(pallet_session::Call::set_keys { keys, proof: vec![] }))
	);

	// - Session::purge_keys
	assert!(proxy.filter(&RuntimeCall::Session(pallet_session::Call::purge_keys {})));

	// - Utility calls (for batching)
	assert!(proxy.filter(&RuntimeCall::Utility(pallet_utility::Call::batch { calls: vec![] })));

	// StakingOperator DISALLOWS staking operations (those are on Asset Hub after AHM):
	// - Staking calls
	assert!(!proxy.filter(&RuntimeCall::Staking(pallet_staking::Call::bond {
		value: 1000,
		payee: pallet_staking::RewardDestination::Stash
	})));
	assert!(
		!proxy.filter(&RuntimeCall::Staking(pallet_staking::Call::nominate { targets: vec![] }))
	);
	assert!(!proxy.filter(&RuntimeCall::Staking(pallet_staking::Call::validate {
		prefs: pallet_staking::ValidatorPrefs::default()
	})));
	assert!(!proxy.filter(&RuntimeCall::Staking(pallet_staking::Call::chill {})));

	// - NominationPools calls
	assert!(!proxy.filter(&RuntimeCall::NominationPools(pallet_nomination_pools::Call::join {
		amount: 1000,
		pool_id: 1
	})));

	// - VoterList calls
	assert!(!proxy.filter(&RuntimeCall::VoterList(pallet_bags_list::Call::rebag {
		dislocated: sp_runtime::MultiAddress::Id(AccountId::from([0u8; 32])),
	})));

	// Verify is_superset relationship
	let staking_proxy = TransparentProxyType(ProxyType::Staking);
	assert!(staking_proxy.is_superset(&proxy));
	assert!(TransparentProxyType(ProxyType::NonTransfer).is_superset(&proxy));

	// Staking proxy can add/remove StakingOperator proxies
	let delegate = sp_runtime::MultiAddress::Id(AccountId::from([1u8; 32]));
	assert!(staking_proxy.filter(&RuntimeCall::Proxy(pallet_proxy::Call::add_proxy {
		delegate: delegate.clone(),
		proxy_type: TransparentProxyType(ProxyType::StakingOperator),
		delay: 0,
	})));
	assert!(staking_proxy.filter(&RuntimeCall::Proxy(pallet_proxy::Call::remove_proxy {
		delegate: delegate.clone(),
		proxy_type: TransparentProxyType(ProxyType::StakingOperator),
		delay: 0,
	})));

	// But Staking proxy cannot add/remove other proxy types
	assert!(!staking_proxy.filter(&RuntimeCall::Proxy(pallet_proxy::Call::add_proxy {
		delegate,
		proxy_type: TransparentProxyType(ProxyType::Any),
		delay: 0,
	})));
}

/// A Staking proxy can add/remove a StakingOperator proxy for the account it is proxying.
#[test]
fn staking_proxy_can_manage_staking_operator() {
	use frame_support::assert_ok;
	use sp_runtime::traits::StaticLookup;

	// Given: Build storage with balances for test accounts
	let mut t = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();

	let alice: AccountId = [1u8; 32].into();
	let bob: AccountId = [2u8; 32].into();
	let carol: AccountId = [3u8; 32].into();

	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![
			(alice.clone(), 100 * UNITS),
			(bob.clone(), 100 * UNITS),
			(carol.clone(), 100 * UNITS),
		],
		dev_accounts: None,
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| {
		// Given: Alice has Bob as her Staking proxy
		assert_ok!(Proxy::add_proxy(
			RuntimeOrigin::signed(alice.clone()),
			<Runtime as frame_system::Config>::Lookup::unlookup(bob.clone()),
			TransparentProxyType(ProxyType::Staking),
			0
		));

		// When: Bob (via proxy) adds Carol as StakingOperator for Alice
		let add_call = RuntimeCall::Proxy(pallet_proxy::Call::add_proxy {
			delegate: <Runtime as frame_system::Config>::Lookup::unlookup(carol.clone()),
			proxy_type: TransparentProxyType(ProxyType::StakingOperator),
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
			alice_proxies.0.iter().any(|p| p.delegate == carol &&
				p.proxy_type == TransparentProxyType(ProxyType::StakingOperator)),
			"Carol should be Alice's StakingOperator proxy"
		);

		// When: Bob tries to add an Any proxy for Alice
		let add_any_call = RuntimeCall::Proxy(pallet_proxy::Call::add_proxy {
			delegate: <Runtime as frame_system::Config>::Lookup::unlookup(carol.clone()),
			proxy_type: TransparentProxyType(ProxyType::Any),
			delay: 0,
		});
		// Note: proxy() returns Ok(()) even when inner call fails (result is in event)
		let _ = Proxy::proxy(
			RuntimeOrigin::signed(bob.clone()),
			<Runtime as frame_system::Config>::Lookup::unlookup(alice.clone()),
			None,
			Box::new(add_any_call),
		);

		// Then: Carol was NOT added as Any proxy (filter rejected it)
		let alice_proxies = pallet_proxy::Proxies::<Runtime>::get(&alice);
		assert!(
			!alice_proxies.0.iter().any(
				|p| p.delegate == carol && p.proxy_type == TransparentProxyType(ProxyType::Any)
			),
			"Carol should NOT be Alice's Any proxy - Staking proxy cannot add Any"
		);

		// When: Bob (via proxy) removes Carol as StakingOperator for Alice
		let remove_call = RuntimeCall::Proxy(pallet_proxy::Call::remove_proxy {
			delegate: <Runtime as frame_system::Config>::Lookup::unlookup(carol.clone()),
			proxy_type: TransparentProxyType(ProxyType::StakingOperator),
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
			!alice_proxies.0.iter().any(|p| p.delegate == carol &&
				p.proxy_type == TransparentProxyType(ProxyType::StakingOperator)),
			"Carol should no longer be Alice's StakingOperator proxy"
		);
	});
}
