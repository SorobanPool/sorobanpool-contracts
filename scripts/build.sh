#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

# Ensure reproducible compilation with path remapping
export CARGO_BUILD_RUSTFLAGS="--remap-path-prefix=${PWD}=. ${CARGO_BUILD_RUSTFLAGS:-}"

stellar contract build

echo ""
echo "=== WASM Release Hashes (SHA-256) ==="
python3 - << 'PY'
import hashlib
from pathlib import Path

target_dir = Path("target/wasm32v1-none/release")
wasm_files = sorted(target_dir.glob("*.wasm"))
if not wasm_files:
    print("Warning: No wasm files found in", target_dir)
for f in wasm_files:
    h = hashlib.sha256(f.read_bytes()).hexdigest()
    print(f"{f.stem:<16}  {h}")
PY
