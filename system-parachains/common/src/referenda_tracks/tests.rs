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
// along with Polkadot. If not, see <http://www.gnu.org/licenses/>.

//! Tests for [`RepairReferendaTracks`], on a mock runtime with the real referenda, scheduler and
//! preimage pallets.

use super::*;
use alloc::borrow::Cow;
use codec::{DecodeWithMemTracking, MaxEncodedLen};
use frame_support::{
	assert_ok, derive_impl, parameter_types,
	traits::{
		schedule::DispatchTime, ConstU32, ConstU64, EqualPrivilegeOnly, OnInitialize, OriginTrait,
		StorePreimage, VoteTally,
	},
};
use frame_system::EnsureRoot;
use pallet_referenda::{Curve, Track, TrackInfo};
use scale_info::TypeInfo;
use sp_runtime::{str_array as s, BuildStorage, Perbill, StateVersion};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		Preimage: pallet_preimage,
		Scheduler: pallet_scheduler,
		Referenda: pallet_referenda,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountData = pallet_balances::AccountData<u64>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type AccountStore = System;
}

impl pallet_preimage::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<u64>;
	type Consideration = ();
}

parameter_types! {
	pub MaxWeight: Weight = Weight::from_parts(2_000_000_000_000, u64::MAX);
}

impl pallet_scheduler::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaxWeight;
	type ScheduleOrigin = EnsureRoot<u64>;
	type MaxScheduledPerBlock = ConstU32<100>;
	type WeightInfo = ();
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type Preimages = Preimage;
	type BlockNumberProvider = System;
}

/// Prepare 2 blocks, decide for 20, so a referendum stays deciding for the whole test.
const fn track(id: u8, max_deciding: u32) -> Track<u8, u64, u64> {
	Track {
		id,
		info: TrackInfo {
			name: s("track"),
			max_deciding,
			decision_deposit: 10,
			prepare_period: 2,
			decision_period: 20,
			confirm_period: 2,
			min_enactment_period: 1,
			min_approval: Curve::LinearDecreasing {
				length: Perbill::from_percent(100),
				floor: Perbill::from_percent(50),
				ceil: Perbill::from_percent(100),
			},
			min_support: Curve::LinearDecreasing {
				length: Perbill::from_percent(100),
				floor: Perbill::from_percent(0),
				ceil: Perbill::from_percent(100),
			},
		},
	}
}

/// Track 0 has one slot, like Root. Track 1 has three.
pub struct TestTracks;
impl TracksInfo<u64, u64> for TestTracks {
	type Id = u8;
	type RuntimeOrigin = <RuntimeOrigin as OriginTrait>::PalletsOrigin;

	fn tracks() -> impl Iterator<Item = Cow<'static, Track<Self::Id, u64, u64>>> {
		static DATA: [Track<u8, u64, u64>; 2] = [track(0, 1), track(1, 3)];
		DATA.iter().map(Cow::Borrowed)
	}

	fn track_for(origin: &Self::RuntimeOrigin) -> Result<Self::Id, ()> {
		match frame_system::RawOrigin::try_from(origin.clone()) {
			Ok(frame_system::RawOrigin::Root) => Ok(0),
			Ok(frame_system::RawOrigin::None) => Ok(1),
			_ => Err(()),
		}
	}
}

impl pallet_referenda::Config for Test {
	type WeightInfo = ();
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type Scheduler = Scheduler;
	type Currency = Balances;
	type SubmitOrigin = frame_system::EnsureSigned<u64>;
	type CancelOrigin = EnsureRoot<u64>;
	type KillOrigin = EnsureRoot<u64>;
	type Slash = ();
	type Votes = u32;
	type Tally = Tally;
	type SubmissionDeposit = ConstU64<2>;
	type MaxQueued = ConstU32<10>;
	type UndecidingTimeout = ConstU64<100>;
	type AlarmInterval = ConstU64<1>;
	type Tracks = TestTracks;
	type Preimages = Preimage;
	type BlockNumberProvider = System;
}

#[derive(
	Encode, Debug, Decode, DecodeWithMemTracking, TypeInfo, Eq, PartialEq, Clone, MaxEncodedLen,
)]
pub struct Tally {
	pub ayes: u32,
	pub nays: u32,
}

impl<Class> VoteTally<u32, Class> for Tally {
	fn new(_: Class) -> Self {
		Self { ayes: 0, nays: 0 }
	}

	fn ayes(&self, _: Class) -> u32 {
		self.ayes
	}

	fn support(&self, _: Class) -> Perbill {
		Perbill::from_percent(self.ayes)
	}

	fn approval(&self, _: Class) -> Perbill {
		if self.ayes + self.nays > 0 {
			Perbill::from_rational(self.ayes, self.ayes + self.nays)
		} else {
			Perbill::zero()
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn unanimity(_: Class) -> Self {
		Self { ayes: 100, nays: 0 }
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn rejection(_: Class) -> Self {
		Self { ayes: 0, nays: 100 }
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn from_requirements(support: Perbill, approval: Perbill, _: Class) -> Self {
		let ayes = support.mul_ceil(100u32);
		let nays = ((ayes as u64) * 1_000_000_000u64 / approval.deconstruct() as u64) as u32 - ayes;
		Self { ayes, nays }
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn setup(_: Class, _: Perbill) {}
}

fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pallet_balances::GenesisConfig::<Test> {
		balances: (1..=6).map(|who| (who, 100)).collect(),
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

fn run_to(n: u64) {
	while System::block_number() < n {
		let next = System::block_number() + 1;
		System::set_block_number(next);
		Scheduler::on_initialize(next);
	}
}

/// Submit a referendum on `track` and place its decision deposit. It is ready to decide two
/// blocks after submission.
fn submit(track: u8) -> ReferendumIndex {
	let index = pallet_referenda::ReferendumCount::<Test>::get();
	let proposal_origin: OriginCaller = match track {
		0 => frame_system::RawOrigin::Root.into(),
		_ => frame_system::RawOrigin::None.into(),
	};
	let call = RuntimeCall::System(frame_system::Call::remark { remark: index.encode() });
	assert_ok!(Referenda::submit(
		RuntimeOrigin::signed(1),
		Box::new(proposal_origin),
		<Preimage as StorePreimage>::bound(call).unwrap(),
		DispatchTime::After(1),
	));
	assert_ok!(Referenda::place_decision_deposit(RuntimeOrigin::signed(2), index));
	index
}

/// What the Asset Hub Migration did to an ongoing referendum whose call it could not map: write
/// it as `Cancelled` without telling its track.
fn cancel_like_ahm(index: ReferendumIndex) {
	let Some(ReferendumInfo::Ongoing(status)) = ReferendumInfoFor::<Test>::get(index) else {
		panic!("referendum {index} is not ongoing");
	};
	ReferendumInfoFor::<Test>::insert(
		index,
		ReferendumInfo::Cancelled(
			System::block_number(),
			Some(status.submission_deposit),
			status.decision_deposit,
		),
	);
}

/// Run the migration with the try-runtime checks when they are compiled in.
fn migrate() {
	#[cfg(feature = "try-runtime")]
	let state = RepairReferendaTracks::<Test>::pre_upgrade().unwrap();
	RepairReferendaTracks::<Test>::on_runtime_upgrade();
	#[cfg(feature = "try-runtime")]
	RepairReferendaTracks::<Test>::post_upgrade(state).unwrap();
}

fn census(track: u8) -> TrackCensus {
	Census::<Test>::take().tracks.get(&track).copied().unwrap_or_default()
}

fn queue(track: u8) -> Vec<ReferendumIndex> {
	TrackQueue::<Test>::get(track).iter().map(|(index, _)| *index).collect()
}

fn is_deciding(index: ReferendumIndex) -> bool {
	matches!(
		ReferendumInfoFor::<Test>::get(index),
		Some(ReferendumInfo::Ongoing(status)) if status.deciding.is_some()
	)
}

fn root() -> Vec<u8> {
	sp_io::storage::root(StateVersion::V1)
}

#[test]
fn frees_a_phantom_slot() {
	new_test_ext().execute_with(|| {
		let a = submit(1);
		run_to(3);
		assert_eq!(census(1), TrackCensus { stored: 1, deciding: 1, pending: 0 });

		cancel_like_ahm(a);
		assert_eq!(census(1), TrackCensus { stored: 1, deciding: 0, pending: 0 });
		// Nothing on chain would ever release it.
		run_to(30);
		assert_eq!(DecidingCount::<Test>::get(1), 1);

		migrate();
		assert_eq!(census(1), TrackCensus { stored: 0, deciding: 0, pending: 0 });

		// A second run changes nothing.
		let after = root();
		migrate();
		assert_eq!(root(), after);
	});
}

#[test]
fn freeing_a_phantom_starts_the_queued_referendum() {
	// The Kusama Asset Hub Root track before ref 668: one slot, held by a phantom, and a
	// referendum waiting for it.
	new_test_ext().execute_with(|| {
		let a = submit(0);
		let b = submit(0);
		run_to(3);
		assert!(is_deciding(a));
		assert_eq!(queue(0), vec![b]);

		cancel_like_ahm(a);
		run_to(10);
		assert!(!is_deciding(b), "b stays stuck behind the phantom");
		assert_eq!(queue(0), vec![b]);

		migrate();
		assert!(is_deciding(b));
		assert!(queue(0).is_empty());
		assert_eq!(census(0), TrackCensus { stored: 1, deciding: 1, pending: 0 });
	});
}

#[test]
fn a_pending_alarm_is_not_a_phantom() {
	new_test_ext().execute_with(|| {
		let a = submit(1);
		run_to(3);
		assert_ok!(Referenda::cancel(RuntimeOrigin::root(), a));
		// The slot is released, but the count only drops when the alarm runs next block.
		assert_eq!(census(1), TrackCensus { stored: 1, deciding: 0, pending: 1 });

		let before = root();
		migrate();
		assert_eq!(root(), before, "the migration must not free the slot a second time");

		run_to(4);
		assert_eq!(census(1), TrackCensus { stored: 0, deciding: 0, pending: 0 });
	});
}

#[test]
fn frees_only_the_phantom_next_to_a_pending_alarm() {
	new_test_ext().execute_with(|| {
		let a = submit(1);
		let b = submit(1);
		run_to(3);
		assert_eq!(DecidingCount::<Test>::get(1), 2);

		cancel_like_ahm(a);
		assert_ok!(Referenda::cancel(RuntimeOrigin::root(), b));
		assert_eq!(census(1), TrackCensus { stored: 2, deciding: 0, pending: 1 });

		migrate();
		assert_eq!(census(1), TrackCensus { stored: 1, deciding: 0, pending: 1 });

		run_to(4);
		assert_eq!(census(1), TrackCensus { stored: 0, deciding: 0, pending: 0 });
	});
}

#[test]
fn removes_a_stale_queue_entry() {
	// Polkadot #693: cancelled while queued, still in `TrackQueue(0)`.
	new_test_ext().execute_with(|| {
		let a = submit(0);
		let b = submit(0);
		let c = submit(0);
		run_to(3);
		assert!(is_deciding(a));
		let stale = TrackQueue::<Test>::get(0);
		assert_eq!(stale.len(), 2);

		assert_ok!(Referenda::cancel(RuntimeOrigin::root(), b));
		// The pallet in use leaves `b` behind; polkadot-sdk#13236 removes it. Put the entry back
		// either way, so the test does not depend on which version runs.
		TrackQueue::<Test>::insert(0, stale);
		assert!(queue(0).contains(&b));

		migrate();
		assert_eq!(queue(0), vec![c]);
		assert!(is_deciding(a));
		assert_eq!(census(0), TrackCensus { stored: 1, deciding: 1, pending: 0 });
	});
}

#[test]
fn removes_duplicate_queue_entries() {
	new_test_ext().execute_with(|| {
		let _a = submit(0);
		let b = submit(0);
		run_to(3);
		let mut queue_with_duplicate = TrackQueue::<Test>::get(0);
		let entry = queue_with_duplicate[0];
		assert_eq!(entry.0, b);
		queue_with_duplicate.try_push(entry).unwrap();
		TrackQueue::<Test>::insert(0, queue_with_duplicate);
		assert_eq!(queue(0), vec![b, b]);

		migrate();
		assert_eq!(queue(0), vec![b]);
	});
}

#[test]
fn never_raises_a_count() {
	new_test_ext().execute_with(|| {
		let _a = submit(1);
		run_to(3);
		DecidingCount::<Test>::insert(1, 0);
		assert_eq!(census(1), TrackCensus { stored: 0, deciding: 1, pending: 0 });

		migrate();
		assert_eq!(DecidingCount::<Test>::get(1), 0);
	});
}

#[test]
fn leaves_consistent_state_alone() {
	new_test_ext().execute_with(|| {
		let _a = submit(0);
		let _b = submit(0);
		let _c = submit(1);
		run_to(3);
		assert_eq!(census(0), TrackCensus { stored: 1, deciding: 1, pending: 0 });
		assert_eq!(census(1), TrackCensus { stored: 1, deciding: 1, pending: 0 });

		let before = root();
		migrate();
		assert_eq!(root(), before);
	});
}

#[test]
fn repairs_every_track_in_one_run() {
	// Today's live state, on one chain: a phantom on a busy track, a phantom on an idle track and
	// a cancelled referendum left in a queue.
	new_test_ext().execute_with(|| {
		let a = submit(0);
		let b = submit(0);
		let c = submit(1);
		let d = submit(1);
		run_to(3);
		let stale = TrackQueue::<Test>::get(0);
		assert_ok!(Referenda::cancel(RuntimeOrigin::root(), b));
		TrackQueue::<Test>::insert(0, stale);
		cancel_like_ahm(c);
		assert_eq!(census(0), TrackCensus { stored: 1, deciding: 1, pending: 0 });
		assert_eq!(census(1), TrackCensus { stored: 2, deciding: 1, pending: 0 });

		migrate();
		assert!(queue(0).is_empty());
		assert!(is_deciding(a));
		assert!(is_deciding(d));
		assert_eq!(census(0), TrackCensus { stored: 1, deciding: 1, pending: 0 });
		assert_eq!(census(1), TrackCensus { stored: 1, deciding: 1, pending: 0 });
	});
}

#[test]
fn two_phantoms_with_one_queued_referendum() {
	new_test_ext().execute_with(|| {
		let a = submit(1);
		let b = submit(1);
		let c = submit(1);
		let d = submit(1);
		run_to(3);
		assert_eq!(queue(1), vec![d]);
		cancel_like_ahm(a);
		cancel_like_ahm(b);
		assert_eq!(census(1), TrackCensus { stored: 3, deciding: 1, pending: 0 });

		// The first freed slot starts `d`, the second one is decremented.
		migrate();
		assert!(is_deciding(c));
		assert!(is_deciding(d));
		assert!(queue(1).is_empty());
		assert_eq!(census(1), TrackCensus { stored: 2, deciding: 2, pending: 0 });

		// `d` was started with a working alarm: both referenda end and release their slots.
		run_to(60);
		assert_eq!(census(1), TrackCensus { stored: 0, deciding: 0, pending: 0 });
	});
}

/// The try-runtime checks must fail on the outcomes they exist to catch.
#[cfg(feature = "try-runtime")]
mod post_upgrade_rejects {
	use super::*;

	#[test]
	fn a_phantom_left_in_place() {
		new_test_ext().execute_with(|| {
			let a = submit(1);
			run_to(3);
			cancel_like_ahm(a);
			let state = RepairReferendaTracks::<Test>::pre_upgrade().unwrap();
			assert!(RepairReferendaTracks::<Test>::post_upgrade(state).is_err());
		});
	}

	#[test]
	fn a_slot_freed_twice() {
		new_test_ext().execute_with(|| {
			let _a = submit(1);
			run_to(3);
			let state = RepairReferendaTracks::<Test>::pre_upgrade().unwrap();
			DecidingCount::<Test>::insert(1, 0);
			assert!(RepairReferendaTracks::<Test>::post_upgrade(state).is_err());
		});
	}

	#[test]
	fn a_live_queued_referendum_dropped() {
		new_test_ext().execute_with(|| {
			let _a = submit(0);
			let _b = submit(0);
			run_to(3);
			let state = RepairReferendaTracks::<Test>::pre_upgrade().unwrap();
			TrackQueue::<Test>::remove(0);
			assert!(RepairReferendaTracks::<Test>::post_upgrade(state).is_err());
		});
	}
}
