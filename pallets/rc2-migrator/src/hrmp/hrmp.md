# HRMP migration

HRMP stays on the Relay Chain; after the migration only its deposits are held elsewhere, on the Coretime chain by `pallet-hrmp-para`. This stage sends every deposit the Relay Chain records, one record per channel or open request with what each side pays, so that `pallet-hrmp-para` holds what the records say. The balance itself travelled with the accounts stage, as an `HrmpDeposit` hold on the para's sibling account.

The `HrmpOngoing` blocks walk the open requests, then the channels, from a cursor (`HrmpKey`).

| Case | Sent |
|------|------|
| Open request (`HrmpOpenChannelRequests`) | The sender's recorded deposit, and once confirmed the recipient's: the configured value, unless a system para is involved. |
| Open channel (`HrmpChannels`) | Each side's recorded deposit; a system para's is zero. |
| Channel with a pending close request, or where neither side pays | Nothing: the accounts stage refunded the former's deposits. |

The amounts are the ones the accounts stage indexed the reserves with, so the `HrmpDeposit` holds cover them.

## Session boundaries

`hrmp` opens, closes and cleans up channels at session boundaries only, such operation must not happen during the migration. The warm-up should close any operation and wait for one session boundary for the remaining operations to settle. If an operation goes through and a boundary falls in the data stages anyway, governance can reconcile the Coretime chain against the Relay Chain's records with `pallet-hrmp-para::force_set_deposit`.

## On the Coretime chain

`receive_hrmp_deposits` takes each record in its own transaction and, for each side that pays: releases up to its amount from the para's `HrmpDeposit` hold, then `pallet-hrmp-para` holds what it needs, or as much as the para can cover, and what didn't get hold again goes back under the `HrmpDeposit` hold. A failure is rolled back and logged, the batch continues; governance can reconcile as above. Since a side is set rather than added to, a deposit delivered twice, after a rewind, is correctly held once.
