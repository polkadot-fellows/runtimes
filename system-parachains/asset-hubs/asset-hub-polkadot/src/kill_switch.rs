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
//! Anyone can pause one of the [`KillSwitchGroup`]s, or a single call, by putting up its deposit.
//! The Fellowship or Root can end the pause early and refund or slash it, or extend it. Only Root
//! can pause without a deposit. Nothing but the kill switch can pause or unpause through tx-pause.

use super::*;
use frame_support::traits::{
	Contains, EnsureOrigin, EnsureOriginWithArg, GetCallMetadata, NeverEnsureOrigin,
	PalletInfoAccess,
};
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

/// Pallets whose calls can never be paused:
///
/// - what keeps the chain running, and what governance and the Fellowship need to step in,
/// - what other chains call over XCM, like bridge status reports and People chain updates,
/// - what takes unsigned or authorized transactions. Those don't pay fees, so a paused one would
///   still get into blocks for free.
///
/// Wrappers like `Utility` are in here too. The call filter still checks every call inside them.
pub fn never_pausable_pallets() -> [&'static str; 28] {
	[
		System::name(),
		ParachainSystem::name(),
		Timestamp::name(),
		Session::name(),
		CollatorSelection::name(),
		XcmpQueue::name(),
		MessageQueue::name(),
		StakingRcClient::name(),
		MultiBlockElection::name(),
		MultiBlockElectionVerifier::name(),
		MultiBlockElectionUnsigned::name(),
		MultiBlockElectionSigned::name(),
		TxPause::name(),
		KillSwitch::name(),
		Preimage::name(),
		Scheduler::name(),
		Referenda::name(),
		ConvictionVoting::name(),
		Whitelist::name(),
		Utility::name(),
		Multisig::name(),
		Proxy::name(),
		Parameters::name(),
		ToKusamaXcmRouter::name(),
		MembersSubscriber::name(),
		AliasAccounts::name(),
		Pgas::name(),
		Claims::name(),
	]
}

/// Every call of [`never_pausable_pallets`], plus `PolkadotXcm::send` so governance and the
/// Fellowship can still send messages out.
pub struct NeverPausable;
impl Contains<RuntimeCallNameOf<Runtime>> for NeverPausable {
	fn contains((pallet, call): &RuntimeCallNameOf<Runtime>) -> bool {
		never_pausable_pallets()
			.iter()
			.any(|never| never.as_bytes() == pallet.as_slice()) ||
			(pallet.as_slice() == PolkadotXcm::name().as_bytes() && call.as_slice() == b"send")
	}
}

/// Groups of calls that can be paused together on Asset Hub. Any other call can still be paused on
/// its own.
///
/// tx-pause works on whole calls, so a group covers every asset those calls touch, not a single
/// asset. It only stops extrinsics and XCM `Transact`. Contracts going through the precompiles,
/// assets arriving over XCM, and swaps for paying fees in another asset aren't covered yet.
/// Neither are transfers of pool LP tokens (`PoolAssets`).
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
pub enum KillSwitchGroup {
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

impl KillSwitchGroup {
	/// The calls this group pauses, by pallet.
	fn calls(&self) -> Vec<(&'static str, &'static [&'static str])> {
		const EXECUTE: &[&str] = &["execute"];
		const PSM: &[&str] = &["mint", "redeem"];
		const SWAPS: &[&str] = &[
			"swap_exact_tokens_for_tokens",
			"swap_tokens_for_exact_tokens",
			"add_liquidity",
			"remove_liquidity",
		];
		const ASSET_TRANSFERS: &[&str] = &[
			"transfer",
			// The asset's admin can move anyone's balance with it.
			"force_transfer",
			"transfer_keep_alive",
			"transfer_approved",
			"transfer_all",
			"approve_transfer",
		];
		const XCM_TRANSFERS: &[&str] = &[
			"transfer_assets",
			"transfer_assets_using_type_and_then",
			"limited_reserve_transfer_assets",
			"limited_teleport_assets",
			"reserve_transfer_assets",
			"teleport_assets",
			"execute",
		];
		match self {
			Self::Psm => vec![(Psm::name(), PSM)],
			Self::Swaps => vec![(AssetConversion::name(), SWAPS), (PolkadotXcm::name(), EXECUTE)],
			Self::AssetTransfers => vec![
				(Assets::name(), ASSET_TRANSFERS),
				(ForeignAssets::name(), ASSET_TRANSFERS),
				(PolkadotXcm::name(), EXECUTE),
			],
			Self::XcmTransfers => vec![(PolkadotXcm::name(), XCM_TRANSFERS)],
		}
	}
}

/// Root can do everything. The Fellowship can end and extend pauses and settle the deposits of
/// pauses that ran out, but can't pause anything itself or touch a pause Root started.
pub struct KillSwitchAdmin;
impl<Target> EnsureOriginWithArg<RuntimeOrigin, AdminAction<Target>> for KillSwitchAdmin {
	type Success = ();

	fn try_origin(o: RuntimeOrigin, action: &AdminAction<Target>) -> Result<(), RuntimeOrigin> {
		let o = match <EnsureRoot<AccountId> as EnsureOrigin<_>>::try_origin(o) {
			Ok(_) => return Ok(()),
			Err(o) => o,
		};
		match action {
			AdminAction::ForcePause(_) => Err(o),
			AdminAction::Unpause(_) | AdminAction::Extend(_) | AdminAction::Settle(_) =>
				<EnsureXcm<IsFellowshipVoice<FellowshipLocation>> as EnsureOrigin<_>>::try_origin(o)
					.map(|_| ()),
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin(_: &AdminAction<Target>) -> Result<RuntimeOrigin, ()> {
		// The Fellowship's XCM origin can't be made up for benchmarks, and Root does the same work.
		Ok(RuntimeOrigin::root())
	}
}

/// The groups and what they cost to pause.
pub struct KillSwitchGroups;
impl pallet_kill_switch::PauseGroups<KillSwitchGroup, RuntimeCallNameOf<Runtime>, Balance>
	for KillSwitchGroups
{
	fn all() -> Vec<KillSwitchGroup> {
		use KillSwitchGroup::*;
		vec![Psm, Swaps, AssetTransfers, XcmTransfers]
	}

	fn calls(group: &KillSwitchGroup) -> Vec<RuntimeCallNameOf<Runtime>> {
		group
			.calls()
			.into_iter()
			.flat_map(|(pallet, calls)| calls.iter().map(move |call| (pallet, *call)))
			.map(|(pallet, call)| {
				Some((
					pallet.as_bytes().to_vec().try_into().ok()?,
					call.as_bytes().to_vec().try_into().ok()?,
				))
			})
			// A name longer than `MaxNameLen` leaves the group empty, so `integrity_test` fails
			// instead of the group quietly pausing less.
			.collect::<Option<_>>()
			.unwrap_or_default()
	}

	fn deposit(group: &KillSwitchGroup) -> Option<Balance> {
		use dynamic_params::kill_switch::*;
		Some(match group {
			KillSwitchGroup::Psm => PsmDeposit::get(),
			KillSwitchGroup::Swaps => SwapsDeposit::get(),
			KillSwitchGroup::AssetTransfers => AssetTransfersDeposit::get(),
			KillSwitchGroup::XcmTransfers => XcmTransfersDeposit::get(),
		})
	}
}

/// Every call that exists in the runtime. tx-pause still refuses the [`NeverPausable`] ones.
pub struct ExistingCall;
impl Contains<RuntimeCallNameOf<Runtime>> for ExistingCall {
	fn contains((pallet, call): &RuntimeCallNameOf<Runtime>) -> bool {
		let (Ok(pallet), Ok(call)) = (core::str::from_utf8(pallet), core::str::from_utf8(call))
		else {
			return false;
		};
		// `get_call_names` panics for a pallet that doesn't exist.
		RuntimeCall::get_module_names().contains(&pallet) &&
			RuntimeCall::get_call_names(pallet).contains(&call)
	}
}

/// Sets the deposits we'd suggest turning the kill switch on with: 10k DOT for the PSM, swaps and
/// single calls, 50k DOT for asset and XCM transfers, which hit every asset or everything leaving
/// the chain. On chain this is a governance call, this is for tests and benchmarks.
#[cfg(any(feature = "std", feature = "runtime-benchmarks"))]
pub fn set_suggested_deposits() {
	use dynamic_params::kill_switch::{
		AssetTransfersDeposit, CallDeposit, Parameters as P, PsmDeposit, SwapsDeposit,
		XcmTransfersDeposit,
	};
	for param in [
		P::PsmDeposit(PsmDeposit, Some(10_000 * UNITS)),
		P::SwapsDeposit(SwapsDeposit, Some(10_000 * UNITS)),
		P::AssetTransfersDeposit(AssetTransfersDeposit, Some(50_000 * UNITS)),
		P::XcmTransfersDeposit(XcmTransfersDeposit, Some(50_000 * UNITS)),
		P::CallDeposit(CallDeposit, Some(10_000 * UNITS)),
	] {
		pallet_parameters::Pallet::<Runtime>::set_parameter(
			RuntimeOrigin::root(),
			RuntimeParameters::KillSwitch(param),
		)
		.expect("root can set parameters");
	}
}

#[cfg(feature = "runtime-benchmarks")]
pub struct KillSwitchBenchmarkHelper;
#[cfg(feature = "runtime-benchmarks")]
impl pallet_kill_switch::BenchmarkHelper<KillSwitchGroup> for KillSwitchBenchmarkHelper {
	fn group() -> KillSwitchGroup {
		// The one with the most calls.
		KillSwitchGroup::AssetTransfers
	}

	fn enable() {
		set_suggested_deposits();
	}
}

impl pallet_kill_switch::Config for Runtime {
	type Group = KillSwitchGroup;
	type CallName = RuntimeCallNameOf<Runtime>;
	type Groups = KillSwitchGroups;
	type ValidCall = ExistingCall;
	type CallDeposit = dynamic_params::kill_switch::CallDeposit;
	type TransactionPause = TxPause;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	// Like every other slash on this chain.
	type OnSlash = Dap;
	type AdminOrigin = KillSwitchAdmin;
	type BlockNumberProvider = RelaychainDataProvider<Runtime>;
	type PauseDuration = dynamic_params::kill_switch::PauseDuration;
	type MaxCallsPerTarget = ConstU32<16>;
	type MaxDepositsPerPause = ConstU32<10>;
	// The 4 groups, the rest is for single calls.
	type MaxActivePauses = ConstU32<64>;
	type WeightInfo = weights::pallet_kill_switch::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = KillSwitchBenchmarkHelper;
}

#[cfg(test)]
mod tests {
	use super::*;
	use frame_support::traits::GetCallMetadata;
	use pallet_kill_switch::PauseGroups;

	#[test]
	fn every_group_call_exists() {
		for group in KillSwitchGroups::all() {
			for (pallet, calls) in group.calls() {
				let known = RuntimeCall::get_call_names(pallet);
				for call in calls {
					assert!(known.contains(call), "{pallet}::{call} doesn't exist");
				}
			}
			// None got dropped for being too long.
			let expected: usize = group.calls().iter().map(|(_, calls)| calls.len()).sum();
			assert_eq!(KillSwitchGroups::calls(&group).len(), expected);
			assert!(KillSwitchGroups::calls(&group).iter().all(ExistingCall::contains));
		}
	}

	#[test]
	fn no_group_touches_a_whitelisted_call() {
		for group in KillSwitchGroups::all() {
			for call in KillSwitchGroups::calls(&group) {
				assert!(!NeverPausable::contains(&call), "{group:?} would pause {call:?}");
			}
		}
	}

	/// A new pallet has to go here or into [`never_pausable_pallets`], so nobody adds one whose
	/// calls anyone can pause without thinking about it.
	#[test]
	fn every_pallet_is_pausable_or_not() {
		let pausable = [
			ParachainInfo::name(),
			WeightReclaim::name(),
			MultiBlockMigrations::name(),
			Balances::name(),
			TransactionPayment::name(),
			AssetTxPayment::name(),
			Vesting::name(),
			Dap::name(),
			Authorship::name(),
			Aura::name(),
			AuraExt::name(),
			PolkadotXcm::name(),
			CumulusXcm::name(),
			SnowbridgeSystemFrontend::name(),
			Indices::name(),
			VerifySignature::name(),
			Assets::name(),
			Uniques::name(),
			Nfts::name(),
			ForeignAssets::name(),
			PoolAssets::name(),
			AssetConversion::name(),
			Psm::name(),
			AssetsFreezer::name(),
			AssetsHolder::name(),
			Treasury::name(),
			Origins::name(),
			Bounties::name(),
			ChildBounties::name(),
			AssetRate::name(),
			MultiAssetBounties::name(),
			NominationPools::name(),
			VoterList::name(),
			DelegatedStaking::name(),
			Staking::name(),
			Revive::name(),
			AssetsPrecompiles::name(),
			AssetsPrecompilesPermit::name(),
			VestingPrecompiles::name(),
			DotnsGateway::name(),
			OriginRestriction::name(),
			NetworkSuffix::name(),
			PgasAllowance::name(),
			AhOps::name(),
		];
		let never = never_pausable_pallets();
		for pallet in RuntimeCall::get_module_names() {
			let listed = pausable.contains(pallet) as u8 + never.contains(pallet) as u8;
			assert_eq!(listed, 1, "{pallet} must be either pausable or never pausable");
		}
	}

	#[test]
	fn never_pausable_send_exists() {
		assert!(RuntimeCall::get_call_names(PolkadotXcm::name()).contains(&"send"));
	}

	#[test]
	fn existing_call_only_takes_real_calls() {
		let name = |pallet: &str, call: &str| -> RuntimeCallNameOf<Runtime> {
			(
				pallet.as_bytes().to_vec().try_into().unwrap(),
				call.as_bytes().to_vec().try_into().unwrap(),
			)
		};
		assert!(ExistingCall::contains(&name("Balances", "transfer_allow_death")));
		assert!(!ExistingCall::contains(&name("Balances", "no_such_call")));
		// Doesn't panic for a pallet that doesn't exist.
		assert!(!ExistingCall::contains(&name("NoSuchPallet", "transfer")));
		assert!(!ExistingCall::contains(&(
			vec![0xff].try_into().unwrap(),
			vec![].try_into().unwrap()
		)));
	}
}
