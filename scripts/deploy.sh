#!/usr/bin/env bash
# Usage: NETWORK=local|testnet|mainnet [SOURCE=<stellar key name>] scripts/deploy.sh
# Uploads and deploys all contracts, then writes deployments/<network>.json.
set -euo pipefail
cd "$(dirname "$0")/.."
NETWORK="${NETWORK:?set NETWORK=local|testnet|mainnet}"
if [ "$NETWORK" = "mainnet" ]; then
  [ "${MAINNET_CONFIRM:-}" = "yes" ] || { echo "refusing: set MAINNET_CONFIRM=yes"; exit 1; }
  [ -z "$(git status --porcelain)" ] || { echo "refusing: working tree not clean"; exit 1; }
  git describe --exact-match --tags HEAD >/dev/null 2>&1 || { echo "refusing: HEAD is not a release tag"; exit 1; }
  AUDIT_HASH="$(jq -r '.auditReportHash // empty' deployments/mainnet.json)"
  [ -n "$AUDIT_HASH" ] || { echo "refusing: deployments/mainnet.json has no auditReportHash"; exit 1; }
  : "${ADMIN_ADDRESS:?mainnet needs ADMIN_ADDRESS (the admin multisig)}"
  : "${USDC_CONTRACT_ID:?mainnet needs USDC_CONTRACT_ID}"
  : "${TREASURY_ADDRESS:?mainnet needs TREASURY_ADDRESS}"
fi

SOURCE="${SOURCE:-sorobanpool-deployer}"
ADMIN_ADDRESS="${ADMIN_ADDRESS:-$(stellar keys address "$SOURCE")}"
TREASURY_ADDRESS="${TREASURY_ADDRESS:-$ADMIN_ADDRESS}"
CONTRACTS=(config registry reputation supplier_bond group_buy disputes)
OUT="deployments/${NETWORK}.json"

scripts/build.sh

if [ -z "${USDC_CONTRACT_ID:-}" ]; then
  [ "$NETWORK" != "mainnet" ] || { echo "USDC_CONTRACT_ID required"; exit 1; }
  ASSET="USDC:$ADMIN_ADDRESS"   # test asset issued by the admin; staging only
  stellar contract asset deploy --asset "$ASSET" --source "$SOURCE" --network "$NETWORK" >/dev/null 2>&1 || true
  USDC_CONTRACT_ID="$(stellar contract id asset --asset "$ASSET" --network "$NETWORK")"
fi

deploy() { # name, constructor args...
  local name="$1"; shift
  stellar contract deploy --wasm "target/wasm32v1-none/release/${name}.wasm" \
    --source "$SOURCE" --network "$NETWORK" -- "$@"
}

CFG="$(deploy config --admin "$ADMIN_ADDRESS" --usdc "$USDC_CONTRACT_ID" --treasury "$TREASURY_ADDRESS")"
declare -A IDS=([config]="$CFG")
for c in registry reputation supplier_bond group_buy disputes; do
  IDS[$c]="$(deploy "$c" --config "$CFG")"
done

COMMIT="$(git rev-parse HEAD)"
JSON="$(jq -n --arg net "$NETWORK" --arg commit "$COMMIT" --arg usdc "$USDC_CONTRACT_ID" \
  --arg admin "$ADMIN_ADDRESS" --arg treasury "$TREASURY_ADDRESS" \
  '{network:$net, commit:$commit, usdc:$usdc, admin:$admin, treasury:$treasury, contracts:{}}')"
for c in "${CONTRACTS[@]}"; do
  HASH="$(sha256sum "target/wasm32v1-none/release/${c}.wasm" | cut -d' ' -f1)"
  JSON="$(echo "$JSON" | jq --arg c "$c" --arg id "${IDS[$c]}" --arg h "$HASH" '.contracts[$c]={id:$id, wasmHash:$h}')"
done
# keep the mainnet audit hash if present
if [ -f "$OUT" ]; then
  AH="$(jq -r '.auditReportHash // empty' "$OUT")"
  [ -z "$AH" ] || JSON="$(echo "$JSON" | jq --arg a "$AH" '.auditReportHash=$a')"
fi
echo "$JSON" > "$OUT"
echo "wrote $OUT"
NETWORK="$NETWORK" SOURCE="$SOURCE" scripts/init.sh
