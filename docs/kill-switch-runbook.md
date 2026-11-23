# Kill switch runbook (Asset Hub Polkadot)

What to do when something on Asset Hub Polkadot gets paused through the kill switch
(`pallet-kill-switch`), who does what, and how long each step is expected to take.

> Draft. The reaction times below are assumptions to tune the parameters against. They still need
> to be confirmed by W3F, Parity and the Fellowship.

## What can be paused

Anyone can pause one of these groups by putting up its deposit:

| Group            | What it stops                                   |
| ---------------- | ----------------------------------------------- |
| `Psm`            | minting and redeeming through the PSM            |
| `Swaps`          | asset conversion swaps and liquidity changes    |
| `AssetTransfers` | transfers of local and foreign assets           |
| `XcmTransfers`   | outgoing XCM transfers                          |

`Swaps`, `AssetTransfers` and `XcmTransfers` also pause local XCM `execute`, since it can do
the same things.

A single call can be paused on its own too. It costs the call deposit, or the deposit of the
priciest group the call is in, whichever is higher.

Some calls can never be paused: governance, the kill switch itself, XCM `send`, and calls that
other system chains drive over XCM. The full list is `never_pausable_pallets()` in
`system-parachains/asset-hubs/asset-hub-polkadot/src/kill_switch.rs`.

Paused calls are blocked for signed extrinsics and for XCM `Transact`. These paths still go
through a pause:

- EVM contracts calling precompiles
- incoming XCM that deposits or withdraws assets
- swaps used to pay transaction fees in a pool asset

If the problem can be reached through one of them, a pause alone won't stop it. Escalate to Root
right away.

## Who can do what

- **Anyone:** `pause` and `extend`, each with a deposit.
- **Fellowship:** `unpause` (refund or slash) and `force_extend` on paused-for pauses.
- **Root:** everything the Fellowship can do, plus `force_pause`, which needs no deposit and can
  have no end. Only Root can end or move a forced pause.

All values (deposits, pause length, cooldown) are parameters only Root can set. The kill switch
ships with every deposit at 0, which means only Root can pause until a referendum sets them.

## Expected reaction times

| Who                     | Step                                                       | Expected time |
| ----------------------- | ---------------------------------------------------------- | ------------- |
| anyone                  | spots the issue and pauses                                  | minutes       |
| W3F / Parity            | notice and take over the deposit for a longer pause         | ~12h          |
| Fellowship              | vote to extend or unpause                                   | ~24h          |
| Fellowship / Parity     | prepare a fix release                                       | 48h to 72h    |
| governance              | enact the fix                                               | ~24h          |

So a real bug can need a pause of about 4 to 5 days end to end, plus buffer. Pause length,
extension costs and limits should be tuned so a pause can last that long without one person
having to fund all of it.

## When a pause happens

1. **Check it.** Look at the `Paused` event: who paused, what, and which deposit. Find the
   issue they're reacting to and confirm the paused calls are the right ones.
2. **Real bug.** Keep it paused. Extend with `force_extend` before it runs out, or take it over
   with a Root `force_pause` if the fix will take long. Prepare the fix.
   **Put the deposit refund in the same proposal as the fix**, so nobody has to remember it later.
3. **False positive.** `unpause` with a refund.
4. **Malicious or griefing.** `unpause` with a slash. The slashed deposit goes to the DAP, and
   the target and each of its calls go on cooldown.
5. **Not sure yet.** Leave it until it's clear. Don't rush a slash or refund call.

## Tips for honest pauses

Pausing the chain to protect it means locking up a large deposit. When a pause turns out to have
protected the chain, the Fellowship will suggest that the Treasury tip the depositor a share of
the deposit they locked, and will put up that Treasury proposal and encourage people to support
it. This isn't done on-chain by the kill switch. It's a commitment on how the Fellowship handles
these cases.

## Planned changes

Still being discussed on [#1344](https://github.com/polkadot-fellows/runtimes/pull/1344):

- Deposits stay held and slashable after a pause ends, instead of being refunded at expiry. The
  Fellowship or Root releases or slashes them later by pause id.
- Escalating deposits for long pauses: the Nth period in a row costs `base + (N - 1) * step`,
  where `step` is a percentage of `base`. Every paid pause and extension counts, whoever pays.
  Switching to an overlapping group doesn't reset it. It resets once the calls have been
  unpaused for two full periods.
- Shorter pause periods (around 6h) once escalation is in.

Proposed cost of keeping a 10k group paused with 6h periods:

| step | first 12h | 3 days (12 periods) | 4.5 days (18 periods) |
| ---- | --------- | ------------------- | --------------------- |
| 0%   | 20k       | 120k                | 180k                  |
| 50%  | 25k       | 450k                | 945k                  |
| 100% | 30k       | 780k                | 1.71M                 |

These are deposits, not fees. An honest pause gets them back, so the cost is mostly the lockup.
The first 12h is meant to be affordable for whoever spots the problem. After that, W3F or
Parity are expected to take over the deposits.

This runbook will be updated once these land.
