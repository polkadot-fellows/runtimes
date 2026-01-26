#!/usr/bin/env bash

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
mkdir -p "$RUNNER_TEMP/solc"
if [[ -x "$RUNNER_TEMP/solc/solc" ]]; then
  echo "solc already present, skipping download"
else
  curl -Lsf --show-error --retry 5 --retry-all-errors --connect-timeout 10 --max-time 300 \
    --output "$RUNNER_TEMP/solc/solc" \
    "https://github.com/ethereum/solidity/releases/download/v${SOLC_VERSION}/${SOLC_NAME}"
  chmod +x "$RUNNER_TEMP/solc/solc"
fi

# Install resolc
mkdir -p "$RUNNER_TEMP/resolc"
if [[ -x "$RUNNER_TEMP/resolc/resolc" ]]; then
  echo "resolc already present, skipping download"
else
  curl -Lsf --show-error --retry 5 --retry-all-errors --connect-timeout 10 --max-time 300 \
    --output "$RUNNER_TEMP/resolc/resolc" \
    "https://github.com/paritytech/revive/releases/download/v${RESOLC_VERSION}/resolc-x86_64-unknown-linux-musl"
  chmod +x "$RUNNER_TEMP/resolc/resolc"
  "$RUNNER_TEMP/resolc/resolc" --version
fi

echo "$RUNNER_TEMP/solc" >> "$GITHUB_PATH"
echo "$RUNNER_TEMP/resolc" >> "$GITHUB_PATH"

