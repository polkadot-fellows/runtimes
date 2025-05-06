# Account Migration

Accounts are migrated with all their balance, locks and reserves at the beginning of the Asset Hub
migration.

## User Impact

Users need to be aware that all of their funds will be moved from the Relay chain to the Asset Hub.
The Account ID will stay the same. This ensures that normal user accounts will be to control their
funds on Asset Hub.

- 🚨 All funds will be **moved** from the Relay Chain to the Asset Hub.
- 🚨 Account IDs of parachain sovereign accounts will be translated from their Relay child to their sibling parachain account.
- The Account ID of normal accounts will stay the same.

## Sovereign Account Translation

For parachain sovereign accounts, it is not possible to just use the same account ID. The sovereign
account address of a parachain is calculated differently, depending on whether it is the account on
the Relay or a parachain (like Asset Hub).  

There are different kinds of sovereign accounts. In this context, we only focus on these parachain
sovereign accounts:
- On the Relay: derived from `"para" ++ para_id ++ 00..`
- On the Asset Hub and all other sibling parachains: derived from `"sibl" ++ para_id ++ 00..`

Our translation logic inverts the derivation and changes the prefix from `"para"` to `"sibl"` for
all accounts that match the pattern `"para" ++ para_id ++ 00..`. The full list of translated
accounts is in [this CSV file](./sovereign_account_translation.csv).

It is advised that parachains check that they can control their account on Asset Hub. They can also
forego this check if they do not need control thereof - for example when they are not holding any
funds on their relay sovereign account. However, please note that someone could still send funds to
that address before or after the migration.

Example for Bifrost: this is the [relay sovereign account](https://polkadot.subscan.io/account/13YMK2eeopZtUNpeHnJ1Ws2HqMQG6Ts9PGCZYGyFbSYoZfcm) and it gets translated to this [sibling sovereign account](https://assethub-polkadot.subscan.io/account/13cKp89TtYknbyYnqnF6dWN75q5ZosvFSuqzoEVkUAaNR47A).

## XCM

The migration happens over XCM. There will be events emitted for the balance being removed from the
Relay Chain and events emitted for the balance being deposited into Asset Hub.

### Provider and Consumer References

After inspecting the state, it’s clear that fully correcting all reference counts is nearly
impossible. Some accounts have over `10` provider references, which are difficult to trace and
reason about. To unwind all of them properly, we would need to analyze the codebase and state
history, which is not feasible.

Before an account is fully withdrawn from the Relay Chain (RC), we will force-update its consumer
and provider references to ensure it can be completely removed. If an account is intended to remain
(fully or partially) on RC, we will update the references accordingly.

To ensure the correct provider and consumer reference counts are established on the Asset Hub (AH),
we inspect the migrating pallets and reallocate the references on AH based on their logic. The
existential deposit (ED) provider reference and hold/freeze consumer references will be
automatically restored, since we use the fungible implementation to reallocate holds/freezes, rather
than manipulating state directly.

Below is a list of known sources of provider and consumer references, with notes on how they are
handled.

Pallets Increasing Provider References (Polkadot / Kusama / Westend):

- delegate_staking (P/K/W): One extra provider reference should be migrated to AH for every account
with the hold reason `pallet_delegated_staking::HoldReason::StakingDelegation`. This ensures the
entire balance, including the ED, can be staked via holds.
Source: https://github.com/paritytech/polkadot-sdk/blob/ab1e12ab6f6c3946c3c61b97328702e719cd1223/substrate/frame/delegated-staking/src/types.rs#L81

- parachains_on_demand (P/K/W): The on-demand pallet pot account should not be migrated to AH and
will remain untouched.
Source: https://github.com/paritytech/polkadot-sdk/blob/ace62f120fbc9ec617d6bab0a5180f0be4441537/polkadot/runtime/parachains/src/on_demand/mod.rs#L407

- crowdloan (P/K/W): The provider reference for a crowdloan fund account allows it to exist without
an ED until funding is received. Since new crowdloans can no longer be created, and only successful
ones are being migrated, we don’t expect any new fund accounts below ED. This reference can be
ignored.
Source: https://github.com/paritytech/polkadot-sdk/blob/9abe25d974f6045d1e97537e0f1e860459053722/polkadot/runtime/common/src/crowdloan/mod.rs#L417

- balances (P/K/W): No special handling is needed, as this is covered by the fungible implementation
during injection on AH.
Source: https://github.com/paritytech/polkadot-sdk/blob/9abe25d974f6045d1e97537e0f1e860459053722/substrate/frame/balances/src/lib.rs#L1035

- session (P/K/W): Validator accounts may receive a provider reference at genesis if they did not
previously exist. This is not relevant for migration. Even if a validator is fully reaped during
migration, they can restore their account by teleporting funds to RC post-migration.
Source: https://github.com/paritytech/polkadot-sdk/blob/8d4138f77106a6af49920ad84f3283f696f3f905/substrate/frame/session/src/lib.rs#L462-L465

- broker (//_): Not relevant for RC and AH runtimes.

Pallets Increasing Consumer References (Polkadot / Kusama / Westend):

- balances (P/K/W): No custom handling is required, as this is covered by the fungible
implementation during account injection on AH.
Source: https://github.com/paritytech/polkadot-sdk/blob/9abe25d974f6045d1e97537e0f1e860459053722/substrate/frame/balances/src/lib.rs#L1035

- recovery (/K/W): A consumer reference is added to the proxy account when it claims an already
initiated recovery process. This reference is later removed when the recovery process ends. For
simplicity, we can ignore this consumer reference, as it might affect only a small number of
accounts, and a decrease without a prior increase will not cause any issues.
See test: `polkadot_integration_tests_ahm::tests::test_account_references`
Source: https://github.com/paritytech/polkadot-sdk/blob/ace62f120fbc9ec617d6bab0a5180f0be4441537/substrate/frame/recovery/src/lib.rs#L610

- session (P/K/W): Validator accounts may be removed from RC during migration (unless they maintain
HRMP channels or register a parachain). Validators who later wish to interact with the session
pallet (e.g., set/remove keys) will need to teleport funds to RC and reinitialize their account. The
only possible inconsistency is if a validator removes already existing keys, causing the consumer
count to decrement from 0 (if no holds/freezes) or from 1 otherwise. Case 1: From 0 — no issue.
Case 2: From 1 — results in a temporarily incorrect consumer count, which will self-correct on any
account update.
See test: `polkadot_integration_tests_ahm::tests::test_account_references`
Source: https://github.com/paritytech/polkadot-sdk/blob/ace62f120fbc9ec617d6bab0a5180f0be4441537/substrate/frame/session/src/lib.rs#L812

- staking (P/K/W): No references are migrated in the new staking pallet version; legacy references are not relevant. TODO: confirm with @Ank4n

- assets, contracts, nfts, uniques, revive (//): Not relevant for RC and AH runtimes.


### XCM "Checking Account" and DOT/KSM Total Issuance tracking

The Relay Chain is currently the "native location" of DOT/KSM, and it is responsible for keeping
track of the token's **total issuance** (across the entire ecosystem).  
The Relay Chain uses a special "checking" account to track oubound and inbound teleports of the
native token (DOT/KSM), and through this account balance, track all "exported" DOT/KSM. Summing
that with the total balance of all other local accounts provides the token's *total issuance*.  
On top of that, the checking account is also used to enforce that the amount of DOT/KSM teleported
"back in" cannot surpass the amount teleported "out".

During AHM, we move all of the above responsibilities from the Relay Chain to Asset Hub. AH will be
the source of truth for DOT/KSM *total issuance*.

#### Migration design assumptions

1. AHM has no implications for the other System Chains' checking accounts - only Relay and AH.
2. Migration of checking account balance falls under base case of generic account migration: no
   special handling when "moving funds" of this account: balance of checking account on Relay will
   simply be migrated over to the same checking account on Asset Hub using same logic and part of
   the same process as all other accounts.
3. Not **all** DOT/KSM will be moved from Relay Chain to AH. Some of it will stay put for various
   reasons covered throughout this doc.
4. The new balance of checking account on AH will be adjusted in a dedicated migration step to
   properly account for the "exported" tokens. The logic for this calculation is described further
   down.
5. To avoid having to coordinate/track DOT/KSM teleports across all System Chains during AHM,
   DOT/KSM teleports shall be disabled for Relay Chain and Asset Hub during accounts migration.

| DOT Teleports (in or out) | **Relay** | **AH** |
|----------|-----|--------|
| _Before_ | Yes | Yes |
| _During_ | No  | No  |
| _After_  | Yes | Yes |


The runtime configurations of Relay and AH need to change in terms of how they use the checking
account before, during and after the migration.

|     DOT Teleports tracking in Checking Account    | **Relay** | **AH** |
|----------|-----------|--------|
| _Before_ |     Yes, MintLocal       |   No Checking     |
| _During_ |     No Checking       |   No Checking     |
| _After_  |     No Checking      |   Yes, MintLocal     |

#### Tracking Total Issuance post-migration

Pre-migration RC checking account tracks total DOT/KSM that "left" RC and is currently on some other
system chain. The DOT/KSM in various accounts on AH is also tracked in this same RC checking account.

Post-migration, we want the tracking to move to AH. So AH checking account will track total DOT/KSM
currrently living on RC or other system chains.

The **important invariant** here is that the DOT/KSM **total issuance** reported by RC pre-migration
matches the total issuance reported by AH post-migration.

To achieve this, we implement the followint arithmetic algorithm:

After all accounts (including checking account) are migrated from RC to AH:

```
	ah_checking_intermediate = ah_check_before + rc_check_before
	(0) rc_check_before = ah_checking_intermediate - ah_check_before

	Invariants:
	(1) rc_check_before = sum_total_before(ah, bh, collectives, coretime, people)
	(2) rc_check_before = sum_total_before(bh, collectives, coretime, people) + ah_total_before

	Because teleports are disabled for RC and AH during migration, we can say:
	(3) sum_total_before(bh, collectives, coretime, people) = sum_total_after(bh, collectives, coretime, people)

	Ergo use (3) in (2):
	(4) rc_check_before = sum_total_after(bh, collectives, coretime, people) + ah_total_before

	We want:
		ah_check_after = sum_total_after(rc, bh, collectives, coretime, people)
		ah_check_after = sum_total_after(bh, collectives, coretime, people) + rc_balance_kept
	Use (3):
		ah_check_after = sum_total_before(bh, collectives, coretime, people) + rc_balance_kept
		ah_check_after = sum_total_before(ah, bh, collectives, coretime, people) - ah_total_before + rc_balance_kept
	Use (1):
		ah_check_after = rc_check_before - ah_total_before + rc_balance_kept

	Finally use (0):
		ah_check_after = ah_checking_intermediate - ah_check_before - ah_total_before + rc_balance_kept
```

At which point, `ah_total_issuance_after` should equal `rc_total_issuance_before`.
