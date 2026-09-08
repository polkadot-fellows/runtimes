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

//! Test runtime for `pallet-rc2-migrator`.

use crate as pallet_rc2_migrator;
use frame_support::{derive_impl, parameter_types, traits::EnsureOrigin};
use sp_runtime::BuildStorage;
use xcm::prelude::*;

type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = u64;

frame_support::construct_runtime! {
	pub enum Test {
		System: frame_system,
		Rc2Migrator: pallet_rc2_migrator,
	}
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
}

/// The account the mock treats as the Coretime chain's dispatch origin.
pub const CORETIME: AccountId = 1005;
/// Any other account, to prove the origin check is not just "somebody".
pub const ALICE: AccountId = 1;

pub const CT_PARA_ID: u32 = 1005;
pub const COOL_OFF: u64 = 10;

parameter_types! {
	pub const CtParaId: u32 = CT_PARA_ID;
	pub const CoolOffPeriod: u64 = COOL_OFF;

	/// Every message the pallet successfully sent, in order.
	pub static SentXcm: Vec<(Location, Xcm<()>)> = vec![];
	/// Makes the router reject everything, to exercise the retry-next-block path.
	pub static SendFails: bool = false;
}

/// Records what the pallet sends instead of delivering it. `deliver` is where the recording
/// happens, so a message that fails validation is never observed as sent.
pub struct RecordingRouter;

impl SendXcm for RecordingRouter {
	type Ticket = (Location, Xcm<()>);

	fn validate(
		dest: &mut Option<Location>,
		msg: &mut Option<Xcm<()>>,
	) -> SendResult<Self::Ticket> {
		if SendFails::get() {
			return Err(SendError::Transport("send forced to fail"));
		}
		let ticket =
			(dest.take().expect("destination is set"), msg.take().expect("message is set"));
		Ok((ticket, Assets::new()))
	}

	fn deliver(ticket: Self::Ticket) -> Result<XcmHash, SendError> {
		let mut sent = SentXcm::get();
		sent.push(ticket);
		SentXcm::set(sent);
		Ok([0; 32])
	}
}

/// Accepts only the account standing in for the Coretime chain. Root is deliberately not
/// accepted: governance's way into the machine is `force_set_stage`, not a forged handshake.
pub struct EnsureCoretime;

impl EnsureOrigin<RuntimeOrigin> for EnsureCoretime {
	type Success = ();

	fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
		match o.clone().into() {
			Ok(frame_system::RawOrigin::Signed(CORETIME)) => Ok(()),
			_ => Err(o),
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
		Ok(RuntimeOrigin::signed(CORETIME))
	}
}

impl pallet_rc2_migrator::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type SendXcm = RecordingRouter;
	type CtParaId = CtParaId;
	type CtOrigin = EnsureCoretime;
	type CoolOffPeriod = CoolOffPeriod;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	SentXcm::set(vec![]);
	SendFails::set(false);

	let storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	let mut ext = sp_io::TestExternalities::new(storage);
	// Block 0 does not record events.
	ext.execute_with(|| System::set_block_number(1));
	ext
}

/// Run `Rc2Migrator::on_initialize` for the next `n` blocks.
pub fn run_blocks(n: u64) {
	for _ in 0..n {
		let now = System::block_number() + 1;
		System::set_block_number(now);
		<Rc2Migrator as frame_support::traits::OnInitialize<u64>>::on_initialize(now);
	}
}

/// The messages sent so far, destination and all.
pub fn sent() -> Vec<(Location, Xcm<()>)> {
	SentXcm::get()
}

/// Decode the `Transact` payload of the `n`th sent message as a Coretime runtime call.
pub fn sent_call(n: usize) -> crate::CtRuntimeCall {
	use codec::Decode;
	let (_, Xcm(instructions)) = sent().get(n).expect("message was sent").clone();
	for instruction in instructions {
		if let Instruction::Transact { call, .. } = instruction {
			return crate::CtRuntimeCall::decode(&mut &call.into_encoded()[..])
				.expect("payload decodes as a Coretime call");
		}
	}
	panic!("message {n} carried no Transact");
}
