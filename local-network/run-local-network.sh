#!/usr/bin/env bash

# Ensure the script is run with one argument
if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <polkadot|kusama>"
    exit 1
fi

# Validate the argument
if [[ "$1" != "polkadot" && "$1" != "kusama" ]]; then
    echo "Error: Argument must be either 'polkadot' or 'kusama'."
    exit 1
fi

# Store the argument in a variable
NETWORK="$1"
echo "Selected network: $NETWORK"

# Create a temporary directory
TEMP_DIR=$(mktemp -d)

# Function to clean up temporary directory on exit
cleanup() {
    rm -rf "$TEMP_DIR"
}

# Trap EXIT signal to ensure cleanup is executed when the script ends
trap cleanup EXIT

echo "Building chainspecs.."

cargo run -q -p chain-spec-generator -- $NETWORK-local > $TEMP_DIR/$NETWORK.json
cargo run -q -p chain-spec-generator -- asset-hub-$NETWORK-local > $TEMP_DIR/asset_hub.json
cargo run -q -p chain-spec-generator -- bridge-hub-$NETWORK-local > $TEMP_DIR/bridge_hub.json
cargo run -q -p chain-spec-generator -- people-$NETWORK-local > $TEMP_DIR/people.json
cargo run -q -p chain-spec-generator -- coretime-$NETWORK-local > $TEMP_DIR/coretime.json

if [[ "$NETWORK" = "polkadot" ]]; then
    cargo run -p chain-spec-generator -- collectives-$NETWORK-local > $TEMP_DIR/coretime.json
fi


PROVIDER=podman
ARCH=$(uname -m)
if [[ "$ARCH" != "x86_64" && "$ARCH" != "i386" && "$ARCH" != "i686" ]]; then
    echo "Non-x86 architecture detected ($ARCH), please ensure that \`polkadot\` and \`polkadot-parachain\` is available in \$PATH."
    PROVIDER=native
fi

if [ -n "$FORCE_NATIVE_PROVIDER" ]; then
    PROVIDER=native
fi

export CHAIN_SPEC_PATH=$TEMP_DIR
zombienet spawn -p $PROVIDER local-network-$NETWORK.toml
