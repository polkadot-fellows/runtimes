# Changelog

Changelog for the runtimes governed by the Polkadot Fellowship.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Added

- Set Ethereum Fulu fork epoch ([#1005](https://github.com/polkadot-fellows/runtimes/pull/1005)).

### Changed

- Remove XCM Transact filters and Pool asset transactors ([#1014](https://github.com/polkadot-fellows/runtimes/pull/1014))

## [2.0.2] 07.11.2025

### Fixed

- Fix AH staking inflation calculation to use correct total issuance (https://github.com/polkadot-fellows/runtimes/pull/998).
- Set invulnerable deposit for Polkadot AssetHub staking election ([#993](https://github.com/polkadot-fellows/runtimes/pull/993))
- Fix staking on Asset Hub via XCM ([#1006](https://github.com/polkadot-fellows/runtimes/pull/1006))
- Fix flaky `curl` download command in CI ([#1006](https://github.com/polkadot-fellows/runtimes/pull/1006))

## [2.0.1] 04.11.2025

### Removed

 - Remove AHM Test code to speed up CI ([#997](https://github.com/polkadot-fellows/runtimes/pull/997))
 - Relinquish AHM multisig functionality post AHM ([#997](https://github.com/polkadot-fellows/runtimes/pull/997))

### Changed

- Bump deps ([#997](https://github.com/polkadot-fellows/runtimes/pull/997))

## [2.0.0] 27.10.2025

### Added

- Scheduled the Polkadot Asset Hub Migration for block [28490502](https://polkadot.subscan.io/block/28490502), circa Tuesday 4th Nov 8 AM UTC ([polkadot-fellows/runtimes/pull/984](https://github.com/polkadot-fellows/runtimes/pull/984)).

### Changed

- Enable preimages pallet on Kusama Relay ([polkadot-fellows/runtimes/pull/957](https://github.com/polkadot-fellows/runtimes/pull/957))
- Allow the AHM multisig to act as preimage manager ([polkadot-fellows/runtimes/pull/976](https://github.com/polkadot-fellows/runtimes/pull/976))
- `RcToAhCall` supports mapping some basic XCMs ([polkadot-fellows/runtimes/pull/983](https://github.com/polkadot-fellows/runtimes/pull/983))
- AHM: map more Referenda XCM instructions ([#983](https://github.com/polkadot-fellows/runtimes/pull/983))
- AHM: Schedule polkadot migration for block [28490502](https://polkadot.subscan.io/block/28490502) ([#984](https://github.com/polkadot-fellows/runtimes/pull/984))

### Fixed

- Let multisig round start from 100 for Polkadot Relay ([polkadot-fellows/runtimes/pull/957](https://github.com/polkadot-fellows/runtimes/pull/957))
- Fix staking-async [sdk #9926](https://github.com/paritytech/polkadot-sdk/pull/9926): chill stakers should not have a score ([#960](https://github.com/polkadot-fellows/runtimes/pull/960))
- Fix resending of duplicate AHM messages ([#970](https://github.com/polkadot-fellows/runtimes/pull/970))

## [1.9.3] 21.10.2025

### Added

- Enable view functions on System Chains([polkadot-fellows/runtimes/pull/981](https://github.com/polkadot-fellows/runtimes/pull/981))

### Fixed

- [BHP](https://github.com/polkadot-fellows/runtimes/pull/978) Add missing snowbridge runtime API to the BridgeHub
  runtime.
- Bump `pallet-staking-async` to `0.6.2` to fix incorrect self stake accounting (https://github.com/polkadot-fellows/runtimes/pull/980)

## [1.9.2] 08.10.2025

### Added

- Kusama Asset Hub: add missing staking Runtime APIs and  re-enable vested transfers ([polkadot-fellows/runtimes/pull/946](https://github.com/polkadot-fellows/runtimes/pull/946))
- Polkadot Asset Hub: add missing staking Runtime APIs ([polkadot-fellows/runtimes/pull/946](https://github.com/polkadot-fellows/runtimes/pull/949))

### Fixed

- [AHM] Do not migrate staking era forcing info to AH ([polkadot-fellows/runtimes/pull/939](https://github.com/polkadot-fellows/runtimes/pull/939))
- [AHM] Small fixes to successfully dry-run migration tests ([polkadot-fellows/runtimes/pull/942](https://github.com/polkadot-fellows/runtimes/pull/942))
- [AHM] Fix crowdloan withdrawing and weight limit ([polkadot-fellows/runtimes/pull/943](https://github.com/polkadot-fellows/runtimes/pull/943))
- [Encointer] Fix remote treasury payout on asset hub ([polkadot-fellows/runtimes/pull/944](https://github.com/polkadot-fellows/runtimes/pull/944))
- [AHM] Post Kusama Migration cleanup ([polkadot-fellows/runtimes/pull/946](https://github.com/polkadot-fellows/runtimes/pull/946))
- [AHM] Improve StakingAsync's VMP Messaging (https://github.com/polkadot-fellows/runtimes/pull/950)

## [1.9.1] 30.09.2025

### Fixed

- Reduce runtime blob size by 800KB to fit the limits ([polkadot-fellows/runtimes/pull/938](https://github.com/polkadot-fellows/runtimes/pull/938))

## [1.9.0] 26.09.2025

### Added

- Enable the Asset Hub Migration for Kusama at block `30423691`, projected to be Tuesday 7th Oct 8 AM UTC ([polkadot-fellows/runtimes/pull/935](https://github.com/polkadot-fellows/runtimes/pull/935))
- Code for the Asset Hub Migration ([polkadot-fellows/runtimes/pull/856](https://github.com/polkadot-fellows/runtimes/pull/856))

### Changed

-  Pallet XCM - Disable reserve_asset_transfer for DOT|KSM ([polkadot-fellows/runtimes/pull/880](https://github.com/polkadot-fellows/runtimes/pull/880))
  🚨 Pallet XCM's `limited_reserve_transfer_assets` and `reserve_transfer_assets` extrinsics now returns an error when it determines that a reserve transfer of DOT|KSM has to be done.
  This is a safeguard in preparation for the Asset Hub Migration (AHM), where the reserve of DOT|KSM will change from the Relay Chain to Asset Hub.
  After the migration, another patch will remove this error case and use the correct reserve.
  🚨 For DOT|KSM cross-chain transfers please use `transfer_assets_using_type_and_then` or `execute`.
  Please see this [Polkadot forum post](https://forum.polkadot.network/t/mandatory-action-guide-for-ahm-broken-native-crosschain-transfers/) for more details.

## [1.7.1] 28.08.2025

### Fixed

- [#9564](https://github.com/paritytech/polkadot-sdk/pull/9564) Correctly map group indices to vote indices when filtering backing statements.

### Changed

- [#861](https://github.com/polkadot-fellows/runtimes/pull/861) Removed the custom fungible adapter used by Kusama AssetHub
- Support Snowbridge bridge reward payouts on AssetHub ([polkadot-fellows/runtimes/pull/865](https://github.com/polkadot-fellows/runtimes/pull/865))

## [1.7.0] 22.08.2025

### Fixed

- Use `pallet-assets` instead of `pallet-balances` for XCM benchmarks on asset hubs ([polkadot-fellows/runtimes/pull/758](https://github.com/polkadot-fellows/runtimes/pull/758))
  - This means XCM benchmarks will have a higher weight.
- All XCM benchmarks use sibling parachain as destination instead of Relay chain to properly adapt weights in context of incoming migration from Relay to Asset Hub ([polkadot-fellows/runtimes/pull/709](https://github.com/polkadot-fellows/runtimes/pull/709))

### Added
- Integrate "Empowered XCM Origins" features to System Chains ([polkadot-fellows/runtimes/pull/799](https://github.com/polkadot-fellows/runtimes/pull/799))
- Test cases for all system chains to verify if parachain is able to process authorize_upgrade call as if it was received from governance chain ([polkadot-fellows/runtimes/pull/783](https://github.com/polkadot-fellows/runtimes/pull/783))
- Add Secretary Salary Pay Test Over XCM ([https://github.com/polkadot-fellows/runtimes/pull/778](https://github.com/polkadot-fellows/runtimes/pull/778))
- Upgrade to Polkadot-SDK `stable2506` ([polkadot-fellows/runtimes/pull/817](https://github.com/polkadot-fellows/runtimes/pull/817))
  - [#7833](https://github.com/paritytech/polkadot-sdk/pull/7833): Add `poke_deposit` extrinsic to pallet-society
  - [#7995](https://github.com/paritytech/polkadot-sdk/pull/7995): Add `PureKilled` event to pallet-proxy
  - [#8254]((https://github.com/paritytech/polkadot-sdk/pull/9202)): Introduce `remove_upgrade_cooldown`
    This dispatchable enables anyone to pay for removing an active upgrade cooldown from a parachain instead of waiting for the cooldown to be finished. It is useful for times when a parachain needs to apply an upgrade faster than the upgrade cooldown, but it will need to pay in this case. The dispatchable enables anyone to remove an upgrade cooldown of any parachain. The caller needs to pay for the removal and the tokens are burned on a successful removal.
  - [#8171](https://github.com/paritytech/polkadot-sdk/pull/8171): Add event `VestingCreated` and emit on vested transfer.
  - [#8382](https://github.com/paritytech/polkadot-sdk/pull/8382): Add `poke_deposit` extrinsic to pallet-bounties
  - [#7592](https://github.com/paritytech/polkadot-sdk/pull/7592): Add Paras `authorize_code_hash` + `apply_authorized_code` feature
    This feature is useful when triggering a Paras pallet call from a different chain than the one where the Paras pallet is deployed. For example, we may want to send `Paras::force_set_current_code(para, code)` from the Collectives and/or Asset Hub to the Relay Chain (because the Relay Chain governance will be migrated to the Asset Hub as a part of AHM).
    The primary reason for this approach is to avoid transferring the entire `new_code` Wasm blob between chains. Instead, we authorize the `code_hash` using root via `fn authorize_force_set_current_code_hash(new_authorization, expire_at)`. This authorization can later be applied by anyone using `Paras::apply_authorized_force_set_current_code(para, new_code)`. If `expire_at` is reached without the authorization being used, it is automatically removed.
  - [#7882](https://github.com/paritytech/polkadot-sdk/pull/7882): Add `poke_deposit` extrinsic to pallet-recovery
    Historically, the collection of storage deposits was running in an infallible context. Meaning we needed to make sure that the caller was able to pay the deposits when the last contract execution returns. To achieve that, we capped the storage deposit limit to the maximum balance of the origin. This made the code more complex: It conflated the deposit limit with the amount of balance the origin has.
    In the meantime, we changed code around to make the deposit collection fallible. But never changed this aspect.
    This PR rectifies that by doing:
    The root storage meter and all its nested meter's limits are completely independent of the origin's balance. This makes it way easier to argue about the limit that a nested meter has at any point.
    Consistently use `StorageDepositNotEnoughFunds` (limit not reached) and `StorageDepositLimitExhausted` (limit reached).
    Origin not being able to pay the existential deposit (ED) for a new account is now `StorageDepositNotEnoughFunds` and traps the caller rather then being a `TransferFailed` return code. Important since we are hiding the ED from contracts, so it should also not be an error code that must be handled.
  - [#8314](https://github.com/paritytech/polkadot-sdk/pull/8314): Add RPCs in the statement store to get the statements and not just the statement data.
    In statement-store, statements can contain a proof with the signature of the statement. This proof is useful to assert that the statement comes from the expected account. This proof also signs for all the statement's fields, which can also be useful information for the receiver.
- Upgrade to Polkadot-SDK `unstable2507` ([polkadot-fellows/runtimes/pull/849](https://github.com/polkadot-fellows/runtimes/pull/849))
  - [#8684](https://github.com/paritytech/polkadot-sdk/pull/8684) Add optional auto-rebag within on-idle to enable incremental correction of account positions within the bags-list during the idle phase of block execution
  - [#8693](https://github.com/paritytech/polkadot-sdk/pull/8693) Add XCM Precompile to pallet-xcm
- [Encointer] use XCM V5 to remotely spend funds from encointer treasury accounts on AHK [polkadot-fellows/runtimes/pull/679](https://github.com/polkadot-fellows/runtimes/pull/679)

### Changed

- Upgrade to Polkadot-SDK `unstable2507` ([polkadot-fellows/runtimes/pull/849](https://github.com/polkadot-fellows/runtimes/pull/849))
  - [#7953](https://github.com/paritytech/polkadot-sdk/pull/7953): Add deposit for setting session keys
    * 🚨 Setting session keys now might charge a storage deposit. The amount can be inspected in the Session::KeyDeposit of the runtime metadata. This value is intended to be set post AHM. Validators should make sure they have some free balance to cover this deposit the next time they want to rotate their keys.
    * Session keys previously could be set only by the associated controller account of a stash. Now, this filter no longer exists, and they can be set by anyone (ergo, the deposit). For validators, please make sure to submit your session keys (henceforth) **from the stash account**.
- Add foreign-consensus cousin Asset Hub as trusted aliaser to allow XCMv5 origin preservation for foreign-consensus parachains [polkadot-fellows/runtimes/pull/794](https://github.com/polkadot-fellows/runtimes/pull/794))
- Configure block providers for pallets requiring block context ([polkadot-fellows/runtimes/pull/813](https://github.com/polkadot-fellows/runtimes/pull/813)):
  - vesting: keep using Relay Chain block provider
  - multisig: switch to local block provider (for unique multisig IDs)
  - proxy: use Relay Chain block provider (for delayed announcements)
  - nfts: use Relay Chain block provider (for minting start/end blocks)
- PolkadotAssetHub: Enable Async Backing ([polkadot-fellows/runtimes/pull/763](https://github.com/polkadot-fellows/runtimes/pull/763))
- Upgrade to Polkadot-SDK `stable2506` ([polkadot-fellows/runtimes/pull/817](https://github.com/polkadot-fellows/runtimes/pull/817))
  - [#9137](https://github.com/paritytech/polkadot-sdk/pull/9137): Pallet XCM - transfer_assets pre-ahm patch
    🚨 Pallet XCM's `transfer_assets` extrinsic now returns an error when it determines that a reserve transfer of DOT|KSM has to be done.
    This is a safeguard in preparation for the Asset Hub Migration (AHM), where the reserve of DOT|KSM will change from the Relay Chain to Asset Hub.
    After the migration, another patch will remove this error case and use the correct reserve.
    🚨 For DOT|KSM cross-chain transfers please use `limited_reserve_transfer_assets` or `transfer_assets_using_type_and_then`.
  - [#8718](https://github.com/paritytech/polkadot-sdk/pull/8718): Contracts: Record ED as part of the storage deposit.
  - [#8554](https://github.com/paritytech/polkadot-sdk/pull/8554): Contracts: pallet-assets ERC20 precompile
  - [#7762](https://github.com/paritytech/polkadot-sdk/pull/7762): Contracts: ERC20 XCM Asset Transactor
    This PR introduces an Asset Transactor for dealing with ERC20 tokens and adds it to Asset Hub Westend.
    This means asset ids of the form `{ parents: 0, interior: X1(AccountKey20 { key, network }) }` will be matched by this transactor and the corresponding transfer function will be called in the smart contract whose address is key.
    If your chain uses pallet-revive, you can support ERC20s as well by adding the transactor, which lives in assets-common.
  - [#8197](https://github.com/paritytech/polkadot-sdk/pull/8197): [pallet-revive] Add `fee_history`
  - [#8148](https://github.com/paritytech/polkadot-sdk/pull/8148): [pallet-revive] eth-rpc refactoring
      - Refactor eth-rpc.
      - Get rid of the in-memory cache; we can just store receipts / logs into sqlite.
      - Track both best and finalized blocks so that we can properly index transactions in case of a Relay Chain re-org.
      - Keep reference to the latest finalized block so that we can use that for queries that use the finalized block tag.
      - Use `--index-last-n-blocks` CLI parameter to re-index the last `n` blocks when the server starts.
      - Fix issue with `gas_price` calculation for EIP1559.
  - [#8545](https://github.com/paritytech/polkadot-sdk/pull/8545): [pallet-revive] eth-rpc improved healthcheck
  - [#8587](https://github.com/paritytech/polkadot-sdk/pull/8587): [pallet-revive] Make subscription task panic on error
  - [#8664](https://github.com/paritytech/polkadot-sdk/pull/8664): [pallet-revive] Fix rpc-types
  - [#8311](https://github.com/paritytech/polkadot-sdk/pull/8311): [pallet-revive] Update tracing RPC methods parameters
    Update `debug_trace*` methods to support extra parameters supported by geth.
    The method now can specify a timeout and whether we should only return a trace for the top call.
  - [#8734](https://github.com/paritytech/polkadot-sdk/pull/8734): [pallet-revive] Contract's nonce starts at 1
  - [#8274](https://github.com/paritytech/polkadot-sdk/pull/8274): [pallet-revive] Add `get_storage_var_key` for variable-sized keys
  - [#8103](https://github.com/paritytech/polkadot-sdk/pull/8103): [pallet-revive] Add genesis config
  - [#8273](https://github.com/paritytech/polkadot-sdk/pull/8273): [pallet-revive] Add net-listening rpc
  - [#8667](https://github.com/paritytech/polkadot-sdk/pull/8667): [pallet-revive] Simplify the storage meter
  - [#7867](https://github.com/paritytech/polkadot-sdk/pull/7867): Make read/write benchmarks more accurate
  - [#8281](https://github.com/paritytech/polkadot-sdk/pull/8281): `XcmPaymentApi::query_weight_to_asset_fee` simple common impl
  - [#8535](https://github.com/paritytech/polkadot-sdk/pull/8535): Make `WeightBounds` return `XcmError` to surface failures
    Improved XCM weight calculation error handling and traceability. The `WeightBounds` trait now returns detailed `XcmError` types instead of opaque results, allowing downstream consumers to access specific error context for failures like instruction decoding issues, weight overflows, and instruction limit violations. Added structured debug logging with contextual information to aid in diagnosing weight estimation failures during message preparation and execution.
  - [#8122](https://github.com/paritytech/polkadot-sdk/pull/8122): Accommodate small changes to unstable V16 metadata format
    🚨 The frame-metadata version is bumped, which leads to a few minor changes to our sp-metadata-ir crate to accommodate small changes in the unstable V16 metadata format.
  - [#8234](https://github.com/paritytech/polkadot-sdk/pull/8234): Set a 16 MiB heap memory limit when decoding an `UncheckedExtrinsic`
  - [#7730](https://github.com/paritytech/polkadot-sdk/pull/7730): Nest errors in pallet-xcm
    To address the issue of vague `LocalExecutionIncomplete` errors in pallet-xcm, the PR introduces `LocalExecutionIncompleteWithError(ExecutionError)`, which nests a compact `ExecutionError` enum—aligned with `XcmError` and excluding strings like in `FailedToTransactAsset`: to provide detailed error information within FRAME's 4-byte limit. This enhances error reporting by specifying causes like insufficient balance or asset transaction failures, with strings logged for debugging.
  - [#7220](https://github.com/paritytech/polkadot-sdk/pull/7220): Yet Another Parachain is introduced, with the main purpose to be a target for the Spammening events, but also to be used like one more general-purpose testing parachain runtime.
  - [#3811](https://github.com/paritytech/polkadot-sdk/pull/3811): Implicit `chill` when full unbonding in pallet-staking.
    Modifies the `unbond` extrinsic to forcefully `chill` stash when unbonding, if the full stake is unbonded.
  - [#8724](https://github.com/paritytech/polkadot-sdk/pull/8724): Implement detailed logging for XCM failures
    Improves diagnostics in XCM-related code by adding detailed error logging, especially within map_err paths. It includes clearer messages, standardized log targets, and richer context to aid runtime developers and node operators in debugging and monitoring.
  - [#7960](https://github.com/paritytech/polkadot-sdk/pull/7960): Stabilize pallet view functions
    Pallet view functions are no longer marked as experimental, and their use is suggested starting from this PR.
  - [#7597](https://github.com/paritytech/polkadot-sdk/pull/7597): Introduce `CreateBare`, deprecated `CreateInherent`
    Rename `CreateInherent` to `CreateBare`, add method `create_bare` and deprecate `create_inherent`.
    Both unsigned transaction and inherent use the extrinsic type `Bare`.
    Before this PR CreateInherent trait was use to generate unsigned transaction, now unsigned transaction can be generated using a proper trait `CreateBare`.
  - [#8599](https://github.com/paritytech/polkadot-sdk/pull/8599): Snowbridge: Unpaid execution when bridging to Ethereum
    In Snowbridge V2, the execution fee on Ethereum is estimated dynamically and injected into the XCM, eliminating the need to preconfigure the bridge fee.
    Additionally, we also aim to avoid maintaining the Asset Hub’s sovereign account on the Bridge Hub.
  - [#8327](https://github.com/paritytech/polkadot-sdk/pull/8327): Update to the latest unstable V16 metadata.
  - [#8038](https://github.com/paritytech/polkadot-sdk/pull/8038): Fix penpal runtime
    Allow using Penpal native asset (PEN) for paying local fees and allow teleporting it from/to AH. Also allow unpaid execution from relay chain for sudo calls.
  - [#8344](https://github.com/paritytech/polkadot-sdk/pull/8344): XCMP weight metering: account for the MQ page position
  - [#8021](https://github.com/paritytech/polkadot-sdk/pull/8021): XCMP: use batching when enqueuing inbound messages
    This PR implements batching for the XCMP inbound enqueueing logic, which leads to an about ~75x performance improvement for that specific code.
  - [#9202](https://github.com/paritytech/polkadot-sdk/pull/9202): `apply_authorized_force_set_current_code` does not need to consume the whole block
- Proxy type `NonTranfer`: Use a whitelist of calls and remove some not useful calls from the whitelist ([polkadot-fellows/runtimes/pull/646](https://github.com/polkadot-fellows/runtimes/pull/646))
- Add Snowbridge V2 pallets, to enable Snowbridge V2 bridging: [polkadot-fellows/runtimes/pull/796](https://github.com/polkadot-fellows/runtimes/pull/796))
- Moves single block migrations from frame_executive::Executive to frame_system::Config. [polkadot-fellows/runtimes/pull/844](https://github.com/polkadot-fellows/runtimes/pull/844)

## [1.6.1] 24.06.2025

### Changed

- Slash and disable lazy and spammy validators as part of the new validator disabling strategy ([SDK #6827](https://github.com/paritytech/polkadot-sdk/pull/6827), [polkadot-fellows/runtimes/pull/782](https://github.com/polkadot-fellows/runtimes/pull/782))
- Switch to UpToLimitWithReEnablingDisablingStrategy (Polkadot & Kusama) which always prioritises highest offenders for disabling instead of stopping when limit is reached ([polkadot-fellows/runtimes/pull/781](https://github.com/polkadot-fellows/runtimes/pull/781))
- Snowbridge: Remove `snowbridge-pallet-system::NativeToForeignId` which is unused. ([#730](https://github.com/polkadot-fellows/runtimes/pull/730))

## [1.6.0] 19.06.2025

### Added

- Bump ParachainHost runtime API version to 13 for polkadot and kusama ([polkadot-fellows/runtimes/pull/768](https://github.com/polkadot-fellows/runtimes/pull/768))
- Update to SDK version `stable2503-6` ([polkadot-fellows/runtimes/pull/762](https://github.com/polkadot-fellows/runtimes/pull/762))
- Update to SDK version `stable2503-5` ([polkadot-fellows/runtimes/pull/711](https://github.com/polkadot-fellows/runtimes/pull/711))
  - [[#711](https://github.com/polkadot-fellows/runtimes/pull/711)] Add missing events to nomination pool extrinsics ([SDK stable2503 #7377](https://github.com/paritytech/polkadot-sdk/pull/7377)).
  - [[#711](https://github.com/polkadot-fellows/runtimes/pull/711)] Add view functions to Proxy pallet for runtime-specific type configuration ([SDK stable2503 #7320](https://github.com/paritytech/polkadot-sdk/pull/7320)).
  - [[#711](https://github.com/polkadot-fellows/runtimes/pull/711)] Core-fellowship: Add permissionless import_member ([SDK stable2503 #7030](https://github.com/paritytech/polkadot-sdk/pull/7030)).
  - [[#711](https://github.com/polkadot-fellows/runtimes/pull/711)] Pallet-broker: add extrinsic to remove a lease ([SDK stable2503 #7026](https://github.com/paritytech/polkadot-sdk/pull/7026)).
  - [[#711](https://github.com/polkadot-fellows/runtimes/pull/711)] Pallet-broker: add extrinsic to remove an assignment ([SDK stable2503 #7080](https://github.com/paritytech/polkadot-sdk/pull/7080)).
  - [[#711](https://github.com/polkadot-fellows/runtimes/pull/711)] Pallet-broker: add extrinsic to reserve a system core without having to wait two sale boundaries ([SDK stable2503 #4273](https://github.com/paritytech/polkadot-sdk/pull/4273)).
- [[#755](https://github.com/polkadot-fellows/runtimes/pull/755)] Added `pallet_revive` to Kusama AssetHub.

### Changed

- Update to SDK version `stable2503-5` ([polkadot-fellows/runtimes/pull/711](https://github.com/polkadot-fellows/runtimes/pull/711))
  - [[#711](https://github.com/polkadot-fellows/runtimes/pull/711)] Alter semantic meaning of 0 in metering limits of EVM contract calls ([SDK stable2503 #6890](https://github.com/paritytech/polkadot-sdk/pull/6890)).
  - [[#711](https://github.com/polkadot-fellows/runtimes/pull/711)] `apply_authorized_upgrade`: Remote authorization if the version check fails ([SDK stable2503 #7812](https://github.com/paritytech/polkadot-sdk/pull/7812)).
  - [[#711](https://github.com/polkadot-fellows/runtimes/pull/711)] `CheckOnlySudoAccount`: Provide some tags ([SDK stable2503 #7838](https://github.com/paritytech/polkadot-sdk/pull/7838)).
  - [[#711](https://github.com/polkadot-fellows/runtimes/pull/711)] Currency to Fungible migration for pallet-staking ([SDK stable2503 #5501](https://github.com/paritytech/polkadot-sdk/pull/5501)).
  - [[#711](https://github.com/polkadot-fellows/runtimes/pull/711)] Enable report_fork_voting() ([SDK stable2503 #6856](https://github.com/paritytech/polkadot-sdk/pull/6856)).
  - [[#711](https://github.com/polkadot-fellows/runtimes/pull/711)] Implement pallet view functions ([SDK stable2503 #4722](https://github.com/paritytech/polkadot-sdk/pull/4722)).
  - [[#711](https://github.com/polkadot-fellows/runtimes/pull/711)] On-demand credits ([SDK stable2503 #5990](https://github.com/paritytech/polkadot-sdk/pull/5990)).
  - [[#711](https://github.com/polkadot-fellows/runtimes/pull/711)] Only allow apply slash to be executed if the slash amount is atleast ED ([SDK stable2503 #6540](https://github.com/paritytech/polkadot-sdk/pull/6540)).
  - [[#711](https://github.com/polkadot-fellows/runtimes/pull/711)] Paras-registrar: Improve error reporting ([SDK stable2503 #6989](https://github.com/paritytech/polkadot-sdk/pull/6989)).
  - [[#711](https://github.com/polkadot-fellows/runtimes/pull/711)] Xcm: convert properly assets in xcmpayment apis ([SDK stable2503 #7134](https://github.com/paritytech/polkadot-sdk/pull/7134)).
  - [[#711](https://github.com/polkadot-fellows/runtimes/pull/711)] Ensure Consistent Topic IDs for Traceable Cross-Chain XCM ([SDK stable2503 #7691](https://github.com/paritytech/polkadot-sdk/pull/7691)).

- [[#753](https://github.com/polkadot-fellows/runtimes/pull/753)] Upgrades Polkadot and Kusama AssetHub to XCM v5. Adds a migration to check upgrade safety.
- [[#754](https://github.com/polkadot-fellows/runtimes/pull/754)]  Change to minimum price controller and configure minimum price of 10 DOT and 1 KSM for Coretime sales. Existing renewals will also be adjusted accordingly and are now no longer completely decoupled from the market. For details on this, please checkout [RFC-149](https://polkadot-fellows.github.io/RFCs/new/0149-rfc-1-renewal-adjustment.html).

- Extend bounty update period to ~10 years ([polkadot-fellows/runtimes/pull/766](https://github.com/polkadot-fellows/runtimes/pull/766))

### Fixed

- Update to SDK version `stable2503-5` ([polkadot-fellows/runtimes/pull/711](https://github.com/polkadot-fellows/runtimes/pull/711))
  - [[#711](https://github.com/polkadot-fellows/runtimes/pull/711)] Xcm: minor fix for compatibility with V4 ([SDK stable2503 #6503](https://github.com/paritytech/polkadot-sdk/pull/6503)).
- Allow `Utility` and `Multisig` calls from `CancelProxy` proxy types in Polkadot/Kusama relaychain runtimes ([polkadot-fellows/runtimes#740](https://github.com/polkadot-fellows/runtimes/pull/740))

## [1.5.1] 22.05.2025

### Fixed

- Enabled XCM instructions `ExchangeAsset` and `AliasOrigin` on the system parachains ([polkadot-fellows/runtimes/pull/700](https://github.com/polkadot-fellows/runtimes/pull/700))
- Correct weights for pallet xcm's `transfer_asset` extrinsic for multiple chains ([polkadot-fellows/runtimes#673](https://github.com/polkadot-fellows/runtimes/pull/673))
- Snowbridge: Update transfer token gas and fee ([polkadot-fellows/runtimes#721](https://github.com/polkadot-fellows/runtimes/pull/721))
- Update to SDK version `stable2412-6` ([polkadot-fellows/runtimes#712](https://github.com/polkadot-fellows/runtimes/pull/712))
  - [stable2412-6 changelog here](https://github.com/paritytech/polkadot-sdk/releases/tag/polkadot-stable2412-6)

### Added

- The Secretary Program ([polkadot-fellows/runtimes#347](https://github.com/polkadot-fellows/runtimes/pull/347))

## [1.5.0] 22.04.2025

### Added

- Now each system extension has its own weight, defined by `ExtensionWeightInfo` ([polkadot-fellows/runtimes/pull/606](https://github.com/polkadot-fellows/runtimes/pull/606))
- Parachains define the default `CoreSelector` strategy, according to [`RFC-0103`](https://polkadot-fellows.github.io/RFCs/approved/0103-introduce-core-index-commitment.html) ([polkadot-fellows/runtimes/pull/606](https://github.com/polkadot-fellows/runtimes/pull/606))
- Update to SDK version `2412-4` ([polkadot-fellows/runtimes/pull/606](https://github.com/polkadot-fellows/runtimes/pull/606))
  - Added XCM v5 ([paritytech/polkadot-sdk/pull/4826](https://github.com/paritytech/polkadot-sdk/pull/4826))
  - Added Trusted Query API calls ([paritytech/polkadot-sdk/pull/6039](https://github.com/paritytech/polkadot-sdk/pull/6039))
  - Bounties Pallet: add approve_bounty_with_curator call ([paritytech/polkadot-sdk/pull/5961](https://github.com/paritytech/polkadot-sdk/pull/5961))
  - Collective: Dynamic deposit based on number of proposals ([paritytech/polkadot-sdk/pull/3151](https://github.com/paritytech/polkadot-sdk/pull/3151))
  - New runtime api that returns the associated pool accounts with a nomination pool ([paritytech/polkadot-sdk/pull/6357](https://github.com/paritytech/polkadot-sdk/pull/6357))
  - Enable RFC103 on Kusama ([polkadot-fellows/runtimes/pull/681](https://github.com/polkadot-fellows/runtimes/pull/681/))

### Changed

- Update to SDK version `2412-2` ([polkadot-fellows/runtimes/pull/606](https://github.com/polkadot-fellows/runtimes/pull/606))
  - Changed from `SignedExtension` to `TransactionExtension` ([paritytech/polkadot-sdk/pull/3685](https://github.com/paritytech/polkadot-sdk/pull/3685))
  - Identity: Decouple usernames from identities ([https://github.com/paritytech/polkadot-sdk/pull/5554](https://github.com/paritytech/polkadot-sdk/pull/5554))
  - Staking: page information to staking::PayoutStarted event ([paritytech/polkadot-sdk/pull/5984](https://github.com/paritytech/polkadot-sdk/pull/5984))
  - Balances: fix: do not emit Issued { amount: 0 } event ([paritytech/polkadot-sdk/pull/5946](https://github.com/paritytech/polkadot-sdk/pull/5946))
  - Snowbridge: Support bridging native ETH ([paritytech/polkadot-sdk/pull/7090](https://github.com/paritytech/polkadot-sdk/pull/7090))
  - Runtime-APIs: Fix DryRunApi client-facing XCM versions ([paritytech/polkadot-sdk/pull/7689](https://github.com/paritytech/polkadot-sdk/pull/7689))
- Kusama: disable/filter `Nis` and `NisCounterpartBalances` pallets calls ([polkadot-fellows/runtimes/pull/656](https://github.com/polkadot-fellows/runtimes/pull/656))
- Increase spend payout period for treasuries from 30 to 90 days to provide sufficient time to address issues with insufficient balance of a specific asset in the treasury pot ([polkadot-fellows/runtimes/pull/647](https://github.com/polkadot-fellows/runtimes/pull/647))
- Asset Hub: remove XCM sufficient asset fee trader ([polkadot-fellows/runtimes#502](https://github.com/polkadot-fellows/runtimes/pull/502))
- Enable Async Backing for Kusama Asset Hub ([polkadot-fellows/runtimes/pull/659](https://github.com/polkadot-fellows/runtimes/pull/659))

## [1.4.3] 14.04.2025

### Changed

- Apply patch for stable2409-6 ([polkadot-fellows/runtimes/pull/623](https://github.com/polkadot-fellows/runtimes/pull/623))
- Disable MBM migrations for all runtimes for check-migrations CI ([polkadot-fellows/runtimes/pull/590](https://github.com/polkadot-fellows/runtimes/pull/590))
- chain-spec-generator supports conditional building (`--no-default-features --features <runtime>` or `--no-default-features --features all-runtimes` or
  `--no-default-features --features all-polkadot` or `--no-default-features --features all-kusama`)([polkadot-fellows/runtimes/pull/637](https://github.com/polkadot-fellows/runtimes/pull/637))

## [1.4.2] 07.03.2025

### Added

- Adds support for remote proxies on AssetHub Polkadot and AssetHub Kusama. ‼️ Builders: Please read the docs and the implications around the lifetime of a proxy on a remote chain. ‼️ ([polkadot-fellows/runtimes#535](https://github.com/polkadot-fellows/runtimes/pull/535))
- Enabled state-trie-migration for Kusama and Polkadot Asset Hubs ([polkadot-fellows/runtimes/pull/604](https://github.com/polkadot-fellows/runtimes/pull/604))

### Fixed

- Correct weights of the scheduler pallet to avoid failing fellowship proposals ([polkadot-fellows/runtimes#614](https://github.com/polkadot-fellows/runtimes/pull/614))

## [1.4.1] 26.02.2025

### Fixed

- Fix an issue related to staking in combination with nomination pools ([polkadot-fellows/runtimes/pull/608](https://github.com/polkadot-fellows/runtimes/pull/608))

## [1.4.0] 07.02.2025

### Fixed

- Fix missing Encointer democracy pallet hook needed for enactment ([polkadot-fellows/runtimes/pull/508](https://github.com/polkadot-fellows/runtimes/pull/508))
- Improve benchmark configuration: fix storage whitelist in benchmarks ([polkadot-fellows/runtimes/pull/525](https://github.com/polkadot-fellows/runtimes/pull/525))
- Coretime chain: allow cross-chain region transfers ([polkadot-fellows/runtimes/pull/483](https://github.com/polkadot-fellows/runtimes/pull/483))
- Unstake the last remaining corrupt ledger ([polkadot-fellows/runtimes/pull/538](https://github.com/polkadot-fellows/runtimes/pull/538))
- Disallow `add_sub` and `set_subs` from `NonTransfer` proxy type in people chain runtimes ([polkadot-fellows/runtimes#518](https://github.com/polkadot-fellows/runtimes/pull/518))
- Added the `XcmRecorder` config item to all runtimes so `local_xcm` can be returned from `DryRunApi` ([polkadot-fellows/runtimes#576](https://github.com/polkadot-fellows/runtimes/pull/576))

### Added

- Asset Hubs: added an AssetExchanger to be able to swap tokens using the xcm executor, even for delivery fees ([polkadot-fellows/runtimes#539](https://github.com/polkadot-fellows/runtimes/pull/539)).
- Location conversion tests for relays and parachains ([polkadot-fellows/runtimes#487](https://github.com/polkadot-fellows/runtimes/pull/487))
- Asset Hubs: XcmPaymentApi now returns all assets in a pool with the native token as acceptable as fee payment ([polkadot-fellows/runtimes#523](https://github.com/polkadot-fellows/runtimes/pull/523))
- ParaRegistration proxy for Polkadot and Kusama ([polkadot-fellows/runtimes#520](https://github.com/polkadot-fellows/runtimes/pull/520))
- Encointer: Swap community currency for KSM from community treasuries subject to democratic decision on allowance ([polkadot-fellows/runtimes#541](https://github.com/polkadot-fellows/runtimes/pull/541))
- Delegate stake pools in Kusama ([polkadot-fellows/runtimes#540](https://github.com/polkadot-fellows/runtimes/pull/540))
- Snowbridge: Add support for bridging Ether ([polkadot-fellows/runtimes#548](https://github.com/polkadot-fellows/runtimes/pull/548))

### Changed

- Kusama Treasury: remove funding to the Kappa Sigma Mu Society and disable burn ([polkadot-fellows/runtimes#507](https://github.com/polkadot-fellows/runtimes/pull/507))
- Kusama Treasury: allow burn parameters to be set via OpenGov ([polkadot-fellows/runtimes#511](https://github.com/polkadot-fellows/runtimes/pull/511))
- Remove Snowbridge create agent and channel extrinsics. ([polkadot-fellows/runtimes#506](https://github.com/polkadot-fellows/runtimes/pull/506))
- Update the XCM `Weigher` from `FixedWeightBounds` to `WeightInfoBounds` with benchmarked weights for Polkadot Collectives ([polkadot-fellows/runtimes#547](https://github.com/polkadot-fellows/runtimes/pull/547))
- Increase max PoV size to 10Mib on Kusama ([polkadot-fellows/runtimes#553](https://github.com/polkadot-fellows/runtimes/pull/553))
- Update to Polkadot SDK `stable2409-4` ([polkadot-fellows/runtimes#558](https://github.com/polkadot-fellows/runtimes/pull/558))
- Asset Hubs: disable vested transfers as preparation for the Asset Hub Migration ([polkadot-fellows/runtime#579](https://github.com/polkadot-fellows/runtimes/pull/579))

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
