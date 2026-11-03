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
//! - `pallet-hrmp-para` here, `pallet-hrmp-relay` there. HRMP itself stays on the relay chain; this
//!   chain only holds its deposits, as the relay chain asks.
//!
//! Requests go up as `Transact` with `OriginKind::Native`, so they land on the relay chain as
//! `Origin::Parachain(BROKER_ID)` — the origin its `ParaOrigin` accepts. The relay chain's messages
//! come back with `OriginKind::Superuser`, landing here as Root, which is what both pallets accept.

use crate::{
	xcm_config::LocationToAccountId, AccountId, Balance, Balances, Runtime, RuntimeHoldReason,
};
use alloc::{vec, vec::Vec};
use codec::Encode;
use cumulus_primitives_core::relay_chain;
use frame_support::{
	parameter_types,
	traits::{
		fungible::HoldConsideration, ConstantStoragePrice, EnsureOrigin, Get, LinearStoragePrice,
	},
};
use frame_system::EnsureRoot;
use polkadot_primitives::AbridgedHostConfiguration;
use system_parachains_constants::kusama::currency::{system_para_deposit, CENTS};
use xcm::latest::prelude::*;
use xcm_executor::traits::ConvertLocation;

parameter_types! {
	/// The relay chain, where every request goes.
	pub RelayLocation: Location = Location::parent();

	/// Mirrors the relay chain's `LOWEST_PUBLIC_ID`: ids below it are system chains, and this
	/// chain never hands one out.
	pub const FirstPublicParaId: u32 = 2_000;

	/// Mirrors the relay chain's `MIN_CODE_SIZE`, used to fail early rather than spend a round
	/// trip on a request the relay chain would refuse. The relay chain still checks the real thing.
	pub const MinCodeSize: u32 = 9;

	/// How long a manager waits before a request counts as gone quiet. In *relay-chain* blocks,
	/// since that is what `BlockNumberProvider` measures here — so it keeps its meaning through a
	/// stall in this chain's own block production. Two hours.
	pub const PendingDeadline: relay_chain::BlockNumber = 1_200;

	/// A para id reservation, priced the way the relay chain's registrar priced it, at this
	/// chain's rates.
	pub const ParaDeposit: Balance = 40 * CENTS;
	/// Per byte of head data plus the largest validation code the relay chain accepts.
	pub const DataDepositPerByte: Balance = system_para_deposit(0, 1);

	pub const ParaIdReservationHoldReason: RuntimeHoldReason =
		RuntimeHoldReason::RegistrarPara(pallet_registrar_para::HoldReason::ParaIdReservation);
	pub const RegistrationHoldReason: RuntimeHoldReason =
		RuntimeHoldReason::RegistrarPara(pallet_registrar_para::HoldReason::Registration);
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

/// The relay chain's live configuration, as `parachain_system` last saw it.
///
/// The registrar prices a registration from it and refuses early what the relay chain would
/// refuse. `None` until this chain has processed its first relay-chain block.
pub struct RelayHostConfiguration;

impl Get<Option<AbridgedHostConfiguration>> for RelayHostConfiguration {
	fn get() -> Option<AbridgedHostConfiguration> {
		cumulus_pallet_parachain_system::HostConfiguration::<Runtime>::get()
	}
}

#[cfg(feature = "runtime-benchmarks")]
impl pallet_registrar_para::BenchmarkHelper for RelayHostConfiguration {
	fn set_host_configuration(config: AbridgedHostConfiguration) {
		cumulus_pallet_parachain_system::HostConfiguration::<Runtime>::put(config);
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
		match <crate::RuntimeOrigin as Into<
			Result<cumulus_pallet_xcm::Origin, crate::RuntimeOrigin>,
		>>::into(o.clone())
		{
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

/// The HRMP half of the transport: answers to deposit requests.
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
	// The relay chain reports with `OriginKind::Superuser`, which arrives here as Root.
	type RelayOrigin = EnsureRoot<AccountId>;
	type ParachainOrigin = EnsureSiblingPara;
	type ForceOrigin = EnsureRoot<AccountId>;
	type FirstPublicParaId = FirstPublicParaId;
	type MinCodeSize = MinCodeSize;
	type HostConfiguration = RelayHostConfiguration;
	type PendingDeadline = PendingDeadline;
	// Relay-chain blocks, so a deadline keeps its meaning through a stall here.
	type BlockNumberProvider = cumulus_pallet_parachain_system::RelaychainDataProvider<Runtime>;
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = RelayHostConfiguration;
}

impl pallet_hrmp_para::Config for Runtime {
	type Currency = Balances;
	type SendToRelay = HrmpRequestToRelay;
	// Deposits are held on the para's sovereign account here, which is where the migration lands
	// them.
	type SovereignAccountOf = SovereignAccountOf;
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}
