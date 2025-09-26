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

pub mod basic_still_works;
pub mod whale_watching;

pub use basic_still_works::ProxyBasicWorks;
pub use whale_watching::ProxyWhaleWatching;

use crate::porting_prelude::*;

use frame_support::pallet_prelude::*;
use sp_runtime::traits::TryConvert;

/// Intent based permission.
///
/// Should be a superset of all possible proxy types.
#[derive(Clone, PartialEq, Eq, RuntimeDebug)]
pub enum Permission {
	Any,
	NonTransfer,
	Governance,
	Staking,
	CancelProxy,
	Auction,
	NominationPools,
	ParaRegistration,
	Assets,
	AssetOwner,
	AssetManager,
	Collator,
	Old,
	#[cfg(feature = "kusama-ahm")]
	Society,
	#[cfg(feature = "kusama-ahm")]
	Spokesperson,
}

// Relay -> Permission
impl TryConvert<rc_proxy_definition::ProxyType, Permission> for Permission {
	fn try_convert(
		proxy: rc_proxy_definition::ProxyType,
	) -> Result<Self, rc_proxy_definition::ProxyType> {
		use rc_proxy_definition::ProxyType;

		Ok(match proxy {
			ProxyType::Any => Permission::Any,
			ProxyType::Auction => Permission::Auction,
			ProxyType::CancelProxy => Permission::CancelProxy,
			ProxyType::Governance => Permission::Governance,
			ProxyType::NominationPools => Permission::NominationPools,
			ProxyType::NonTransfer => Permission::NonTransfer,
			ProxyType::ParaRegistration => Permission::ParaRegistration,
			ProxyType::Staking => Permission::Staking,
			#[cfg(feature = "kusama-ahm")]
			ProxyType::Society => Permission::Society,
			#[cfg(feature = "kusama-ahm")]
			ProxyType::Spokesperson => Permission::Spokesperson,
		})
	}
}

// AH -> Permission
impl TryConvert<asset_hub_polkadot_runtime::ProxyType, Permission> for Permission {
	fn try_convert(
		proxy: asset_hub_polkadot_runtime::ProxyType,
	) -> Result<Self, asset_hub_polkadot_runtime::ProxyType> {
		use asset_hub_polkadot_runtime::ProxyType;

		Ok(match proxy {
			ProxyType::Any => Permission::Any,
			ProxyType::AssetManager => Permission::AssetManager,
			ProxyType::AssetOwner => Permission::AssetOwner,
			ProxyType::Assets => Permission::Assets,
			ProxyType::CancelProxy => Permission::CancelProxy,
			ProxyType::Collator => Permission::Collator,
			ProxyType::Governance => Permission::Governance,
			ProxyType::NominationPools => Permission::NominationPools,
			ProxyType::NonTransfer => Permission::NonTransfer,
			ProxyType::Auction => Permission::Old,
			ProxyType::ParaRegistration => Permission::Old,
			ProxyType::Staking => Permission::Staking,
			#[cfg(feature = "kusama-ahm")]
			ProxyType::Society => Permission::Society,
			#[cfg(feature = "kusama-ahm")]
			ProxyType::Spokesperson => Permission::Spokesperson,
		})
	}
}

// Permission -> Maybe(AH)
impl TryConvert<Permission, asset_hub_polkadot_runtime::ProxyType> for Permission {
	fn try_convert(
		permission: Permission,
	) -> Result<asset_hub_polkadot_runtime::ProxyType, Permission> {
		use asset_hub_polkadot_runtime::ProxyType;

		Ok(match permission {
			Permission::Any => ProxyType::Any,
			Permission::NonTransfer => ProxyType::NonTransfer,
			Permission::Governance => ProxyType::Governance,
			Permission::Staking => ProxyType::Staking,
			Permission::CancelProxy => ProxyType::CancelProxy,
			Permission::Auction => ProxyType::Auction,
			Permission::NominationPools => ProxyType::NominationPools,
			Permission::ParaRegistration => ProxyType::ParaRegistration,
			Permission::Assets => ProxyType::Assets,
			Permission::AssetOwner => ProxyType::AssetOwner,
			Permission::AssetManager => ProxyType::AssetManager,
			Permission::Collator => ProxyType::Collator,
			Permission::Old => return Err(permission),
			#[cfg(feature = "kusama-ahm")]
			Permission::Society => ProxyType::Society,
			#[cfg(feature = "kusama-ahm")]
			Permission::Spokesperson => ProxyType::Spokesperson,
		})
	}
}
