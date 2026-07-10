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

Further reading: `state-machine.md`, `pricing-math.md`, `events.md`, `errors.md`, `storage-and-ttl.md`,
`threat-model.md`, `decisions/`.
