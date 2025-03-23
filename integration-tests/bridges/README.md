# Bridges Tests for Local Polkadot <> Kusama Bridge

This folder contains zombienet based integration test for both onchain and offchain bridges code.
The tests are designed to be run manually.

To start a test, you need to:

- download latest [zombienet release](https://github.com/paritytech/zombienet/releases) to  `~/local_bridge_testing/bin/zombienet`.
- build Polkadot binaries by running commands in the [`polkadot-sdk`](https://github.com/paritytech/polkadot-sdk) repository clone:
  ```
  cargo build -p polkadot --release
  cargo build --bin polkadot-prepare-worker --release
  cargo build --bin polkadot-execute-worker --release
  ```
  Copy the binaries to:
  ```
  ~/local_bridge_testing/bin/polkadot
  ~/local_bridge_testing/bin/polkadot-prepare-worker
  ~/local_bridge_testing/bin/polkadot-execute-worker
  ```
- build Polkadot Parachain binary by running `cargo build -p polkadot-parachain-bin --release` command in the
[`polkadot-sdk`](https://github.com/paritytech/polkadot-sdk) repository clone. Copy the binary to `~/local_bridge_testing/bin/polkadot-parachain`.
- ensure that you have [`node`](https://nodejs.org/en) installed. Additionally, we'll need globally installed
`polkadot/api-cli` / `polkadot/api` packages (use `yarn global add @polkadot/api-cli` to install it).
- build Substrate relay by running `cargo build -p substrate-relay --release` command in the
[`parity-bridges-common`](https://github.com/paritytech/parity-bridges-common) repository clone. Copy the binary to `~/local_bridge_testing/bin/substrate-relay`. 
- build chain spec generator:
  - (you can use the current branch, or you can build generators from different branches, such as from specific tags or releases)
  - add the `sudo` pallet to the Polkadot and Kusama runtimes and give sudo rights to Alice, e.g. by running `git apply ./integration-tests/bridges/sudo-relay.patch` from the fellows root dir.
  - with this change build the chain spec generator by running `cargo build --release -p chain-spec-generator --no-default-features --features fast-runtime,polkadot,kusama,bridge-hub-kusama,bridge-hub-polkadot,asset-hub-kusama,asset-hub-polkadot`
command.
    - Copy the binary to `~/local_bridge_testing/bin/chain-spec-generator-kusama`.
    - Copy the binary to `~/local_bridge_testing/bin/chain-spec-generator-polkadot`.
- check/change the `POLKADOT_BINARY` and `POLKADOT_PARACHAIN_BINARY` paths (and ensure that the nearby variables
have correct values) in the `./run-test.sh`.

After that, you can run `./run-tests.sh <test_name>` command.
E.g. `./run-test.sh 0001-polkadot-kusama-asset-transfer`.
or
E.g. `FRAMEWORK_REPO_PATH=/home/username/polkadot-sdk ./run-test.sh 0001-polkadot-kusama-asset-transfer`.
