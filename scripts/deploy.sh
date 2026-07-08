#!/usr/bin/env bash
# Usage: NETWORK=local|testnet|mainnet scripts/deploy.sh
set -euo pipefail
cd "$(dirname "$0")/.."
NETWORK="${NETWORK:?set NETWORK=local|testnet|mainnet}"
if [ "$NETWORK" = "mainnet" ]; then
  [ "${MAINNET_CONFIRM:-}" = "yes" ] || { echo "refusing: set MAINNET_CONFIRM=yes"; exit 1; }
  [ -z "$(git status --porcelain)" ] || { echo "refusing: working tree not clean"; exit 1; }
  git describe --exact-match --tags HEAD >/dev/null 2>&1 || { echo "refusing: HEAD is not a release tag"; exit 1; }
fi
echo "deploy to $NETWORK: implemented in M1"
exit 1
