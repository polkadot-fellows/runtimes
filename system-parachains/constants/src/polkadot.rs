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

/// Universally recognized accounts.
pub mod account {
	use frame_support::PalletId;

	/// Polkadot treasury pallet id, used to convert into AccountId
	pub const POLKADOT_TREASURY_PALLET_ID: PalletId = PalletId(*b"py/trsry");
	/// Alliance pallet ID.
	/// Used as a temporary place to deposit a slashed imbalance before teleporting to the Treasury.
	pub const ALLIANCE_PALLET_ID: PalletId = PalletId(*b"py/allia");
	/// Referenda pallet ID.
	/// Used as a temporary place to deposit a slashed imbalance before teleporting to the Treasury.
	pub const REFERENDA_PALLET_ID: PalletId = PalletId(*b"py/refer");
	/// Ambassador Referenda pallet ID.
	/// Used as a temporary place to deposit a slashed imbalance before teleporting to the Treasury.
	pub const AMBASSADOR_REFERENDA_PALLET_ID: PalletId = PalletId(*b"py/amref");
	/// Identity pallet ID.
	/// Used as a temporary place to deposit a slashed imbalance before teleporting to the Treasury.
	pub const IDENTITY_PALLET_ID: PalletId = PalletId(*b"py/ident");
	/// Fellowship treasury pallet ID
	pub const FELLOWSHIP_TREASURY_PALLET_ID: PalletId = PalletId(*b"py/feltr");
	/// Ambassador treasury pallet ID
	pub const AMBASSADOR_TREASURY_PALLET_ID: PalletId = PalletId(*b"py/ambtr");
}

/// Consensus-related.
pub mod consensus {
	/// Maximum number of blocks simultaneously accepted by the Runtime, not yet included
	/// into the relay chain.
	pub const UNINCLUDED_SEGMENT_CAPACITY: u32 = 1;
	/// How many parachain blocks are processed by the relay chain per parent. Limits the
	/// number of blocks authored per slot.
	pub const BLOCK_PROCESSING_VELOCITY: u32 = 1;
	/// Relay chain slot duration, in milliseconds.
	pub const RELAY_CHAIN_SLOT_DURATION_MILLIS: u32 = 6000;
}

/// Constants relating to DOT.
pub mod currency {
	use polkadot_core_primitives::Balance;

	/// The default existential deposit for system chains. 1/10th of the Relay Chain's existential
	/// deposit. Individual system parachains may modify this in special cases.
	pub const SYSTEM_PARA_EXISTENTIAL_DEPOSIT: Balance =
		polkadot_runtime_constants::currency::EXISTENTIAL_DEPOSIT / 10;

	/// One "DOT" that a UI would show a user.
	pub const UNITS: Balance = 10_000_000_000;
	pub const DOLLARS: Balance = UNITS; // 10_000_000_000
	pub const GRAND: Balance = DOLLARS * 1_000; // 10_000_000_000_000
	pub const CENTS: Balance = DOLLARS / 100; // 100_000_000
	pub const MILLICENTS: Balance = CENTS / 1_000; // 100_000

	/// Deposit rate for stored data. 1/100th of the Relay Chain's deposit rate. `items` is the
	/// number of keys in storage and `bytes` is the size of the value.
	pub const fn system_para_deposit(items: u32, bytes: u32) -> Balance {
		polkadot_runtime_constants::currency::deposit(items, bytes) / 100
	}
}

/// Constants related to Polkadot fee payment.
pub mod fee {
	use frame_support::{
		pallet_prelude::Weight,
		weights::{
			constants::ExtrinsicBaseWeight, FeePolynomial, WeightToFeeCoefficient,
			WeightToFeeCoefficients, WeightToFeePolynomial,
		},
	};
	use polkadot_core_primitives::Balance;
	use smallvec::smallvec;
	pub use sp_runtime::Perbill;

	/// The block saturation level. Fees will be updates based on this value.
	pub const TARGET_BLOCK_FULLNESS: Perbill = Perbill::from_percent(25);

	/// Cost of every transaction byte at Polkadot system parachains.
	///
	/// It is the Relay Chain (Polkadot) `TransactionByteFee` / 10.
	pub const TRANSACTION_BYTE_FEE: Balance = super::currency::MILLICENTS;

	/// Handles converting a weight scalar to a fee value, based on the scale and granularity of the
	/// node's balance type.
	///
	/// This should typically create a mapping between the following ranges:
	///   - [0, MAXIMUM_BLOCK_WEIGHT]
	///   - [Balance::min, Balance::max]
	///
	/// Yet, it can be used for any other sort of change to weight-fee. Some examples being:
	///   - Setting it to `0` will essentially disable the weight fee.
	///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
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
			// In Polkadot, extrinsic base weight (smallest non-zero weight) is mapped to 1/10 CENT:
			// The standard system parachain configuration is 1/10 of that, as in 1/100 CENT.
			let p = super::currency::CENTS;
			let q = 100 * Balance::from(ExtrinsicBaseWeight::get().ref_time());

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
			// Map 10kb proof to 1 CENT.
			let p = super::currency::CENTS;
			let q = 10_000;

			smallvec![WeightToFeeCoefficient {
				degree: 1,
				negative: false,
				coeff_frac: Perbill::from_rational(p % q, q),
				coeff_integer: p / q,
			}]
		}
	}
}
