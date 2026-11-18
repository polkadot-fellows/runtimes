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

//! Checks of the Sweep stage: the Relay Chain pots hold only free balance before and end empty,
//! the beneficiary gains exactly what is sent to it, and Asset Hub marks what arrived inactive.

use crate::{
	checks::{ParaMigrationCheck, RcMigrationCheck},
	mock::{network, AssetHubPara, CoretimePara, Para},
};
use cumulus_primitives_core::ParaId;
use frame_support::traits::Get;
use sp_runtime::{traits::AccountIdConversion, AccountId32};

type Rc = network::relay::Runtime;
type Ah = network::ah::Runtime;

/// What the Relay Chain's pots held before the migration.
#[derive(Clone, Debug)]
pub struct RcSweepPre {
	/// Every pot and its whole balance.
	pub pots: Vec<(AccountId32, u128)>,
	/// Where the pots' balance goes on Asset Hub.
	pub beneficiary: AccountId32,
	/// What the accounts stage sends from Asset Hub's own sovereign here to the same beneficiary.
	pub ah_sovereign_share: u128,
}

fn total<T>(who: &AccountId32) -> u128
where
	T: frame_system::Config<
		AccountId = AccountId32,
		AccountData = pallet_balances::AccountData<u128>,
	>,
{
	frame_system::Account::<T>::get(who).data.total()
}

/// What the accounts stage sends from Asset Hub's own sovereign account to the beneficiary,
/// worked out from the account itself: its free balance, less the buffer that goes to the
/// Coretime chain with a reserve. A share below the existential deposit goes there too.
fn ah_sovereign_share() -> u128 {
	let ah_sovereign: AccountId32 = ParaId::from(AssetHubPara::PARA_ID).into_account_truncating();
	let data = frame_system::Account::<Rc>::get(&ah_sovereign).data;
	let buffer = if data.reserved == 0 {
		0
	} else {
		data.free.min(<Rc as pallet_rc2_migrator::Config>::CtFreeBuffer::get())
	};
	let share = data.free - buffer;
	if share < <Rc as pallet_balances::Config>::ExistentialDeposit::get() {
		0
	} else {
		share
	}
}

/// Sweep burns only a pot's free balance, so a pot must hold nothing else.
fn assert_only_free_balance(pot: &AccountId32) {
	let account = frame_system::Account::<Rc>::get(pot);
	assert_eq!(
		(account.data.reserved, account.data.frozen, account.consumers),
		(0, 0, 0),
		"pot {pot:?} has a reserve, a freeze or a consumer reference"
	);
}

/// Check the pots, the beneficiary's balance and what Asset Hub marks inactive.
pub struct SweepChecker;

impl RcMigrationCheck for SweepChecker {
	type RcPrePayload = RcSweepPre;
	/// What the pots held, all of it swept.
	type RcPostPayload = u128;

	fn pre_check() -> Self::RcPrePayload {
		let pots = <Rc as pallet_rc2_migrator::Config>::SweptAccounts::get()
			.into_iter()
			.map(|pot| {
				assert_only_free_balance(&pot);
				let amount = total::<Rc>(&pot);
				(pot, amount)
			})
			.collect();
		RcSweepPre {
			pots,
			beneficiary: <Rc as pallet_rc2_migrator::Config>::SweepBeneficiary::get(),
			ah_sovereign_share: ah_sovereign_share(),
		}
	}

	fn post_check(rc_pre: &Self::RcPrePayload) -> Self::RcPostPayload {
		for (pot, _) in &rc_pre.pots {
			assert_eq!(total::<Rc>(pot), 0, "pot {pot:?} should be empty");
		}

		let swept = rc_pre.pots.iter().map(|(_, amount)| amount).sum();
		log::info!(
			target: "runtime::rc2-migrator",
			"Swept {swept} out of {} pot(s)",
			rc_pre.pots.iter().filter(|(_, amount)| *amount > 0).count()
		);
		swept
	}
}

impl ParaMigrationCheck<CoretimePara> for SweepChecker {
	type RcPrePayload = RcSweepPre;
	type RcPostPayload = u128;
	type PrePayload = ();

	fn pre_check(_: &Self::RcPrePayload) -> Self::PrePayload {}

	fn post_check(_: &Self::RcPrePayload, _: &Self::RcPostPayload, _: Self::PrePayload) {}
}

/// Run the DAP hook that moves staging funds to the buffer and marks them inactive.
#[cfg(feature = "polkadot")]
fn mark_beneficiary_inactive() {
	use frame_support::{traits::OnIdle, weights::Weight};
	// The DAP moves its staging account into its buffer and marks it inactive on the way.
	<network::ah::Dap as OnIdle<_>>::on_idle(
		frame_system::Pallet::<Ah>::block_number(),
		Weight::MAX,
	);
}

/// Run the treasury hook that marks the pot's spendable balance inactive.
#[cfg(feature = "kusama")]
fn mark_beneficiary_inactive() {
	use frame_support::traits::OnInitialize;
	// The treasury marks its pot inactive every block.
	<network::ah::Treasury as OnInitialize<_>>::on_initialize(
		frame_system::Pallet::<Ah>::block_number(),
	);
}

impl ParaMigrationCheck<AssetHubPara> for SweepChecker {
	type RcPrePayload = RcSweepPre;
	type RcPostPayload = u128;
	/// The beneficiary's balance before the migration.
	type PrePayload = u128;

	fn pre_check(rc_pre: &Self::RcPrePayload) -> Self::PrePayload {
		total::<Ah>(&rc_pre.beneficiary)
	}

	fn post_check(
		rc_pre: &Self::RcPrePayload,
		swept: &Self::RcPostPayload,
		ah_pre: Self::PrePayload,
	) {
		// The beneficiary gains exactly the pots and Asset Hub's sovereign's share.
		let arrived = total::<Ah>(&rc_pre.beneficiary)
			.checked_sub(ah_pre)
			.expect("the beneficiary did not shrink");
		assert_eq!(
			arrived,
			swept + rc_pre.ah_sovereign_share,
			"the beneficiary got something other than the {swept} swept and the {} from Asset \
			 Hub's sovereign",
			rc_pre.ah_sovereign_share
		);

		// The beneficiary's pallet must mark at least the swept amount inactive, allowing for one
		// existential deposit to remain active.
		let before = pallet_balances::InactiveIssuance::<Ah>::get();
		mark_beneficiary_inactive();
		let marked = pallet_balances::InactiveIssuance::<Ah>::get() - before;
		let ed = <Ah as pallet_balances::Config>::ExistentialDeposit::get();
		assert!(
			marked + ed >= *swept,
			"only {marked} was marked inactive on Asset Hub, of the {swept} swept"
		);
	}
}
