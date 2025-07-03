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

use frame_support::{
	pallet_prelude::*,
	traits::{Currency, Defensive},
};
use frame_system::pallet_prelude::*;
use hex_literal::hex;
use pallet_ah_migrator::types::AhMigrationCheck;
use pallet_rc_migrator::types::{RcMigrationCheck, ToPolkadotSs58};
use sp_runtime::{
	traits::{Dispatchable, TryConvert},
	AccountId32,
};
use std::{collections::BTreeMap, str::FromStr};

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
			ProxyType::OldAuction => Permission::Old,
			ProxyType::OldParaRegistration => Permission::Old,
			ProxyType::Staking => Permission::Staking,
		})
	}
}
