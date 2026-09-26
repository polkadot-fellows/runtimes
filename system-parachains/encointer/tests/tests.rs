use codec::Encode;
use encointer_kusama_runtime::{
	xcm_config::{AssetHubLocation, RelayChainLocation},
	Runtime, RuntimeOrigin, SessionKeys,
};
use frame_support::{assert_err, assert_ok};
use parachains_runtimes_test_utils::GovernanceOrigin;
use sp_runtime::Either;
use xcm::prelude::*;

#[test]
fn governance_authorize_upgrade_works() {
	// no - random para
	assert_err!(
		parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::Location(Location::new(1, Parachain(12334)))),
		Either::Right(InstructionError { index: 0, error: XcmError::Barrier })
	);
	// no - random system para
	assert_err!(
		parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::Location(Location::new(1, Parachain(1765)))),
		Either::Right(InstructionError { index: 0, error: XcmError::Barrier })
	);

	// ok - relaychain
	assert_ok!(parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
		Runtime,
		RuntimeOrigin,
	>(GovernanceOrigin::Location(RelayChainLocation::get())));

	// ok - AssetHub
	assert_ok!(parachains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
		Runtime,
		RuntimeOrigin,
	>(GovernanceOrigin::Location(AssetHubLocation::get())));
}

#[test]
fn session_key_deposit_works() {
	system_parachains_common::test_helpers::session_key_deposit_works::<Runtime>(|owner| {
		let generated = SessionKeys::generate(&owner.encode(), None);
		(generated.keys, generated.proof.encode())
	});
}
