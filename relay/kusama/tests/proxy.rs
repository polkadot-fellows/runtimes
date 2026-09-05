// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Kusama.

// Kusama is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Kusama is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Kusama.  If not, see <http://www.gnu.org/licenses/>.

//! Proxy filter tests.

use frame_support::traits::InstanceFilter;
use kusama_runtime_constants::proxy::ProxyType;
use polkadot_primitives::AccountId;
use polkadot_runtime_common::paras_registrar;
use staging_kusama_runtime::{RuntimeCall, TransparentProxyType};

/// Every `ProxyType` variant. A new variant breaks the `match`, which is the signal to add
/// it here.
fn all_proxy_types() -> Vec<ProxyType> {
	let all = vec![
		ProxyType::Any,
		ProxyType::NonTransfer,
		ProxyType::Governance,
		ProxyType::Staking,
		ProxyType::CancelProxy,
		ProxyType::Auction,
		ProxyType::Society,
		ProxyType::NominationPools,
		ProxyType::Spokesperson,
		ProxyType::ParaRegistration,
	];

	for proxy_type in all.iter() {
		match proxy_type {
			ProxyType::Any |
			ProxyType::NonTransfer |
			ProxyType::Governance |
			ProxyType::Staking |
			ProxyType::CancelProxy |
			ProxyType::Auction |
			ProxyType::Society |
			ProxyType::NominationPools |
			ProxyType::Spokesperson |
			ProxyType::ParaRegistration => (),
		}
	}

	all
}

/// One call for each boundary that the proxy filters draw. The list includes the variants
/// that a filter omits on purpose, such as `Registrar::swap`, because those are the calls
/// that show a lattice violation.
fn representative_calls() -> Vec<RuntimeCall> {
	let account = || AccountId::from([1u8; 32]);

	vec![
		RuntimeCall::System(frame_system::Call::remark { remark: vec![] }),
		RuntimeCall::Indices(pallet_indices::Call::claim { index: 0 }),
		RuntimeCall::Indices(pallet_indices::Call::transfer { new: account().into(), index: 0 }),
		RuntimeCall::Balances(pallet_balances::Call::transfer_keep_alive {
			dest: account().into(),
			value: 1,
		}),
		RuntimeCall::Staking(pallet_staking::Call::chill {}),
		RuntimeCall::Session(pallet_session::Call::purge_keys {}),
		RuntimeCall::Treasury(pallet_treasury::Call::remove_approval { proposal_id: 0 }),
		RuntimeCall::ConvictionVoting(pallet_conviction_voting::Call::unlock {
			class: 0,
			target: account().into(),
		}),
		RuntimeCall::Referenda(pallet_referenda::Call::cancel { index: 0 }),
		RuntimeCall::Vesting(pallet_vesting::Call::vest {}),
		RuntimeCall::Vesting(pallet_vesting::Call::vested_transfer {
			target: account().into(),
			schedule: pallet_vesting::VestingInfo::new(1, 1, 0),
		}),
		RuntimeCall::Utility(pallet_utility::Call::batch { calls: vec![] }),
		RuntimeCall::Proxy(pallet_proxy::Call::reject_announcement {
			delegate: account().into(),
			call_hash: sp_core::H256::zero(),
		}),
		RuntimeCall::Multisig(pallet_multisig::Call::cancel_as_multi {
			threshold: 2,
			other_signatories: vec![account()],
			timepoint: pallet_multisig::Timepoint { height: 0, index: 0 },
			call_hash: [0u8; 32],
		}),
		RuntimeCall::Registrar(paras_registrar::Call::register {
			id: 2000.into(),
			genesis_head: vec![].into(),
			validation_code: vec![].into(),
		}),
		RuntimeCall::Registrar(paras_registrar::Call::deregister { id: 2000.into() }),
		RuntimeCall::Registrar(paras_registrar::Call::reserve {}),
		RuntimeCall::Registrar(paras_registrar::Call::swap { id: 2000.into(), other: 2001.into() }),
		RuntimeCall::Crowdloan(polkadot_runtime_common::crowdloan::Call::dissolve {
			index: 2000.into(),
		}),
		RuntimeCall::Slots(polkadot_runtime_common::slots::Call::trigger_onboard {
			para: 2000.into(),
		}),
		RuntimeCall::Auctions(polkadot_runtime_common::auctions::Call::cancel_auction {}),
		RuntimeCall::VoterList(pallet_bags_list::Call::rebag { dislocated: account().into() }),
		RuntimeCall::NominationPools(pallet_nomination_pools::Call::claim_payout {}),
		RuntimeCall::FastUnstake(pallet_fast_unstake::Call::register_fast_unstake {}),
		RuntimeCall::System(frame_system::Call::remark_with_event { remark: vec![] }),
		RuntimeCall::Society(pallet_society::Call::unbid {}),
	]
}

/// For every declared `is_superset` edge, the superset filter must admit every call that
/// the subset filter admits.
///
/// Cost: 10 proxy types give 100 ordered pairs, of which 26 are declared edges (10
/// reflexive, 9 more under `Any`, 7 more under `NonTransfer`). With 26 calls, the inner
/// loop runs 676 times and makes at most 1352 filter probes. Each probe is one `matches!`
/// on an enum. The test completes in under 10 ms.
#[test]
fn proxy_type_superset_relation_matches_call_filters() {
	let calls = representative_calls();

	for superset in all_proxy_types() {
		for subset in all_proxy_types() {
			if !TransparentProxyType(superset).is_superset(&TransparentProxyType(subset)) {
				continue;
			}

			for call in calls.iter() {
				if TransparentProxyType(subset).filter(call) {
					assert!(
						TransparentProxyType(superset).filter(call),
						"lattice violated: {superset:?} declares itself a superset of \
						 {subset:?}, but rejects {call:?} which {subset:?} admits",
					);
				}
			}
		}
	}
}
