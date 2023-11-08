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

//! Tests for the Polkadot Runtime Configuration

use crate::*;
use frame_support::traits::WhitelistedStorageKeys;
use pallet_transaction_payment::Multiplier;
use parity_scale_codec::Encode;
use sp_core::hexdisplay::HexDisplay;
use std::collections::HashSet;

#[test]
fn call_size() {
	RuntimeCall::assert_size_under(230);
}

#[test]
fn check_whitelist() {
	let whitelist: HashSet<String> = AllPalletsWithSystem::whitelisted_storage_keys()
		.iter()
		.map(|e| HexDisplay::from(&e.key).to_string())
		.collect();

	// Block number
	assert!(whitelist.contains("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac"));
	// Total issuance
	assert!(whitelist.contains("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80"));
	// Execution phase
	assert!(whitelist.contains("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a"));
	// Event count
	assert!(whitelist.contains("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850"));
	// System events
	assert!(whitelist.contains("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7"));
	// XcmPallet VersionDiscoveryQueue
	assert!(whitelist.contains("1405f2411d0af5a7ff397e7c9dc68d194a222ba0333561192e474c59ed8e30e1"));
	// XcmPallet SafeXcmVersion
	assert!(whitelist.contains("1405f2411d0af5a7ff397e7c9dc68d196323ae84c43568be0d1394d5d0d522c4"));
}

#[test]
fn check_treasury_pallet_id() {
	assert_eq!(
		<Treasury as frame_support::traits::PalletInfoAccess>::index() as u8,
		polkadot_runtime_constants::TREASURY_PALLET_ID
	);
}

mod test_fees {
	use super::*;
	use frame_support::{dispatch::GetDispatchInfo, weights::WeightToFee as WeightToFeeT};
	use keyring::Sr25519Keyring::{Alice, Charlie};
	use pallet_transaction_payment::Multiplier;
	use runtime_common::MinimumMultiplier;
	use separator::Separatable;
	use sp_runtime::{assert_eq_error_rate, FixedPointNumber, MultiAddress, MultiSignature};

	#[test]
	fn payout_weight_portion() {
		use pallet_staking::WeightInfo;
		let payout_weight =
			<Runtime as pallet_staking::Config>::WeightInfo::payout_stakers_alive_staked(
				MaxNominatorRewardedPerValidator::get(),
			)
			.ref_time() as f64;
		let block_weight = BlockWeights::get().max_block.ref_time() as f64;

		println!(
			"a full payout takes {:.2} of the block weight [{} / {}]",
			payout_weight / block_weight,
			payout_weight,
			block_weight
		);
		assert!(payout_weight * 2f64 < block_weight);
	}

	#[test]
	fn block_cost() {
		let max_block_weight = BlockWeights::get().max_block;
		let raw_fee = WeightToFee::weight_to_fee(&max_block_weight);

		let fee_with_multiplier = |m: Multiplier| {
			println!(
				"Full Block weight == {} // multiplier: {:?} // WeightToFee(full_block) == {} plank",
				max_block_weight,
				m,
				m.saturating_mul_int(raw_fee).separated_string(),
			);
		};
		fee_with_multiplier(MinimumMultiplier::get());
		fee_with_multiplier(Multiplier::from_rational(1, 2));
		fee_with_multiplier(Multiplier::from_u32(1));
		fee_with_multiplier(Multiplier::from_u32(2));
	}

	#[test]
	fn transfer_cost_min_multiplier() {
		let min_multiplier = MinimumMultiplier::get();
		let call = pallet_balances::Call::<Runtime>::transfer_keep_alive {
			dest: Charlie.to_account_id().into(),
			value: Default::default(),
		};
		let info = call.get_dispatch_info();
		println!("call = {:?} / info = {:?}", call, info);
		// convert to runtime call.
		let call = RuntimeCall::Balances(call);
		let extra: SignedExtra = (
			frame_system::CheckNonZeroSender::<Runtime>::new(),
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckMortality::<Runtime>::from(generic::Era::immortal()),
			frame_system::CheckNonce::<Runtime>::from(1),
			frame_system::CheckWeight::<Runtime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(0),
			claims::PrevalidateAttests::<Runtime>::new(),
		);
		let uxt = UncheckedExtrinsic {
			function: call,
			signature: Some((
				MultiAddress::Id(Alice.to_account_id()),
				MultiSignature::Sr25519(Alice.sign(b"foo")),
				extra,
			)),
		};
		let len = uxt.encoded_size();

		let mut ext = sp_io::TestExternalities::new_empty();
		let mut test_with_multiplier = |m: Multiplier| {
			ext.execute_with(|| {
				pallet_transaction_payment::NextFeeMultiplier::<Runtime>::put(m);
				let fee = TransactionPayment::query_fee_details(uxt.clone(), len as u32);
				println!(
					"multiplier = {:?} // fee details = {:?} // final fee = {:?}",
					pallet_transaction_payment::NextFeeMultiplier::<Runtime>::get(),
					fee,
					fee.final_fee().separated_string(),
				);
			});
		};

		test_with_multiplier(min_multiplier);
		test_with_multiplier(Multiplier::saturating_from_rational(1u128, 1u128));
		test_with_multiplier(Multiplier::saturating_from_rational(1u128, 1_0u128));
		test_with_multiplier(Multiplier::saturating_from_rational(1u128, 1_00u128));
		test_with_multiplier(Multiplier::saturating_from_rational(1u128, 1_000u128));
		test_with_multiplier(Multiplier::saturating_from_rational(1u128, 1_000_000u128));
		test_with_multiplier(Multiplier::saturating_from_rational(1u128, 1_000_000_000u128));
	}

	#[test]
	fn nominator_limit() {
		use pallet_election_provider_multi_phase::WeightInfo;
		// starting point of the nominators.
		let target_voters: u32 = 50_000;

		// assuming we want around 5k candidates and 1k active validators. (March 31, 2021)
		let all_targets: u32 = 5_000;
		let desired: u32 = 1_000;
		let weight_with = |active| {
			<Runtime as pallet_election_provider_multi_phase::Config>::WeightInfo::submit_unsigned(
				active,
				all_targets,
				active,
				desired,
			)
		};

		let mut active = target_voters;
		while weight_with(active).all_lte(OffchainSolutionWeightLimit::get()) ||
			active == target_voters
		{
			active += 1;
		}

		println!("can support {} nominators to yield a weight of {}", active, weight_with(active));
		assert!(active > target_voters, "we need to reevaluate the weight of the election system");
	}

	#[test]
	fn signed_deposit_is_sensible() {
		// ensure this number does not change, or that it is checked after each change.
		// a 1 MB solution should take (40 + 10) DOTs of deposit.
		let deposit = SignedDepositBase::get() + (SignedDepositByte::get() * 1024 * 1024);
		assert_eq_error_rate!(deposit, 50 * DOLLARS, DOLLARS);
	}
}

mod multiplier_tests {
	use super::*;
	use frame_support::{dispatch::DispatchInfo, traits::OnFinalize};
	use runtime_common::{MinimumMultiplier, TargetBlockFullness};
	use scale_info::TypeInfo;
	use separator::Separatable;
	use sp_runtime::traits::Convert;

	fn run_with_system_weight<F>(w: Weight, mut assertions: F)
	where
		F: FnMut() -> (),
	{
		let mut t: sp_io::TestExternalities = frame_system::GenesisConfig::<Runtime>::default()
			.build_storage()
			.unwrap()
			.into();
		t.execute_with(|| {
			System::set_block_consumed_resources(w, 0);
			assertions()
		});
	}

	#[test]
	fn multiplier_can_grow_from_zero() {
		let minimum_multiplier = MinimumMultiplier::get();
		let target = TargetBlockFullness::get() *
			BlockWeights::get().get(DispatchClass::Normal).max_total.unwrap();
		// if the min is too small, then this will not change, and we are doomed forever.
		// the weight is 1/100th bigger than target.
		run_with_system_weight(target.saturating_mul(101) / 100, || {
			let next = SlowAdjustingFeeUpdate::<Runtime>::convert(minimum_multiplier);
			assert!(next > minimum_multiplier, "{:?} !>= {:?}", next, minimum_multiplier);
		})
	}

	#[test]
	fn fast_unstake_estimate() {
		use pallet_fast_unstake::WeightInfo;
		let block_time = BlockWeights::get().max_block.ref_time() as f32;
		let on_idle = weights::pallet_fast_unstake::WeightInfo::<Runtime>::on_idle_check(
			300,
			<Runtime as pallet_fast_unstake::Config>::BatchSize::get(),
		)
		.ref_time() as f32;
		println!("ratio of block weight for full batch fast-unstake {}", on_idle / block_time);
		assert!(on_idle / block_time <= 0.5f32)
	}

	#[test]
	#[ignore]
	fn multiplier_growth_simulator() {
		// assume the multiplier is initially set to its minimum. We update it with values twice the
		//target (target is 25%, thus 50%) and we see at which point it reaches 1.
		let mut multiplier = MinimumMultiplier::get();
		let block_weight = BlockWeights::get().get(DispatchClass::Normal).max_total.unwrap();
		let mut blocks = 0;
		let mut fees_paid = 0;

		frame_system::Pallet::<Runtime>::set_block_consumed_resources(Weight::MAX, 0);
		let info = DispatchInfo { weight: Weight::MAX, ..Default::default() };

		let mut t: sp_io::TestExternalities = frame_system::GenesisConfig::<Runtime>::default()
			.build_storage()
			.unwrap()
			.into();
		// set the minimum
		t.execute_with(|| {
			pallet_transaction_payment::NextFeeMultiplier::<Runtime>::set(MinimumMultiplier::get());
		});

		while multiplier <= Multiplier::from_u32(1) {
			t.execute_with(|| {
				// imagine this tx was called.
				let fee = TransactionPayment::compute_fee(0, &info, 0);
				fees_paid += fee;

				// this will update the multiplier.
				System::set_block_consumed_resources(block_weight, 0);
				TransactionPayment::on_finalize(1);
				let next = TransactionPayment::next_fee_multiplier();

				assert!(next > multiplier, "{:?} !>= {:?}", next, multiplier);
				multiplier = next;

				println!(
					"block = {} / multiplier {:?} / fee = {:?} / fess so far {:?}",
					blocks,
					multiplier,
					fee.separated_string(),
					fees_paid.separated_string()
				);
			});
			blocks += 1;
		}
	}

	#[test]
	#[ignore]
	fn multiplier_cool_down_simulator() {
		// assume the multiplier is initially set to its minimum. We update it with values twice the
		//target (target is 25%, thus 50%) and we see at which point it reaches 1.
		let mut multiplier = Multiplier::from_u32(2);
		let mut blocks = 0;

		let mut t: sp_io::TestExternalities = frame_system::GenesisConfig::<Runtime>::default()
			.build_storage()
			.unwrap()
			.into();
		// set the minimum
		t.execute_with(|| {
			pallet_transaction_payment::NextFeeMultiplier::<Runtime>::set(multiplier);
		});

		while multiplier > Multiplier::from_u32(0) {
			t.execute_with(|| {
				// this will update the multiplier.
				TransactionPayment::on_finalize(1);
				let next = TransactionPayment::next_fee_multiplier();

				assert!(next < multiplier, "{:?} !>= {:?}", next, multiplier);
				multiplier = next;

				println!("block = {} / multiplier {:?}", blocks, multiplier);
			});
			blocks += 1;
		}
	}

	#[test]
	fn ensure_xcm_metadata_is_correct() {
		let path = xcm::VersionedXcm::<()>::type_info().path;
		// Ensure that the name doesn't include `staging` (from the pallet name)
		assert_eq!(vec!["xcm", "VersionedXcm"], path.segments);
	}
}
