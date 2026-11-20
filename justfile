# Common developer recipes for SorobanPool contracts

default:
    @just --list

# Check code formatting and run clippy linter
lint:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings

# Build contracts reproducibly to WASM and print hashes
build:
    cargo build --workspace
    scripts/build.sh

# Run all workspace unit and scenario tests
test:
    cargo test --workspace

# Run client walkthrough example
example:
    cargo run -p examples

# Validate deployments JSON schema
schema:
    python3 scripts/validate_deployments.py

# Check markdown links across documentation
links:
    python3 scripts/check_markdown_links.py

# Generate and verify TypeScript bindings for all contracts
bindings:
    scripts/bindings.sh

# Export contract errors.json and events.json artifacts
artifacts:
    python3 scripts/export_artifacts.py

# Generate line coverage summary report
cov:
    cargo llvm-cov --workspace --summary-only

# Run full validation suite (lint, build, test, schema, links, bindings, artifacts)
check: lint test schema links bindings artifacts
