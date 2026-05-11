#!/usr/bin/env bash
set -euo pipefail

STATUS_DIR="${1:?status dir required}"

HAS_WARNINGS=false
HAS_ERRORS=false

mapfile -t FILES < <(find "$STATUS_DIR" -type f -name '*.json' | sort)

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

if [[ ${#FILES[@]} -eq 0 ]]; then
  echo "No size status files found in $STATUS_DIR."
  echo "_No runtime size status files were produced._" >> full-report.md
  {
    echo "has_violations=false"
    echo "has_errors=false"
  } >> "${GITHUB_OUTPUT:-/dev/stdout}"
  if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
    cat full-report.md >> "$GITHUB_STEP_SUMMARY"
  fi
  exit 0
fi

for f in "${FILES[@]}"; do
  RUNTIME=$(jq -r .runtime "$f")
  SIZE=$(jq -r .size "$f")
  MAX=$(jq -r .max_size "$f")
  PCT=$(jq -r .pct "$f")
  LEVEL=$(jq -r .level "$f")

  ROW="| ${RUNTIME} | ${SIZE} | ${MAX} | ${PCT}% | ${LEVEL} |"
  echo "$ROW" >> full-report.md

  case "$LEVEL" in
    warning) HAS_WARNINGS=true; echo "$ROW" >> violations.md ;;
    error)   HAS_ERRORS=true;   echo "$ROW" >> violations.md ;;
    missing) HAS_ERRORS=true;   echo "$ROW" >> violations.md ;;
  esac
done

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
