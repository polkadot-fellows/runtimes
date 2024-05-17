# Changelog

Changelog for the runtimes governed by the Polkadot Fellowship.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Changed

- Kusama chains: allow arbitrary XCM execution ([polkadot-fellows/runtimes#261](https://github.com/polkadot-fellows/runtimes/pull/261))
- Allow everything through XCM SafeCallFilter ([polkadot-fellows/runtimes#285](https://github.com/polkadot-fellows/runtimes/pull/285))

### Added

- Add `pallet-vesting` to Asset Hubs ([polkadot-fellows/runtimes#269](https://github.com/polkadot-fellows/runtimes/pull/269))
- Add Pay Salary Collectives test ([polkadot-fellows/runtimes#260](https://github.com/polkadot-fellows/runtimes/pull/260))
- Add `pallet-xcm::transfer_assets_using_type_and_then()` for complex asset transfers ([polkadot-fellows/runtimes#311](https://github.com/polkadot-fellows/runtimes/pull/311))

### Removed

- Remove one-shot migrations from Kusama Coretime ([polkadot-fellows/runtimes#300](https://github.com/polkadot-fellows/runtimes/pull/300))
- Remove DMP queue and allow `system::authorize_upgrade` in XCM's call filter ([polkadot-fellows/runtimes#280](https://github.com/polkadot-fellows/runtimes/pull/280))
- Allow Sending XCM messages using a Signed origin on Kusama ([polkadot-fellows/runtimes#290](https://github.com/polkadot-fellows/runtimes/pull/290))

### Fixed

- Include patch to release stuck collator bonds ([polkadot-fellows/runtimes#289](https://github.com/polkadot-fellows/runtimes/pull/289))
- Safeguard pallet-balances against consumer ref underflow ([polkadot-fellows/runtimes#309](https://github.com/polkadot-fellows/runtimes/pull/309))

## [1.2.3] 29.04.2024

### Added

- Add migration to Kusama Coretime to onboard People Chain without long delay ([polkadot-fellows/runtimes#286](https://github.com/polkadot-fellows/runtimes/pull/286))

### Fixed

- Clean up outdated assignment in Kusama Coretime Chain state ([polkadot-fellows/runtimes#286](https://github.com/polkadot-fellows/runtimes/pull/286))

## [1.2.2] 20.04.2024

### Fixed

- Polkadot Bridge Hub: Unstuck bridge with Kusama ([polkadot-fellows/runtimes#277](https://github.com/polkadot-fellows/runtimes/pull/277)).
- Fix Kusama Coretime launch issues: import leases and fix renewals for short leases ([polkadot-fellows/runtimes#276](https://github.com/polkadot-fellows/runtimes/pull/276))

## [1.2.1] 09.04.2024

### Changed

- Modify runtimes for phase two of People Chain launch (Kusama) ([polkadot-fellows/runtimes#246](https://github.com/polkadot-fellows/runtimes/pull/246))

## [1.2.0] 28.03.2024

### Added

- Remove state-trie-migration pallet from kusama, add state trie migration to V1 on polkadot ([polkadot-fellows/runtimes#170](https://github.com/polkadot-fellows/runtimes/pull/170))
- Introduce chain spec generator ([polkadot-fellows/runtimes#127](https://github.com/polkadot-fellows/runtimes/pull/127))
- Add [Encointer](https://encointer.org) system parachain runtime, completing [RFC22](https://github.com/polkadot-fellows/RFCs/blob/main/text/
0022-adopt-encointer-runtime.md) ([polkadot-fellows/runtimes#80](https://github.com/polkadot-fellows/runtimes/pull/80))
- Feature for enabling debug prints in the Polkadot and Kusama runtime ([polkadot-fellows/runtimes#85](https://github.com/polkadot-fellows/runtimes/pull/85))
- Added new "Wish for Change" track ([polkadot-fellows/runtimes#184](https://github.com/polkadot-fellows/runtimes/pull/184))
- Enable Coretime and on-demand on Kusama ([polkadot-fellows/runtimes#159](https://github.com/polkadot-fellows/runtimes/pull/159))
- Refund any leases that are not migrated to Coretime (have holes in them/have not yet started) ([polkadot-fellows/runtimes#206](https://github.com/polkadot-fellows/runtimes/pull/206))
- Enable Elastic Scaling node side feature for Kusama ([polkadot-fellows/runtimes#205](https://github.com/polkadot-fellows/runtimes/pull/205))
- Cancel Parachain Auctions ([polkadot-fellows/runtimes#215](https://github.com/polkadot-fellows/runtimes/pull/215))
- Upgrade encointer protocol to 6.1.0 ([polkadot-fellows/runtimes#236](https://github.com/polkadot-fellows/runtimes/pull/236))
- Update NFT deposits according to RFC-45 ([polkadot-fellows/runtimes#237](https://github.com/polkadot-fellows/runtimes/pull/237))
- Add Kusama People Chain ([polkadot-fellows/runtimes#217](https://github.com/polkadot-fellows/runtimes/pull/217))
- Asset Conversion setup for Polkadot Asset Hub, and XCM Swap Weight Trader for both Asset Hubs ([polkadot-fellows/runtimes#218](https://github.com/polkadot-fellows/runtimes/pull/218))
- Adds Snowbridge to Kusama and Polkadot ([polkadot-fellows/runtimes#130](https://github.com/polkadot-fellows/runtimes/pull/130))
- Add the Kusama Coretime Chain ([polkadot-fellows/runtimes#212](https://github.com/polkadot-fellows/runtimes/pull/212))

### Changed

- Upgrade parachains runtime API from v7 to v8 in Kusama ([context](https://paritytech.github.io/polkadot-sdk/book/protocol-validator-disabling.html), [polkadot-fellows/runtimes#148](https://github.com/polkadot-fellows/runtimes/pull/148)).
- Fixed the lowering of Asset Hub existential deposits.
- MMR leaves generated by `pallet_mmr` point to the next-authority-set of the current block instead of the prior block [polkadot-fellows/runtimes#169](https://github.com/polkadot-fellows/runtimes/pull/169)
- Deprecate the `xcm::body::TREASURER_INDEX` constant and use the standard `Treasury` variant from the `xcm::BodyId` type instead ([polkadot-fellows/runtimes#149](https://github.com/polkadot-fellows/runtimes/pull/149))
- Bump parachains runtime API to v9 in Kusama to enable the `node_features` function [polkadot-fellows/runtimes#194](https://github.com/polkadot-fellows/runtimes/pull/194)
- Bump parachains runtime API to v10 in Kusama to enable the `approval-voting-params` function [polkadot-fellows/runtimes#204](https://github.com/polkadot-fellows/runtimes/pull/204)
- Use Relay Chain's Treasury Pallet account as a destination for XCM fees on System Parachain ([polkadot-fellows/runtimes#191](https://github.com/polkadot-fellows/runtimes/pull/191))
- Bump parachains runtime API to v10 in Polkadot to enable async-backing subsystems(still in backwards compatible mode) [polkadot-fellows/runtimes#222](https://github.com/polkadot-fellows/runtimes/pull/222)
- Prepared system parachain runtimes for async backing enabling ([polkadot-fellows/runtimes#228](https://github.com/polkadot-fellows/runtimes/pull/228))
- Update runtime weights [polkadot-fellows/runtimes#223](https://github.com/polkadot-fellows/runtimes/pull/223)
- Treasury Spend detects relative locations of the native asset ([polkadot-fellows/runtimes#233](https://github.com/polkadot-fellows/runtimes/pull/233))
- Increase consumer reference limits for Asset Hubs ([polkadot-fellows/runtimes#258](https://github.com/polkadot-fellows/runtimes/pull/258))
- Updated Asset Hub asset class creation deposit to use `system_para_deposit()` ([polkadot-fellows/runtimes#259](https://github.com/polkadot-fellows/runtimes/pull/259))

### Removed

- Removed the `SafeCallFilter` from the Relay Chain XCM config ([polkadot-fellows/runtimes#172](https://github.com/polkadot-fellows/runtimes/pull/172)).
- Removed the `ImOnline` pallet ([polkadot-fellows/runtimes#178](https://github.com/polkadot-fellows/runtimes/pull/178))

### Fixed

- Fixed the cost of a single byte, sent over bridge to use the `TransactionByteFee` constant of the bridged chain [polkadot-fellows/runtimes#174](https://github.com/polkadot-fellows/runtimes/pull/174).

### Based on Polkadot-SDK

- Upgrade dependencies to the [polkadot-sdk@1.5.0](https://github.com/paritytech/polkadot-sdk/releases/tag/polkadot-v1.5.0) release ([polkadot-fellows/runtimes#137](https://github.com/polkadot-fellows/runtimes/pull/137))
- Upgrade dependencies to the [polkadot-sdk@1.6.0](https://github.com/paritytech/polkadot-sdk/releases/tag/polkadot-v1.6.0) release ([polkadot-fellows/runtimes#159](https://github.com/polkadot-fellows/runtimes/pull/159))
- Upgrade dependencies to the [polkadot-sdk@1.7.0](https://github.com/paritytech/polkadot-sdk/releases/tag/polkadot-v1.7.0) release ([polkadot-fellows/runtimes#187](https://github.com/polkadot-fellows/runtimes/pull/187))

## [1.1.1] 25.01.2024

### Fixed

- Fixed the lowering of Asset Hub existential deposits ([polkadot-fellows/runtimes#158](https://github.com/polkadot-fellows/runtimes/pull/158)).

## [1.1.0] 10.01.2024

### Changed

- Upgrade parachains runtime API from v5 to v7 in Polkadot and Kusama ([polkadot-fellows/runtimes#56](https://github.com/polkadot-fellows/runtimes/pull/56))
- Upgrade Preimage pallet's config implementations to adapt the new `Consideration` API ([polkadot-fellows/runtimes#56](https://github.com/polkadot-fellows/runtimes/pull/56))
- Remove `experimental` feature flag for `pallet-society`, `pallet-xcm`, and `runtime-common` crates imports ([polkadot-fellows/runtimes#56](https://github.com/polkadot-fellows/runtimes/pull/56))
- Election provider: use a geometric deposit base calculation for EPM signed submissions in Polkadot and Kusama ([polkadot-fellows/runtimes#56](https://github.com/polkadot-fellows/runtimes/pull/56))
- Make `IdentityInfo` generic in `pallet-identity` ([polkadot-fellows/runtimes#87](https://github.com/polkadot-fellows/runtimes/pull/87)). Context: https://github.com/paritytech/polkadot-sdk/pull/1661
- Whitelist `force_default_xcm_version` in XCM call filter ([polkadot-fellows/runtimes#45](https://github.com/polkadot-fellows/runtimes/pull/45))
- Update the fellowship salary budget amount in alignment with the Fellowship Salary [RFC](https://github.com/polkadot-fellows/RFCs/pull/50) ([polkadot-fellows/runtimes#121](https://github.com/polkadot-fellows/runtimes/pull/121))
- Set up an account ID for the local root location on Polkadot Collectives ([polkadot-fellows/runtimes#125](https://github.com/polkadot-fellows/runtimes/pull/125))
- Increase confirmation period for treasury spend tracks on Polkadot & Kusama ([polkadot-fellows/runtimes#119](https://github.com/polkadot-fellows/runtimes/pull/119))

### Added

- Enable async backing on Kusama ([polkadot-fellows/runtimes#87](https://github.com/polkadot-fellows/runtimes/pull/87)). Context: https://github.com/paritytech/polkadot-sdk/pull/1543
- Implemented GenesisBuilder API for all runtimes ([polkadot-fellows/runtimes#87](https://github.com/polkadot-fellows/runtimes/pull/87)). Context: https://github.com/paritytech/polkadot-sdk/pull/1492
- XCM transport fees are now exponential and are sent to a treasury account ([polkadot-fellows/runtimes#87](https://github.com/polkadot-fellows/runtimes/pull/87)). Context: https://github.com/paritytech/polkadot-sdk/pull/1234
- System parachains are now trusted teleporters of each other ([polkadot-fellows/runtimes#87](https://github.com/polkadot-fellows/runtimes/pull/87)). Context: https://github.com/paritytech/polkadot-sdk/pull/1368
- Treasury is able to spend various asset kinds ([polkadot-fellows/runtimes#87](https://github.com/polkadot-fellows/runtimes/pull/87))
- Add BEEFY to Polkadot ([polkadot-fellows/runtimes#65](https://github.com/polkadot-fellows/runtimes/pull/65))
- Fellowship Treasury pallet on Polkadot Collectives ([polkadot-fellows/runtimes#109](https://github.com/polkadot-fellows/runtimes/pull/109))
- Added Polkadot <> Kusama bridge to support asset transfers between Asset Hubs ([polkadot-fellows/runtimes#108](https://github.com/polkadot-fellows/runtimes/pull/108))

### Fixed

- Add missing weight functions for `runtime_parachains_hrmp` and `preimage` pallets ([polkadot-fellows/runtimes#56](https://github.com/polkadot-fellows/runtimes/pull/56))
- Fix for Reward Deficit in the pool ([polkadot-fellows/runtimes#87](https://github.com/polkadot-fellows/runtimes/pull/87)). Context: https://github.com/paritytech/polkadot-sdk/pull/1255

## [1.0.1] 14.11.2023

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
