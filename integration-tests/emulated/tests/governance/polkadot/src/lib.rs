// Copyright (C) Parity Technologies and the various Polkadot contributors, see Contributions.md
// for a list of specific contributors.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use emulated_integration_tests_common::{
	impls::{assert_expected_events, bx, Encode, TestExt},
	xcm_emulator::Chain,
};
use frame_support::{
	assert_ok,
	dispatch::{DispatchResultWithPostInfo, PostDispatchInfo},
};
use polkadot_system_emulated_network::CollectivesPolkadotPara as CollectivesPolkadot;
use sp_core::H256;
use sp_runtime::traits::{Dispatchable, Hash};
use xcm::{latest::prelude::*, VersionedLocation, VersionedXcm};

#[cfg(test)]
mod open_gov_on_asset_hub;
#[cfg(test)]
mod open_gov_on_relay;

/// Wraps a runtime call in a whitelist preimage call and dispatches it
pub fn dispatch_whitelisted_call_with_preimage<T>(
	call: T::RuntimeCall,
	origin: T::RuntimeOrigin,
) -> DispatchResultWithPostInfo
where
	T: Chain,
	T::Runtime: pallet_whitelist::Config,
	T::RuntimeCall: From<pallet_whitelist::Call<T::Runtime>>
		+ Into<<T::Runtime as pallet_whitelist::Config>::RuntimeCall>
		+ Dispatchable<RuntimeOrigin = T::RuntimeOrigin, PostInfo = PostDispatchInfo>,
{
	T::execute_with(|| {
		let whitelist_call: T::RuntimeCall =
			pallet_whitelist::Call::<T::Runtime>::dispatch_whitelisted_call_with_preimage {
				call: Box::new(call.into()),
			}
			.into();
		whitelist_call.dispatch(origin)
	})
}

/// Encodes a runtime call, stores it as a preimage, and returns its H256 hash
pub fn dispatch_note_preimage_call<T>(call: T::RuntimeCall) -> H256
where
	T: Chain,
	T::Runtime: frame_system::Config<Hash = H256> + pallet_preimage::Config,
	T::RuntimeCall: Encode
		+ From<pallet_preimage::Call<T::Runtime>>
		+ Dispatchable<RuntimeOrigin = T::RuntimeOrigin, PostInfo = PostDispatchInfo>,
	T::RuntimeOrigin: From<<T::Runtime as frame_system::Config>::RuntimeOrigin>,
{
	T::execute_with(|| {
		let call_bytes = call.encode();
		let call_hash = <T::Runtime as frame_system::Config>::Hashing::hash(&call_bytes);
		let preimage_call: T::RuntimeCall =
			pallet_preimage::Call::<T::Runtime>::note_preimage { bytes: call_bytes.clone() }.into();

		let root_origin = T::RuntimeOrigin::from(frame_system::RawOrigin::Root.into());
		assert_ok!(preimage_call.dispatch(root_origin));
		call_hash
	})
}

/// Builds an XCM call to send an authorize upgrade message using the provided location
pub fn build_xcm_send_authorize_upgrade_call<T, D>(location: Location) -> T::RuntimeCall
where
	T: Chain,
	T::Runtime: pallet_xcm::Config,
	T::RuntimeCall: Encode + From<pallet_xcm::Call<T::Runtime>>,
	D: Chain,
	D::Runtime: frame_system::Config<Hash = H256>,
	D::RuntimeCall: Encode + From<frame_system::Call<D::Runtime>>,
{
	let code_hash = [1u8; 32].into();
	// TODO: calculate real weight
	let weight = Weight::from_parts(5_000_000_000, 500_000);

	let transact_call: D::RuntimeCall = frame_system::Call::authorize_upgrade { code_hash }.into();

	let call: T::RuntimeCall = pallet_xcm::Call::send {
		dest: bx!(VersionedLocation::from(location)),
		message: bx!(VersionedXcm::from(Xcm(vec![
			UnpaidExecution { weight_limit: Unlimited, check_origin: None },
			Transact {
				origin_kind: OriginKind::Superuser,
				require_weight_at_most: weight,
				call: transact_call.encode().into(),
			}
		]))),
	}
	.into();
	call
}

/// CollectivesPolkadot dispatches `pallet_xcm::send` with `OriginKind:Xcm` to the dest with encoded
/// whitelist call.
pub fn collectives_send_whitelist(
	dest: Location,
	encoded_whitelist_call: impl FnOnce() -> Vec<u8>,
) {
	CollectivesPolkadot::execute_with(|| {
		type RuntimeEvent = <CollectivesPolkadot as Chain>::RuntimeEvent;
		type RuntimeCall = <CollectivesPolkadot as Chain>::RuntimeCall;
		type RuntimeOrigin = <CollectivesPolkadot as Chain>::RuntimeOrigin;
		type Runtime = <CollectivesPolkadot as Chain>::Runtime;

		let whitelist_call = RuntimeCall::PolkadotXcm(pallet_xcm::Call::<Runtime>::send {
			dest: bx!(VersionedLocation::from(dest)),
			message: bx!(VersionedXcm::from(Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::Xcm,
					require_weight_at_most: Weight::from_parts(5_000_000_000, 500_000),
					call: encoded_whitelist_call().into(),
				}
			]))),
		});

		use collectives_polkadot_runtime::fellowship::pallet_fellowship_origins::Origin::Fellows as FellowsOrigin;
		let fellows_origin: RuntimeOrigin = FellowsOrigin.into();
		assert_ok!(whitelist_call.dispatch(fellows_origin));
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				RuntimeEvent::PolkadotXcm(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});
}
