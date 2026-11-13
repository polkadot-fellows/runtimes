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

//! Generic test cases shared by the system-parachain runtime tests.

use codec::Decode;
use frame_support::{
	assert_noop, assert_ok,
	traits::{
		fungible::{Inspect, InspectHold, Mutate},
		tokens::Preservation,
		Get, Hooks,
	},
	weights::Weight,
};
use frame_system::RawOrigin;
use pallet_session::{HoldReason, NextKeys};
use parachains_runtimes_test_utils::{
	AccountIdOf, BalanceOf, BasicParachainRuntime, ExtBuilder, SessionKeysOf, ValidatorIdOf,
};
use sp_keystore::{testing::MemoryKeystore, KeystoreExt};
use sp_runtime::{
	traits::{BlockNumberProvider, Dispatchable, TrailingZeroInput, Zero},
	DispatchError, TokenError,
};

/// Checks the lifecycle of the `pallet_session` key deposit.
///
/// Registering keys must lock storage cost from the caller, so that anyone can call `set_keys`
/// without it becoming free storage. Registrations made before the deposit existed (e.g. genesis
/// collators) hold nothing and must keep working.
///
/// `generate_keys` returns fresh session keys and the proof that `owner` controls them.
pub fn session_key_deposit_works<Runtime>(
	generate_keys: impl Fn(&AccountIdOf<Runtime>) -> (SessionKeysOf<Runtime>, Vec<u8>),
) where
	Runtime:
		BasicParachainRuntime + pallet_session::Config<Currency = pallet_balances::Pallet<Runtime>>,
	<Runtime as pallet_balances::Config>::RuntimeHoldReason: From<HoldReason>,
	<Runtime as frame_system::Config>::RuntimeCall: From<pallet_session::Call<Runtime>>,
	AccountIdOf<Runtime>: From<[u8; 32]>,
	ValidatorIdOf<Runtime>: From<AccountIdOf<Runtime>>,
{
	type Balances<R> = pallet_balances::Pallet<R>;
	type System<R> = frame_system::Pallet<R>;

	let deposit: BalanceOf<Runtime> = <Runtime as pallet_session::Config>::KeyDeposit::get();
	let ed = <Balances<Runtime> as Inspect<_>>::minimum_balance();
	assert!(!deposit.is_zero(), "`set_keys` must not be deposit-free");

	let genesis_collator: AccountIdOf<Runtime> = [1u8; 32].into();
	let alice: AccountIdOf<Runtime> = [2u8; 32].into();
	let bob: AccountIdOf<Runtime> = [3u8; 32].into();
	// Genesis keys are not checked for ownership, so any bytes will do.
	let genesis_keys = SessionKeysOf::<Runtime>::decode(&mut TrailingZeroInput::new(&[1u8; 32]))
		.expect("keys decode from zero-padded input");

	let mut ext = ExtBuilder::<Runtime>::default()
		.with_balances(vec![
			(genesis_collator.clone(), ed + deposit),
			(alice.clone(), ed + deposit),
			(bob.clone(), ed + deposit - 1u32.into()),
		])
		.with_collators(vec![genesis_collator.clone()])
		.with_session_keys(vec![(
			genesis_collator.clone(),
			genesis_collator.clone().into(),
			genesis_keys,
		)])
		.build();
	ext.register_extension(KeystoreExt::new(MemoryKeystore::new()));

	ext.execute_with(|| {
		let held = |who: &AccountIdOf<Runtime>| {
			Balances::<Runtime>::balance_on_hold(&HoldReason::Keys.into(), who)
		};
		let free = |who: &AccountIdOf<Runtime>| Balances::<Runtime>::balance(who);
		let has_keys = |who: &AccountIdOf<Runtime>| {
			NextKeys::<Runtime>::contains_key(ValidatorIdOf::<Runtime>::from(who.clone()))
		};
		let dispatch = |who: &AccountIdOf<Runtime>, call: pallet_session::Call<Runtime>| {
			<Runtime as frame_system::Config>::RuntimeCall::from(call)
				.dispatch(RawOrigin::Signed(who.clone()).into())
				.map(|_| ())
				.map_err(|e| e.error)
		};
		let set_keys = |who: &AccountIdOf<Runtime>| -> Result<(), DispatchError> {
			let (keys, proof) = generate_keys(who);
			dispatch(who, pallet_session::Call::set_keys { keys, proof })
		};
		let purge_keys =
			|who: &AccountIdOf<Runtime>| dispatch(who, pallet_session::Call::purge_keys {});

		// GIVEN an account with exactly ED + deposit free and no keys
		let consumers = System::<Runtime>::consumers(&alice);
		// WHEN it registers keys for the first time
		assert_ok!(set_keys(&alice));
		// THEN exactly the deposit is held, leaving the account at ED
		assert!(has_keys(&alice));
		assert_eq!(held(&alice), deposit);
		assert_eq!(free(&alice), ed);

		// WHEN it rotates its keys
		assert_ok!(set_keys(&alice));
		// THEN nothing more is charged: the storage footprint does not grow
		assert_eq!(held(&alice), deposit);
		assert_eq!(free(&alice), ed);

		// WHEN it purges its keys
		assert_ok!(purge_keys(&alice));
		// THEN the full deposit is returned and no reference is leaked
		assert!(!has_keys(&alice));
		assert!(held(&alice).is_zero());
		assert_eq!(free(&alice), ed + deposit);
		assert_eq!(System::<Runtime>::consumers(&alice), consumers);

		// GIVEN an account one unit short of ED + deposit
		// WHEN it registers keys
		// THEN it fails and writes nothing, since the deposit must not reap the account
		assert_noop!(set_keys(&bob), TokenError::FundsUnavailable);

		// GIVEN a collator whose keys were set at genesis, without a deposit
		let consumers = System::<Runtime>::consumers(&genesis_collator);
		assert!(has_keys(&genesis_collator));
		assert!(held(&genesis_collator).is_zero());
		// WHEN it rotates its keys
		assert_ok!(set_keys(&genesis_collator));
		// THEN no deposit is taken, so existing collators keep working unchanged
		assert!(held(&genesis_collator).is_zero());
		assert_eq!(free(&genesis_collator), ed + deposit);
		assert_eq!(System::<Runtime>::consumers(&genesis_collator), consumers);
		// WHEN it purges its keys
		assert_ok!(purge_keys(&genesis_collator));
		// THEN it succeeds and releases the genesis consumer reference
		assert!(!has_keys(&genesis_collator));
		assert_eq!(free(&genesis_collator), ed + deposit);
		assert_eq!(System::<Runtime>::consumers(&genesis_collator), consumers - 1);
	});
}

/// Dust accumulates for the forward to Asset Hub instead of being burned here, where the burn would
/// not show in the network total that Asset Hub tracks.
pub fn dust_accumulates_instead_of_being_burned<Runtime>()
where
	Runtime: pallet_balances::Config + pallet_accumulate_and_forward::Config,
	AccountIdOf<Runtime>: From<[u8; 32]>,
{
	type Balances<Runtime> = pallet_balances::Pallet<Runtime>;

	// Given: a funded accumulation account and an account holding exactly the ED.
	let ed = <Runtime as pallet_balances::Config>::ExistentialDeposit::get();
	let accumulation_account =
		pallet_accumulate_and_forward::Pallet::<Runtime>::accumulation_account();
	assert_ok!(Balances::<Runtime>::mint_into(&accumulation_account, ed));
	let alice = AccountIdOf::<Runtime>::from([1u8; 32]);
	let bob = AccountIdOf::<Runtime>::from([2u8; 32]);
	assert_ok!(Balances::<Runtime>::mint_into(&alice, ed));
	assert_ok!(Balances::<Runtime>::mint_into(&bob, ed));
	let issuance_before = Balances::<Runtime>::total_issuance();
	let accumulated_before = Balances::<Runtime>::balance(&accumulation_account);

	// When: Alice is reaped, leaving dust behind.
	let dust = ed / 2u32.into();
	assert_ok!(<Balances<Runtime> as Mutate<_>>::transfer(
		&alice,
		&bob,
		ed - dust,
		Preservation::Expendable,
	));

	// Then: the dust reaches the accumulation account and nothing is burned.
	assert_eq!(Balances::<Runtime>::balance(&alice), Zero::zero());
	assert_eq!(Balances::<Runtime>::balance(&accumulation_account), accumulated_before + dust);
	assert_eq!(Balances::<Runtime>::total_issuance(), issuance_before, "the dust is not burned");
}

/// The forward period is counted in relay chain blocks, whatever the local block number is.
pub fn forward_period_counts_relay_blocks<Runtime>()
where
	Runtime: pallet_accumulate_and_forward::Config + cumulus_pallet_parachain_system::Config,
	<Runtime as pallet_accumulate_and_forward::Config>::BlockNumberProvider:
		BlockNumberProvider<BlockNumber = u32>,
	<Runtime as pallet_accumulate_and_forward::Config>::Currency: Mutate<AccountIdOf<Runtime>>,
{
	type Relay<Runtime> = cumulus_pallet_parachain_system::RelaychainDataProvider<Runtime>;
	type Currency<Runtime> = <Runtime as pallet_accumulate_and_forward::Config>::Currency;
	type AccumulateForward<Runtime> = pallet_accumulate_and_forward::Pallet<Runtime>;
	let last_forward = pallet_accumulate_and_forward::LastForwardBlock::<Runtime>::get;
	let on_idle = || {
		<AccumulateForward<Runtime> as Hooks<_>>::on_idle(
			frame_system::Pallet::<Runtime>::block_number(),
			Weight::MAX,
		)
	};
	let fund = || {
		assert_ok!(Currency::<Runtime>::mint_into(
			&AccumulateForward::<Runtime>::accumulation_account(),
			Currency::<Runtime>::minimum_balance() +
				<Runtime as pallet_accumulate_and_forward::Config>::MinTransferAmount::get(),
		));
	};
	let period = <Runtime as pallet_accumulate_and_forward::Config>::TransferPeriod::get();
	let start = 10 * period;

	// Given: a funded accumulation account, and a local block number that differs from the relay
	// one and never moves.
	fund();
	frame_system::Pallet::<Runtime>::set_block_number(1u32.into());

	// When: `on_idle` runs at relay block `start`.
	Relay::<Runtime>::set_block_number(start);
	on_idle();
	// Then: a forward is attempted and recorded at the relay block.
	assert_eq!(last_forward(), Some(start));

	// When: the relay is one block short of a full period later.
	fund();
	Relay::<Runtime>::set_block_number(start + period - 1);
	on_idle();
	// Then: no forward is attempted.
	assert_eq!(last_forward(), Some(start));

	// When: a full period of relay blocks has passed.
	Relay::<Runtime>::set_block_number(start + period);
	on_idle();
	// Then: the next forward is attempted.
	assert_eq!(last_forward(), Some(start + period));
}
