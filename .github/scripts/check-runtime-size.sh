#!/usr/bin/env bash
# Verify a runtime's compressed WASM size against its configured limit.
# Warns at 80% of the limit and fails the job at 95% to block the release.
set -euo pipefail

RUNTIME="${1:?runtime name required}"
JSON="${2:?srtool json path required}"
MAX_SIZE="${3:?max code size required}"
WARN_PCT=80
ERR_PCT=95

SIZE=$(jq -r '.runtimes.compressed.subwasm.size // empty' "$JSON")
if [[ -z "$SIZE" ]]; then
  echo "::error title=Runtime size check::could not read compressed WASM size for ${RUNTIME} from ${JSON}"
  exit 1
fi

PCT=$(awk -v s="$SIZE" -v m="$MAX_SIZE" 'BEGIN { printf "%.1f", (s / m) * 100 }')

if (( SIZE * 100 > MAX_SIZE * ERR_PCT )); then
  echo "::error title=Runtime WASM oversize::${RUNTIME} compressed WASM is ${SIZE} bytes (${PCT}% of max ${MAX_SIZE}); exceeds ${ERR_PCT}% error threshold, blocking the release"
  exit 1
elif (( SIZE * 100 > MAX_SIZE * WARN_PCT )); then
  echo "::warning title=Runtime WASM nearing limit::${RUNTIME} compressed WASM is ${SIZE} bytes (${PCT}% of max ${MAX_SIZE}); exceeds ${WARN_PCT}% warning threshold"
else
  echo "${RUNTIME}: ${SIZE} bytes (${PCT}% of max ${MAX_SIZE}) - within limits"
fi
