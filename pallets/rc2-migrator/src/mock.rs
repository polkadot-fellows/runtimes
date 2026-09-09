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

//! Test runtime for `pallet-rc2-migrator`.

use crate as pallet_rc2_migrator;
use frame_support::{
	derive_impl, ord_parameter_types, parameter_types,
	traits::{OnInitialize, Time},
};
use frame_system::EnsureSignedBy;
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
/// Relay-chain block time, so the mock clock advances the way a real one does.
pub const BLOCK_TIME_MS: u64 = 6_000;

parameter_types! {
	pub const CtParaId: u32 = CT_PARA_ID;
	pub const CoolOffPeriod: u64 = COOL_OFF;

	/// Every message the pallet successfully sent, in order.
	pub static SentXcm: Vec<(Location, Xcm<()>)> = vec![];
	/// Makes the router reject everything, to exercise the retry-next-block path.
	pub static SendFails: bool = false;

	/// The mock wall clock, in milliseconds. Advanced by [`run_blocks`].
	pub static MockNow: u64 = BLOCK_TIME_MS;
}

/// Stands in for `pallet_timestamp`, which the pallet only needs through [`Time`].
pub struct MockTime;

impl Time for MockTime {
	type Moment = u64;

	fn now() -> Self::Moment {
		MockNow::get()
	}
}

/// Records what the pallet sends instead of delivering it.
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

ord_parameter_types! {
	pub const CoretimeAccount: AccountId = CORETIME;
}

impl pallet_rc2_migrator::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type SendXcm = RecordingRouter;
	type CtParaId = CtParaId;
	type TimeProvider = MockTime;
	type CtOrigin = EnsureSignedBy<CoretimeAccount, AccountId>;
	type CoolOffPeriod = CoolOffPeriod;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	SentXcm::set(vec![]);
	SendFails::set(false);
	MockNow::set(BLOCK_TIME_MS);

	let storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	let mut ext = sp_io::TestExternalities::new(storage);
	// Block 0 does not record events.
	ext.execute_with(|| System::set_block_number(1));
	ext
}

/// Run the next `n` blocks, advancing the clock like a real chain does.
///
/// The clock moves *after* the block's hooks, mirroring the timestamp inherent, which is an
/// extrinsic and so runs after `on_initialize`.
pub fn run_blocks(n: u64) {
	for _ in 0..n {
		let now = System::block_number() + 1;
		System::set_block_number(now);
		<Rc2Migrator as OnInitialize<u64>>::on_initialize(now);
		MockNow::set(now.saturating_mul(BLOCK_TIME_MS));
	}
}

/// The mock clock's current value.
pub fn now_ms() -> u64 {
	MockTime::now()
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
