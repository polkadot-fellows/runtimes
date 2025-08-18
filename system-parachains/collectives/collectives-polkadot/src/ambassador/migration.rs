// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Cumulus.

// Cumulus is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Cumulus is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Cumulus.  If not, see <http://www.gnu.org/licenses/>.

//! Migrations.

use frame_support::{pallet_prelude::*, traits::OnRuntimeUpgrade, weights::Weight};

pub(crate) mod add_accounts {
	use super::*;
	#[cfg(feature = "try-runtime")]
	use crate::ambassador::AmbassadorCollectiveInstance;
	use frame_support::parameter_types;
	use pallet_ranked_collective::{
		Config, IdToIndex, IndexToId, MemberCount, MemberRecord, Members, Rank,
	};
	#[cfg(feature = "try-runtime")]
	use sp_std::vec::Vec;

	parameter_types! {
		// Public key (hex)
		pub const Addresses: [(Rank, [u8; 32]); 13] = [
			(0, hex_literal::hex!("54361bceb4403e1af7c893688a76c35357477da7e36371b981728ddf8f978e0c")),
			(0, hex_literal::hex!("c0c799b66754bfb56799dfef8071772d8c5ea2a87dd0c969493066aed94e645c")),
			(0, hex_literal::hex!("30c9d60350b04b6bce9b4c692b2db6ab91a16cd990952716de59e4dfbc79406f")),
			(0, hex_literal::hex!("ea0ab1b08b58a3708b50ba9928c4e25ad71d68efcbb868a2f75b987d0e8e4108")),
			(0, hex_literal::hex!("08e1973238f78c6046f097a1aebc62d8590156b0ae414c3148cb8c8a5fee931a")),
			(0, hex_literal::hex!("0ed039e96de24a2f5e9954c9bddab6ea98712e16dc41140e715e9a98d9eca64e")),
			(0, hex_literal::hex!("46cd0f31add423547c14efd7593f07807c1783153317308d52c6fc9995ae2067")),
			(0, hex_literal::hex!("76126bccbd03939a60016a0719775d47876f7eb25713d165e592a81a7ce2957c")),
			(0, hex_literal::hex!("16b8eaf319bfa79d8d5494f3e3f20d6a5bb37766433d500eb1eac2fcb816276c")),
			(0, hex_literal::hex!("28e41f254f174a58c5499459af0f1c8834ebbcbc2402ee963707950c69480a77")),
			(0, hex_literal::hex!("6c7499cc79bee0f02862e75504c7c5924c9ea55e977fd5c23407919a7addb258")),
			(0, hex_literal::hex!("b4fbf400039d8159aa0ebbe79890cc0688187e353d1be52ea64e7772d5b73077")),
			(0, hex_literal::hex!("ae71605d54343a5b19964e876da7aaddaa8a6c9e17244d7839f344eefcce2c6c")),
		];
	}

	/// Implements `OnRuntimeUpgrade` trait.
	#[allow(dead_code)]
	pub struct Migration<T, I = ()>(PhantomData<(T, I)>);

	impl<T: Config<I>, I: 'static> OnRuntimeUpgrade for Migration<T, I>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]>,
	{
		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
			let has_members =
				pallet_ranked_collective::Members::<T, AmbassadorCollectiveInstance>::iter()
					.next()
					.is_some();
			ensure!(!has_members, "the collective must be uninitialized.");
			Ok(Vec::new())
		}

		fn on_runtime_upgrade() -> Weight {
			let mut weight = T::DbWeight::get().reads(1);

			for (desired_rank, account_id32) in Addresses::get() {
				let who: T::AccountId = account_id32.into();

				// Set collective pallet storage
				let record = MemberRecord::new(desired_rank);
				<Members<T, I>>::insert(&who, record);
				MemberCount::<T, I>::mutate(desired_rank, |count| *count = count.saturating_add(1));
				let count_at_rank = MemberCount::<T, I>::get(desired_rank);
				IdToIndex::<T, I>::insert(
					desired_rank,
					who.clone(),
					count_at_rank.saturating_sub(1),
				);
				IndexToId::<T, I>::insert(desired_rank, count_at_rank.saturating_sub(1), who);

				weight = weight
					.saturating_add(T::DbWeight::get().writes(2))
					.saturating_add(T::DbWeight::get().reads_writes(2, 2));
			}

			weight
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
			ensure!(MemberCount::<T, I>::get(0) == 13, "invalid members count at rank 0.");
			Ok(())
		}
	}
}

#[cfg(test)]
pub mod tests {
	use super::add_accounts::Addresses;
	use crate::{ambassador::AmbassadorCollectiveInstance as Ambassador, Runtime, System};
	use frame_support::traits::OnRuntimeUpgrade;
	use pallet_ranked_collective::Rank;
	use parachains_common::AccountId;
	use sp_core::crypto::Ss58Codec;
	use sp_runtime::{AccountId32, BuildStorage};

	#[test]
	fn check_addresses() {
		let addresses = Addresses::get();
		let ambassador_ss58: [(Rank, _); 13] = [
			(0, "12uR6ZinxBstfeh9zX5d415Z29XkSfNR4Nfkn6afAusKc52n"),
			(0, "15MmVHDWD5gkJzJfGVRGeHewNmRFqbuMXUSZN4spR22Lu9cM"),
			(0, "126yFs7wRkEsknx7NKWrw4UHhUcgcxR8XsiSdPfcpaoLXfe3"),
			(0, "16HsP9V3D2WQsKdaeyBBLmvSRXLsH5So62vnXoJumqGj5j8U"),
			(0, "1CeQ49R2mCScmBGuxoeS9FDq9Ssnb5xTttVWE6zFgRpbL2d"),
			(0, "1LRXY86unTYZgExx1cQnHxscaJb7VC8YFaPAWby5mbdCsgp"),
			(0, "12bqGW8fWoxawNkZWat5VW8UT3EP1rrsFsP8p3V2g8BYf2ZE"),
			(0, "13fp87SGGtXfCEdwqu9v7tVcb34gmhEGM1zDHvDAFzC3Yjk7"),
			(0, "1WnzAGy4czRWseZxJmHG6sgfEyaBrGtFsMgapFooryqtNFj"),
			(0, "1vcgYAMi3jNBugvufvo5wZWioArkSp1vDos8PLrMYu1Gwp9"),
			(0, "13TCowCJVpcD1iezJTFMaBPBm6xMyGyhYTYqoiavsfky4jox"),
			(0, "156JTy81GoyNtiAZYyXoivhoWzzz7NcMmrqJeQBs4Qhn1A61"),
			(0, "14wj1gbmKVLs61qczztaADNAYHtQ1TJyDneJFXZ6GSXDkTDo"),
		];

		for (index, val) in ambassador_ss58.iter().enumerate() {
			let account: AccountId32 = <AccountId as Ss58Codec>::from_string(val.1).unwrap();
			let account32: [u8; 32] = account.clone().into();
			assert_eq!(addresses[index].0, ambassador_ss58[index].0, "ranks must be equal.");
			assert_eq!(addresses[index].1, account32, "accounts must be equal.");
		}
	}

	#[test]
	fn test_add_accounts() {
		use super::add_accounts::Migration;
		use pallet_ranked_collective::{IdToIndex, IndexToId, MemberCount, MemberRecord, Members};

		let t = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();
		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext.execute_with(|| {
			assert_eq!(MemberCount::<Runtime, Ambassador>::get(0), 0);
			Migration::<Runtime, Ambassador>::on_runtime_upgrade();
			assert_eq!(MemberCount::<Runtime, Ambassador>::get(0), 13);
			for (rank, account_id32) in Addresses::get() {
				let who = <Runtime as frame_system::Config>::AccountId::from(account_id32);
				assert!(IdToIndex::<Runtime, Ambassador>::get(0, &who).is_some());
				assert!(IdToIndex::<Runtime, Ambassador>::get(rank + 1, &who).is_none());
				let index = IdToIndex::<Runtime, Ambassador>::get(rank, &who).unwrap();
				assert_eq!(IndexToId::<Runtime, Ambassador>::get(rank, index).unwrap(), who);
				assert_eq!(
					Members::<Runtime, Ambassador>::get(&who).unwrap(),
					MemberRecord::new(rank)
				);
			}
		});
	}
}
