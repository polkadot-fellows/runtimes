// This file is part of Substrate.

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

//! Tests for the AccountTranslator trait implementation.

#[cfg(test)]
mod tests {
	use super::*;
	use codec::Encode;
	use sp_runtime::AccountId32;

	#[test]
	fn test_try_translate_rc_sovereign_to_ah_with_valid_para_account() {
		// Create a valid parachain sovereign account
		let para_id = 1000u16;
		let mut raw_account = [0u8; 32];
		raw_account[0..4].copy_from_slice(b"para");
		raw_account[4..6].copy_from_slice(&para_id.encode());
		// The remaining bytes are already zero (26 zero bytes as expected)

		let rc_account = AccountId32::from(raw_account);

		// Test translation
		let (ah_account, extracted_para_id) = crate::translate_rc_sovereign_to_ah(&rc_account);

		assert!(extracted_para_id.is_some());
		let extracted_para_id = extracted_para_id.unwrap();

		// Verify the para_id was extracted correctly
		assert_eq!(extracted_para_id, para_id);

		// Verify the AH account has the correct "sibl" prefix
		let ah_raw: &[u8; 32] = ah_account.as_ref();
		assert_eq!(&ah_raw[0..4], b"sibl");
		assert_eq!(&ah_raw[4..6], &para_id.encode());

		// Verify the remaining bytes are zeros
		assert_eq!(&ah_raw[6..], &[0u8; 26]);
	}

	#[test]
	fn test_try_translate_rc_sovereign_to_ah_with_invalid_prefix() {
		// Create an account with invalid prefix
		let mut raw_account = [0u8; 32];
		raw_account[0..4].copy_from_slice(b"inva");

		let rc_account = AccountId32::from(raw_account);

		// Test translation
		let (ah_account, extracted_para_id) = crate::translate_rc_sovereign_to_ah(&rc_account);

		assert!(extracted_para_id.is_none());
		assert_eq!(ah_account, rc_account); // Should return original account
	}

	#[test]
	fn test_try_translate_rc_sovereign_to_ah_with_non_zero_suffix() {
		// Create an account with "para" prefix but non-zero suffix
		let para_id = 1000u16;
		let mut raw_account = [0u8; 32];
		raw_account[0..4].copy_from_slice(b"para");
		raw_account[4..6].copy_from_slice(&para_id.encode());
		raw_account[31] = 1; // Set last byte to non-zero

		let rc_account = AccountId32::from(raw_account);

		// Test translation
		let (ah_account, extracted_para_id) = crate::translate_rc_sovereign_to_ah(&rc_account);

		assert!(extracted_para_id.is_none());
		assert_eq!(ah_account, rc_account); // Should return original account
	}

	#[test]
	fn test_try_translate_rc_sovereign_to_ah_with_regular_account() {
		// Create a regular account (not a parachain sovereign account)
		let regular_account = AccountId32::from([1u8; 32]);

		// Test translation
		let (ah_account, extracted_para_id) = crate::translate_rc_sovereign_to_ah(&regular_account);

		assert!(extracted_para_id.is_none());
		assert_eq!(ah_account, regular_account); // Should return original account
	}

	#[test]
	fn test_try_translate_rc_sovereign_to_ah_with_various_para_ids() {
		// Test with different para IDs
		let para_ids = [0u16, 1u16, 1000u16, 2000u16, 65535u16];

		for para_id in para_ids {
			let mut raw_account = [0u8; 32];
			raw_account[0..4].copy_from_slice(b"para");
			raw_account[4..6].copy_from_slice(&para_id.encode());

			let rc_account = AccountId32::from(raw_account);

			// Test translation
			let (ah_account, extracted_para_id) = crate::translate_rc_sovereign_to_ah(&rc_account);

			assert!(extracted_para_id.is_some());
			let extracted_para_id = extracted_para_id.unwrap();

			// Verify the para_id was extracted correctly
			assert_eq!(extracted_para_id, para_id);

			// Verify the AH account has the correct "sibl" prefix
			let ah_raw: &[u8; 32] = ah_account.as_ref();
			assert_eq!(&ah_raw[0..4], b"sibl");
			assert_eq!(&ah_raw[4..6], &para_id.encode());
		}
	}

	#[test]
	fn test_roundtrip_consistency() {
		// Test that we can't accidentally translate an already translated account
		let para_id = 1000u16;
		let mut raw_account = [0u8; 32];
		raw_account[0..4].copy_from_slice(b"para");
		raw_account[4..6].copy_from_slice(&para_id.encode());

		let rc_account = AccountId32::from(raw_account);

		// First translation
		let (ah_account, para_id1) = crate::translate_rc_sovereign_to_ah(&rc_account);
		assert!(para_id1.is_some());

		// Try to translate the already translated account
		let (ah_account2, para_id2) = crate::translate_rc_sovereign_to_ah(&ah_account);
		assert!(
			para_id2.is_none(),
			"Should not be able to translate an already translated account"
		);
		assert_eq!(ah_account, ah_account2); // Should return the same account
	}
}
