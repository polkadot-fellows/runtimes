# Account migration

Move all the tokens at this stage. Every account is withdrawn and split by destination: reserves go to the Coretime chain as holds, free balance is teleported to Asset Hub.

The migration ends with zero DOT on the Relay Chain. A few accounts are not withdrawn here because a later stage handles them; they are listed below with that stage.

## What is on the Relay Chain

Each row is one case this stage handles; the rule is filled in as the case is implemented. `tests::migrate_one_account_per_case` runs one account per row.

### Balances

Every deposit on the Relay Chain is an unnamed reserve, so an account's reserve does not say which deposits it is made of. When the stage starts, the deposits the pallets record are indexed per account (`ExpectedReserves`), and each account's reserve is attributed in the order of the rows below. A reserve that covers less than the records goes to the deposits that continue first.

| Case | Rule |
|------|------|
| Free balance | Burned here, teleported to Asset Hub as free balance of the same account. |
| Registrar deposit, reserved on the para manager (`paras_registrar::Paras`). A record can say more than the manager has reserved. | Re-established on the Coretime chain as a `RegistrarDeposit` hold, up to what is reserved. A working buffer of free balance (`CtFreeBuffer`) follows it, so the manager can pay fees there; a remainder below the existential deposit follows too instead of being teleported. |
| HRMP channel deposit, reserved on the para sovereign (`hrmp::HrmpChannels`) | Re-established on the Coretime chain as an `HrmpDeposit` hold, under the para's sibling account, with the same buffer and rounding as a registrar deposit. A channel with a pending close request is returning its deposits anyway: they are refunded, so they leave as free balance. |
| HRMP open-channel request deposit, reserved on the sender and, once accepted, the recipient (`hrmp::HrmpOpenChannelRequests`) | Same as a channel deposit. The recipient's is reserved at the active configuration's value and recorded only when the next session turns the request into a channel, so it is indexed at that value. |
| Proxy deposit, reserved on the delegator; a pure proxy's deposit is reserved on the account that created it (`pallet_proxy::Proxies`) | A set with at least one permission that has a meaning on the Coretime chain (the runtime decides which, with `TryFrom<ProxyType> for PortableProxyType`) continues there as a `ProxyDeposit` hold, buffer and rounding as above. A set with none is refunded: it leaves as free balance, reported with `DepositRefunded`. A pure proxy's deposit is indexed under its creator: the delegator never signed, has an `Any` delegate, and its own reserve does not cover it. |
| Proxy announcement deposit, reserved on the announcer (`pallet_proxy::Announcements`) | Announcements are not migrated. Refunded: leaves as free balance, reported with `DepositRefunded`. |
| Multisig operation deposit, reserved on the depositor (`pallet_multisig::Multisigs`) | The operation cannot complete on a retired chain. Refunded: leaves as free balance, reported with `DepositRefunded`. |
| Reserved balance that no deposit record explains | |
| Named holds: preimage deposits, and leftovers from pallets that left the Relay Chain in AHM v1 (`DelegatedStaking`) | |
| Locks and freezes | |

### Accounts with a rule of their own

| Case | Rule |
|------|------|
| Para sovereign accounts, `para…` | Translated to the para's sibling account, `sibl…`, its address on Asset Hub and the Coretime chain. Otherwise like any account: deposits and the buffer to the Coretime chain, free balance to Asset Hub. |
| Sovereign accounts of system paras | Translated like any para. Asset Hub's own lands on an account nothing there controls; it is to go to the sweep beneficiary instead (`TODO(ahm-v2)` in `destination`). |
| Sibling-format accounts, `sibl…` | Pass through: the same bytes are that para's sovereign on Asset Hub. |
| Pure proxies: nonce 0 and at least one `Any` proxy | The whole balance goes to the Coretime chain as free balance, since the delegate keeps control only where the definitions are recreated. |
| Accounts another pallet references through a consumer reference (validators' session keys) | Drained to a zero-balance shell: the record stays, the balance leaves like any other account's. Reported with `AccountShellDrained`. |
| Module accounts, `modl…` | Not withdrawn here. Sweep empties the pots. |
| Accounts below the existential deposit | Not withdrawn here. Sweep Dust reaps them. |
| The migration manager | Not withdrawn here. It pays for the calls that drive the migration and is reaped at Cool Off. |

Issuance that no account holds is not an account; TI Correction burns it.

## On the Coretime chain

`receive_accounts` mints each account and places its holds through the fungible APIs, each account in a transaction of its own. An account that fails is rolled back and parked in `FailedAccounts`; the rest of the batch continues. `CtMintedTotal` counts what was minted, to reconcile against what the Relay Chain burned.
