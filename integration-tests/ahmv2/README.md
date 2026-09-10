# AHM v2 migration integration tests

Rust dry-run harness for the AHM v2 migration: moving the remaining Relay Chain state (balances
with holds/reserves, registrar, HRMP, …) to the Coretime chain and Asset Hub.

The Relay Chain and the Coretime chain are loaded from `try-runtime` snapshots of real network
state. Blocks are produced by calling hooks directly and DMP/UMP messages are shuttled between the
chains by hand -- i.e. without nodes and networking. Each migration PR extends `tests.rs` with the
scenarios it introduces.

Asset Hub is not loaded. It has no migrator of its own and only ever receives a teleport, so its
multi-GB snapshot buys nothing until the balances leg lands — at which point it is needed, to
prove the teleports arrived and drained Asset Hub's XCM checking account in lockstep. Adding it is
one `Para` impl.

## The `ahm-v2` feature

The migrator pallets are wired into the runtimes behind each runtime's `ahm-v2` feature, which
this crate enables on every runtime it depends on. The migration's storage layout is still
changing — the stage machine grows a variant per data stage — so the pallets are deliberately
absent from released runtimes, which srtool builds per-package with an explicit feature list.

This is the successor of the AHM v1 harness (`integration-tests/ahm` on the
`dev-asset-hub-migration` branch), which was hardcoded to the Relay Chain and Asset Hub.

## Running

```bash
cd integration-tests/ahmv2
just test                 # Polkadot
NETWORK=kusama just test  # same suite against Kusama
```

That is all: it creates any missing snapshot and runs the test suite. Extra arguments are passed
through to `cargo test` (e.g. `just test rc_and_coretime`). Snapshot creation needs the
[try-runtime CLI](https://github.com/paritytech/try-runtime-cli).

Snapshots land in `snapshots/<network>/` (gitignored; override with `SNAP_DIR`) and are kept until
you delete them, so development does not re-create them per run — point `SNAP_DIR` at snapshots
you already have rather than scraping again. The default endpoints are public; override them with
`RC_URI` / `CT_URI` to scrape from your own nodes.

There is deliberately no CI workflow for this suite: it needs multi-hundred-MB snapshots of live
chains, and while the migration is under construction no pull request can break it. It runs
locally, on demand.

To run against specific snapshot files, bypass the justfile:

```bash
SNAP_RC=... SNAP_CT=... cargo test -p polkadot-integration-tests-ahmv2
```

Snapshots are cached in memory per test process and re-hydrated per test, so each test gets fresh
externalities without reloading from disk.

Every produced block asserts that no `MessageQueue::Processed { success: false }` event was emitted
and that consumed weight stays below 80% of the block limit. Every shuttled message is decoded
against the receiving runtime's `RuntimeCall` (catches encode/decode drift between the chains).
