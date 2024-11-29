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

use polkadot_primitives::AccountId;
use polkadot_runtime::xcm_config::SovereignAccountOf;
use sp_core::crypto::Ss58Codec;
use xcm::prelude::*;
use xcm_runtime_apis::conversions::LocationToAccountHelper;

const ALICE: [u8; 32] = [1u8; 32];

#[test]
fn location_conversion_works() {
	// the purpose of hardcoded values is to catch an unintended location conversion logic change.
	struct TestCase {
		description: &'static str,
		location: Location,
		expected_account_id_str: &'static str,
	}

	let test_cases = vec![
		// DescribeTerminus
		TestCase {
			description: "DescribeTerminus Child",
			location: Location::new(0, [Parachain(1111)]),
			expected_account_id_str: "5Ec4AhP4h37t7TFsAZ4HhFq6k92usAAJDUC3ADSZ4H4Acru3",
		},
		// DescribePalletTerminal
		TestCase {
			description: "DescribePalletTerminal Child",
			location: Location::new(0, [Parachain(1111), PalletInstance(50)]),
			expected_account_id_str: "5FjEBrKn3STAFsZpQF4jzwxUYHNGnNgzdZqSQfTzeJ82XKp6",
		},
		// DescribeAccountId32Terminal
		TestCase {
			description: "DescribeAccountId32Terminal Child",
			location: Location::new(
				0,
				[
					Parachain(1111),
					Junction::AccountId32 { network: None, id: AccountId::from(ALICE).into() },
				],
			),
			expected_account_id_str: "5D6CDyPd9Mya81xFN3nChiKqLvUzd8zS9fwKhfCW6FtJKjS2",
		},
		// DescribeAccountKey20Terminal
		TestCase {
			description: "DescribeAccountKey20Terminal Child",
			location: Location::new(
				0,
				[Parachain(1111), AccountKey20 { network: None, key: [123u8; 20] }],
			),
			expected_account_id_str: "5DEZsy7tsnNXB7ehLGkF8b4EUqfLQWqEzGiy2RrneC8uRNMK",
		},
		// DescribeTreasuryVoiceTerminal
		TestCase {
			description: "DescribeTreasuryVoiceTerminal Child",
			location: Location::new(
				0,
				[Parachain(1111), Plurality { id: BodyId::Treasury, part: BodyPart::Voice }],
			),
			expected_account_id_str: "5GenE4vJgHvwYVcD6b4nBvH5HNY4pzpVHWoqwFpNMFT7a2oX",
		},
		// DescribeBodyTerminal
		TestCase {
			description: "DescribeBodyTerminal Child",
			location: Location::new(
				0,
				[Parachain(1111), Plurality { id: BodyId::Unit, part: BodyPart::Voice }],
			),
			expected_account_id_str: "5DPgGBFTTYm1dGbtB1VWHJ3T3ScvdrskGGx6vSJZNP1WNStV",
		},
	];

	for tc in test_cases {
		let expected =
			AccountId::from_string(tc.expected_account_id_str).expect("Invalid AccountId string");

		let got = LocationToAccountHelper::<AccountId, SovereignAccountOf>::convert_location(
			tc.location.into(),
		)
		.unwrap();

		assert_eq!(got, expected, "{}", tc.description);
	}
}
