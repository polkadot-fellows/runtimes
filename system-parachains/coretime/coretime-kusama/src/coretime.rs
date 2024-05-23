// Copyright (C) Parity Technologies and the various Polkadot contributors, see Contributions.md
// for a list of specific contributors.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::*;
use codec::{Decode, Encode};
use cumulus_pallet_parachain_system::RelaychainDataProvider;
use cumulus_primitives_core::relay_chain;
use frame_support::{
	parameter_types,
	traits::{
		fungible::{Balanced, Credit},
		OnUnbalanced,
	},
};
use pallet_broker::{
	AdaptPrice, CoreAssignment, CoreIndex, CoretimeInterface, PartsOf57600, RCBlockNumberOf,
};
use sp_runtime::{
	traits::{AccountIdConversion, One, Saturating},
	FixedU64,
};
use xcm::latest::prelude::*;

/// A type containing the encoding of the coretime pallet in the Relay chain runtime. Used to
/// construct any remote calls. The codec index must correspond to the index of `Coretime` in the
/// `construct_runtime` of the Relay chain.
#[derive(Encode, Decode)]
enum RelayRuntimePallets {
	#[codec(index = 74)]
	Coretime(CoretimeProviderCalls),
}

/// Call encoding for the calls needed from the relay coretime pallet.
#[derive(Encode, Decode)]
enum CoretimeProviderCalls {
	#[codec(index = 1)]
	RequestCoreCount(CoreIndex),
	#[codec(index = 2)]
	RequestRevenueInfoAt(relay_chain::BlockNumber),
	#[codec(index = 3)]
	CreditAccount(AccountId, Balance),
	#[codec(index = 4)]
	AssignCore(
		CoreIndex,
		relay_chain::BlockNumber,
		Vec<(CoreAssignment, PartsOf57600)>,
		Option<relay_chain::BlockNumber>,
	),
}

parameter_types! {
	/// The holding account into which burnt funds will be moved at the point of sale. This will be
	/// burnt periodically.
	pub CoretimeBurnAccount: AccountId = PalletId(*b"py/ctbrn").into_account_truncating();
}

/// Burn revenue from coretime sales. See
/// [RFC-010](https://polkadot-fellows.github.io/RFCs/approved/0010-burn-coretime-revenue.html).
pub struct BurnRevenue;
impl OnUnbalanced<Credit<AccountId, Balances>> for BurnRevenue {
	fn on_nonzero_unbalanced(credit: Credit<AccountId, Balances>) {
		let _ = <Balances as Balanced<_>>::resolve(&CoretimeBurnAccount::get(), credit);
	}
}

parameter_types! {
	/// The revenue from on-demand coretime sales. This is distributed amonst those who contributed
	/// regions to the pool.
	pub storage CoretimeRevenue: Option<(BlockNumber, Balance)> = None;
}

/// Type that implements the [`CoretimeInterface`] for the allocation of Coretime. Meant to operate
/// from the parachain context. That is, the parachain provides a market (broker) for the sale of
/// coretime, but assumes a `CoretimeProvider` (i.e. a Relay Chain) to actually provide cores.
pub struct CoretimeAllocator;
impl CoretimeInterface for CoretimeAllocator {
	type AccountId = AccountId;
	type Balance = Balance;
	type RealyChainBlockNumberProvider = RelaychainDataProvider<Runtime>;

	fn request_core_count(count: CoreIndex) {
		use crate::coretime::CoretimeProviderCalls::RequestCoreCount;
		let request_core_count_call = RelayRuntimePallets::Coretime(RequestCoreCount(count));

		// Weight for `request_core_count` from Kusama runtime benchmarks:
		// `ref_time` = 7889000 + (3 * 25000000) + (1 * 100000000) = 182889000
		// `proof_size` = 1636
		// Add 5% to each component and round to 2 significant figures.
		let call_weight = Weight::from_parts(190_000_000, 1700);

		let message = Xcm(vec![
			Instruction::UnpaidExecution {
				weight_limit: WeightLimit::Unlimited,
				check_origin: None,
			},
			Instruction::Transact {
				origin_kind: OriginKind::Native,
				require_weight_at_most: call_weight,
				call: request_core_count_call.encode().into(),
			},
		]);

		match PolkadotXcm::send_xcm(Here, Location::parent(), message) {
			Ok(_) => log::debug!(
				target: "runtime::coretime",
				"Request to update schedulable cores sent successfully."
			),
			Err(e) => log::error!(
				target: "runtime::coretime",
				"Failed to send request to update schedulable cores: {:?}",
				e
			),
		}
	}

	fn request_revenue_info_at(when: RCBlockNumberOf<Self>) {
		use crate::coretime::CoretimeProviderCalls::RequestRevenueInfoAt;
		let _request_revenue_info_at_call =
			RelayRuntimePallets::Coretime(RequestRevenueInfoAt(when));

		log::debug!(
			target: "runtime::coretime",
			"`request_revenue` is unmiplemented on the relay."
		);
	}

	fn credit_account(who: Self::AccountId, amount: Self::Balance) {
		use crate::coretime::CoretimeProviderCalls::CreditAccount;
		let _credit_account_call = RelayRuntimePallets::Coretime(CreditAccount(who, amount));

		log::debug!(
			target: "runtime::coretime",
			"`credit_account` is unmiplemented on the relay."
		);
	}

	fn assign_core(
		core: CoreIndex,
		begin: RCBlockNumberOf<Self>,
		assignment: Vec<(CoreAssignment, PartsOf57600)>,
		end_hint: Option<RCBlockNumberOf<Self>>,
	) {
		use crate::coretime::CoretimeProviderCalls::AssignCore;
		let assign_core_call =
			RelayRuntimePallets::Coretime(AssignCore(core, begin, assignment, end_hint));

		// Weight for `assign_core` from Kusama runtime benchmarks:
		// `ref_time` = 10177115 + (1 * 25000000) + (2 * 100000000) + (80 * 13932) = 236291675
		// `proof_size` = 3612
		// Add 5% to each component and round to 2 significant figures.
		let call_weight = Weight::from_parts(248_000_000, 3800);

		let message = Xcm(vec![
			Instruction::UnpaidExecution {
				weight_limit: WeightLimit::Unlimited,
				check_origin: None,
			},
			Instruction::Transact {
				origin_kind: OriginKind::Native,
				require_weight_at_most: call_weight,
				call: assign_core_call.encode().into(),
			},
		]);

		match PolkadotXcm::send_xcm(Here, Location::parent(), message) {
			Ok(_) => log::debug!(
				target: "runtime::coretime",
				"Core assignment sent successfully."
			),
			Err(e) => log::error!(
				target: "runtime::coretime",
				"Core assignment failed to send: {:?}",
				e
			),
		}
	}

	fn check_notify_revenue_info() -> Option<(RCBlockNumberOf<Self>, Self::Balance)> {
		let revenue = CoretimeRevenue::get();
		CoretimeRevenue::set(&None);
		revenue
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn ensure_notify_revenue_info(when: RCBlockNumberOf<Self>, revenue: Self::Balance) {
		CoretimeRevenue::set(&Some((when, revenue)));
	}
}

/// Implements the [`AdaptPrice`] trait to control the price changes of bulk coretime. This tweaks
/// the [`pallet_broker::Linear`] implementation which hard-corrects to 0 if cores are offered but
/// not sold for just one sale. The monotonic lead-in is increased in magnitude to enable faster
/// price finding. The change in base price between sales has a lower limit of 0.5 to allow downward
/// pressure to be applied, while keeping a conservative upper limit of 1.2 (movements capped at 20%
/// if cores sell out) to avoid runaway prices in the early sales. The intention is that this will
/// be coupled with a low number of cores per sale and a 100% ideal bulk ratio for the first sales.
pub struct PriceAdapter;
impl AdaptPrice for PriceAdapter {
	fn leadin_factor_at(when: FixedU64) -> FixedU64 {
		// Start at 5x the base price and decrease to the base price at the end of leadin.
		FixedU64::from(5).saturating_sub(FixedU64::from(4) * when)
	}

	fn adapt_price(sold: CoreIndex, target: CoreIndex, limit: CoreIndex) -> FixedU64 {
		if sold <= target {
			// Range of [0.5, 1.0].
			FixedU64::from_rational(1, 2).saturating_add(FixedU64::from_rational(
				(sold).into(),
				(target.saturating_mul(2)).into(),
			))
		} else {
			// Range of (1.0, 1.2].

			// Unchecked math: In this branch we know that sold > target. The limit must be >= sold
			// by construction, and thus target must be < limit.
			FixedU64::one().saturating_add(FixedU64::from_rational(
				(sold - target).into(),
				(limit - target).saturating_mul(5).into(),
			))
		}
	}
}

parameter_types! {
	pub const BrokerPalletId: PalletId = PalletId(*b"py/broke");
}

impl pallet_broker::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type OnRevenue = BurnRevenue;
	#[cfg(feature = "fast-runtime")]
	type TimeslicePeriod = ConstU32<10>;
	#[cfg(not(feature = "fast-runtime"))]
	type TimeslicePeriod = ConstU32<80>;
	type MaxLeasedCores = ConstU32<50>;
	type MaxReservedCores = ConstU32<10>;
	type Coretime = CoretimeAllocator;
	type ConvertBalance = sp_runtime::traits::Identity;
	type WeightInfo = weights::pallet_broker::WeightInfo<Runtime>;
	type PalletId = BrokerPalletId;
	type AdminOrigin = EnsureRoot<AccountId>;
	#[cfg(feature = "runtime-benchmarks")]
	type PriceAdapter = pallet_broker::Linear;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type PriceAdapter = PriceAdapter;
}
