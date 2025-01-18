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

use bp_bridge_hub_kusama::Perbill;
use bp_messages::LegacyLaneId;
use bp_polkadot_core::Signature;
use bridge_hub_polkadot_runtime::{
	bridge_to_kusama_config::{
		AssetHubKusamaParaId, BridgeGrandpaKusamaInstance, BridgeHubKusamaLocation,
		BridgeParachainKusamaInstance, DeliveryRewardInBalance, KusamaGlobalConsensusNetwork,
		OnBridgeHubPolkadotRefundBridgeHubKusamaMessages, RelayersForLegacyLaneIdsMessagesInstance,
		RequiredStakeForStakeAndSlash, WithBridgeHubKusamaMessagesInstance,
		XcmOverBridgeHubKusamaInstance,
	},
	xcm_config::{
		DotRelayLocation, LocationToAccountId, RelayNetwork, RelayTreasuryLocation,
		RelayTreasuryPalletAccount, XcmConfig,
	},
	AllPalletsWithoutSystem, BridgeRejectObsoleteHeadersAndMessages, Executive, ExistentialDeposit,
	ParachainSystem, PolkadotXcm, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin, SessionKeys,
	SignedExtra, TransactionPayment, UncheckedExtrinsic, SLOT_DURATION,
};
use bridge_hub_test_utils::{test_cases::from_parachain, SlotDurations};
use codec::{Decode, Encode};
use frame_support::{dispatch::GetDispatchInfo, parameter_types, traits::ConstU8};
use parachains_common::{AccountId, AuraId, Balance};
use sp_consensus_aura::SlotDuration;
use sp_core::crypto::Ss58Codec;
use sp_keyring::AccountKeyring::Alice;
use sp_runtime::{
	generic::{Era, SignedPayload},
	AccountId32,
};
use system_parachains_constants::polkadot::{
	consensus::RELAY_CHAIN_SLOT_DURATION_MILLIS, fee::WeightToFee,
};
use xcm::latest::prelude::*;
use xcm_executor::traits::ConvertLocation;
use xcm_runtime_apis::conversions::LocationToAccountHelper;

// Para id of sibling chain used in tests.
pub const SIBLING_PARACHAIN_ID: u32 = 1000;
// Random para id of sibling chain used in tests.
pub const SIBLING_SYSTEM_PARACHAIN_ID: u32 = 1008;
// Random para id of bridged chain from different global consensus used in tests.
pub const BRIDGED_LOCATION_PARACHAIN_ID: u32 = 1000;

parameter_types! {
	pub SiblingParachainLocation: Location = Location::new(1, [Parachain(SIBLING_PARACHAIN_ID)]);
	pub SiblingSystemParachainLocation: Location = Location::new(1, [Parachain(SIBLING_SYSTEM_PARACHAIN_ID)]);
	pub BridgedUniversalLocation: InteriorLocation = [GlobalConsensus(KusamaGlobalConsensusNetwork::get()), Parachain(BRIDGED_LOCATION_PARACHAIN_ID)].into();
}

// Runtime from tests PoV
type RuntimeTestsAdapter = from_parachain::WithRemoteParachainHelperAdapter<
	Runtime,
	AllPalletsWithoutSystem,
	BridgeGrandpaKusamaInstance,
	BridgeParachainKusamaInstance,
	WithBridgeHubKusamaMessagesInstance,
	RelayersForLegacyLaneIdsMessagesInstance,
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
		(OnBridgeHubPolkadotRefundBridgeHubKusamaMessages::default()),
		frame_metadata_hash_extension::CheckMetadataHash::<Runtime>::new(false),
	);
	let payload = SignedPayload::new(call.clone(), extra.clone()).unwrap();
	let signature = payload.using_encoded(|e| sender.sign(e));
	UncheckedExtrinsic::new_signed(call, account_id.into(), Signature::Sr25519(signature), extra)
}

fn construct_and_apply_extrinsic(
	relayer_at_target: sp_keyring::AccountKeyring,
	call: RuntimeCall,
) -> sp_runtime::DispatchOutcome {
	let xt = construct_extrinsic(relayer_at_target, call);
	let r = Executive::apply_extrinsic(xt);
	r.unwrap()
}

fn construct_and_estimate_extrinsic_fee(call: RuntimeCall) -> Balance {
	let info = call.get_dispatch_info();
	let xt = construct_extrinsic(Alice, call);
	TransactionPayment::compute_fee(xt.encoded_size() as _, &info, 0)
}

fn collator_session_keys() -> bridge_hub_test_utils::CollatorSessionKeys<Runtime> {
	bridge_hub_test_utils::CollatorSessionKeys::new(
		AccountId::from(Alice),
		AccountId::from(Alice),
		SessionKeys { aura: AuraId::from(Alice.public()) },
	)
}

fn slot_durations() -> SlotDurations {
	SlotDurations {
		relay: SlotDuration::from_millis(RELAY_CHAIN_SLOT_DURATION_MILLIS.into()),
		para: SlotDuration::from_millis(SLOT_DURATION),
	}
}

bridge_hub_test_utils::test_cases::include_teleports_for_native_asset_works!(
	Runtime,
	AllPalletsWithoutSystem,
	XcmConfig,
	CheckingAccount,
	WeightToFee,
	ParachainSystem,
	collator_session_keys(),
	slot_durations(),
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
	let relay_ed = polkadot_runtime_constants::currency::EXISTENTIAL_DEPOSIT;
	let bridge_hub_ed = ExistentialDeposit::get();
	assert_eq!(relay_ed / 10, bridge_hub_ed);
}

#[test]
fn initialize_bridge_by_governance_works() {
	bridge_hub_test_utils::test_cases::initialize_bridge_by_governance_works::<
		Runtime,
		BridgeGrandpaKusamaInstance,
	>(collator_session_keys(), bp_bridge_hub_polkadot::BRIDGE_HUB_POLKADOT_PARACHAIN_ID)
}

#[test]
fn change_bridge_grandpa_pallet_mode_by_governance_works() {
	// for Kusama finality
	bridge_hub_test_utils::test_cases::change_bridge_grandpa_pallet_mode_by_governance_works::<
		Runtime,
		BridgeGrandpaKusamaInstance,
	>(collator_session_keys(), bp_bridge_hub_polkadot::BRIDGE_HUB_POLKADOT_PARACHAIN_ID)
}

#[test]
fn change_bridge_parachains_pallet_mode_by_governance_works() {
	// for Kusama parachains finality
	bridge_hub_test_utils::test_cases::change_bridge_parachains_pallet_mode_by_governance_works::<
		Runtime,
		BridgeParachainKusamaInstance,
	>(collator_session_keys(), bp_bridge_hub_polkadot::BRIDGE_HUB_POLKADOT_PARACHAIN_ID)
}

#[test]
fn change_bridge_messages_pallet_mode_by_governance_works() {
	// for Kusama messages
	bridge_hub_test_utils::test_cases::change_bridge_messages_pallet_mode_by_governance_works::<
		Runtime,
		WithBridgeHubKusamaMessagesInstance,
	>(collator_session_keys(), bp_bridge_hub_polkadot::BRIDGE_HUB_POLKADOT_PARACHAIN_ID)
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
			|| ExportMessage { network: Kusama, destination: Parachain(AssetHubKusamaParaId::get().into()).into(), xcm: Xcm(vec![]) },
			Some((DotRelayLocation::get(), ExistentialDeposit::get()).into()),
			// value should be >= than value generated by `can_calculate_weight_for_paid_export_message_with_reserve_transfer`
			Some((DotRelayLocation::get(), bp_bridge_hub_polkadot::BridgeHubPolkadotBaseXcmFeeInDots::get()).into()),
			|| {
				PolkadotXcm::force_xcm_version(RuntimeOrigin::root(), Box::new(BridgeHubKusamaLocation::get()), XCM_VERSION).expect("version saved!");

				// we need to create lane between sibling parachain and remote destination
				bridge_hub_test_utils::ensure_opened_bridge::<
				Runtime,
				XcmOverBridgeHubKusamaInstance,
				LocationToAccountId,
				DotRelayLocation,
			>(
				SiblingParachainLocation::get(),
				BridgedUniversalLocation::get(),
				false,
				|locations, _fee| {
					bridge_hub_test_utils::open_bridge_with_storage::<
						Runtime,
						XcmOverBridgeHubKusamaInstance
					>(locations, LegacyLaneId([0, 0, 0, 1]))
				}
			).1
			},
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
		ConstU8<2>,
	>(
		collator_session_keys(),
		slot_durations(),
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
		|| (),
	)
}

#[test]
fn relayed_incoming_message_works() {
	from_parachain::relayed_incoming_message_works::<RuntimeTestsAdapter>(
		collator_session_keys(),
		slot_durations(),
		bp_bridge_hub_polkadot::BRIDGE_HUB_POLKADOT_PARACHAIN_ID,
		bp_bridge_hub_kusama::BRIDGE_HUB_KUSAMA_PARACHAIN_ID,
		SIBLING_PARACHAIN_ID,
		Polkadot,
		|| {
			// we need to create lane between sibling parachain and remote destination
			bridge_hub_test_utils::ensure_opened_bridge::<
				Runtime,
				XcmOverBridgeHubKusamaInstance,
				LocationToAccountId,
				DotRelayLocation,
			>(
				SiblingParachainLocation::get(),
				BridgedUniversalLocation::get(),
				false,
				|locations, _fee| {
					bridge_hub_test_utils::open_bridge_with_storage::<
						Runtime,
						XcmOverBridgeHubKusamaInstance,
					>(locations, LegacyLaneId([0, 0, 0, 1]))
				},
			)
			.1
		},
		construct_and_apply_extrinsic,
		true,
	)
}

#[test]
fn free_relay_extrinsic_works() {
	// from Polkadot
	from_parachain::free_relay_extrinsic_works::<RuntimeTestsAdapter>(
		collator_session_keys(),
		slot_durations(),
		bp_bridge_hub_polkadot::BRIDGE_HUB_POLKADOT_PARACHAIN_ID,
		bp_bridge_hub_kusama::BRIDGE_HUB_KUSAMA_PARACHAIN_ID,
		SIBLING_PARACHAIN_ID,
		Polkadot,
		|| {
			// we need to create lane between sibling parachain and remote destination
			bridge_hub_test_utils::ensure_opened_bridge::<
				Runtime,
				XcmOverBridgeHubKusamaInstance,
				LocationToAccountId,
				DotRelayLocation,
			>(
				SiblingParachainLocation::get(),
				BridgedUniversalLocation::get(),
				false,
				|locations, _fee| {
					bridge_hub_test_utils::open_bridge_with_storage::<
						Runtime,
						XcmOverBridgeHubKusamaInstance,
					>(locations, LegacyLaneId([0, 0, 0, 1]))
				},
			)
			.1
		},
		construct_and_apply_extrinsic,
		true,
	)
}

#[test]
pub fn can_calculate_weight_for_paid_export_message_with_reserve_transfer() {
	bridge_hub_test_utils::check_sane_fees_values(
		"bp_bridge_hub_polkadot::BridgeHubPolkadotBaseXcmFeeInDots",
		bp_bridge_hub_polkadot::BridgeHubPolkadotBaseXcmFeeInDots::get(),
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
pub fn can_calculate_fee_for_standalone_message_delivery_transaction() {
	bridge_hub_test_utils::check_sane_fees_values(
		"bp_bridge_hub_polkadot::BridgeHubPolkadotBaseDeliveryFeeInDots",
		bp_bridge_hub_polkadot::BridgeHubPolkadotBaseDeliveryFeeInDots::get(),
		|| {
			from_parachain::can_calculate_fee_for_standalone_message_delivery_transaction::<
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
pub fn can_calculate_fee_for_standalone_message_confirmation_transaction() {
	bridge_hub_test_utils::check_sane_fees_values(
		"bp_bridge_hub_polkadot::BridgeHubPolkadotBaseConfirmationFeeInDots",
		bp_bridge_hub_polkadot::BridgeHubPolkadotBaseConfirmationFeeInDots::get(),
		|| {
			from_parachain::can_calculate_fee_for_standalone_message_confirmation_transaction::<
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

#[test]
fn treasury_pallet_account_not_none() {
	assert_eq!(
		RelayTreasuryPalletAccount::get(),
		LocationToAccountId::convert_location(&RelayTreasuryLocation::get()).unwrap()
	)
}

#[test]
fn location_conversion_works() {
	let alice_32 = xcm::prelude::AccountId32 {
		network: None,
		id: polkadot_core_primitives::AccountId::from(Alice).into(),
	};
	let bob_20 = AccountKey20 { network: None, key: [123u8; 20] };

	// the purpose of hardcoded values is to catch an unintended location conversion logic change.
	struct TestCase {
		description: &'static str,
		location: Location,
		expected_account_id_str: &'static str,
	}

	let test_cases = vec![
		// DescribeTerminus
		TestCase {
			description: "DescribeTerminus Parent",
			location: Location::new(1, Here),
			expected_account_id_str: "5Dt6dpkWPwLaH4BBCKJwjiWrFVAGyYk3tLUabvyn4v7KtESG",
		},
		TestCase {
			description: "DescribeTerminus Sibling",
			location: Location::new(1, [Parachain(1111)]),
			expected_account_id_str: "5Eg2fnssmmJnF3z1iZ1NouAuzciDaaDQH7qURAy3w15jULDk",
		},
		// DescribePalletTerminal
		TestCase {
			description: "DescribePalletTerminal Parent",
			location: Location::new(1, [PalletInstance(50)]),
			expected_account_id_str: "5CnwemvaAXkWFVwibiCvf2EjqwiqBi29S5cLLydZLEaEw6jZ",
		},
		TestCase {
			description: "DescribePalletTerminal Sibling",
			location: Location::new(1, [Parachain(1111), PalletInstance(50)]),
			expected_account_id_str: "5GFBgPjpEQPdaxEnFirUoa51u5erVx84twYxJVuBRAT2UP2g",
		},
		// DescribeAccountId32Terminal
		TestCase {
			description: "DescribeAccountId32Terminal Parent",
			location: Location::new(1, [alice_32]),
			expected_account_id_str: "5EueAXd4h8u75nSbFdDJbC29cmi4Uo1YJssqEL9idvindxFL",
		},
		TestCase {
			description: "DescribeAccountId32Terminal Sibling",
			location: Location::new(1, [Parachain(1111), alice_32]),
			expected_account_id_str: "5Dmbuiq48fU4iW58FKYqoGbbfxFHjbAeGLMtjFg6NNCw3ssr",
		},
		// DescribeAccountKey20Terminal
		TestCase {
			description: "DescribeAccountKey20Terminal Parent",
			location: Location::new(1, [bob_20]),
			expected_account_id_str: "5CJeW9bdeos6EmaEofTUiNrvyVobMBfWbdQvhTe6UciGjH2n",
		},
		TestCase {
			description: "DescribeAccountKey20Terminal Sibling",
			location: Location::new(1, [Parachain(1111), bob_20]),
			expected_account_id_str: "5CE6V5AKH8H4rg2aq5KMbvaVUDMumHKVPPQEEDMHPy3GmJQp",
		},
		// DescribeTreasuryVoiceTerminal
		TestCase {
			description: "DescribeTreasuryVoiceTerminal Parent",
			location: Location::new(1, [Plurality { id: BodyId::Treasury, part: BodyPart::Voice }]),
			expected_account_id_str: "5CUjnE2vgcUCuhxPwFoQ5r7p1DkhujgvMNDHaF2bLqRp4D5F",
		},
		TestCase {
			description: "DescribeTreasuryVoiceTerminal Sibling",
			location: Location::new(
				1,
				[Parachain(1111), Plurality { id: BodyId::Treasury, part: BodyPart::Voice }],
			),
			expected_account_id_str: "5G6TDwaVgbWmhqRUKjBhRRnH4ry9L9cjRymUEmiRsLbSE4gB",
		},
		// DescribeBodyTerminal
		TestCase {
			description: "DescribeBodyTerminal Parent",
			location: Location::new(1, [Plurality { id: BodyId::Unit, part: BodyPart::Voice }]),
			expected_account_id_str: "5EBRMTBkDisEXsaN283SRbzx9Xf2PXwUxxFCJohSGo4jYe6B",
		},
		TestCase {
			description: "DescribeBodyTerminal Sibling",
			location: Location::new(
				1,
				[Parachain(1111), Plurality { id: BodyId::Unit, part: BodyPart::Voice }],
			),
			expected_account_id_str: "5DBoExvojy8tYnHgLL97phNH975CyT45PWTZEeGoBZfAyRMH",
		},
	];

	for tc in test_cases {
		let expected = polkadot_core_primitives::AccountId::from_string(tc.expected_account_id_str)
			.expect("Invalid AccountId string");

		let got = LocationToAccountHelper::<polkadot_core_primitives::AccountId, LocationToAccountId>::convert_location(
			tc.location.into(),
		)
			.unwrap();

		assert_eq!(got, expected, "{}", tc.description);
	}
}
