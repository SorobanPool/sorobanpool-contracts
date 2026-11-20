#!/usr/bin/env python3
"""
Validates deployments/*.json files against deployments/schema.json.
Uses standard library only so it runs without extra dependencies in CI and locally.
"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEPLOYMENTS_DIR = ROOT / "deployments"
SCHEMA_FILE = DEPLOYMENTS_DIR / "schema.json"

ADDRESS_RE = re.compile(r"^[CG][A-Z0-9]{55}$")
HASH_RE = re.compile(r"^[0-9a-fA-F]{64}$")
NETWORKS = {"testnet", "mainnet", "local", "futurenet"}

def validate_deployment(file_path: Path) -> list:
    errors = []
    try:
        data = json.loads(file_path.read_text(encoding="utf-8"))
    except Exception as e:
        return [f"Invalid JSON: {e}"]

    if not isinstance(data, dict):
        return ["Root must be an object"]

    # 1. Required fields
    if "network" not in data:
        errors.append("Missing required field: 'network'")
    elif data["network"] not in NETWORKS:
        errors.append(f"Invalid network '{data['network']}', must be one of {NETWORKS}")

    if "contracts" not in data:
        errors.append("Missing required field: 'contracts'")
    elif not isinstance(data["contracts"], dict):
        errors.append("'contracts' must be an object")
    else:
        for c_name, c_val in data["contracts"].items():
            if not isinstance(c_val, dict):
                errors.append(f"contract '{c_name}' must be an object")
                continue
            # Non-empty contract entry check
            c_id = c_val.get("id")
            c_wasm = c_val.get("wasmHash")
            if c_id is not None:
                if not isinstance(c_id, str) or not ADDRESS_RE.match(c_id):
                    errors.append(f"contract '{c_name}.id' must be a valid Stellar contract address: '{c_id}'")
            if c_wasm is not None:
                if not isinstance(c_wasm, str) or not HASH_RE.match(c_wasm):
                    errors.append(f"contract '{c_name}.wasmHash' must be a 64-char hex sha256: '{c_wasm}'")

    # 2. Addresses (if provided and non-null)
    for addr_field in ("usdc", "admin", "treasury"):
        val = data.get(addr_field)
        if val is not None and val != "":
            if not isinstance(val, str) or not ADDRESS_RE.match(val):
                errors.append(f"'{addr_field}' must be a valid Stellar address (G... or C...): '{val}'")

    # 3. Hashes (if provided and non-null)
    for hash_field in ("commit", "auditReportHash"):
        val = data.get(hash_field)
        if val is not None and val != "":
            if not isinstance(val, str) or not (len(val) in (40, 64) and re.match(r"^[0-9a-fA-F]+$", val)):
                errors.append(f"'{hash_field}' must be a valid hex hash: '{val}'")

    # 4. smartWallet
    sw = data.get("smartWallet")
    if sw is not None:
        if not isinstance(sw, dict):
            errors.append("'smartWallet' must be an object")
        else:
            sw_hash = sw.get("wasmHash")
            if sw_hash is not None and (not isinstance(sw_hash, str) or not HASH_RE.match(sw_hash)):
                errors.append(f"'smartWallet.wasmHash' must be a 64-char hex sha256: '{sw_hash}'")

    return errors

def main():
    if not SCHEMA_FILE.exists():
        print(f"Error: Schema file not found at {SCHEMA_FILE}", file=sys.stderr)
        sys.exit(1)

    deployment_files = sorted(
        f for f in DEPLOYMENTS_DIR.glob("*.json") if f.name != "schema.json"
    )

    if not deployment_files:
        print("Error: No deployment files found to validate", file=sys.stderr)
        sys.exit(1)

    total_errors = {}
    for f in deployment_files:
        errs = validate_deployment(f)
        if errs:
            total_errors[f.name] = errs
        else:
            print(f"  OK: {f.name}")

    if total_errors:
        print("\nDeployment validation errors detected:", file=sys.stderr)
        for fname, errs in total_errors.items():
            print(f"  [{fname}]:", file=sys.stderr)
            for err in errs:
                print(f"    - {err}", file=sys.stderr)
        sys.exit(1)

    print(f"\nAll {len(deployment_files)} deployment files match schema successfully.")

if __name__ == "__main__":
    main()
