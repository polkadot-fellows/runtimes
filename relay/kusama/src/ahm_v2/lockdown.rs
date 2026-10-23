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

use crate::{parachains_paras, parachains_slashing, Runtime, RuntimeCall};
use frame_support::{
	traits::{Contains, ProcessMessageError},
	weights::Weight,
};
use kusama_runtime_constants::system_parachain::{ASSET_HUB_ID, BROKER_ID};
use pallet_rc2_migrator::RcMigrationStage;
use xcm::latest::{Instruction, Junction::Parachain, Location};
use xcm_executor::traits::{DenyExecution, Properties};

/// Refuses every inbound message once the migration starts, except from this chain, the Coretime
/// chain and Asset Hub.
pub struct DenyOnceMigrationStarts;
impl DenyExecution for DenyOnceMigrationStarts {
	fn deny_execution<RuntimeCall>(
		origin: &Location,
		_instructions: &mut [Instruction<RuntimeCall>],
		_max_weight: Weight,
		_properties: &mut Properties,
	) -> Result<(), ProcessMessageError> {
		let stage = RcMigrationStage::<Runtime>::get();
		if !stage.has_started() {
			Ok(())
		} else if stage.is_ongoing() {
			only_from_self_coretime_or_asset_hub(origin)
		} else {
			// TODO(ahm-v2): paras reach `HrmpRelay` and `RegistrarRelay` over XCM after the
			// migration; let that envelope through here.
			only_from_self_coretime_or_asset_hub(origin)
		}
	}
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
/// reports, calls the Coretime chain and Asset Hub send over XCM, and the migrator itself.
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
		// TODO(ahm-v2): the Fellowship and its preimages end with the migration here. Decide
		// whether it moves off this chain first.
		FellowshipCollective(..) => (OFF, OFF),
		FellowshipReferenda(..) => (OFF, OFF),
		Whitelist(..) => (OFF, OFF),
		Claims(..) => (OFF, OFF),
		Vesting(..) => (OFF, OFF),
		Society(..) => (OFF, OFF),
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
		// TODO(ahm-v2): HRMP stays on this chain, with its deposits held on the Coretime chain.
		// Decide what opens after the migration.
		Hrmp(..) => (OFF, OFF),
		ParasSlashing(..) => (OFF, OFF),
		OnDemandAssignmentProvider(..) => (OFF, OFF),
		// TODO(ahm-v2): the registrar moves to the Coretime chain. Paras reach it through
		// `RegistrarRelay::relay_request`, which joins this list closed (before and) during
		// the migration and open after.
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
	use codec::Encode;
	use frame_support::{
		assert_err, assert_noop, assert_ok,
		traits::{fungible::Mutate, Contains},
	};
	use kusama_runtime_constants::{
		currency::UNITS,
		system_parachain::{ASSET_HUB_ID, BRIDGE_HUB_ID, BROKER_ID},
	};
	use pallet_rc2_migrator::{
		test_utils::{every_call, CallName},
		MigrationStageOf, RcMigrationStage,
	};
	use sp_runtime::{traits::Dispatchable, DispatchError};
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
		("Rc2Migrator", "set_limits"),
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
		for name in ENABLED {
			assert!(calls.iter().any(|(n, _)| n == name), "{name:?} is not a call of this runtime");
		}

		for stage in stages_before_start().into_iter().chain(stages_from_start()) {
			at(&stage, || {
				for (name, call) in &calls {
					let expected = if stage.has_started() {
						// From the start, a call passes exactly when it is listed in `ENABLED`.
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
				assert_noop!(
					remark.clone().dispatch(RuntimeOrigin::signed(ALICE)).map_err(|e| e.error),
					frame_system::Error::<Runtime>::CallFiltered
				);
				assert_ok!(remark.clone().dispatch(RuntimeOrigin::root()));
			});
		}
	}

	#[test]
	fn a_transact_goes_through_the_call_filter_unless_it_is_root() {
		let filtered: MaybeErrorCode =
			DispatchError::from(frame_system::Error::<Runtime>::CallFiltered)
				.encode()
				.into();
		// The sender, how its call dispatches, and what the dispatch returns once the migration has
		// started. The Coretime chain dispatches as `parachains_origin`, which the filter applies
		// to. Asset Hub as `Superuser` is Root (`LocationAsSuperuser`), which skips it.
		let senders = [
			(BROKER_ID, OriginKind::Native, filtered),
			(ASSET_HUB_ID, OriginKind::Superuser, MaybeErrorCode::Success),
		];

		// GIVEN the migration has not started. THEN both remarks run.
		for stage in stages_before_start() {
			for (id, kind, _) in &senders {
				assert_ok!(execute_from(&stage, para(*id), remark(*kind, MaybeErrorCode::Success)));
			}
		}

		// GIVEN the migration has started. THEN the barrier still admits both, the call filter
		// refuses the Coretime chain's remark and Root's still runs.
		for stage in stages_from_start() {
			for (id, kind, status) in &senders {
				assert_ok!(execute_from(&stage, para(*id), remark(*kind, status.clone())));
			}
		}
	}

	#[test]
	fn inbound_teleports_are_refused_once_the_migration_starts() {
		let asset_hub = para(ASSET_HUB_ID);

		// GIVEN the migration has not started. THEN a teleport from Asset Hub lands.
		for stage in stages_before_start() {
			assert_ok!(execute_from(&stage, asset_hub.clone(), teleport()));
		}

		// GIVEN the migration has started. THEN the same teleport is refused as untrusted, also
		// after it is done.
		for stage in stages_from_start() {
			assert_err!(
				execute_from(&stage, asset_hub.clone(), teleport()),
				XcmError::UntrustedTeleportLocation
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
			assert_ok!(<Balances as Mutate<AccountId>>::mint_into(&account, 100 * UNITS));
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

	fn para(id: u32) -> Location {
		Location::new(0, [Parachain(id)])
	}

	/// A `System::remark` dispatched as `origin_kind`, followed by a check that its dispatch
	/// returned `status`, so a refused call fails the message instead of vanishing.
	fn remark(origin_kind: OriginKind, status: MaybeErrorCode) -> Xcm<RuntimeCall> {
		Xcm(vec![
			UnpaidExecution { weight_limit: Unlimited, check_origin: None },
			Transact {
				origin_kind,
				fallback_max_weight: None,
				call: RuntimeCall::System(frame_system::Call::remark { remark: vec![1] })
					.encode()
					.into(),
			},
			ExpectTransactStatus(status),
		])
	}

	#[test]
	fn only_coretime_and_asset_hub_reach_this_chain_once_the_migration_starts() {
		// GIVEN the migration has not started. THEN a para and any system chain get through.
		for stage in stages_before_start() {
			for (origin, message) in [
				(para(2000), paid()),
				(para(BRIDGE_HUB_ID), paid()),
				(para(BRIDGE_HUB_ID), unpaid()),
			] {
				assert_ok!(execute_from(&stage, origin, message));
			}
		}

		// GIVEN the migration has started. THEN a para and every other system chain are refused,
		// paid or unpaid, also after it is done. The Coretime chain and Asset Hub get through.
		for stage in stages_from_start() {
			for message in [paid(), unpaid()] {
				for origin in [para(2000), para(BRIDGE_HUB_ID)] {
					assert_err!(execute_from(&stage, origin, message.clone()), XcmError::Barrier);
				}
				for origin in [para(BROKER_ID), para(ASSET_HUB_ID)] {
					assert_ok!(execute_from(&stage, origin, message.clone()));
				}
			}
		}
	}
}
