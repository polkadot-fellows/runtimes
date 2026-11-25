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

//! Test cases for the chains where validators collate through `pallet-validator-collators`.

use codec::Decode;
use frame_support::{
	assert_ok,
	traits::{Get, OnFinalize, OnInitialize},
};
use frame_system::{pallet_prelude::BlockNumberFor, RawOrigin};
use pallet_validator_collators::{
	CollatorDraw, DrawnCollators, ExternalValidatorSet, MaxCollators, PendingRotation,
	RotationState,
};
use parachains_runtimes_test_utils::{
	AccountIdOf, BasicParachainRuntime, ExtBuilder, RuntimeHelper, SessionKeysOf, ValidatorIdOf,
};
use sp_keystore::{testing::MemoryKeystore, KeystoreExt};
use sp_runtime::traits::{Dispatchable, TrailingZeroInput};

/// Validators that register keys do not collate until governance raises the cap, and then
/// collate without a runtime upgrade.
///
/// `generate_keys` returns fresh session keys and the proof that the account controls them.
/// `deliver` hands the validator set of an era to the chain the way it arrives there.
pub fn validators_collate_only_after_governance_raises_the_cap<Runtime, AllPalletsWithoutSystem>(
	generate_keys: impl Fn(&AccountIdOf<Runtime>) -> (SessionKeysOf<Runtime>, Vec<u8>),
	deliver: impl Fn(u32, Vec<AccountIdOf<Runtime>>),
) where
	Runtime: BasicParachainRuntime
		+ pallet_validator_collators::Config
		+ pallet_session::Config<Currency = pallet_balances::Pallet<Runtime>>,
	<Runtime as frame_system::Config>::RuntimeCall:
		From<pallet_session::Call<Runtime>> + From<pallet_validator_collators::Call<Runtime>>,
	AccountIdOf<Runtime>: From<[u8; 32]>,
	ValidatorIdOf<Runtime>: From<AccountIdOf<Runtime>>,
	AllPalletsWithoutSystem:
		OnInitialize<BlockNumberFor<Runtime>> + OnFinalize<BlockNumberFor<Runtime>>,
{
	type Session<R> = pallet_session::Pallet<R>;
	type System<R> = frame_system::Pallet<R>;
	type Helper<R, P> = RuntimeHelper<R, P>;

	let invulnerables: Vec<AccountIdOf<Runtime>> = vec![[4u8; 32].into(), [1u8; 32].into()];
	let with_keys: AccountIdOf<Runtime> = [2u8; 32].into();
	let without_keys: AccountIdOf<Runtime> = [3u8; 32].into();
	let ed = <Runtime as pallet_balances::Config>::ExistentialDeposit::get();
	let funds = <Runtime as pallet_session::Config>::KeyDeposit::get() + ed + ed;
	// Genesis keys are not checked for ownership, so any distinct bytes will do.

	let mut ext = ExtBuilder::<Runtime>::default()
		.with_balances(vec![(with_keys.clone(), funds)])
		.with_collators(invulnerables.clone())
		.with_session_keys(
			invulnerables
				.iter()
				.zip([1u8, 4u8])
				.map(|(who, seed)| {
					let keys =
						SessionKeysOf::<Runtime>::decode(&mut TrailingZeroInput::new(&[seed; 32]))
							.expect("keys decode from zero-padded input");
					(who.clone(), who.clone().into(), keys)
				})
				.collect(),
		)
		.build();
	ext.register_extension(KeystoreExt::new(MemoryKeystore::new()));

	ext.execute_with(|| {
		let dispatch = |origin: RawOrigin<AccountIdOf<Runtime>>,
		                call: <Runtime as frame_system::Config>::RuntimeCall| {
			call.dispatch(origin.into()).map(|_| ()).map_err(|e| e.error)
		};
		let (keys, proof) = generate_keys(&with_keys);
		assert_ok!(dispatch(
			RawOrigin::Signed(with_keys.clone()),
			pallet_session::Call::<Runtime>::set_keys { keys, proof }.into()
		));
		let next_two_blocks = || {
			let now: u32 = System::<Runtime>::block_number()
				.try_into()
				.unwrap_or_else(|_| panic!("the block number fits a u32"));
			Helper::<Runtime, AllPalletsWithoutSystem>::run_to_block(
				now + 2,
				invulnerables[0].clone(),
			);
		};

		let collator_selection = || {
			pallet_collator_selection::Invulnerables::<Runtime>::get()
				.into_iter()
				.map(ValidatorIdOf::<Runtime>::from)
				.collect::<Vec<_>>()
		};

		// GIVEN no stored cap, two invulnerables, and a validator with registered keys
		assert!(!MaxCollators::<Runtime>::exists());
		assert_eq!(MaxCollators::<Runtime>::get(), CollatorDraw::None);
		let session_before = Session::<Runtime>::current_index();
		assert_eq!(Session::<Runtime>::validators(), collator_selection());

		// WHEN the validator set of era 1 arrives
		deliver(1, vec![with_keys.clone(), without_keys.clone()]);
		assert_eq!(ExternalValidatorSet::<Runtime>::get().map(|set| set.era), Some(1));
		next_two_blocks();

		// THEN the two forced rotations draw no validator, and the collator-selection list collates
		// unchanged, in its order
		assert_eq!(Session::<Runtime>::current_index(), session_before + 2);
		assert_eq!(PendingRotation::<Runtime>::get(), RotationState::Idle);
		assert_eq!(
			DrawnCollators::<Runtime>::get().map(|drawn| (drawn.era, drawn.collators.len())),
			Some((1, 0))
		);
		assert_eq!(Session::<Runtime>::validators(), collator_selection());

		// WHEN governance removes the cap
		assert_ok!(dispatch(
			RawOrigin::Root,
			pallet_validator_collators::Call::<Runtime>::set_max_collators {
				max: CollatorDraw::All
			}
			.into()
		));
		next_two_blocks();

		// THEN the validator with keys collates after the collator-selection list
		assert_eq!(Session::<Runtime>::current_index(), session_before + 4);
		let mut expected = collator_selection();
		expected.push(with_keys.clone().into());
		assert_eq!(Session::<Runtime>::validators(), expected);
	});
}

/// Collator selection's kick threshold is one session while no validator may collate, and
/// otherwise covers two Aura rounds of the capped validators, the invulnerables and the
/// candidates.
///
/// Each case is a cap and the expected threshold.
pub fn kick_threshold_follows_the_cap<Runtime>(
	session_length: BlockNumberFor<Runtime>,
	cases: &[(CollatorDraw, BlockNumberFor<Runtime>)],
) where
	Runtime: BasicParachainRuntime + pallet_validator_collators::Config,
	ValidatorIdOf<Runtime>: From<AccountIdOf<Runtime>>,
{
	let threshold = || <Runtime as pallet_collator_selection::Config>::KickThreshold::get();

	ExtBuilder::<Runtime>::default().build().execute_with(|| {
		// GIVEN no validator may collate
		// WHEN a candidate waits for its slot
		// THEN it is kicked as stale after one session, as before validators could collate
		assert_eq!(MaxCollators::<Runtime>::get(), CollatorDraw::None);
		assert_eq!(threshold(), session_length);

		for (cap, expected) in cases {
			// GIVEN governance sets a cap
			MaxCollators::<Runtime>::put(*cap);
			// THEN the candidate gets two Aura rounds of that many validators, at most
			// `MaxValidators`, plus the invulnerables and candidates, rounded up to whole sessions
			assert_eq!(threshold(), *expected, "cap {cap:?}");
		}
	});
}
