// Copyright (C) Polkadot Fellows.
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

//! The relay chain's AHM v2 lockdown: which calls each migration stage allows, and the barrier
//! that refuses inbound messages once the migration starts.

use crate::{
	parachains_hrmp, parachains_paras, parachains_slashing, xcm_config::OnlyParachains, ParaId,
	Runtime, RuntimeCall,
};
use frame_support::{
	traits::{Contains, ProcessMessageError},
	weights::Weight,
};
use pallet_rc2_migrator::RcMigrationStage;
use polkadot_runtime_constants::system_parachain::{ASSET_HUB_ID, BROKER_ID};
use xcm::latest::{Instruction, Junction::Parachain, Location};
use xcm_builder::IsChildSystemParachain;
use xcm_executor::traits::{DenyExecution, Properties};

/// Refuses every inbound message once the migration starts, except from this chain, the Coretime
/// chain and Asset Hub. Once the migration is done, ordinary parachains get through again, to reach
/// their own HRMP calls.
pub struct DenyOnceMigrationStarts;
impl DenyExecution for DenyOnceMigrationStarts {
	fn deny_execution<RuntimeCall>(
		origin: &Location,
		_instructions: &mut [Instruction<RuntimeCall>],
		_max_weight: Weight,
		_properties: &mut Properties,
	) -> Result<(), ProcessMessageError> {
		let stage = RcMigrationStage::<Runtime>::get();
		if !stage.has_started() || (stage.is_finished() && is_ordinary_parachain(origin)) {
			Ok(())
		} else {
			only_from_self_coretime_or_asset_hub(origin)
		}
	}
}

fn is_ordinary_parachain(origin: &Location) -> bool {
	OnlyParachains::contains(origin) && !IsChildSystemParachain::<ParaId>::contains(origin)
}

/// CT assigns cores and answers the migrator. AH governance dispatches here as Root / admin body,
/// and its staking sends the validator set and session keys.
fn only_from_self_coretime_or_asset_hub(origin: &Location) -> Result<(), ProcessMessageError> {
	match origin.unpack() {
		(0, []) => Ok(()),
		(0, [Parachain(id)]) if *id == BROKER_ID || *id == ASSET_HUB_ID => Ok(()),
		_ => Err(ProcessMessageError::Unsupported),
	}
}

/// Contains all calls that are enabled during the migration.
pub struct CallsEnabledDuringMigration;
impl Contains<RuntimeCall> for CallsEnabledDuringMigration {
	fn contains(call: &RuntimeCall) -> bool {
		call_allowed_status(call).0
	}
}

/// Contains all calls that are enabled after the migration.
pub struct CallsEnabledAfterMigration;
impl Contains<RuntimeCall> for CallsEnabledAfterMigration {
	fn contains(call: &RuntimeCall) -> bool {
		call_allowed_status(call).1
	}
}

/// Return whether a call is enabled during and after the migration.
///
/// During is from the start signal to the Coretime chain until the end of the cool-off. After is
/// from `MigrationDone` on.
///
/// Every call a signed account can make is disabled. Enabled are inherents, unsigned validator
/// reports, calls the Coretime chain and Asset Hub send over XCM, and the migrator itself. After
/// the migration, the HRMP calls a parachain makes for itself open again; their deposits are held
/// on the Coretime chain.
///
/// Root skips this filter, so a call only Root can make is `OFF` here and still works.
pub fn call_allowed_status(call: &RuntimeCall) -> (bool, bool) {
	use RuntimeCall::*;
	const ON: bool = true;
	const OFF: bool = false;

	match call {
		// Enabled during and after the migration.
		// Applies a runtime upgrade governance already authorized.
		System(frame_system::Call::apply_authorized_upgrade { .. }) => (ON, ON),
		Babe(pallet_babe::Call::report_equivocation_unsigned { .. }) => (ON, ON),
		// Only the `set` inherent.
		Timestamp(..) => (ON, ON),
		Grandpa(pallet_grandpa::Call::report_equivocation_unsigned { .. }) => (ON, ON),
		// Asset Hub or the admin origin only. Era rotation should continue.
		StakingAhClient(..) => (ON, ON),
		// Asset Hub's StakingAdmin reaches it over XCM, not as Root.
		Parameters(..) => (ON, ON),
		// Only the `enter` inherent.
		ParaInherent(..) => (ON, ON),
		Paras(
			parachains_paras::Call::include_pvf_check_statement { .. } |
			parachains_paras::Call::apply_authorized_force_set_current_code { .. },
		) => (ON, ON),
		ParasSlashing(parachains_slashing::Call::report_dispute_lost_unsigned { .. }) => (ON, ON),
		// The Coretime chain only. `request_revenue_at` lets it claim the on-demand revenue earned
		// before the start.
		Coretime(
			runtime_parachains::coretime::Call::request_core_count { .. } |
			runtime_parachains::coretime::Call::request_revenue_at { .. } |
			runtime_parachains::coretime::Call::assign_core { .. },
		) => (ON, ON),
		Beefy(
			pallet_beefy::Call::report_double_voting_unsigned { .. } |
			pallet_beefy::Call::report_fork_voting_unsigned { .. } |
			pallet_beefy::Call::report_future_block_voting_unsigned { .. },
		) => (ON, ON),
		// Checks its own origins; drives the migration.
		Rc2Migrator(..) => (ON, ON),
		// The Coretime chain's reports and answers, Root, and unsigned code uploads.
		RegistrarRelay(..) | HrmpRelay(..) => (ON, ON),
		// The HRMP calls a parachain makes for itself. After the migration their deposits are held
		// on the Coretime chain; the calls themselves stay here.
		Hrmp(
			parachains_hrmp::Call::hrmp_init_open_channel { .. } |
			parachains_hrmp::Call::hrmp_accept_open_channel { .. } |
			parachains_hrmp::Call::hrmp_close_channel { .. } |
			parachains_hrmp::Call::hrmp_cancel_open_request { .. } |
			parachains_hrmp::Call::establish_channel_with_system { .. },
		) => (OFF, ON),

		// Disabled during and after the migration.
		System(..) => (OFF, OFF),
		Scheduler(..) => (OFF, OFF),
		Preimage(..) => (OFF, OFF),
		Babe(..) => (OFF, OFF),
		Indices(..) => (OFF, OFF),
		Balances(..) => (OFF, OFF),
		Staking(..) => (OFF, OFF),
		Session(..) => (OFF, OFF),
		Grandpa(..) => (OFF, OFF),
		Treasury(..) => (OFF, OFF),
		ConvictionVoting(..) => (OFF, OFF),
		Referenda(..) => (OFF, OFF),
		Whitelist(..) => (OFF, OFF),
		Claims(..) => (OFF, OFF),
		Vesting(..) => (OFF, OFF),
		Utility(..) => (OFF, OFF),
		Proxy(..) => (OFF, OFF),
		Multisig(..) => (OFF, OFF),
		Bounties(..) => (OFF, OFF),
		ChildBounties(..) => (OFF, OFF),
		ElectionProviderMultiPhase(..) => (OFF, OFF),
		VoterList(..) => (OFF, OFF),
		NominationPools(..) => (OFF, OFF),
		FastUnstake(..) => (OFF, OFF),
		ParasShared(..) => (OFF, OFF),
		ParaInclusion(..) => (OFF, OFF),
		Paras(..) => (OFF, OFF),
		// Root only.
		Configuration(..) | Initializer(..) | ParasDisputes(..) => (OFF, OFF),
		// Signed callers and the channel manager only.
		Hrmp(..) => (OFF, OFF),
		ParasSlashing(..) => (OFF, OFF),
		OnDemand(..) => (OFF, OFF),
		// TODO(ahm-v2): the calls a parachain makes for itself (`deregister`, `add_lock`,
		// `remove_lock`, `set_current_head`) have no relay-chain path once the registry is on the
		// Coretime chain; a para reaches `RegistrarPara` there directly.
		Registrar(..) => (OFF, OFF),
		Slots(..) => (OFF, OFF),
		Auctions(..) => (OFF, OFF),
		Crowdloan(..) => (OFF, OFF),
		// On-demand orders close at the start, so credits have no use.
		Coretime(..) => (OFF, OFF),
		XcmPallet(..) => (OFF, OFF),
		MessageQueue(..) => (OFF, OFF),
		AssetRate(..) => (OFF, OFF),
		Beefy(..) => (OFF, OFF),
	}
}

#[cfg(test)]
mod tests {
	use crate::{
		xcm_config::SovereignAccountOf, AccountId, Balances, PostAhmFilter, Runtime, RuntimeCall,
		RuntimeOrigin,
	};
	use frame_support::traits::{fungible::Mutate, Contains};
	use pallet_rc2_migrator::{
		test_utils::{every_call, CallName},
		MigrationStageOf, RcMigrationStage,
	};
	use polkadot_runtime_constants::{
		currency::UNITS,
		system_parachain::{ASSET_HUB_ID, BRIDGE_HUB_ID, BROKER_ID},
	};
	use sp_runtime::traits::Dispatchable;
	use xcm::latest::prelude::*;
	use xcm_executor::{traits::ConvertLocation, XcmExecutor};

	type Stage = MigrationStageOf<Runtime>;

	/// Any account.
	const ALICE: AccountId = AccountId::new([1; 32]);

	/// Run `f` with the migration at `stage`.
	fn at<R>(stage: &Stage, f: impl FnOnce() -> R) -> R {
		sp_io::TestExternalities::default().execute_with(|| {
			RcMigrationStage::<Runtime>::put(stage);
			f()
		})
	}

	fn stages_before_start() -> Vec<Stage> {
		vec![Stage::Pending, Stage::Scheduled { start: 0 }]
	}

	/// One stage from each part of the migration, and its end.
	fn stages_from_start() -> Vec<Stage> {
		vec![
			Stage::WaitingForCt,
			Stage::WarmUp { end_at: 0 },
			Stage::AccountsOngoing { last_key: None },
			Stage::CoolOff { end_at: 0 },
			Stage::MigrationDone,
		]
	}

	/// Every call the lockdown leaves enabled: inherents, unsigned validator reports, applying code
	/// governance already authorized, calls only the Coretime chain or Asset Hub can make, and the
	/// migrator's own.
	const ENABLED: &[CallName] = &[
		("System", "apply_authorized_upgrade"),
		("Babe", "report_equivocation_unsigned"),
		("Timestamp", "set"),
		("Grandpa", "report_equivocation_unsigned"),
		("StakingAhClient", "validator_set"),
		("StakingAhClient", "set_mode"),
		("StakingAhClient", "force_on_migration_end"),
		("StakingAhClient", "set_keys_from_ah"),
		("StakingAhClient", "purge_keys_from_ah"),
		("Parameters", "set_parameter"),
		("ParaInherent", "enter"),
		("Paras", "include_pvf_check_statement"),
		("Paras", "apply_authorized_force_set_current_code"),
		("ParasSlashing", "report_dispute_lost_unsigned"),
		("Coretime", "request_core_count"),
		("Coretime", "request_revenue_at"),
		("Coretime", "assign_core"),
		("Beefy", "report_double_voting_unsigned"),
		("Beefy", "report_fork_voting_unsigned"),
		("Beefy", "report_future_block_voting_unsigned"),
		("Rc2Migrator", "schedule_migration"),
		("Rc2Migrator", "cancel_migration"),
		("Rc2Migrator", "ct_ready"),
		("Rc2Migrator", "pause_migration"),
		("Rc2Migrator", "resume_migration"),
		("Rc2Migrator", "force_set_stage"),
		("Rc2Migrator", "set_manager"),
		("Rc2Migrator", "vote_manager_multisig"),
		("Rc2Migrator", "set_ct_ump_queue_priority"),
		("Rc2Migrator", "receive_query_response"),
		("Rc2Migrator", "retry_batch"),
		("Rc2Migrator", "abandon_batch"),
		("Rc2Migrator", "set_unprocessed_msg_buffer"),
		("RegistrarRelay", "receive"),
		("RegistrarRelay", "apply_authorized_code"),
		("HrmpRelay", "receive"),
		("HrmpRelay", "force_answer"),
	];

	/// Every call that opens again once the migration is done: the HRMP calls a parachain makes
	/// for itself.
	const ENABLED_AFTER: &[CallName] = &[
		("Hrmp", "hrmp_init_open_channel"),
		("Hrmp", "hrmp_accept_open_channel"),
		("Hrmp", "hrmp_close_channel"),
		("Hrmp", "hrmp_cancel_open_request"),
		("Hrmp", "establish_channel_with_system"),
	];

	/// Calls the test below cannot build. It builds each call from zero bytes, and these take an
	/// argument whose first byte is a version or variant tag that zero is not: XCM's `Versioned*`
	/// types start at version 3, BABE's `NextConfigDescriptor` at 1. Each falls under its pallet's
	/// catch-all arm in [`super::call_allowed_status`].
	const UNDECODABLE: &[CallName] = &[
		("Babe", "plan_config_change"),
		("Treasury", "spend"),
		("XcmPallet", "send"),
		("XcmPallet", "teleport_assets"),
		("XcmPallet", "reserve_transfer_assets"),
		("XcmPallet", "execute"),
		("XcmPallet", "force_subscribe_version_notify"),
		("XcmPallet", "force_unsubscribe_version_notify"),
		("XcmPallet", "limited_reserve_transfer_assets"),
		("XcmPallet", "limited_teleport_assets"),
		("XcmPallet", "transfer_assets"),
		("XcmPallet", "claim_assets"),
		("XcmPallet", "transfer_assets_using_type_and_then"),
		("XcmPallet", "add_authorized_alias"),
		("XcmPallet", "remove_authorized_alias"),
		("AssetRate", "create"),
		("AssetRate", "update"),
		("AssetRate", "remove"),
	];

	/// If this fails after an upgrade, a call was added, renamed or removed. For a new call that no
	/// signed account can reach (an inherent, an unsigned report, an XCM-only or migrator call),
	/// enable it in [`super::call_allowed_status`] and list it in [`ENABLED`]. For a call a signed
	/// account can reach, keep it disabled. A rename or removal only needs the lists updated.
	#[test]
	fn every_call_but_the_listed_ones_is_refused_once_the_migration_starts() {
		let (calls, undecodable) = every_call::<RuntimeCall>();
		assert_eq!(undecodable, UNDECODABLE, "calls the test cannot build changed");
		// Every listed call is a call of this runtime.
		for name in ENABLED.iter().chain(ENABLED_AFTER) {
			assert!(calls.iter().any(|(n, _)| n == name), "{name:?} is not a call of this runtime");
		}

		for stage in stages_before_start().into_iter().chain(stages_from_start()) {
			at(&stage, || {
				for (name, call) in &calls {
					let expected = if stage.is_finished() {
						// After the migration, a call passes exactly when it is listed in
						// `ENABLED` or `ENABLED_AFTER`.
						ENABLED.contains(name) || ENABLED_AFTER.contains(name)
					} else if stage.has_started() {
						// During the migration, a call passes exactly when it is listed in
						// `ENABLED`.
						ENABLED.contains(name)
					} else {
						// Before the start, the lockdown adds nothing: the filter is
						// `PostAhmFilter`.
						PostAhmFilter::contains(call)
					};
					// This fails both ways: a listed call that is refused, and an unlisted call
					// that passes.
					assert_eq!(
						<Runtime as frame_system::Config>::BaseCallFilter::contains(call),
						expected,
						"{}::{} at {stage:?}",
						name.0,
						name.1,
					);
				}
			});
		}
	}

	#[test]
	fn root_skips_the_filter() {
		let remark = RuntimeCall::System(frame_system::Call::remark { remark: vec![1] });

		for stage in stages_from_start() {
			at(&stage, || {
				// THEN a signed account's remark is filtered, and Root's is not.
				assert_eq!(
					remark.clone().dispatch(RuntimeOrigin::signed(ALICE)).map_err(|e| e.error),
					Err(frame_system::Error::<Runtime>::CallFiltered.into()),
					"at {stage:?}"
				);
				assert!(remark.clone().dispatch(RuntimeOrigin::root()).is_ok(), "at {stage:?}");
			});
		}
	}

	#[test]
	fn inbound_teleports_are_refused_once_the_migration_starts() {
		let asset_hub = Location::new(0, [Parachain(ASSET_HUB_ID)]);

		// GIVEN the migration has not started. THEN a teleport from Asset Hub lands.
		for stage in stages_before_start() {
			assert_eq!(execute_from(&stage, asset_hub.clone(), teleport()), Ok(()), "at {stage:?}");
		}

		// GIVEN the migration has started. THEN the same teleport is refused as untrusted, also
		// after it is done.
		for stage in stages_from_start() {
			assert_eq!(
				execute_from(&stage, asset_hub.clone(), teleport()),
				Err(XcmError::UntrustedTeleportLocation),
				"at {stage:?}"
			);
		}
	}

	/// Execute `message` from `origin`, with `origin`'s account on this chain funded so a paid
	/// message can complete.
	fn execute_from(
		stage: &Stage,
		origin: Location,
		message: Xcm<RuntimeCall>,
	) -> Result<(), XcmError> {
		at(stage, || {
			let account = SovereignAccountOf::convert_location(&origin).expect("origin converts");
			<Balances as Mutate<AccountId>>::mint_into(&account, 100 * UNITS).unwrap();
			XcmExecutor::<crate::xcm_config::XcmConfig>::prepare_and_execute(
				origin,
				message,
				&mut [0u8; 32],
				Weight::MAX,
				Weight::zero(),
			)
			.ensure_complete()
			.map_err(|e| e.error)
		})
	}

	fn unpaid() -> Xcm<RuntimeCall> {
		Xcm(vec![UnpaidExecution { weight_limit: Unlimited, check_origin: None }, ClearOrigin])
	}

	fn teleport() -> Xcm<RuntimeCall> {
		Xcm(vec![
			UnpaidExecution { weight_limit: Unlimited, check_origin: None },
			ReceiveTeleportedAsset((Here, 10 * UNITS).into()),
			DepositAsset {
				assets: AllCounted(1).into(),
				beneficiary: AccountId32 { network: None, id: ALICE.into() }.into(),
			},
		])
	}

	fn paid() -> Xcm<RuntimeCall> {
		Xcm(vec![
			WithdrawAsset((Here, 10 * UNITS).into()),
			BuyExecution { fees: (Here, UNITS).into(), weight_limit: Unlimited },
			DepositAsset {
				assets: AllCounted(1).into(),
				beneficiary: AccountId32 { network: None, id: ALICE.into() }.into(),
			},
		])
	}

	#[test]
	fn only_coretime_and_asset_hub_reach_this_chain_while_the_migration_runs() {
		let para = |id| Location::new(0, [Parachain(id)]);

		// GIVEN the migration has not started. THEN a para and any system chain get through.
		for stage in stages_before_start() {
			for (origin, message) in [
				(para(2000), paid()),
				(para(BRIDGE_HUB_ID), paid()),
				(para(BRIDGE_HUB_ID), unpaid()),
			] {
				assert_eq!(
					execute_from(&stage, origin.clone(), message),
					Ok(()),
					"{origin:?} at {stage:?}"
				);
			}
		}

		// GIVEN the migration has started. THEN every other system chain is refused, paid or
		// unpaid, also after it is done. A para is refused until it is done. The Coretime chain
		// and Asset Hub get through.
		for stage in stages_from_start() {
			for message in [paid(), unpaid()] {
				assert_eq!(
					execute_from(&stage, para(BRIDGE_HUB_ID), message.clone()),
					Err(XcmError::Barrier),
					"BridgeHub at {stage:?}"
				);
				for origin in [para(BROKER_ID), para(ASSET_HUB_ID)] {
					assert_eq!(
						execute_from(&stage, origin.clone(), message.clone()),
						Ok(()),
						"{origin:?} at {stage:?}"
					);
				}
			}
			// A para pays its way in once the migration is done, as before it started.
			assert_eq!(
				execute_from(&stage, para(2000), paid()),
				if stage.is_finished() { Ok(()) } else { Err(XcmError::Barrier) },
				"a para at {stage:?}"
			);
		}
	}
}
