# Minimal Relay migration integration tests

Rust dry-run harness for the Minimal Relay migration: moving the remaining Relay Chain state
(balances with holds/reserves, registrar, HRMP, …) to the Coretime chain and Asset Hub.

Three chains -- RC, CT and AH -- are loaded from `try-runtime` snapshots of real network state. 
Blocks are produced by calling hooks directly and DMP/UMP messages are shuttled between the 
chains by hand -- i.e. without nodes and networking.

This is the successor of the AHM v1 harness (`integration-tests/ahm` on the
`dev-asset-hub-migration` branch), extended from two chains to three.

## Running

```bash
cd integration-tests/minimal-relay
just test                  # Polkadot; extra args go to cargo test, e.g. `just test rc_and_coretime`
just test-kusama           # same tests bound to the Kusama runtimes
```

Both need the three snapshots present first. `just --list` shows all recipes.

### Getting the snapshots

```bash
just snapshots-from-rpc            # scrapes public RPC nodes: slow (AH is ~3 GB) but always works
just snapshots-from-rpc-kusama
just snapshots                     # tries the CI artifacts; see below, usually there are none
```

Snapshots land in `integration-tests/minimal-relay/snapshots/` (gitignored) as `snap_{rc,ah,ct}.snap`
and `snap_{rc,ah,ct}_ksm.snap`. Set `SNAP_DIR` to keep them elsewhere -- worth doing, so several
checkouts share one copy. Freshness rarely matters during development, so don't re-download per run.

The fellowship CI does snapshot every chain daily, but it stores them as an Actions *cache*, which
nothing outside a workflow run can download. `check-migrations.yml` uploads a snapshot as an
artifact only on a run that missed that cache and is neither a schedule nor a dispatch, with one
day of retention -- so on any given day there is usually nothing to fetch and `just snapshots`
will say so and point you at the RPC scrape. Hosting the snapshots somewhere fetchable, or adding
a workflow that re-uploads the cached ones as long-retention artifacts, is the fix.

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
