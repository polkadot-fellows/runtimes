#!/usr/bin/env bash

set -eu

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/env"

sudo apt update
sudo apt install --assume-yes openssl pkg-config g++ make cmake protobuf-compiler libssl-dev libclang-dev libudev-dev git lz4

# Free space on the runner
df -h
sudo apt -y autoremove --purge
sudo apt -y autoclean
sudo rm -rf /usr/share/dotnet
sudo rm -rf /opt/ghc
sudo rm -rf "/usr/local/share/boost"
sudo rm -rf "$AGENT_TOOLSDIRECTORY"
df -h

# Install solc
mkdir -p solc
if [[ -x solc/solc ]]; then
  echo "solc already present, skipping download"
else
  curl -Lsf --show-error --retry 5 --retry-all-errors --connect-timeout 10 --max-time 300 \
    --output solc/solc \
    "https://github.com/ethereum/solidity/releases/download/v${SOLC_VERSION}/${SOLC_NAME}"
  chmod +x solc/solc
fi

# Install resolc
mkdir -p resolc
if [[ -x resolc/resolc ]]; then
  echo "resolc already present, skipping download"
else
  curl -Lsf --show-error --retry 5 --retry-all-errors --connect-timeout 10 --max-time 300 \
    --output resolc/resolc \
    "https://github.com/paritytech/revive/releases/download/v${RESOLC_VERSION}/resolc-x86_64-unknown-linux-musl"
  chmod +x resolc/resolc
  ./resolc/resolc --version
fi

echo "$PWD/solc" >> "$GITHUB_PATH"
echo "$PWD/resolc" >> "$GITHUB_PATH"

