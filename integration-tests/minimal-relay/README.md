# Minimal Relay migration integration tests

Rust dry-run harness for the Minimal Relay migration: moving the remaining Relay Chain state
(balances with holds/reserves, registrar, HRMP, …) to the Coretime chain and Asset Hub.

Three chains -- RC, CT and AH -- are loaded from `try-runtime` snapshots of real network state.
Blocks are produced by calling hooks directly and DMP/UMP messages are shuttled between the
chains by hand -- i.e. without nodes and networking. Each migration PR extends `tests.rs` with
the scenarios it introduces.

This is the successor of the AHM v1 harness (`integration-tests/ahm` on the
`dev-asset-hub-migration` branch), extended from two chains to three.

## Running

```bash
cd integration-tests/minimal-relay
just test                 # Polkadot
NETWORK=kusama just test  # same suite against Kusama
```

That is all: it downloads any missing snapshot and runs the test suite. Extra arguments are
passed through to `cargo test` (e.g. `just test rc_and_coretime`). `just test-fast` skips
everything that needs the multi-GB Asset Hub snapshot. See `just --list` for all recipes.

Snapshots land in `snapshots/<network>/` (gitignored; override with `SNAP_DIR`) and are kept
until you delete them or re-run `just snapshots` -- freshness rarely matters during development,
so don't re-download per run. They come from the daily `Check Migrations` CI artifacts of this
repo (needs an authenticated `gh` CLI); `just snapshots-from-rpc` scrapes public RPC nodes
instead, which is slow for Asset Hub (~4 GB).

To run against specific snapshot files, bypass the justfile:

```bash
SNAP_RC=... SNAP_AH=... SNAP_CT=... cargo test -p polkadot-integration-tests-minimal-relay
```

Snapshots are cached in memory per test process and re-hydrated per test, so each test gets fresh
externalities without reloading from disk.

Every produced block asserts that no `MessageQueue::Processed { success: false }` event was
emitted and that consumed weight stays below 80% of the block limit. Every shuttled message is
decoded against the receiving runtime's `RuntimeCall` (catches encode/decode drift between the
chains).
