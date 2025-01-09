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

extern crate alloc;

#[cfg(test)]
mod tests;

use alloc::vec::Vec;
use codec::{Encode, MaxEncodedLen};
use frame_support::{storage::storage_prefix, Parameter, StorageHasher, Twox64Concat};
use pallet_proxy::ProxyDefinition;
use scale_info::TypeInfo;
use sp_core::Hasher;
use sp_runtime::traits::Saturating;

pub use cumulus_primitives_core::PersistedValidationData;
pub use pallet::*;

/// The remote proxy interface.
pub trait RemoteProxyInterface<AccountId, ProxyType, BlockNumber> {
	/// The remote account id.
	type RemoteAccountId: Parameter + MaxEncodedLen;
	/// The remote proxy type.
	type RemoteProxyType: Parameter + MaxEncodedLen;
	/// The remote block number.
	type RemoteBlockNumber: Parameter + Saturating + MaxEncodedLen + Default;
	/// The hash type used by the remote chain.
	type Hash: Parameter + MaxEncodedLen;
	/// The hasher used by the remote chain.
	type Hasher: Hasher<Out = Self::Hash>;

	/// Get the latest block to storage root mapping.
	fn block_to_storage_root(
		validation_data: &PersistedValidationData,
	) -> Option<(Self::RemoteBlockNumber, <Self::Hasher as Hasher>::Out)>;

	/// The storage key where to find the [`ProxyDefinition`] for the given proxy account in the
	/// remote chain.
	fn proxy_definition_storage_key(proxy: &Self::RemoteAccountId) -> Vec<u8> {
		let mut key = storage_prefix("Proxy".as_bytes(), "Proxies".as_bytes()).to_vec();
		proxy.using_encoded(|p| {
			key.extend(Twox64Concat::hash(p));
		});
		key
	}

	/// Convert the local account id to the remote account id.
	///
	/// If the conversion is not possible, return `None`.
	fn local_to_remote_account_id(local: &AccountId) -> Option<Self::RemoteAccountId>;

	/// Convert the remote proxy definition to the local proxy definition.
	///
	/// If the conversion is not possible, return `None`.
	fn remote_to_local_proxy_defintion(
		remote: ProxyDefinition<
			Self::RemoteAccountId,
			Self::RemoteProxyType,
			Self::RemoteBlockNumber,
		>,
	) -> Option<ProxyDefinition<AccountId, ProxyType, BlockNumber>>;
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use cumulus_pallet_parachain_system::OnSystemEvent;
	use cumulus_primitives_core::PersistedValidationData;
	use frame_support::{dispatch_context, pallet_prelude::*, traits::IsSubType};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::{Dispatchable, StaticLookup, Zero};

	type AccountIdLookupOf<T> = <<T as frame_system::Config>::Lookup as StaticLookup>::Source;

	type RemoteBlockNumberOf<T, I> = <<T as Config<I>>::RemoteProxy as RemoteProxyInterface<
		<T as frame_system::Config>::AccountId,
		<T as pallet_proxy::Config>::ProxyType,
		BlockNumberFor<T>,
	>>::RemoteBlockNumber;
	type RemoteAccountIdOf<T, I> = <<T as Config<I>>::RemoteProxy as RemoteProxyInterface<
		<T as frame_system::Config>::AccountId,
		<T as pallet_proxy::Config>::ProxyType,
		BlockNumberFor<T>,
	>>::RemoteAccountId;
	type RemoteHasherOf<T, I> = <<T as Config<I>>::RemoteProxy as RemoteProxyInterface<
		<T as frame_system::Config>::AccountId,
		<T as pallet_proxy::Config>::ProxyType,
		BlockNumberFor<T>,
	>>::Hasher;
	type RemoteHashOf<T, I> = <<T as Config<I>>::RemoteProxy as RemoteProxyInterface<
		<T as frame_system::Config>::AccountId,
		<T as pallet_proxy::Config>::ProxyType,
		BlockNumberFor<T>,
	>>::Hash;
	type RemoteProxyTypeOf<T, I> = <<T as Config<I>>::RemoteProxy as RemoteProxyInterface<
		<T as frame_system::Config>::AccountId,
		<T as pallet_proxy::Config>::ProxyType,
		BlockNumberFor<T>,
	>>::RemoteProxyType;

	#[pallet::pallet]
	pub struct Pallet<T, I = ()>(_);

	/// Stores the last [`Config::MaxStorageRootsToKeep`] block to storage root mappings of the
	/// target chain.
	#[pallet::storage]
	pub type BlockToRoot<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Twox64Concat, RemoteBlockNumberOf<T, I>, RemoteHashOf<T, I>, OptionQuery>;

	/// Configuration trait.
	#[pallet::config]
	pub trait Config<I: 'static = ()>: frame_system::Config + pallet_proxy::Config {
		/// Maximum number of storage roots to keep in storage.
		///
		/// The storage roots are used to validate the remote proofs. The more we keep in storage,
		/// the older the proof can be.
		type MaxStorageRootsToKeep: Get<RemoteBlockNumberOf<Self, I>>;

		/// The interface for interacting with the remote proxy.
		type RemoteProxy: RemoteProxyInterface<
			Self::AccountId,
			Self::ProxyType,
			BlockNumberFor<Self>,
		>;
	}

	impl<T: Config, I: 'static> OnSystemEvent for Pallet<T, I> {
		fn on_validation_data(validation_data: &PersistedValidationData) {
			let Some((block, hash)) = T::RemoteProxy::block_to_storage_root(&validation_data)
			else {
				return;
			};

			// Update the block to root mappings.
			BlockToRoot::<T>::insert(block.clone(), hash);
			BlockToRoot::<T>::remove(block.saturating_sub(T::MaxStorageRootsToKeep::get()));
		}

		fn on_validation_code_applied() {}
	}

	#[pallet::error]
	#[derive(PartialEq)]
	pub enum Error<T, I = ()> {
		/// The local account id could not converted to the remote account id.
		CouldNotConvertLocalToRemoteAccountId,
		/// The anchor block of the remote proof is unknown.
		UnknownProofAnchorBlock,
		/// The proxy definition could not be found in the proof.
		InvalidProof,
		/// Failed to decode the remote proxy definition from the proof.
		ProxyDefinitionDecodingFailed,
		/// Announcement, if made at all, was made too recently.
		Unannounced,
		/// Could not find any matching proxy definition in the proof.
		DidNotFindMatchingProxyDefinition,
		/// Proxy proof not registered.
		ProxyProofNotRegistered,
	}

	/// The remote proxy proof to prove the existence of a proxy account.
	#[derive(core::fmt::Debug, Clone, Decode, Encode, TypeInfo, PartialEq, Eq)]
	pub enum RemoteProxyProof<RemoteBlockNumber> {
		/// Assumes the default proxy storage layout.
		V1 { proof: Vec<Vec<u8>>, block: RemoteBlockNumber },
	}

	#[derive(Default)]
	struct RemoteProxyContext<RemoteBlockNumber> {
		proofs: Vec<RemoteProxyProof<RemoteBlockNumber>>,
	}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::call_index(0)]
		#[pallet::weight({
			let di = call.get_dispatch_info();
			( // AccountData for inner call origin accountdata.
				T::DbWeight::get().reads_writes(1, 1)
				.saturating_add(di.weight),
			di.class)
		})]
		pub fn remote_proxy(
			origin: OriginFor<T>,
			real: AccountIdLookupOf<T>,
			force_proxy_type: Option<T::ProxyType>,
			call: Box<<T as pallet_proxy::Config>::RuntimeCall>,
			proof: RemoteProxyProof<RemoteBlockNumberOf<T, I>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let real = T::Lookup::lookup(real)?;

			Self::do_remote_proxy(who, real, force_proxy_type, call, proof)
		}

		#[pallet::call_index(1)]
		#[pallet::weight(0)]
		pub fn register_remote_proxy_proof(
			origin: OriginFor<T>,
			proof: RemoteProxyProof<RemoteBlockNumberOf<T, I>>,
		) -> DispatchResult {
			ensure_signed(origin)?;

			dispatch_context::with_context::<RemoteProxyContext<RemoteBlockNumberOf<T, I>>, _>(
				|context| {
					context.or_default().proofs.push(proof);
				},
			);

			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(0)]
		pub fn remote_proxy_with_registered_proof(
			origin: OriginFor<T>,
			real: AccountIdLookupOf<T>,
			force_proxy_type: Option<T::ProxyType>,
			call: Box<<T as pallet_proxy::Config>::RuntimeCall>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let real = T::Lookup::lookup(real)?;

			let proof = dispatch_context::with_context::<
				RemoteProxyContext<RemoteBlockNumberOf<T, I>>,
				_,
			>(|context| context.or_default().proofs.pop())
			.flatten()
			.ok_or_else(|| Error::<T, I>::ProxyProofNotRegistered)?;

			Self::do_remote_proxy(who, real, force_proxy_type, call, proof)
		}
	}

	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		fn do_remote_proxy(
			who: T::AccountId,
			real: T::AccountId,
			force_proxy_type: Option<T::ProxyType>,
			call: Box<<T as pallet_proxy::Config>::RuntimeCall>,
			proof: RemoteProxyProof<RemoteBlockNumberOf<T, I>>,
		) -> DispatchResult {
			let Some(real_remote) = T::RemoteProxy::local_to_remote_account_id(&real) else {
				return Err(Error::<T, I>::CouldNotConvertLocalToRemoteAccountId.into());
			};

			let def = match proof {
				RemoteProxyProof::V1 { proof, block } => {
					let Some(storage_root) = BlockToRoot::<T, I>::get(block) else {
						return Err(Error::<T, I>::UnknownProofAnchorBlock.into());
					};

					let key = T::RemoteProxy::proxy_definition_storage_key(&real_remote);

					let db =
						sp_trie::StorageProof::new(proof).into_memory_db::<RemoteHasherOf<T, I>>();
					let value = sp_trie::read_trie_value::<sp_trie::LayoutV1<_>, _>(
						&db,
						&storage_root,
						&key,
						None,
						None,
					)
					.ok()
					.flatten()
					.ok_or_else(|| Error::<T, I>::InvalidProof)?;

					let proxy_definitions = alloc::vec::Vec::<
						ProxyDefinition<
							RemoteAccountIdOf<T, I>,
							RemoteProxyTypeOf<T, I>,
							RemoteBlockNumberOf<T, I>,
						>,
					>::decode(&mut &value[..])
					.map_err(|_| Error::<T, I>::ProxyDefinitionDecodingFailed)?;

					let f = |x: &ProxyDefinition<
						T::AccountId,
						T::ProxyType,
						BlockNumberFor<T>,
					>|
					 -> bool {
						x.delegate == who &&
							force_proxy_type.as_ref().map_or(true, |y| &x.proxy_type == y)
					};

					proxy_definitions
						.into_iter()
						.filter_map(|pd| T::RemoteProxy::remote_to_local_proxy_defintion(pd))
						.find(f)
						.ok_or_else(|| Error::<T, I>::DidNotFindMatchingProxyDefinition)?
				},
			};

			ensure!(def.delay.is_zero(), Error::<T, I>::Unannounced);

			Self::do_proxy(def, real, *call);

			Ok(())
		}

		/// TODO: Make upstream public and use that one.
		fn do_proxy(
			def: ProxyDefinition<T::AccountId, T::ProxyType, BlockNumberFor<T>>,
			real: T::AccountId,
			call: <T as pallet_proxy::Config>::RuntimeCall,
		) {
			use frame_support::traits::{InstanceFilter as _, OriginTrait as _};
			// This is a freshly authenticated new account, the origin restrictions doesn't apply.
			let mut origin: T::RuntimeOrigin = frame_system::RawOrigin::Signed(real).into();
			origin.add_filter(move |c: &<T as frame_system::Config>::RuntimeCall| {
				let c = <T as pallet_proxy::Config>::RuntimeCall::from_ref(c);
				// We make sure the proxy call does access this pallet to change modify proxies.
				match c.is_sub_type() {
					// Proxy call cannot add or remove a proxy with more permissions than it already
					// has.
					Some(pallet_proxy::Call::add_proxy { ref proxy_type, .. }) |
					Some(pallet_proxy::Call::remove_proxy { ref proxy_type, .. })
						if !def.proxy_type.is_superset(proxy_type) =>
						false,
					// Proxy call cannot remove all proxies or kill pure proxies unless it has full
					// permissions.
					Some(pallet_proxy::Call::remove_proxies { .. }) |
					Some(pallet_proxy::Call::kill_pure { .. })
						if def.proxy_type != T::ProxyType::default() =>
						false,
					_ => def.proxy_type.filter(c),
				}
			});
			let e = call.dispatch(origin);
			frame_system::Pallet::<T>::deposit_event(
				<T as pallet_proxy::Config>::RuntimeEvent::from(
					pallet_proxy::Event::ProxyExecuted {
						result: e.map(|_| ()).map_err(|e| e.error),
					},
				),
			);
		}
	}
}
