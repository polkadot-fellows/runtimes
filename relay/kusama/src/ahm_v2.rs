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

//! AHM v2 migration wiring: the relay-chain side of moving account, proxy, registrar and HRMP
//! state to the Coretime chain.
//!
//! Compiled only with the `ahm-v2` feature, which released runtimes do not enable. The
//! integration tests turn it on to drive the real runtime.

pub mod lockdown;

use crate::{
	xcm_config::{Broker, XcmRouter},
	AccountId, BrokerId, BrokerPalletId, CrowdloanId, DelegatedStakingPalletId, OnDemandPalletId,
	PoolsPalletId, ProxyType, Runtime, RuntimeEvent, SocietyPalletId, Timestamp,
	TransparentProxyType, TreasuryPalletId,
};
use frame_support::{
	parameter_types,
	traits::{Contains, Equals},
};
use frame_system::EnsureRoot;
use kusama_runtime_constants::{currency::UNITS, system_parachain::ASSET_HUB_ID};
use migrator_types::PortableProxyType;
use pallet_xcm::EnsureXcm;
use sp_runtime::traits::AccountIdConversion;

parameter_types! {
	pub const AhParaId: u32 = ASSET_HUB_ID;
	pub const CtFreeBuffer: u128 = UNITS;
}

/// The pots of this chain, left for Sweep.
pub struct SweptAccounts;
impl Contains<AccountId> for SweptAccounts {
	fn contains(who: &AccountId) -> bool {
		[
			TreasuryPalletId::get(),
			SocietyPalletId::get(),
			BrokerPalletId::get(),
			OnDemandPalletId::get(),
			CrowdloanId::get(),
			PoolsPalletId::get(),
			DelegatedStakingPalletId::get(),
		]
		.into_iter()
		.any(|id| AccountIdConversion::<AccountId>::into_account_truncating(&id) == *who)
	}
}

impl pallet_rc2_migrator::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type SendXcm = XcmRouter;
	type CtParaId = BrokerId;
	type AhParaId = AhParaId;
	type CtFreeBuffer = CtFreeBuffer;
	type TimeProvider = Timestamp;
	type CtOrigin = EnsureXcm<Equals<Broker>>;
	type AdminOrigin = EnsureRoot<AccountId>;
	type PreMigrationCalls = frame_support::traits::Everything;
	type IntraMigrationCalls = lockdown::CallsEnabledDuringMigration;
	type PostMigrationCalls = lockdown::CallsEnabledAfterMigration;
	type SweptAccounts = SweptAccounts;
}

/// Which proxy permissions travel to the Coretime chain. The others have no meaning there, so
/// their definitions stay here.
impl TryFrom<TransparentProxyType> for PortableProxyType {
	type Error = ();

	fn try_from(t: TransparentProxyType) -> Result<Self, ()> {
		match t.0 {
			ProxyType::Any => Ok(PortableProxyType::Any),
			ProxyType::NonTransfer => Ok(PortableProxyType::NonTransfer),
			ProxyType::CancelProxy => Ok(PortableProxyType::CancelProxy),
			ProxyType::ParaRegistration => Ok(PortableProxyType::ParaRegistration),
			ProxyType::Governance |
			ProxyType::Staking |
			ProxyType::Auction |
			ProxyType::Society |
			ProxyType::Spokesperson |
			ProxyType::NominationPools => Err(()),
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::{ProxyType, Runtime, RuntimeCall, TransparentProxyType};
	use codec::Encode;
	use migrator_types::PortableProxyType;
	use pallet_ct_migrator::{Rc2MigratorCall, Rc2RuntimeCall};

	/// Which proxy permissions travel to the Coretime chain, one assertion per variant so a new
	/// variant has to be classified here.
	#[test]
	fn proxy_permissions_that_travel() {
		for (proxy_type, portable) in [
			(ProxyType::Any, Some(PortableProxyType::Any)),
			(ProxyType::NonTransfer, Some(PortableProxyType::NonTransfer)),
			(ProxyType::CancelProxy, Some(PortableProxyType::CancelProxy)),
			(ProxyType::ParaRegistration, Some(PortableProxyType::ParaRegistration)),
			(ProxyType::Governance, None),
			(ProxyType::Staking, None),
			(ProxyType::Auction, None),
			(ProxyType::Society, None),
			(ProxyType::Spokesperson, None),
			(ProxyType::NominationPools, None),
		] {
			assert_eq!(
				PortableProxyType::try_from(TransparentProxyType(proxy_type)).ok(),
				portable,
				"{proxy_type:?}"
			);
		}
	}

	/// Ensure the pallet + call index aligns.
	#[test]
	fn the_coretime_chain_encodes_this_chains_calls_correctly() {
		assert_eq!(
			Rc2RuntimeCall::Rc2Migrator(Rc2MigratorCall::CtReady).encode(),
			RuntimeCall::Rc2Migrator(pallet_rc2_migrator::Call::<Runtime>::ct_ready {}).encode(),
		);
	}
}
