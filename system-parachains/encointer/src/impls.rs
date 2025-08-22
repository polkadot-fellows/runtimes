// Copyright (c) 2023 Encointer Association
// This file is part of Encointer
//
// Encointer is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Encointer is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Encointer.  If not, see <http://www.gnu.org/licenses/>.

//! Some trait implementations for the encointer-kusama-runtime.

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarks {
	use crate::{ParachainSystem};
	use core::marker::PhantomData;
	use cumulus_primitives_core::{ChannelStatus, GetChannelInfo};
	use frame_support::traits::{
		tokens::{PaymentStatus},
	};
	use pallet_encointer_treasuries::Transfer;
	use sp_core::Get;
	use xcm::opaque::latest::Junction::Parachain;
    use crate::PolkadotXcm;
    use crate::RuntimeOrigin;
    use xcm::latest::Location;
    use xcm::GetVersion;

    /// Trait for setting up any prerequisites for successful execution of benchmarks.
	pub trait EnsureSuccessful {
		fn ensure_successful();
	}

	/// Implementation of the [`EnsureSuccessful`] trait which opens an HRMP channel between
	/// the Collectives and a parachain with a given ID.
	pub struct OpenHrmpChannel<I>(PhantomData<I>);
	impl<I: Get<u32>> EnsureSuccessful for OpenHrmpChannel<I> {
		fn ensure_successful() {
			let para_id = I::get();

			// open HRMP channel
			if let ChannelStatus::Closed = ParachainSystem::get_channel_status(para_id.into()) {
				ParachainSystem::open_outbound_hrmp_channel_for_benchmarks_or_tests(para_id.into())
			}

			// set XCM version for sibling parachain
			let sibling_parachain = Location::new(1, [Parachain(para_id)]);
			if PolkadotXcm::get_version_for(&sibling_parachain).is_none() {
				if let Err(e) = PolkadotXcm::force_xcm_version(
					RuntimeOrigin::root(),
					sibling_parachain.into(),
					system_parachains_constants::genesis_presets::SAFE_XCM_VERSION,
				) {
					log::error!(
						"Failed to `force_xcm_version` for para_id: {para_id:?}, error: {e:?}"
					);
				}
			}
		}
	}

	/// Type that wraps a type implementing the [`Transfer`] trait to decorate its
	/// [`Transfer::ensure_successful`] function with a provided implementation of the
	/// [`EnsureSuccessful`] trait.
	pub struct TransferWithEnsure<O, E>(PhantomData<(O, E)>);
	impl<O, E> Transfer for TransferWithEnsure<O, E>
	where
		O: Transfer,
		E: EnsureSuccessful,
	{
		type AssetKind = O::AssetKind;
		type Payer = O::Payer;
		type Balance = O::Balance;
		type Beneficiary = O::Beneficiary;
		type Error = O::Error;
		type Id = O::Id;

		fn transfer(
			from: &Self::Payer,
			to: &Self::Beneficiary,
			asset_kind: Self::AssetKind,
			amount: Self::Balance,
		) -> Result<Self::Id, Self::Error> {
			O::transfer(from, to, asset_kind, amount)
		}
		fn check_payment(id: Self::Id) -> PaymentStatus {
			O::check_payment(id)
		}
		fn ensure_successful(
			from: &Self::Payer,
			to: &Self::Beneficiary,
			asset_kind: Self::AssetKind,
			amount: Self::Balance,
		) {
			E::ensure_successful();
			O::ensure_successful(from, to, asset_kind, amount)
		}
		fn ensure_concluded(id: Self::Id) {
			O::ensure_concluded(id)
		}
	}
}
