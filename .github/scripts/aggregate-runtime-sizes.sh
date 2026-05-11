#!/usr/bin/env bash
set -euo pipefail

STATUS_DIR="${1:?status dir required}"
MATRIX_FILE="${2:-}"

HAS_WARNINGS=false
HAS_ERRORS=false

mapfile -t FILES < <(find "$STATUS_DIR" -type f -name '*.json' 2>/dev/null | sort)

{
  echo "## Runtime WASM size report"
  echo
  echo "| Runtime | Size (bytes) | Max (bytes) | % of max | Status |"
  echo "|---------|-------------:|------------:|---------:|:------:|"
} > full-report.md

{
  echo "## Runtime WASM size threshold breaches"
  echo
  echo "| Runtime | Size (bytes) | Max (bytes) | % of max | Status |"
  echo "|---------|-------------:|------------:|---------:|:------:|"
} > violations.md

declare -A SEEN=()

for f in "${FILES[@]}"; do
  RUNTIME=$(jq -r .runtime "$f")
  SIZE=$(jq -r .size "$f")
  MAX=$(jq -r .max_size "$f")
  PCT=$(jq -r .pct "$f")
  LEVEL=$(jq -r .level "$f")

  SEEN[$RUNTIME]=1
  ROW="| ${RUNTIME} | ${SIZE} | ${MAX} | ${PCT}% | ${LEVEL} |"
  echo "$ROW" >> full-report.md

  case "$LEVEL" in
    warning) HAS_WARNINGS=true; echo "$ROW" >> violations.md ;;
    error)   HAS_ERRORS=true;   echo "$ROW" >> violations.md ;;
    missing) HAS_ERRORS=true;   echo "$ROW" >> violations.md ;;
  esac
done

if [[ -n "$MATRIX_FILE" && -f "$MATRIX_FILE" ]]; then
  while IFS= read -r RUNTIME; do
    [[ -z "$RUNTIME" ]] && continue
    if [[ -z "${SEEN[$RUNTIME]:-}" ]]; then
      HAS_ERRORS=true
      ROW="| ${RUNTIME} | - | - | - | missing |"
      echo "$ROW" >> full-report.md
      echo "$ROW" >> violations.md
      echo "::error title=Runtime size status missing::No size status artifact found for ${RUNTIME}; the build or check did not run."
    fi
  done < <(jq -r '.[].name' "$MATRIX_FILE")
fi

if [[ ${#FILES[@]} -eq 0 && -z "$MATRIX_FILE" ]]; then
  echo "_No runtime size status files were produced and no matrix provided for cross-check._" >> full-report.md
fi

if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
  cat full-report.md >> "$GITHUB_STEP_SUMMARY"
fi

{
  if $HAS_WARNINGS || $HAS_ERRORS; then
    echo "has_violations=true"
  else
    echo "has_violations=false"
  fi
  if $HAS_ERRORS; then
    echo "has_errors=true"
  else
    echo "has_errors=false"
  fi
} >> "${GITHUB_OUTPUT:-/dev/stdout}"
