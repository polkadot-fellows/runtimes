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

/// Polkadot-specific constants for the Bulletin parachain.
pub mod polkadot_constants {
	use polkadot_core_primitives::Balance;

	/// Consensus-related constants.
	pub mod consensus {
		use frame_support::weights::{constants::WEIGHT_REF_TIME_PER_SECOND, Weight};

		/// Maximum number of blocks simultaneously accepted by the Runtime, not yet included
		/// into the relay chain.
		pub const UNINCLUDED_SEGMENT_CAPACITY: u32 = 1;
		/// How many parachain blocks are processed by the relay chain per parent. Limits the
		/// number of blocks authored per slot.
		pub const BLOCK_PROCESSING_VELOCITY: u32 = 1;
		/// Relay chain slot duration, in milliseconds.
		pub const RELAY_CHAIN_SLOT_DURATION_MILLIS: u32 = 6000;

		/// Maximum block weight for Polkadot system parachains.
		/// We allow for 2 seconds of compute with a 6 second average block.
		pub const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
			WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2),
			cumulus_primitives_core::relay_chain::MAX_POV_SIZE as u64,
		);
	}

	// TODO: check fees this and align with some System parachain (not AH)
	/// Currency-related constants for Polkadot (DOT).
	pub mod currency {
		use super::Balance;

		/// One "DOT" that a UI would show a user.
		pub const UNITS: Balance = 10_000_000_000;
		pub const DOLLARS: Balance = UNITS; // 10_000_000_000
		pub const CENTS: Balance = DOLLARS / 100; // 100_000_000
		pub const MILLICENTS: Balance = CENTS / 1_000; // 100_000

		/// The existential deposit for Polkadot system parachains.
		/// 1/10th of the Relay Chain's existential deposit.
		pub const EXISTENTIAL_DEPOSIT: Balance = UNITS / 100; // 0.01 DOT = 100_000_000

		/// Deposit rate for stored data. 1/100th of the Relay Chain's deposit rate.
		pub const fn deposit(items: u32, bytes: u32) -> Balance {
			// Polkadot relay deposit rate: items * 20 * DOLLARS + bytes * 100 * MILLICENTS
			// System parachain rate: 1/100th of that
			(items as Balance * 20 * DOLLARS + bytes as Balance * 100 * MILLICENTS) / 100
		}
	}

	/// Fee-related constants.
	pub mod fee {
		use super::Balance;
		use frame_support::{
			pallet_prelude::Weight,
			weights::{
				constants::ExtrinsicBaseWeight, FeePolynomial, WeightToFeeCoefficient,
				WeightToFeeCoefficients, WeightToFeePolynomial,
			},
		};
		use smallvec::smallvec;
		pub use sp_runtime::Perbill;

		/// Handles converting a weight scalar to a fee value, based on the scale and granularity
		/// of the node's balance type.
		pub struct WeightToFee;
		impl frame_support::weights::WeightToFee for WeightToFee {
			type Balance = Balance;

			fn weight_to_fee(weight: &Weight) -> Self::Balance {
				let time_poly: FeePolynomial<Balance> = RefTimeToFee::polynomial().into();
				let proof_poly: FeePolynomial<Balance> = ProofSizeToFee::polynomial().into();

				// Take the maximum instead of the sum to charge by the more scarce resource.
				time_poly.eval(weight.ref_time()).max(proof_poly.eval(weight.proof_size()))
			}
		}

		/// Maps the reference time component of `Weight` to a fee.
		pub struct RefTimeToFee;
		impl WeightToFeePolynomial for RefTimeToFee {
			type Balance = Balance;
			fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
				// In Polkadot, extrinsic base weight (smallest non-zero weight) is mapped to 1/10
				// CENT: The standard system parachain configuration is 1/20 of that, as in
				// 1/200 CENT.
				let p = super::currency::CENTS;
				let q = 200 * Balance::from(ExtrinsicBaseWeight::get().ref_time());

				smallvec![WeightToFeeCoefficient {
					degree: 1,
					negative: false,
					coeff_frac: Perbill::from_rational(p % q, q),
					coeff_integer: p / q,
				}]
			}
		}

		/// Maps the proof size component of `Weight` to a fee.
		pub struct ProofSizeToFee;
		impl WeightToFeePolynomial for ProofSizeToFee {
			type Balance = Balance;
			fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
				// Map 20kb proof to 1 CENT.
				let p = super::currency::CENTS;
				let q = 20_000;

				smallvec![WeightToFeeCoefficient {
					degree: 1,
					negative: false,
					coeff_frac: Perbill::from_rational(p % q, q),
					coeff_integer: p / q,
				}]
			}
		}
	}

	/// Time-related constants.
	pub mod time {
		use parachains_common::BlockNumber;

		/// This determines the average expected block time that we are targeting.
		/// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
		/// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
		/// up by `pallet_aura` to implement `fn slot_duration()`.
		///
		/// Change this to adjust the block time.
		pub const MILLISECS_PER_BLOCK: u64 = 6000;

		// NOTE: Currently it is not possible to change the slot duration after the chain has
		// started.       Attempting to do so will brick block production.
		pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

		// Time is measured by number of blocks.
		pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
		pub const HOURS: BlockNumber = MINUTES * 60;
		pub const DAYS: BlockNumber = HOURS * 24;
	}

	/// XCM version to use in genesis.
	pub const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;
}
