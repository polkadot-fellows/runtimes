# Account migration

Move all the tokens at this stage. Every account is withdrawn and split by destination: reserves go to the Coretime chain as holds, free balance is teleported to Asset Hub.

The migration ends with zero DOT on the Relay Chain. A few accounts are not withdrawn here because a later stage handles them; they are listed below with that stage.

## What is on the Relay Chain

Each row is one case this stage handles; the rule is filled in as the case is implemented. `tests::migrate_one_account_per_case` runs one account per row.

### Balances

| Case | Rule |
|------|------|
| Free balance | Burned here, teleported to Asset Hub as free balance of the same account. |
| Registrar deposit, reserved on the para manager (`paras_registrar::Paras`). A record can say more than the manager has reserved. | |
| HRMP channel deposit, reserved on the para sovereign (`hrmp::HrmpChannels`) | |
| HRMP open-channel request deposit, reserved on the sender and, once accepted, the recipient (`hrmp::HrmpOpenChannelRequests`) | |
| Proxy deposit, reserved on the delegator; a pure proxy's deposit is reserved on the account that created it (`pallet_proxy::Proxies`) | |
| Proxy announcement deposit, reserved on the announcer (`pallet_proxy::Announcements`) | |
| Multisig operation deposit, reserved on the depositor (`pallet_multisig::Multisigs`) | |
| Reserved balance that no deposit record explains | |
| Named holds: preimage deposits, and leftovers from pallets that left the Relay Chain in AHM v1 (`DelegatedStaking`) | |
| Locks and freezes | |

### Accounts with a rule of their own

| Case | Rule |
|------|------|
| Para sovereign accounts, `para…` | |
| Sovereign accounts of system paras | |
| Sibling-format accounts, `sibl…` | |
| Pure proxies: nonce 0 and at least one `Any` proxy | |
| Accounts another pallet references through a consumer reference (validators' session keys) | |
| Module accounts, `modl…` | Not withdrawn here. Sweep empties the pots. |
| Accounts below the existential deposit | Not withdrawn here. Sweep Dust reaps them. |
| The migration manager | Not withdrawn here. It pays for the calls that drive the migration and is reaped at Cool Off. |

Issuance that no account holds is not an account; TI Correction burns it.
