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

#![cfg(test)]

use bp_polkadot_core::Signature;
use bridge_hub_polkadot_runtime::{
	bridge_to_ethereum_config::{EthereumGatewayAddress, EthereumNetwork},
	bridge_to_kusama_config::OnBridgeHubPolkadotRefundBridgeHubKusamaMessages,
	xcm_config::{XcmConfig, XcmFeeManagerFromComponentsBridgeHub},
	AllPalletsWithoutSystem, BridgeRejectObsoleteHeadersAndMessages, Executive,
	MessageQueueServiceWeight, Runtime, RuntimeCall, RuntimeEvent, SessionKeys, SignedExtra,
	UncheckedExtrinsic,
};
use codec::{Decode, Encode};
use cumulus_primitives_core::XcmError::{FailedToTransactAsset, TooExpensive};
use frame_support::{parameter_types, traits::Contains};
use parachains_common::{AccountId, AuraId, Balance};
pub use parachains_runtimes_test_utils::test_cases::change_storage_constant_by_governance_works;
use snowbridge_pallet_ethereum_client::WeightInfo;
use sp_core::H160;
use sp_keyring::AccountKeyring::Alice;
use sp_runtime::{
	generic::{Era, SignedPayload},
	AccountId32,
};
use xcm::latest::prelude::*;
use xcm_builder::HandleFee;
use xcm_executor::traits::{FeeManager, FeeReason};

parameter_types! {
		pub const DefaultBridgeHubEthereumBaseFee: Balance = 2_750_872_500_000;
}

fn collator_session_keys() -> bridge_hub_test_utils::CollatorSessionKeys<Runtime> {
	bridge_hub_test_utils::CollatorSessionKeys::new(
		AccountId::from(Alice),
		AccountId::from(Alice),
		SessionKeys { aura: AuraId::from(Alice.public()) },
	)
}

#[test]
pub fn transfer_token_to_ethereum_works() {
	snowbridge_runtime_test_common::send_transfer_token_message_success::<Runtime, XcmConfig>(
		1,
		collator_session_keys(),
		1013,
		1000,
		H160::random(),
		H160::random(),
		DefaultBridgeHubEthereumBaseFee::get(),
		Box::new(|runtime_event_encoded: Vec<u8>| {
			match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
				Ok(RuntimeEvent::EthereumOutboundQueue(event)) => Some(event),
				_ => None,
			}
		}),
	)
}

#[test]
pub fn unpaid_transfer_token_to_ethereum_fails_with_barrier() {
	snowbridge_runtime_test_common::send_unpaid_transfer_token_message::<Runtime, XcmConfig>(
		11155111,
		collator_session_keys(),
		1013,
		1000,
		H160::random(),
		H160::random(),
	)
}

#[test]
pub fn transfer_token_to_ethereum_fee_not_enough() {
	snowbridge_runtime_test_common::send_transfer_token_message_failure::<Runtime, XcmConfig>(
		1,
		collator_session_keys(),
		1013,
		1000,
		DefaultBridgeHubEthereumBaseFee::get() + 1_000_000_000,
		H160::random(),
		H160::random(),
		// fee not enough
		1_000_000,
		TooExpensive,
	)
}

#[test]
pub fn transfer_token_to_ethereum_insufficient_fund() {
	snowbridge_runtime_test_common::send_transfer_token_message_failure::<Runtime, XcmConfig>(
		1,
		collator_session_keys(),
		1013,
		1000,
		1_000_000_000,
		H160::random(),
		H160::random(),
		DefaultBridgeHubEthereumBaseFee::get(),
		FailedToTransactAsset("Funds are unavailable"),
	)
}

#[test]
fn change_ethereum_gateway_by_governance_works() {
	change_storage_constant_by_governance_works::<Runtime, EthereumGatewayAddress, H160>(
		collator_session_keys(),
		bp_bridge_hub_polkadot::BRIDGE_HUB_POLKADOT_PARACHAIN_ID,
		Box::new(|call| RuntimeCall::System(call).encode()),
		|| (EthereumGatewayAddress::key().to_vec(), EthereumGatewayAddress::get()),
		|_| [1; 20].into(),
	)
}

/// Fee is not waived when origin is none.
#[test]
fn test_xcm_fee_manager_from_components_bh_origin_none() {
	assert!(!TestXcmFeeManager::is_waived(None, FeeReason::ChargeFees));
}

/// Fee is not waived when origin is not in waived location.
#[test]
fn test_xcm_fee_manager_from_components_bh_origin_not_in_waived_locations() {
	assert!(!TestXcmFeeManager::is_waived(
		Some(&Location::new(1, [Parachain(1)])),
		FeeReason::DepositReserveAsset
	));
}

/// Fee is waived when origin is in waived location.
#[test]
fn test_xcm_fee_manager_from_components_bh_origin_in_waived_locations() {
	assert!(TestXcmFeeManager::is_waived(
		Some(&Location::new(1, [Parachain(2)])),
		FeeReason::DepositReserveAsset
	));
}

/// Fee is waived when origin is in waived location with Export message, but not to Ethereum.
#[test]
fn test_xcm_fee_manager_from_components_bh_origin_in_waived_locations_with_export_to_polkadot_reason(
) {
	assert!(TestXcmFeeManager::is_waived(
		Some(&Location::new(1, [Parachain(2)])),
		FeeReason::Export { network: Polkadot, destination: Here }
	));
}

/// Fee is not waived when origin is in waived location but exported to Ethereum.
#[test]
fn test_xcm_fee_manager_from_components_bh_in_waived_locations_with_export_to_ethereum_reason() {
	assert!(!TestXcmFeeManager::is_waived(
		Some(&Location::new(1, [Parachain(1)])),
		FeeReason::Export { network: EthereumNetwork::get(), destination: Here }
	));
}

struct MockWaivedLocations;
impl Contains<Location> for MockWaivedLocations {
	fn contains(loc: &Location) -> bool {
		loc == &Location::new(1, [Parachain(2)])
	}
}

struct MockFeeHandler;
impl HandleFee for MockFeeHandler {
	fn handle_fee(fee: Assets, _context: Option<&XcmContext>, _reason: FeeReason) -> Assets {
		fee
	}
}

type TestXcmFeeManager = XcmFeeManagerFromComponentsBridgeHub<MockWaivedLocations, MockFeeHandler>;

#[test]
fn max_message_queue_service_weight_is_more_than_beacon_extrinsic_weights() {
	let max_message_queue_weight = MessageQueueServiceWeight::get();
	let force_checkpoint =
		<Runtime as snowbridge_pallet_ethereum_client::Config>::WeightInfo::force_checkpoint();
	let submit_checkpoint =
		<Runtime as snowbridge_pallet_ethereum_client::Config>::WeightInfo::submit();
	max_message_queue_weight.all_gt(force_checkpoint);
	max_message_queue_weight.all_gt(submit_checkpoint);
}

// FAIL-CI @bkontur can you help me to check why it's exceeding the weight limits?
#[test]
fn ethereum_client_consensus_extrinsics_work() {
	snowbridge_runtime_test_common::ethereum_extrinsic(
		collator_session_keys(),
		1013,
		construct_and_apply_extrinsic,
	);
}

#[test]
fn ethereum_to_polkadot_message_extrinsics_work() {
	snowbridge_runtime_test_common::ethereum_to_polkadot_message_extrinsics_work(
		collator_session_keys(),
		1013,
		construct_and_apply_extrinsic,
	);
}

#[test]
fn ethereum_outbound_queue_processes_messages_before_message_queue_works() {
	snowbridge_runtime_test_common::ethereum_outbound_queue_processes_messages_before_message_queue_works::<
		Runtime,
		XcmConfig,
		AllPalletsWithoutSystem,
	>(
		1,
		collator_session_keys(),
		1013,
		1000,
		H160::random(),
		H160::random(),
		DefaultBridgeHubEthereumBaseFee::get(),
		Box::new(|runtime_event_encoded: Vec<u8>| {
			match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
				Ok(RuntimeEvent::EthereumOutboundQueue(event)) => Some(event),
				_ => None,
			}
		}),
	)
}

fn construct_extrinsic(
	sender: sp_keyring::AccountKeyring,
	call: RuntimeCall,
) -> UncheckedExtrinsic {
	let account_id = AccountId32::from(sender.public());
	let extra: SignedExtra = (
		frame_system::CheckNonZeroSender::<Runtime>::new(),
		frame_system::CheckSpecVersion::<Runtime>::new(),
		frame_system::CheckTxVersion::<Runtime>::new(),
		frame_system::CheckGenesis::<Runtime>::new(),
		frame_system::CheckEra::<Runtime>::from(Era::immortal()),
		frame_system::CheckNonce::<Runtime>::from(
			frame_system::Pallet::<Runtime>::account(&account_id).nonce,
		),
		frame_system::CheckWeight::<Runtime>::new(),
		pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(0),
		BridgeRejectObsoleteHeadersAndMessages,
		(OnBridgeHubPolkadotRefundBridgeHubKusamaMessages::default()),
		cumulus_primitives_storage_weight_reclaim::StorageWeightReclaim::new(),
		frame_metadata_hash_extension::CheckMetadataHash::<Runtime>::new(false),
	);
	let payload = SignedPayload::new(call.clone(), extra.clone()).unwrap();
	let signature = payload.using_encoded(|e| sender.sign(e));
	UncheckedExtrinsic::new_signed(call, account_id.into(), Signature::Sr25519(signature), extra)
}

fn construct_and_apply_extrinsic(
	origin: sp_keyring::AccountKeyring,
	call: RuntimeCall,
) -> sp_runtime::DispatchOutcome {
	let xt = construct_extrinsic(origin, call);
	let r = Executive::apply_extrinsic(xt);
	r.unwrap()
}
