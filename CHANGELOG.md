# Changelog

Changelog for the runtimes governed by the Polkadot Fellowship.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Changed

- Upgrade parachains runtime API from v5 to v7 in Polkadot and Kusama ([polkadot-fellows/runtimes#56](https://github.com/polkadot-fellows/runtimes/pull/56))
- Upgrade Preimage pallet's config implementations to adapt the new `Consideration` API ([polkadot-fellows/runtimes#56](https://github.com/polkadot-fellows/runtimes/pull/56))
- Remove `experimental` feature flag for `pallet-society`, `pallet-xcm`, and `runtime-common` crates imports ([polkadot-fellows/runtimes#56](https://github.com/polkadot-fellows/runtimes/pull/56))
- Election provider: use a geometric deposit base calculation for EPM signed submissions in Polkadot and Kusama ([polkadot-fellows/runtimes#56](https://github.com/polkadot-fellows/runtimes/pull/56))
- Make `IdentityInfo` generic in `pallet-identity` ([polkadot-fellows/runtimes#87](https://github.com/polkadot-fellows/runtimes/pull/87)). Context: https://github.com/paritytech/polkadot-sdk/pull/1661

### Added

- Enable async backing on Kusama ([polkadot-fellows/runtimes#87](https://github.com/polkadot-fellows/runtimes/pull/87)). Context: https://github.com/paritytech/polkadot-sdk/pull/1543
- Implemented GenesisBuilder API for all runtimes ([polkadot-fellows/runtimes#87](https://github.com/polkadot-fellows/runtimes/pull/87)). Context: https://github.com/paritytech/polkadot-sdk/pull/1492
- XCM transport fees are now exponential and are sent to a treasury account ([polkadot-fellows/runtimes#87](https://github.com/polkadot-fellows/runtimes/pull/87)). Context: https://github.com/paritytech/polkadot-sdk/pull/1234
- System parachains are now trusted teleporters of each other ([polkadot-fellows/runtimes#87](https://github.com/polkadot-fellows/runtimes/pull/87)). Context: https://github.com/paritytech/polkadot-sdk/pull/1368
- Treasury is able to spend various asset kinds ([polkadot-fellows/runtimes#87](https://github.com/polkadot-fellows/runtimes/pull/87))

### Fixed

- Add missing weight functions for `runtime_parachains_hrmp` and `preimage` pallets ([polkadot-fellows/runtimes#56](https://github.com/polkadot-fellows/runtimes/pull/56))
- Fix for Reward Deficit in the pool ([polkadot-fellows/runtimes#87](https://github.com/polkadot-fellows/runtimes/pull/87)). Context: https://github.com/paritytech/polkadot-sdk/pull/1255

## [1.0.1] 22.10.2023

### Changed

- Restore governance lock periods to 7 days in Polkadot ([polkadot-fellows/runtimes#86](https://github.com/polkadot-fellows/runtimes/pull/86))

## [1.0.0] 22.10.2023

### Changed

- Update Polkadot ideal staking rate ([polkadot-fellows/runtimes#26](https://github.com/polkadot-fellows/runtimes/pull/26))
- Treasury deprecate `propose_spend` dispatchable ([paritytech/substrate#14538](https://github.com/paritytech/substrate/pull/14538))
- Use benchmarked weights for `XCM` ([paritytech/polkadot#7077](https://github.com/paritytech/polkadot/pull/7077))
- Put HRMP Channel Management on General Admin Track ([paritytech/polkadot#7477](https://github.com/paritytech/polkadot/pull/7477))
- Improve locking mechanism for parachains ([paritytech/polkadot-sdk#1290](https://github.com/paritytech/polkadot-sdk/pull/1290))
- Allow Root to initiate auctions ([paritytech/polkadot#7449](https://github.com/paritytech/polkadot/pull/7449))
- Remark: Allow any kind of origin ([paritytech/substrate#14260](https://github.com/paritytech/substrate/pull/14260))
- Im-Online: Remove network state from heartbeats ([paritytech/substrate#14251](https://github.com/paritytech/substrate/pull/14251))
- Nomination pools: disallow setting above global max commission ([paritytech/substrate#14496](https://github.com/paritytech/substrate/pull/14496))
- Rename Statemint/Statemine to Asset Hub ([paritytech/cumulus#2633](https://github.com/paritytech/cumulus/pull/2633))
- Fellowship: Voters can initiate proposals on their votable tracks ([paritytech/cumulus#2725](https://github.com/paritytech/cumulus/pull/2725))
- Root can promote on Polkadot Collectives ([paritytech/cumulus#2781](https://github.com/paritytech/cumulus/pull/2781))
- Add New Assets Privileged Functions to Appropriate Proxy Types ([paritytech/cumulus#2839](https://github.com/paritytech/cumulus/pull/2839))
- Better Handling of Candidates Who Become Invulnerable ([paritytech/cumulus#2801](https://github.com/paritytech/cumulus/pull/2801))

### Added

- Implement dynamic number of nominators ([paritytech/substrate#12970](https://github.com/paritytech/substrate/pull/12970) & [paritytech/polkadot#6807](https://github.com/paritytech/polkadot/pull/6807))
- Upgrade Kusama to Society V2 ([paritytech/polkadot#7356](https://github.com/paritytech/polkadot/pull/7356))
- Kusama state version switch and migration ([paritytech/polkadot#7015](https://github.com/paritytech/polkadot/pull/7015))
- Add Nomination Pools and Voters List to Staking Proxy ([paritytech/polkadot#7448](https://github.com/paritytech/polkadot/pull/7448))
- Add minting price to the pre-signed mint object ([paritytech/substrate#14242](https://github.com/paritytech/substrate/pull/14242))
- Add mint price to the witness object on mint and confirm it ([paritytech/substrate#14257](https://github.com/paritytech/substrate/pull/14257))
- Stabilize Metadata V15 ([paritytech/substrate#14481](https://github.com/paritytech/substrate/pull/14481))
- Add Ability to Add/Remove Invulnerable Collators ([paritytech/cumulus#2596](https://github.com/paritytech/cumulus/pull/2596))
- Polkadot Fellowship promotion/demotion periods, members activity and salaries ([paritytech/cumulus#2607](https://github.com/paritytech/cumulus/pull/2607))
- Add asset conversion to asset hub Kusama ([paritytech/cumulus#2935](https://github.com/paritytech/cumulus/pull/2935))

### Fixed

- Unlock/unreserve Gov v1 balances and remove kvs ([paritytech/polkadot#7314](https://github.com/paritytech/polkadot/pull/7314))
- Polkadot 28 days as conviction voting period ([paritytech/polkadot#7595](https://github.com/paritytech/polkadot/pull/7595))
- XCM: Fix issue with RequestUnlock ([paritytech/polkadot#7278](https://github.com/paritytech/polkadot/pull/7278))
- Clear Existing HRMP Channel Request When Force Opening ([paritytech/polkadot#7389](https://github.com/paritytech/polkadot/pull/7389))
- Prune upgrade cooldowns ([paritytech/polkadot#7470](https://github.com/paritytech/polkadot/pull/7470))
- Assets `destroy_accounts` releases the deposit
  ([paritytech/substrate#14443](https://github.com/paritytech/substrate/pull/14443))
- Update Polkadot Collectives to use `limited_teleport_assets` for automatic slash handling, as
  `teleport_assets` is deprecated and caused a failing integration test. ([polkadot-fellows/runtimes#46](https://github.com/polkadot-fellows/runtimes/pull/46))
