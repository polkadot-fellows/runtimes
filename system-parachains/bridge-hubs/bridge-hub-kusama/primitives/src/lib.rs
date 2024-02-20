// Copyright (C) Parity Technologies (UK) Ltd.
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
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! Module with configuration which reflects BridgeHubKusama runtime setup (AccountId, Headers,
//! Hashes...)

#![cfg_attr(not(feature = "std"), no_std)]

pub use bp_bridge_hub_cumulus::*;
use bp_messages::*;
use bp_runtime::{
	decl_bridge_finality_runtime_apis, decl_bridge_messages_runtime_apis, Chain, ChainId, Parachain,
};
use frame_support::{
	dispatch::DispatchClass,
	sp_runtime::{MultiAddress, MultiSigner},
};
use sp_runtime::{FixedPointNumber, FixedU128, RuntimeDebug, Saturating};

/// BridgeHubKusama parachain.
#[derive(RuntimeDebug)]
pub struct BridgeHubKusama;

impl Chain for BridgeHubKusama {
	const ID: ChainId = *b"bhks";

	type BlockNumber = BlockNumber;
	type Hash = Hash;
	type Hasher = Hasher;
	type Header = Header;

	type AccountId = AccountId;
	type Balance = Balance;
	type Nonce = Nonce;
	type Signature = Signature;

	fn max_extrinsic_size() -> u32 {
		*BlockLength::get().max.get(DispatchClass::Normal)
	}

	fn max_extrinsic_weight() -> Weight {
		BlockWeights::get()
			.get(DispatchClass::Normal)
			.max_extrinsic
			.unwrap_or(Weight::MAX)
	}
}

impl Parachain for BridgeHubKusama {
	const PARACHAIN_ID: u32 = BRIDGE_HUB_KUSAMA_PARACHAIN_ID;
}

impl ChainWithMessages for BridgeHubKusama {
	const WITH_CHAIN_MESSAGES_PALLET_NAME: &'static str =
		WITH_BRIDGE_HUB_KUSAMA_MESSAGES_PALLET_NAME;
	const MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX: MessageNonce =
		MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX;
	const MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX: MessageNonce =
		MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX;
}

/// Public key of the chain account that may be used to verify signatures.
pub type AccountSigner = MultiSigner;

/// The address format for describing accounts.
pub type Address = MultiAddress<AccountId, ()>;

/// Identifier of BridgeHubKusama in the Kusama relay chain.
pub const BRIDGE_HUB_KUSAMA_PARACHAIN_ID: u32 = 1002;

/// Name of the With-BridgeHubKusama messages pallet instance that is deployed at bridged chains.
pub const WITH_BRIDGE_HUB_KUSAMA_MESSAGES_PALLET_NAME: &str = "BridgeKusamaMessages";

/// Name of the With-BridgeHubKusama bridge-relayers pallet instance that is deployed at bridged
/// chains.
pub const WITH_BRIDGE_HUB_KUSAMA_RELAYERS_PALLET_NAME: &str = "BridgeRelayers";

/// Pallet index of `BridgePolkadotMessages: pallet_bridge_messages::<Instance1>`.
pub const WITH_BRIDGE_KUSAMA_TO_POLKADOT_MESSAGES_PALLET_INDEX: u8 = 53;

decl_bridge_finality_runtime_apis!(bridge_hub_kusama);
decl_bridge_messages_runtime_apis!(bridge_hub_kusama);

frame_support::parameter_types! {
	/// The XCM fee that is paid for executing XCM program (with `ExportMessage` instruction) at the Kusama
	/// BridgeHub.
	/// (initially was calculated by test `BridgeHubKusama::can_calculate_weight_for_paid_export_message_with_reserve_transfer` + `33%`)
	pub const BridgeHubKusamaBaseXcmFeeInKsms: u128 = 386_365_000;

	/// Transaction fee that is paid at the Kusama BridgeHub for delivering single inbound message.
	/// (initially was calculated by test `BridgeHubKusama::can_calculate_fee_for_complex_message_delivery_transaction` + `33%`)
	pub const BridgeHubKusamaBaseDeliveryFeeInKsms: u128 = 56_374_989_788;

	/// Transaction fee that is paid at the Kusama BridgeHub for delivering single outbound message confirmation.
	/// (initially was calculated by test `BridgeHubKusama::can_calculate_fee_for_complex_message_confirmation_transaction` + `33%`)
	pub const BridgeHubKusamaBaseConfirmationFeeInKsms: u128 = 53_808_755_240;
}

/// Compute the total estimated fee that needs to be paid in KSMs by the sender when sending
/// message from Kusama Bridge Hub to Polkadot Bridge Hub.
pub fn estimate_kusama_to_polkadot_message_fee(
	bridge_hub_polkadot_base_delivery_fee_in_udots: Balance,
) -> Balance {
	// Sender must pay:
	//
	// 1) an approximate cost of XCM execution (`ExportMessage` and surroundings) at Kusama bridge
	//    Hub;
	//
	// 2) the approximate cost of Kusama -> Polkadot message delivery transaction on Polkadot Bridge
	//    Hub, converted into KSMs using 1:5 conversion rate;
	//
	// 3) the approximate cost of Kusama -> Polkadot message confirmation transaction on Kusama
	//    Bridge Hub.
	BridgeHubKusamaBaseXcmFeeInKsms::get()
		.saturating_add(convert_from_udot_to_uksm(bridge_hub_polkadot_base_delivery_fee_in_udots))
		.saturating_add(BridgeHubKusamaBaseConfirmationFeeInKsms::get())
}

/// Convert from uDOTs to uKSMs.
fn convert_from_udot_to_uksm(price_in_udot: Balance) -> Balance {
	// assuming exchange rate is 5 DOTs for 1 KSM
	let ksm_to_dot_economic_rate = FixedU128::from_rational(1, 5);
	// tokens have different nominals and we need to take that into account
	let nominal_ratio = FixedU128::from_rational(
		kusama_runtime_constants::currency::UNITS,
		polkadot_runtime_constants::currency::UNITS,
	);

	ksm_to_dot_economic_rate
		.saturating_mul(nominal_ratio)
		.saturating_mul(FixedU128::saturating_from_integer(price_in_udot))
		.into_inner() /
		FixedU128::DIV
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn convert_from_udot_to_uksm_works() {
		let price_in_udot = 77 * polkadot_runtime_constants::currency::UNITS;
		let same_price_in_uksm = convert_from_udot_to_uksm(price_in_udot);

		let price_in_dot =
			FixedU128::from_rational(price_in_udot, polkadot_runtime_constants::currency::UNITS);
		let price_in_ksm =
			FixedU128::from_rational(same_price_in_uksm, kusama_runtime_constants::currency::UNITS);
		assert_eq!(price_in_dot / FixedU128::saturating_from_integer(5), price_in_ksm);
	}
}
