#!/usr/bin/env bash
# Wires the deployed contracts together and sets staging defaults. Run by deploy.sh.
# Mainnet wiring must be done by the admin multisig; this script refuses to run there.
set -euo pipefail
cd "$(dirname "$0")/.."
NETWORK="${NETWORK:?set NETWORK}"
[ "$NETWORK" != "mainnet" ] || { echo "mainnet init must be signed by the admin multisig (build the XDR by hand)"; exit 1; }
SOURCE="${SOURCE:-sorobanpool-deployer}"
F="deployments/${NETWORK}.json"
id() { jq -r ".contracts.$1.id" "$F"; }
CFG="$(id config)"
call() { stellar contract invoke --id "$CFG" --source "$SOURCE" --network "$NETWORK" -- "$@" >/dev/null; }

call set_address --key registry --addr "$(id registry)"
call set_address --key rep --addr "$(id reputation)"
call set_address --key bond --addr "$(id supplier_bond)"
call set_address --key group_buy --addr "$(id group_buy)"
call set_address --key disputes --addr "$(id disputes)"
ADMIN="$(jq -r .admin "$F")"
call set_attestor --a "$ADMIN" --enabled true
call set_arbiter --a "$ADMIN" --enabled true
for cat in rice beans garri oil sugar noodles seasoning beverage toiletry fabric phoneacc stationery cement roofing packaging; do
  call set_category --cat "$cat" --allowed true
done
echo "initialised $NETWORK"
