# AHM v2 migration integration tests

Rust dry-run harness for the AHM v2 migration: moving account, proxy, registrar and HRMP state
from the Relay Chain to the Coretime chain.

The Relay Chain and the Coretime chain are loaded from `try-runtime` snapshots of real network
state. Blocks are produced by calling hooks directly and DMP/UMP messages are shuttled between the
chains by hand (no nodes or networking). 

## The `ahm-v2` feature

The migrator pallets are wired into the runtimes behind each runtime's `ahm-v2` feature, which
this crate enables on the runtimes it selects. The runtimes gate them on
`all(feature = "ahm-v2", not(feature = "on-chain-release-build"))`, so a release build drops the
pallets even when `ahm-v2` is on.

## Running

```bash
cd integration-tests/ahmv2
just test                 # Polkadot
NETWORK=kusama just test  # same suite against Kusama
```

That is all: it creates any missing snapshot and runs the test suite. Extra arguments are passed
through to `cargo test` (e.g. `just test rc_and_coretime`). Snapshot creation needs the
[try-runtime CLI](https://github.com/paritytech/try-runtime-cli).

Snapshots land in `snapshots/<network>/` (gitignored) and are kept until you delete them, so runs
do not re-create them. `SNAP_DIR` overrides the parent directory: point it at a tree that already
holds `<network>/snap_rc.snap` and `<network>/snap_ct.snap`. The default endpoints are public;
override them with `RC_URI` / `CT_URI` to scrape from your own nodes.

To run against specific snapshot files, bypass the justfile. The network is a cargo feature,
`polkadot` (default) or `kusama`:

```bash
SNAP_RC=... SNAP_CT=... cargo test -p polkadot-integration-tests-ahmv2
SNAP_RC=... SNAP_CT=... cargo test -p polkadot-integration-tests-ahmv2 \
  --no-default-features --features kusama
```

Snapshots are cached in memory per test process and re-hydrated per test, so each test gets fresh
externalities without reloading from disk.

Every produced block asserts that no `MessageQueue::Processed { success: false }` event was emitted
and that consumed weight stays below 80% of the block limit. Every shuttled message is decoded
against the receiving runtime's `RuntimeCall` (catches encode/decode drift between the chains).
