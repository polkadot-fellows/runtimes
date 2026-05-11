#!/usr/bin/env bash
set -euo pipefail

: "${GH_TOKEN:?GH_TOKEN required}"
: "${VERSION:?VERSION required}"
: "${SUMMARY_PATH:?SUMMARY_PATH required}"

REPO="${GITHUB_REPOSITORY:-polkadot-fellows/runtimes}"
SERVER="${GITHUB_SERVER_URL:-https://github.com}"
RUN_ID="${GITHUB_RUN_ID:-}"
TITLE="Runtime WASM size threshold breach in v${VERSION}"

BODY_FILE=$(mktemp)
{
  echo "The release workflow detected one or more runtime WASM artifacts at or above the configured size thresholds."
  echo
  echo "Thresholds: warning at 80% of max, error at 95% of max. An error-level breach blocks the release."
  echo
  if [[ -s "$SUMMARY_PATH" ]]; then
    cat "$SUMMARY_PATH"
  else
    echo "_No detailed violations table was produced - see workflow logs._"
  fi
  echo
  if [[ -n "$RUN_ID" ]]; then
    echo "Workflow run: ${SERVER}/${REPO}/actions/runs/${RUN_ID}"
  fi
} > "$BODY_FILE"

EXISTING=$(gh issue list --repo "$REPO" --state open \
  --search "in:title \"${TITLE}\"" \
  --json number,title \
  --jq "[.[] | select(.title == \"${TITLE}\")][0].number // empty")

if [[ -n "$EXISTING" ]]; then
  gh issue comment "$EXISTING" --repo "$REPO" --body-file "$BODY_FILE"
  echo "Commented on existing issue #${EXISTING}"
else
  # Use the existing "Release" label (created for release tracking issues).
  gh issue create \
    --repo "$REPO" \
    --title "$TITLE" \
    --body-file "$BODY_FILE" \
    --label "Release"
fi
