# Launching a local network

We provide a script to run a relay chain and all the associated system chains locally. The script requires the following tools avaible in the `$PATH`:

- [zombienet](https://github.com/paritytech/zombienet)
- [podman](https://podman.io/)
- Some relative new Rust version

Launching a local network is done like this:
```sh
./run-local-network.sh NETWORK
```

`NETWORK` needs to be replaced by either `kusama` or `polkadot`. The script will print links to access the chains via `polkadot-js`.
