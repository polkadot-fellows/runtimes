// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The kill switch and the tx-pause instance it drives.
//!
//! Anyone can pause one of the [`KillSwitchTarget`]s by putting up its deposit. The Fellowship or
//! Root can end the pause early and refund or slash it, or extend it. Only Root can pause without a
//! deposit. Nothing but the kill switch can pause or unpause through tx-pause.

use super::*;
use frame_support::traits::{Contains, EnsureOrigin, EnsureOriginWithArg, NeverEnsureOrigin};
use pallet_kill_switch::AdminAction;
use pallet_tx_pause::RuntimeCallNameOf;

parameter_types! {
	pub const MaxNameLen: u32 = 256;
}

impl pallet_tx_pause::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	// Only the kill switch pauses and unpauses calls.
	type PauseOrigin = NeverEnsureOrigin<()>;
	type UnpauseOrigin = NeverEnsureOrigin<()>;
	type WhitelistedCalls = NeverPausable;
	type MaxNameLen = MaxNameLen;
	// Its calls can't be dispatched, so the weights don't matter.
	type WeightInfo = pallet_tx_pause::weights::SubstrateWeight<Runtime>;
}

/// Pallets whose calls can never be paused, so the chain keeps running and governance and the
/// Fellowship can always step in.
///
/// Wrappers like `Utility` are in here too. The call filter still checks every call inside them.
pub const NEVER_PAUSABLE: &[&str] = &[
	"System",
	"ParachainSystem",
	"Timestamp",
	"Session",
	"CollatorSelection",
	"XcmpQueue",
	"MessageQueue",
	"StakingRcClient",
	"MultiBlockElection",
	"MultiBlockElectionVerifier",
	"MultiBlockElectionUnsigned",
	"MultiBlockElectionSigned",
	"TxPause",
	"KillSwitch",
	"Preimage",
	"Scheduler",
	"Referenda",
	"ConvictionVoting",
	"Whitelist",
	"Utility",
	"Multisig",
	"Proxy",
];

/// Everything in [`NEVER_PAUSABLE`], plus `PolkadotXcm::send` so governance and the Fellowship can
/// still send messages out.
pub struct NeverPausable;
impl Contains<RuntimeCallNameOf<Runtime>> for NeverPausable {
	fn contains((pallet, call): &RuntimeCallNameOf<Runtime>) -> bool {
		NEVER_PAUSABLE.iter().any(|never| never.as_bytes() == pallet.as_slice()) ||
			(pallet.as_slice() == b"PolkadotXcm" && call.as_slice() == b"send")
	}
}

/// What can be paused on Asset Hub.
///
/// tx-pause works on whole calls, so a target covers every asset those calls touch, not a single
/// asset. It only stops extrinsics and XCM `Transact`. Contracts going through the precompiles and
/// assets arriving over XCM aren't covered yet.
#[derive(
	Clone,
	Copy,
	PartialEq,
	Eq,
	Debug,
	Encode,
	Decode,
	DecodeWithMemTracking,
	scale_info::TypeInfo,
	MaxEncodedLen,
)]
pub enum KillSwitchTarget {
	/// Minting and redeeming through the PSM.
	#[codec(index = 0)]
	Psm,
	/// Swaps and liquidity changes in asset conversion pools. Also pauses `PolkadotXcm::execute`,
	/// since `ExchangeAsset` swaps through the same pools.
	#[codec(index = 1)]
	Swaps,
	/// Moving local and foreign assets. Also pauses `PolkadotXcm::execute`, since a local XCM can
	/// move them too.
	#[codec(index = 2)]
	AssetTransfers,
	/// Sending assets to other chains.
	#[codec(index = 3)]
	XcmTransfers,
}

impl KillSwitchTarget {
	/// The calls this target pauses, by pallet.
	fn calls(&self) -> &'static [(&'static str, &'static [&'static str])] {
		const ASSET_TRANSFERS: &[&str] = &[
			"transfer",
			"transfer_keep_alive",
			"transfer_approved",
			"transfer_all",
			"approve_transfer",
		];
		match self {
			Self::Psm => &[("Psm", &["mint", "redeem"])],
			Self::Swaps => &[
				(
					"AssetConversion",
					&[
						"swap_exact_tokens_for_tokens",
						"swap_tokens_for_exact_tokens",
						"add_liquidity",
						"remove_liquidity",
					],
				),
				("PolkadotXcm", &["execute"]),
			],
			Self::AssetTransfers => &[
				("Assets", ASSET_TRANSFERS),
				("ForeignAssets", ASSET_TRANSFERS),
				("PolkadotXcm", &["execute"]),
			],
			Self::XcmTransfers => &[(
				"PolkadotXcm",
				&[
					"transfer_assets",
					"transfer_assets_using_type_and_then",
					"limited_reserve_transfer_assets",
					"limited_teleport_assets",
					"reserve_transfer_assets",
					"teleport_assets",
					"execute",
				],
			)],
		}
	}
}

parameter_types! {
	pub const KillSwitchPauseDuration: BlockNumber = RC_DAYS;
	pub const KillSwitchCooldown: BlockNumber = 7 * RC_DAYS;
}

/// Root can do everything. The Fellowship can end and extend pauses, but can't pause anything
/// itself or touch a pause Root started.
pub struct KillSwitchAdmin;
impl EnsureOriginWithArg<RuntimeOrigin, AdminAction<KillSwitchTarget>> for KillSwitchAdmin {
	type Success = ();

	fn try_origin(
		o: RuntimeOrigin,
		action: &AdminAction<KillSwitchTarget>,
	) -> Result<(), RuntimeOrigin> {
		let o = match EnsureRoot::<AccountId>::try_origin(o) {
			Ok(_) => return Ok(()),
			Err(o) => o,
		};
		match action {
			AdminAction::ForcePause(_) => Err(o),
			AdminAction::Unpause(_) | AdminAction::Extend(_) =>
				EnsureXcm::<IsFellowshipVoice<FellowshipLocation>>::try_origin(o).map(|_| ()),
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin(_: &AdminAction<KillSwitchTarget>) -> Result<RuntimeOrigin, ()> {
		// The Fellowship's XCM origin can't be made up for benchmarks, and Root does the same work.
		Ok(RuntimeOrigin::root())
	}
}

/// The targets and what they cost to pause.
pub struct KillSwitchTargets;
impl pallet_kill_switch::PauseTargets<KillSwitchTarget, RuntimeCallNameOf<Runtime>, Balance>
	for KillSwitchTargets
{
	fn all() -> Vec<KillSwitchTarget> {
		use KillSwitchTarget::*;
		vec![Psm, Swaps, AssetTransfers, XcmTransfers]
	}

	fn calls(target: &KillSwitchTarget) -> Vec<RuntimeCallNameOf<Runtime>> {
		target
			.calls()
			.iter()
			.flat_map(|(pallet, calls)| calls.iter().map(move |call| (*pallet, *call)))
			// The names are all far below `MaxNameLen`, the tests check that none get lost here.
			.filter_map(|(pallet, call)| {
				Some((
					pallet.as_bytes().to_vec().try_into().ok()?,
					call.as_bytes().to_vec().try_into().ok()?,
				))
			})
			.collect()
	}

	fn deposit(target: &KillSwitchTarget) -> Option<Balance> {
		Some(match target {
			KillSwitchTarget::Psm | KillSwitchTarget::Swaps => 10_000 * UNITS,
			// These hit every asset, or everything leaving the chain.
			KillSwitchTarget::AssetTransfers | KillSwitchTarget::XcmTransfers => 50_000 * UNITS,
		})
	}
}

#[cfg(feature = "runtime-benchmarks")]
pub struct KillSwitchBenchmarkHelper;
#[cfg(feature = "runtime-benchmarks")]
impl pallet_kill_switch::BenchmarkHelper<KillSwitchTarget> for KillSwitchBenchmarkHelper {
	fn target() -> KillSwitchTarget {
		// The one with the most calls.
		KillSwitchTarget::AssetTransfers
	}
}

impl pallet_kill_switch::Config for Runtime {
	type Target = KillSwitchTarget;
	type CallName = RuntimeCallNameOf<Runtime>;
	type Targets = KillSwitchTargets;
	type TransactionPause = TxPause;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	// Like every other slash on this chain.
	type OnSlash = Dap;
	type AdminOrigin = KillSwitchAdmin;
	type BlockNumberProvider = RelaychainDataProvider<Runtime>;
	type PauseDuration = KillSwitchPauseDuration;
	type Cooldown = KillSwitchCooldown;
	type MaxCallsPerTarget = ConstU32<16>;
	type MaxDepositsPerPause = ConstU32<10>;
	type MaxActivePauses = ConstU32<8>;
	// Placeholder until the pallet is benchmarked on this chain.
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = KillSwitchBenchmarkHelper;
}

#[cfg(test)]
mod tests {
	use super::*;
	use frame_support::traits::{GetCallMetadata, PalletsInfoAccess};
	use pallet_kill_switch::PauseTargets;

	#[test]
	fn every_target_call_exists() {
		for target in KillSwitchTargets::all() {
			for (pallet, calls) in target.calls() {
				let known = RuntimeCall::get_call_names(pallet);
				for call in *calls {
					assert!(known.contains(call), "{pallet}::{call} doesn't exist");
				}
			}
			// None got dropped for being too long.
			let expected: usize = target.calls().iter().map(|(_, calls)| calls.len()).sum();
			assert_eq!(KillSwitchTargets::calls(&target).len(), expected);
		}
	}

	#[test]
	fn no_target_touches_a_whitelisted_call() {
		for target in KillSwitchTargets::all() {
			for call in KillSwitchTargets::calls(&target) {
				assert!(!NeverPausable::contains(&call), "{target:?} would pause {call:?}");
			}
		}
	}

	#[test]
	fn whitelisted_pallets_exist() {
		let pallets = <AllPalletsWithSystem as PalletsInfoAccess>::infos();
		for pallet in NEVER_PAUSABLE {
			assert!(
				pallets.iter().any(|info| info.name == *pallet),
				"{pallet} isn't in the runtime"
			);
		}
		assert!(RuntimeCall::get_call_names("PolkadotXcm").contains(&"send"));
	}
}
