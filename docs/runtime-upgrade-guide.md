# Runtime Upgrade Guide

This guide documents the process for submitting runtime upgrades for Kusama and Polkadot networks via OpenGov referenda.

## Prerequisites

### Install opengov-cli

```shell
cargo install --locked --force --git https://github.com/joepetrowski/opengov-cli
```

> **Note**: Even if you already have opengov-cli installed, you'll likely need to update as the chains are upgraded and metadata changes.

Verify installation:
```shell
opengov-cli --help
```

### Understanding the Process

The upgrade process consists of two main steps:
1. **Build the upgrade call:** Generate a batched call that upgrades the relay chain and all system parachains
2. **Submit referenda:** Create the preimages and referenda for governance approval
3. **Apply Authorized Upgrade:** Upload the matching code blobs to the corresponding chains

## Enactment Timing

Aim for approximately two weeks between referendum submission and enactment, targeting Monday/Tuesday/Wednesday at 8:00 UTC. There's some drift on both chains, so it always ends up later, but this ensures engineers are online.
For higher priority upgrades, coordinate with JUST for estimates based on the number of votes they can gather. Expedited upgrades can be enacted in as little as one day.

---

## Upgrade Process

Reference release: https://github.com/polkadot-fellows/runtimes/releases

### Step 1: Build the Upgrade Call

```shell
opengov-cli build-upgrade --network <NETWORK> --relay-version <VERSION>
```

Replace `<NETWORK>` with `kusama` or `polkadot`, and `<VERSION>` with the release version (e.g., `v1.5.1`).

This command will:
- Download runtime WASM blobs from the GitHub release
- Generate `authorize_upgrade` calls for each system parachain
- Output Blake2-256 hashes for verification against srtool
- Create a batched call file at `./upgrade-<network>-<version>/<network>-<version>.call`

### Step 2: Generate Referendum Calls

```shell
opengov-cli submit-referendum \
    --proposal "./upgrade-<network>-<version>/<network>-<version>.call" \
    --network "<NETWORK>" \
    --track "whitelistedcaller"
```

This outputs several transaction links, which can be used to submit the transactions manually via [dev.papi.how](https://dev.papi.how).
Record the **hash** and **length** values from the output - you'll need these for testing.

> **Network difference:** For Polkadot, the Fellowship referendum is submitted on the **Collectives** parachain. For Kusama, it's on the relay chain.

### Step 3: Test with Chopsticks

Fork the network locally to verify the upgrade executes without errors.

**Kusama:**
```shell
npx @acala-network/chopsticks@latest xcm \
    -r kusama \
    -p kusama-asset-hub \
    -p kusama-people \
    -p kusama-coretime \
    -p encointer-kusama \
    -p kusama-bridge-hub
```

**Polkadot:**
```shell
npx @acala-network/chopsticks@latest xcm \
    -r polkadot \
    -p polkadot-collectives \
    -p polkadot-asset-hub \
    -p polkadot-coretime \
    -p polkadot-people \
    -p polkadot-bridge-hub
```

#### Upload Preimages

Upload preimages using the URLs from opengov-cli output:

| Preimage | Kusama | Polkadot |
|----------|--------|----------|
| Public referendum | Relay (port 8005) | Relay (port 8005) |
| Fellowship whitelist | Relay (port 8005) | Collectives (port 8000) |

#### Dispatch the Fellowship Whitelist Call

Open the JS console for the Fellowship chain:
- **Kusama:** [Relay chain console](https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:8005#/js)
- **Polkadot:** [Collectives console](https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:8000#/js)

Inject and execute the whitelist call:

```javascript
const number = (await api.rpc.chain.getHeader()).number.toNumber()

await api.rpc('dev_setStorage', {
  scheduler: {
    agenda: [
      [
        [number + 1], [
          {
            call: {
              Lookup: {
                hash: '<FELLOWSHIP_CALL_HASH>',
                len: <FELLOWSHIP_CALL_LENGTH>
              }
            },
            origin: {
              // Kusama: { Origins: 'Fellows' }
              // Polkadot: { FellowshipOrigins: 'Fellows' }
            }
          }
        ]
      ]
    ]
  }
})

await api.rpc('dev_newBlock', { count: 1 })
```

#### Dispatch the Public Referendum Call

In the [relay chain JS console](https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:8005#/js), dispatch the whitelisted-caller referendum:

```javascript
const number = (await api.rpc.chain.getHeader()).number.toNumber()

await api.rpc('dev_setStorage', {
  scheduler: {
    agenda: [
      [
        [number + 1], [
          {
            call: {
              Lookup: {
                hash: '<PUBLIC_CALL_HASH>',
                len: <PUBLIC_CALL_LENGTH>
              }
            },
            origin: {
              Origins: 'WhitelistedCaller'
            }
          }
        ]
      ]
    ]
  }
})

await api.rpc('dev_newBlock', { count: 1 })
```

#### Verify the Upgrade

1. Check `system -> authorizedUpgrade` in chain state - the hash should match the relay runtime hash from the release
2. Upload the WASM via `system -> applyAuthorizedUpgrade` (submit unsigned)
3. Repeat for system parachains on their respective ports (8000-8004)

### Step 4: Apply the Authorized Upgrade

Once the referenda have passed, submit a `apply_authorized_upgrade` extrinsic with the corresponding code to all to-be-upgraded chains. More documentation is available [here](https://github.com/paritytech/polkadot-sdk/blob/de84d9b5e8542127b03eeefd9ed87b46566509a4/substrate/frame/system/src/lib.rs#L57).
