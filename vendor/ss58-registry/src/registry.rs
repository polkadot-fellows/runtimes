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

use super::*;

include!(concat!(env!("OUT_DIR"), "/registry_gen.rs"));

#[cfg(feature = "std")]
impl std::fmt::Display for Ss58AddressFormatRegistry {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		let lookup = PREFIX_TO_INDEX
			.binary_search_by_key(&from_known_address_format(*self), |(prefix, _)| *prefix)
			.expect("always be found");
		let (_, idx) = PREFIX_TO_INDEX[lookup];
		write!(f, "{}", ALL_SS58_ADDRESS_FORMAT_NAMES[idx])
	}
}

impl TryFrom<Ss58AddressFormat> for Ss58AddressFormatRegistry {
	type Error = ParseError;

	fn try_from(x: Ss58AddressFormat) -> Result<Ss58AddressFormatRegistry, ParseError> {
		PREFIX_TO_INDEX
			.binary_search_by_key(&x.prefix(), |(prefix, _)| *prefix)
			.map(|lookup| {
				let (_, idx) = PREFIX_TO_INDEX[lookup];
				ALL_SS58_ADDRESS_FORMATS[idx]
			})
			.map_err(|_| ParseError)
	}
}

/// const function to convert [`Ss58AddressFormat`] to u16
pub const fn from_known_address_format(x: Ss58AddressFormatRegistry) -> u16 {
	x as u16
}

impl core::fmt::Debug for TokenRegistry {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		let token: Token = (*self).into();
		f.debug_struct("TokenRegistry")
			.field("name", &token.name)
			.field("decimals", &token.decimals)
			.finish()
	}
}
