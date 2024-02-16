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
use bridge_hub_kusama_runtime::{
	bridge_to_polkadot_config::{
		AssetHubPolkadotParaId, BridgeGrandpaPolkadotInstance, BridgeHubPolkadotChainId,
		BridgeHubPolkadotLocation, BridgeParachainPolkadotInstance, DeliveryRewardInBalance,
		PolkadotGlobalConsensusNetwork, RefundBridgeHubPolkadotMessages,
		RequiredStakeForStakeAndSlash, WithBridgeHubPolkadotMessageBridge,
		WithBridgeHubPolkadotMessagesInstance, XCM_LANE_FOR_ASSET_HUB_KUSAMA_TO_ASSET_HUB_POLKADOT,
	},
	xcm_config::{KsmRelayLocation, RelayNetwork, XcmConfig},
	AllPalletsWithoutSystem, BridgeRejectObsoleteHeadersAndMessages, Executive, ExistentialDeposit,
	ParachainSystem, PolkadotXcm, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin, SessionKeys,
	SignedExtra, TransactionPayment, UncheckedExtrinsic,
};
use bridge_hub_test_utils::test_cases::from_parachain;
use codec::{Decode, Encode};
use frame_support::{dispatch::GetDispatchInfo, parameter_types, traits::ConstU8};
use parachains_common::{AccountId, AuraId, Balance};
use sp_keyring::AccountKeyring::Alice;
use sp_runtime::{
	generic::{Era, SignedPayload},
	AccountId32,
};
use system_parachains_constants::kusama::fee::WeightToFee;
use xcm::latest::prelude::*;

const ALICE: [u8; 32] = [1u8; 32];

// Para id of sibling chain used in tests.
pub const SIBLING_PARACHAIN_ID: u32 = 1000;

// Runtime from tests PoV
type RuntimeTestsAdapter = from_parachain::WithRemoteParachainHelperAdapter<
	Runtime,
	AllPalletsWithoutSystem,
	BridgeGrandpaPolkadotInstance,
	BridgeParachainPolkadotInstance,
	WithBridgeHubPolkadotMessagesInstance,
	WithBridgeHubPolkadotMessageBridge,
>;

parameter_types! {
	pub CheckingAccount: AccountId = PolkadotXcm::check_account();
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
	relayer_at_target: sp_keyring::AccountKeyring,
	call: RuntimeCall,
) -> sp_runtime::DispatchOutcome {
	let xt = construct_extrinsic(relayer_at_target, call);
	let r = Executive::apply_extrinsic(xt);
	r.unwrap()
}

fn construct_and_estimate_extrinsic_fee(batch: pallet_utility::Call<Runtime>) -> Balance {
	let batch_call = RuntimeCall::Utility(batch);
	let batch_info = batch_call.get_dispatch_info();
	let xt = construct_extrinsic(Alice, batch_call);
	TransactionPayment::compute_fee(xt.encoded_size() as _, &batch_info, 0)
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
fn test_ed_is_one_tenth_of_relay() {
	let relay_ed = kusama_runtime_constants::currency::EXISTENTIAL_DEPOSIT;
	let bridge_hub_ed = ExistentialDeposit::get();
	assert_eq!(relay_ed / 10, bridge_hub_ed);
}

#[test]
fn initialize_bridge_by_governance_works() {
	bridge_hub_test_utils::test_cases::initialize_bridge_by_governance_works::<
		Runtime,
		BridgeGrandpaPolkadotInstance,
	>(collator_session_keys(), bp_bridge_hub_kusama::BRIDGE_HUB_KUSAMA_PARACHAIN_ID)
}

#[test]
fn change_delivery_reward_by_governance_works() {
	bridge_hub_test_utils::test_cases::change_storage_constant_by_governance_works::<
		Runtime,
		DeliveryRewardInBalance,
		Balance,
	>(
		collator_session_keys(),
		bp_bridge_hub_kusama::BRIDGE_HUB_KUSAMA_PARACHAIN_ID,
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
		bp_bridge_hub_kusama::BRIDGE_HUB_KUSAMA_PARACHAIN_ID,
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
			WithBridgeHubPolkadotMessagesInstance,
		>(
			collator_session_keys(),
			bp_bridge_hub_kusama::BRIDGE_HUB_KUSAMA_PARACHAIN_ID,
			SIBLING_PARACHAIN_ID,
			Box::new(|runtime_event_encoded: Vec<u8>| {
				match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
					Ok(RuntimeEvent::BridgePolkadotMessages(event)) => Some(event),
					_ => None,
				}
			}),
			|| ExportMessage { network: Polkadot, destination: X1(Parachain(AssetHubPolkadotParaId::get().into())), xcm: Xcm(vec![]) },
			XCM_LANE_FOR_ASSET_HUB_KUSAMA_TO_ASSET_HUB_POLKADOT,
			Some((KsmRelayLocation::get(), ExistentialDeposit::get()).into()),
			// value should be >= than value generated by `can_calculate_weight_for_paid_export_message_with_reserve_transfer`
			Some((KsmRelayLocation::get(), bp_bridge_hub_kusama::BridgeHubKusamaBaseXcmFeeInKsms::get()).into()),
			|| PolkadotXcm::force_xcm_version(RuntimeOrigin::root(), Box::new(BridgeHubPolkadotLocation::get()), XCM_VERSION).expect("version saved!"),
		)
}

#[test]
fn message_dispatch_routing_works() {
	bridge_hub_test_utils::test_cases::message_dispatch_routing_works::<
		Runtime,
		AllPalletsWithoutSystem,
		XcmConfig,
		ParachainSystem,
		WithBridgeHubPolkadotMessagesInstance,
		RelayNetwork,
		PolkadotGlobalConsensusNetwork,
		ConstU8<2>,
	>(
		collator_session_keys(),
		bp_bridge_hub_kusama::BRIDGE_HUB_KUSAMA_PARACHAIN_ID,
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
		XCM_LANE_FOR_ASSET_HUB_KUSAMA_TO_ASSET_HUB_POLKADOT,
		|| (),
	)
}

#[test]
fn relayed_incoming_message_works() {
	from_parachain::relayed_incoming_message_works::<RuntimeTestsAdapter>(
		collator_session_keys(),
		bp_bridge_hub_kusama::BRIDGE_HUB_KUSAMA_PARACHAIN_ID,
		bp_bridge_hub_polkadot::BRIDGE_HUB_POLKADOT_PARACHAIN_ID,
		BridgeHubPolkadotChainId::get(),
		SIBLING_PARACHAIN_ID,
		Kusama,
		XCM_LANE_FOR_ASSET_HUB_KUSAMA_TO_ASSET_HUB_POLKADOT,
		|| (),
		construct_and_apply_extrinsic,
	)
}

#[test]
pub fn complex_relay_extrinsic_works() {
	from_parachain::complex_relay_extrinsic_works::<RuntimeTestsAdapter>(
		collator_session_keys(),
		bp_bridge_hub_kusama::BRIDGE_HUB_KUSAMA_PARACHAIN_ID,
		bp_bridge_hub_polkadot::BRIDGE_HUB_POLKADOT_PARACHAIN_ID,
		SIBLING_PARACHAIN_ID,
		BridgeHubPolkadotChainId::get(),
		Kusama,
		XCM_LANE_FOR_ASSET_HUB_KUSAMA_TO_ASSET_HUB_POLKADOT,
		|| (),
		construct_and_apply_extrinsic,
	);
}

#[test]
pub fn can_calculate_weight_for_paid_export_message_with_reserve_transfer() {
	check_sane_fees_values(
		"bp_bridge_hub_kusama::BridgeHubKusamaBaseXcmFeeInKsms",
		bp_bridge_hub_kusama::BridgeHubKusamaBaseXcmFeeInKsms::get(),
		|| {
			bridge_hub_test_utils::test_cases::can_calculate_weight_for_paid_export_message_with_reserve_transfer::<
				Runtime,
				XcmConfig,
				WeightToFee,
			>()
		},
		Perbill::from_percent(33),
		Some(-33),
		&format!(
			"Estimate fee for `ExportMessage` for runtime: {:?}",
			<Runtime as frame_system::Config>::Version::get()
		),
	)
}

#[test]
pub fn can_calculate_fee_for_complex_message_delivery_transaction() {
	check_sane_fees_values(
		"bp_bridge_hub_kusama::BridgeHubKusamaBaseDeliveryFeeInKsms",
		bp_bridge_hub_kusama::BridgeHubKusamaBaseDeliveryFeeInKsms::get(),
		|| {
			from_parachain::can_calculate_fee_for_complex_message_delivery_transaction::<
				RuntimeTestsAdapter,
			>(collator_session_keys(), construct_and_estimate_extrinsic_fee)
		},
		Perbill::from_percent(33),
		Some(-33),
		&format!(
			"Estimate fee for `single message delivery` for runtime: {:?}",
			<Runtime as frame_system::Config>::Version::get()
		),
	)
}

#[test]
pub fn can_calculate_fee_for_complex_message_confirmation_transaction() {
	check_sane_fees_values(
		"bp_bridge_hub_kusama::BridgeHubKusamaBaseConfirmationFeeInKsms",
		bp_bridge_hub_kusama::BridgeHubKusamaBaseConfirmationFeeInKsms::get(),
		|| {
			from_parachain::can_calculate_fee_for_complex_message_confirmation_transaction::<
				RuntimeTestsAdapter,
			>(collator_session_keys(), construct_and_estimate_extrinsic_fee)
		},
		Perbill::from_percent(33),
		Some(-33),
		&format!(
			"Estimate fee for `single message confirmation` for runtime: {:?}",
			<Runtime as frame_system::Config>::Version::get()
		),
	)
}

use sp_runtime::Perbill;

// TODO:(PR#159): remove when `polkadot-sdk@1.8.0` bump (https://github.com/polkadot-fellows/runtimes/issues/186)
/// A helper function for comparing the actual value of a fee constant with its estimated value. The
/// estimated value can be overestimated (`overestimate_in_percent`), and if the difference to the
/// actual value is below `margin_overestimate_diff_in_percent_for_lowering`, we should lower the
/// actual value.
pub fn check_sane_fees_values(
	const_name: &str,
	actual: u128,
	calculate_estimated_fee: fn() -> u128,
	overestimate_in_percent: Perbill,
	margin_overestimate_diff_in_percent_for_lowering: Option<i16>,
	label: &str,
) {
	let estimated = calculate_estimated_fee();
	let estimated_plus_overestimate = estimated + (overestimate_in_percent * estimated);
	let diff_to_estimated = diff_as_percent(actual, estimated);
	let diff_to_estimated_plus_overestimate = diff_as_percent(actual, estimated_plus_overestimate);

	log::error!(
		target: "bridges::estimate",
		"{label}:\nconstant: {const_name}\n[+] actual: {actual}\n[+] estimated: {estimated} ({diff_to_estimated:.2?})\n[+] estimated(+33%): {estimated_plus_overestimate} ({diff_to_estimated_plus_overestimate:.2?})",
	);

	// check if estimated value is sane
	assert!(
		estimated <= actual,
		"estimated: {estimated}, actual: {actual}, please adjust `{const_name}` to the value: {estimated_plus_overestimate}",
	);
	assert!(
		estimated_plus_overestimate <= actual,
		"estimated_plus_overestimate: {estimated_plus_overestimate}, actual: {actual}, please adjust `{const_name}` to the value: {estimated_plus_overestimate}",
	);

	if let Some(margin_overestimate_diff_in_percent_for_lowering) =
		margin_overestimate_diff_in_percent_for_lowering
	{
		assert!(
			diff_to_estimated_plus_overestimate > margin_overestimate_diff_in_percent_for_lowering as f64,
			"diff_to_estimated_plus_overestimate: {diff_to_estimated_plus_overestimate:.2}, overestimate_diff_in_percent_for_lowering: {margin_overestimate_diff_in_percent_for_lowering}, please adjust `{const_name}` to the value: {estimated_plus_overestimate}",
		);
	}
}

// TODO:(PR#159): remove when `polkadot-sdk@1.8.0` bump (https://github.com/polkadot-fellows/runtimes/issues/186)
pub fn diff_as_percent(left: u128, right: u128) -> f64 {
	let left = left as f64;
	let right = right as f64;
	((left - right).abs() / left) * 100f64 * (if left >= right { -1 } else { 1 }) as f64
}
