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

//! The Coretime chain's half of the parachain control plane.
//!
//! After the Minimal Relay migration this chain is where a parachain's life is managed from: it
//! hands out para ids, holds every deposit, records who manages what, and drives the relay chain
//! over XCM. The relay chain keeps only the things it alone can do.
//!
//! Two pairs of pallets, one per concern:
//!
//! - `pallet-registrar-para` here, `pallet-registrar-relay` there.
//! - `pallet-hrmp-para` here, `pallet-hrmp-relay` there.
//!
//! Requests go up as `Transact` with `OriginKind::Native`, so they land on the relay chain as
//! `Origin::Parachain(BROKER_ID)` — the origin its `ParaOrigin` accepts. Verdicts come back with
//! `OriginKind::Superuser`, landing here as Root, which is what `RelayOrigin` accepts.

use alloc::{vec, vec::Vec};
use crate::{
	xcm_config::LocationToAccountId, AccountId, Balance, Balances, Runtime, RuntimeEvent,
	RuntimeHoldReason,
};
use codec::Encode;
use cumulus_primitives_core::relay_chain;
use frame_support::{
	parameter_types,
	traits::{
		fungible::HoldConsideration, ConstBool, ConstU32, ConstantStoragePrice, Contains,
		EnsureOrigin, LinearStoragePrice,
	},
};
use frame_system::EnsureRoot;
use pallet_broker::CoreAssignment;
use kusama_runtime_constants::system_parachain::BROKER_ID;
use system_parachains_constants::kusama::currency::{system_para_deposit, CENTS};
use xcm::latest::prelude::*;
use xcm_executor::traits::ConvertLocation;

parameter_types! {
	/// The relay chain, where every request goes.
	pub RelayLocation: Location = Location::parent();

	/// This chain's own para id, naming it as one end of a system channel.
	pub const SelfParaId: u32 = BROKER_ID;

	/// Mirrors the relay chain's `LOWEST_PUBLIC_ID`: ids below it are system chains, and this
	/// chain never hands one out.
	pub const FirstPublicParaId: u32 = 2_000;

	/// Local mirrors of the relay chain's live configuration, used to fail early rather than
	/// spend a round trip on a request the relay chain would refuse. The relay chain still checks
	/// the real thing.
	pub const MinCodeSize: u32 = 9;
	pub const MaxCodeSize: u32 = polkadot_primitives::MAX_CODE_SIZE;
	pub const MaxHeadDataSize: u32 = polkadot_primitives::MAX_HEAD_DATA_SIZE;
	pub const MaxHrmpCapacity: u32 = 1_000;
	pub const MaxHrmpMessageSize: u32 = 102_400;

	/// How long a manager waits before a request counts as gone quiet. In *relay-chain* blocks,
	/// since that is what `BlockNumberProvider` measures here — so it keeps its meaning through a
	/// stall in this chain's own block production. Two hours.
	pub const PendingDeadline: relay_chain::BlockNumber = 1_200;

	/// A para id reservation, priced the way the relay chain's registrar priced it, at this
	/// chain's rates.
	pub const ParaDeposit: Balance = 40 * CENTS;
	/// Per byte of head data plus the largest validation code the relay chain accepts.
	pub const DataDepositPerByte: Balance = system_para_deposit(0, 1);
	/// One end of an HRMP channel.
	pub const HrmpChannelDeposit: Balance = 10 * CENTS;
	/// What it costs to skip the rest of a para's upgrade cooldown. Burned, not held.
	pub const UpgradeCooldownCost: Balance = 100 * CENTS;

	pub const ParaIdReservationHoldReason: RuntimeHoldReason =
		RuntimeHoldReason::RegistrarPara(pallet_registrar_para::HoldReason::ParaIdReservation);
	pub const RegistrationHoldReason: RuntimeHoldReason =
		RuntimeHoldReason::RegistrarPara(pallet_registrar_para::HoldReason::Registration);
	pub const HrmpChannelHoldReason: RuntimeHoldReason =
		RuntimeHoldReason::HrmpPara(pallet_hrmp_para::HoldReason::Channel);
}

/// Which paras this chain treats as system chains.
///
/// Two things hang off it: a channel with or amongst system chains is deposit-free, and a para may
/// pair *itself* with a system chain without going through governance.
pub struct SystemParas;

impl Contains<u32> for SystemParas {
	fn contains(para_id: &u32) -> bool {
		*para_id < FirstPublicParaId::get()
	}
}

/// A para's sovereign account on this chain.
///
/// The same conversion XCM uses for a sibling, which is what makes a migrated HRMP deposit and a
/// freshly taken one land on the same account.
pub struct SovereignAccountOf;

impl sp_runtime::traits::Convert<u32, AccountId> for SovereignAccountOf {
	fn convert(para_id: u32) -> AccountId {
		LocationToAccountId::convert_location(&Location::new(1, [Parachain(para_id)]))
			.expect("a sibling parachain location always converts; qed")
	}
}

/// Whether a para still holds coretime, and therefore must not be pulled out from under itself.
///
/// This is what replaces the relay chain's lock-at-first-head. The relay chain locks a para at its
/// first block because that is the only "in use" signal it has; this chain hosts coretime, so it
/// can ask the question directly.
///
/// Two sources, because a para can hold a core two ways:
///
/// - a **legacy lease**, which names the task outright;
/// - a **workload**, the schedule a core is actually running, where the para appears as
///   `CoreAssignment::Task`.
///
/// Regions are deliberately *not* consulted. A region is owned by an account and confers nothing
/// until it is assigned to a task, at which point it shows up in the workload — so reading regions
/// would lock paras that merely have coretime bought on their behalf.
///
/// **Known limitation.** The workload scan is linear in the number of cores. That is bounded and
/// small today, and these are rare governance-ish calls, but a production version should keep a
/// reverse index rather than scanning. The relay chain is the real backstop either way: it refuses
/// to deregister anything that is not an idle parathread, whoever asks.
pub struct CoretimeAssignments;

impl pallet_registrar_para::AssignmentChecker for CoretimeAssignments {
	fn has_assignment(para_id: u32) -> bool {
		let leased = pallet_broker::Leases::<Runtime>::get()
			.iter()
			.any(|lease| lease.task == para_id);
		if leased {
			return true;
		}

		pallet_broker::Workload::<Runtime>::iter_values().any(|schedule| {
			schedule
				.iter()
				.any(|item| matches!(item.assignment, CoreAssignment::Task(task) if task == para_id))
		})
	}
}

/// Accepts Root, or a sibling parachain acting for itself.
///
/// This is what lets a parachain keep speaking for itself after the migration: it retargets its
/// `Transact` from the relay chain to here, and the XCM origin converter turns it into a sibling
/// location which this resolves back to a para id.
pub struct EnsureSiblingPara;

impl EnsureOrigin<crate::RuntimeOrigin> for EnsureSiblingPara {
	type Success = u32;

	fn try_origin(o: crate::RuntimeOrigin) -> Result<Self::Success, crate::RuntimeOrigin> {
		match <crate::RuntimeOrigin as Into<Result<cumulus_pallet_xcm::Origin, crate::RuntimeOrigin>>>::into(
			o.clone(),
		) {
			Ok(cumulus_pallet_xcm::Origin::SiblingParachain(id)) => Ok(id.into()),
			_ => Err(o),
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<crate::RuntimeOrigin, ()> {
		Err(())
	}
}

/// Calls on the relay chain, as this chain must encode them.
///
/// Audit: the outer indices are the pallet indices in the relay chain's `construct_runtime!`, the
/// inner ones the `#[pallet::call_index]` in each pallet. Nothing here is checked by the compiler,
/// so `call_encoding` in the minimal-relay integration tests decodes what this chain would send
/// using the real relay `RuntimeCall` — which is why these types are public.
#[derive(Encode)]
pub enum RelayRuntimePallets {
	#[codec(index = 250)]
	RegistrarRelay(RegistrarRelayCalls),
	#[codec(index = 251)]
	HrmpRelay(HrmpRelayCalls),
}

#[derive(Encode)]
pub enum RegistrarRelayCalls {
	/// `#[pallet::call_index]` of `receive` in `pallet-registrar-relay`: one entry point for
	/// every message variant, so this router needs no routing.
	#[codec(index = 0)]
	Receive(registrar_primitives::MessageToRelay<AccountId>),
}

#[derive(Encode)]
pub enum HrmpRelayCalls {
	/// `#[pallet::call_index]` of `receive` in `pallet-hrmp-relay`.
	#[codec(index = 0)]
	Receive(hrmp_primitives::MessageToRelay),
}

/// Hand a `Transact` to the relay chain.
///
/// `OriginKind::Native` so it arrives as `Origin::Parachain(BROKER_ID)` rather than as Root — the
/// relay pallets accept only that one parachain, which is the whole trust boundary.
fn send_to_relay(call: Vec<u8>) -> Result<(), ()> {
	let message = Xcm(vec![
		Instruction::UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
		Instruction::Transact {
			origin_kind: OriginKind::Native,
			fallback_max_weight: None,
			call: call.into(),
		},
	]);

	send_xcm::<crate::xcm_config::XcmRouter>(RelayLocation::get(), message)
		.map(|_| ())
		.map_err(|_| ())
}

/// The registrar half of the transport.
pub struct RegistrarRequestToRelay;

impl pallet_registrar_para::SendToRelay for RegistrarRequestToRelay {
	type AccountId = AccountId;

	fn send(message: registrar_primitives::MessageToRelay<AccountId>) -> Result<(), ()> {
		send_to_relay(
			RelayRuntimePallets::RegistrarRelay(RegistrarRelayCalls::Receive(message)).encode(),
		)
	}
}

/// The HRMP half of the transport.
pub struct HrmpRequestToRelay;

impl pallet_hrmp_para::SendToRelay for HrmpRequestToRelay {
	fn send(message: hrmp_primitives::MessageToRelay) -> Result<(), ()> {
		send_to_relay(RelayRuntimePallets::HrmpRelay(HrmpRelayCalls::Receive(message)).encode())
	}
}

impl pallet_registrar_para::Config for Runtime {
	type ReservationConsideration = HoldConsideration<
		AccountId,
		Balances,
		ParaIdReservationHoldReason,
		ConstantStoragePrice<ParaDeposit, Balance>,
	>;
	type RegistrationConsideration = HoldConsideration<
		AccountId,
		Balances,
		RegistrationHoldReason,
		LinearStoragePrice<frame_support::traits::ConstU128<0>, DataDepositPerByte, Balance>,
	>;
	type SendToRelay = RegistrarRequestToRelay;
	type AssignmentChecker = CoretimeAssignments;
	// This chain hosts coretime, so it must be able to tell when a para holds a core. The startup
	// check refuses `NoAssignments` here, which would otherwise leave every live parachain's
	// manager able to deregister it.
	type RequireAssignmentLock = ConstBool<true>;
	// The relay chain reports with `OriginKind::Superuser`, which arrives here as Root.
	type RelayOrigin = EnsureRoot<AccountId>;
	type ParachainOrigin = EnsureSiblingPara;
	type FirstPublicParaId = FirstPublicParaId;
	type MinCodeSize = MinCodeSize;
	type MaxCodeSize = MaxCodeSize;
	type MaxHeadDataSize = MaxHeadDataSize;
	type PendingDeadline = PendingDeadline;
	// Relay-chain blocks, so a deadline keeps its meaning through a stall here.
	type BlockNumberProvider = cumulus_pallet_parachain_system::RelaychainDataProvider<Runtime>;
	type Fungible = Balances;
	type UpgradeCooldownCost = UpgradeCooldownCost;
	type WeightInfo = ();
}

impl pallet_hrmp_para::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ChannelConsideration = HoldConsideration<
		AccountId,
		Balances,
		HrmpChannelHoldReason,
		ConstantStoragePrice<HrmpChannelDeposit, Balance>,
	>;
	type SendToRelay = HrmpRequestToRelay;
	type RelayOrigin = EnsureRoot<AccountId>;
	type ParachainOrigin = EnsureSiblingPara;
	// Deposits are held on the para's sovereign account here, not on whoever calls — which is
	// where the migration lands them.
	type SovereignAccountOf = SovereignAccountOf;
	type SelfParaId = SelfParaId;
	type SystemParas = SystemParas;
	type MaxCapacity = MaxHrmpCapacity;
	type MaxMessageSize = MaxHrmpMessageSize;
	type PendingDeadline = PendingDeadline;
	type BlockNumberProvider = cumulus_pallet_parachain_system::RelaychainDataProvider<Runtime>;
	type WeightInfo = ();
}

// Referenced only so the `ConstU32` import is used in every feature combination.
#[allow(unused)]
type _KeepImports = ConstU32<0>;
