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
use bridge_hub_kusama_runtime::{
	bridge_to_ethereum_config::{EthereumGatewayAddress, EthereumNetwork},
	bridge_to_polkadot_config::RefundBridgeHubPolkadotMessages,
	xcm_config::{XcmConfig, XcmFeeManagerFromComponentsBridgeHub},
	BridgeRejectObsoleteHeadersAndMessages, Executive, MessageQueueServiceWeight, Runtime,
	RuntimeCall, RuntimeEvent, SessionKeys, SignedExtra, UncheckedExtrinsic,
};
use bridge_hub_test_utils::ValidatorIdOf;
use codec::{Decode, Encode};
use cumulus_primitives_core::XcmError::{FailedToTransactAsset, NotHoldingFees};
use frame_support::{
	assert_err, assert_ok, parameter_types,
	traits::{Contains, OnFinalize, OnInitialize},
};
use frame_system::pallet_prelude::BlockNumberFor;
use kusama_runtime_constants::currency::UNITS;
use parachains_common::{AccountId, AuraId, Balance};
pub use parachains_runtimes_test_utils::test_cases::change_storage_constant_by_governance_works;
use parachains_runtimes_test_utils::{
	AccountIdOf, CollatorSessionKeys, ExtBuilder, XcmReceivedFrom,
};
use snowbridge_core::{gwei, meth, ChannelId, ParaId, Rewards};
use snowbridge_pallet_ethereum_client::WeightInfo;
use snowbridge_pallet_system::{PricingParametersOf, WeightInfo as EthereumSystemWeightInfo};
use snowbridge_runtime_test_common::initial_fund;
use sp_core::H160;
use sp_keyring::AccountKeyring::Alice;
use sp_runtime::{
	generic::{Era, SignedPayload},
	traits::Header,
	AccountId32, FixedU128, Saturating,
};
use xcm::{latest::prelude::*, v3::Error};
use xcm_builder::HandleFee;
use xcm_executor::{
	traits::{FeeManager, FeeReason},
	XcmExecutor,
};

type RuntimeHelper<Runtime, AllPalletsWithoutSystem = ()> =
	parachains_runtimes_test_utils::RuntimeHelper<Runtime, AllPalletsWithoutSystem>;

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
	send_transfer_token_message_success::<Runtime, XcmConfig>(
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
		collator_session_keys(),
		1013,
		1000,
		H160::random(),
		H160::random(),
	)
}

#[test]
pub fn transfer_token_to_ethereum_fee_not_enough() {
	send_transfer_token_message_failure::<Runtime, XcmConfig>(
		collator_session_keys(),
		1013,
		1000,
		DefaultBridgeHubEthereumBaseFee::get() + 1_000_000_000,
		H160::random(),
		H160::random(),
		// fee not enough
		1_000_000_000,
		Box::new(|call| RuntimeCall::EthereumSystem(call).encode()),
		NotHoldingFees,
	)
}

#[test]
pub fn transfer_token_to_ethereum_insufficient_fund() {
	send_transfer_token_message_failure::<Runtime, XcmConfig>(
		collator_session_keys(),
		1013,
		1000,
		1_000_000_000,
		H160::random(),
		H160::random(),
		DefaultBridgeHubEthereumBaseFee::get(),
		Box::new(|call| RuntimeCall::EthereumSystem(call).encode()),
		FailedToTransactAsset("Funds are unavailable"),
	)
}

#[test]
fn change_ethereum_gateway_by_governance_works() {
	change_storage_constant_by_governance_works::<Runtime, EthereumGatewayAddress, H160>(
		collator_session_keys(),
		bp_bridge_hub_kusama::BRIDGE_HUB_KUSAMA_PARACHAIN_ID,
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

#[allow(clippy::too_many_arguments)]
pub fn send_transfer_token_message_failure<Runtime, XcmConfig>(
	collator_session_key: CollatorSessionKeys<Runtime>,
	runtime_para_id: u32,
	assethub_parachain_id: u32,
	initial_amount: u128,
	weth_contract_address: H160,
	destination_address: H160,
	fee_amount: u128,
	system_call_encode: Box<dyn Fn(snowbridge_pallet_system::Call<Runtime>) -> Vec<u8>>,
	expected_error: Error,
) where
	Runtime: frame_system::Config
		+ pallet_balances::Config
		+ pallet_session::Config
		+ pallet_xcm::Config
		+ parachain_info::Config
		+ pallet_collator_selection::Config
		+ cumulus_pallet_parachain_system::Config
		+ snowbridge_pallet_outbound_queue::Config
		+ snowbridge_pallet_system::Config,
	XcmConfig: xcm_executor::Config,
	ValidatorIdOf<Runtime>: From<AccountIdOf<Runtime>>,
	<<Runtime as snowbridge_pallet_system::Config>::Token as frame_support::traits::fungible::Inspect<<Runtime as frame_system::Config>::AccountId>>::Balance: From<u128>
{
	ExtBuilder::<Runtime>::default()
		.with_collators(collator_session_key.collators())
		.with_session_keys(collator_session_key.session_keys())
		.with_para_id(runtime_para_id.into())
		.with_tracing()
		.build()
		.execute_with(|| {
			assert_ok!(<snowbridge_pallet_system::Pallet<Runtime>>::initialize(
				runtime_para_id.into(),
				assethub_parachain_id.into(),
			));

			let require_weight_at_most =
				<Runtime as snowbridge_pallet_system::Config>::WeightInfo::set_pricing_parameters();

			let set_pricing_parameters_call = system_call_encode(snowbridge_pallet_system::Call::<
				Runtime,
			>::set_pricing_parameters {
				params: {
					PricingParametersOf::<Runtime> {
						exchange_rate: FixedU128::from_rational(1, 75),
						fee_per_gas: gwei(20),
						rewards: Rewards {
							local: (UNITS / 100).into(), // 0.01 KSM
							remote: meth(1),
						},
						multiplier: FixedU128::from_rational(1, 1),
					}
				},
			});

			assert_ok!(RuntimeHelper::<Runtime>::execute_as_governance(
				set_pricing_parameters_call,
				require_weight_at_most
			)
			.ensure_complete());

			// fund asset hub sovereign account enough so it can pay fees
			initial_fund::<Runtime>(assethub_parachain_id, initial_amount);

			let outcome = send_transfer_token_message::<Runtime, XcmConfig>(
				assethub_parachain_id,
				weth_contract_address,
				destination_address,
				fee_amount,
			);
			assert_err!(outcome.ensure_complete(), expected_error);
		});
}

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
	// TODO: add test after dependencies are upgraded to >= 1.8
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
		(RefundBridgeHubPolkadotMessages::default()),
		frame_metadata_hash_extension::CheckMetadataHash::<Runtime>::new(false),
	);
	let payload = SignedPayload::new(call.clone(), extra.clone()).unwrap();
	let signature = payload.using_encoded(|e| sender.sign(e));
	UncheckedExtrinsic::new_signed(
		call,
		account_id.into(),
		Signature::Sr25519(signature.clone()),
		extra,
	)
}

fn construct_and_apply_extrinsic(
	origin: sp_keyring::AccountKeyring,
	call: RuntimeCall,
) -> sp_runtime::DispatchOutcome {
	let xt = construct_extrinsic(origin, call);
	let r = Executive::apply_extrinsic(xt);
	r.unwrap()
}

// TODO remove when Ethereum network ID has been extracted as a param
pub fn send_transfer_token_message<Runtime, XcmConfig>(
	assethub_parachain_id: u32,
	weth_contract_address: H160,
	destination_address: H160,
	fee_amount: u128,
) -> Outcome
where
	Runtime: frame_system::Config
		+ pallet_balances::Config
		+ pallet_session::Config
		+ pallet_xcm::Config
		+ parachain_info::Config
		+ pallet_collator_selection::Config
		+ cumulus_pallet_parachain_system::Config
		+ snowbridge_pallet_outbound_queue::Config,
	XcmConfig: xcm_executor::Config,
{
	let assethub_parachain_location = Location::new(1, Parachain(assethub_parachain_id));
	let asset = Asset {
		id: AssetId(Location::new(
			0,
			[AccountKey20 { network: None, key: weth_contract_address.into() }],
		)),
		fun: Fungible(1000000000),
	};
	let assets = vec![asset.clone()];

	let inner_xcm = Xcm(vec![
		WithdrawAsset(Assets::from(assets.clone())),
		ClearOrigin,
		BuyExecution { fees: asset, weight_limit: Unlimited },
		DepositAsset {
			assets: Wild(All),
			beneficiary: Location::new(
				0,
				[AccountKey20 { network: None, key: destination_address.into() }],
			),
		},
		SetTopic([0; 32]),
	]);

	let fee =
		Asset { id: AssetId(Location { parents: 1, interior: Here }), fun: Fungible(fee_amount) };

	// prepare transfer token message
	let xcm = Xcm(vec![
		WithdrawAsset(Assets::from(vec![fee.clone()])),
		BuyExecution { fees: fee, weight_limit: Unlimited },
		ExportMessage { network: Ethereum { chain_id: 1 }, destination: Here, xcm: inner_xcm },
	]);

	// execute XCM
	let mut hash = xcm.using_encoded(sp_io::hashing::blake2_256);
	XcmExecutor::<XcmConfig>::prepare_and_execute(
		assethub_parachain_location,
		xcm,
		&mut hash,
		RuntimeHelper::<Runtime>::xcm_max_weight(XcmReceivedFrom::Sibling),
		Weight::zero(),
	)
}

pub fn send_transfer_token_message_success<Runtime, XcmConfig>(
	collator_session_key: CollatorSessionKeys<Runtime>,
	runtime_para_id: u32,
	assethub_parachain_id: u32,
	weth_contract_address: H160,
	destination_address: H160,
	fee_amount: u128,
	snowbridge_pallet_outbound_queue: Box<
		dyn Fn(Vec<u8>) -> Option<snowbridge_pallet_outbound_queue::Event<Runtime>>,
	>,
) where
	Runtime: frame_system::Config
		+ pallet_balances::Config
		+ pallet_session::Config
		+ pallet_xcm::Config
		+ parachain_info::Config
		+ pallet_collator_selection::Config
		+ pallet_message_queue::Config
		+ cumulus_pallet_parachain_system::Config
		+ snowbridge_pallet_outbound_queue::Config
		+ snowbridge_pallet_system::Config,
	XcmConfig: xcm_executor::Config,
	ValidatorIdOf<Runtime>: From<AccountIdOf<Runtime>>,
	<Runtime as frame_system::Config>::AccountId: From<sp_runtime::AccountId32> + AsRef<[u8]>,
{
	ExtBuilder::<Runtime>::default()
		.with_collators(collator_session_key.collators())
		.with_session_keys(collator_session_key.session_keys())
		.with_para_id(runtime_para_id.into())
		.with_tracing()
		.build()
		.execute_with(|| {
			<snowbridge_pallet_system::Pallet<Runtime>>::initialize(
				runtime_para_id.into(),
				assethub_parachain_id.into(),
			)
			.unwrap();

			// fund asset hub sovereign account enough so it can pay fees
			initial_fund::<Runtime>(assethub_parachain_id, 5_000_000_000_000);

			let outcome = send_transfer_token_message::<Runtime, XcmConfig>(
				assethub_parachain_id,
				weth_contract_address,
				destination_address,
				fee_amount,
			);

			assert_ok!(outcome.ensure_complete());

			// check events
			let mut events = <frame_system::Pallet<Runtime>>::events()
				.into_iter()
				.filter_map(|e| snowbridge_pallet_outbound_queue(e.event.encode()));
			assert!(events.any(|e| matches!(
				e,
				snowbridge_pallet_outbound_queue::Event::MessageQueued { .. }
			)));

			let block_number = <frame_system::Pallet<Runtime>>::block_number();
			let next_block_number = <frame_system::Pallet<Runtime>>::block_number()
				.saturating_add(BlockNumberFor::<Runtime>::from(1u32));

			// finish current block
			<pallet_message_queue::Pallet<Runtime>>::on_finalize(block_number);
			<snowbridge_pallet_outbound_queue::Pallet<Runtime>>::on_finalize(block_number);
			<frame_system::Pallet<Runtime>>::on_finalize(block_number);

			// start next block
			<frame_system::Pallet<Runtime>>::set_block_number(next_block_number);
			<frame_system::Pallet<Runtime>>::on_initialize(next_block_number);
			<snowbridge_pallet_outbound_queue::Pallet<Runtime>>::on_initialize(next_block_number);
			<pallet_message_queue::Pallet<Runtime>>::on_initialize(next_block_number);

			// finish next block
			<pallet_message_queue::Pallet<Runtime>>::on_finalize(next_block_number);
			<snowbridge_pallet_outbound_queue::Pallet<Runtime>>::on_finalize(next_block_number);
			let included_head = <frame_system::Pallet<Runtime>>::finalize();

			let origin: ParaId = assethub_parachain_id.into();
			let channel_id: ChannelId = origin.into();

			let nonce = snowbridge_pallet_outbound_queue::Nonce::<Runtime>::try_get(channel_id);
			assert_ok!(nonce);
			assert_eq!(nonce.unwrap(), 1);

			let digest = included_head.digest();

			let digest_items = digest.logs();
			assert!(digest_items.len() == 1 && digest_items[0].as_other().is_some());
		});
}
