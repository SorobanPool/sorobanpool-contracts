# sorobanpool-contracts

Soroban smart contracts for SorobanPool: group buying with escrow. **Buy together. Pay on delivery.** Built on Stellar.

Contracts: `config`, `registry`, `group_buy`, `disputes`, `supplier_bond`, `reputation`, plus shared crate `sp_common`.

The product and architecture brief (source of truth): [docs/brief.md](docs/brief.md). Sibling repos: [backend](https://github.com/SorobanPool/sorobanpool-backend), [frontend](https://github.com/SorobanPool/sorobanpool-frontend).

## Develop
```
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
stellar contract build     # wasm output in target/wasm32v1-none/release
```
Mainnet deploys require `MAINNET_CONFIRM=yes`, a clean tree and a release tag (see `scripts/deploy.sh`). Testnet only until the audit milestone is complete.

Status: feature-complete and deployed to testnet (not audited; do not use on mainnet). See `deployments/testnet.json`, `docs/`, and `vectors/pricing-vectors.json` (the cross-repo pricing parity vectors).

## Quality gates (CI)
- 62 tests, including scenario flows, 600 pricing vectors, a proptest conservation property, and randomised operation-sequence fuzzing (tokens conserved, escrow equals recorded balance, rejected calls are typed contract errors).
- Line coverage floor of 90% on `group_buy`, `disputes` and `supplier_bond` (currently 95%, 95% and 94%), enforced by the `coverage` job.
- Resource budget tests at a 200-member pool: `commit`, `settle`, `claim_refund` stay under 50% and `push_refunds(25)` under 80% of an assumed 100M-CPU / 40 MiB limit (measured: 4%, 7%, 3% and 39% CPU). These run natively, which under-reports Wasm cost, so re-check with network simulation before mainnet.
- Not done: coverage-guided fuzzing (needs nightly Rust), formal verification, external audit.

## Layout
- `contracts/` — `config`, `registry`, `reputation`, `supplier_bond`, `group_buy`, `disputes`
- `crates/sp_common` — shared types, math, cross-contract client traits, TTL helpers
- `crates/scenarios` — end-to-end scenario and property tests across all contracts
- `bindings/` — generated TypeScript bindings (CI fails when stale); `artifacts/` — `errors.json`, `events.json`
- `docs/` — state machine, pricing math, events, errors, storage/TTL, threat model, ADRs

## Testnet
`NETWORK=testnet scripts/deploy.sh` (key alias `sorobanpool-deployer`, funded by friendbot). The staging USDC is a test asset issued by the admin. Mainnet deploys additionally require `MAINNET_CONFIRM=yes`, a clean tree, a release tag, an audit hash and an admin multisig.
