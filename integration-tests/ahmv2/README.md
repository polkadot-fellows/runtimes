# AHM v2 migration integration tests

Rust dry-run harness for the AHM v2 migration: moving the remaining Relay Chain state (balances
with holds/reserves, registrar, HRMP, …) to the Coretime chain and Asset Hub.

The Relay Chain and the Coretime chain are loaded from `try-runtime` snapshots of real network
state. Blocks are produced by calling hooks directly and DMP/UMP messages are shuttled between the
chains by hand -- i.e. without nodes and networking. Each migration PR extends `tests.rs` with the
scenarios it introduces.

Asset Hub is not loaded. The registrar and HRMP legs of the migration do not touch it, and it only
ever receives a teleport, so its (multi-GB) snapshot buys nothing until the balances leg lands.
Adding it back is one `Para` impl.

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
you delete them, so development does not re-create them per run. CI instead reuses the snapshot
cache that the `Check Migrations` workflow refreshes daily.

To run against specific snapshot files, bypass the justfile:

```bash
SNAP_RC=... SNAP_CT=... cargo test -p polkadot-integration-tests-ahmv2
```

Snapshots are cached in memory per test process and re-hydrated per test, so each test gets fresh
externalities without reloading from disk.

Every produced block asserts that no `MessageQueue::Processed { success: false }` event was emitted
and that consumed weight stays below 80% of the block limit. Every shuttled message is decoded
against the receiving runtime's `RuntimeCall` (catches encode/decode drift between the chains).
