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

//! The relay chain's half of the parachain control plane.
//!
//! After the Minimal Relay migration this chain accepts no signed origins, so the user-facing
//! registrar and HRMP calls all start on the Coretime chain. What stays here is the part only this
//! chain can do — onboarding, lifecycles, PVF pre-checks, message routing — reached through
//! `pallet-registrar-relay` and `pallet-hrmp-relay`.
//!
//! Requests arrive from exactly one origin: the Coretime chain, as an XCM `Transact` converted to
//! `parachains_origin::Origin::Parachain`. Reports go back as `Transact` with
//! `OriginKind::Superuser`, which Coretime converts to Root — the origin both para-side pallets
//! accept as their `RelayOrigin`.

use alloc::{vec, vec::Vec};
use crate::{parachains_origin, Hrmp, Registrar, Runtime, RuntimeEvent, RuntimeOrigin};
use codec::Encode;
use core::marker::PhantomData;
use frame_support::{
	parameter_types,
	traits::{Contains, EnsureOrigin, ProcessMessageError},
	weights::Weight,
};
use polkadot_runtime_constants::{currency::CENTS, system_parachain::BROKER_ID};
use sp_runtime::transaction_validity::TransactionPriority;
use xcm::latest::prelude::*;
use xcm_executor::traits::{Properties, ShouldExecute};

parameter_types! {
	/// Where reports are sent, and the only para allowed to drive the control plane.
	pub ControlPlaneLocation: Location = Location::new(0, [Parachain(BROKER_ID)]);

	/// The largest head data this chain will hold while a registration waits for its code.
	///
	/// The bound that matters is the **product** with `MaxPendingRegistrations`: this is
	/// relay-chain state that no deposit here pays for, and an entry only leaves when the code
	/// lands or Coretime cancels. 100 x 1 MiB is the real commitment. Nothing expires on its own;
	/// governance drops an abandoned entry with `force_drop_pending`.
	pub const RegistrarMaxHeadDataSize: u32 = polkadot_primitives::MAX_HEAD_DATA_SIZE;
	pub const RegistrarMaxCodeSize: u32 = polkadot_primitives::MAX_CODE_SIZE;
	pub const MaxPendingRegistrations: u32 = 100;

	/// Uploading a registration's validation code is unsigned and feeless, so it needs a priority
	/// of its own. Below the top of the range, so it cannot crowd out inherents.
	pub const RegistrarUnsignedPriority: TransactionPriority = TransactionPriority::MAX / 2;

	/// The execution budget a forwarded request carries, withdrawn from the asking para's
	/// sovereign account **on the Coretime chain** — written as Coretime sees the asset. Surplus
	/// is refunded to the same account, so this only needs to be comfortably above one call's
	/// fee; a para whose sovereign cannot cover it has its request fail over there.
	pub ForwardedRequestFee: Asset = (Location::parent(), 10 * CENTS).into();
}

/// Accepts Root, or the Coretime chain.
///
/// Same shape as [`crate::EnsureAssetHub`]: match the parachain origin the XCM converter produced
/// and check the id. Root is kept so governance kept a way in.
pub struct EnsureCoretime;

impl EnsureOrigin<RuntimeOrigin> for EnsureCoretime {
	type Success = ();

	fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
		match <RuntimeOrigin as Into<Result<parachains_origin::Origin, RuntimeOrigin>>>::into(
			o.clone(),
		) {
			Ok(parachains_origin::Origin::Parachain(id)) if id == BROKER_ID.into() => Ok(()),
			_ => Err(o),
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
		Ok(RuntimeOrigin::root())
	}
}

/// Calls on the Coretime chain, as this chain must encode them.
///
/// Audit: the outer indices are the pallet indices in the Coretime `construct_runtime!`, and the
/// inner ones are `#[pallet::call_index]` of `receive` in each pallet. Nothing here is checked by
/// the compiler, so `call_encoding` in the minimal-relay integration tests decodes what this chain
/// would send using the real Coretime `RuntimeCall` — which is why these types are public.
#[derive(Encode)]
pub enum CoretimeRuntimePallets {
	#[codec(index = 60)]
	RegistrarPara(RegistrarParaCalls),
	#[codec(index = 61)]
	HrmpPara(HrmpParaCalls),
}

#[derive(Encode)]
pub enum RegistrarParaCalls {
	/// A para asked this chain to deregister it; forwarded on its behalf.
	#[codec(index = 4)]
	Deregister(u32),
	#[codec(index = 0)]
	Receive(registrar_primitives::MessageToPara),
	/// A para asked this chain to lock it against its manager.
	#[codec(index = 6)]
	AddLock(u32),
	/// A para asked this chain to unlock it.
	#[codec(index = 7)]
	RemoveLock(u32),
	/// A para asked this chain to set its head data.
	#[codec(index = 9)]
	SetCurrentHead(u32, Vec<u8>),
}

#[derive(Encode)]
pub enum HrmpParaCalls {
	/// `(sender, recipient, max_capacity, max_message_size)`.
	#[codec(index = 0)]
	OpenChannel(u32, u32, u32, u32),
	/// `(sender, recipient)`.
	#[codec(index = 1)]
	AcceptOpenChannel(u32, u32),
	/// `(sender, recipient, initiator)`.
	#[codec(index = 2)]
	CloseChannel(u32, u32, u32),
	/// `(sender, recipient)`.
	#[codec(index = 3)]
	CancelOpenRequest(u32, u32),
	#[codec(index = 4)]
	Receive(hrmp_primitives::MessageToPara),
	/// `(sender, recipient)`.
	#[codec(index = 5)]
	EstablishSystemChannel(u32, u32),
}

/// A parachain acting for itself, whether or not it is a system chain.
///
/// The calls a parachain dispatches for itself accept this; every other call that accepts a
/// parachain keeps `EnsureParachain` and so stays system-chains-only. Two shapes arrive here:
///
/// - a **system** chain, as `parachains_origin::Origin::Parachain` — unchanged from today;
/// - any **other** parachain, as `pallet_registrar_relay::Origin::Para`, which the XCM origin
///   converter hands it precisely so that it reaches these calls and nothing else.
pub struct EnsureAnyParaSelf;

impl EnsureOrigin<RuntimeOrigin> for EnsureAnyParaSelf {
	type Success = polkadot_primitives::Id;

	fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
		// A non-system parachain, through the narrow origin the XCM converter hands it.
		let o =
			match <RuntimeOrigin as Into<Result<pallet_registrar_relay::Origin, RuntimeOrigin>>>::into(o) {
				Ok(pallet_registrar_relay::Origin::Para(id)) => return Ok(id.into()),
				Err(o) => o,
			};

		// A system chain, exactly as before.
		match <RuntimeOrigin as Into<Result<parachains_origin::Origin, RuntimeOrigin>>>::into(o) {
			Ok(parachains_origin::Origin::Parachain(id)) => Ok(id),
			Err(o) => Err(o),
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
		Err(())
	}
}

/// Forwards a parachain's own registrar and HRMP requests to the Coretime chain.
///
/// This is what keeps a parachain's encoded call unchanged after the control plane moves: the relay
/// chain keeps its para-facing calls, and in remote mode their bodies land here instead of touching
/// relay-chain state. The request travels **as the para**: this chain descends its own origin to
/// the asking para's, so Coretime sees the para itself and serves it through the same origin checks
/// a para arriving directly would meet. Descending is sound because this chain has authority over
/// its children's locations and knows exactly which para each request arrived from — and the system
/// chains trust it absolutely.
///
/// The work is **priced on Coretime**: the message withdraws [`ForwardedRequestFee`] from the
/// para's sovereign account there and buys its execution with it, surplus refunded. Nothing is paid
/// on this chain — a para's relay-chain sovereign is empty by design — which is why each para's
/// forwards are rationed by [`pallet_registrar_relay::Pallet::note_forwarded`].
/// Tells the control plane that a para has produced its first block.
///
/// The relay chain's whole part in the manager lock, and the one fact Coretime cannot observe for
/// itself. A para's first block is what decides that its manager may no longer change its code,
/// rewrite its head or deregister it — not its first coretime assignment, which would lock a para
/// whose genesis blob is wrong out of the very calls that would fix it, permanently, since only
/// the para or root can lift a lock and a para that never booted cannot ask.
///
/// Reports each para once and only for paras onboarded through `pallet-registrar-relay`: migrated
/// paras arrived on Coretime already live and already locked, so there is nothing left to tell it.
pub struct NoteFirstHeadToCoretime;

impl runtime_parachains::paras::OnNewHead for NoteFirstHeadToCoretime {
	fn on_new_head(id: polkadot_primitives::Id, _head: &polkadot_primitives::HeadData) -> Weight {
		pallet_registrar_relay::Pallet::<Runtime>::note_first_head(id.into())
	}
}

pub struct ForwardToCoretime;

impl ForwardToCoretime {
	/// Whether the control plane has taken over. Requests are applied locally until it has.
	///
	/// Keyed on the migration being *finished*, not merely started: Coretime's registry is empty
	/// until the migration hands it over, so forwarding earlier would record state there for paras
	/// it does not yet know about. The window in between is covered by the call filter, which blocks
	/// these calls outright while the migration runs — see `PostAhmFilter`.
	fn remote() -> bool {
		pallet_rc2_migrator::RcMigrationStage::<Runtime>::get().is_finished()
	}

	fn registrar(as_para: u32, call: RegistrarParaCalls) -> Result<(), ()> {
		forward_as_para(as_para, CoretimeRuntimePallets::RegistrarPara(call).encode())
	}

	fn hrmp(as_para: u32, call: HrmpParaCalls) -> Result<(), ()> {
		forward_as_para(as_para, CoretimeRuntimePallets::HrmpPara(call).encode())
	}
}

impl registrar_primitives::ParaRequestRouter for ForwardToCoretime {
	fn is_remote() -> bool {
		Self::remote()
	}

	fn deregister(para_id: u32) -> Result<(), ()> {
		Self::registrar(para_id, RegistrarParaCalls::Deregister(para_id))
	}

	fn add_lock(para_id: u32) -> Result<(), ()> {
		Self::registrar(para_id, RegistrarParaCalls::AddLock(para_id))
	}

	fn remove_lock(para_id: u32) -> Result<(), ()> {
		Self::registrar(para_id, RegistrarParaCalls::RemoveLock(para_id))
	}

	fn set_current_head(para_id: u32, head: Vec<u8>) -> Result<(), ()> {
		Self::registrar(para_id, RegistrarParaCalls::SetCurrentHead(para_id, head))
	}
}

impl hrmp_primitives::ParaRequestRouter for ForwardToCoretime {
	fn is_remote() -> bool {
		Self::remote()
	}

	fn open_channel(
		sender: u32,
		recipient: u32,
		max_capacity: u32,
		max_message_size: u32,
	) -> Result<(), ()> {
		Self::hrmp(
			sender,
			HrmpParaCalls::OpenChannel(sender, recipient, max_capacity, max_message_size),
		)
	}

	fn accept_open_channel(sender: u32, recipient: u32) -> Result<(), ()> {
		Self::hrmp(recipient, HrmpParaCalls::AcceptOpenChannel(sender, recipient))
	}

	// The initiator travels explicitly. Either end may close, so this is not about authority — it
	// is about Coretime and then the relay chain recording *which* para asked, which only this
	// chain knows: it is the para origin the call arrived with.
	fn close_channel(initiator: u32, channel: hrmp_primitives::ChannelId) -> Result<(), ()> {
		Self::hrmp(
			initiator,
			HrmpParaCalls::CloseChannel(channel.sender, channel.recipient, initiator),
		)
	}

	fn cancel_open_request(sender: u32, channel: hrmp_primitives::ChannelId) -> Result<(), ()> {
		Self::hrmp(sender, HrmpParaCalls::CancelOpenRequest(channel.sender, channel.recipient))
	}

	fn establish_channel_with_system(sender: u32, target: u32) -> Result<(), ()> {
		Self::hrmp(sender, HrmpParaCalls::EstablishSystemChannel(sender, target))
	}
}

/// Forward one of `as_para`'s own requests to the Coretime chain, as that para, at its expense.
///
/// The message descends this chain's origin to the para's, so Coretime's origin converter hands
/// the dispatch the para's own origin — the same one it would get arriving directly — and the
/// execution is bought with [`ForwardedRequestFee`] withdrawn from the para's sovereign account
/// there, surplus refunded to it. `Err(())` (the para is over its forward budget, or the transport
/// refused) surfaces to the caller as `RequestNotForwarded`, with nothing written anywhere.
fn forward_as_para(as_para: u32, call: Vec<u8>) -> Result<(), ()> {
	if !pallet_registrar_relay::Pallet::<Runtime>::note_forwarded(as_para) {
		return Err(());
	}

	let fee = ForwardedRequestFee::get();
	let message = Xcm(vec![
		Instruction::DescendOrigin(Parachain(as_para).into()),
		Instruction::WithdrawAsset(fee.clone().into()),
		Instruction::BuyExecution { fees: fee, weight_limit: WeightLimit::Unlimited },
		Instruction::Transact {
			origin_kind: OriginKind::Native,
			fallback_max_weight: None,
			call: call.into(),
		},
		Instruction::RefundSurplus,
		Instruction::DepositAsset {
			assets: AssetFilter::Wild(WildAsset::All),
			beneficiary: Location::new(1, [Parachain(as_para)]),
		},
	]);

	send_xcm::<crate::xcm_config::XcmRouter>(ControlPlaneLocation::get(), message)
		.map(|_| ())
		.map_err(|_| ())
}

/// Admits, without payment, exactly the message a parachain sends to reach its forwarded calls.
///
/// After the migration a parachain has no funds here — emptying its relay-chain sovereign account
/// is one of the migration's goals — so the paid door is closed to it in practice. What opens
/// instead is deliberately narrow: the origin must be a child parachain, and the message must be
/// `UnpaidExecution` followed by a single `Transact` with `OriginKind::Native` (a trailing
/// `SetTopic` is tolerated, since routers append one).
///
/// `Native` is the containment. A non-system para's native origin is the narrow
/// `pallet_registrar_relay::Origin::Para`, which only the forwarding calls accept — so the program
/// admitted here reaches those calls and nothing else. In particular
/// `OriginKind::SovereignAccount` is not admitted, so free execution can never reach signed calls.
/// The forwarded work itself is priced on the Coretime chain (see [`forward_as_para`]), and
/// rationed per para per block (see `pallet_registrar_relay::Pallet::note_forwarded`).
pub struct AllowUnpaidParaControlFrom<T>(PhantomData<T>);

impl<T: Contains<Location>> ShouldExecute for AllowUnpaidParaControlFrom<T> {
	fn should_execute<Call>(
		origin: &Location,
		instructions: &mut [Instruction<Call>],
		max_weight: Weight,
		_properties: &mut Properties,
	) -> Result<(), ProcessMessageError> {
		if !T::contains(origin) {
			return Err(ProcessMessageError::Unsupported);
		}
		// Nothing changes until the control plane has actually moved: before the migration these
		// calls act locally and a para pays its way in as it always has, and during it they are
		// filtered — free execution for a local call would be a behaviour change with no upside.
		if !pallet_rc2_migrator::RcMigrationStage::<Runtime>::get().is_finished() {
			return Err(ProcessMessageError::Unsupported);
		}
		let unpaid_covers = |weight_limit: &WeightLimit| match weight_limit {
			WeightLimit::Unlimited => true,
			WeightLimit::Limited(limit) => limit.all_gte(max_weight),
		};
		match instructions {
			[UnpaidExecution { weight_limit, .. }, Transact { origin_kind: OriginKind::Native, .. }] |
			[UnpaidExecution { weight_limit, .. }, Transact { origin_kind: OriginKind::Native, .. }, SetTopic(_)]
				if unpaid_covers(weight_limit) =>
				Ok(()),
			_ => Err(ProcessMessageError::Unsupported),
		}
	}
}

/// Hand a `Transact` to the Coretime chain.
///
/// `OriginKind::Superuser` so it lands there as Root, which is what both para-side pallets accept.
/// A send failure is returned, not raised: for a report the right answer is always to carry on,
/// because this chain's own state is already correct and the parachain has its own way to ask
/// again.
fn send_to_coretime(call: Vec<u8>) -> Result<(), ()> {
	let message = Xcm(vec![
		Instruction::UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
		Instruction::Transact {
			origin_kind: OriginKind::Superuser,
			fallback_max_weight: None,
			call: call.into(),
		},
	]);

	send_xcm::<crate::xcm_config::XcmRouter>(ControlPlaneLocation::get(), message)
		.map(|_| ())
		.map_err(|_| ())
}

/// The registrar half of the transport.
pub struct RegistrarReportToCoretime;

impl pallet_registrar_relay::SendToPara for RegistrarReportToCoretime {
	fn send(message: registrar_primitives::MessageToPara) -> Result<(), ()> {
		send_to_coretime(
			CoretimeRuntimePallets::RegistrarPara(RegistrarParaCalls::Receive(message)).encode(),
		)
	}
}

/// The HRMP half of the transport.
pub struct HrmpReportToCoretime;

impl pallet_hrmp_relay::SendToPara for HrmpReportToCoretime {
	fn send(message: hrmp_primitives::MessageToPara) -> Result<(), ()> {
		send_to_coretime(
			CoretimeRuntimePallets::HrmpPara(HrmpParaCalls::Receive(message)).encode(),
		)
	}
}

impl pallet_registrar_relay::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ParaOrigin = EnsureCoretime;
	type SendToPara = RegistrarReportToCoretime;
	// The real registry. A registration driven from Coretime goes through the same `do_register`
	// a local one would, so it cannot bypass any rule this chain has — it only skips the deposit,
	// which is held on Coretime instead.
	type Registrar = Registrar;
	type MaxHeadDataSize = RegistrarMaxHeadDataSize;
	type MaxCodeSize = RegistrarMaxCodeSize;
	type MaxPendingRegistrations = MaxPendingRegistrations;
	type UnsignedPriority = RegistrarUnsignedPriority;
	// One forwarded request per para per block, across the registrar and HRMP together. These are
	// rare, human-paced calls; what is rationed is unpaid relay-chain execution.
	type MaxForwardsPerBlock = frame_support::traits::ConstU32<1>;
	type WeightInfo = ();
}

impl pallet_hrmp_relay::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ParaOrigin = EnsureCoretime;
	type SendToPara = HrmpReportToCoretime;
	// The real HRMP pallet, driven deposit-free: the deposits live on Coretime now, so reserving
	// here would charge a para twice, against a sovereign account the migration has emptied.
	type Registry = Hrmp;
	type WeightInfo = ();
}
