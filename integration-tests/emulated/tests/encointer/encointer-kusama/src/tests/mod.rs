use crate::*;
use encointer_kusama_runtime::{
	treasuries_xcm_payout::{ConstantKsmFee, GetRemoteFee},
	AccountId,
};
use polkadot_runtime_common::impls::VersionedLocatableAsset;
use xcm_runtime_apis::fees::runtime_decl_for_xcm_payment_api::XcmPaymentApiV1;

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
		ConstantKsmFee::get_remote_fee(Xcm::new(), None),
		(Location::parent(), execution_fees).into()
	);
}
