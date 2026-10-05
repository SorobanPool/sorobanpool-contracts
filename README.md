# sorobanpool-contracts

Soroban smart contracts for SorobanPool: group buying with escrow. **Buy together. Pay on delivery.** Built on Stellar.

Contracts: `config`, `registry`, `group_buy`, `disputes`, `supplier_bond`, `reputation`, plus shared crate `sp_common`.

## Develop
```
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
stellar contract build     # wasm output in target/wasm32v1-none/release
```
Mainnet deploys require `MAINNET_CONFIRM=yes`, a clean tree and a release tag (see `scripts/deploy.sh`). Testnet only until the audit milestone is complete.

Status: M1 (core contracts) deployed to testnet; see `deployments/testnet.json`, `docs/`, and `vectors/pricing-vectors.json` (the cross-repo pricing parity vectors).

## Layout
- `contracts/` — `config`, `registry`, `reputation`, `supplier_bond`, `group_buy`, `disputes`
- `crates/sp_common` — shared types, math, cross-contract client traits, TTL helpers
- `crates/scenarios` — end-to-end scenario and property tests across all contracts
- `bindings/` — generated TypeScript bindings (CI fails when stale); `artifacts/` — `errors.json`, `events.json`
- `docs/` — state machine, pricing math, events, errors, storage/TTL, threat model, ADRs

## Testnet
`NETWORK=testnet scripts/deploy.sh` (key alias `sorobanpool-deployer`, funded by friendbot). The staging USDC is a test asset issued by the admin. Mainnet deploys additionally require `MAINNET_CONFIRM=yes`, a clean tree, a release tag, an audit hash and an admin multisig.
