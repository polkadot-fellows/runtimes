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

use super::*;
use core::{cmp::Ordering, marker::PhantomData};
use frame_support::{
	dispatch::DispatchResultWithPostInfo,
	traits::{Currency, Get, Imbalance, OnUnbalanced, OriginTrait, PrivilegeCmp},
};
use pallet_alliance::{ProposalIndex, ProposalProvider};
use parachains_common::impls::NegativeImbalance;
use sp_runtime::DispatchError;
use xcm_executor::traits::ConvertLocation;

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

type ProposalOf<T, I> = <T as pallet_collective::Config<I>>::Proposal;

type HashOf<T> = <T as frame_system::Config>::Hash;

/// Type alias to conveniently refer to the `Currency::Balance` associated type.
pub type BalanceOf<T> =
	<pallet_balances::Pallet<T> as Currency<<T as frame_system::Config>::AccountId>>::Balance;

/// Implements `OnUnbalanced::on_unbalanced` to teleport slashed assets to relay chain treasury
/// account.
pub struct ToParentTreasury<TreasuryAccount, AccountIdConverter, T>(
	PhantomData<(TreasuryAccount, AccountIdConverter, T)>,
);

impl<TreasuryAccount, AccountIdConverter, T> OnUnbalanced<NegativeImbalance<T>>
	for ToParentTreasury<TreasuryAccount, AccountIdConverter, T>
where
	T: pallet_balances::Config + pallet_xcm::Config + frame_system::Config,
	<<T as frame_system::Config>::RuntimeOrigin as OriginTrait>::AccountId: From<AccountIdOf<T>>,
	[u8; 32]: From<<T as frame_system::Config>::AccountId>,
	TreasuryAccount: Get<AccountIdOf<T>>,
	BalanceOf<T>: Into<Fungibility>,
	AccountIdConverter: ConvertLocation<AccountIdOf<T>>,
{
	fn on_unbalanced(amount: NegativeImbalance<T>) {
		let amount = match amount.drop_zero() {
			Ok(..) => return,
			Err(amount) => amount,
		};
		let imbalance = amount.peek();
		let root_location: Location = Here.into();
		let root_account: AccountIdOf<T> =
			match AccountIdConverter::convert_location(&root_location) {
				Some(a) => a,
				None => {
					log::warn!("Failed to convert root origin into account id");
					return
				},
			};
		let treasury_account: AccountIdOf<T> = TreasuryAccount::get();

		<pallet_balances::Pallet<T>>::resolve_creating(&root_account, amount);

		let result = <pallet_xcm::Pallet<T>>::limited_teleport_assets(
			<<T as frame_system::Config>::RuntimeOrigin>::root(),
			Box::new(Parent.into()),
			Box::new(
				Junction::AccountId32 { network: None, id: treasury_account.into() }
					.into_location()
					.into(),
			),
			Box::new((Parent, imbalance).into()),
			0,
			WeightLimit::Unlimited,
		);

		if let Err(err) = result {
			log::warn!("Failed to teleport slashed assets: {err:?}");
		}
	}
}

/// Proposal provider for alliance pallet.
/// Adapter from collective pallet to alliance proposal provider trait.
pub struct AllianceProposalProvider<T, I = ()>(PhantomData<(T, I)>);

impl<T, I> ProposalProvider<AccountIdOf<T>, HashOf<T>, ProposalOf<T, I>>
	for AllianceProposalProvider<T, I>
where
	T: pallet_collective::Config<I> + frame_system::Config,
	I: 'static,
{
	fn propose_proposal(
		who: AccountIdOf<T>,
		threshold: u32,
		proposal: Box<ProposalOf<T, I>>,
		length_bound: u32,
	) -> Result<(u32, u32), DispatchError> {
		pallet_collective::Pallet::<T, I>::do_propose_proposed(
			who,
			threshold,
			proposal,
			length_bound,
		)
	}

	fn vote_proposal(
		who: AccountIdOf<T>,
		proposal: HashOf<T>,
		index: ProposalIndex,
		approve: bool,
	) -> Result<bool, DispatchError> {
		pallet_collective::Pallet::<T, I>::do_vote(who, proposal, index, approve)
	}

	fn close_proposal(
		proposal_hash: HashOf<T>,
		proposal_index: ProposalIndex,
		proposal_weight_bound: Weight,
		length_bound: u32,
	) -> DispatchResultWithPostInfo {
		pallet_collective::Pallet::<T, I>::do_close(
			proposal_hash,
			proposal_index,
			proposal_weight_bound,
			length_bound,
		)
	}

	fn proposal_of(proposal_hash: HashOf<T>) -> Option<ProposalOf<T, I>> {
		pallet_collective::ProposalOf::<T, I>::get(proposal_hash)
	}
}

/// Used to compare the privilege of an origin inside the scheduler.
pub struct EqualOrGreatestRootCmp;

impl PrivilegeCmp<OriginCaller> for EqualOrGreatestRootCmp {
	fn cmp_privilege(left: &OriginCaller, right: &OriginCaller) -> Option<Ordering> {
		if left == right {
			return Some(Ordering::Equal)
		}
		match (left, right) {
			// Root is greater than anything.
			(OriginCaller::system(frame_system::RawOrigin::Root), _) => Some(Ordering::Greater),
			_ => None,
		}
	}
}

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarks {
	use super::*;
	use cumulus_primitives_core::{ChannelStatus, GetChannelInfo};
	use frame_support::traits::{
		fungible,
		tokens::{Pay, PaymentStatus},
	};
	use pallet_ranked_collective::Rank;
	use sp_runtime::traits::Convert;

	/// Rank to salary conversion helper type.
	pub struct RankToSalary<Fungible>(PhantomData<Fungible>);
	impl<Fungible> Convert<Rank, Balance> for RankToSalary<Fungible>
	where
		Fungible: fungible::Inspect<AccountId, Balance = Balance>,
	{
		fn convert(r: Rank) -> Balance {
			Balance::from(r).saturating_mul(Fungible::minimum_balance())
		}
	}

	/// Trait for setting up any prerequisites for successful execution of benchmarks.
	pub trait EnsureSuccessful {
		fn ensure_successful();
	}

	/// Implementation of the [`EnsureSuccessful`] trait which opens an HRMP channel between
	/// the Collectives and a parachain with a given ID.
	pub struct OpenHrmpChannel<I>(PhantomData<I>);
	impl<I: Get<u32>> EnsureSuccessful for OpenHrmpChannel<I> {
		fn ensure_successful() {
			let para_id = I::get();

			// open HRMP channel
			if let ChannelStatus::Closed = ParachainSystem::get_channel_status(para_id.into()) {
				ParachainSystem::open_outbound_hrmp_channel_for_benchmarks_or_tests(para_id.into())
			}

			// set XCM version for sibling parachain
			let sibling_parachain = Location::new(1, [Parachain(para_id)]);
			if PolkadotXcm::get_version_for(&sibling_parachain).is_none() {
				if let Err(e) = PolkadotXcm::force_xcm_version(
					RuntimeOrigin::root(),
					sibling_parachain.into(),
					system_parachains_constants::genesis_presets::SAFE_XCM_VERSION,
				) {
					log::error!(
						"Failed to `force_xcm_version` for para_id: {para_id:?}, error: {e:?}"
					);
				}
			}
		}
	}

	/// Type that wraps a type implementing the [`Pay`] trait to decorate its
	/// [`Pay::ensure_successful`] function with a provided implementation of the
	/// [`EnsureSuccessful`] trait.
	pub struct PayWithEnsure<O, E>(PhantomData<(O, E)>);
	impl<O, E> Pay for PayWithEnsure<O, E>
	where
		O: Pay,
		E: EnsureSuccessful,
	{
		type AssetKind = O::AssetKind;
		type Balance = O::Balance;
		type Beneficiary = O::Beneficiary;
		type Error = O::Error;
		type Id = O::Id;

		fn pay(
			who: &Self::Beneficiary,
			asset_kind: Self::AssetKind,
			amount: Self::Balance,
		) -> Result<Self::Id, Self::Error> {
			O::pay(who, asset_kind, amount)
		}
		fn check_payment(id: Self::Id) -> PaymentStatus {
			O::check_payment(id)
		}
		fn ensure_successful(
			who: &Self::Beneficiary,
			asset_kind: Self::AssetKind,
			amount: Self::Balance,
		) {
			E::ensure_successful();
			O::ensure_successful(who, asset_kind, amount)
		}
		fn ensure_concluded(id: Self::Id) {
			O::ensure_concluded(id)
		}
	}
}
