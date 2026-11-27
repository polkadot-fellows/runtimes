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

use codec::{Decode, DecodeLimit, Encode};
use cumulus_primitives_core::{
	relay_chain::well_known_keys::ONE_EPOCH_AGO_RANDOMNESS,
	runtime_decl_for_key_to_include_in_relay_proof::KeyToIncludeInRelayProofV1, RelayStorageKey,
	XcmpMessageFormat, XcmpMessageSource,
};
use frame_support::{
	assert_ok,
	dispatch::GetDispatchInfo,
	traits::{EnsureOrigin, Get, Hooks, OnFinalize, OnInitialize, OneSessionHandler},
	weights::Weight,
};
use frame_system::{pallet_prelude::BlockNumberFor, RawOrigin};
use pallet_validator_collators::{
	CollatorDraw, DrawnCollators, ExternalValidatorSet, MaxCollators, PendingRotation,
	RotationState,
};
use parachains_runtimes_test_utils::{
	AccountIdOf, BasicParachainRuntime, ExtBuilder, RuntimeHelper, SessionKeysOf, ValidatorIdOf,
};
use sp_io::TestExternalities;
use sp_keystore::{testing::MemoryKeystore, KeystoreExt};
use sp_runtime::traits::{Block as BlockT, Dispatchable, TrailingZeroInput};
use std::collections::BTreeSet;
use xcm::{latest::prelude::*, VersionedXcm, MAX_XCM_DECODE_DEPTH};
use xcm_executor::traits::ConvertLocation;

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
		// AND each of the two blocks counted its author through `pallet-authorship`, so reports
		// can credit the blocks validators author
		assert_eq!(
			pallet_validator_collators::AuthoredBlocks::<Runtime>::iter_values().sum::<u32>(),
			2
		);
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

/// The regular session rotation keeps its period and offset while no rotation is forced, so
/// changes of invulnerables and candidates and stale-candidate kicks still take effect.
pub fn regular_rotation_keeps_its_period<Runtime>(period: u32, offset: u32)
where
	Runtime: BasicParachainRuntime + pallet_validator_collators::Config,
	ValidatorIdOf<Runtime>: From<AccountIdOf<Runtime>>,
	BlockNumberFor<Runtime>: From<u32>,
{
	type ShouldEnd<R> = <R as pallet_session::Config>::ShouldEndSession;
	use pallet_session::ShouldEndSession;

	ExtBuilder::<Runtime>::default().build().execute_with(|| {
		// GIVEN no rotation is forced
		assert_eq!(PendingRotation::<Runtime>::get(), RotationState::Idle);
		for session in 1..=3 {
			let end = offset + session * period;
			// WHEN the session period ends
			// THEN the session rotates, and not a block earlier or later
			assert!(ShouldEnd::<Runtime>::should_end_session(end.into()), "block {end}");
			assert!(
				!ShouldEnd::<Runtime>::should_end_session((end - 1).into()),
				"block {}",
				end - 1
			);
			assert!(
				!ShouldEnd::<Runtime>::should_end_session((end + 1).into()),
				"block {}",
				end + 1
			);
		}
	});
}

/// The largest header the relay chain accepts carries the Aura authorities when they change.
/// `allowance` covers what the test block lacks, the seal, the collator's digests and a large
/// block number.
pub fn aura_authorities_fit_the_relay_head_data<Runtime>(max_head_data_size: u32, allowance: u32)
where
	Runtime: BasicParachainRuntime + pallet_aura::Config,
	ValidatorIdOf<Runtime>: From<AccountIdOf<Runtime>>,
	AccountIdOf<Runtime>: From<[u8; 32]>,
{
	use codec::Encode;
	let max = <Runtime as pallet_aura::Config>::MaxAuthorities::get();
	let entry = |i: u32| {
		let mut bytes = [0u8; 32];
		bytes[..4].copy_from_slice(&i.to_le_bytes());
		let key = <Runtime as pallet_aura::Config>::AuthorityId::decode(&mut &bytes[..])
			.expect("32 bytes decode as an Aura key");
		(AccountIdOf::<Runtime>::from(bytes), key)
	};
	let collators = (0..max + 100).map(entry).collect::<Vec<_>>();

	ExtBuilder::<Runtime>::default().build().execute_with(|| {
		// GIVEN more collators than `MaxAuthorities`, as a large validator cap may draw
		// WHEN a session enacts them
		let iter = || collators.iter().map(|(who, key)| (who, key.clone()));
		<pallet_aura::Pallet<Runtime> as OneSessionHandler<AccountIdOf<Runtime>>>::on_new_session(
			true,
			iter(),
			iter(),
		);
		// THEN Aura keeps the first `MaxAuthorities`, so collator selection's collators, which
		// come first, keep collating
		let authorities = pallet_aura::Authorities::<Runtime>::get();
		assert_eq!(authorities.len() as u32, max);
		assert_eq!(authorities.first(), collators.first().map(|(_, key)| key));
		// AND the header that announces them stays under the relay chain's head-data limit
		let header = frame_system::Pallet::<Runtime>::finalize();
		let size = header.encode().len() as u32 + allowance;
		assert!(size <= max_head_data_size, "a header with {max} authorities is {size} bytes");
	});
}

/// Only Asset Hub's location as an XCM origin sets the validator set. Root, the relay chain, a
/// sibling parachain origin of Asset Hub, Asset Hub's sovereign account and each of `others` as an
/// XCM origin are rejected.
pub fn only_asset_hub_sets_the_validator_set<Runtime, LocationToAccountId>(
	asset_hub: Location,
	others: Vec<Location>,
) where
	Runtime: pallet_validator_collators::Config,
	<Runtime as frame_system::Config>::RuntimeOrigin: From<pallet_xcm::Origin>
		+ From<cumulus_pallet_xcm::Origin>
		+ From<RawOrigin<AccountIdOf<Runtime>>>,
	LocationToAccountId: ConvertLocation<AccountIdOf<Runtime>>,
{
	type SetOrigin<R> = <R as pallet_validator_collators::Config>::SetOrigin;
	type Origin<R> = <R as frame_system::Config>::RuntimeOrigin;
	let as_xcm = |location: Location| Origin::<Runtime>::from(pallet_xcm::Origin::Xcm(location));

	// GIVEN Asset Hub's location as an XCM origin
	// WHEN it sets the validator set THEN it is accepted
	assert!(SetOrigin::<Runtime>::try_origin(as_xcm(asset_hub.clone())).is_ok());

	// GIVEN root, the relay chain, one of `others`, a sibling parachain origin of Asset Hub, or
	// Asset Hub's sovereign account
	let Some(Parachain(asset_hub_id)) = asset_hub.last() else {
		panic!("Asset Hub is a sibling parachain")
	};
	let sovereign = LocationToAccountId::convert_location(&asset_hub).unwrap();
	let rejected = [Origin::<Runtime>::from(RawOrigin::Root), as_xcm(Location::parent())]
		.into_iter()
		.chain(others.into_iter().map(as_xcm))
		.chain([
			cumulus_pallet_xcm::Origin::SiblingParachain((*asset_hub_id).into()).into(),
			RawOrigin::Signed(sovereign).into(),
		]);
	for origin in rejected {
		// WHEN it sets the validator set THEN it is rejected
		assert!(SetOrigin::<Runtime>::try_origin(origin).is_err());
	}
}

/// The call Asset Hub sends to store the validator set of era 1.
fn set_validators<Runtime: pallet_validator_collators::Config>(
	validators: Vec<AccountIdOf<Runtime>>,
) -> pallet_validator_collators::Call<Runtime> {
	pallet_validator_collators::Call::set_validators {
		era: 1,
		validators: validators
			.into_iter()
			.collect::<BTreeSet<_>>()
			.try_into()
			.unwrap_or_else(|_| panic!("the set fits `MaxValidators`")),
	}
}

/// Asset Hub's unpaid `Transact` of `set_validators` passes the barrier and stores the set, while
/// another sibling's is stopped at the barrier. `rejected_by_call` passes the barrier and the call
/// rejects its origin.
pub fn asset_hub_message_sets_the_validator_set_unpaid<Runtime, XcmConfig>(
	mut ext: TestExternalities,
	asset_hub: Location,
	rejected_by_call: Location,
) where
	Runtime: pallet_validator_collators::Config,
	XcmConfig: xcm_executor::Config<RuntimeCall = <Runtime as frame_system::Config>::RuntimeCall>,
	<Runtime as frame_system::Config>::RuntimeCall: From<pallet_validator_collators::Call<Runtime>>,
	AccountIdOf<Runtime>: From<[u8; 32]>,
{
	let alice = AccountIdOf::<Runtime>::from([1u8; 32]);
	let execute = |from: Location| {
		let call: <Runtime as frame_system::Config>::RuntimeCall =
			set_validators::<Runtime>(vec![alice.clone()]).into();
		let message: Xcm<<Runtime as frame_system::Config>::RuntimeCall> = Xcm::builder_unsafe()
			.unpaid_execution(Unlimited, None)
			.transact(OriginKind::Xcm, None, call.encode())
			.build();
		let mut id = [0u8; 32];
		xcm_executor::XcmExecutor::<XcmConfig>::prepare_and_execute(
			from,
			message,
			&mut id,
			Weight::MAX,
			// No credit, so only the barrier lets the message through.
			Weight::zero(),
		)
	};
	ext.execute_with(|| {
		// GIVEN the unpaid `Transact` of `set_validators` that Asset Hub sends
		// WHEN another sibling sends it
		// THEN the barrier rejects it
		assert_eq!(
			execute(Location::new(1, Parachain(2000)))
				.ensure_complete()
				.map_err(|e| e.error),
			Err(XcmError::Barrier)
		);
		// WHEN `rejected_by_call` sends it THEN the call rejects its origin
		assert_ok!(execute(rejected_by_call).ensure_complete());
		assert!(ExternalValidatorSet::<Runtime>::get().is_none());

		// WHEN Asset Hub sends it
		// THEN it executes without fees and stores the set
		assert_ok!(execute(asset_hub).ensure_complete());
		assert_eq!(
			ExternalValidatorSet::<Runtime>::get()
				.map(|set| (set.era, set.validators.into_inner().into_iter().collect::<Vec<_>>())),
			Some((1, vec![alice.clone()]))
		);
	});
}

/// The report block sends Asset Hub the blocks of the stored set's validators, as an unpaid
/// `Transact` of `encoded_report` with this chain's native origin.
pub fn report_reaches_asset_hub_as_an_unpaid_transact<Runtime, OutboundMessages>(
	asset_hub: Location,
	encoded_report: impl Fn(&[(AccountIdOf<Runtime>, u32)]) -> Vec<u8>,
) where
	Runtime: BasicParachainRuntime + pallet_validator_collators::Config,
	OutboundMessages: XcmpMessageSource,
	<Runtime as frame_system::Config>::RuntimeEvent:
		From<pallet_validator_collators::Event<Runtime>>,
	<Runtime as frame_system::Config>::RuntimeOrigin: From<pallet_xcm::Origin>,
	ValidatorIdOf<Runtime>: From<AccountIdOf<Runtime>>,
	AccountIdOf<Runtime>: From<[u8; 32]>,
{
	type System<R> = frame_system::Pallet<R>;
	type ValidatorCollators<R> = pallet_validator_collators::Pallet<R>;
	let alice = AccountIdOf::<Runtime>::from([1u8; 32]);
	let invulnerable = AccountIdOf::<Runtime>::from([9u8; 32]);
	let Some(Parachain(asset_hub_id)) = asset_hub.last().cloned() else {
		panic!("Asset Hub is a sibling parachain")
	};
	ExtBuilder::<Runtime>::default()
		.with_safe_xcm_version(xcm::latest::VERSION)
		.build()
		.execute_with(|| {
			// GIVEN a stored set with Alice, an open channel to Asset Hub, and Alice and an
			// invulnerable outside the set authored blocks
			cumulus_pallet_parachain_system::Pallet::<Runtime>::open_outbound_hrmp_channel_for_benchmarks_or_tests(
				asset_hub_id.into(),
			);
			assert_ok!(ValidatorCollators::<Runtime>::set_validators(
				pallet_xcm::Origin::Xcm(asset_hub).into(),
				1,
				[alice.clone()]
					.into_iter()
					.collect::<BTreeSet<_>>()
					.try_into()
					.unwrap_or_else(|_| panic!("one validator fits `MaxValidators`")),
			));
			for who in [&alice, &alice, &alice, &invulnerable] {
				<ValidatorCollators<Runtime> as pallet_authorship::EventHandler<_, _>>::note_author(
					who.clone(),
				);
			}

			// WHEN the report block comes
			let period = pallet_validator_collators::ReportPeriod::<Runtime>::get();
			System::<Runtime>::reset_events();
			ValidatorCollators::<Runtime>::on_poll(
				period,
				&mut frame_support::weights::WeightMeter::new(),
			);

			// THEN only Alice's count is reported
			let reported: <Runtime as frame_system::Config>::RuntimeEvent =
				pallet_validator_collators::Event::<Runtime>::BlocksReported {
					authors: 2,
					reported: 1,
				}
				.into();
			assert!(System::<Runtime>::events().iter().any(|record| record.event == reported));
			// AND it is queued for Asset Hub as an unpaid `Transact` with this chain's native
			// origin
			let messages = OutboundMessages::take_outbound_messages(usize::MAX, &[]);
			assert_eq!(messages.len(), 1);
			let (destination, page) = &messages[0];
			assert_eq!(*destination, asset_hub_id.into());
			let mut data = &page[..];
			assert_eq!(
				XcmpMessageFormat::decode(&mut data),
				Ok(XcmpMessageFormat::ConcatenatedVersionedXcm)
			);
			let xcm: Xcm<()> =
				VersionedXcm::<()>::decode_with_depth_limit(MAX_XCM_DECODE_DEPTH, &mut data)
					.unwrap()
					.try_into()
					.unwrap();
			let expected_call = encoded_report(&[(alice.clone(), 3)]);
			assert!(matches!(
				xcm.0.as_slice(),
				[
					UnpaidExecution { weight_limit: Unlimited, check_origin: None },
					Transact { origin_kind: OriginKind::Native, call, .. },
					SetTopic(_),
				] if call.encoded() == expected_call.as_slice()
			));
		});
}

/// The largest validator set Asset Hub sends fits `service_weight`, the message queue's service
/// weight, so it is not set aside as overweight.
pub fn full_validator_set_fits_the_message_queue<Runtime>(service_weight: Weight)
where
	Runtime: pallet_validator_collators::Config,
	AccountIdOf<Runtime>: From<[u8; 32]>,
{
	// GIVEN the largest validator set Asset Hub sends
	let max = <Runtime as pallet_validator_collators::Config>::MaxValidators::get();
	let call = set_validators::<Runtime>(
		(0..max)
			.map(|i| {
				let mut account = [0u8; 32];
				account[..4].copy_from_slice(&i.to_le_bytes());
				AccountIdOf::<Runtime>::from(account)
			})
			.collect(),
	);
	// WHEN this chain processes it from its message queue
	// THEN it fits the service weight, so it is not set aside as overweight
	let weight = call.get_dispatch_info().call_weight;
	assert!(service_weight.all_gte(weight), "a set of {max} validators weighs {weight:?}");
}

/// The relay chain keys a collator proves include the epoch randomness that
/// `pallet-validator-collators` reads from every relay chain state proof.
pub fn relay_proof_request_includes_the_validator_collators_randomness_key<Runtime, Block>()
where
	Block: BlockT,
	Runtime: KeyToIncludeInRelayProofV1<Block>,
{
	// GIVEN the runtime wires pallet-validator-collators, which reads the relay chain epoch
	// randomness from every relay chain state proof
	// WHEN a collator asks the runtime which relay chain keys to prove
	let keys = Runtime::keys_to_prove().keys;
	// THEN the randomness key is requested
	assert!(keys.contains(&RelayStorageKey::Top(ONE_EPOCH_AGO_RANDOMNESS.to_vec())));
}
