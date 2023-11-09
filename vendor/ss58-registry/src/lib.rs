// Copyright (C) 2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0
//
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
#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]
#![deny(unsafe_code)]

//! Enumerations of well-known SS58 account types and tokens used in the Polkadot ecosystem.
use core::convert::TryFrom;
mod address_format;
mod error;
mod registry;
#[cfg(test)]
mod tests;
mod token;

pub use address_format::{from_address_format, Ss58AddressFormat};
pub use error::ParseError;
pub use registry::{from_known_address_format, Ss58AddressFormatRegistry, TokenRegistry};
pub use token::{Token, TokenAmount};

#[cfg(feature = "std")]
use registry::PREFIX_TO_INDEX;
use registry::{ALL_SS58_ADDRESS_FORMATS, ALL_SS58_ADDRESS_FORMAT_NAMES};
