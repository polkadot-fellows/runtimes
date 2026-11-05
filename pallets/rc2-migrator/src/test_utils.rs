// Copyright (C) Polkadot Fellows.
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

//! Helpers for the runtimes' tests.

use alloc::{vec, vec::Vec};
use codec::Decode;
use scale_info::{TypeDef, TypeInfo};
use sp_runtime::traits::TrailingZeroInput;

/// A call of a runtime, by pallet and call name.
pub type CallName = (&'static str, &'static str);

/// Every call of a runtime, decoded from zero bytes, and the names of those whose arguments do not
/// decode from zeros.
pub fn every_call<Call: TypeInfo + Decode>() -> (Vec<(CallName, Call)>, Vec<CallName>) {
	let TypeDef::Variant(pallets) = Call::type_info().type_def else {
		panic!("the runtime call is an enum")
	};
	let (mut calls, mut skipped) = (vec![], vec![]);
	for pallet in pallets.variants {
		let TypeDef::Variant(variants) = pallet.fields[0].ty.type_info().type_def else {
			panic!("a pallet's calls are an enum")
		};
		for variant in variants.variants {
			// The pallet and call indices, then zeros for the arguments.
			match Call::decode(&mut TrailingZeroInput::new(&[pallet.index, variant.index])) {
				Ok(call) => calls.push(((pallet.name, variant.name), call)),
				Err(_) => skipped.push((pallet.name, variant.name)),
			}
		}
	}
	(calls, skipped)
}
