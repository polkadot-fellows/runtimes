# Account migration

Every account is withdrawn and split by destination: deposits go to the Coretime chain as holds, free balance is teleported to Asset Hub. The migration ends with zero DOT on the Relay Chain; the few accounts a later stage handles are listed below with that stage.

## What is on the Relay Chain

One row per case; `tests::migrate_one_account_per_case` runs one account per row.

### Balances

Every deposit on the Relay Chain is an unnamed reserve. When the stage starts, the deposits the pallets record are indexed per account (`ExpectedReserves`), and each account's reserve is attributed in the order of the rows below; a reserve that covers less than the records goes to the deposits that continue first.

| Case | Rule |
|------|------|
| Free balance | Burned here, teleported to Asset Hub as free balance of the same account. |
| Registrar deposit, reserved on the para manager (`paras_registrar::Paras`). A record can say more than the manager has reserved. | A `RegistrarDeposit` hold on the Coretime chain, up to what is reserved. A buffer of free balance (`CtFreeBuffer`) goes with it, so the manager can pay fees there; a remainder below the existential deposit follows too instead of being teleported. |
| HRMP channel deposit, reserved on the para sovereign (`hrmp::HrmpChannels`) | An `HrmpDeposit` hold under the para's sibling account, with the same buffer and rounding. A channel with a pending close request is returning its deposits anyway: refunded, they leave as free balance. |
| HRMP open-channel request deposit, reserved on the sender and, once accepted, the recipient (`hrmp::HrmpOpenChannelRequests`) | Same as a channel deposit. The recipient's is reserved at the active configuration's value and only recorded once the channel opens, so it is indexed at that value. |
| Proxy deposit, reserved on the delegator; a pure proxy's deposit is reserved on the account that created it (`pallet_proxy::Proxies`) | A set with at least one permission that has a meaning on the Coretime chain (the runtime decides which, with `TryFrom<ProxyType> for PortableProxyType`) continues there as a `ProxyDeposit` hold, buffer and rounding as above. A set with none is refunded: it leaves as free balance, reported with `DepositRefunded`. A pure proxy's deposit is indexed under its creator, the `Any` delegate of a delegator that never signed. |
| Proxy announcement deposit, reserved on the announcer (`pallet_proxy::Announcements`) | Announcements are not migrated. Refunded, reported with `DepositRefunded`. |
| Multisig operation deposit, reserved on the depositor (`pallet_multisig::Multisigs`) | The operation cannot complete on a retired chain. Refunded, reported with `DepositRefunded`. |
| Reserved balance that no deposit record explains | An `UnattributedReserve` hold on the Coretime chain, parked there for governance to look at. Reported with `UnattributedReserve`. On Polkadot this is about 600 DOT, nearly all on the system paras' sovereigns. |
| Named holds: preimage deposits, and leftovers from pallets that left the Relay Chain in AHM v1 (`DelegatedStaking`) | Preimage deposits are released when the stage starts, through the preimage pallet: nothing dispatches a preimage here after the migration. An unrequested blob is removed, a requested one stays. Any other named hold is released when the account is withdrawn. Both leave as free balance. |
| Locks and freezes | None are expected: the pallets that set them left in AHM v1. A leftover is released when the account is withdrawn and leaves as free balance. |

### Accounts with a rule of their own

| Case | Rule |
|------|------|
| Para sovereign accounts, `para…` | Translated to the para's sibling account, `sibl…`, its address on Asset Hub and the Coretime chain. Otherwise like any account: deposits and the buffer to the Coretime chain, free balance to Asset Hub. |
| Sovereign accounts of system paras | Translated like any para. Asset Hub's own lands on an account nothing there controls; it is to go to the sweep beneficiary instead (`TODO(ahm-v2)` in `destination`). |
| Sibling-format accounts, `sibl…` | Pass through: the same bytes are that para's sovereign on Asset Hub. |
| Pure proxies: never signed, and have an `Any` delegate | The whole balance goes to the Coretime chain as free balance: the delegate keeps control only where the definitions are recreated. |
| Accounts another pallet references (validators' session keys) | Drained to a zero-balance shell: the record stays, the balance leaves like any other. Reported with `AccountShellDrained`. |
| The runtime's `SweptAccounts`: its pots | Not withdrawn here. Sweep empties them. |
| Accounts below the existential deposit | Not withdrawn here. Sweep Dust reaps them. |
| The migration manager | Not withdrawn here. It pays for the calls that drive the migration and is reaped at Cool Off. |

Issuance that no account holds is not an account; TI Correction burns it.

An account that cannot be withdrawn is left as it was and reported with `AccountNotWithdrawn`; a rewind to `AccountsInit` retries it.

## On the Coretime chain

`receive_accounts` mints each account and places its holds, each account in a transaction of its own. An account that fails is rolled back and parked in `FailedAccounts`; the rest of the batch continues. `CtMintedTotal` counts what was minted, to reconcile against what the Relay Chain burned.
