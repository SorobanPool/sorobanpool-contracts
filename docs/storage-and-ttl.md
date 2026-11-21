# Storage and TTL

This document details the storage layout, key schemas, and TTL management policies across all seven Soroban contracts in SorobanPool.

## High-Level Architecture & TTL Policies

- **Instance Storage:** Reserved for global contract configurations (e.g., config pointer, admin, treasury address, global parameters) and sequential ID counters (`NextId`).
- **Persistent Storage:** Used for core stateful entities (pools, commitments, member directory pages, bonds, reputation tallies, registry records, dispute records, and passkey public keys).
- **Temporary Storage:** Never used for financial or ledger state.

### TTL Extension Strategy

1. **Persistent Entries:**
   Every read or write to a persistent entry invokes `sp_common::ttl::extend_persistent` (or `extend_ttl` directly), maintaining a threshold of ~30 days (518,400 ledgers) and extending the entry to ~60 days (1,036,800 ledgers). In `smart_wallet`, passkey signers are extended on every authorization up to ~120 days.

| Constant | Value (Ledgers) | Time (at 5s/ledger) | Purpose |
|---|---|---|---|
| `TTL_THRESHOLD` | 518,400 | ~30 days | Extension is triggered when remaining entry TTL drops below this threshold |
| `TTL_EXTEND_TO` | 1,036,800 | ~60 days | Target TTL to extend the entry to when bumping |
2. **Instance and Code Storage:**
   **Instance and code TTL are never extended by inline contract calls.** In Soroban, extending instance storage also extends the contract's executable WASM code, which charges rent on the total contract bytecode. Extending this on standard user calls (e.g. `create_pool`) would cause severe, unnecessary transaction fees (measured up to ~159 XLM). Instead, an automated backend `ttl-extend` keeper job inspects contract instances, bytecode, active pools, open disputes, bonds, and active user accounts when remaining TTL drops below ~20 days and bumps them deliberately from the sponsor account.
3. **Archival & Finality:**
   Final pools (`Settled`, `Expired`, `Failed`, `Cancelled` with all refunds fully paid) are left to archive after 180 days. Pools with outstanding obligations (`owed_total > refunds_paid`) are monitored and refreshed by the backend until all refunds are drained.

---

## Storage Layout by Contract

### 1. `config` Contract

Stores protocol-wide addresses, financial parameters, and role whitelists.

| Storage Key Variant | Storage Type | Key Schema | Value Type | Written By | TTL Policy | Bounded / Paged |
|---|---|---|---|---|---|---|
| `Key::Admin` | Instance | `()` | `Address` | Constructor, `transfer_admin` | Keeper maintained | Bounded (1 entry) |
| `Key::Usdc` | Instance | `()` | `Address` | Constructor | Keeper maintained | Bounded (1 entry) |
| `Key::Treasury` | Instance | `()` | `Address` | Constructor, `set_treasury` | Keeper maintained | Bounded (1 entry) |
| `Key::Params` | Instance | `()` | `Params` | Constructor, `set_params` | Keeper maintained | Bounded (1 entry) |
| `Key::Guardian` | Instance | `()` | `Option<Address>` | Constructor, `set_guardian` | Keeper maintained | Bounded (1 entry) |
| `Key::Addr(Symbol)` | Persistent | `Symbol` | `Address` | Admin (`set_address`) | Threshold ~30d, extend ~60d | Bounded (contract registry) |
| `Key::Attestor(Address)` | Persistent | `Address` | `bool` | Admin (`set_attestor`) | Threshold ~30d, extend ~60d | Bounded by active attestor count |
| `Key::Arbiter(Address)` | Persistent | `Address` | `bool` | Admin (`set_arbiter`) | Threshold ~30d, extend ~60d | Bounded by active arbiter count |
| `Key::Category(Symbol)` | Persistent | `Symbol` | `bool` | Admin (`set_category`) | Threshold ~30d, extend ~60d | Bounded protocol categories |
| `Key::Paused(Symbol)` | Persistent | `Symbol` | `bool` | Guardian / Admin (`pause`, `unpause`) | Threshold ~30d, extend ~60d | Bounded pause scopes |

---

### 2. `registry` Contract

Tracks trader/supplier/organizer verification attestations and geographic clusters.

| Storage Key Variant | Storage Type | Key Schema | Value Type | Written By | TTL Policy | Bounded / Paged |
|---|---|---|---|---|---|---|
| `Key::Config` | Instance | `()` | `Address` | Constructor | Keeper maintained | Bounded (1 entry) |
| `Key::Rec(Address, Role)` | Persistent | `(Address, Role)` | `RegistrationRecord` | Attestor (`register`), Admin (`suspend_user`) | Threshold ~30d, extend ~60d | **Unbounded**: grows with users; indexed by `(Address, Role)` for $O(1)$ lookups |
| `Key::Cluster(Address)` | Persistent | `Address` | `Symbol` | Attestor (`register`) | Threshold ~30d, extend ~60d | **Unbounded**: grows with users; $O(1)$ lookup |

---

### 3. `reputation` Contract

Records historical performance, completed pools, volume, and supplier failure penalties.

| Storage Key Variant | Storage Type | Key Schema | Value Type | Written By | TTL Policy | Bounded / Paged |
|---|---|---|---|---|---|---|
| `Key::Config` | Instance | `()` | `Address` | Constructor | Keeper maintained | Bounded (1 entry) |
| `Key::Stats(Address, Role)` | Persistent | `(Address, Role)` | `ReputationStats` | `group_buy` / `disputes` contracts | Threshold ~30d, extend ~60d | **Unbounded**: grows with active participants; $O(1)$ update |

---

### 4. `supplier_bond` Contract

Manages staked supplier collateral, advance disbursements, and slashing upon default.

| Storage Key Variant | Storage Type | Key Schema | Value Type | Written By | TTL Policy | Bounded / Paged |
|---|---|---|---|---|---|---|
| `Key::Config` | Instance | `()` | `Address` | Constructor | Keeper maintained | Bounded (1 entry) |
| `Key::Bond(Address)` | Persistent | `Address` | `BondAccount` | Supplier (`deposit`, `withdraw`), `group_buy` (`lock`, `release`, `slash`) | Threshold ~30d, extend ~60d | **Unbounded**: grows with registered suppliers; $O(1)$ updates |

---

### 5. `group_buy` Contract

Core pooling engine managing deposits, commitments, escrow balances, and settlements.

| Storage Key Variant | Storage Type | Key Schema | Value Type | Written By | TTL Policy | Bounded / Paged |
|---|---|---|---|---|---|---|
| `Key::Config` | Instance | `()` | `Address` | Constructor | Keeper maintained | Bounded (1 entry) |
| `Key::NextId` | Instance | `()` | `u64` | `create_pool` | Keeper maintained | Bounded (1 entry) |
| `Key::Pool(u64)` | Persistent | `u64` (pool_id) | `Pool` | `create_pool`, state machine transitions | Threshold ~30d, extend ~60d | **Unbounded**: grows with pools created; $O(1)$ read/write |
| `Key::Commit(u64, Address)` | Persistent | `(u64, Address)` | `Commitment` | `commit`, `increase`, `withdraw_commitment`, `claim_refund` | Threshold ~30d, extend ~60d | **Unbounded**: grows with member joins; $O(1)$ lookup |
| `Key::Page(u64, u32)` | Persistent | `(u64, page_idx)` | `Vec<Address>` | `commit` | Threshold ~30d, extend ~60d | **Paged**: Stores member addresses in chunks of up to `PAGE_SIZE = 50`. Pools enforce `max_members_per_pool = 200` (max 4 pages). `allocate_shortfall` and `push_refunds` process in batches of 50 to guarantee bounded compute/memory limits. |

---

### 6. `disputes` Contract

Orchestrates member/supplier disputes, escrow freezing, evidence submission, and arbitration.

| Storage Key Variant | Storage Type | Key Schema | Value Type | Written By | TTL Policy | Bounded / Paged |
|---|---|---|---|---|---|---|
| `Key::Config` | Instance | `()` | `Address` | Constructor | Keeper maintained | Bounded (1 entry) |
| `Key::NextId` | Instance | `()` | `u64` | `open` | Keeper maintained | Bounded (1 entry) |
| `Key::Dispute(u64)` | Persistent | `u64` (dispute_id) | `Dispute` | `open`, `resolve`, `timeout` | Threshold ~30d, extend ~60d | **Unbounded**: grows with disputes raised; $O(1)$ access |
| `Key::PoolList(u64)` | Persistent | `u64` (pool_id) | `Vec<u64>` | `open` | Threshold ~30d, extend ~60d | Bounded per pool by member dispute limits |
| `Key::Evidence(u64, Address)` | Persistent | `(u64, Address)` | `BytesN<32>` | Dispute parties (`add_evidence`) | Threshold ~30d, extend ~60d | Bounded to 1 evidence hash per party per dispute |

---

### 7. `smart_wallet` Contract

Passkey-controlled WebAuthn (secp256r1) user account contract.

| Storage Key Variant | Storage Type | Key Schema | Value Type | Written By | TTL Policy | Bounded / Paged |
|---|---|---|---|---|---|---|
| `Key::Count` | Instance | `()` | `u32` | Constructor, `add_signer`, `remove_signer` | Keeper maintained | Bounded (1 entry) |
| `Key::Signer(Bytes)` | Persistent | `Bytes` (credential_id) | `BytesN<65>` (public key) | Constructor, `add_signer`, `remove_signer` | Extended on every auth (threshold ~30d to ~120d) | **Bounded**: strictly capped at `MAX_SIGNERS = 10` per wallet. No paging required. |

---

## Reviewer Checklist

- [x] `config`: `Admin`, `Usdc`, `Treasury`, `Params`, `Guardian`, `Addr`, `Attestor`, `Arbiter`, `Category`, `Paused`
- [x] `registry`: `Config`, `Rec`, `Cluster`
- [x] `reputation`: `Config`, `Stats`
- [x] `supplier_bond`: `Config`, `Bond`
- [x] `group_buy`: `Config`, `NextId`, `Pool`, `Commit`, `Page`
- [x] `disputes`: `Config`, `NextId`, `Dispute`, `PoolList`, `Evidence`
- [x] `smart_wallet`: `Count`, `Signer`
- [x] Paged collections explicitly documented (`group_buy::Key::Page` with `PAGE_SIZE = 50`)
