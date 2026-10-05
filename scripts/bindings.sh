#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
for c in config registry group_buy disputes supplier_bond reputation; do
  stellar contract bindings typescript \
    --wasm "target/wasm32v1-none/release/$c.wasm" \
    --output-dir "bindings/$c" --overwrite
done
