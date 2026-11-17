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
	xcm_config::{CoretimeLocation, XcmRouter},
	AccountId, BrokerId, BrokerPalletId, CrowdloanId, DelegatedStakingPalletId, OnDemandPalletId,
	PoolsPalletId, ProxyType, Runtime, RuntimeEvent, Timestamp, TransparentProxyType,
	TreasuryPalletId, XcmPallet,
};
use frame_support::{
	pallet_prelude::{Identity, ValueQuery},
	parameter_types,
	traits::{Contains, Equals},
};
use frame_system::EnsureRoot;
use migrator_types::PortableProxyType;
use pallet_xcm::EnsureXcm;
use polkadot_runtime_constants::{currency::UNITS, system_parachain::ASSET_HUB_ID};
use sp_core::H256;
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

/// `pallet-xcm`'s asset traps, which it keeps private, for the migrator to take. Named as in
/// `pallet-xcm`, since an alias's name is part of its storage key.
#[frame_support::storage_alias(pallet_name)]
pub type AssetTraps = StorageMap<XcmPallet, Identity, H256, u32, ValueQuery>;

impl pallet_rc2_migrator::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type SendXcm = XcmRouter;
	type CtParaId = BrokerId;
	type AhParaId = AhParaId;
	type CtFreeBuffer = CtFreeBuffer;
	type TimeProvider = Timestamp;
	type CtOrigin = EnsureXcm<Equals<CoretimeLocation>>;
	type AdminOrigin = EnsureRoot<AccountId>;
	type PreMigrationCalls = frame_support::traits::Everything;
	type IntraMigrationCalls = lockdown::CallsEnabledDuringMigration;
	type PostMigrationCalls = lockdown::CallsEnabledAfterMigration;
	type SweptAccounts = SweptAccounts;
	type XcmAssetTraps = AssetTraps;
}

/// Which proxy permissions travel to the Coretime chain. The others have no meaning there, so
/// their definitions stay here.
impl TryFrom<TransparentProxyType<ProxyType>> for PortableProxyType {
	type Error = ();

	fn try_from(t: TransparentProxyType<ProxyType>) -> Result<Self, ()> {
		match t.0 {
			ProxyType::Any => Ok(PortableProxyType::Any),
			ProxyType::NonTransfer => Ok(PortableProxyType::NonTransfer),
			ProxyType::CancelProxy => Ok(PortableProxyType::CancelProxy),
			ProxyType::ParaRegistration => Ok(PortableProxyType::ParaRegistration),
			ProxyType::Governance |
			ProxyType::Staking |
			ProxyType::Auction |
			ProxyType::NominationPools => Err(()),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::{AssetTraps, UNITS};
	use crate::{
		xcm_config::XcmConfig, AccountId, Balances, ProxyType, Rc2Migrator, Runtime, RuntimeCall,
		RuntimeEvent, RuntimeOrigin, TransparentProxyType,
	};
	use codec::Encode;
	use frame_support::{assert_ok, traits::fungible::Mutate};
	use migrator_types::PortableProxyType;
	use pallet_ct_migrator::{Rc2MigratorCall, Rc2RuntimeCall};
	use pallet_rc2_migrator::{AssetTrap, MigrationStage, RcMigrationStage, TakenAssetTraps};
	use xcm::latest::prelude::*;
	use xcm_executor::XcmExecutor;

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

	/// The migrator takes a trap `pallet-xcm` made, named as its `AssetsTrapped` event reports
	/// it, out of `pallet-xcm`'s own storage.
	#[test]
	fn the_migrator_takes_a_trap_pallet_xcm_made() {
		sp_io::TestExternalities::default().execute_with(|| {
			// Block 0 does not record events.
			frame_system::Pallet::<Runtime>::set_block_number(1);

			// GIVEN Alice's assets trapped by a message of hers that fails after withdrawing them.
			let alice = AccountId::new([1; 32]);
			assert_ok!(<Balances as Mutate<AccountId>>::mint_into(&alice, 100 * UNITS));
			let outcome = XcmExecutor::<XcmConfig>::prepare_and_execute(
				AccountId32 { network: None, id: alice.into() },
				Xcm::<RuntimeCall>(vec![WithdrawAsset((Here, 10 * UNITS).into()), Trap(0)]),
				&mut [0u8; 32],
				Weight::MAX,
				Weight::MAX,
			);
			assert_eq!(outcome.ensure_complete().map_err(|e| e.error), Err(XcmError::Trap(0)));
			let (hash, trap) = frame_system::Pallet::<Runtime>::events()
				.into_iter()
				.find_map(|record| match record.event {
					RuntimeEvent::XcmPallet(pallet_xcm::Event::AssetsTrapped {
						hash,
						origin,
						assets,
					}) => Some((hash, AssetTrap { claimer: origin.into(), assets })),
					_ => None,
				})
				.expect("the assets were trapped");
			assert_eq!(AssetTraps::get(hash), 1);

			// WHEN the manager submits it as reported.
			RcMigrationStage::<Runtime>::put(MigrationStage::WaitingForAssetTraps);
			assert_ok!(Rc2Migrator::submit_asset_traps(
				RuntimeOrigin::root(),
				vec![trap.clone()],
				false
			));

			// THEN `pallet-xcm` no longer holds it, and the migrator does, under the same key.
			assert!(!AssetTraps::contains_key(hash));
			assert_eq!(TakenAssetTraps::<Runtime>::get(&trap), 1);
		});
	}
}
