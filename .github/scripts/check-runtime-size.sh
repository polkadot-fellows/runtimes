#!/usr/bin/env bash
set -euo pipefail

RUNTIME="${1:?runtime name required}"
JSON="${2:?srtool json path required}"
OUT="${3:?output status path required}"
MAX_SIZE="${4:-5242880}"
WARN_PCT="${5:-80}"
ERR_PCT="${6:-95}"

if [[ ! -f "$JSON" ]]; then
  echo "::error title=Runtime size check::srtool json not found for $RUNTIME: $JSON"
  jq -n --arg runtime "$RUNTIME" --argjson max_size "$MAX_SIZE" \
    '{runtime: $runtime, size: 0, max_size: $max_size, pct: 0, level: "missing"}' > "$OUT"
  exit 0
fi

SIZE=$(jq -r '.runtimes.compressed.subwasm.size // empty' "$JSON")
if [[ -z "$SIZE" ]]; then
  echo "::error title=Runtime size check::Could not read .runtimes.compressed.subwasm.size from $JSON for $RUNTIME"
  jq -n --arg runtime "$RUNTIME" --argjson max_size "$MAX_SIZE" \
    '{runtime: $runtime, size: 0, max_size: $max_size, pct: 0, level: "missing"}' > "$OUT"
  exit 0
fi

WARN_BYTES=$(( MAX_SIZE * WARN_PCT / 100 ))
ERR_BYTES=$(( MAX_SIZE * ERR_PCT / 100 ))
PCT=$(awk -v s="$SIZE" -v m="$MAX_SIZE" 'BEGIN { printf "%.2f", (s/m)*100 }')

if (( SIZE > ERR_BYTES )); then
  LEVEL="error"
  echo "::error title=Runtime WASM oversize::${RUNTIME} compressed WASM is ${SIZE} bytes (${PCT}% of max ${MAX_SIZE}); error threshold ${ERR_PCT}% exceeded"
elif (( SIZE > WARN_BYTES )); then
  LEVEL="warning"
  echo "::warning title=Runtime WASM nearing limit::${RUNTIME} compressed WASM is ${SIZE} bytes (${PCT}% of max ${MAX_SIZE}); warning threshold ${WARN_PCT}% exceeded"
else
  LEVEL="ok"
  echo "${RUNTIME}: ${SIZE} bytes (${PCT}% of max ${MAX_SIZE}) - within limits"
fi

jq -n \
  --arg runtime "$RUNTIME" \
  --argjson size "$SIZE" \
  --argjson max_size "$MAX_SIZE" \
  --arg pct "$PCT" \
  --arg level "$LEVEL" \
  '{runtime: $runtime, size: $size, max_size: $max_size, pct: ($pct|tonumber), level: $level}' \
  > "$OUT"

if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
  {
    echo "**${RUNTIME}**: ${SIZE} bytes (${PCT}% of max ${MAX_SIZE}) - **${LEVEL}**"
  } >> "$GITHUB_STEP_SUMMARY"
fi
