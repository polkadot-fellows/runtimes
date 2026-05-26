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

//! Dynamic parameters.

use super::*;

/// Dynamic runtime parameters configurable on-chain through [`pallet_parameters`].
#[dynamic_params(RuntimeParameters, pallet_parameters::Parameters::<Runtime>)]
pub mod dynamic_params {
	use super::*;

	/// Fellowship Salary Parameters.
	#[dynamic_pallet_params]
	#[codec(index = 0)]
	pub mod fellowship_salary {
		/// Fellowship Salary Configuration.
		///
		/// Defaults to USDT on Asset Hub (`PalletInstance(50)/GeneralIndex(1984)`) asset with 6
		/// decimals and budget value of 250,000 USDT (i.e., `250_000_000_000` means
		/// 250,000.000000 USDT).
		#[codec(index = 0)]
		pub static SalaryConfig: crate::parameters::FellowshipSalaryConfig =
			crate::parameters::FellowshipSalaryConfig {
				asset: Box::new(VersionedLocatableAsset::V5 {
					location: AssetHubLocation::get(),
					asset_id: AssetId(Location::new(0, [PalletInstance(50), GeneralIndex(1984)])),
				}),
				budget: 250_000 * 1_000_000,
			};
	}

	/// Secretary Salary Parameters.
	#[dynamic_pallet_params]
	#[codec(index = 1)]
	pub mod secretary_salary {
		/// Secretary Salary Configuration.
		///
		/// Defaults to USDT on Asset Hub (`PalletInstance(50)/GeneralIndex(1984)`) asset with 6
		/// decimals and budget value of 13,332 USDT (i.e., `13_332_000_000` means
		/// 13,332.000000 USDT) and salary for rank 1 of 6,666 USDT (i.e., `6666_000000` means
		/// 6,666.000000 USDT).
		#[codec(index = 0)]
		pub static SalaryConfig: crate::parameters::SecretarySalaryConfig =
			crate::parameters::SecretarySalaryConfig {
				asset: Box::new(VersionedLocatableAsset::V5 {
					location: AssetHubLocation::get(),
					asset_id: AssetId(Location::new(0, [PalletInstance(50), GeneralIndex(1984)])),
				}),
				budget: 13_332 * 1_000_000,
				salary_rank1: 6666 * 1_000_000,
			};
	}
}

impl pallet_parameters::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeParameters = RuntimeParameters;
	type AdminOrigin = DynamicParameterOrigin;
	type WeightInfo = weights::pallet_parameters::WeightInfo<Runtime>;
}

#[cfg(feature = "runtime-benchmarks")]
impl Default for RuntimeParameters {
	fn default() -> Self {
		RuntimeParameters::FellowshipSalary(dynamic_params::fellowship_salary::Parameters::Asset(
			dynamic_params::fellowship_salary::Asset,
			None,
		))
	}
}

/// Origin allowed to change dynamic runtime parameters.
///
/// Each [`RuntimeParametersKey`] variant defines its own access rules; see the
/// per-variant matches in [`Self::try_origin`].
pub struct DynamicParameterOrigin;
impl EnsureOriginWithArg<RuntimeOrigin, RuntimeParametersKey> for DynamicParameterOrigin {
	type Success = ();

	fn try_origin(
		origin: RuntimeOrigin,
		key: &RuntimeParametersKey,
	) -> Result<Self::Success, RuntimeOrigin> {
		match key {
			// Fellowship salary parameters can be set by Root, the FellowshipAdmin
			// origin (i.e. token holder referendum), or by a vote among all Fellows.
			RuntimeParametersKey::FellowshipSalary(_) |
			RuntimeParametersKey::SecretarySalary(_) => EitherOfDiverse::<
				EnsureRoot<AccountId>,
				EitherOfDiverse<
					EnsureXcm<IsVoiceOfBody<AssetHubLocation, FellowshipAdminBodyId>>,
					Fellows,
				>,
			>::ensure_origin(origin.clone())
			.map(|_| ())
			.map_err(|_| origin),
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin(_key: &RuntimeParametersKey) -> Result<RuntimeOrigin, ()> {
		Ok(RuntimeOrigin::root())
	}
}

/// Fellowship Salary Configuration.
#[derive(
	Encode,
	Decode,
	scale_info::TypeInfo,
	DecodeWithMemTracking,
	MaxEncodedLen,
	Clone,
	PartialEq,
	Eq,
	Debug,
)]
pub struct FellowshipSalaryConfig {
	/// Fellowship Salary Asset.
	pub asset: Box<VersionedLocatableAsset>,
	/// Fellowship salary budget for a single period (i.e., `RegistrationPeriod` +
	/// `PayoutPeriod`), expressed as the raw value of the `asset` (e.g., USDT on Asset Hub with 6
	/// decimals).
	pub budget: u128,
}

/// Secretary Salary Configuration.
#[derive(
	Encode,
	Decode,
	scale_info::TypeInfo,
	DecodeWithMemTracking,
	MaxEncodedLen,
	Clone,
	PartialEq,
	Eq,
	Debug,
)]
pub struct SecretarySalaryConfig {
	/// Secretary Salary Asset.
	pub asset: Box<VersionedLocatableAsset>,
	/// Secretary salary budget for a single period (i.e., `RegistrationPeriod` +
	/// `PayoutPeriod`), expressed as the raw value of the `asset` (e.g., USDT on Asset Hub with 6
	/// decimals).
	pub budget: u128,
	/// Secretary salary for rank 1 in `asset`, expressed in the raw
	/// value of the asset (e.g., USDT on Asset Hub with 6 decimals).
	pub salary_rank1: u128,
}
