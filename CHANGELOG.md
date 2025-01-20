# Changelog

Changelog for the runtimes governed by the Polkadot Fellowship.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Fixed

- Fix missing Encointer democracy pallet hook needed for enactment ([polkadot-fellows/runtimes/pull/508](https://github.com/polkadot-fellows/runtimes/pull/508))
- Improve benchmark configuration: fix storage whitelist in benchmarks ([polkadot-fellows/runtimes/pull/525](https://github.com/polkadot-fellows/runtimes/pull/525))

### Fixed

- Disallow `add_sub` and `set_subs` from `NonTransfer` proxy type in people chain runtimes ([polkadot-fellows/runtimes#518](https://github.com/polkadot-fellows/runtimes/pull/518))

### Added

- Location conversion tests for relays and parachains ([polkadot-fellows/runtimes#487](https://github.com/polkadot-fellows/runtimes/pull/487))

- ParaRegistration proxy for Polkadot and Kusama ([polkadot-fellows/runtimes#520](https://github.com/polkadot-fellows/runtimes/pull/520))
- Encointer: Swap community currency for KSM from community treasuries subject to democratic decision on allowance ([polkadot-fellows/runtimes#541](https://github.com/polkadot-fellows/runtimes/pull/541))

- Delegate stake pools in Kusama ([polkadot-fellows/runtimes#540](https://github.com/polkadot-fellows/runtimes/pull/540))

### Changed

- Kusama Treasury: remove funding to the Kappa Sigma Mu Society and disable burn ([polkadot-fellows/runtimes#507](https://github.com/polkadot-fellows/runtimes/pull/507))
- Kusama Treasury: allow burn parameters to be set via OpenGov ([polkadot-fellows/runtimes#511](https://github.com/polkadot-fellows/runtimes/pull/511))
- Remove Snowbridge create agent and channel extrinsics. ([polkadot-fellows/runtimes#506](https://github.com/polkadot-fellows/runtimes/pull/506))

#### From [#490](https://github.com/polkadot-fellows/runtimes/pull/490)

- Transfer Polkadot-native assets to Ethereum ([SDK `stable2409` #5710](https://github.com/paritytech/polkadot-sdk/pull/5710), [SDK #5546](https://github.com/paritytech/polkadot-sdk/pull/5546))
- Add possibility to inject non-authorities session-keys in genesis ([SDK `stable2409` #5078](https://github.com/paritytech/polkadot-sdk/pull/5078))
- \[bridges-v2\] Permissionless lanes ([SDK `stable2409` #4949](https://github.com/paritytech/polkadot-sdk/pull/4949))
- \[Assets\] Call implementation for `transfer_all` ([SDK `stable2409` #4527](https://github.com/paritytech/polkadot-sdk/pull/4527))
- Tx Payment: drop ED requirements for tx payments with exchangeable asset ([SDK `stable2409` #4488](https://github.com/paritytech/polkadot-sdk/pull/4488))
- Coretime auto-renew ([SDK `stable2409` #4424](https://github.com/paritytech/polkadot-sdk/pull/4424))
- Initialises pallet-delegated-staking ([SDK `v1.12.0` #3904](https://github.com/paritytech/polkadot-sdk/pull/3904))

### Changed

#### From [#490](https://github.com/polkadot-fellows/runtimes/pull/490)

- Polkadot Primitives v8 ([SDK v1.16 #5525](https://github.com/paritytech/polkadot-sdk/pull/5525)).
- Relax `XcmFeeToAccount` trait bound on `AccountId` ([SDK v1.16 #4959](https://github.com/paritytech/polkadot-sdk/pull/4959))
- Bridges V2 refactoring backport and `pallet_bridge_messages` simplifications ([SDK `stable2407` #4935](https://github.com/paritytech/polkadot-sdk/pull/4935))
- Renamed `assigner_on_demand` to `on_demand` ([SDK `stable2409` #4706](https://github.com/paritytech/polkadot-sdk/pull/4706)).
- \[BEEFY\] Add runtime support for reporting fork voting ([SDK `stable2407` #4522](https://github.com/paritytech/polkadot-sdk/pull/4522)).
- Migrates Nomination Pool to use delegated staking: i.e. allowing delegated funds to be held in member's own account
  instead of the pool account. This would enable pool member funds to be used for voting in opengov.
  ([SDK `v1.13.0` #3905](https://github.com/paritytech/polkadot-sdk/pull/3905))

## [1.3.4] 01.11.2024

### Changed

- Change Polkadot inflation to 120M DOT per year ([polkadot-fellows/runtimes#471](https://github.com/polkadot-fellows/runtimes/pull/471))
- Update foreign asset ids in Asset Hub Polkadot and Asset Hub Kusama from v3 to v4 locations ([polkadot-fellows/runtimes#472](https://github.com/polkadot-fellows/runtimes/pull/472))
- Lower Parachain and Data Deposits to Encourage Experimentation on Kusama ([polkadot-fellows/runtimes#501](https://github.com/polkadot-fellows/runtimes/pull/501))

### Fixed

- Fix `experimental_inflation_info` in Polkadot and remove unused code (https://github.com/polkadot-fellows/runtimes/pull/497)

## [1.3.3] 01.10.2024

### Changed

- Allow signed origins to send arbitrary XCMs from some system chains ([polkadot-fellows/runtimes#407](https://github.com/polkadot-fellows/runtimes/pull/407))
- Include the Core and Salary pallets into the Fellowship proxy ([polkadot-fellows/runtimes#454](https://github.com/polkadot-fellows/runtimes/pull/454))
- Add new community democracy and treasuries pallets to Encointer ([polkadot-fellows/runtimes#456](https://github.com/polkadot-fellows/runtimes/pull/456))
- Change target block time for Encointer to 6s ([polkadot-fellows/runtimes#462](https://github.com/polkadot-fellows/runtimes/pull/462))
- Asset Hubs: allow Polkadot, Kusama and Ethereum assets across P<>K bridge ([polkadot-fellows/runtimes#421](https://github.com/polkadot-fellows/runtimes/pull/421)).

### Fixed

- Chain-spec generator: propagate the `on_chain_release_build` feature to the chain-spec generator. Without this the live/genesis chain-specs contain a wrongly-configured WASM blob ([polkadot-fellows/runtimes#450](https://github.com/polkadot-fellows/runtimes/pull/450)).
- Adds a migration to the Polkadot Coretime chain to fix an issue from the initial Coretime migration. ([polkadot-fellows/runtimes#458](https://github.com/polkadot-fellows/runtimes/pull/458))
- Adds migrations to restore currupted staking ledgers in Polkadot and Kusama ([polkadot-fellows/runtimes#447](https://github.com/polkadot-fellows/runtimes/pull/447))

### Added

- Polkadot: Make the current inflation formula adjustable ([polkadot-fellows/runtimes#443](https://github.com/polkadot-fellows/runtimes/pull/443))

## [1.3.2] 27.08.2024

### Fixed

- Kusama: Revert accidental changes to inflation formula ([polkadot-fellows/runtimes#445](https://github.com/polkadot-fellows/runtimes/pull/445)).

## [1.3.1] 23.08.2024

### Fixed

- [🚨 Breaking Change] Polkadot Collectives: enable transaction payment ([polkadot-fellows/runtimes#442](https://github.com/polkadot-fellows/runtimes/pull/442))

## [1.3.0] 20.08.2024

### Added

- Kusama: Relay General Admin Origin mapping to xcm Location ([polkadot-fellows/runtimes#383](https://github.com/polkadot-fellows/runtimes/pull/383))
- Encointer, PeopleKusama, PeoplePolkadot: Configure delivery fees for UMP ([polkadot-fellows/runtimes#390](https://github.com/polkadot-fellows/runtimes/pull/390))
- Introduce a new dispatchable function `set_partial_params` in `pallet-core-fellowship` ([runtimes#381](https://github.com/polkadot-fellows/runtimes/pull/381), [SDK v1.14 #3843](https://github.com/paritytech/polkadot-sdk/pull/3843)).
- RFC-5: Add request revenue info ([runtimes#381](https://github.com/polkadot-fellows/runtimes/pull/381), [SDK v1.14 #3940](https://github.com/paritytech/polkadot-sdk/pull/3940)).
- Core-Fellowship: new `promote_fast` call ([runtimes#381](https://github.com/polkadot-fellows/runtimes/pull/381), [SDK v1.14 #4877](https://github.com/paritytech/polkadot-sdk/pull/4877)).
- Pallet ranked collective: max member count per rank ([runtimes#381](https://github.com/polkadot-fellows/runtimes/pull/381), [SDK v1.14 #4807](https://github.com/paritytech/polkadot-sdk/pull/4807)).
- All runtimes: XcmPaymentApi and DryRunApi ([polkadot-fellows/runtimes#380](https://github.com/polkadot-fellows/runtimes/pull/380))
- Fast promotion tracks for the Fellowship ranks I-III ([polkadot-fellows/runtimes#356](https://github.com/polkadot-fellows/runtimes/pull/356)).
- All runtimes: add `LocationToAccountApi` ([polkadot-fellows/runtimes#413](https://github.com/polkadot-fellows/runtimes/pull/413))
- Enable Agile Coretime on Polkadot ([polkadot-fellows/runtimes#401](https://github.com/polkadot-fellows/runtimes/pull/401))
- Add the Polkadot Coretime Chain runtime ([polkadot-fellows/runtimes#410](https://github.com/polkadot-fellows/runtimes/pull/410))
- Kusama: Add a "Spokesperson" proxy type only allowed to send remarks ([polkadot-fellows/runtimes#430](https://github.com/polkadot-fellows/runtimes/pull/430))
- Add the Polkadot and Kusama Coretime Chain specs ([polkadot-fellows/runtimes#432](https://github.com/polkadot-fellows/runtimes/pull/432))
- Migration to remove all but the 21 first elected Head Ambassador members from the Program ([polkadot-fellows/runtimes#422](https://github.com/polkadot-fellows/runtimes/pull/422)).
- Kusama: Make the current inflation formula adjustable ([polkadot-fellows/runtimes#364](https://github.com/polkadot-fellows/runtimes/pull/364))
- Port Agile Coretime migration from polkadot-sdk in order to fix leases with gaps handling([polkadot-fellows/runtimes#426](https://github.com/polkadot-fellows/runtimes/pull/426))

#### From [#322](https://github.com/polkadot-fellows/runtimes/pull/322)

- Add `claim_assets` extrinsic to `pallet-xcm` ([SDK v1.9 #3403](https://github.com/paritytech/polkadot-sdk/pull/3403)).
- Add `Deposited`/`Withdrawn` events for `pallet-assets` ([SDK v1.12 #4312](https://github.com/paritytech/polkadot-sdk/pull/4312)).
- Add `MaxRank` Config to `pallet-core-fellowship` ([SDK v1.13 #3393](https://github.com/paritytech/polkadot-sdk/pull/3393)).
- Add Extra Check in Primary Username Setter ([SDK v1.13 #4534](https://github.com/paritytech/polkadot-sdk/pull/4534)).
- Add HRMP notification handlers to the xcm-executor ([SDK v1.10 #3696](https://github.com/paritytech/polkadot-sdk/pull/3696)).
- Add retry mechanics to `pallet-scheduler` ([SDK v1.8 #3060](https://github.com/paritytech/polkadot-sdk/pull/3060)).
- Add support for versioned notification for HRMP pallet ([SDK v1.12 #4281](https://github.com/paritytech/polkadot-sdk/pull/4281)).
- Adds ability to trigger tasks via unsigned transactions ([SDK v1.11 #4075](https://github.com/paritytech/polkadot-sdk/pull/4075)).
- Asset Conversion: Pool Account ID derivation with additional Pallet ID seed ([SDK v1.11 #3250](https://github.com/paritytech/polkadot-sdk/pull/3250)).
- Asset Conversion: Pool Touch Call ([SDK v1.11 #3251](https://github.com/paritytech/polkadot-sdk/pull/3251)).
- Balances: add failsafe for consumer ref underflow ([SDK v1.12 #3865](https://github.com/paritytech/polkadot-sdk/pull/3865)).
- Bridge: added force_set_pallet-state call to pallet-bridge-grandpa ([SDK v1.13 #4465](https://github.com/paritytech/polkadot-sdk/pull/4465)).
- Burn extrinsic call and `fn burn_from` `Preservation` argument ([SDK v1.12 #3964](https://github.com/paritytech/polkadot-sdk/pull/3964)).
- GenesisConfig presets for runtime ([SDK v1.11 #2714](https://github.com/paritytech/polkadot-sdk/pull/2714)).
- Im-online pallet offchain storage cleanup ([SDK v1.8 #2290](https://github.com/paritytech/polkadot-sdk/pull/2290)).
- Implements a percentage cap on staking rewards from era inflation ([SDK v1.8 #1660](https://github.com/paritytech/polkadot-sdk/pull/1660)).
- Introduce submit_finality_proof_ex call to bridges GRANDPA pallet ([SDK v1.8 #3225](https://github.com/paritytech/polkadot-sdk/pull/3225)).
- New call `hrmp.establish_channel_with_system` to allow parachains to establish a channel with a system parachain ([SDK v1.11 #3721](https://github.com/paritytech/polkadot-sdk/pull/3721)).
- New runtime api to check if a validator has pending pages of rewards for an era ([SDK v1.12 #4301](https://github.com/paritytech/polkadot-sdk/pull/4301)).
- Pallet-xcm: add new extrinsic for asset transfers using explicit reserve ([SDK v1.11 #3695](https://github.com/paritytech/polkadot-sdk/pull/3695)).
- Ranked collective introduce `Add` and `Remove` origins ([SDK v1.8 #3212](https://github.com/paritytech/polkadot-sdk/pull/3212)).
- Runtime apis to help with delegate-stake based Nomination Pools ([SDK v1.13 #4537](https://github.com/paritytech/polkadot-sdk/pull/4537)).

### Changed

- Polkadot chains: allow arbitrary XCM execution ([polkadot-fellows/runtimes#345](https://github.com/polkadot-fellows/runtimes/pull/345)).
- Bounties: Remove payout delay ([polkadot-fellows/runtimes#386](https://github.com/polkadot-fellows/runtimes/pull/386)).
- Polkadot System Chains: Reduce the base transaction fee by half ([polkadot-fellows/runtimes#398](https://github.com/polkadot-fellows/runtimes/pull/398)).
- Asset Hubs: setup auto incremented asset id to 50_000_000 for trust backed assets ([polkadot-fellows/runtimes#414](https://github.com/polkadot-fellows/runtimes/pull/414)).
- Upgrade dependencies to the [polkadot-sdk@1.13.0](https://github.com/paritytech/polkadot-sdk/releases/tag/polkadot-v1.13.0) release ([polkadot-fellows/runtimes#332](https://github.com/polkadot-fellows/runtimes/pull/332)).
- Filter `interlace` calls on the Polkadot Coretime Chain until the Relay chain implementation is more mature ([polkadot-fellows/runtimes#438](https://github.com/polkadot-fellows/runtimes/pull/438)).

#### From [#322](https://github.com/polkadot-fellows/runtimes/pull/322)

- The `MessageQueue` also runs "on idle", this causes `MessageQueue::Processed` events to be emitted in other phases than just initialization ([SDK v1.13 #3844](https://github.com/paritytech/polkadot-sdk/pull/3844)).
- AdaptPrice trait is now price controlled ([SDK v1.13 #4521](https://github.com/paritytech/polkadot-sdk/pull/4521)).
- Allow StakingAdmin to manage nomination pool configurations ([SDK v1.11 #3959](https://github.com/paritytech/polkadot-sdk/pull/3959)).
- Bridge: make some headers submissions free ([SDK v1.12 #4102](https://github.com/paritytech/polkadot-sdk/pull/4102)).
- Improving on_demand_assigner emitted events ([SDK v1.13 #4339](https://github.com/paritytech/polkadot-sdk/pull/4339)).
- `pallet-broker::start_sales`: Take `extra_cores` and not total cores ([SDK v1.11 #4221](https://github.com/paritytech/polkadot-sdk/pull/4221)).
- Pallet-nomination-pools: `chill` is permissionless if depositor's stake is less than `min_nominator_bond` ([SDK v1.9 #3453](https://github.com/paritytech/polkadot-sdk/pull/3453)).
- `polkadot_runtime_parachains::coretime`: Expose `MaxXcmTransactWeight` ([SDK v1.11 #4189](https://github.com/paritytech/polkadot-sdk/pull/4189)).
- Pools: Make PermissionlessWithdraw the default claim permission ([SDK v1.10 #3438](https://github.com/paritytech/polkadot-sdk/pull/3438)).
- Prevents staking controllers from becoming stashes of different ledgers; Ensures that no ledger in bad state is mutated ([SDK v1.9 #3639](https://github.com/paritytech/polkadot-sdk/pull/3639)).
- Snowbridge: deposit extra fee to beneficiary on Asset Hub ([SDK v1.12 #4175](https://github.com/paritytech/polkadot-sdk/pull/4175)).
- Storage bound the XCMP queue pallet ([SDK v1.13 #3952](https://github.com/paritytech/polkadot-sdk/pull/3952)).
- Validator disabling strategy in runtime ([SDK v1.12 #2226](https://github.com/paritytech/polkadot-sdk/pull/2226)).

### Fixed

- Fix claim queue size ([runtimes#381](https://github.com/polkadot-fellows/runtimes/pull/381), [SDK v1.14 #4691](https://github.com/paritytech/polkadot-sdk/pull/4691)).
- `pallet-referenda`: Ensure to schedule referenda earliest at the next block ([runtimes#381](https://github.com/polkadot-fellows/runtimes/pull/381), [SDK v1.14 #4823](https://github.com/paritytech/polkadot-sdk/pull/4823)).
- Don't partially modify HRMP pages ([runtimes#381](https://github.com/polkadot-fellows/runtimes/pull/381), [SDK v1.14 #4710](https://github.com/paritytech/polkadot-sdk/pull/4710)).
- Coretime Chain: mitigate behaviour with many assignments on one core ([runtimes#434](https://github.com/polkadot-fellows/runtimes/pull/434)).
- Port Agile Coretime migration from polkadot-sdk in order to fix leases with gaps handling([polkadot-fellows/runtimes#426](https://github.com/polkadot-fellows/runtimes/pull/426))

#### From [#322](https://github.com/polkadot-fellows/runtimes/pull/322)

- CheckWeight checks for combined extrinsic length and proof size ([SDK v1.12 #4326](https://github.com/paritytech/polkadot-sdk/pull/4326)).
- Decrement total_deposit when clearing collection metadata ([SDK v1.11 #3976](https://github.com/paritytech/polkadot-sdk/pull/3976)).
- Detect incorrect pre-image length when submitting a referenda ([SDK v1.10 #3850](https://github.com/paritytech/polkadot-sdk/pull/3850)).
- Fix `schedule_code_upgrade` when called by the owner/root ([SDK v1.10 #3341](https://github.com/paritytech/polkadot-sdk/pull/3341)).
- Fix algorithmic complexity of the on-demand scheduler ([SDK v1.10 #3190](https://github.com/paritytech/polkadot-sdk/pull/3190)).
- Fix call enum's metadata regression ([SDK v1.9 #3513](https://github.com/paritytech/polkadot-sdk/pull/3513)).
- Fix dust unbonded for zero existential deposit ([SDK v1.12 #4364](https://github.com/paritytech/polkadot-sdk/pull/4364)).
- Fix extrinsics count logging in frame-system ([SDK v1.12 #4461](https://github.com/paritytech/polkadot-sdk/pull/4461)).
- Fix kusama 0 backing rewards when entering active set ([SDK v1.10 #3722](https://github.com/paritytech/polkadot-sdk/pull/3722)).
- Fix Stuck Collator Funds ([SDK v1.11 #4229](https://github.com/paritytech/polkadot-sdk/pull/4229)).
- Fix weight calculation and event emission in pallet-membership ([SDK v1.9 #3324](https://github.com/paritytech/polkadot-sdk/pull/3324)).
- Fix weight refund for `pallet-collator-selection::set_candidacy_bond` ([SDK v1.9 #3643](https://github.com/paritytech/polkadot-sdk/pull/3643)).
- Fixed `GrandpaConsensusLogReader::find_scheduled_change` ([SDK v1.11 #4208](https://github.com/paritytech/polkadot-sdk/pull/4208)).
- Fixes a scenario where a nomination pool's `TotalValueLocked` is out of sync due to staking's implicit withdraw ([SDK v1.8 #3052](https://github.com/paritytech/polkadot-sdk/pull/3052)).
- Handle legacy lease swaps on coretime ([SDK v1.10 #3714](https://github.com/paritytech/polkadot-sdk/pull/3714)).
- Ignore mandatory extrinsics in total PoV size check ([SDK v1.13 #4571](https://github.com/paritytech/polkadot-sdk/pull/4571)).
- Pallet assets: minor improvement on errors returned for some calls ([SDK v1.11 #4118](https://github.com/paritytech/polkadot-sdk/pull/4118)).
- Pallet-broker: Fix `Linear::adapt_price` behavior at zero ([SDK v1.9 #3636](https://github.com/paritytech/polkadot-sdk/pull/3636)).
- Pallet-broker: Fix claim revenue behaviour for zero timeslices ([SDK v1.11 #3997](https://github.com/paritytech/polkadot-sdk/pull/3997)).
- Pallet-broker: Support renewing leases expired in a previous period ([SDK v1.11 #4089](https://github.com/paritytech/polkadot-sdk/pull/4089)).
- Pallet-broker: Use saturating math in input validation ([SDK v1.11 #4151](https://github.com/paritytech/polkadot-sdk/pull/4151)).
- Pallet-xcm: fix transport fees for remote reserve transfers ([SDK v1.10 #3792](https://github.com/paritytech/polkadot-sdk/pull/3792)).
- Patch pool to handle extra consumer ref when destroying ([SDK v1.13 #4503](https://github.com/paritytech/polkadot-sdk/pull/4503)).
- Region reserve transfers fix ([SDK v1.11 #3455](https://github.com/paritytech/polkadot-sdk/pull/3455)).
- Snowbridge - Ethereum Client - Reject finalized updates without a sync committee in next store period ([SDK v1.13 #4478](https://github.com/paritytech/polkadot-sdk/pull/4478)).
- Treat XCM ExceedsStackLimit errors as transient in the MQ pallet ([SDK v1.12 #4202](https://github.com/paritytech/polkadot-sdk/pull/4202)).
- Unrequest a pre-image when it failed to execute ([SDK v1.10 #3849](https://github.com/paritytech/polkadot-sdk/pull/3849)).
- Validate code when scheduling uprades ([SDK v1.8 #3232](https://github.com/paritytech/polkadot-sdk/pull/3232)).
- XCMP: Use the number of 'ready' pages in XCMP suspend logic ([SDK v1.9 #2393](https://github.com/paritytech/polkadot-sdk/pull/2393)).

### Removed

- Remove deprecated calls from treasury pallet ([runtimes#381](https://github.com/polkadot-fellows/runtimes/pull/381), [SDK v1.14 #3820](https://github.com/paritytech/polkadot-sdk/pull/3820)).
- Treasury pallet: - remove unused config parameters ([runtimes#381](https://github.com/polkadot-fellows/runtimes/pull/381), [SDK v1.14 #4831](https://github.com/paritytech/polkadot-sdk/pull/4831)).
- Remove Identity from Polkadot Relay Chain ([runtimes#415](https://github.com/polkadot-fellows/runtimes/pull/415))
- Kusama: Remove unused Snowbridge code and configs ([polkadot-fellows/runtimes#411](https://github.com/polkadot-fellows/runtimes/pull/411)).
- Remove the identity ops pallet after the invalid judgments have been cleared ([polkadot-fellows/runtimes#408](https://github.com/polkadot-fellows/runtimes/pull/408)).

#### From [#322](https://github.com/polkadot-fellows/runtimes/pull/322)

- Deprecate dmp-queue pallet ([SDK v1.13 #4475](https://github.com/paritytech/polkadot-sdk/pull/4475)).
- Deprecate XCMv2 ([SDK v1.13 #4131](https://github.com/paritytech/polkadot-sdk/pull/4131)).
- Identity: Remove double encoding username signature payload ([SDK v1.13 #4646](https://github.com/paritytech/polkadot-sdk/pull/4646)).
- Pallet-xcm: deprecate execute and send in favor of execute_blob and send_blob ([SDK v1.10 #3749](https://github.com/paritytech/polkadot-sdk/pull/3749)).
- Pallet-xcm: deprecate transfer extrinsics without weight limit ([SDK v1.10 #3927](https://github.com/paritytech/polkadot-sdk/pull/3927)).
- Remove `parametrized-consensus-hook` feature ([SDK v1.13 #4380](https://github.com/paritytech/polkadot-sdk/pull/4380)).

## [1.2.8] 03.07.2024

### Changed

- Snowbridge: Sync headers on demand ([polkadot-fellows/runtimes#365](https://github.com/polkadot-fellows/runtimes/pull/365))
- Polkadot chains: allow arbitrary XCM execution ([polkadot-fellows/runtimes#345](https://github.com/polkadot-fellows/runtimes/pull/345)).

Note: This release only affects the following runtimes and is not a full system release:

- Polkadot Relay Chain
- Polkadot Asset Hub
- Polkadot Bridge Hub
- Polkadot Collectives
- Kusama Relay Chain
- Kusama Bridge Hub

### Fixed

- Kusama People: Build the metadata hash at build time, so that `CheckMetadata` can use it at runtime ([polkadot-fellows/runtimes#371](https://github.com/polkadot-fellows/runtimes/pull/371))

## [1.2.7] 14.06.2024

Note: This release only affects the following runtimes and is not a full system release:

- Polkadot Relay Chain
- Polkadot People

### Changed

- Updated Relay and People configurations to complete launch ([polkadot-fellows/runtimes#350](https://github.com/polkadot-fellows/runtimes/pull/350))

## [1.2.6] 13.06.2024

Note: This release only affects the following runtimes and is not a full system release:

- Polkadot Relay Chain
- Polkadot Asset Hub
- Polkadot People
- Kusama Relay Chain
- Kusama Asset Hub
- Kusama People

### Added

- Add the Polkadot People Chain ([polkadot-fellows/runtimes#319](https://github.com/polkadot-fellows/runtimes/pull/319))

### Changed

- Set max asset ID restriction for the creation of trusted assets ([polkadot-fellows/runtimes#346](https://github.com/polkadot-fellows/runtimes/pull/346))

### Fixed

- Kusama People: clear requested judgements that do not have corresponding deposits reserved ([polkadot-fellows/runtimes#339](https://github.com/polkadot-fellows/runtimes/pull/339))

### Changed

- People chain now uses 6-second block times ([polkadot-fellows/runtimes#308](https://github.com/polkadot-fellows/runtimes/pull/308))

### Removed

- Removed Identity-related code from Kusama Relay Chain ([polkadot-fellows/runtimes#315](https://github.com/polkadot-fellows/runtimes/pull/315))

## [1.2.5] 06.06.2024

### Added

- Staking runtime api to check if reward is pending for an era ([polkadot-fellows/runtimes#318](https://github.com/polkadot-fellows/runtimes/pull/318))
- Allow any parachain to have bidirectional channel with any system parachains ([polkadot-fellows/runtimes#329](https://github.com/polkadot-fellows/runtimes/pull/329))
- Update price controller of broker pallet to use higher leadin, without adjusting the minimum price too much ([polkadot-fellows/runtimes#334](https://github.com/polkadot-fellows/runtimes/pull/334))
- Enable support for new hardware signers like the generic ledger app ([polkadot-fellows/runtimes#337](https://github.com/polkadot-fellows/runtimes/pull/337))

### Changed

- Transaction payments work via new `fungible` trait implementation ([polkadot-fellows/runtimes#332](https://github.com/polkadot-fellows/runtimes/pull/332))
- Block `request_judgement` calls on the Relay Chain ([polkadot-fellows/runtimes#338](https://github.com/polkadot-fellows/runtimes/pull/338))

### Fixed

- Handle extra erroneous consumer reference when a nomination pool is destroying ([polkadot-fellows/runtimes#318](https://github.com/polkadot-fellows/runtimes/pull/318))
- Introduce [Encointer](https://encointer.org) collator selection and send fees to authors instead of treasury ([polkadot-fellows/runtimes#270](https://github.com/polkadot-fellows/runtimes/pull/270))

## [1.2.4] 20.05.2024

### Changed

- Kusama chains: allow arbitrary XCM execution ([polkadot-fellows/runtimes#261](https://github.com/polkadot-fellows/runtimes/pull/261))
- Allow everything through XCM SafeCallFilter ([polkadot-fellows/runtimes#285](https://github.com/polkadot-fellows/runtimes/pull/285))
- Disable Coretime credit purchasing until we have the credit system implemented ([polkadot-fellows/runtimes#312](https://github.com/polkadot-fellows/runtimes/pull/312))

### Added

- Add `pallet-vesting` to Asset Hubs ([polkadot-fellows/runtimes#269](https://github.com/polkadot-fellows/runtimes/pull/269))
- Add Pay Salary Collectives test ([polkadot-fellows/runtimes#260](https://github.com/polkadot-fellows/runtimes/pull/260))
- Add `pallet-xcm::transfer_assets_using_type_and_then()` for complex asset transfers ([polkadot-fellows/runtimes#311](https://github.com/polkadot-fellows/runtimes/pull/311))
- The Ambassador Program ([polkadot-fellows/runtimes#291](https://github.com/polkadot-fellows/runtimes/pull/291))

### Removed

- Remove one-shot migrations from Kusama Coretime ([polkadot-fellows/runtimes#300](https://github.com/polkadot-fellows/runtimes/pull/300))
- Remove DMP queue and allow `system::authorize_upgrade` in XCM's call filter ([polkadot-fellows/runtimes#280](https://github.com/polkadot-fellows/runtimes/pull/280))
- Allow Sending XCM messages using a Signed origin on Kusama ([polkadot-fellows/runtimes#290](https://github.com/polkadot-fellows/runtimes/pull/290))

### Fixed

- Include patch to release stuck collator bonds ([polkadot-fellows/runtimes#289](https://github.com/polkadot-fellows/runtimes/pull/289))
- Safeguard pallet-balances against consumer ref underflow ([polkadot-fellows/runtimes#309](https://github.com/polkadot-fellows/runtimes/pull/309))
- Polkadot Bridge Hub: Unstuck Snowbridge ([polkadot-fellows/runtimes#313](https://github.com/polkadot-fellows/runtimes/pull/313))

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
- Add [Encointer](https://encointer.org) system parachain runtime, completing [RFC22](https://github.com/polkadot-fellows/RFCs/blob/main/text/0022-adopt-encointer-runtime.md) ([polkadot-fellows/runtimes#80](https://github.com/polkadot-fellows/runtimes/pull/80))
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
- Make `IdentityInfo` generic in `pallet-identity` ([polkadot-fellows/runtimes#87](https://github.com/polkadot-fellows/runtimes/pull/87)). Context: <https://github.com/paritytech/polkadot-sdk/pull/1661>
- Whitelist `force_default_xcm_version` in XCM call filter ([polkadot-fellows/runtimes#45](https://github.com/polkadot-fellows/runtimes/pull/45))
- Update the fellowship salary budget amount in alignment with the Fellowship Salary [RFC](https://github.com/polkadot-fellows/RFCs/pull/50) ([polkadot-fellows/runtimes#121](https://github.com/polkadot-fellows/runtimes/pull/121))
- Set up an account ID for the local root location on Polkadot Collectives ([polkadot-fellows/runtimes#125](https://github.com/polkadot-fellows/runtimes/pull/125))
- Increase confirmation period for treasury spend tracks on Polkadot & Kusama ([polkadot-fellows/runtimes#119](https://github.com/polkadot-fellows/runtimes/pull/119))
- Drop ED requirement for transaction payments with an exchangeable asset ([polkadot-fellows/runtimes#310](https://github.com/polkadot-fellows/runtimes/pull/310))

### Added

- Enable async backing on Kusama ([polkadot-fellows/runtimes#87](https://github.com/polkadot-fellows/runtimes/pull/87)). Context: <https://github.com/paritytech/polkadot-sdk/pull/1543>
- Implemented GenesisBuilder API for all runtimes ([polkadot-fellows/runtimes#87](https://github.com/polkadot-fellows/runtimes/pull/87)). Context: <https://github.com/paritytech/polkadot-sdk/pull/1492>
- XCM transport fees are now exponential and are sent to a treasury account ([polkadot-fellows/runtimes#87](https://github.com/polkadot-fellows/runtimes/pull/87)). Context: <https://github.com/paritytech/polkadot-sdk/pull/1234>
- System parachains are now trusted teleporters of each other ([polkadot-fellows/runtimes#87](https://github.com/polkadot-fellows/runtimes/pull/87)). Context: <https://github.com/paritytech/polkadot-sdk/pull/1368>
- Treasury is able to spend various asset kinds ([polkadot-fellows/runtimes#87](https://github.com/polkadot-fellows/runtimes/pull/87))
- Add BEEFY to Polkadot ([polkadot-fellows/runtimes#65](https://github.com/polkadot-fellows/runtimes/pull/65))
- Fellowship Treasury pallet on Polkadot Collectives ([polkadot-fellows/runtimes#109](https://github.com/polkadot-fellows/runtimes/pull/109))
- Added Polkadot <> Kusama bridge to support asset transfers between Asset Hubs ([polkadot-fellows/runtimes#108](https://github.com/polkadot-fellows/runtimes/pull/108))

### Fixed

- Add missing weight functions for `runtime_parachains_hrmp` and `preimage` pallets ([polkadot-fellows/runtimes#56](https://github.com/polkadot-fellows/runtimes/pull/56))
- Fix for Reward Deficit in the pool ([polkadot-fellows/runtimes#87](https://github.com/polkadot-fellows/runtimes/pull/87)). Context: <https://github.com/paritytech/polkadot-sdk/pull/1255>

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
