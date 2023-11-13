# Changelog

Changelog for the runtimes governed by the Polkadot Fellowship.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

###########################################

### Node & Polkadot

TODO for bridging: port https://github.com/paritytech/polkadot-sdk/pull/2023/files

- [x] (NODE) runtime-api: cleanup after v7 stabilization (#1729 @ordian) [Node]
- [x] (NODE) PVF: more filesystem sandboxing (#1373 @mrcnski) [Node]
- [x] (ROCOCO) rococo-runtime: RococoGenesisExt removed (#1490 michalkucharczyk) [Node]
- [x] (NODE) ix subkey inspect output text padding (#1744 @btwiuse) [Node]
- [x] (NODE) Use Extensions to register offchain worker custom extensions (#1719 @skunert) [Node]
- [x] (NODE) Remove kusama and polkadot runtime crates (#1731 @bkchr) [Node, Polkadot]
- [x] (NODE) PVF: Add back socket path parameter, use tmp socket path (#1780 @mrcnski) [Node]
- [x] (NODE) Delete full db directory with purge-chain subcommand (#1786 @skunert) [Node]
- [x] (NODE) Mixnet integration (#1346 @zdave-parity) [Node]
- [x] (NODE) Update testnet bootnode dns name (#1712 @BulatSaif) [Node]
- [x] (NODE) remote-ext: fix state download stall on slow connections and reduce memory usage (#1295 @liamaharon) [Node]
- [x] (NODE) PVF worker: bump landlock, update ABI docs (#1850 @mrcnski) [Node]
- [x] (NODE) sc-consensus-beefy: improve gossip logic (#1852 @acatangiu) [Node]
- [x] (NODE) Include polkadot version in artifact path (#1828 @eagr) [Node]
- [x] (NODE) Paired-key Crypto Scheme (#1705 @drskalman) [Node]
- [x] (POLKADOT) fix: GoAhead signal only set when runtime upgrade is enacted from parachain side (#1176 @Daanvdplas) [Polkadot]
- [x] (NODE) Arkworks Elliptic Curve utils overhaul (#1870 @davxy) [Node]
- [x] (ROCOCO) Adding migrations to clean Rococo Gov 1 storage & reserved funds (#1849 @al3mart) [Polkadot]
- [x] (NODE) Update the alerts to use a new metric substrate_unbounded_channel_size (#1568 @BulatSaif) [Node]
- [x] (NODE) sc-consensus-beefy: fix initialization when state is unavailable (#1888 @acatangiu ) [Node]
- [x] (NODE) Start BEEFY client by default for Polkadot nodes (#1913 @serban300) [Node]
- [x] (NODE) Do not force collators to update after enabling async backing (#1920 @bkchr) [Node]
- [x] (NODE) sc-executor: Increase maximum instance count (#1856 @bkchr) [Node]
- [x] (ROCOCO) Re-enable Identity on Westend and Rococo (#1901 @joepetrowski) [Polkadot]
- [x] (NODE) polkadot: eradicate LeafStatus (#1565 @ordian) [Node, Polakdot]
- [x] (NODE) polkadot: enable tikv-jemallocator/unprefixed_malloc_on_supported_platforms (#2002 @andresilva) [Polkadot]
- [x] (NODE) PVF: Add worker check during tests and benches (#1771 @mrcnski) [Node]
- [x] (NODE) Application Crypto and BEEFY Support for paired (ECDSA,BLS) crypto (#1815 @drskalman) [Node]
- [x] (NODE) basic-authorship: Improve time recording and logging (#2010 @bkchr) [Node]


### Frame & Pallets

- [x] (NO MIGRATION REQUIRED) **Breaking Change** Ensure correct variant count in Runtime[Hold/Freeze]Reason (#1900 @kianenigma) [Frame]
- [x] (PALLET IS NOT USED) **Breaking Change** Add MaxTipAmount for pallet-tips (#1709 @AurevoirXavier) [Frame] 
- [x] (INTERNAL) Associated type Hasher for QueryPreimage, StorePreimage and Bounded (#1720 @muraca) [Frame]
- [x] (INTERNAL) Add custom error message for StorageNoopGuard (#1727 @seadanda) [Frame]
- [x] (INTERNAL) Add event field names to HRMP Event variants (#1695 @seadanda) [Pallets]
- [x] (INTERNAL) add some events for pallet-bounties (#1706 @xlc) [Pallets]
- [x] **ADDED MIGRATION** [NPoS] Fix for Reward Deficit in the pool (#1255 @Ank4n) [Pallets]
- [x] (INTERNAL) frame-support: RuntimeDebug\Eq\PartialEq impls for Imbalance (#1717 @muharem) [Frame]
- [x] **ADDED MIGRATION** Tvl pool staking (#1322 @PieWol) [Frame]
- [X] **Could be added later (ping author)** Init System Parachain storage versions and add migration check jobs to CI (#1344 @liamaharon) [Frame]
- [x] (INTERNAL) expose the last relay chain block number as an API from parachain-system (#1761 @rphmeier) [Pallets]
- [x] (INTERNAL) feat: compute pallet/storage prefix hash at compile time (#1539 @yjhmelody) [Frame]
- [ ] **Could be added later (ping author)** Treasury spends various asset kinds (#1333 @muharem) [Frame]
- [x] (INTERNAL) Make CheckNonce refuse transactions signed by accounts with no providers (#1578 @zdave-parity) [Frame]
- [x] (INTERNAL) Warn on unchecked weight witness (#1818 @ggwpez) [Frame]
- [x] (INTERNAL) frame: use derive-impl for beefy and mmr pallets (#1867 @acatangiu) [Pallets]
- [x] (INTERNAL) Macros to use path instead of ident (#1474 @juangirini) [Frame]
- [X] **descr saids that no migration required** Refactor staking ledger (#1484 @gpestana) [Frame, Pallets]
- [x] (INTERNAL) extract amount method for fungible/s Imbalance (#1847 @muharem) [Frame]
- [x] (INTERNAL) Allow Locks/Holds/Reserves/Freezes by default when using pallet_balances TestDefaultConfig (#1880 @liamaharon) [Frame, Pallets]
- [x] (INTERNAL) nit: use traits::tokens::fungible => use traits::fungible (#1753 @gilescope) [Pallets]
- [ ] **TODO: fix of migration that has happened or not?** Fix para-scheduler migration on Rococo (#1921 @ggwpez) [Pallets]
- [x] (INTERNAL) Trading trait and deal with metadata in Mutate trait for nonfungibles_v2 (#1561 @AlexD10S) [Pallets]
- [x] (INTERNAL) Message Queue use proper overweight limit (#1873 @ggwpez) [Frame]
- [ ] **TODO: fix of migration that has happened or not?** paras-scheduler: Fix migration to V1 (#1969 @bkchr) [Pallets]
- [x] (INTERNAL) Resolve Credit to Account impls of OnUnbalanced trait (#1876 @muharem) [Frame]
- [x] (INTERNAL) CheckWeight: Add more logging (#1996 @bkchr) [Frame]
- [x] (NO MIGRATION REQUIRED) Make IdentityInfo generic in pallet-identity (#1661 @georgepisaltu) [Pallets]
- [x] (INTERNAL) Small optimisation to --profile dev wasm builds (#1851 @liamaharon) [Frame]


### Tests, Benchmarks & Documentation

- [X] Point documentation links to monorepo (#1741 @skunert) [Documentation]
- [X] Revive Substrate Crate (#1477 @ggwpez) [Documentation]
- [X] Adding try_state hook for Treasury pallet (#1820 @wentelteefje) [Tests]
- [X] Fix links to implementers' guide (#1865 @antonva) [Documentation]
- [X] frame: use derive-impl for beefy and mmr pallets (#1867 @acatangiu) [Tests]
- [X] Remove clippy clone-double-ref lint noise (#1860 @seadanda) [Tests]
- [X] Publish xcm-emulator crate (#1881 @NachoPal) [Tests]
- [X] bridges: add missing crate descriptions (#1919 @acatangiu) [Documentation]
- [X] Publish penpal-runtime crate (#1904 @NachoPal) [Tests]
- [X] Use prebuilt try-runtime binary in CI (#1898 @liamaharon) [Tests]
- [X] Start BEEFY gadget by default for Polkadot nodes (#1945 @serban300) [Documentation]
- [X] Refactor candidates test in paras_inherent (#2004 @tdimitrov) [Tests]


### XCM, Bridges & Misc

- [x] (RPC) archive: Implement height, hashByHeight and call (#1582 lexnv) [RPC API]
- [X] (INTERNAL) Enable mocking contracts (#1331 @pmikolajczyk41) [Smart Contracts]
- [ ] **TODO: Updated formulae, but still need to run benchmarks** Use Weight::MAX for reserve_asset_deposited, receive_teleported_asset benchmarks (#1726 @bkontur) [XCM]
- [X] (INTERNAL) allow treasury to do reserve asset transfers (#1447 @samelamin) [XCM]
- [ ] **TODO: We do not need v8, right?** Disabled validators runtime API (#1257 @tdimitrov) [Runtime API]
- [X] (INTERNAL) Small enhancements for NetworkExportTable and xcm-builder (#1848 @bkontur) [XCM]
- [X] (INTERNAL) increase MAX_ASSETS_FOR_BUY_EXECUTION (#1733 @xlc) [XCM]
- [X] (NO MIGRATION REQUIRED) Introduce XcmFeesToAccount fee manager (#1234 @KiChjang) [XCM]
- [X] (INTERNAL) Update bridges subtree (#1944 @bkontur) [Bridges]
- [X] (INTERNAL) XCM MultiAssets: sort after reanchoring (#2129 @serban300) [XCM]
- [X] **for Snowfork guys - no need for us** Direct XCM ExportMessage fees for different bridges to different receiver accounts (#2021 @serban300) [Bridges]


### Parachains & Cumulus

- [X] (INTERNAL) Add event field names to HRMP Event variants (#1695 @seadanda) [System-Parachains]
- [ ] **TODO: do the same for fellowship SP?** Init System Parachain storage versions and add migration check jobs to CI (#1344 @liamaharon) [System-Parachains]
- [X] (INTERNAL) [xcm-emulator] Decouple the AccountId type from AccountId32 (#1458 @NachoPal) [System-Parachains]
- [X] (NODE) Fix Asset Hub collator crashing when starting from genesis (#1788 @georgepisaltu) [Cumulus]
- [ ] **TODO: Updated formulae, but still need to run benchmarks** Use Weight::MAX for reserve_asset_deposited, receive_teleported_asset benchmarks (#1726 @bkontur) [System-Parachains]
- [X] (INTERNAL) Xcm emulator nits (#1649 @bkontur) [Cumulus, System-Parachains]
- [ ] **changed, but shall we make relay trust to BH** Make System Parachains trusted Teleporters (#1368 @NachoPal) [System-Parachains]
- [X] (ROCOCO) cumulus: add asset-hub-rococo runtime based on asset-hub-kusama and add asset-bridging support to it #(1215 @acatangiu) [Cumulus]
- [X] (NODE) Cumulus: Allow aura to use initialized collation request receiver (#1911 @skunert) [Cumulus]
- [X] (NODE) Expose prometheus metrics for minimal-relay-chain node in collators (#1942 @skunert) [Cumulus]
- [X] (ROCOCO) [testnet] AssetHubRococo nits (#1954 @bkontur) [Cumulus]
- [X] (ROCOCO) Remove (rococo/westend)-runtime deps from testnet AssetHubs (#1979 @bkontur) [Cumulus]
- [X] (ROCOCO) [testnet] BridgeHubRococo nits (#1972 @bkontur) [Cumulus]
- [X] (TESTS) Removed TODO from test-case for hard-coded delivery fee estimation (#2042  @bkontur) [Cumulus]
- [X] (ROCOCO) [testnet] Align testnet system parachain runtimes using RelayTreasuryLocation and SystemParachains in the same way (#2023 @bkontur) [Cumulus]
- [X] (ROCOCO) [testnet] Add AssetHubRococo <-> AssetHubWestend asset bridging support (#1967 @bkontur) [Cumulus]
###########################################


## [1.0.3] XX.XX.XXXX

### Changed

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
