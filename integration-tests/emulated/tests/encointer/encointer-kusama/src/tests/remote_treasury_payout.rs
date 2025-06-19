use crate::*;
use emulated_integration_tests_common::{
	xcm_emulator::{sp_tracing, ConvertLocation},
	USDT_ID,
};
use encointer_kusama_runtime::{
	treasuries_xcm_payout::{ConstantKsmFee, GetRemoteFee},
	AccountId,
};
use frame_support::{
	assert_ok,
	traits::{fungible::Mutate as M, fungibles::Mutate},
};
use kusama_system_emulated_network::asset_hub_kusama_emulated_chain::AssetHubKusamaParaPallet;
use polkadot_runtime_common::impls::VersionedLocatableAsset;
use xcm::latest::Junctions::X2;
use xcm_runtime_apis::fees::runtime_decl_for_xcm_payment_api::XcmPaymentApiV1;

fn remote_fee() -> u128 {
	let fee_asset = ConstantKsmFee::get_remote_fee(Xcm::new(), None);
	let Asset { id: _, ref fun } = fee_asset;
	let fee_amount = match fun {
		Fungible(fee) => *fee,
		NonFungible(_) => panic!("Invalid fee"),
	};

	fee_amount
}

fn treasury_account() -> AccountId {
	<EncointerKusama as TestExt>::execute_with(|| {
		encointer_kusama_runtime::EncointerTreasuries::get_community_treasury_account_unchecked(
			None,
		)
	})
}

fn treasury_location_on_ah() -> Location {
	// Transact the parents native asset on parachain 1000.
	let asset_kind = VersionedLocatableAsset::V5 {
		location: (Parent, Parachain(1000)).into(),
		asset_id: v5::AssetId(Location::parent()),
	};

	let treasury_account = treasury_account();

	<EncointerKusama as TestExt>::execute_with(|| {
		let treasury_location_on_ah = encointer_kusama_runtime::TransferOverXcm::sender_on_remote(
			&treasury_account,
			asset_kind.clone(),
		)
		.unwrap();

		treasury_location_on_ah
	})
}

fn treasury_account_on_ah() -> AccountId {
	let treasury_location_on_ah = treasury_location_on_ah();
	println!("treasury_location_on_ah: {:?}", treasury_location_on_ah);
	let treasury_account_on_ah =
		encointer_kusama_runtime::xcm_config::LocationToAccountId::convert_location(
			&treasury_location_on_ah,
		)
		.unwrap();
	println!("treasury_account_on_ah: {:?}", treasury_account_on_ah);

	treasury_account_on_ah
}

#[test]
fn treasury_location_on_ah_works() {
	let treasury = treasury_account();
	assert_eq!(
		treasury_location_on_ah(),
		Location::new(
			1,
			X2([Parachain(1001), AccountId32 { network: None, id: treasury.into() }].into(),),
		)
	);
}

#[test]
fn treasury_location_to_account_id_works() {
	let treasury_location_on_ah = treasury_location_on_ah();

	let treasury_account_on_assethub_encointer =
		encointer_kusama_runtime::xcm_config::LocationToAccountId::convert_location(
			&treasury_location_on_ah,
		)
		.unwrap();

	let treasury_account_on_assethub_ah =
		asset_hub_kusama_runtime::xcm_config::LocationToAccountId::convert_location(
			&treasury_location_on_ah,
		)
		.unwrap();

	assert_eq!(treasury_account_on_assethub_ah, treasury_account_on_assethub_encointer);
}

#[test]
fn constant_remote_execution_fees_are_correct() {
	let sender = AccountId::new([1u8; 32]);
	let recipient = AccountId::new([5u8; 32]);

	// Transact the parents native asset on parachain 1000.
	let asset_kind = VersionedLocatableAsset::V5 {
		location: (Parent, Parachain(1000)).into(),
		asset_id: v5::AssetId(Location::parent()),
	};

	let transfer_amount = 1_000_000_000_000u128;

	let mut remote_message = Xcm::<()>::new();
	<EncointerKusama as TestExt>::execute_with(|| {
		let (message, _, _) = encointer_kusama_runtime::TransferOverXcm::get_remote_transfer_xcm(
			&sender,
			&recipient,
			asset_kind.clone(),
			transfer_amount,
		)
		.unwrap();
		remote_message = message;
	});

	let mut execution_fees = 0;

	<AssetHubKusama as TestExt>::execute_with(|| {
		type Runtime = <AssetHubKusama as Chain>::Runtime;

		let weight = Runtime::query_xcm_weight(VersionedXcm::V5(remote_message.clone())).unwrap();
		execution_fees = Runtime::query_weight_to_asset_fee(
			weight,
			VersionedAssetId::from(AssetId(Location::parent())),
		)
		.unwrap();
	});

	assert_eq!(
		// The constant fee ignores the xcm anyhow
		ConstantKsmFee::get_remote_fee(Xcm::new(), None),
		(Location::parent(), execution_fees).into()
	);
}

#[test]
fn remote_treasury_payout_works() {
	sp_tracing::init_for_tests();

	const SPEND_AMOUNT: u128 = 10_000_000;
	const ONE_KSM: u128 = 1_000_000_000_000;
	const TREASURY_INITIAL_BALANCE: u128 = 100 * ONE_KSM;
	let recipient = AccountId::new([5u8; 32]);

	let asset_kind = VersionedLocatableAsset::V5 {
		location: (Parent, Parachain(1000)).into(),
		asset_id: AssetId((PalletInstance(50), GeneralIndex(USDT_ID.into())).into()),
	};

	let treasury_account = treasury_account_on_ah();
	println!("treasury_account: {:?}", treasury_account);

	<AssetHubKusama as TestExt>::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaParaPallet>::Assets;
		type Balances = <AssetHubKusama as AssetHubKusamaParaPallet>::Balances;

		// USDT created at genesis, mint some assets to the treasury account.
		assert_ok!(<Assets as Mutate<_>>::mint_into(USDT_ID, &treasury_account, SPEND_AMOUNT * 4));
		assert_ok!(<Balances as M<_>>::mint_into(&treasury_account, TREASURY_INITIAL_BALANCE));

		// // Check starting balance
		assert_eq!(Assets::balance(USDT_ID, &treasury_account), SPEND_AMOUNT * 4);
		assert_eq!(Balances::free_balance(&treasury_account), TREASURY_INITIAL_BALANCE);
		assert_eq!(Assets::balance(USDT_ID, &recipient), 0);
	});

	<EncointerKusama as TestExt>::execute_with(|| {
		encointer_kusama_runtime::EncointerTreasuries::do_spend_asset(
			None,
			&recipient,
			asset_kind.clone(),
			SPEND_AMOUNT,
		)
		.unwrap();
	});

	<AssetHubKusama as TestExt>::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaParaPallet>::Assets;
		type Balances = <AssetHubKusama as AssetHubKusamaParaPallet>::Balances;

		// Check ending balance
		assert_eq!(
			Balances::free_balance(&treasury_account),
			TREASURY_INITIAL_BALANCE - remote_fee()
		);
		assert_eq!(Assets::balance(USDT_ID, &treasury_account), SPEND_AMOUNT * 3);
		assert_eq!(Assets::balance(USDT_ID, &recipient), SPEND_AMOUNT);
	});
}

#[test]
fn account_from_log_matches() {
	// Fixme: Why do we get this in the above test. We fund the correct account:
	// withdraw_asset what=Asset { id: AssetId(Location { parents: 1, interior: Here }), fun:
	// Fungible(12749033321) } who=Location  { parents: 1, interior: X2([Parachain(1001),
	// AccountId32 { network: None, id: [150, 141, 187, 98, 102, 33, 87, 174, 108, 105, 38, 201, 33,
	// 252, 99, 215, 105, 11, 253, 230, 89, 13, 87, 138, 18, 41, 154, 220, 108, 179, 239, 229] }]) }
	// 2025-06-19T07:45:58.473305Z TRACE get_version_1: state: method="Get" ext_id=7eba
	// key=26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da9759e3ef811e8e4c7e6df550a0dfaf910084bd52ce2fef65d1481558542743bf14359db2cffba3f1b98431daf7c55dc25
	// result=None result_encoded=00 2025-06-19T07:45:58.473312Z TRACE get_version_1: state:
	// method="Get" ext_id=7eba key=3a7472616e73616374696f6e5f6c6576656c3a result=Some(02000000)
	// result_encoded=0102000000 2025-06-19T07:45:58.473315Z TRACE set_version_1: state:
	// method="Put" ext_id=7eba key=3a7472616e73616374696f6e5f6c6576656c3a value=Some(01000000)
	// value_encoded=0101000000 2025-06-19T07:45:58.473321Z DEBUG xcm::process: XCM execution
	// failed at instruction index=1 error=FailedToTransactAsset("Funds are unavailable")
	let loc = Location {
		parents: 1,
		interior: X2([
			Parachain(1001),
			AccountId32 {
				network: None,
				id: [
					150, 141, 187, 98, 102, 33, 87, 174, 108, 105, 38, 201, 33, 252, 99, 215, 105,
					11, 253, 230, 89, 13, 87, 138, 18, 41, 154, 220, 108, 179, 239, 229,
				],
			},
		]
		.into()),
	};

	assert_eq!(treasury_location_on_ah(), loc)
}
