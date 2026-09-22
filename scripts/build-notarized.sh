#!/usr/bin/env bash
# Build signed + notarized macOS DMG.
#
# Prerequisites:
#   1. Developer ID Application cert installed (signingIdentity in tauri.conf.json).
#   2. App-specific password stored via:
#        xcrun notarytool store-credentials "toyomimi" \
#          --apple-id "<your Apple ID>" --team-id "<your Team ID>"
#
# Usage:  ./scripts/build-notarized.sh
#
# Reads the app-specific password from your terminal env. If not set, prompts.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# Load secrets from .env (gitignored). Format:
#   APPLE_PASSWORD=xxxx-xxxx-xxxx-xxxx
if [[ -f "$REPO_ROOT/.env" ]]; then
  set -a
  # shellcheck disable=SC1091
  source "$REPO_ROOT/.env"
  set +a
fi

APPLE_ID="${APPLE_ID:?set APPLE_ID in .env}"
APPLE_TEAM_ID="${APPLE_TEAM_ID:?set APPLE_TEAM_ID in .env}"

if [[ -z "${APPLE_PASSWORD:-}" ]]; then
  echo "ERROR: APPLE_PASSWORD not set." >&2
  echo "Create .env in repo root with: APPLE_PASSWORD=xxxx-xxxx-xxxx-xxxx" >&2
  exit 1
fi

export APPLE_ID APPLE_TEAM_ID APPLE_PASSWORD

echo "Building with notarization..."
echo "  APPLE_ID=$APPLE_ID"
echo "  APPLE_TEAM_ID=$APPLE_TEAM_ID"
echo "  APPLE_PASSWORD=*** (${#APPLE_PASSWORD} chars)"
echo

npm run tauri build
