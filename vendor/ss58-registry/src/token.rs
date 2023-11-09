// Copyright (C) 2021-2022 Parity Technologies (UK) Ltd.
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

#[cfg(feature = "std")]
use num_format::{Locale, ToFormattedString};

/// Name and decimals of a given token.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Token {
	/// The short name (ticker) of the token
	pub name: &'static str,
	/// The number of decimals the token has (smallest granularity of the token)
	pub decimals: u8,
}

impl core::fmt::Debug for Token {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("Token")
			.field("name", &self.name)
			.field("decimals", &self.decimals)
			.finish()
	}
}

impl Token {
	/// Creates the specified amount of [`Token`] with its name and decimals filled from the
	/// [`TokenRegistry`] variant.
	///
	/// ```
	/// # use ss58_registry::{Token, TokenRegistry};
	/// # #[cfg(feature = "std")]
	/// # fn x() {
	/// let token: Token = TokenRegistry::Dot.into();
	/// let my_amount = token.amount(100_000_000);
	/// assert_eq!(format!("{}", my_amount), "0.010 DOT");
	/// assert_eq!(format!("{:?}", my_amount), "0.010 DOT (100,000,000)");
	/// # }
	/// # #[cfg(not(feature = "std"))]
	/// # fn x() {}
	/// # x();
	/// ```
	pub fn amount(&self, amount: u128) -> TokenAmount {
		TokenAmount { token: self.clone(), amount }
	}
}

/// A given amount of token. Can be used for nicely formatted output and token-aware comparison of
/// different amounts.
///
/// ```
/// # use ss58_registry::{Token, TokenAmount};
/// # #[cfg(feature = "std")]
/// # fn x() {
/// let token = Token { name: "I❤U", decimals: 8 };
/// let my_amount = token.amount(100_000_000_000);
/// assert_eq!(format!("{}", my_amount), "1,000.000 I❤U");
/// assert_eq!(format!("{:?}", my_amount), "1000.000 I❤U (100,000,000,000)");
/// # }
/// # #[cfg(not(feature = "std"))]
/// # fn x() {}
/// # x();
/// ```
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TokenAmount {
	/// The token this amount is from.
	pub token: Token,
	/// The amount in the smallest granularity of the token.
	pub amount: u128,
}

#[cfg(feature = "std")]
impl std::fmt::Display for TokenAmount {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		let multiplier = u128::pow(10, self.token.decimals as u32);
		write!(
			f,
			"{}.{:0>3} {}",
			(self.amount / multiplier).to_formatted_string(&Locale::en),
			self.amount % multiplier / (multiplier / 1000),
			self.token.name
		)
	}
}

#[cfg(feature = "std")]
impl std::fmt::Debug for TokenAmount {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		let multiplier = u128::pow(10, self.token.decimals as u32);
		write!(
			f,
			"{}.{:0>3} {} ({})",
			self.amount / multiplier,
			self.amount % multiplier / (multiplier / 1000),
			self.token.name,
			self.amount.to_formatted_string(&Locale::en),
		)
	}
}
