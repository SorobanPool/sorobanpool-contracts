# Architecture

Normative rules: build brief sections 3, 4 and 6 (`docs/brief.md`). Contracts are the source of truth for escrow;
the backend cannot move funds, and every user exit is permissionless and time-checked.

```
              user's passkey (phone secure hardware)
                         │ WebAuthn assertion over the Soroban auth payload
                         ▼
  ┌──────────────┐  require_auth  ┌───────────────────────────────────────────────┐
  │ smart_wallet │◀───────────────│ group_buy  (pools, escrow, settlement, refunds)│
  │ one per user │                └──┬──────────────┬──────────────┬──────────────┘
  └──────────────┘                   │ freeze /     │ record       │ reserve / release / slash
                                     │ apply_outcome│              ▼
                         ┌───────────┴──┐   ┌──────┴─────┐  ┌───────────────┐
                         │  disputes    │──▶│ reputation │  │ supplier_bond │
                         └──────────────┘   └────────────┘  └───────────────┘
                                 every contract reads ▼
                         ┌──────────────────────────────────────────────┐
                         │ config: admin, guardian, params + hard caps, │
                         │ pause scopes, sibling addresses, attestors,  │
                         │ arbiters, categories                         │
                         └──────────────────────────────────────────────┘
                         registry: roles, status, verification hashes (no PII)
```

| Contract | Responsibility | Privileged callers |
|---|---|---|
| `config` | Parameters within hard caps, pause scopes, address registry, attestors, arbiters, categories | admin (multisig on mainnet); guardian may pause |
| `registry` | Trader / organizer / supplier status and verification levels | attestors (`attest`), admin, `group_buy` / `disputes` (`suspend`) |
| `group_buy` | Pool state machine, commitments, escrow, shortfall allocation, settlement, refunds | `disputes` only for `freeze` / `apply_outcome` |
| `disputes` | Disputes, deposits, arbiter resolution, SLA timeout | arbiters (`resolve`) |
| `supplier_bond` | Supplier bonds, advance reservations, slashing | `group_buy` |
| `reputation` | Counters and tiers | `group_buy`, `disputes` |
| `smart_wallet` | A user's account: passkey signers, WebAuthn `__check_auth` (ADR 0006) | the account itself |

Cross-contract privileged calls: the callee resolves the expected caller through `config.get_address`, requires
`caller.require_auth()` and checks equality. Shared types, errors, math (tiers, pro-rata allocation), event and
TTL helpers live in `crates/sp_common`; `crates/scenarios` wires every contract together with a real token for
scenario, property, fuzz and budget tests.

---

## Reproducible Builds

Contract WASM binaries are built to be deterministically reproducible so that community members, node operators, and auditors can independently verify on-chain bytecode against this repository.

### Build Toolchain
- **Rust Toolchain:** `1.98.1` (pinned via `rust-toolchain.toml`)
- **Compilation Target:** `wasm32v1-none`
- **Stellar CLI:** `28.1.0` (invokes `stellar contract build`)
- **Optimization Profile:** Defined in root `Cargo.toml` (`[profile.release]` with `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`, `overflow-checks = true`).

### Path Remapping & Hash Verification
Building via `scripts/build.sh` applies `--remap-path-prefix` to eliminate local absolute filesystem paths from debug sections and metadata, ensuring that two separate checkouts compiled on the same toolchain produce byte-identical `.wasm` artifacts:

```bash
scripts/build.sh
```

Upon compilation, `scripts/build.sh` prints a stable SHA-256 table:

```text
=== WASM Release Hashes (SHA-256) ===
config            276d7103918fa48feed687f3ca81933ae635cf4af9d0ab0694c4c91ad788c96b
disputes          e9e743028b972355784736e1f228b73c696b8f787ed3db53d70baf4909399739
group_buy         3b07bc93c6bb248f6315d49515015ff995f350c537fa3f8470d82442fced4e8f
registry          b08c55b4ef989bed330b8340ff7577d06fe43dd3f5fd6b308776bfa58999f19c
reputation        e47922406a45673bfb86cdd369aaa70b0330d99dad27592e9be3b9e5201cfc3d
smart_wallet      8d06c8e55250dfee0f48d2723305a9e66d83d00e20e9198de0ee6e098a62dd14
supplier_bond     f5ac9307c4d7dc44434764d18b0454c6543d420cc1c2e1f73220a6ada08b5acc
```

These hashes match the deployed contract wasm hashes recorded in [`deployments/testnet.json`](../deployments/testnet.json).

---

Further reading: `state-machine.md`, `pricing-math.md`, `events.md`, `errors.md`, `storage-and-ttl.md`,
`threat-model.md`, `decisions/`.
