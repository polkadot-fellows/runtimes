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

//! Stepped curve implementation for gradual value transitions.
//!
//! This module provides a stepped curve system that allows values to change over time
//! in discrete steps, moving asymptotically towards a target value.
//!
//! Originally extracted from https://github.com/paritytech/polkadot-sdk/pull/9556
//! To be replaced after that is merged and available.

use frame_support::pallet_prelude::{CheckedDiv, Zero};
use scale_info::TypeInfo;
use sp_arithmetic::{traits::Saturating, FixedU128};
use sp_runtime::{
	traits::{Bounded, One},
	FixedPointNumber, Perbill, SaturatedConversion,
};

/// The step type for the stepped curve.
#[derive(PartialEq, Eq, sp_core::RuntimeDebug, TypeInfo, Clone)]
pub enum Step {
	/// Move towards a desired value by a percentage of the remaining difference at each step.
	///
	/// Step size will be (target_total - current_value) * pct.
	RemainingPct {
		/// The asymptote the curve will move towards.
		target: FixedU128,
		/// The percentage closer to the `target` at each step.
		pct: Perbill,
	},
}

/// A stepped curve.
///
/// Steps every `period` from the `initial_value` as defined by `step`.
/// First step from `initial_value` takes place at `start` + `period`.
#[derive(PartialEq, Eq, sp_core::RuntimeDebug, TypeInfo, Clone)]
pub struct SteppedCurve {
	/// The starting point for the curve.
	pub start: FixedU128,
	/// The initial value of the curve at the `start` point.
	pub initial_value: FixedU128,
	/// The change to apply at the end of each `period`.
	pub step: Step,
	/// The duration of each step.
	pub period: FixedU128,
}

impl SteppedCurve {
	/// Creates a new `SteppedCurve`.
	pub fn new(start: FixedU128, initial_value: FixedU128, step: Step, period: FixedU128) -> Self {
		Self { start, initial_value, step, period }
	}

	/// Returns the magnitude of the step size occuring at the start of this point's period.
	/// If no step has occured, will return 0.
	///
	/// Ex. In period 4, the last step taken was 10 -> 7, it would return 3.
	pub fn last_step_size(&self, point: FixedU128) -> FixedU128 {
		// No step taken yet.
		if point <= self.start {
			return Zero::zero();
		}

		// If the period is zero, the value never changes.
		if self.period.is_zero() {
			return Zero::zero();
		}

		// Calculate how many full periods have passed, saturate.
		let num_periods =
			(point - self.start).checked_div(&self.period).unwrap_or(FixedU128::max_value());

		// No periods have passed.
		if num_periods < One::one() {
			return Zero::zero();
		}

		// Points for calculating step difference.
		let prev_period_point = self
			.start
			.saturating_add((num_periods - One::one()).saturating_mul(self.period));
		let curr_period_point = self.start.saturating_add(num_periods.saturating_mul(self.period));

		// Evaluate the curve at those two points.
		let val_prev = self.evaluate(prev_period_point);
		let val_curr = self.evaluate(curr_period_point);

		if val_curr >= val_prev {
			val_curr.saturating_sub(val_prev)
		} else {
			val_prev.saturating_sub(val_curr)
		}
	}

	/// Evaluate the curve at a given point.
	///
	/// Max number of steps is `u32::MAX`.
	pub fn evaluate(&self, point: FixedU128) -> FixedU128 {
		let initial = self.initial_value;

		// If the point is before the curve starts, return the initial value.
		if point <= self.start {
			return initial;
		}

		// If the period is zero, the value never changes.
		if self.period.is_zero() {
			return initial;
		}

		// Calculate how many full periods have passed, downsampled to usize.
		let num_periods =
			(point - self.start).checked_div(&self.period).unwrap_or(FixedU128::max_value());
		let num_periods_u32 = (num_periods.into_inner() / FixedU128::DIV).saturated_into::<u32>();

		// No periods have passed.
		if num_periods_u32.is_zero() {
			return initial;
		}

		match self.step {
			Step::RemainingPct { target: asymptote, pct: percent } => {
				// asymptote +/- diff(asymptote, initial_value) * (1-percent)^num_periods.
				let ratio = FixedU128::one().saturating_sub(FixedU128::from_perbill(percent));
				let scale = ratio.saturating_pow(num_periods_u32 as usize);

				if initial >= asymptote {
					let diff = initial.saturating_sub(asymptote);
					asymptote.saturating_add(diff.saturating_mul(scale))
				} else {
					let diff = asymptote.saturating_sub(initial);
					asymptote.saturating_sub(diff.saturating_mul(scale))
				}
			},
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn curve_returns_initial_value_before_start() {
		let curve = SteppedCurve::new(
			FixedU128::from_u32(100), // start at 100
			FixedU128::from_u32(50),  // initial value of 50
			Step::RemainingPct {
				// get 10% closer to 100 on each step
				target: FixedU128::from_u32(100),
				pct: Perbill::from_percent(10),
			},
			FixedU128::from_u32(10), // period of 10
		);

		// Before start
		assert_eq!(curve.evaluate(FixedU128::from_u32(50)), FixedU128::from_u32(50));
		assert_eq!(curve.evaluate(FixedU128::from_u32(99)), FixedU128::from_u32(50));
		// At start (should still be initial value, no period elapsed)
		assert_eq!(curve.evaluate(FixedU128::from_u32(100)), FixedU128::from_u32(50));
	}

	#[test]
	fn curve_never_changes_with_zero_period() {
		let curve = SteppedCurve::new(
			FixedU128::from_u32(0),
			FixedU128::from_u32(100),
			Step::RemainingPct { target: FixedU128::from_u32(200), pct: Perbill::from_percent(50) },
			FixedU128::zero(), // zero period
		);

		// Any point should return initial value
		assert_eq!(curve.evaluate(FixedU128::from_u32(0)), FixedU128::from_u32(100));
		assert_eq!(curve.evaluate(FixedU128::from_u32(1000)), FixedU128::from_u32(100));
		assert_eq!(curve.evaluate(FixedU128::from_u32(1_000_000)), FixedU128::from_u32(100));
	}

	#[test]
	fn first_step_occurs_at_start_plus_period() {
		let curve = SteppedCurve::new(
			FixedU128::from_u32(100), // start
			FixedU128::from_u32(80),  // initial (moving down towards 50)
			Step::RemainingPct {
				target: FixedU128::from_u32(50),
				pct: Perbill::from_percent(20), // 20% closer each step
			},
			FixedU128::from_u32(10), // period
		);

		// Just before first step (still in period 0)
		assert_eq!(curve.evaluate(FixedU128::from_u32(109)), FixedU128::from_u32(80));

		// At first step (start + period = 110)
		// Calculation: 50 + (80 - 50) * (1 - 0.20)^1 = 50 + 30 * 0.8 = 50 + 24 = 74
		let first_step_value = curve.evaluate(FixedU128::from_u32(110));
		assert_eq!(first_step_value, FixedU128::from_u32(74));
	}

	#[test]
	fn remaining_pct_increases_towards_target() {
		// Start at 100, target 200, 50% closer each step
		let curve = SteppedCurve::new(
			FixedU128::zero(),
			FixedU128::from_u32(100),
			Step::RemainingPct {
				target: FixedU128::from_u32(200),
				pct: Perbill::from_percent(50), // 50% of remaining distance
			},
			FixedU128::from_u32(1), // period of 1
		);

		// Period 0: before first step
		assert_eq!(curve.evaluate(FixedU128::from_rational(5, 10)), FixedU128::from_u32(100));

		// Period 1: 200 - (200 - 100) * 0.5^1 = 200 - 50 = 150
		assert_eq!(curve.evaluate(FixedU128::from_u32(1)), FixedU128::from_u32(150));

		// Period 2: 200 - (200 - 100) * 0.5^2 = 200 - 25 = 175
		assert_eq!(curve.evaluate(FixedU128::from_u32(2)), FixedU128::from_u32(175));

		// Period 3: 200 - (200 - 100) * 0.5^3 = 200 - 12.5 = 187.5
		assert_eq!(curve.evaluate(FixedU128::from_u32(3)), FixedU128::from_rational(1875, 10));

		// Period 4: 200 - (200 - 100) * 0.5^4 = 200 - 6.25 = 193.75
		assert_eq!(curve.evaluate(FixedU128::from_u32(4)), FixedU128::from_rational(19375, 100));
	}

	#[test]
	fn remaining_pct_decreases_towards_target() {
		// Start at 200, target 100, 50% closer each step
		let curve = SteppedCurve::new(
			FixedU128::zero(),
			FixedU128::from_u32(200),
			Step::RemainingPct { target: FixedU128::from_u32(100), pct: Perbill::from_percent(50) },
			FixedU128::from_u32(1),
		);

		// Period 0: before first step
		assert_eq!(curve.evaluate(FixedU128::from_rational(5, 10)), FixedU128::from_u32(200));

		// Period 1: 100 + (200 - 100) * 0.5^1 = 100 + 50 = 150
		assert_eq!(curve.evaluate(FixedU128::from_u32(1)), FixedU128::from_u32(150));

		// Period 2: 100 + (200 - 100) * 0.5^2 = 100 + 25 = 125
		assert_eq!(curve.evaluate(FixedU128::from_u32(2)), FixedU128::from_u32(125));

		// Period 3: 100 + (200 - 100) * 0.5^3 = 100 + 12.5 = 112.5
		assert_eq!(curve.evaluate(FixedU128::from_u32(3)), FixedU128::from_rational(1125, 10));
	}

	#[test]
	fn curve_approaches_but_never_reaches_target() {
		let curve = SteppedCurve::new(
			FixedU128::zero(),
			FixedU128::from_u32(0),
			Step::RemainingPct { target: FixedU128::from_u32(100), pct: Perbill::from_percent(50) },
			FixedU128::from_u32(1),
		);

		// After 10 periods: 100 - 100 * 0.5^10 = 100 - 0.09765625 ≈ 99.9
		let value_10 = curve.evaluate(FixedU128::from_u32(10));
		assert!(value_10 < FixedU128::from_u32(100));
		assert!(value_10 > FixedU128::from_u32(99));

		// After 20 periods: even closer
		let value_20 = curve.evaluate(FixedU128::from_u32(20));
		assert!(value_20 < FixedU128::from_u32(100));
		assert!(value_20 > value_10);
	}

	#[test]
	fn last_step_size_zero_before_first_step() {
		let curve = SteppedCurve::new(
			FixedU128::from_u32(100),
			FixedU128::from_u32(50),
			Step::RemainingPct { target: FixedU128::from_u32(100), pct: Perbill::from_percent(10) },
			FixedU128::from_u32(10),
		);

		// Before start
		assert_eq!(curve.last_step_size(FixedU128::from_u32(50)), FixedU128::zero());
		assert_eq!(curve.last_step_size(FixedU128::from_u32(100)), FixedU128::zero());

		// Still in first period (before start + period)
		assert_eq!(curve.last_step_size(FixedU128::from_u32(105)), FixedU128::zero());
		assert_eq!(curve.last_step_size(FixedU128::from_u32(109)), FixedU128::zero());
	}

	#[test]
	fn last_step_size_correct_at_first_step() {
		let curve = SteppedCurve::new(
			FixedU128::from_u32(100),
			FixedU128::from_u32(50),
			Step::RemainingPct {
				target: FixedU128::from_u32(100),
				pct: Perbill::from_percent(20), // 20% closer
			},
			FixedU128::from_u32(10),
		);

		// At first step (110): 100 - (100 - 50) * 0.8 = 100 - 40 = 60
		// Step size: 60 - 50 = 10
		assert_eq!(curve.last_step_size(FixedU128::from_u32(110)), FixedU128::from_u32(10));

		// Anywhere in the first period after the step, should still report the same step size
		assert_eq!(curve.last_step_size(FixedU128::from_u32(115)), FixedU128::from_u32(10));

		// Second period:
		// At second step (120): 100 - (100 - 50) * 0.8^2 = 100 - 32 = 68
		// Step size: 68 - 60 = 8
		assert_eq!(curve.last_step_size(FixedU128::from_u32(120)), FixedU128::from_u32(8));
	}

	#[test]
	fn last_step_size_decreases_over_time() {
		let curve = SteppedCurve::new(
			FixedU128::zero(),
			FixedU128::from_u32(0),
			Step::RemainingPct { target: FixedU128::from_u32(100), pct: Perbill::from_percent(50) },
			FixedU128::from_u32(1),
		);

		// Step 1: 0 -> 50, size = 50
		let step1 = curve.last_step_size(FixedU128::from_u32(1));
		assert_eq!(step1, FixedU128::from_u32(50));

		// Step 2: 50 -> 75, size = 25
		let step2 = curve.last_step_size(FixedU128::from_u32(2));
		assert_eq!(step2, FixedU128::from_u32(25));

		// Step 3: 75 -> 87.5, size = 12.5
		let step3 = curve.last_step_size(FixedU128::from_u32(3));
		assert_eq!(step3, FixedU128::from_rational(125, 10));
	}
}
