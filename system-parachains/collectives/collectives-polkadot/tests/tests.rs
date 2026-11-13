use codec::Encode;
use collectives_polkadot_runtime::{
	ambassador::{AmbassadorReferendaInstance, AmbassadorTreasuryInstance},
	fellowship::{FellowshipReferendaInstance, FellowshipTreasuryInstance},
	xcm_config::{AssetHubLocation, RelayChainLocation},
	AccumulateForward, Balances, ExistentialDeposit, ProxyType, Runtime, RuntimeCall,
	RuntimeOrigin, SessionKeys,
};
use frame_support::{
	assert_err, assert_ok,
	traits::{
		fungible::{Inspect, Mutate},
		InstanceFilter, OnUnbalanced,
	},
};
use parachains_common::{AccountId, Balance};
use parachains_runtimes_test_utils::GovernanceOrigin;
use sp_runtime::{AccountId32, Either};
use xcm::prelude::*;

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
		Either::Right(InstructionError { index: 0, error: XcmError::Barrier })
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

#[test]
fn session_key_deposit_works() {
	system_parachains_test_utils::session_key_deposit_works::<Runtime>(|owner| {
		let generated = SessionKeys::generate(&owner.encode(), None);
		(generated.keys, generated.proof.encode())
	});
}

const PROXY_TYPES: [ProxyType; 8] = [
	ProxyType::Any,
	ProxyType::NonTransfer,
	ProxyType::CancelProxy,
	ProxyType::Collator,
	ProxyType::Alliance,
	ProxyType::Fellowship,
	ProxyType::Ambassador,
	ProxyType::Secretary,
];

/// `payout_other` pays the claimant's salary to an arbitrary account, so only `Any` may call it.
#[test]
fn only_any_proxy_can_redirect_salary_payouts() {
	let beneficiary = AccountId32::new([0u8; 32]);
	let payout_other = [
		RuntimeCall::FellowshipSalary(pallet_salary::Call::payout_other {
			beneficiary: beneficiary.clone(),
		}),
		RuntimeCall::AmbassadorSalary(pallet_salary::Call::payout_other {
			beneficiary: beneficiary.clone(),
		}),
		RuntimeCall::SecretarySalary(pallet_salary::Call::payout_other { beneficiary }),
	];

	for proxy_type in PROXY_TYPES {
		for call in &payout_other {
			assert_eq!(
				proxy_type.filter(call),
				proxy_type == ProxyType::Any,
				"{proxy_type:?} filter is wrong for {call:?}"
			);
		}
	}

	// Claiming the salary to the claimant itself is still possible.
	let fellowship_payout = RuntimeCall::FellowshipSalary(pallet_salary::Call::payout {});
	assert!(ProxyType::NonTransfer.filter(&fellowship_payout));
	assert!(ProxyType::Fellowship.filter(&fellowship_payout));
	let ambassador_payout = RuntimeCall::AmbassadorSalary(pallet_salary::Call::payout {});
	assert!(ProxyType::NonTransfer.filter(&ambassador_payout));
	assert!(ProxyType::Ambassador.filter(&ambassador_payout));
	assert!(
		ProxyType::Secretary.filter(&RuntimeCall::SecretarySalary(pallet_salary::Call::payout {}))
	);
}

/// Every `is_superset` edge must be backed by the call filters, otherwise a proxy can escalate
/// itself by adding a "subset" proxy that allows more calls.
#[test]
fn proxy_type_superset_relation_matches_call_filters() {
	let who = AccountId32::new([0u8; 32]);
	let remark = RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
	let calls = vec![
		remark.clone(),
		RuntimeCall::Balances(pallet_balances::Call::transfer_keep_alive {
			dest: who.clone().into(),
			value: 1,
		}),
		RuntimeCall::CollatorSelection(pallet_collator_selection::Call::leave_intent {}),
		RuntimeCall::Session(pallet_session::Call::purge_keys {}),
		RuntimeCall::Utility(pallet_utility::Call::batch { calls: vec![] }),
		RuntimeCall::Multisig(pallet_multisig::Call::as_multi_threshold_1 {
			other_signatories: vec![],
			call: Box::new(remark),
		}),
		RuntimeCall::Proxy(pallet_proxy::Call::reject_announcement {
			delegate: who.clone().into(),
			call_hash: Default::default(),
		}),
		RuntimeCall::Alliance(pallet_alliance::Call::retire {}),
		RuntimeCall::FellowshipCollective(pallet_ranked_collective::Call::cleanup_poll {
			poll_index: 0,
			max: 0,
		}),
		RuntimeCall::FellowshipSalary(pallet_salary::Call::payout {}),
		RuntimeCall::FellowshipSalary(pallet_salary::Call::payout_other {
			beneficiary: who.clone(),
		}),
		RuntimeCall::AmbassadorCollective(pallet_ranked_collective::Call::cleanup_poll {
			poll_index: 0,
			max: 0,
		}),
		RuntimeCall::AmbassadorSalary(pallet_salary::Call::payout {}),
		RuntimeCall::AmbassadorSalary(pallet_salary::Call::payout_other {
			beneficiary: who.clone(),
		}),
		RuntimeCall::SecretaryCollective(pallet_ranked_collective::Call::cleanup_poll {
			poll_index: 0,
			max: 0,
		}),
		RuntimeCall::SecretarySalary(pallet_salary::Call::payout {}),
		RuntimeCall::SecretarySalary(pallet_salary::Call::payout_other { beneficiary: who }),
	];

	for sup in &PROXY_TYPES {
		for sub in PROXY_TYPES.iter().filter(|sub| sup.is_superset(sub)) {
			for call in calls.iter().filter(|call| sub.filter(call)) {
				assert!(
					sup.filter(call),
					"{sup:?} is a superset of {sub:?}, but does not allow {call:?}"
				);
			}
		}
	}
}

type NegativeImbalance =
	<Balances as frame_support::traits::Currency<AccountId>>::NegativeImbalance;

/// Slashes `amount` from `who` and hands it to `Handler`, as a pallet does with a slashed deposit.
fn slash_into<Handler: OnUnbalanced<NegativeImbalance>>(who: &AccountId, amount: Balance) {
	use frame_support::traits::Currency;
	Handler::on_unbalanced(<Balances as Currency<_>>::slash(who, amount).0);
}

/// Slashed deposits and treasury burns accumulate for the forward to the DAP on Asset Hub instead
/// of being burned here, where the burn would not show in the network total that Asset Hub tracks.
#[test]
fn slashes_and_treasury_burns_accumulate_instead_of_being_burned() {
	sp_io::TestExternalities::default().execute_with(|| {
		// Given: the accumulation account holds the ED, as funded out of band.
		let ed: Balance = ExistentialDeposit::get();
		let accumulation_account = AccumulateForward::accumulation_account();
		assert_ok!(Balances::mint_into(&accumulation_account, ed));
		let alice = AccountId::from([1u8; 32]);
		assert_ok!(Balances::mint_into(&alice, 10 * ed));
		let issuance_before = Balances::total_issuance();

		// When: every slash and burn handler receives a sub-ED amount.
		let slash = ed / 4;
		slash_into::<<Runtime as pallet_alliance::Config>::Slashed>(&alice, slash);
		slash_into::<<Runtime as pallet_referenda::Config<FellowshipReferendaInstance>>::Slash>(
			&alice, slash,
		);
		slash_into::<<Runtime as pallet_referenda::Config<AmbassadorReferendaInstance>>::Slash>(
			&alice, slash,
		);
		slash_into::<
			<Runtime as pallet_treasury::Config<FellowshipTreasuryInstance>>::BurnDestination,
		>(&alice, slash);
		slash_into::<
			<Runtime as pallet_treasury::Config<AmbassadorTreasuryInstance>>::BurnDestination,
		>(&alice, slash);

		// Then: all of it reaches the accumulation account and nothing is burned.
		assert_eq!(Balances::balance(&accumulation_account), ed + 5 * slash);
		assert_eq!(Balances::total_issuance(), issuance_before);
	});
}

#[test]
fn dust_accumulates_instead_of_being_burned() {
	sp_io::TestExternalities::default().execute_with(
		system_parachains_test_utils::dust_accumulates_instead_of_being_burned::<Runtime>,
	);
}

#[test]
fn forward_period_counts_relay_blocks() {
	sp_io::TestExternalities::default()
		.execute_with(system_parachains_test_utils::forward_period_counts_relay_blocks::<Runtime>);
}
