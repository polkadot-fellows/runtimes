# Kill switch runbook (Asset Hub Polkadot)

What to do when something on Asset Hub Polkadot gets paused through the kill switch
(`pallet-kill-switch`), who does what, and how long each step is expected to take.

> Draft. The Fellowship and fix times below come from past fix referenda. W3F or Parity funding
> extensions is not agreed, it's being checked with both.

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

If the problem can be reached through one of them, a pause alone won't stop it. Raise it with the
Fellowship right away. Anything only Root can do goes through a Fellowship whitelist and the
Whitelisted Caller track, which takes about 2 days even under pressure (see below).

## Who can do what

- **Anyone:** `pause` and `extend`, each with a deposit.
- **Fellowship:** `unpause` (refund or slash) and `force_extend` on paused-for pauses, and
  `settle` (refund or slash) on paused-for pauses that ran out.
- **Root:** everything the Fellowship can do, plus `force_pause`, which needs no deposit and can
  have no end. Only Root can end or move a forced pause.

All values (deposits, pause length) are parameters only Root can set. The kill switch
ships with every deposit at 0, which means only Root can pause until a referendum sets them.

## Expected reaction times

| Who                     | Step                                                        | Expected time        |
| ----------------------- | ----------------------------------------------------------- | -------------------- |
| anyone                  | spots the issue and pauses                                  | minutes              |
| Fellowship              | vote to extend (`force_extend`, no deposit), unpause or settle | ~1 to 1.5 days    |
| W3F / Parity            | may fund extensions with their own deposits                 | not agreed, being checked |
| Fellowship / Parity     | write and release the fix                                   | depends on the bug   |
| governance              | fix release to live on-chain                                | ~2 days, 72h target  |

Every runtime fix goes through a Fellowship whitelist (Fellows track on Collectives) and the
Whitelisted Caller track on Polkadot, voted side by side. Getting both up after a release takes
minutes to an hour, and enactment after the votes about ten minutes. The public vote is the
bottleneck, because its support threshold only drops to normal turnout around day four or five,
so a fast fix needs a lot of DOT voting early. A plain Root referendum is far too slow for this:
the only one that passed this year took about a month.

Past urgent fixes, from submission to executed:

| Fix                     | Polkadot (Whitelisted Caller) | Collectives (Fellowship whitelist) |
| ----------------------- | ----------------------------- | ---------------------------------- |
| v2.0.7                  | 46.6h (#1840)                 | 44.8h (#476)                       |
| June fix                | 63.6h (#1914)                 | 33.8h (#569)                       |
| 2.5.1                   | 38.9h (#1960)                 | 14.2h (#665)                       |
| dotUSD supply fix       | 52.4h (#1961)                 | 32.8h (#668)                       |

Routine upgrades take closer to five or six days, since nobody pushes turnout.

So a real bug needs a pause of however long the fix takes to write, plus about 2 to 4 days to get
it live. Pause length, extension costs and limits should be tuned so a pause can last that long
without one person having to fund all of it.

## When a pause happens

1. **Check it.** Look at the `Paused` event: who paused, what, and which deposit. Find the
   issue they're reacting to and confirm the paused calls are the right ones.
2. **Real bug.** Keep it paused. Extend with `force_extend` before it runs out. A Root
   `force_pause` takeover also works, but Root means a whitelist and Whitelisted Caller vote, so
   it's only worth it if it can ride along with the fix. Prepare the fix.
   A Root `force_pause` takeover refunds the deposits behind the pause right away. If the pause
   instead runs out or gets unpaused by the fix, **put the refund (`settle` or `unpause` without
   slash) in the same proposal as the fix**, so nobody has to remember it later.
3. **False positive.** `unpause` with a refund. The deposits come back right away.
4. **Malicious or griefing.** `unpause` with a slash. The slashed deposit goes to the DAP. Anyone
   can pause the same thing again right away, there is no cooldown on purpose: it would let
   someone get slashed on purpose to keep everyone else from pausing a real problem.
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

**Start a vote within 12 hours, decide within 36, 48 at the latest.** There is a Fellowship vote
to extend, unpause with a refund, or unpause with a slash. If it's not clear yet whether the pause
was honest, the vote can be to extend and look further. Nobody should be rushed into a slash.
Past emergency whitelists took 14 to 45 hours, so 24 hours would be missed more often than not.

**Get the fix live on-chain within 72 hours of its release, 96 at the latest,** if the pause was
for a real bug. What counts is the fix being live, not just voted on. The proposal that enacts
the fix also refunds the deposits behind the pause. The clock starts at the release: how long it
takes to write the fix depends on the bug, and nobody can put a number on that.

These are targets, not guarantees. The 96 hours leaves about a day for a slower vote or a missing
large voter.

Honest long pauses are kept going by the Fellowship with `force_extend`, which needs no deposit.
The vote takes about a day, so it has to start well before the pause runs out. Whether W3F or
Parity would also fund extensions with their own deposits isn't agreed, and it's being checked
with both. Nothing here depends on it.

## Tips

Whoever puts up a deposit locks a lot of funds and takes the slash risk, and an honest pause only
gets them their own funds back. Anyone can open a Treasury tip for a pause that protected the
chain. Whether to pay one is up to OpenGov. It isn't a Fellowship commitment.

## Before deposits are turned on

The kill switch ships with every deposit at 0, so only Root can pause. These have to land before
a referendum turns the deposits on. Without them, someone can keep re-pausing after every slash,
paying one deposit per Fellowship vote:

- **A Fellowship fast track for lifting pauses.** A low-threshold track open to all rank 3+, so
  whoever is awake can lift a pause within hours instead of a day or more. Only for lifting and
  extending, not for slashing or pausing. The slash can wait, the deposits stay locked until
  `settle`.
- **Escalating deposits.** The Nth paid period in a row costs `base + (N - 1) * step`, where
  `step` is a percentage of `base`. Every paid pause and extension counts, whoever pays, and so
  does pausing again right after a slash. Switching to an overlapping group doesn't reset it. It
  resets once the calls have been unpaused for two full periods. Since the Fellowship extends
  honest long pauses for free, this only hits whoever keeps paying to extend, so it can be steep.
  It also replaces the cooldown this pallet used to have, which could be used to block pausing.
- **Shorter pause periods** (around 6h) once escalation is in.

Both are follow-ups to [#1344](https://github.com/polkadot-fellows/runtimes/pull/1344).

What someone extending on their own money would pay to keep a 10k group paused with 6h periods:

| step | first 12h | 3 days (12 periods) | 4.5 days (18 periods) |
| ---- | --------- | ------------------- | --------------------- |
| 0%   | 20k       | 120k                | 180k                  |
| 50%  | 25k       | 450k                | 945k                  |
| 100% | 30k       | 780k                | 1.71M                 |

These are deposits, not fees. An honest pause gets them back, so the cost is mostly the lockup.
The first 12h is meant to be affordable for whoever spots the problem. After that, an honest
pause is extended by the Fellowship, so these numbers only matter for someone paying to keep it
going on their own. The first pauser's deposit stays locked until the pause is settled. The step
gets picked together with the fast track.

This runbook will be updated once these land.
