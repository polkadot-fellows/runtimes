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

use bp_polkadot_core::Signature;
use bridge_hub_polkadot_runtime::{
	bridge_to_kusama_config::{
		AssetHubKusamaParaId, BridgeGrandpaKusamaInstance, BridgeHubKusamaChainId, BridgeParachainKusamaInstance,
		DeliveryRewardInBalance, KusamaGlobalConsensusNetwork, RefundBridgeHubKusamaMessages,
		RequiredStakeForStakeAndSlash, WithBridgeHubKusamaMessageBridge,
		WithBridgeHubKusamaMessagesInstance, XCM_LANE_FOR_ASSET_HUB_POLKADOT_TO_ASSET_HUB_KUSAMA,
	},
	xcm_config::{DotRelayLocation, RelayNetwork, XcmConfig},
	AllPalletsWithoutSystem, BridgeRejectObsoleteHeadersAndMessages, Executive, ExistentialDeposit,
	ParachainSystem, PolkadotXcm, Runtime, RuntimeCall, RuntimeEvent, SessionKeys, SignedExtra,
	UncheckedExtrinsic,
};
use codec::{Decode, Encode};
use frame_support::parameter_types;
use frame_system::pallet_prelude::HeaderFor;
use parachains_common::{polkadot::fee::WeightToFee, AccountId, AuraId, Balance};
use sp_keyring::AccountKeyring::Alice;
use sp_runtime::{
	generic::{Era, SignedPayload},
	AccountId32,
};
use xcm::latest::prelude::*;

const ALICE: [u8; 32] = [1u8; 32];

// Para id of sibling chain used in tests.
pub const SIBLING_PARACHAIN_ID: u32 = 1000;

parameter_types! {
	pub CheckingAccount: AccountId = PolkadotXcm::check_account();
}

fn construct_extrinsic(
	sender: sp_keyring::AccountKeyring,
	call: RuntimeCall,
) -> UncheckedExtrinsic {
	let extra: SignedExtra = (
		frame_system::CheckNonZeroSender::<Runtime>::new(),
		frame_system::CheckSpecVersion::<Runtime>::new(),
		frame_system::CheckTxVersion::<Runtime>::new(),
		frame_system::CheckGenesis::<Runtime>::new(),
		frame_system::CheckEra::<Runtime>::from(Era::immortal()),
		frame_system::CheckNonce::<Runtime>::from(0),
		frame_system::CheckWeight::<Runtime>::new(),
		pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(0),
		BridgeRejectObsoleteHeadersAndMessages,
		(RefundBridgeHubKusamaMessages::default()),
	);
	let payload = SignedPayload::new(call.clone(), extra.clone()).unwrap();
	let signature = payload.using_encoded(|e| sender.sign(e));
	UncheckedExtrinsic::new_signed(
		call,
		AccountId32::from(sender.public()).into(),
		Signature::Sr25519(signature.clone()),
		extra,
	)
}

fn construct_and_apply_extrinsic(
	relayer_at_target: sp_keyring::AccountKeyring,
	batch: pallet_utility::Call<Runtime>,
) -> sp_runtime::DispatchOutcome {
	let batch_call = RuntimeCall::Utility(batch);
	let xt = construct_extrinsic(relayer_at_target, batch_call);
	let r = Executive::apply_extrinsic(xt);
	r.unwrap()
}

fn executive_init_block(header: &HeaderFor<Runtime>) {
	Executive::initialize_block(header)
}

fn collator_session_keys() -> bridge_hub_test_utils::CollatorSessionKeys<Runtime> {
	bridge_hub_test_utils::CollatorSessionKeys::new(
		AccountId::from(Alice),
		AccountId::from(Alice),
		SessionKeys { aura: AuraId::from(Alice.public()) },
	)
}

bridge_hub_test_utils::test_cases::include_teleports_for_native_asset_works!(
	Runtime,
	AllPalletsWithoutSystem,
	XcmConfig,
	CheckingAccount,
	WeightToFee,
	ParachainSystem,
	bridge_hub_test_utils::CollatorSessionKeys::new(
		AccountId::from(ALICE),
		AccountId::from(ALICE),
		SessionKeys { aura: AuraId::from(sp_core::sr25519::Public::from_raw(ALICE)) }
	),
	ExistentialDeposit::get(),
	Box::new(|runtime_event_encoded: Vec<u8>| {
		match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
			Ok(RuntimeEvent::PolkadotXcm(event)) => Some(event),
			_ => None,
		}
	}),
	1002
);

#[test]
fn initialize_bridge_by_governance_works() {
	bridge_hub_test_utils::test_cases::initialize_bridge_by_governance_works::<
		Runtime,
		BridgeGrandpaKusamaInstance,
	>(
		collator_session_keys(),
		bp_bridge_hub_polkadot::BRIDGE_HUB_POLKADOT_PARACHAIN_ID,
		Box::new(|call| RuntimeCall::BridgeKusamaGrandpa(call).encode()),
	)
}

#[test]
fn change_delivery_reward_by_governance_works() {
	bridge_hub_test_utils::test_cases::change_storage_constant_by_governance_works::<
		Runtime,
		DeliveryRewardInBalance,
		Balance,
	>(
		collator_session_keys(),
		bp_bridge_hub_polkadot::BRIDGE_HUB_POLKADOT_PARACHAIN_ID,
		Box::new(|call| RuntimeCall::System(call).encode()),
		|| (DeliveryRewardInBalance::key().to_vec(), DeliveryRewardInBalance::get()),
		|old_value| old_value.checked_mul(2).unwrap(),
	)
}

#[test]
fn change_required_stake_by_governance_works() {
	bridge_hub_test_utils::test_cases::change_storage_constant_by_governance_works::<
		Runtime,
		RequiredStakeForStakeAndSlash,
		Balance,
	>(
		collator_session_keys(),
		bp_bridge_hub_polkadot::BRIDGE_HUB_POLKADOT_PARACHAIN_ID,
		Box::new(|call| RuntimeCall::System(call).encode()),
		|| (RequiredStakeForStakeAndSlash::key().to_vec(), RequiredStakeForStakeAndSlash::get()),
		|old_value| old_value.checked_mul(2).unwrap(),
	)
}

#[test]
fn handle_export_message_from_system_parachain_add_to_outbound_queue_works() {
	bridge_hub_test_utils::test_cases::handle_export_message_from_system_parachain_to_outbound_queue_works::<
			Runtime,
			XcmConfig,
			WithBridgeHubKusamaMessagesInstance,
		>(
			collator_session_keys(),
			bp_bridge_hub_polkadot::BRIDGE_HUB_POLKADOT_PARACHAIN_ID,
			SIBLING_PARACHAIN_ID,
			Box::new(|runtime_event_encoded: Vec<u8>| {
				match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
					Ok(RuntimeEvent::BridgeKusamaMessages(event)) => Some(event),
					_ => None,
				}
			}),
			|| ExportMessage { network: Kusama, destination: X1(Parachain(AssetHubKusamaParaId::get().into())), xcm: Xcm(vec![]) },
			XCM_LANE_FOR_ASSET_HUB_POLKADOT_TO_ASSET_HUB_KUSAMA,
			Some((DotRelayLocation::get(), ExistentialDeposit::get()).into()),
			// value should be >= than value generated by `can_calculate_weight_for_paid_export_message_with_reserve_transfer`
			Some((DotRelayLocation::get(), bp_bridge_hub_polkadot::BridgeHubPolkadotBaseXcmFeeInDots::get()).into()),
			|| (),
		)
}

#[test]
fn message_dispatch_routing_works() {
	bridge_hub_test_utils::test_cases::message_dispatch_routing_works::<
		Runtime,
		AllPalletsWithoutSystem,
		XcmConfig,
		ParachainSystem,
		WithBridgeHubKusamaMessagesInstance,
		RelayNetwork,
		KusamaGlobalConsensusNetwork,
	>(
		collator_session_keys(),
		bp_bridge_hub_polkadot::BRIDGE_HUB_POLKADOT_PARACHAIN_ID,
		SIBLING_PARACHAIN_ID,
		Box::new(|runtime_event_encoded: Vec<u8>| {
			match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
				Ok(RuntimeEvent::ParachainSystem(event)) => Some(event),
				_ => None,
			}
		}),
		Box::new(|runtime_event_encoded: Vec<u8>| {
			match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
				Ok(RuntimeEvent::XcmpQueue(event)) => Some(event),
				_ => None,
			}
		}),
		XCM_LANE_FOR_ASSET_HUB_POLKADOT_TO_ASSET_HUB_KUSAMA,
		|| (),
	)
}

#[test]
fn relayed_incoming_message_works() {
	bridge_hub_test_utils::test_cases::relayed_incoming_message_works::<
		Runtime,
		AllPalletsWithoutSystem,
		XcmConfig,
		ParachainSystem,
		BridgeGrandpaKusamaInstance,
		BridgeParachainKusamaInstance,
		WithBridgeHubKusamaMessagesInstance,
		WithBridgeHubKusamaMessageBridge,
	>(
		collator_session_keys(),
		bp_bridge_hub_polkadot::BRIDGE_HUB_POLKADOT_PARACHAIN_ID,
		bp_bridge_hub_kusama::BRIDGE_HUB_KUSAMA_PARACHAIN_ID,
		SIBLING_PARACHAIN_ID,
		Polkadot,
		XCM_LANE_FOR_ASSET_HUB_POLKADOT_TO_ASSET_HUB_KUSAMA,
		|| (),
	)
}

#[test]
pub fn complex_relay_extrinsic_works() {
	bridge_hub_test_utils::test_cases::complex_relay_extrinsic_works::<
		Runtime,
		AllPalletsWithoutSystem,
		XcmConfig,
		ParachainSystem,
		BridgeGrandpaKusamaInstance,
		BridgeParachainKusamaInstance,
		WithBridgeHubKusamaMessagesInstance,
		WithBridgeHubKusamaMessageBridge,
	>(
		collator_session_keys(),
		bp_bridge_hub_polkadot::BRIDGE_HUB_POLKADOT_PARACHAIN_ID,
		bp_bridge_hub_kusama::BRIDGE_HUB_KUSAMA_PARACHAIN_ID,
		SIBLING_PARACHAIN_ID,
		BridgeHubKusamaChainId::get(),
		Polkadot,
		XCM_LANE_FOR_ASSET_HUB_POLKADOT_TO_ASSET_HUB_KUSAMA,
		ExistentialDeposit::get(),
		executive_init_block,
		construct_and_apply_extrinsic,
		|| (),
	);
}

#[test]
pub fn can_calculate_weight_for_paid_export_message_with_reserve_transfer() {
	let estimated = bridge_hub_test_utils::test_cases::can_calculate_weight_for_paid_export_message_with_reserve_transfer::<
			Runtime,
			XcmConfig,
			WeightToFee,
		>();

	// check if estimated value is sane
	let max_expected = bp_bridge_hub_polkadot::BridgeHubPolkadotBaseXcmFeeInDots::get();
	assert!(
			estimated <= max_expected,
			"calculated: {:?}, max_expected: {:?}, please adjust `bp_bridge_hub_polkadot::BridgeHubPolkadotBaseXcmFeeInDots` value",
			estimated,
			max_expected
		);
}

// TODO: replace me with direct usages of `bridge_hub_test_utils` after deps are bumped to (at
// least) 1.4
//
// Following two tests have to be implemented properly after upgrade to 1.6.
// See https://github.com/paritytech/polkadot-sdk/pull/2139/ and https://github.com/paritytech/parity-bridges-common/pull/2728
// for impl details
//
// Until that, anyone can run it manually by doing following:
//
// 1) cargo vendor ../vendored-dependencies
// 2) apply relevant changes from above PRs
// 3) change workspace Cargo.toml:
// [patch.crates-io]
// bp-polkadot-core = { path = "../vendored-dependencies/bp-polkadot-core" }
// bridge-hub-test-utils = { path = "../vendored-dependencies/bridge-hub-test-utils" }
// bridge-runtime-common = { path = "../vendored-dependencies/bridge-runtime-common" }
// 4) add actual tests code and do `cargo test -p bridge-hub-polkadot-runtime`

#[test]
pub fn can_calculate_fee_for_complex_message_delivery_transaction() {}

#[test]
pub fn can_calculate_fee_for_complex_message_confirmation_transaction() {}
