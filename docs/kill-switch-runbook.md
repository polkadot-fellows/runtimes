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

Paused calls are blocked for signed extrinsics and for XCM `Transact`. These paths keep working
during a pause:

- EVM contracts calling precompiles
- incoming XCM that deposits or withdraws assets
- swaps used to pay transaction fees in a pool asset
- transfers of pool LP tokens (`PoolAssets`)

If the problem can be reached through one of them, a pause alone won't stop it. Escalate to Root
right away.

## Who can do what

- **Anyone:** `pause` and `extend`, each with a deposit.
- **Fellowship:** `unpause` (refund or slash) and `force_extend` on paused-for pauses, and
  `settle` (refund or slash) on paused-for pauses that ran out.
- **Root:** everything the Fellowship can do, plus `force_pause`, which needs no deposit and can
  have no end. Only Root can end or move a forced pause.

All values (deposits, pause length, cooldown) are parameters only Root can set. The kill switch
ships with every deposit at 0, which means only Root can pause until a referendum sets them.

## Expected reaction times

| Who                     | Step                                                       | Expected time |
| ----------------------- | ---------------------------------------------------------- | ------------- |
| anyone                  | spots the issue and pauses                                  | minutes       |
| W3F / Parity            | notice and keep it going with their own `extend` deposits   | ~12h          |
| Fellowship              | vote to extend or unpause                                   | ~24h          |
| Fellowship / Parity     | fix released and enacted on-chain                           | ~72h (to check) |

So a real bug can need a pause of about 3 to 4 days end to end, plus buffer. Pause length,
extension costs and limits should be tuned so a pause can last that long without one person
having to fund all of it.

## When a pause happens

1. **Check it.** Look at the `Paused` event: who paused, what, and which deposit. Find the
   issue they're reacting to and confirm the paused calls are the right ones.
2. **Real bug.** Keep it paused. Extend with `force_extend` before it runs out, or take it over
   with a Root `force_pause` if the fix will take long. Prepare the fix.
   A Root `force_pause` takeover refunds the deposits behind the pause right away. If the pause
   instead runs out or gets unpaused by the fix, **put the refund (`settle` or `unpause` without
   slash) in the same proposal as the fix**, so nobody has to remember it later.
3. **False positive.** `unpause` with a refund. The deposits come back right away.
4. **Malicious or griefing.** `unpause` with a slash. The slashed deposit goes to the DAP, and
   the target and each of its calls go on cooldown.
5. **Not sure yet.** Let it run out. The calls work again, but the deposits stay locked and
   slashable until someone calls `settle(id, slash)`, with the id from the `Expired` event. There's
   no deadline, so take the time to look into it.

## Fellowship commitments

> Proposed. To be agreed by the whole Fellowship through a referendum on the Members track, with
> a remark pointing at this version of the text. Until then, this is only a proposal.

When something on Asset Hub Polkadot is paused through the kill switch, the Fellowship commits
to the following.

**Look at it within 12 hours.** Someone in the Fellowship checks what was paused and why, says
so in the Fellowship channel, and links the issue being reacted to.

**Decide within 24 hours.** There is a Fellowship vote to extend, unpause with a refund, or
unpause with a slash. If it's not clear yet whether the pause was honest, the vote can be to
extend and look further. Nobody should be rushed into a slash.

**Get the fix enacted on-chain within 72 hours** if the pause was for a real bug. What counts is
the fix being live, not just voted on. The proposal that enacts the fix also refunds the deposits
behind the pause. The 72 hours is a working number, to be checked against how long past fix
referenda actually took before this goes up for a vote.

These are targets, not guarantees. If one is missed, the fallback is a Root referendum. The
times W3F and Parity need to start extending with their own deposits (about 12h) are not the
Fellowship's to promise.
They're in the table above as assumptions until W3F and Parity confirm them.

## Tips

Whoever puts up a deposit locks a lot of funds and takes the slash risk, and an honest pause only
gets them their own funds back. Anyone can open a Treasury tip for a pause that protected the
chain. Whether to pay one is up to OpenGov. It isn't a Fellowship commitment.

## Planned changes

Still being discussed on [#1344](https://github.com/polkadot-fellows/runtimes/pull/1344):

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
Parity are expected to pay for the extensions with their own deposits. The first pauser's
deposit stays locked until the pause is settled.

This runbook will be updated once these land.
