#!/bin/bash
case "$1" in
  polkadot-local)
    chain-spec-builder --chain-spec-path /dev/stdout create \
      -n "Polkadot Local Testnet" -i polkadot-local -t local \
      -r "${POLKADOT_WASM}" \
      named-preset local_testnet
    ;;
  asset-hub-polkadot-local)
    chain-spec-builder --chain-spec-path /dev/stdout create \
      -n "Polkadot Asset Hub Local" -i asset-hub-polkadot-local -t local \
      -r "${ASSET_HUB_POLKADOT_WASM}" \
      --relay-chain polkadot-local -p 1000 \
      named-preset local_testnet
    ;;
  bridge-hub-polkadot-local)
    chain-spec-builder --chain-spec-path /dev/stdout create \
      -n "Polkadot Bridge Hub Local" -i bridge-hub-polkadot-local -t local \
      -r "${BRIDGE_HUB_POLKADOT_WASM}" \
      --relay-chain polkadot-local -p 1002 \
      named-preset local_testnet
    ;;
  *)
    echo "chain: $1 not supported" >&2
    exit 1
    ;;
esac
