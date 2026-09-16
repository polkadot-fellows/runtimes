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

//! What `PostAhmFilter` closes, and what it must leave open.
//!
//! The registrar and HRMP entry points moved to the Coretime chain, so this chain has to refuse
//! the calls it used to serve while still accepting the requests Coretime sends back. Both halves
//! matter: block too little and two control planes run at once and diverge; block too much and
//! every parachain flow stops, silently — a call filtered inside XCM surfaces only as a `Transact`
//! that did nothing.

use frame_support::{
	traits::{Contains, PalletsInfoAccess},
	weights::Weight,
};
use pallet_rc2_migrator::{MigrationStageOf, RcMigrationStage};
use polkadot_primitives::{AccountId, HrmpChannelId};
use polkadot_runtime::{
	xcm_config::{Barrier, XcmConfig},
	AllPalletsWithSystem, PostAhmFilter, Runtime, RuntimeCall,
};
use polkadot_runtime_common::{crowdloan, paras_registrar};
use polkadot_runtime_constants::{currency::UNITS, system_parachain::ASSET_HUB_ID};
use runtime_parachains::hrmp;
use sp_runtime::BuildStorage;
use xcm::latest::prelude::*;
use xcm_executor::{
	traits::{Properties, ShouldExecute},
	XcmExecutor,
};

type Stage = MigrationStageOf<Runtime>;

/// The filter reads the migration stage, so it needs storage. `Pending` is the default, and is
/// what a fresh runtime upgrade lands in.
fn allowed_at(stage: Stage, call: &RuntimeCall) -> bool {
	let mut ext: sp_io::TestExternalities = frame_system::GenesisConfig::<Runtime>::default()
		.build_storage()
		.unwrap()
		.into();
	ext.execute_with(|| {
		RcMigrationStage::<Runtime>::put(stage);
		PostAhmFilter::contains(call)
	})
}

/// The registrar's own calls: served until the migration starts, closed from then on.
///
/// The upgrade must not be the cut-off. The Coretime pallets hold nothing until the migration
/// hands state over, so closing these at the upgrade would leave users with no registrar on
/// either chain for as long as governance takes to schedule the start. Root is unaffected
/// throughout: it bypasses the base call filter entirely, which is how governance and the
/// migration still reach these.
///
/// The calls a parachain dispatches **for itself** follow a three-phase schedule rather than
/// closing for good, because after the migration their bodies no longer touch this chain — they
/// forward the request to Coretime on the para's behalf. Keeping them open is what lets every
/// parachain go on encoding exactly the call it encodes today.
///
/// The middle phase is the load-bearing one. The forwarder turns on when the migration is
/// **finished**, so while it is running these calls would still take the local path and act on a
/// half-drained registry. They must be shut for exactly that window and no longer.
#[test]
fn para_facing_calls_reopen_as_forwarders_once_the_migration_is_done() {
	for call in [
		RuntimeCall::Registrar(paras_registrar::Call::<Runtime>::deregister { id: 2000.into() }),
		RuntimeCall::Registrar(paras_registrar::Call::<Runtime>::add_lock { para: 2000.into() }),
		RuntimeCall::Registrar(paras_registrar::Call::<Runtime>::remove_lock { para: 2000.into() }),
		RuntimeCall::Registrar(paras_registrar::Call::<Runtime>::set_current_head {
			para: 2000.into(),
			new_head: polkadot_primitives::HeadData(vec![1, 2, 3]),
		}),
		RuntimeCall::Hrmp(hrmp::Call::<Runtime>::hrmp_init_open_channel {
			recipient: 2001.into(),
			proposed_max_capacity: 8,
			proposed_max_message_size: 1024,
		}),
		RuntimeCall::Hrmp(hrmp::Call::<Runtime>::hrmp_accept_open_channel { sender: 2000.into() }),
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
	] {
		// Before the migration begins: served locally, exactly as today.
		assert!(allowed_at(Stage::Pending, &call), "{call:?} must survive the upgrade");
		assert!(
			allowed_at(Stage::Scheduled { start: 100 }, &call),
			"{call:?} must stay open until the migration actually begins"
		);

		// While it runs: shut. The forwarder is not on yet and the registry is half drained.
		for stage in [Stage::RegistrarInit, Stage::HrmpInit, Stage::Paused] {
			assert!(
				!allowed_at(stage.clone(), &call),
				"{call:?} must be closed while the migration runs, at {stage:?}"
			);
		}

		// Afterwards: open again, now forwarding to Coretime.
		assert!(
			allowed_at(Stage::MigrationDone, &call),
			"{call:?} must reopen as a forwarder once the migration is done"
		);
	}
}

/// The calls that cannot be forwarded, and so close for good.
///
/// `reserve` and `swap` have no Coretime counterpart a para may drive — ids are allocated there and
/// swap is retired. `schedule_code_upgrade` carries the whole validation code, which is precisely
/// what the Coretime protocol refuses to move: it commits to a hash and a length and has the blob
/// uploaded separately. A parachain's ordinary upgrade path is `parachain_system`'s
/// `authorize_upgrade`, which never touches this pallet.
#[test]
fn calls_that_cannot_be_forwarded_stay_closed() {
	for call in [
		RuntimeCall::Registrar(paras_registrar::Call::<Runtime>::reserve {}),
		RuntimeCall::Registrar(paras_registrar::Call::<Runtime>::swap {
			id: 2000.into(),
			other: 2001.into(),
		}),
		RuntimeCall::Registrar(paras_registrar::Call::<Runtime>::schedule_code_upgrade {
			para: 2000.into(),
			new_code: polkadot_primitives::ValidationCode(vec![1; 32]),
		}),
	] {
		assert!(allowed_at(Stage::Pending, &call), "{call:?} must survive the upgrade");
		assert!(
			allowed_at(Stage::Scheduled { start: 100 }, &call),
			"{call:?} must stay open until the migration begins"
		);
		for stage in [Stage::RegistrarInit, Stage::Paused, Stage::MigrationDone] {
			assert!(!allowed_at(stage.clone(), &call), "{call:?} must be closed at {stage:?}");
		}
	}
}

/// `PostAhmFilter` closes the calls this chain used to serve; this closes the *origin* those calls
/// would have arrived with. The two are complementary and neither substitutes for the other: a
/// `Contains<RuntimeCall>` cannot see who is calling, so without this a non-system parachain could
/// still reach any relay-chain pallet nobody thought to filter.
mod origins {
	use frame_support::traits::EnsureOrigin;
	use polkadot_runtime::{
		para_control::EnsureAnyParaSelf, xcm_config::SystemChildParachainAsNative, RuntimeOrigin,
	};
	use polkadot_runtime_constants::system_parachain::{ASSET_HUB_ID, BROKER_ID};
	use runtime_parachains::origin as parachains_origin;
	use xcm::latest::prelude::*;
	use xcm_executor::traits::ConvertOrigin;

	fn native_origin(para: u32) -> Result<RuntimeOrigin, Location> {
		SystemChildParachainAsNative::convert_origin(
			Location::new(0, [Parachain(para)]),
			OriginKind::Native,
		)
	}

	#[test]
	fn system_parachains_keep_their_own_origin() {
		// Coretime drives the parachain control plane and Asset Hub drives staking; both reach
		// this chain as `Origin::Parachain(id)`, not as Root.
		for para in [BROKER_ID, ASSET_HUB_ID] {
			assert!(native_origin(para).is_ok(), "system para {para} must keep its origin");
		}
	}

	#[test]
	fn ordinary_parachains_get_the_narrow_control_plane_origin() {
		// Not `parachains_origin::Origin::Parachain`, which eleven calls accept — the control
		// plane's own origin, which is accepted by the nine calls a para dispatches for itself and
		// by nothing else. Two properties, and the second is the one that matters:
		for para in [2000u32, 2004, 3367] {
			let origin =
				native_origin(para).unwrap_or_else(|_| {
					panic!("para {para} must dispatch here as itself, or it cannot reach the forwarders")
				});

			// It resolves to the para, for the calls that accept it.
			assert_eq!(
				<EnsureAnyParaSelf as EnsureOrigin<RuntimeOrigin>>::try_origin(origin.clone()).ok(),
				Some(para.into()),
				"para {para} must resolve through the control-plane origin"
			);

			// And it is *not* the system-chain origin, so nothing that accepts only that can be
			// reached with it — including a pallet nobody remembered to filter.
			assert!(
				<RuntimeOrigin as Into<Result<parachains_origin::Origin, RuntimeOrigin>>>::into(
					origin
				)
				.is_err(),
				"para {para} must not obtain the system-chain parachain origin"
			);
		}
	}
}

// ---------------------------------------------------------------------------
// What the migration closes for good, from its first block
// ---------------------------------------------------------------------------

const ALICE: AccountId = AccountId::new([1u8; 32]);

/// Every stage in which the relay chain still serves its users, and every stage in which it must
/// not. `MigrationDone` sits with the latter: what the migration closes does not reopen.
fn open_stages() -> [Stage; 2] {
	[Stage::Pending, Stage::Scheduled { start: 1_000 }]
}
fn closed_stages() -> [Stage; 5] {
	[
		Stage::WaitingForCt,
		Stage::AccountsOngoing { last_key: None },
		Stage::Paused,
		Stage::CoolOff { end_at: 10 },
		Stage::MigrationDone,
	]
}

fn every_stage() -> impl Iterator<Item = Stage> {
	open_stages().into_iter().chain(closed_stages())
}

fn assert_closes_at_migration_start(call: RuntimeCall) {
	for stage in open_stages() {
		assert!(allowed_at(stage.clone(), &call), "{call:?} must stay open at {stage:?}");
	}
	for stage in closed_stages() {
		assert!(!allowed_at(stage.clone(), &call), "{call:?} must be closed at {stage:?}");
	}
}

/// The calls a signed origin could use to move value or resize a reserve while the accounts stage
/// is draining them.
///
/// One per pallet the filter names: the arms match on the pallet, so a single call witnesses each.
#[test]
fn value_movers_close_when_the_migration_starts() {
	for call in [
		RuntimeCall::Balances(pallet_balances::Call::<Runtime>::transfer_allow_death {
			dest: ALICE.into(),
			value: UNITS,
		}),
		RuntimeCall::XcmPallet(pallet_xcm::Call::<Runtime>::transfer_assets {
			dest: Box::new(Parachain(ASSET_HUB_ID).into_location().into_versioned()),
			beneficiary: Box::new(
				Location::new(0, [AccountId32 { network: None, id: ALICE.into() }])
					.into_versioned(),
			),
			assets: Box::new(Assets::from(vec![(Here, UNITS).into()]).into()),
			fee_asset_item: 0,
			weight_limit: Unlimited,
		}),
		RuntimeCall::Multisig(pallet_multisig::Call::<Runtime>::approve_as_multi {
			threshold: 2,
			other_signatories: vec![ALICE],
			maybe_timepoint: None,
			call_hash: [0u8; 32],
			max_weight: Weight::zero(),
		}),
		RuntimeCall::Preimage(pallet_preimage::Call::<Runtime>::note_preimage {
			bytes: vec![1, 2, 3],
		}),
		RuntimeCall::OnDemand(
			runtime_parachains::on_demand::Call::<Runtime>::place_order_allow_death {
				max_amount: UNITS,
				para_id: 2000.into(),
			},
		),
		RuntimeCall::Crowdloan(crowdloan::Call::<Runtime>::withdraw {
			who: ALICE,
			index: 0.into(),
		}),
	] {
		assert_closes_at_migration_start(call);
	}
}

/// Using a proxy is not a value movement; changing the proxy map or an announcement is, because
/// both resize a reserve.
#[test]
fn proxies_keep_working_but_stop_changing() {
	let add = RuntimeCall::Proxy(pallet_proxy::Call::<Runtime>::add_proxy {
		delegate: ALICE.into(),
		proxy_type: polkadot_runtime::TransparentProxyType(
			polkadot_runtime_constants::proxy::ProxyType::Any,
		),
		delay: 0,
	});
	assert_closes_at_migration_start(add);

	let announce = RuntimeCall::Proxy(pallet_proxy::Call::<Runtime>::announce {
		real: ALICE.into(),
		call_hash: Default::default(),
	});
	assert_closes_at_migration_start(announce);

	let poke = RuntimeCall::Proxy(pallet_proxy::Call::<Runtime>::poke_deposit {});
	assert_closes_at_migration_start(poke);

	// The wrapper stays dispatchable throughout.
	let use_proxy = RuntimeCall::Proxy(pallet_proxy::Call::<Runtime>::proxy {
		real: ALICE.into(),
		force_proxy_type: None,
		call: Box::new(RuntimeCall::System(frame_system::Call::<Runtime>::remark {
			remark: vec![],
		})),
	});
	for stage in every_stage() {
		assert!(allowed_at(stage.clone(), &use_proxy), "using a proxy must survive {stage:?}");
	}
}

/// The filter is a list, not a mode: a call it does not name is unaffected at every stage.
#[test]
fn calls_the_migration_does_not_name_are_untouched() {
	let call = RuntimeCall::System(frame_system::Call::<Runtime>::remark { remark: vec![1, 2, 3] });
	for stage in every_stage() {
		assert!(allowed_at(stage.clone(), &call), "{call:?} must be unaffected at {stage:?}");
	}
}

/// The filter names the pallets it closes and lets every other call through, so a pallet added to
/// this runtime is reachable under the drain unless someone decides otherwise.
///
/// This list is that decision, recorded. Adding or removing a pallet fails here, and the fix is to
/// say which side of `PostAhmFilter` the new one belongs on.
#[test]
fn every_pallet_has_been_weighed_against_the_filter() {
	let mut present = <AllPalletsWithSystem as PalletsInfoAccess>::infos()
		.into_iter()
		.map(|pallet| pallet.name)
		.collect::<Vec<_>>();
	present.sort_unstable();

	let considered = vec![
		"AssetRate",
		"Auctions",
		"AuthorityDiscovery",
		"Authorship",
		"Babe",
		"Balances",
		"Beefy",
		"BeefyMmrLeaf",
		"Bounties",
		"ChildBounties",
		"Claims",
		"Configuration",
		"ConvictionVoting",
		"Coretime",
		"Crowdloan",
		"DelegatedStaking",
		"Dmp",
		"ElectionProviderMultiPhase",
		"FastUnstake",
		"Grandpa",
		"Historical",
		"Hrmp",
		"HrmpRelay",
		"Indices",
		"Initializer",
		"MessageQueue",
		"Mmr",
		"Multisig",
		"NominationPools",
		"Offences",
		"OnDemand",
		"Origins",
		"ParaInclusion",
		"ParaInherent",
		"ParaScheduler",
		"ParaSessionInfo",
		"ParachainsOrigin",
		"Parameters",
		"Paras",
		"ParasDisputes",
		"ParasShared",
		"ParasSlashing",
		"Preimage",
		"Proxy",
		"Rc2Migrator",
		"RcMigrator",
		"Referenda",
		"Registrar",
		"RegistrarRelay",
		"Scheduler",
		"Session",
		"Slots",
		"Staking",
		"StakingAhClient",
		"System",
		"Timestamp",
		"TransactionPayment",
		"Treasury",
		"Utility",
		"Vesting",
		"VoterList",
		"Whitelist",
		"XcmPallet",
	];

	assert_eq!(present, considered);
}

/// The executor's teleport trust, which a call filter cannot cover: an inbound
/// `ReceiveTeleportedAsset` dispatches nothing on this chain.
mod inbound_teleports {
	use super::*;

	fn teleport_from_asset_hub(stage: Stage) -> Outcome {
		let mut ext: sp_io::TestExternalities = frame_system::GenesisConfig::<Runtime>::default()
			.build_storage()
			.unwrap()
			.into();
		ext.execute_with(|| {
			RcMigrationStage::<Runtime>::put(stage);
			let message = Xcm::<RuntimeCall>(vec![
				ReceiveTeleportedAsset(Assets::from(vec![(Here, 10 * UNITS).into()])),
				DepositAsset {
					assets: AllCounted(1).into(),
					beneficiary: Location::new(
						0,
						[AccountId32 { network: None, id: ALICE.into() }],
					),
				},
			]);
			let weight = Weight::from_parts(10_000_000_000, 1_000_000);
			XcmExecutor::<XcmConfig>::prepare_and_execute(
				Parachain(ASSET_HUB_ID),
				message,
				&mut [0u8; 32],
				weight,
				weight,
			)
		})
	}

	#[test]
	fn a_system_chain_can_teleport_here_until_the_migration_starts() {
		for stage in open_stages() {
			assert_eq!(
				teleport_from_asset_hub(stage.clone()).ensure_complete(),
				Ok(()),
				"teleports must still land at {stage:?}"
			);
		}
	}

	#[test]
	fn no_one_can_teleport_here_once_the_migration_starts() {
		for stage in closed_stages() {
			let error = teleport_from_asset_hub(stage.clone())
				.ensure_complete()
				.expect_err("teleport must be refused");
			assert_eq!(
				error.error,
				XcmError::UntrustedTeleportLocation,
				"teleport must be refused as untrusted at {stage:?}"
			);
		}
	}
}

/// The barrier, which a call filter cannot reach: upward messages arrive with candidates.
mod inbound_messages {
	use super::*;

	fn barrier_admits(stage: Stage, origin: Location, mut message: Xcm<RuntimeCall>) -> bool {
		let mut ext: sp_io::TestExternalities = frame_system::GenesisConfig::<Runtime>::default()
			.build_storage()
			.unwrap()
			.into();
		ext.execute_with(|| {
			RcMigrationStage::<Runtime>::put(stage);
			let weight = Weight::from_parts(10_000_000_000, 1_000_000);
			let mut properties = Properties { weight_credit: Weight::zero(), message_id: None };
			Barrier::should_execute(&origin, message.inner_mut(), weight, &mut properties).is_ok()
		})
	}

	/// What any parachain may send: pay for execution up front.
	fn paid_message() -> Xcm<RuntimeCall> {
		Xcm(vec![
			WithdrawAsset((Here, UNITS).into()),
			BuyExecution { fees: (Here, UNITS).into(), weight_limit: Unlimited },
			ClearOrigin,
		])
	}

	/// What a system chain may send: execution it does not pay for.
	fn unpaid_message() -> Xcm<RuntimeCall> {
		Xcm(vec![UnpaidExecution { weight_limit: Unlimited, check_origin: None }, ClearOrigin])
	}

	/// Shut while the migration runs, open before it and again after it: the stages in which the
	/// para's control-plane calls forward to the Coretime chain are the ones it may speak in.
	#[test]
	fn ordinary_parachains_are_refused_only_while_the_migration_runs() {
		let para = Location::new(0, [Parachain(2000)]);
		for stage in open_stages() {
			assert!(
				barrier_admits(stage.clone(), para.clone(), paid_message()),
				"an ordinary para must be admitted at {stage:?}"
			);
		}
		for stage in [
			Stage::WaitingForCt,
			Stage::AccountsOngoing { last_key: None },
			Stage::Paused,
			Stage::CoolOff { end_at: 10 },
		] {
			assert!(
				!barrier_admits(stage.clone(), para.clone(), paid_message()),
				"an ordinary para must be refused at {stage:?}"
			);
		}
		assert!(
			barrier_admits(Stage::MigrationDone, para, paid_message()),
			"an ordinary para must be admitted again once the migration is done"
		);
	}

	/// The migration's own traffic and Asset Hub's staking traffic travel as system-chain
	/// messages; the gate must not touch them at any stage.
	#[test]
	fn system_chains_are_admitted_throughout() {
		let asset_hub = Location::new(0, [Parachain(ASSET_HUB_ID)]);
		for stage in every_stage() {
			assert!(
				barrier_admits(stage.clone(), asset_hub.clone(), unpaid_message()),
				"a system chain must be admitted at {stage:?}"
			);
		}
	}
}
