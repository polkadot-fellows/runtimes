#!/bin/bash

bridged_chain=$1

# Add Alice as bridge owner
# We do this only if there is a `.genesis.runtimeGenesis.patch` object.
# Otherwise we're working with the raw chain spec.
chain-spec-builder --chain-spec-path /dev/stdout create \
  -n "Polkadot Bridge Hub Local" -i bridge-hub-polkadot-local -t local \
  -r "${BRIDGE_HUB_POLKADOT_WASM}" \
  --relay-chain polkadot-local -p 1002 \
  named-preset local_testnet \
  | jq 'if .genesis.runtimeGenesis.patch
    then .genesis.runtimeGenesis.patch.bridge'$bridged_chain'Grandpa.owner = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"
    else .
    end'
