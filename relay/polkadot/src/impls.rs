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

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarks {
	use crate::{xcm_config::CheckAccount, Balances, ExistentialDeposit};
	use frame_support::{
		dispatch::RawOrigin,
		traits::{Currency, EnsureOrigin},
	};

	pub struct InitializeReaperForBenchmarking<A, E>(core::marker::PhantomData<(A, E)>);
	impl<A, O: Into<Result<RawOrigin<A>, O>> + From<RawOrigin<A>>, E: EnsureOrigin<O>>
		EnsureOrigin<O> for InitializeReaperForBenchmarking<A, E>
	{
		type Success = E::Success;

		fn try_origin(o: O) -> Result<E::Success, O> {
			E::try_origin(o)
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn try_successful_origin() -> Result<O, ()> {
			// initialize the XCM Check Account with the existential deposit
			Balances::make_free_balance_be(&CheckAccount::get(), ExistentialDeposit::get());

			// call the real implementation
			E::try_successful_origin()
		}
	}
}
