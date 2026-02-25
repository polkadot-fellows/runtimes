#!/usr/bin/env bash
# Dry-run a proposal via the scheduler on a chopsticks-forked chain.
#
# Prerequisites:
#   - Start chopsticks (single or xcm mode) before running this script.
#   - npm install (from dry-run/ directory)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

if ! command -v node &>/dev/null; then
  echo "Error: node is required. Install Node.js or run inside nix-shell -p nodejs" >&2
  exit 1
fi

if [ ! -d "$SCRIPT_DIR/node_modules/@polkadot/api" ]; then
  echo "Error: @polkadot/api not found. Run 'npm install' from the dry-run/ directory first." >&2
  exit 1
fi

exec node "$SCRIPT_DIR/proposal.mjs" "$@"
