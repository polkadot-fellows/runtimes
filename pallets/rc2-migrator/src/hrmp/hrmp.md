# HRMP migration

HRMP stays on the Relay Chain; after the migration only its deposits are held elsewhere, on the Coretime chain by `pallet-hrmp-para`. This stage sends every deposit the Relay Chain records, as a `(channel, side, amount)`, so that `pallet-hrmp-para` holds what the records say. The balance itself travelled with the accounts stage, as an `HrmpDeposit` hold on the para's sibling account; the records here are read, not changed.

| Case | Sent |
|------|------|
| Open channel (`HrmpChannels`) | Both sides' deposits, by the `HrmpOngoing` blocks. |
| Channel with a pending close request | Nothing: the accounts stage refunded its deposits. |
| Open request (`HrmpOpenChannelRequests`) | The sender's deposit, and the recipient's (the configured value, unless a system para is involved) once accepted, by the `HrmpInit` block. |
| A side that pays nothing (system channels) | Nothing. |

The amounts are the ones the accounts stage indexed the reserves with, so the `HrmpDeposit` holds cover them.

## Session boundaries

`hrmp` opens, closes and cleans up channels at session boundaries only, and nothing can be asked of it once the migration has started. Schedule the warm-up to end right after a boundary (`schedule_migration`), so the data stages run within one session and the records do not change under them. If a boundary falls in the data stages anyway, governance reconciles the Coretime chain against the Relay Chain's records with `pallet-hrmp-para::force_set_deposit`: zero for a channel closed at the boundary (its deposits were refunded), the recorded amounts for a channel opened at it.

## On the Coretime chain

`receive_hrmp_deposits` takes each deposit in its own transaction: releases up to its amount from the para's `HrmpDeposit` hold, then `pallet-hrmp-para` holds or releases so that the side holds the amount, or as much as the para can cover. A failure is rolled back and logged, the batch continues; governance reconciles as above. Since a side is set rather than added to, a deposit delivered twice, after a rewind, is held once.
