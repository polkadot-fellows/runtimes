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
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! Test that Multisig Account IDs result in the same IDs and they can still dispatch calls.

#[cfg(feature = "kusama-ahm")]
use crate::porting_prelude::*;

use frame_support::{dispatch::GetDispatchInfo, pallet_prelude::Weight, traits::Currency};
use pallet_ah_migrator::types::AhMigrationCheck;
use pallet_rc_migrator::types::RcMigrationCheck;
use rand::{seq::IteratorRandom, Rng};
use sp_core::Get;
use sp_runtime::{traits::StaticLookup, AccountId32};

type RelayRuntime = polkadot_runtime::Runtime;
type AssetHubRuntime = asset_hub_polkadot_runtime::Runtime;

pub struct MultisigStillWork;

#[derive(Clone)]
pub struct Multisig<AccountId> {
	pub signatories: Vec<AccountId>,
	pub threshold: u16,
	pub pure: AccountId,
}

pub type MultisigOf<T> = Multisig<<T as frame_system::Config>::AccountId>;

impl RcMigrationCheck for MultisigStillWork {
	type RcPrePayload = Vec<MultisigOf<RelayRuntime>>;

	fn pre_check() -> Self::RcPrePayload {
		// We generate 100 multisigs consisting of between 1 and 10 signatories.
		// Just use the first 1000 accs to make the generation a bit faster.
		let accounts = frame_system::Account::<RelayRuntime>::iter()
			.take(1000)
			.map(|(id, _)| id)
			.collect::<Vec<_>>();
		let mut multisigs = Vec::new();
		let mut rng = rand::rng();

		for _i in 0..100 {
			// Threshold must be at least 2, otherwise it's not really a multisig and the TX fails.
			// We can use `as_multi_1` if we really want to test this as well
			let num_signatories = rng.random_range(2..=10);
			// pick num_signatories random accounts
			let mut signatories =
				accounts.iter().cloned().choose_multiple(&mut rng, num_signatories);
			signatories.sort();

			let threshold = rng.random_range(2..=num_signatories) as u16;
			let pure =
				pallet_multisig::Pallet::<RelayRuntime>::multi_account_id(&signatories, threshold);
			let multisig = Multisig { signatories, threshold, pure };

			// Check that it would work
			multisig_works::<
				RelayRuntime,
				polkadot_runtime::RuntimeCall,
				polkadot_runtime::RuntimeOrigin,
			>(&multisig);

			multisigs.push(multisig.clone());
		}

		// TODO: @ggwpez supposed to be error? errors for Kusama
		log::error!("multisigs num: {:?}", multisigs.len());
		multisigs
	}

	fn post_check(_: Self::RcPrePayload) {}
}

fn fund_account<T: pallet_balances::Config<Balance = u128>>(account: &T::AccountId) {
	let amount = pure_balance::<T>();
	let _ = pallet_balances::Pallet::<T>::deposit_creating(&account.clone(), amount);
}

fn pure_balance<T: pallet_balances::Config<Balance = u128>>() -> u128 {
	let ed = <T as pallet_balances::Config>::ExistentialDeposit::get();
	ed * 100_000_000_000u128
}

fn multisig_works<
	T: pallet_multisig::Config<RuntimeCall = RuntimeCall>
		+ pallet_balances::Config<Balance = u128>
		+ frame_system::Config<AccountId = AccountId32, RuntimeOrigin = RuntimeOrigin>,
	RuntimeCall: From<pallet_balances::Call<T>> + codec::Encode + GetDispatchInfo,
	RuntimeOrigin: frame_support::traits::OriginTrait<AccountId = AccountId32>,
>(
	multisig: &MultisigOf<T>,
) {
	// Have to fund again since the pre_checks cannot modify storage
	for signatory in multisig.signatories.iter() {
		fund_account::<AssetHubRuntime>(signatory);
	}
	fund_account::<AssetHubRuntime>(&multisig.pure);

	// Send some funds to Alice
	let ed = <T as pallet_balances::Config>::ExistentialDeposit::get();
	let value = ed * 10;
	let alice = AccountId32::from([1u8; 32]);
	let call: RuntimeCall = pallet_balances::Call::transfer_allow_death {
		dest: <<T as frame_system::Config>::Lookup as StaticLookup>::unlookup(alice.clone()),
		value,
	}
	.into();
	let call_hash = call.using_encoded(sp_core::hashing::blake2_256);
	let call_weight = call.get_dispatch_info().call_weight;

	// All other signatories approve
	let timepoint = pallet_multisig::Pallet::<T>::timepoint();
	for (i, signatory) in multisig.signatories[1..]
		.iter()
		.take(multisig.threshold as usize - 1)
		.enumerate()
	{
		let other_signatories = multisig
			.signatories
			.iter()
			.filter(|&s| s != signatory)
			.cloned()
			.collect::<Vec<_>>();
		let timepoint = if i == 0 { None } else { Some(timepoint) };

		pallet_multisig::Pallet::<T>::approve_as_multi(
			RuntimeOrigin::signed(signatory.clone()),
			multisig.threshold,
			other_signatories,
			timepoint,
			call_hash,
			Weight::zero(),
		)
		.unwrap();
	}
	// Last one executes
	pallet_multisig::Pallet::<T>::as_multi(
		RuntimeOrigin::signed(multisig.signatories[0].clone()),
		multisig.threshold,
		multisig.signatories[1..].to_vec(),
		Some(timepoint),
		Box::new(call),
		call_weight,
	)
	.unwrap();
}

impl AhMigrationCheck for MultisigStillWork {
	type RcPrePayload = Vec<MultisigOf<AssetHubRuntime>>;
	type AhPrePayload = ();

	fn pre_check(_: Self::RcPrePayload) -> Self::AhPrePayload {}
	fn post_check(multisigs: Self::RcPrePayload, _: Self::AhPrePayload) {
		for multisig in multisigs {
			let ah_multisig = pallet_multisig::Pallet::<AssetHubRuntime>::multi_account_id(
				&multisig.signatories,
				multisig.threshold,
			);
			// sanity check
			assert_eq!(multisig.pure, ah_multisig, "multisig pure account id should be the same");

			multisig_works::<
				AssetHubRuntime,
				asset_hub_polkadot_runtime::RuntimeCall,
				asset_hub_polkadot_runtime::RuntimeOrigin,
			>(&multisig);
		}
	}
}
