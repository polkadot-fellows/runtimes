#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 2 ]; then
    echo "Usage: $0 <wasm-file> <max-mib>" >&2
    exit 2
fi

WASM="$1"
LIMIT_MIB="$2"

if ! [[ "$LIMIT_MIB" =~ ^[0-9]+$ ]]; then
    echo "Error: <max-mib> must be a positive integer, got: $LIMIT_MIB" >&2
    exit 2
fi

if [ ! -f "$WASM" ]; then
    echo "Error: file not found: $WASM" >&2
    exit 2
fi

LIMIT_BYTES=$((LIMIT_MIB * 1024 * 1024))

# Substrate's compressed-blob magic prefix (8 bytes) precedes the zstd payload.
ZSTD_PREFIX_HEX="52bc537646db8e05"

COMPRESSED_BYTES=$(wc -c < "$WASM")
FILE_PREFIX_HEX=$(head -c 8 "$WASM" | od -An -tx1 | tr -d ' \n')

if [ "$FILE_PREFIX_HEX" = "$ZSTD_PREFIX_HEX" ]; then
    if ! command -v zstd >/dev/null 2>&1; then
        echo "Error: 'zstd' is required to decompress the runtime blob but is not installed." >&2
        echo "Install it with e.g. 'sudo apt install -y zstd' on the CI runner." >&2
        exit 2
    fi
    UNCOMPRESSED_BYTES=$(tail -c +9 "$WASM" | zstd -d --stdout --no-progress 2>/dev/null | wc -c)
    STATE="zstd-compressed"
else
    # The file does not carry the substrate compression prefix; treat the raw
    # file size as the uncompressed size.
    UNCOMPRESSED_BYTES="$COMPRESSED_BYTES"
    STATE="not compressed"
fi

human() {
    numfmt --to=iec-i --suffix=B --format="%.2f" "$1"
}

USED_PCT=$(awk -v u="$UNCOMPRESSED_BYTES" -v l="$LIMIT_BYTES" 'BEGIN { printf "%.2f", (u / l) * 100 }')

echo "File:              $WASM"
echo "On-disk size:      $COMPRESSED_BYTES bytes ($(human "$COMPRESSED_BYTES"), $STATE)"
echo "Uncompressed size: $UNCOMPRESSED_BYTES bytes ($(human "$UNCOMPRESSED_BYTES"))"
echo "Limit:             $LIMIT_BYTES bytes ($LIMIT_MIB MiB)"
echo "Usage:             ${USED_PCT}% of limit"

if [ "$UNCOMPRESSED_BYTES" -gt "$LIMIT_BYTES" ]; then
    OVER=$((UNCOMPRESSED_BYTES - LIMIT_BYTES))
    echo "::error file=$WASM::Uncompressed runtime size ($(human "$UNCOMPRESSED_BYTES")) exceeds the ${LIMIT_MIB} MiB limit by $(human "$OVER"). Nodes will reject this blob during decompression (compression bomb check)."
    exit 1
fi

echo "OK: uncompressed runtime size is within the ${LIMIT_MIB} MiB limit."
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
