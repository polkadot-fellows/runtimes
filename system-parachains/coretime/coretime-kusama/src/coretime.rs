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
		fungible::{Balanced, Credit, Inspect},
		tokens::{Fortitude, Preservation},
		DefensiveResult, OnUnbalanced,
	},
	weights::constants::{WEIGHT_PROOF_SIZE_PER_KB, WEIGHT_REF_TIME_PER_MICROS},
};
use frame_system::Pallet as System;
use kusama_runtime_constants::{system_parachain::coretime, time::DAYS as RELAY_DAYS};
use pallet_broker::{
	CoreAssignment, CoreIndex, CoretimeInterface, PartsOf57600, RCBlockNumberOf, TaskId,
};
use parachains_common::{AccountId, Balance};
use sp_runtime::traits::{AccountIdConversion, MaybeConvert};
use xcm::latest::prelude::*;
use xcm_config::LocationToAccountId;
use xcm_executor::traits::{ConvertLocation, TransactAsset};

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
pub struct BurnCoretimeRevenue;
impl OnUnbalanced<Credit<AccountId, Balances>> for BurnCoretimeRevenue {
	fn on_nonzero_unbalanced(amount: Credit<AccountId, Balances>) {
		let acc = CoretimeBurnAccount::get();
		if !System::<Runtime>::account_exists(&acc) {
			// The account doesn't require ED to survive.
			System::<Runtime>::inc_providers(&acc);
		}
		Balances::resolve(&acc, amount).defensive_ok();
	}
}

type AssetTransactor = <xcm_config::XcmConfig as xcm_executor::Config>::AssetTransactor;

fn burn_at_relay(stash: &AccountId, value: Balance) -> Result<(), XcmError> {
	let dest = Location::parent();
	let stash_location =
		Junction::AccountId32 { network: None, id: stash.clone().into() }.into_location();
	let asset = Asset { id: AssetId(Location::parent()), fun: Fungible(value) };
	let dummy_xcm_context = XcmContext { origin: None, message_id: [0; 32], topic: None };

	let withdrawn = AssetTransactor::withdraw_asset(&asset, &stash_location, None)?;

	// TODO https://github.com/polkadot-fellows/runtimes/issues/404
	AssetTransactor::can_check_out(&dest, &asset, &dummy_xcm_context)?;

	let parent_assets = Into::<Assets>::into(withdrawn)
		.reanchored(&dest, &Here)
		.defensive_map_err(|_| XcmError::ReanchorFailed)?;

	PolkadotXcm::send_xcm(
		Here,
		Location::parent(),
		Xcm(vec![
			Instruction::UnpaidExecution {
				weight_limit: WeightLimit::Unlimited,
				check_origin: None,
			},
			ReceiveTeleportedAsset(parent_assets.clone()),
			BurnAsset(parent_assets),
		]),
	)?;

	AssetTransactor::check_out(&dest, &asset, &dummy_xcm_context);

	Ok(())
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
	type RelayChainBlockNumberProvider = RelaychainDataProvider<Runtime>;

	fn request_core_count(count: CoreIndex) {
		use crate::coretime::CoretimeProviderCalls::RequestCoreCount;
		let request_core_count_call = RelayRuntimePallets::Coretime(RequestCoreCount(count));

		// Weight for `request_core_count` from Kusama runtime benchmarks:
		// `ref_time`, `proof_size`, reads, writes
		// 9_670_000, 1640, 3, 1
		// Add 30% to each component with a healthy round up.
		let call_weight =
			Weight::from_parts(250 * WEIGHT_REF_TIME_PER_MICROS, 3 * WEIGHT_PROOF_SIZE_PER_KB);

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
		let request_revenue_info_at_call =
			RelayRuntimePallets::Coretime(RequestRevenueInfoAt(when));

		// Weight for `request_revenue_at` from Kusama runtime benchmarks:
		// `ref_time`, `proof_size`, reads, writes
		// 94_091_000, 6384, 7, 5
		// Add 30% to each component with a healthy round up.
		let call_weight =
			Weight::from_parts(1000 * WEIGHT_REF_TIME_PER_MICROS, 9 * WEIGHT_PROOF_SIZE_PER_KB);

		let message = Xcm(vec![
			Instruction::UnpaidExecution {
				weight_limit: WeightLimit::Unlimited,
				check_origin: None,
			},
			Instruction::Transact {
				origin_kind: OriginKind::Native,
				require_weight_at_most: call_weight,
				call: request_revenue_info_at_call.encode().into(),
			},
		]);

		match PolkadotXcm::send_xcm(Here, Location::parent(), message) {
			Ok(_) => log::debug!(
				target: "runtime::coretime",
				"Revenue info request sent successfully."
			),
			Err(e) => log::error!(
				target: "runtime::coretime",
				"Request for revenue info failed to send: {:?}",
				e
			),
		}
	}

	fn credit_account(who: Self::AccountId, amount: Self::Balance) {
		use crate::coretime::CoretimeProviderCalls::CreditAccount;
		let _credit_account_call = RelayRuntimePallets::Coretime(CreditAccount(who, amount));

		log::debug!(
			target: "runtime::coretime",
			"`credit_account` is unimplemented on the relay."
		);
	}

	fn assign_core(
		core: CoreIndex,
		begin: RCBlockNumberOf<Self>,
		assignment: Vec<(CoreAssignment, PartsOf57600)>,
		end_hint: Option<RCBlockNumberOf<Self>>,
	) {
		use crate::coretime::CoretimeProviderCalls::AssignCore;

		// Weight for `assign_core` from Kusama runtime benchmarks:
		// `ref_time`, `proof_size`, reads, writes
		// 12_042_907 + 80 * 13_919, 3545, 1, 2
		// Add 30% to each component with a healthy round up.
		let call_weight =
			Weight::from_parts(350 * WEIGHT_REF_TIME_PER_MICROS, 5 * WEIGHT_PROOF_SIZE_PER_KB);

		// The relay chain currently only allows `assign_core` to be called with a complete mask
		// and only ever with increasing `begin`. The assignments must be truncated to avoid
		// dropping that core's assignment completely.

		// This shadowing of `assignment` is temporary and can be removed when the relay can accept
		// multiple messages to assign a single core.
		let assignment = if assignment.len() > 28 {
			let mut total_parts = 0u16;
			// Account for missing parts with a new `Idle` assignment at the start as
			// `assign_core` on the relay assumes this is sorted. We'll add the rest of the
			// assignments and sum the parts in one pass, so this is just initialized to 0.
			let mut assignment_truncated = vec![(CoreAssignment::Idle, 0)];
			// Truncate to first 27 non-idle assignments.
			assignment_truncated.extend(
				assignment
					.into_iter()
					.filter(|(a, _)| *a != CoreAssignment::Idle)
					.take(27)
					.inspect(|(_, parts)| total_parts += *parts)
					.collect::<Vec<_>>(),
			);

			// Set the parts of the `Idle` assignment we injected at the start of the vec above.
			assignment_truncated[0].1 = 57_600u16.saturating_sub(total_parts);
			assignment_truncated
		} else {
			assignment
		};

		let assign_core_call =
			RelayRuntimePallets::Coretime(AssignCore(core, begin, assignment, end_hint));

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

	fn on_new_timeslice(t: pallet_broker::Timeslice) {
		// Burn roughly once per day. TIMESLICE_PERIOD tested to be != 0.
		const BURN_PERIOD: pallet_broker::Timeslice =
			RELAY_DAYS.saturating_div(coretime::TIMESLICE_PERIOD);
		// If checked_rem returns `None`, `TIMESLICE_PERIOD` is misconfigured for some reason. We
		// have bigger issues with the chain, but we still want to burn.
		if t.checked_rem(BURN_PERIOD).map_or(false, |r| r != 0) {
			return;
		}

		let stash = CoretimeBurnAccount::get();
		let value =
			Balances::reducible_balance(&stash, Preservation::Expendable, Fortitude::Polite);

		if value > 0 {
			log::debug!(target: "runtime::coretime", "Going to burn {value} stashed tokens at RC");
			match burn_at_relay(&stash, value) {
				Ok(()) => {
					log::debug!(target: "runtime::coretime", "Succesfully burnt {value} tokens");
				},
				Err(err) => {
					log::error!(target: "runtime::coretime", "burn_at_relay failed: {err:?}");
				},
			}
		}
	}
}

parameter_types! {
	pub const BrokerPalletId: PalletId = PalletId(*b"py/broke");
}

pub struct SovereignAccountOf;
impl MaybeConvert<TaskId, AccountId> for SovereignAccountOf {
	fn maybe_convert(id: TaskId) -> Option<AccountId> {
		// Currently all tasks are parachains
		let location = Location::new(1, [Parachain(id)]);
		LocationToAccountId::convert_location(&location)
	}
}

impl pallet_broker::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type OnRevenue = BurnCoretimeRevenue;
	type TimeslicePeriod = ConstU32<{ coretime::TIMESLICE_PERIOD }>;
	type MaxLeasedCores = ConstU32<50>;
	type MaxReservedCores = ConstU32<10>;
	type Coretime = CoretimeAllocator;
	type ConvertBalance = sp_runtime::traits::Identity;
	type WeightInfo = weights::pallet_broker::WeightInfo<Runtime>;
	type PalletId = BrokerPalletId;
	type AdminOrigin = EnsureRoot<AccountId>;
	type SovereignAccountOf = SovereignAccountOf;
	type MaxAutoRenewals = ConstU32<100>;
	type PriceAdapter = pallet_broker::CenterTargetPrice<Balance>;
}
