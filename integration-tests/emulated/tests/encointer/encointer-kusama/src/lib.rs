#[cfg(test)]
mod tests;

// Substrate
pub use frame_support::{
	assert_err, assert_ok,
	dispatch::RawOrigin,
	pallet_prelude::Weight,
	sp_runtime::{AccountId32, DispatchError, DispatchResult},
	traits::fungibles::Inspect,
};
pub use sp_runtime::traits::Dispatchable;
pub use emulated_integration_tests_common::{
	xcm_emulator::{
		assert_expected_events, bx, helpers::weight_within_threshold, Chain, Parachain as Para,
		RelayChain as Relay, Test, TestArgs, TestContext, TestExt,
	},
	PROOF_SIZE_THRESHOLD, REF_TIME_THRESHOLD, RESERVABLE_ASSET_ID, XCM_V5,
};
pub use integration_tests_helpers::{
	test_parachain_is_trusted_teleporter_for_relay, test_relay_is_trusted_teleporter,
};
pub use kusama_system_emulated_network::{
	AssetHubKusamaPara as AssetHubKusama, EncointerKusamaPara as EncointerKusama,
};
pub use kusama_system_emulated_network::{
	asset_hub_kusama_emulated_chain::{
		genesis::{AssetHubKusamaAssetOwner, ED as ASSET_HUB_KUSAMA_ED},
		AssetHubKusamaParaPallet as AssetHubKusamaPallet,
	},
	encointer_kusama_emulated_chain::{
		EncointerKusamaParaPallet as EncointerKusamaPallet,
	},
	kusama_emulated_chain::{genesis::ED as KUSAMA_ED, KusamaRelayPallet as KusamaPallet},
	penpal_emulated_chain::{
		CustomizableAssetFromSystemAssetHub, PenpalAParaPallet as PenpalAPallet, PenpalAssetOwner,
		PenpalBParaPallet as PenpalBPallet, ED as PENPAL_ED,
	}, AssetHubKusamaParaReceiver as AssetHubKusamaReceiver,
	AssetHubKusamaParaSender as AssetHubKusamaSender, BridgeHubKusamaPara as BridgeHubKusama,
	BridgeHubKusamaParaReceiver as BridgeHubKusamaReceiver, KusamaRelay as Kusama,
	KusamaRelayReceiver as KusamaReceiver, KusamaRelaySender as KusamaSender,
	PenpalAPara as PenpalA, PenpalAParaReceiver as PenpalAReceiver,
	PenpalAParaSender as PenpalASender, PenpalBPara as PenpalB,
	PenpalBParaReceiver as PenpalBReceiver,
};
pub use xcm::{
	prelude::{AccountId32 as AccountId32Junction, *},
	v5::{self, Error, NetworkId::Kusama as KusamaId},
};
pub use parachains_common::{AccountId, Balance};

pub use asset_test_utils::xcm_helpers;

pub type SystemParaToRelayTest = Test<EncointerKusama, Kusama>;
pub type ParaToSystemParaTest = Test<EncointerKusama, AssetHubKusama>;
