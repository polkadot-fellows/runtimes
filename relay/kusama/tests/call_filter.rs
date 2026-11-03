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
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! What the AHM v2 lockdown does to the registrar and HRMP calls, and what a parachain's own
//! origin can reach here once the migration is done.
//!
//! The registrar entry points moved to the Coretime chain, so this chain has to refuse the calls
//! it used to serve while still accepting the requests Coretime sends back. HRMP stays here, so
//! its para-facing calls must reopen once the migration is done, and nothing else may open with
//! them. Both halves matter: block too little and two control planes run at once and diverge;
//! block too much and every parachain flow stops, silently — a call filtered inside XCM surfaces
//! only as a `Transact` that did nothing.

use frame_support::traits::Contains;
use pallet_rc2_migrator::{MigrationStageOf, RcMigrationStage};
use polkadot_primitives::HrmpChannelId;
use polkadot_runtime_common::paras_registrar;
use runtime_parachains::hrmp;
use sp_runtime::BuildStorage;
use staging_kusama_runtime::{Runtime, RuntimeCall};

type Stage = MigrationStageOf<Runtime>;

/// The filter reads the migration stage, so it needs storage. `Pending` is the default, and is
/// what a fresh runtime upgrade lands in.
fn allowed_at(stage: &Stage, call: &RuntimeCall) -> bool {
	with_stage(stage, || <Runtime as frame_system::Config>::BaseCallFilter::contains(call))
}

fn with_stage<R>(stage: &Stage, f: impl FnOnce() -> R) -> R {
	let mut ext: sp_io::TestExternalities = frame_system::GenesisConfig::<Runtime>::default()
		.build_storage()
		.unwrap()
		.into();
	ext.execute_with(|| {
		RcMigrationStage::<Runtime>::put(stage.clone());
		f()
	})
}

/// The HRMP calls a parachain dispatches for itself.
fn hrmp_para_calls() -> Vec<RuntimeCall> {
	vec![
		RuntimeCall::Hrmp(hrmp::Call::<Runtime>::hrmp_init_open_channel {
			recipient: 2001.into(),
			proposed_max_capacity: 8,
			proposed_max_message_size: 1024,
		}),
		RuntimeCall::Hrmp(hrmp::Call::<Runtime>::hrmp_accept_open_channel { sender: 2001.into() }),
		RuntimeCall::Hrmp(hrmp::Call::<Runtime>::hrmp_close_channel {
			channel_id: HrmpChannelId { sender: 2000.into(), recipient: 2001.into() },
		}),
		RuntimeCall::Hrmp(hrmp::Call::<Runtime>::hrmp_cancel_open_request {
			channel_id: HrmpChannelId { sender: 2000.into(), recipient: 2001.into() },
			open_requests: 1,
		}),
		RuntimeCall::Hrmp(hrmp::Call::<Runtime>::establish_channel_with_system {
			target_system_chain: 1000.into(),
		}),
	]
}

/// The registrar's para-facing calls.
fn registrar_para_calls() -> Vec<RuntimeCall> {
	vec![
		RuntimeCall::Registrar(paras_registrar::Call::<Runtime>::deregister { id: 2000.into() }),
		RuntimeCall::Registrar(paras_registrar::Call::<Runtime>::add_lock { para: 2000.into() }),
		RuntimeCall::Registrar(paras_registrar::Call::<Runtime>::remove_lock { para: 2000.into() }),
		RuntimeCall::Registrar(paras_registrar::Call::<Runtime>::set_current_head {
			para: 2000.into(),
			new_head: polkadot_primitives::HeadData(vec![1, 2, 3]),
		}),
		RuntimeCall::Registrar(paras_registrar::Call::<Runtime>::swap {
			id: 2000.into(),
			other: 2001.into(),
		}),
		RuntimeCall::Registrar(paras_registrar::Call::<Runtime>::schedule_code_upgrade {
			para: 2000.into(),
			new_code: polkadot_primitives::ValidationCode(vec![1; 32]),
		}),
	]
}

/// HRMP's para-facing calls close at the start of the migration and reopen once it is done.
///
/// The upgrade must not be the cut-off: closing these at the upgrade would leave parachains with
/// no channel management for as long as governance takes to schedule the start. The middle phase
/// is the load-bearing one: the deposit provider switches to the Coretime chain when the migration
/// is **finished**, so while it runs these calls would still reserve on a chain whose balances
/// are being drained. They must be shut for exactly that window and no longer.
#[test]
fn hrmp_para_facing_calls_reopen_once_the_migration_is_done() {
	for call in hrmp_para_calls() {
		assert!(allowed_at(&Stage::Pending, &call), "{call:?} must survive the upgrade");
		assert!(
			allowed_at(&Stage::Scheduled { start: 100 }, &call),
			"{call:?} must stay open until the migration actually begins"
		);
		for stage in
			[Stage::RegistrarInit, Stage::HrmpInit, Stage::AccountsOngoing { last_key: None }]
		{
			assert!(
				!allowed_at(&stage, &call),
				"{call:?} must be closed while the migration runs, at {stage:?}"
			);
		}
		assert!(
			allowed_at(&Stage::MigrationDone, &call),
			"{call:?} must reopen once the migration is done"
		);
	}
}

/// The registrar's own calls close at the start of the migration and stay closed: the registry
/// lives on the Coretime chain from then on, and `reserve` would hand out an id that is live
/// there.
// TODO(ahm-v2): the four calls a parachain dispatches for itself (`deregister`, `add_lock`,
// `remove_lock`, `set_current_head`) have no relay-chain path once the registry has moved; a para
// reaches `RegistrarPara` on the Coretime chain directly.
#[test]
fn registrar_calls_close_for_good_once_the_migration_starts() {
	let reserve = RuntimeCall::Registrar(paras_registrar::Call::<Runtime>::reserve {});
	for call in registrar_para_calls().into_iter().chain([reserve]) {
		assert!(allowed_at(&Stage::Pending, &call), "{call:?} must survive the upgrade");
		assert!(
			allowed_at(&Stage::Scheduled { start: 100 }, &call),
			"{call:?} must stay open until the migration begins"
		);
		for stage in [Stage::RegistrarInit, Stage::HrmpInit, Stage::MigrationDone] {
			assert!(!allowed_at(&stage, &call), "{call:?} must be closed at {stage:?}");
		}
	}
}

/// What a parachain's own origin reaches once the migration is done, and nothing else.
///
/// A non-system parachain dispatches here as `Origin::Parachain(id)`, the same origin a system
/// chain gets. The call filter cannot see who is calling, and an origin check cannot see the
/// migration stage, so what a para can reach is the intersection of the two: this pins it to
/// HRMP's five para-facing calls. Every other call that accepts a parachain origin is filtered,
/// and every call the lockdown leaves enabled refuses the origin.
mod origins {
	use super::*;
	use frame_support::traits::EnsureOrigin;
	use kusama_runtime_constants::system_parachain::{ASSET_HUB_ID, BROKER_ID};
	use runtime_parachains::origin as parachains_origin;
	use sp_runtime::{traits::Dispatchable, AccountId32, DispatchError};
	use staging_kusama_runtime::{
		para_control::EnsureCoretime, xcm_config::LocalOriginConverter, RuntimeOrigin,
	};
	use xcm::latest::prelude::*;
	use xcm_executor::traits::ConvertOrigin;

	const PARA: u32 = 2000;

	fn para_origin(para: u32) -> RuntimeOrigin {
		parachains_origin::Origin::Parachain(para.into()).into()
	}

	fn dispatch_as_para(call: RuntimeCall) -> Result<(), DispatchError> {
		with_stage(&Stage::MigrationDone, || {
			call.dispatch(para_origin(PARA)).map(|_| ()).map_err(|e| e.error)
		})
	}

	#[test]
	fn every_child_parachain_dispatches_here_as_itself() {
		// System chains and ordinary parachains alike arrive as `Origin::Parachain(id)`: the
		// Coretime chain to drive the control plane, any other para to manage its channels.
		for para in [BROKER_ID, ASSET_HUB_ID, PARA, 3367] {
			let origin = LocalOriginConverter::convert_origin(
				Location::new(0, [Parachain(para)]),
				OriginKind::Native,
			)
			.unwrap_or_else(|_| panic!("para {para} must dispatch here as itself"));
			assert_eq!(
				<RuntimeOrigin as Into<Result<parachains_origin::Origin, RuntimeOrigin>>>::into(
					origin
				)
				.ok(),
				Some(parachains_origin::Origin::Parachain(para.into())),
			);
		}
		// Only the Coretime chain passes as the control plane.
		assert!(EnsureCoretime::try_origin(para_origin(BROKER_ID)).is_ok());
		assert!(EnsureCoretime::try_origin(para_origin(PARA)).is_err());
	}

	#[test]
	fn a_parachain_reaches_its_hrmp_calls_and_nothing_else_once_the_migration_is_done() {
		let filtered: DispatchError = frame_system::Error::<Runtime>::CallFiltered.into();

		// THEN HRMP's para-facing calls dispatch: each gets past the filter and the origin check
		// and fails on its arguments alone, since nothing is onboarded here.
		let expected: Vec<DispatchError> = vec![
			hrmp::Error::<Runtime>::OpenHrmpChannelInvalidSender.into(),
			hrmp::Error::<Runtime>::AcceptHrmpChannelDoesntExist.into(),
			hrmp::Error::<Runtime>::CloseHrmpChannelDoesntExist.into(),
			hrmp::Error::<Runtime>::OpenHrmpChannelDoesntExist.into(),
			hrmp::Error::<Runtime>::OpenHrmpChannelInvalidSender.into(),
		];
		for (call, error) in hrmp_para_calls().into_iter().zip(expected) {
			assert_eq!(dispatch_as_para(call.clone()), Err(error), "{call:?}");
		}

		// THEN every other call that accepts a parachain origin is filtered.
		let other_para_calls = registrar_para_calls().into_iter().chain([
			RuntimeCall::Hrmp(hrmp::Call::<Runtime>::establish_system_channel {
				sender: PARA.into(),
				recipient: 2001.into(),
			}),
			RuntimeCall::Hrmp(hrmp::Call::<Runtime>::poke_channel_deposits {
				sender: PARA.into(),
				recipient: 2001.into(),
			}),
			RuntimeCall::Hrmp(hrmp::Call::<Runtime>::force_process_hrmp_open { channels: 1 }),
			RuntimeCall::Balances(pallet_balances::Call::<Runtime>::transfer_all {
				dest: sp_runtime::MultiAddress::Id(AccountId32::new([1; 32])),
				keep_alive: false,
			}),
			RuntimeCall::Crowdloan(polkadot_runtime_common::crowdloan::Call::<Runtime>::dissolve {
				index: PARA.into(),
			}),
			RuntimeCall::XcmPallet(pallet_xcm::Call::<Runtime>::force_default_xcm_version {
				maybe_xcm_version: None,
			}),
		]);
		for call in other_para_calls {
			assert_eq!(dispatch_as_para(call.clone()), Err(filtered.clone()), "{call:?}");
		}

		// THEN every call the lockdown leaves enabled refuses a parachain's origin.
		let bad_origin: DispatchError = DispatchError::BadOrigin;
		let enabled = [
			RuntimeCall::RegistrarRelay(pallet_registrar_relay::Call::<Runtime>::receive {
				message: registrar_primitives::MessageToRelay::V1(
					registrar_primitives::MessageToRelayV1::Deregister {
						para_id: PARA,
						message_id: 1,
					},
				),
			}),
			RuntimeCall::HrmpRelay(pallet_hrmp_relay::Call::<Runtime>::receive {
				message: hrmp_primitives::MessageToRelay::V1(
					hrmp_primitives::MessageToRelayV1::DepositsSet { id: 1, result: Ok(()) },
				),
			}),
			RuntimeCall::HrmpRelay(pallet_hrmp_relay::Call::<Runtime>::force_answer {
				id: 1,
				result: Ok(()),
			}),
			RuntimeCall::Rc2Migrator(pallet_rc2_migrator::Call::<Runtime>::ct_ready {}),
		];
		for call in enabled {
			assert_eq!(dispatch_as_para(call.clone()), Err(bad_origin.clone()), "{call:?}");
		}
		// The Coretime chain's calls refuse any other para with an error of their own.
		let core_count = RuntimeCall::Coretime(
			runtime_parachains::coretime::Call::<Runtime>::request_core_count { count: 1 },
		);
		assert_eq!(
			dispatch_as_para(core_count),
			Err(runtime_parachains::coretime::Error::<Runtime>::NotBroker.into())
		);
	}
}

/// The Coretime chain encodes this chain's calls by hand, as pallet index + call index, using the
/// same numbers on both networks. Nothing in the type system ties those numbers to this
/// `construct_runtime!`.
///
/// The cross-network drift test in `integration-tests/minimal-relay` decodes real `Transact`
/// payloads, but only with Polkadot's runtimes. What is left for Kusama is that its indices are
/// the ones the encoders assume — so assert exactly that. A `construct_runtime!` reorder here is
/// the realistic way this breaks, and it would otherwise surface only as XCM that silently does
/// nothing.
#[test]
fn the_control_plane_pallets_sit_where_coretime_expects() {
	use frame_support::traits::PalletInfo;

	type Info = <Runtime as frame_system::Config>::PalletInfo;
	assert_eq!(Info::index::<staging_kusama_runtime::RegistrarRelay>(), Some(250));
	assert_eq!(Info::index::<staging_kusama_runtime::HrmpRelay>(), Some(251));
}
