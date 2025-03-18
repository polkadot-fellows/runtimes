# Launching a local network

We provide a script to run a relay chain and all the associated system chains locally. The script requires the following tools avaible in the `$PATH`:

- [zombie-cli](https://crates.io/crates/zombie-cli)
- [podman](https://podman.io/) or [docker](https://www.docker.com/products/docker-desktop/)
- Some relative new Rust version
- At non-x86 systems: `polkadot` and `polkadot-parachain`

Launching a local network is done like this:
```sh
./run-local-network.sh NETWORK
```

`NETWORK` needs to be replaced by either `kusama` or `polkadot`. The script will print links to access the chains via `polkadot-js`.
