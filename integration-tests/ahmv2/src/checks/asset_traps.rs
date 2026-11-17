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

//! Check that the asset traps the manager hands over leave `pallet-xcm` and are paid to their
//! claimers on Asset Hub.
//!
//! The snapshot's own traps are not handed over, since the test does not know what they hold. It
//! makes traps of its own with [`make_test_traps`] instead.

use crate::{
	checks::{ParaMigrationCheck, RcMigrationCheck},
	mock::{network, AssetHubPara, CoretimePara},
};
use pallet_rc2_migrator::{AssetTrap, RcMigratedBalance, TakenAssetTraps};
use sp_runtime::AccountId32;
use xcm::latest::prelude::*;
use xcm_executor::traits::ConvertLocation;

type Rc = network::relay::Runtime;
type Ah = network::ah::Runtime;
type XcmAssetTraps = <Rc as pallet_rc2_migrator::Config>::XcmAssetTraps;

pub struct AssetTrapsChecker;

/// The traps the test makes and hands over, with how many times each was trapped: one for each
/// kind of claimer Asset Hub can pay, and one worth too little to pay.
///
/// No claimer is paid on an account the accounts stage pays too, so the accounts check still sees
/// only what that stage sent.
pub fn test_traps() -> Vec<(AssetTrap, u32)> {
	let units = network::constants::currency::UNITS;
	let this_network = network::relay::xcm_config::ThisNetwork::get();
	let trap = |claimer: Location, amount: u128| AssetTrap {
		claimer: claimer.into(),
		assets: Assets::from((Here, amount)).into(),
	};
	let account = |id| Junction::AccountId32 { network: None, id };
	vec![
		(
			trap(
				Location::new(
					0,
					[Junction::AccountId32 { network: Some(this_network), id: [0xa7; 32] }],
				),
				3 * units,
			),
			1,
		),
		(trap(Location::new(0, [Parachain(3999)]), 2 * units), 2),
		(trap(Location::new(0, [Parachain(3999), account([0xa8; 32])]), units), 1),
		(
			trap(
				Location::new(
					0,
					[Parachain(3999), AccountKey20 { network: None, key: [0xa9; 20] }],
				),
				units,
			),
			1,
		),
		(
			trap(
				Location::new(
					0,
					[Parachain(3999), Plurality { id: BodyId::Index(3), part: BodyPart::Voice }],
				),
				units,
			),
			1,
		),
		// worth less than Asset Hub's existential deposit
		(trap(Location::new(0, [Parachain(3998)]), ah_existential_deposit() - 1), 1),
	]
}

/// Make the [`test_traps`] in `pallet-xcm`, as failed messages would have.
pub fn make_test_traps() {
	for (trap, count) in test_traps() {
		XcmAssetTraps::mutate(hash(&trap), |held| *held += count);
	}
}

fn ah_existential_deposit() -> u128 {
	<Rc as pallet_rc2_migrator::Config>::AhExistentialDeposit::get()
}

fn hash(trap: &AssetTrap) -> sp_core::H256 {
	trap.hash().expect("every test claimer converts")
}

/// What `trap`, made `count` times, is worth, or `None` if it is too little to pay.
fn payable(trap: &AssetTrap, count: u32) -> Option<u128> {
	let amount = trap.native_amount().expect("every test trap holds only the native token") *
		u128::from(count);
	(amount >= ah_existential_deposit()).then_some(amount)
}

/// The account on Asset Hub each payable test trap is paid to, and how much.
fn payouts() -> Vec<(AccountId32, u128)> {
	test_traps()
		.into_iter()
		.filter_map(|(trap, count)| {
			let amount = payable(&trap, count)?;
			let beneficiary = trap.ah_beneficiary().expect("every test claimer is payable");
			let who = network::ah::xcm_config::LocationToAccountId::convert_location(&beneficiary)
				.expect("Asset Hub has an account for every claimer it is paid");
			Some((who, amount))
		})
		.collect()
}

fn ah_balance(who: &AccountId32) -> u128 {
	frame_system::Account::<Ah>::get(who).data.total()
}

impl RcMigrationCheck for AssetTrapsChecker {
	type RcPrePayload = ();
	type RcPostPayload = ();

	fn pre_check() -> Self::RcPrePayload {
		for (trap, count) in test_traps() {
			assert_eq!(XcmAssetTraps::get(hash(&trap)), count, "the test made {trap:?}");
		}
	}

	fn post_check(_: &Self::RcPrePayload) -> Self::RcPostPayload {
		let mut paid = 0;
		for (trap, count) in test_traps() {
			let held = XcmAssetTraps::get(hash(&trap));
			match payable(&trap, count) {
				Some(amount) => {
					assert_eq!(held, 0, "{trap:?} was taken out of `pallet-xcm`");
					paid += amount;
				},
				None => assert_eq!(held, count, "{trap:?} is too little to pay, so it stays"),
			}
		}
		assert_eq!(TakenAssetTraps::<Rc>::iter().count(), 0, "every taken trap was paid");
		assert_eq!(RcMigratedBalance::<Rc>::get().ah_asset_traps, paid);
	}
}

impl ParaMigrationCheck<CoretimePara> for AssetTrapsChecker {
	type RcPrePayload = ();
	type RcPostPayload = ();
	type PrePayload = ();

	fn pre_check(_: &Self::RcPrePayload) -> Self::PrePayload {}

	fn post_check(_: &Self::RcPrePayload, _: &Self::RcPostPayload, _: Self::PrePayload) {}
}

impl ParaMigrationCheck<AssetHubPara> for AssetTrapsChecker {
	type RcPrePayload = ();
	type RcPostPayload = ();
	/// The balance of each account a test trap is paid to.
	type PrePayload = Vec<u128>;

	fn pre_check(_: &Self::RcPrePayload) -> Self::PrePayload {
		payouts().iter().map(|(who, _)| ah_balance(who)).collect()
	}

	fn post_check(_: &Self::RcPrePayload, _: &Self::RcPostPayload, before: Self::PrePayload) {
		for ((who, amount), before) in payouts().into_iter().zip(before) {
			assert_eq!(ah_balance(&who) - before, amount, "{who:?} was paid its trap on Asset Hub");
		}
	}
}
