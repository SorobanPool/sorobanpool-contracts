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

Status: M0 (foundations). Contracts are placeholders until M1.
