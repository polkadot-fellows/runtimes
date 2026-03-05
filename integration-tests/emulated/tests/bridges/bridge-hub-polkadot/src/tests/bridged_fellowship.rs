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

use crate::tests::*;

/// Test that the Polkadot Collectives Fellowship can whitelist a call on Kusama Asset Hub
/// over the bridge, validating that `EnsureXcm<IsVoiceOfBody<FellowshipLocation, FellowsBodyId>>`
/// correctly accepts the bridged Fellows origin.
///
/// CollectivesPolkadot dispatches an XCM using origin: FellowsOrigin, to AHP containing
/// `InitiateTransfer` with `preserve_origin: true` that forwards the message to AHK over
/// the bridge. On AHK, the origin is aliased to the Polkadot Fellows Plurality, then Transact
/// dispatches `whitelist_call` with the Polkadot Fellows Plurality XCM origin.
#[test]
fn fellowship_whitelist_on_kah() {
	use collectives_polkadot_runtime::fellowship::pallet_fellowship_origins::Origin::Fellows as FellowsOrigin;
	use frame_support::BoundedVec;
	use sp_runtime::traits::Dispatchable;

	let call_hash = sp_core::H256::from([1u8; 32]);

	// Encode whitelist_call for AHK
	let whitelist_call: Vec<u8> =
		<AssetHubKusama as Chain>::RuntimeCall::Whitelist(pallet_whitelist::Call::<
			<AssetHubKusama as Chain>::Runtime,
		>::whitelist_call {
			call_hash,
		})
		.encode();

	// XCM to execute on AHK (final destination, appended by InitiateTransfer)
	let xcm_on_ahk = Xcm::<()>(vec![
		Transact {
			origin_kind: OriginKind::Xcm,
			call: whitelist_call.into(),
			fallback_max_weight: None,
		},
		ExpectTransactStatus(MaybeErrorCode::Success),
	]);

	// AHK destination as seen from AHP
	let ahk_from_ahp = asset_hub_kusama_location();

	// XCM to execute on AHP: unpaid execution (system parachain), then InitiateTransfer
	// forwards message to AHK over bridge, preserving the Fellows origin via AliasOrigin.
	let xcm_for_ahp = VersionedXcm::from(Xcm::<()>(vec![
		UnpaidExecution { weight_limit: Unlimited, check_origin: None },
		InitiateTransfer {
			destination: ahk_from_ahp,
			remote_fees: None,
			preserve_origin: true,
			assets: BoundedVec::new(),
			remote_xcm: xcm_on_ahk,
		},
	]));

	// Fund AHP's SA on BHP for bridge transport fees
	BridgeHubPolkadot::fund_para_sovereign(AssetHubPolkadot::para_id(), 10_000_000_000_000u128);

	// Set XCM versions
	AssetHubPolkadot::force_xcm_version(asset_hub_kusama_location(), XCM_VERSION);
	BridgeHubPolkadot::force_xcm_version(bridge_hub_kusama_location(), XCM_VERSION);

	// Send from CollectivesPolkadot with FellowsOrigin to AHP
	let ahp_from_collectives = Location::new(1, [Parachain(AssetHubPolkadot::para_id().into())]);
	CollectivesPolkadot::execute_with(|| {
		type RuntimeCall = <CollectivesPolkadot as Chain>::RuntimeCall;
		type RuntimeOrigin = <CollectivesPolkadot as Chain>::RuntimeOrigin;
		type Runtime = <CollectivesPolkadot as Chain>::Runtime;

		let send_call = RuntimeCall::PolkadotXcm(pallet_xcm::Call::<Runtime>::send {
			dest: bx!(VersionedLocation::from(ahp_from_collectives)),
			message: bx!(xcm_for_ahp),
		});

		let fellows_origin: RuntimeOrigin = FellowsOrigin.into();
		assert_ok!(send_call.dispatch(fellows_origin));

		type RuntimeEvent = <CollectivesPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				RuntimeEvent::PolkadotXcm(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	// AHP processes the InitiateTransfer and forwards to AHK over bridge
	AssetHubPolkadot::execute_with(|| {
		AssetHubPolkadot::assert_xcmp_queue_success(None);
	});

	assert_bridge_hub_polkadot_message_accepted(true);
	assert_bridge_hub_kusama_message_received();

	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
		AssetHubKusama::assert_xcmp_queue_success(None);
		assert_expected_events!(
			AssetHubKusama,
			vec![
				RuntimeEvent::Whitelist(pallet_whitelist::Event::CallWhitelisted {
					call_hash: hash,
				}) => {
					hash: *hash == call_hash,
				},
			]
		);
	});
}
