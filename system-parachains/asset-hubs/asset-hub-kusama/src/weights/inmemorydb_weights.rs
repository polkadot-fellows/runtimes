//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 49.1.0
//! DATE: 2025-09-10 (Y/M/D)
//! HOSTNAME: `versi-developer-0`, CPU: `Intel(R) Xeon(R) CPU @ 2.60GHz`
//!
//! DATABASE: `InMemoryDb`, RUNTIME: `Polkadot Asset Hub`
//! BLOCK-NUM: `BlockId::Number(9640792)`
//! SKIP-WRITE: `false`, SKIP-READ: `false`, WARMUPS: `1`
//! STATE-VERSION: `V1`, STATE-CACHE-SIZE: ``
//! WEIGHT-PATH: ``
//! METRIC: `Average`, WEIGHT-MUL: `1.0`, WEIGHT-ADD: `0`

// Executed Command:
//   ./target/production/polkadot-parachain
//   benchmark
//   storage
//   --warmups
//   1
//   --state-version
//   1
//   --base-path
//   /opt/local-ssd/polkadot-asset-hub/
//   --chain
//   cumulus/polkadot-parachain/chain-specs/asset-hub-polkadot.json
//   --detailed-log-output
//   --enable-trie-cache
//   --trie-cache-size
//   10737418240
//   --batch-size
//   10000
//   --mode
//   validate-block
//   --validate-block-rounds
//   100

/// Storage DB weights for the `Polkadot Asset Hub` runtime and `InMemoryDb`.
pub mod constants {
	use frame_support::weights::constants;
	use sp_core::parameter_types;
	use sp_weights::RuntimeDbWeight;

	parameter_types! {
		/// `InMemoryDb` weights are measured in the context of the validation functions.
		/// To avoid submitting overweight blocks to the relay chain this is the configuration
		/// parachains should use.
		pub const InMemoryDbWeight: RuntimeDbWeight = RuntimeDbWeight {
			// Time to read one storage item.
			// Calculated by multiplying the *Average* of all values with `1.0` and adding `0`.
			//
			// Stats nanoseconds:
			//   Min, Max: 13_036, 14_636
			//   Average:  13_701
			//   Median:   13_739
			//   Std-Dev:  327.35
			//
			// Percentiles nanoseconds:
			//   99th: 14_322
			//   95th: 14_185
			//   75th: 13_962
			read: 13_701 * constants::WEIGHT_REF_TIME_PER_NANOS,

			// Time to write one storage item.
			// Calculated by multiplying the *Average* of all values with `1.0` and adding `0`.
			//
			// Stats nanoseconds:
			//   Min, Max: 31_957, 34_238
			//   Average:  33_060
			//   Median:   33_048
			//   Std-Dev:  230.45
			//
			// Percentiles nanoseconds:
			//   99th: 33_927
			//   95th: 33_440
			//   75th: 33_157
			write: 33_060 * constants::WEIGHT_REF_TIME_PER_NANOS,
		};
	}

	#[cfg(test)]
	mod test_db_weights {
		use super::InMemoryDbWeight as W;
		use sp_weights::constants;

		/// Checks that all weights exist and have sane values.
		// NOTE: If this test fails but you are sure that the generated values are fine,
		// you can delete it.
		#[test]
		fn bound() {
			// At least 1 µs.
			assert!(
				W::get().reads(1).ref_time() >= constants::WEIGHT_REF_TIME_PER_MICROS,
				"Read weight should be at least 1 µs."
			);
			assert!(
				W::get().writes(1).ref_time() >= constants::WEIGHT_REF_TIME_PER_MICROS,
				"Write weight should be at least 1 µs."
			);
			// At most 1 ms.
			assert!(
				W::get().reads(1).ref_time() <= constants::WEIGHT_REF_TIME_PER_MILLIS,
				"Read weight should be at most 1 ms."
			);
			assert!(
				W::get().writes(1).ref_time() <= constants::WEIGHT_REF_TIME_PER_MILLIS,
				"Write weight should be at most 1 ms."
			);
		}
	}
}
