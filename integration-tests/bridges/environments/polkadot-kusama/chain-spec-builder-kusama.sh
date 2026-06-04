#!/bin/bash
case "$1" in
  kusama-local)
    chain-spec-builder --chain-spec-path /dev/stdout create \
      -n "Kusama Local Testnet" -i kusama-local -t local \
      -r "${KUSAMA_WASM}" \
      named-preset local_testnet
    ;;
  asset-hub-kusama-local)
    chain-spec-builder --chain-spec-path /dev/stdout create \
      -n "Kusama Asset Hub Local" -i asset-hub-kusama-local -t local \
      -r "${ASSET_HUB_KUSAMA_WASM}" \
      --relay-chain kusama-local -p 1000 \
      named-preset local_testnet
    ;;
  bridge-hub-kusama-local)
    chain-spec-builder --chain-spec-path /dev/stdout create \
      -n "Kusama Bridge Hub Local" -i bridge-hub-kusama-local -t local \
      -r "${BRIDGE_HUB_KUSAMA_WASM}" \
      --relay-chain kusama-local -p 1002 \
      named-preset local_testnet
    ;;
  *)
    echo "chain: $1 not supported" >&2
    exit 1
    ;;
esac
