# Bridges Tests for Local Polkadot <> Kusama Bridge

This folder contains zombienet based integration test for both onchain and offchain bridges code.
The tests are designed to be run manually.

To start a test, you need to:

- download latest [zombienet release](https://github.com/paritytech/zombienet/releases) to 
`~/local_bridge_testing/bin/zombienet`.

- build Polkadot binary by running `cargo build -p polkadot --release` command in the
[`polkadot-sdk`](https://github.com/paritytech/polkadot-sdk) repository clone.

- build Polkadot Parachain binary by running `cargo build -p polkadot-parachain-bin --release` command in the
[`polkadot-sdk`](https://github.com/paritytech/polkadot-sdk) repository clone.

- ensure that you have [`node`](https://nodejs.org/en) installed. Additionally, we'll need globally installed
`polkadot/api-cli` package (use `yarn global add @polkadot/api-cli` to install it).

- build Substrate relay by running `cargo build -p substrate-relay --release` command in the
[`parity-bridges-common`](https://github.com/paritytech/parity-bridges-common) repository clone. Copy the binary to `~/local_bridge_testing/bin/substrate-relay`.

- add the `sudo` pallet to the Polkadot and Kusama runtimes and give sudo rights to Alice. With this change build 
the chain spec generator by running `cargo build --release -p chain-spec-generator --features fast-runtime` 
command. Copy the binary to `~/local_bridge_testing/bin/chain-spec-generator`.

- change the `POLKADOT_BINARY` and `POLKADOT_PARACHAIN_BINARY` paths (and ensure that the nearby variables
have correct values) in the `./run-test.sh`.

After that, you can run `./run-tests.sh <test_name>` command.
