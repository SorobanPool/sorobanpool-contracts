# SorobanPool — Build Brief

> **Audience:** Claude Code (the build agent) and human contributors.
> **Purpose:** Everything needed to build SorobanPool from an empty GitHub org to a mainnet-ready product.
> **Org layout:** three repositories — `sorobanpool-contracts`, `sorobanpool-backend`, `sorobanpool-frontend`.
> **Status of this document:** source of truth. If code and brief disagree, raise it in an issue and update the brief in the same PR that changes behaviour.
> This copy lives in `sorobanpool-contracts/docs/brief.md`; the backend and frontend READMEs link to it.

---

## 0. How to use this brief (instructions for the build agent)

1. Read the whole brief before writing code. Sections 3–5 define the domain; sections 6–8 define each repo; sections 9–14 define cross-cutting rules.
2. Build in the order in **Section 15 (Milestones)**. Contracts first, because backend and frontend depend on the generated contract bindings.
3. **Verify every external dependency before using it.** Versions, package names and APIs named here are intended choices, not guarantees. Before adding a dependency: check the latest stable version (crates.io / npm), read its current docs, pin the exact version. If something named here no longer exists or has changed, pick the closest maintained equivalent and record the decision as an ADR in `docs/decisions/` (Section 14.4).
4. Never put secrets in code, tests, fixtures or commit history. Use git-ignored `.env` files with a committed `.env.example`.
5. Every PR must build, pass lint and tests, update affected docs, and reference an issue.
6. When a requirement is ambiguous, choose the option that is **safer for escrowed funds**, write the assumption in the PR description, and continue.
7. Testnet only until Milestone 6 (audit) is complete. Mainnet scripts require the `MAINNET_CONFIRM=yes` guard (Section 12).

---

## 1. Product summary

**SorobanPool** lets market traders **pool their money to buy in bulk** from suppliers at wholesale prices. Funds sit in a Soroban escrow and are released to the supplier **only when the goods are delivered and confirmed**. Each trader receives their share of the order.

One-liner: **"Buy together. Pay on delivery."** Tagline on all surfaces: **"Built on Stellar."**

### 1.1 The problem

Small traders in Nigerian markets (provisions, foodstuff, drinks, fabric, phone accessories, building materials) buy in small quantities from middlemen at retail-ish prices because:

- They cannot afford the supplier's **minimum order quantity (MOQ)** alone.
- Informal group buying exists (traders contribute cash to one person who buys for everyone), but it fails on **trust**: the collector can disappear, short-deliver, or mark up; suppliers can collect payment and not deliver; disputes have no record.
- Suppliers want bigger orders but cannot extend credit or trust unknown groups.

### 1.2 The solution

- A supplier lists a **bulk offer** with **price tiers** (the more units ordered, the lower the unit price) and an MOQ.
- An **organizer** (often a market association rep or a trusted trader) opens a **Pool** for that offer with a fill deadline and a delivery point.
- Traders **commit units** and pay into escrow. They pay the **current tier's price** at commit time; when the pool closes at a better tier, the difference is **refunded automatically**.
- If the MOQ is not reached by the deadline, **everyone is refunded automatically**.
- If filled, the supplier accepts and delivers. Delivery is confirmed by the organizer and members (or auto-confirmed after a window if nobody disputes). Escrow then pays the supplier, minus fees.
- Disputes (short delivery, wrong or bad goods) freeze the affected funds and go to arbitration with evidence.
- Every completed pool builds **reputation** for suppliers, organizers and traders.

### 1.3 Who uses it

| Persona | Need | Primary surface |
|---|---|---|
| **Trader** ("Iya Bisi", provisions stall, Android, patchy data) | Lower unit cost, no risk of losing money to a collector | Mobile PWA |
| **Organizer** (market association rep / respected trader) | Run group buys for their market without carrying cash or blame | Mobile PWA |
| **Supplier** (distributor, importer, manufacturer's dealer, farm aggregator) | Larger guaranteed orders, payment certainty, less cash handling | Supplier portal (web, mobile-friendly) |
| **Arbiter** (SorobanPool ops or vetted third party) | Resolve disputes quickly and fairly | Admin/arbiter console |
| **Operator/admin** (SorobanPool team) | KYB/KYC, category rules, risk, parameters, incidents | Admin console |

### 1.4 Goals and non-goals

**Goals (v1)**
- Trader can join a pool and pay in **under 2 minutes**, from a WhatsApp link.
- **Zero loss** to traders from organizer or supplier non-performance: funds only move on delivery confirmation, auto-refund on failure.
- Traders never need to hold XLM, see a seed phrase, or understand "gas".
- All escrow balances, commitments, deliveries and disputes verifiable on-chain.
- Works on a low-end Android phone on 3G.

**Non-goals (v1)**
- No credit / buy-now-pay-later for traders or suppliers.
- No logistics network of our own (delivery is the supplier's responsibility; we track status).
- No regulated product categories: no pharmaceuticals, alcohol, tobacco, firearms, agrochemicals, baby formula, or anything requiring special licences (Section 13).
- No resale marketplace between traders.
- No native mobile app (PWA only). No governance token.

### 1.5 Naming / brand note

The Stellar Development Foundation's brand policy restricts using the "Stellar" and "Soroban" marks in product, company, domain and social names without SDF's written consent. "SorobanPool" uses the full "Soroban" mark. **Before public launch the team must obtain SDF consent or rename.** Keep the name in **one config constant per repo** (`APP_NAME`) and keep logos/assets in one folder, so a rename is a one-line change plus assets. Always describe the product as "Built on Stellar". Note that "Soroban pool" is also used in the ecosystem to mean liquidity/credit pools; product copy must say "group buying" clearly.

---

## 2. Glossary

| Term | Meaning |
|---|---|
| **Offer** | A supplier's listing: product, unit, price tiers, MOQ, max quantity, lead time, delivery area. |
| **Price tier** | `(min_units, unit_price)` pair. The tier reached by total committed units sets the final unit price. |
| **MOQ** | Minimum order quantity (units) for the supplier to accept. |
| **Pool** | One group buy against one offer, with a fill deadline and delivery point. |
| **Commitment** | A trader's units in a pool and the amount they paid into escrow. |
| **Organizer** | The person who opens and runs a pool; confirms bulk arrival. |
| **Hub** | The delivery point (shop, warehouse, market gate) where goods are received and split. |
| **Allocation** | The units each member is entitled to collect from the hub. |
| **Settlement** | Releasing escrow to supplier (and fees) after confirmation, plus refunds of price-tier difference. |
| **Dispute** | A claim that delivery was short, late, wrong or defective; freezes the claimed portion. |
| **Arbiter** | Address authorised to resolve disputes. |
| **Attestor** | Backend key that writes KYC/KYB verification hashes on-chain. |
| **Anchor** | Stellar on/off-ramp provider (SEP-6/SEP-24) for NGN ↔ USDC. |
| **Smart wallet** | Soroban contract account controlled by a passkey; the user's on-chain identity. |

---

## 3. Core domain rules

These rules are normative. Contracts enforce them; backend and frontend mirror them and never relax them.

### 3.1 Units and money

- On-chain asset: **USDC** (Stellar Asset Contract, SEP-41 interface). Address is configuration, never hardcoded.
- Amounts are `i128` in the token's smallest unit (USDC on Stellar = 7 decimals). Fees in basis points (`u32`). Time is ledger timestamp seconds (`u64`). Quantities are `u32` whole units.
- **Pool prices are fixed in USDC at pool creation.** Suppliers typically think in naira; the supplier portal converts their naira tier prices into USDC using an FX quote at the moment they publish/refresh the offer, and shows both. Because fill windows are short (Section 3.3), FX exposure is small but real; it is disclosed to both sides (Section 13).
- Checked arithmetic everywhere; overflow panics with an error code.
- Rounding: amounts owed to the supplier round **down**; refunds to traders round **up** where the difference is dust; the contract keeps a dust account that is swept to treasury and documented.

### 3.2 Participant lifecycle

```
Unregistered → Registered → Verified → [Suspended] → Closed
```

- **Trader:** phone + passkey + basic KYC (BVN or NIN) for limits above the starter tier.
- **Organizer:** Verified trader with tier ≥ O1 (Section 3.8) or approved by admin.
- **Supplier:** KYB — CAC registration check, director KYC, bank account name match, physical address. Only Verified suppliers can publish offers.
- No PII on-chain, only verification hashes and levels.

### 3.3 Pool lifecycle

```
Open → Filled → Accepted → Dispatched → Delivered → Settled
  │       │         │            │            └→ Disputed → Resolved → Settled
  │       │         │            └→ (delivery deadline missed) → Failed → Refunded
  │       │         └→ (supplier rejects / no response) → Failed → Refunded
  └→ (deadline, MOQ not met) → Expired → Refunded
  └→ Cancelled (organizer, only before any commitment, or admin at any time before Dispatched) → Refunded
```

- **Open:** traders can commit and (before `fill_deadline`) withdraw commitments. Max fill window: 7 days (configurable ≤ 14 days hard cap).
- **Filled:** reached when `now ≥ fill_deadline` and `total_units ≥ MOQ`, or early when `total_units ≥ max_units` or organizer closes early with `total_units ≥ MOQ`. Commitment withdrawal no longer allowed.
- **Accepted:** supplier accepts within `accept_window` (default 24h). Optional **supplier advance** released now (Section 3.6).
- **Dispatched:** supplier marks dispatched with optional waybill hash.
- **Delivered:** organizer confirms **bulk arrival at hub** with received quantity and evidence hash. If received < ordered, the shortfall is recorded and triggers a pro-rata partial refund path (Section 3.5).
- **Settled:** escrow released after the **confirmation window** (default 48h after Delivered) closes with no open disputes, or earlier if members holding ≥ `early_release_weight` (default 60%) of units confirm receipt.
- **Failed/Expired/Cancelled → Refunded:** every member can claim a full refund (pull pattern), and a keeper also pushes refunds in batches.
- Delivery deadline: `accepted_at + offer.lead_time + delivery_grace` (default grace 48h). Missed → anyone can call `fail_delivery`, which refunds all members and returns any supplier advance via the supplier's bond (Section 3.6).

### 3.4 Pricing and commitments

- Offer has 1–5 tiers, sorted ascending by `min_units`, with strictly decreasing `unit_price`. Tier 1 `min_units` = MOQ.
- When a trader commits `n` units, they pay `n × price_of_current_tier_ceiling`, where the ceiling price is the price of the tier the pool is currently in (before their units) — i.e. **the highest price they could end up paying**. This guarantees escrow always covers the final bill.
- At settlement, final unit price = price of the tier reached by `total_units`. Each member's refund = `paid − n × final_price`.
- Per-member caps: `max_units_per_member` (offer-level), plus a platform cap per trader tier (Section 3.8). No member may hold > 50% of a pool's units unless they are the only member (anti-gaming of tiers).
- Members can increase their commitment while Open. Decreasing/withdrawing is allowed while Open and more than 2h before `fill_deadline` (prevents last-minute tier sabotage).

### 3.5 Delivery confirmation, allocations and shortfall

- On Delivered, organizer submits `received_units` and an evidence hash (photos/video stored off-chain, hash on-chain).
- If `received_units < total_units`: shortfall is allocated **pro-rata** to members (rounding down per member; leftover units assigned by commit order). Each member's payable reduces accordingly; their extra paid amount is refunded at settlement. Supplier is paid only for received units.
- Each member sees their allocation and **confirms pickup** in the app (optional but increases trust score and enables early release).
- If organizer does not confirm within `organizer_confirm_window` (default 48h after supplier marks Dispatched + lead time), members holding ≥ 50% of units can confirm delivery collectively, or the supplier can open a dispute to force arbitration.

### 3.6 Supplier advance and bond (optional per supplier)

- Suppliers at tier ≥ S2 may receive an **advance** of up to `advance_bp` (default 0; S2 max 2000 bp = 20%, S3 max 3000 bp = 30%; hard cap 4000 bp) of the escrow when they accept, to buy stock.
- Any supplier receiving advances must hold a **bond** in the `supplier_bond` contract ≥ the total outstanding advances across their active pools. Bond is slashed to refund members if the supplier fails delivery.
- v1 launches with advances **disabled** globally (`advance_enabled = false`) until reputation data exists; code paths and tests must exist.

### 3.7 Disputes

- Who can open: any member (for their allocation), the organizer (for the whole pool), the supplier (if organizer/members refuse to confirm).
- Window: from Delivered until the confirmation window ends (48h), extended to 72h for perishables category.
- Opening a dispute requires a reason code (`SHORT`, `WRONG_ITEM`, `DAMAGED`, `QUALITY`, `NOT_DELIVERED`, `OTHER`), claimed units, and evidence hash. It **freezes** the claimed amount; the rest of the pool may settle normally.
- Arbiter resolves with an outcome per dispute: `ReleaseToSupplier`, `RefundMember(units)`, `Split(bp)`, or `RefundPool` (whole pool). Resolution must happen within `arbitration_sla` (default 5 days); if the arbiter misses the SLA, the frozen amount is **refunded to the disputer** (time-checked, permissionless).
- Dispute deposit to prevent spam: 1% of claimed amount (min 0.5 USDC), returned if the disputer wins or it is a split; forfeited to the counterparty if fully rejected.
- Arbiter set managed by admin multisig; each resolution emits an event with the arbiter address and a reasoning hash.

### 3.8 Reputation and tiers

**Suppliers** (S0–S3): based on pools delivered on time, dispute rate, disputes lost, short-delivery rate, months active. Tier controls visibility ranking, max pool size, and advance eligibility.

| Tier | Requirement | Max pool value (USDC) | Advance |
|---|---|---|---|
| S0 | Verified, no history | 2,000 | No |
| S1 | ≥ 3 settled pools, 0 lost disputes | 10,000 | No |
| S2 | ≥ 15 settled, lost-dispute rate < 5%, on-time ≥ 90% | 30,000 | ≤ 20% |
| S3 | ≥ 50 settled, lost-dispute rate < 2%, on-time ≥ 95%, ≥ 6 months | 100,000 | ≤ 30% |

**Organizers** (O0–O2): settled pools organized, disputes against organizer lost, member satisfaction (pickup confirmations). O0 can run pools ≤ 500 USDC; O1 ≤ 5,000; O2 ≤ 25,000 (also bounded by supplier tier).

**Traders** (T0–T2): T0 (phone + passkey) commit ≤ 50 USDC per pool; T1 (KYC) ≤ 500; T2 (KYC + ≥ 5 settled pools, 0 frivolous disputes) ≤ 2,500.

All counters and tiers are on-chain in the `reputation` contract; thresholds are admin-configurable within hard caps.

### 3.9 Fees

- **Platform fee:** `platform_fee_bp` (default 150 bp = 1.5%) deducted from the supplier payout. Hard cap 500 bp.
- **Organizer fee:** optional, set per pool by organizer, ≤ 100 bp (hard cap 200 bp), deducted from the supplier payout, shown to members before they commit.
- **Traders pay no platform fee** in v1 (they pay only the product price; anchor fees for naira deposit are shown separately).
- Fees are taken only at settlement; failed/expired pools charge nothing.

---

## 4. System architecture

```
                   ┌───────────────────────────────────────────────┐
                   │              sorobanpool-frontend              │
                   │ Trader/Organizer PWA · Supplier portal ·       │
                   │ Admin & Arbiter console                        │
                   └───────────────┬─────────────────┬─────────────┘
                    REST/WS (JWT)  │                 │ sign with passkey / wallet
                                   ▼                 ▼
┌─────────────────────────────────────────┐   ┌─────────────────────────┐
│           sorobanpool-backend           │   │  Stellar network        │
│ API (NestJS) · Workers (BullMQ)         │──▶│  Soroban RPC / Horizon  │
│ Indexer · KYC/KYB · Anchor · Catalog    │   │  Contracts (below)      │
│ Evidence store · Notifier · Relayer     │◀──│  Events                 │
└──────┬──────────────┬──────────┬────────┘   └─────────────────────────┘
       │              │          │
  PostgreSQL        Redis    Object storage (evidence, product images)
                             External: KYC/KYB, SMS/WhatsApp, NGN anchor, FX
```

### 4.1 Contracts (in `sorobanpool-contracts`)

| Contract | Responsibility |
|---|---|
| `config` | Admin multisig, guardian, pause scopes, parameters + hard caps, sibling address registry, attestors, arbiters, category allow-list. |
| `registry` | Traders, organizers, suppliers: status, verification hashes/levels. |
| `group_buy` | Pools, tiers snapshot, commitments, escrow, state machine, settlement, refunds, shortfall allocation. |
| `disputes` | Dispute creation, deposits, freezing, arbiter resolution, SLA timeout. Calls back into `group_buy` for fund movements. |
| `supplier_bond` | Supplier bonds, advance accounting, slashing on failure. |
| `reputation` | Counters and tiers for suppliers, organizers, traders. Writable only by `group_buy` and `disputes`. |
| `smart_wallet` (external/adapted) | Passkey-controlled user account. Prefer an existing maintained (ideally audited) passkey smart-wallet implementation over writing one. |

Offers (product details, images, descriptions) live **off-chain** in the backend catalog. When a pool is created, the contract stores a **snapshot** of the commercial terms that matter for money (supplier, unit label hash, tiers, MOQ, max units, lead time, category) plus `offer_hash = sha256(canonical offer JSON)`. Later offer edits never affect existing pools.

### 4.2 Trust boundaries

- **Contracts** are the source of truth for escrow, commitments, states, disputes, reputation.
- **Backend** is trusted for: KYC/KYB attestations (hash only), catalog content, evidence storage, notifications, anchor flows, fee sponsorship, indexing. It **cannot** move escrowed funds. If fully compromised, worst cases are fake verifications (mitigated by attestor caps and admin revocation), fake catalog content (mitigated by on-chain term snapshot shown at commit time), and service denial — not theft.
- **Arbiters** can move only frozen dispute amounts, only according to the outcome types in 3.7, and only within SLA.
- **Frontend** is untrusted.
- **User exits never depend on the backend:** refund claims, `expire`, `fail_delivery`, dispute SLA timeout and settlement after window are all permissionless and time-checked.

---

## 5. Key user flows (end-to-end)

Each flow lists UI → backend → chain steps. Each must have an E2E test (Section 11.4).

### 5.1 Trader onboarding
1. Trader opens a pool link (usually from WhatsApp) or the app → phone number → OTP.
2. Creates a **passkey** → backend deploys a smart wallet (backend pays fees) → `registry.register_trader`.
3. Profile: name, market, state/LGA, stall type (optional). T0 limits apply immediately.
4. Optional KYC (BVN/NIN + selfie) for T1 → attestor writes hash on-chain.

### 5.2 Supplier onboarding and offers
1. Supplier signs up on the portal: business name, CAC number, director details, bank account, address, categories.
2. KYB via provider + manual review in admin console → attestor `registry.attest_supplier`.
3. Supplier creates an offer: product, brand, unit label ("carton of 40 pieces", "50kg bag"), photos, tiers in NGN, MOQ, max units, max per member, lead time, delivery areas (states/LGAs), category, validity period.
4. Portal shows USDC equivalents at the current FX quote; supplier publishes. Offer stored off-chain with `offer_hash`.
5. Offers expire after their validity period (max 14 days) and must be refreshed (re-quotes FX).

### 5.3 Organizer creates a pool
1. Organizer browses offers available in their area, picks one, sets: fill deadline (≤ 7 days), delivery hub (address + contact), pickup window, optional organizer fee, optional "my market only" restriction.
2. Backend builds `group_buy.create_pool` args with the term snapshot; organizer signs with passkey; relayer submits.
3. App generates a **share card** (image + link) for WhatsApp: product, current price, next price break, units to go, deadline.

### 5.4 Trader joins and pays
1. Trader opens link → sees product, supplier rating, current tier price, price at each break, MOQ progress bar, deadline, hub location, organizer, and the rule "money released only after delivery".
2. Chooses units → sees **max you pay now** and **expected refund if more people join**.
3. Payment: **Pay with naira** (anchor deposit → USDC to wallet → commit) or **Pay with dollar balance**.
4. Signs `group_buy.commit(pool_id, member, units)` → USDC moves into escrow.
5. Real-time progress updates; notifications when a price break is reached and 24h/2h before deadline.

### 5.5 Pool fills or expires
- At deadline, keeper calls `group_buy.close(pool_id)` (permissionless). If `total_units ≥ MOQ` → Filled, supplier notified (SMS + portal). Else → Expired, refunds pushed by keeper, and claimable by members.

### 5.6 Supplier accepts, dispatches
1. Supplier sees filled pool: final units, final tier price, hub, payout estimate after fees.
2. `group_buy.accept(pool_id)` within 24h (else anyone can `fail_accept` → refunds).
3. Supplier marks `dispatch(pool_id, waybill_hash)`; members notified with ETA.

### 5.7 Delivery, split and settlement
1. Goods arrive at hub. Organizer counts, takes photos/video, records `received_units` → evidence uploaded, hash on-chain via `confirm_delivery`.
2. Members get "Your goods are ready: collect X units from [hub] between [window]".
3. Members collect and tap **I've collected** (`confirm_pickup`).
4. When the confirmation window ends with no open disputes (or early-release weight reached), keeper calls `settle(pool_id)`: supplier paid (minus fees and minus frozen dispute amounts), organizer fee paid, members' tier-difference and shortfall refunds credited (pull) and pushed by keeper.

### 5.8 Dispute
1. Member taps **Report a problem**, picks reason, units affected, uploads photos; pays dispute deposit from refund-able balance or wallet.
2. `disputes.open(...)` freezes the claimed amount.
3. Supplier and organizer notified; each can add evidence (hashes) within 48h.
4. Arbiter reviews in console, picks outcome, signs `disputes.resolve(...)`; funds move accordingly; reputation updated.
5. If the arbiter misses SLA, anyone can call `disputes.timeout(dispute_id)` → refund to disputer.

### 5.9 Supplier failure
- No acceptance in 24h → `fail_accept` → full refunds, supplier reputation hit.
- Delivery deadline missed → `fail_delivery` → full refunds; if an advance was paid, the supplier's bond is slashed to cover it; reputation hit; supplier auto-suspended after 2 failures in 90 days.

### 5.10 Admin and arbiter
- KYB/KYC queues, category allow-list, offer moderation (take down misleading offers — only affects new pools), suspensions, arbiter management (multisig), parameter proposals, risk dashboards, dispute queue with SLA timers, audit log.

---

## 6. Repo: `sorobanpool-contracts`

### 6.1 Stack
- Rust stable pinned via `rust-toolchain.toml`; WASM target as specified by current Soroban docs (verify, e.g. `wasm32v1-none`).
- `soroban-sdk` latest stable (verify; pin exact version in workspace).
- Stellar CLI (`stellar`) for build, deploy, bindings.
- Tests: SDK testutils, `proptest`; optional `cargo-fuzz` for pricing/allocation math.

### 6.2 Layout
```
sorobanpool-contracts/
├── Cargo.toml
├── rust-toolchain.toml
├── contracts/
│   ├── config/
│   ├── registry/
│   ├── group_buy/
│   ├── disputes/
│   ├── supplier_bond/
│   └── reputation/
├── crates/
│   └── sp_common/            # types, errors, math (tiers, pro-rata), events, storage keys, TTL helpers
├── scripts/
│   ├── build.sh
│   ├── deploy.sh             # NETWORK=local|testnet|mainnet (guarded)
│   ├── init.sh
│   └── bindings.sh
├── deployments/
│   ├── testnet.json
│   └── mainnet.json
├── bindings/
├── docs/
│   ├── architecture.md
│   ├── state-machine.md      # diagram + every transition, who can call, preconditions
│   ├── pricing-math.md       # worked examples incl. shortfall + disputes
│   ├── storage-and-ttl.md
│   ├── events.md
│   ├── errors.md
│   ├── threat-model.md
│   └── decisions/
└── .github/workflows/ci.yml
```

### 6.3 Shared types (`sp_common`)

```rust
#[contracttype] pub enum Role { Trader, Organizer, Supplier }
#[contracttype] pub enum Status { Registered, Verified, Suspended, Closed }

#[contracttype]
pub enum PoolState { Open, Filled, Accepted, Dispatched, Delivered, Settled,
                     Expired, Failed, Cancelled }

#[contracttype]
pub struct Tier { pub min_units: u32, pub unit_price: i128 }

#[contracttype]
pub struct PoolTerms {
    pub supplier: Address,
    pub offer_hash: BytesN<32>,
    pub unit_label_hash: BytesN<32>,
    pub category: Symbol,
    pub tiers: Vec<Tier>,           // 1..=5, ascending min_units, strictly descending price
    pub moq: u32,                   // == tiers[0].min_units
    pub max_units: u32,
    pub max_per_member: u32,
    pub lead_time_secs: u64,
    pub perishable: bool,
}

#[contracttype]
pub struct Pool {
    pub id: u64,
    pub organizer: Address,
    pub terms: PoolTerms,
    pub hub_hash: BytesN<32>,       // hash of hub address/contact (off-chain)
    pub organizer_fee_bp: u32,
    pub platform_fee_bp: u32,       // snapshot at creation
    pub cluster: Option<Symbol>,    // "my market only"
    pub fill_deadline: u64,
    pub created_at: u64,
    pub filled_at: u64,
    pub accepted_at: u64,
    pub dispatched_at: u64,
    pub delivered_at: u64,
    pub total_units: u32,
    pub received_units: u32,
    pub escrow_balance: i128,
    pub frozen_amount: i128,
    pub advance_paid: i128,
    pub member_count: u32,
    pub state: PoolState,
}

#[contracttype]
pub struct Commitment {
    pub member: Address,
    pub units: u32,
    pub paid: i128,
    pub allocated_units: u32,       // after shortfall
    pub refund_due: i128,
    pub refund_claimed: bool,
    pub picked_up: bool,
    pub committed_at: u64,
}

#[contracttype]
pub enum DisputeReason { Short, WrongItem, Damaged, Quality, NotDelivered, Other }

#[contracttype]
pub enum Outcome { ReleaseToSupplier, RefundMember(u32 /*units*/), Split(u32 /*bp to member*/), RefundPool }

#[contracttype]
pub struct Dispute {
    pub id: u64,
    pub pool_id: u64,
    pub opener: Address,
    pub reason: DisputeReason,
    pub claimed_units: u32,
    pub claimed_amount: i128,
    pub deposit: i128,
    pub evidence: Vec<BytesN<32>>,
    pub opened_at: u64,
    pub resolved: bool,
    pub outcome: Option<Outcome>,
}

#[contracttype]
pub struct Params {
    pub platform_fee_bp: u32,
    pub max_organizer_fee_bp: u32,
    pub max_fill_window_secs: u64,
    pub accept_window_secs: u64,
    pub delivery_grace_secs: u64,
    pub confirm_window_secs: u64,
    pub perishable_confirm_window_secs: u64,
    pub organizer_confirm_window_secs: u64,
    pub early_release_weight_bp: u32,
    pub dispute_deposit_bp: u32,
    pub dispute_deposit_min: i128,
    pub arbitration_sla_secs: u64,
    pub withdraw_lock_before_deadline_secs: u64,
    pub max_member_share_bp: u32,       // 5000
    pub advance_enabled: bool,
    pub supplier_tier_caps: Vec<i128>,
    pub organizer_tier_caps: Vec<i128>,
    pub trader_tier_caps: Vec<i128>,
    pub supplier_advance_bp: Vec<u32>,
}
```

Errors: one `#[contracterror]` enum per contract, codes namespaced (config 100s, registry 200s, group_buy 300s, disputes 400s, supplier_bond 500s, reputation 600s). Document all in `docs/errors.md`; frontend maps each code to plain-language EN + Pidgin messages.

**Hard caps in `config` (contract upgrade required to change):** platform fee ≤ 500 bp; organizer fee ≤ 200 bp; fill window ≤ 14 days; advance ≤ 4000 bp; arbitration SLA ≤ 14 days; dispute deposit ≤ 500 bp.

### 6.4 Contract interfaces (target API; record any SDK-driven changes)

**config**
```rust
fn __constructor(env, admin: Address, usdc: Address, treasury: Address);
fn set_params(env, params: Params);                 // admin; validates hard caps
fn get_params(env) -> Params;
fn set_address(env, key: Symbol, addr: Address);
fn get_address(env, key: Symbol) -> Address;
fn set_attestor(env, a: Address, enabled: bool);
fn set_arbiter(env, a: Address, enabled: bool);
fn is_arbiter(env, a: Address) -> bool;
fn set_category(env, cat: Symbol, allowed: bool);
fn is_category_allowed(env, cat: Symbol) -> bool;
fn pause(env, scope: Symbol);    // admin or guardian: "create","commit","accept","all"
fn unpause(env, scope: Symbol);  // admin only
fn is_paused(env, scope: Symbol) -> bool;
fn set_guardian(env, g: Address);
fn upgrade(env, wasm_hash: BytesN<32>);
```
Pausing **never** blocks refunds, refund claims, `expire`, `fail_*`, dispute timeout, or settlement after the window.

**registry**
```rust
fn register(env, user: Address, role: Role, profile_hash: BytesN<32>, cluster: Option<Symbol>);
fn attest(env, attestor: Address, user: Address, role: Role, ver_hash: BytesN<32>, level: u32);
fn revoke(env, admin: Address, user: Address, role: Role, reason: Symbol);
fn suspend(env, caller: Address, user: Address, role: Role, reason: Symbol); // admin or group_buy/disputes
fn unsuspend(env, admin: Address, user: Address, role: Role);
fn status(env, user: Address, role: Role) -> Status;
fn level(env, user: Address, role: Role) -> u32;
fn cluster_of(env, user: Address) -> Option<Symbol>;
```

**group_buy**
```rust
fn create_pool(env, organizer: Address, terms: PoolTerms, hub_hash: BytesN<32>,
               organizer_fee_bp: u32, cluster: Option<Symbol>, fill_deadline: u64) -> u64;
fn cancel_pool(env, caller: Address, pool_id: u64);                 // organizer if no commitments; admin before Dispatched
fn commit(env, member: Address, pool_id: u64, units: u32) -> i128;  // returns amount charged
fn increase(env, member: Address, pool_id: u64, extra_units: u32) -> i128;
fn withdraw_commitment(env, member: Address, pool_id: u64);         // Open and before lock
fn close_early(env, organizer: Address, pool_id: u64);              // total ≥ MOQ
fn close(env, pool_id: u64);                                        // permissionless after deadline → Filled|Expired
fn accept(env, supplier: Address, pool_id: u64);
fn reject(env, supplier: Address, pool_id: u64);
fn fail_accept(env, pool_id: u64);                                  // permissionless after accept window
fn dispatch(env, supplier: Address, pool_id: u64, waybill_hash: Option<BytesN<32>>);
fn fail_delivery(env, pool_id: u64);                                // permissionless after delivery deadline
fn confirm_delivery(env, organizer: Address, pool_id: u64, received_units: u32, evidence: BytesN<32>);
fn member_confirm_delivery(env, member: Address, pool_id: u64);      // collective fallback (≥50% weight)
fn confirm_pickup(env, member: Address, pool_id: u64);
fn settle(env, pool_id: u64);                                       // permissionless after window or early-release weight
fn claim_refund(env, member: Address, pool_id: u64) -> i128;
fn push_refunds(env, pool_id: u64, max: u32) -> u32;                 // bounded batch, permissionless
// Hooks callable only by `disputes`:
fn freeze(env, caller: Address, pool_id: u64, amount: i128);
fn apply_outcome(env, caller: Address, pool_id: u64, member: Option<Address>, to_member: i128, to_supplier: i128);
// Views
fn pool(env, id: u64) -> Pool;
fn commitment(env, pool_id: u64, member: Address) -> Option<Commitment>;
fn members(env, pool_id: u64, start: u32, limit: u32) -> Vec<Address>;
fn current_tier(env, pool_id: u64) -> (u32 /*index*/, i128 /*unit price*/);
fn quote_commit(env, pool_id: u64, units: u32) -> i128;
fn payout_preview(env, pool_id: u64) -> (i128 /*supplier*/, i128 /*platform*/, i128 /*organizer*/, i128 /*refunds*/);
```

Implementation notes:
- **Escrow invariant:** `escrow_balance == sum(member.paid) − sum(refunds paid) − payouts − advance_paid` at all times; checked in debug builds and property tests.
- **Coverage invariant:** after every commit, `escrow_balance ≥ total_units × final_price_if_closed_now`.
- Per-pool member limit (default 200) to keep settlement bounded; settlement computes aggregates incrementally (store running sums at commit time) so `settle` is O(1); per-member refunds are computed lazily on claim / batch push.
- Shortfall allocation must be deterministic and documented in `pricing-math.md` with worked examples.
- Settlement with frozen disputes: pay supplier `payable − frozen`; remaining frozen amounts paid out by `apply_outcome`.

**disputes**
```rust
fn open(env, opener: Address, pool_id: u64, reason: DisputeReason, claimed_units: u32, evidence: BytesN<32>) -> u64;
fn add_evidence(env, party: Address, dispute_id: u64, evidence: BytesN<32>); // opener, supplier, organizer
fn resolve(env, arbiter: Address, dispute_id: u64, outcome: Outcome, reasoning_hash: BytesN<32>);
fn timeout(env, dispute_id: u64);                                          // permissionless after SLA
fn dispute(env, id: u64) -> Dispute;
fn open_disputes_of(env, pool_id: u64) -> Vec<u64>;
```
Arbiter cannot resolve a dispute on a pool they organized, supplied or joined (checked on-chain).

**supplier_bond**
```rust
fn deposit(env, supplier: Address, amount: i128);
fn withdraw(env, supplier: Address, amount: i128);     // only free bond (bond − outstanding advances)
fn reserve_for_advance(env, caller: Address, supplier: Address, amount: i128); // caller = group_buy
fn release_advance(env, caller: Address, supplier: Address, amount: i128);     // on settlement
fn slash(env, caller: Address, supplier: Address, amount: i128) -> i128;       // on fail_delivery
fn bond_of(env, supplier: Address) -> (i128 /*total*/, i128 /*reserved*/);
```

**reputation**
```rust
fn record(env, caller: Address, subject: Address, role: Role, event: Symbol); // caller ∈ {group_buy, disputes}
fn stats(env, subject: Address, role: Role) -> RepStats;
fn tier(env, subject: Address, role: Role) -> u32;
```
`RepStats` holds counters: pools_settled, pools_failed, on_time, late, disputes_opened, disputes_won, disputes_lost, short_deliveries, pickups_confirmed, first_active_at. Tier rules from Section 3.8, computed deterministically.

Cross-contract privileged calls: callee resolves caller via `config.get_address`, requires `caller.require_auth()` and equality.

### 6.5 Events
Every state change emits an event: topics `(contract_symbol, event_symbol, pool_id_or_address)`, data a `#[contracttype]` struct. Minimum set:

`user_registered, user_attested, user_revoked, user_suspended, user_unsuspended, pool_created, pool_cancelled, committed, commitment_increased, commitment_withdrawn, tier_reached, pool_filled, pool_expired, pool_accepted, pool_rejected, pool_failed, advance_paid, pool_dispatched, delivery_confirmed, shortfall_recorded, pickup_confirmed, pool_settled, refund_credited, refund_claimed, dispute_opened, dispute_evidence, dispute_resolved, dispute_timed_out, bond_deposited, bond_withdrawn, bond_slashed, rep_updated, tier_changed, params_updated, paused, unpaused, upgraded`.

Payloads documented in `docs/events.md`; the indexer depends on this file, so any change is breaking.

### 6.6 Storage and TTL
- Instance: config pointers, counters.
- Persistent: pools, commitments, member index pages, disputes, bonds, registry entries, rep stats.
- Temporary: none for money-related data.
- Extend TTL on every touch of persistent entries (helpers in `sp_common`); backend `ttl-extend` job extends all entries for non-final pools, open disputes, bonds, active users, and contract instances/code.
- Final pools (Settled/Refunded with all refunds claimed) may be allowed to archive after 180 days; document in `storage-and-ttl.md`.

### 6.7 Contract security requirements
- `require_auth()` on every function acting for an address.
- Checks-effects-interactions; state updated before token transfers.
- No unbounded loops: member lists paginated, refund pushes batched with `max`.
- Tier price ordering, MOQ = first tier, max ≥ MOQ validated in `create_pool`.
- Organizer cannot commit to their own pool beyond `max_member_share_bp` and cannot be the supplier.
- Supplier cannot commit to or organize a pool on their own offer.
- Category must be allowed at creation.
- Upgrade only via admin multisig; `upgraded` event with wasm hash.
- `docs/threat-model.md` must cover: organizer–supplier collusion (fake delivery confirmations → mitigated by member dispute window, pickup confirmations, early-release weight, reputation), tier sabotage (last-minute withdrawals → withdrawal lock), sybil members inflating tiers (T0 caps, KYC for larger amounts, max member share), arbiter capture (multisig-managed set, conflict checks, SLA timeout defaulting to disputer, public reasoning hashes), griefing disputes (deposit), supplier no-show with advance (bond), attestor key compromise, FX movement between offer refresh and pool close.

### 6.8 Testing (contracts)
- Unit tests for every function and error path.
- Scenario tests for every flow in Section 5, including: expire with refunds, tier improvement refunds, shortfall pro-rata, dispute each outcome type, dispute timeout, fail_accept, fail_delivery with and without advance/bond, early release, collective member confirmation.
- Property tests: escrow invariant; coverage invariant; sum of refunds + payouts + fees == total paid; no member pays more than `units × tier1_price`; final unit price is monotonic non-increasing in total units.
- Time-travel tests for every window.
- Budget tests: `commit`, `settle`, `claim_refund` stay under 50% of network CPU/memory limits; `push_refunds(max=25)` stays under 80%.
- Coverage ≥ 90% lines on `group_buy`, `disputes`, `supplier_bond`.

### 6.9 Bindings
- `scripts/bindings.sh` → `stellar contract bindings typescript` per contract into `bindings/`.
- Published as `@sorobanpool/contracts-<name>` (GitHub Packages) or consumed by git tag; backend/frontend pin versions.
- CI fails on stale bindings; CI exports `errors.json` and `events.json` artefacts.

---

## 7. Repo: `sorobanpool-backend`

### 7.1 Stack
- Node.js LTS, TypeScript strict, **NestJS**.
- PostgreSQL 16 + **Prisma**; Redis + **BullMQ**.
- `@stellar/stellar-sdk` (pinned) + generated bindings.
- Object storage (S3-compatible) for product images and evidence; images resized (sharp) to WebP thumbnails for low bandwidth.
- Zod for env and external inputs; class-validator DTOs.
- Pino logs, OpenTelemetry, Prometheus metrics, Sentry.
- Tests: Vitest/Jest, Supertest, Testcontainers (Postgres, Redis, MinIO, Stellar local).

### 7.2 Layout
```
sorobanpool-backend/
├── src/
│   ├── main.ts / app.module.ts
│   ├── config/
│   ├── common/
│   ├── auth/              # OTP, passkeys, JWT, sessions
│   ├── users/             # traders, organizers, suppliers, admins
│   ├── wallets/           # smart wallet deploy
│   ├── verification/      # KYC (traders/organizers), KYB (suppliers), attestor
│   ├── catalog/           # offers, products, images, categories, FX-refreshed USDC tiers
│   ├── pools/             # create/commit/close prepare, read models, share cards
│   ├── delivery/          # dispatch, hub confirmation, pickups, evidence
│   ├── disputes/          # evidence, arbiter workflow, SLA timers
│   ├── bonds/
│   ├── anchor/            # SEP-1/10/24/6
│   ├── fx/
│   ├── relayer/           # fee sponsorship
│   ├── indexer/
│   ├── keeper/
│   ├── notifications/     # SMS, WhatsApp, push, email; templates EN + PCM
│   ├── evidence/          # upload, hashing, access control
│   ├── sharecards/        # OG images for WhatsApp links
│   ├── admin/
│   ├── risk/
│   └── health/
├── prisma/schema.prisma, migrations/
├── test/
├── docker-compose.yml     # postgres, redis, minio, api, worker, indexer
├── Dockerfile
├── .env.example
└── docs/ (api.md / OpenAPI, runbooks/, decisions/)
```
One codebase, roles via `ROLE=api|worker|indexer`.

### 7.3 Data model (Prisma, abridged)

```prisma
model User {
  id            String   @id @default(cuid())
  phone         String   @unique
  displayName   String?
  language      Lang     @default(EN)      // EN, PCM; later HA, YO, IG
  walletAddress String?  @unique
  roles         UserRole[]
  status        String
  createdAt     DateTime @default(now())
  passkeys      Passkey[]
}
model UserRole { userId String; role String /* TRADER|ORGANIZER|SUPPLIER|ARBITER|ADMIN */; level Int @default(0); verHash String?; @@id([userId, role]) }
model Passkey { id String @id; userId String; credentialId String @unique; publicKey Bytes; counter Int; createdAt DateTime @default(now()) }

model TraderProfile { userId String @id; market String?; state String; lga String; cluster String? }
model SupplierProfile {
  userId String @id; businessName String; cacNumber String; address String; state String; lga String
  categories String[]; deliveryAreas Json; bankName String?; bankAccountMasked String?
  kybStatus String; kybRecordId String?
}
model VerificationRecord { id String @id @default(cuid()); userId String; kind String /* KYC|KYB */; provider String; providerRef String; result Json /* encrypted */; status String; createdAt DateTime @default(now()) }

model Offer {
  id String @id @default(cuid())
  supplierId String
  title String; brand String?; description String
  unitLabel String; category String; perishable Boolean
  images String[]
  tiersNgn Json            // [{minUnits, priceNgn}]
  tiersUsdc Json           // [{minUnits, priceUsdc}] at fxQuoteId
  fxQuoteId String
  moq Int; maxUnits Int; maxPerMember Int; leadTimeHours Int
  deliveryAreas Json
  validUntil DateTime
  offerHash String         // sha256 of canonical JSON
  status String            // DRAFT|LIVE|EXPIRED|TAKEN_DOWN
  createdAt DateTime @default(now()); updatedAt DateTime @updatedAt
}

model Pool {               // read model + off-chain extras
  id BigInt @id
  offerId String
  organizerAddress String; supplierAddress String
  hubAddress String; hubContact String; hubHash String
  pickupWindow Json
  state String
  totalUnits Int; receivedUnits Int?
  currentTierIdx Int; currentUnitPrice Decimal
  fillDeadline DateTime
  escrowBalance Decimal; frozenAmount Decimal
  shareSlug String @unique
  lastEventLedger Int
}
model Commitment { poolId BigInt; memberAddress String; units Int; paid Decimal; allocatedUnits Int?; refundDue Decimal?; refundClaimed Boolean @default(false); pickedUp Boolean @default(false); @@id([poolId, memberAddress]) }
model Dispute { id BigInt @id; poolId BigInt; openerAddress String; reason String; claimedUnits Int; claimedAmount Decimal; state String; outcome Json?; arbiterAddress String?; slaDueAt DateTime; lastEventLedger Int }
model Evidence { id String @id @default(cuid()); ownerId String; poolId BigInt?; disputeId BigInt?; kind String /* DELIVERY|DISPUTE|WAYBILL */; objectKey String; sha256 String; mime String; createdAt DateTime @default(now()) }
model Bond { supplierAddress String @id; total Decimal; reserved Decimal; lastEventLedger Int }
model AnchorTransfer { id String @id; userId String; kind String; anchor String; anchorTxId String; amountNgn Decimal?; amountUsdc Decimal?; status String; poolId BigInt?; createdAt DateTime @default(now()); updatedAt DateTime @updatedAt }
model FxQuote { id String @id @default(cuid()); pair String; rate Decimal; sources Json; createdAt DateTime @default(now()) }
model ChainEvent { id String @id; ledger Int; contract String; topic String; payload Json; processedAt DateTime? }
model IndexerCursor { id String @id; lastLedger Int; updatedAt DateTime @updatedAt }
model Notification { id String @id @default(cuid()); userId String; channel String; template String; payload Json; status String; sentAt DateTime? }
model AuditLog { id String @id @default(cuid()); actorId String; action String; target String; data Json; at DateTime @default(now()) }
```
On-chain-derived tables are read models rebuildable from `ChainEvent`. PII and bank details encrypted at the application layer (AES-256-GCM, key from a secret manager/KMS).

### 7.4 API (REST under `/v1`, OpenAPI generated)

Auth: `POST /auth/otp/request`, `POST /auth/otp/verify`, passkey register/login options + verify, `POST /auth/logout`.

Users & verification: `GET /me`, `PATCH /me`, `POST /wallets`, `POST /kyc/start`, `GET /kyc/status`, `POST /suppliers/apply`, `GET /suppliers/me`.

Catalog
- `GET /offers?state=&lga=&category=&q=` (public, cached)
- `GET /offers/:id`
- `POST /offers` / `PATCH /offers/:id` / `POST /offers/:id/publish` / `POST /offers/:id/refresh-fx` (supplier)
- `POST /uploads/sign` → pre-signed upload URL (images, evidence), with size/MIME limits

Pools
- `POST /pools/prepare` `{offerId, fillDeadline, hub, pickupWindow, organizerFeeBp, clusterOnly}` → tx to sign
- `GET /pools/:id`, `GET /p/:shareSlug` (public preview), `GET /pools?mine=true`
- `GET /pools/:id/quote?units=` → amount now, expected final, next break
- `POST /pools/:id/commit/prepare` `{units, method: "USDC"|"NGN"}`
- `POST /pools/:id/increase/prepare`, `POST /pools/:id/withdraw/prepare`
- `POST /pools/:id/close-early/prepare` (organizer)
- `POST /pools/:id/accept/prepare`, `/reject/prepare`, `/dispatch/prepare` (supplier)
- `POST /pools/:id/delivery/prepare` `{receivedUnits, evidenceIds[]}` (organizer)
- `POST /pools/:id/pickup/prepare`
- `POST /pools/:id/refund/prepare`
- `GET /pools/:id/sharecard.png`

Disputes: `POST /disputes/prepare`, `POST /disputes/:id/evidence/prepare`, `GET /disputes/:id`, `GET /disputes?mine=true`.

Bonds: `POST /bonds/deposit/prepare`, `POST /bonds/withdraw/prepare`, `GET /bonds/me`.

Tx: `POST /tx/submit` `{signedPayload, idempotencyKey}`.

Anchor: `POST /anchor/deposit/start`, `POST /anchor/withdraw/start`, `GET /anchor/transfers/:id`, callback/polling handler.

Arbiter (role ARBITER): `GET /arbiter/queue`, `GET /arbiter/disputes/:id` (evidence via short-lived signed URLs), `POST /arbiter/disputes/:id/resolve/prepare` (arbiter signs with own wallet; server never holds arbiter keys).

Admin: KYB/KYC queues and decisions, offer moderation, category list, suspensions, parameter proposals (builds multisig XDR), risk overview, audit log.

Realtime: WebSocket channel per pool (progress, tier reached, state changes) and per user.

All mutating endpoints: rate-limited, idempotency keys, audit-logged when admin/arbiter.

### 7.5 Relayer / fee sponsorship
- Users never hold XLM. Backend simulates, assembles, **fee-bumps** and submits; client only signs Soroban auth entries with passkey (traders/organizers/suppliers) or a connected wallet (arbiters/admin signers).
- Allow-list of sponsorable contract functions; per-user daily caps; simulation must pass.
- Evaluate an existing Soroban relay/fee-sponsorship service before building; ADR.
- Sponsor balance alert at < 7 days runway.

### 7.6 Indexer
- Poll Soroban RPC `getEvents` for all contract IDs from cursor; store raw in `ChainEvent` idempotently; project to read models transactionally; push WebSocket updates.
- Handle RPC event retention: alert if behind retention window; document backfill source.
- Metric `indexer_lag_ledgers`; alert > 20.

### 7.7 Keeper jobs

| Job | Schedule | Action |
|---|---|---|
| `close-pools` | 1 min | `close` pools past fill deadline. |
| `fail-accept` | 5 min | `fail_accept` pools past accept window. |
| `fail-delivery` | 10 min | `fail_delivery` past delivery deadline. |
| `settle` | 5 min | `settle` pools past confirmation window with no open disputes, or early-release weight reached. |
| `push-refunds` | 2 min | `push_refunds(max=25)` for pools with unclaimed refunds. |
| `dispute-timeout` | 10 min | `timeout` disputes past SLA. |
| `reminders` | 5 min | Deadline, price-break, ready-for-pickup, confirm-delivery reminders. |
| `offer-expiry` | 15 min | Expire offers past `validUntil`; notify suppliers to refresh. |
| `fx-refresh` | 1 min | Refresh NGN/USD from ≥ 2 sources; flag > 2% divergence; block offer publishing on divergence. |
| `ttl-extend` | hourly | Extend TTL of live entries and contract instances/code. |
| `anchor-poll` | 1 min | Track anchor transfers; on completed deposit tied to a pending commit, notify user to finish/auto-finish commit. |
| `sponsor-balance` | 15 min | Sponsor XLM balance check. |
| `reconcile` | daily | On-chain escrow totals vs read models; alert on mismatch. |

### 7.8 Verification
- Provider adapter interface: `verifyBvn`, `verifyNin`, `liveness`, `faceMatch`, `verifyCac`, `verifyBankAccountName`. Candidate Nigerian providers to evaluate: Smile ID, Dojah, Prembly (verify current offerings/pricing; ADR).
- On-chain only `ver_hash = sha256(userId || providerRef || level || salt)`.
- Attestor key in a secret manager / HSM-backed signer; only `registry.attest`; daily caps + spike alerts.
- Supplier KYB: CAC lookup, director KYC, bank account name must match business or director name, manual review of address (photo/video call) before first offer.

### 7.9 Catalog rules
- Category allow-list mirrors on-chain `config` categories. Initial allowed: packaged provisions (rice, beans, garri, oil, sugar, noodles, seasoning), beverages (non-alcoholic), toiletries/household, fabric/textiles, phone accessories, stationery, building materials (cement, roofing sheets), packaging materials.
- Offers require ≥ 2 photos, unit label, brand (if branded), tiers, MOQ, lead time.
- Automated checks: tier ordering, price sanity vs category median (flag outliers for review), banned keywords.
- Moderation takedown only affects new pools.

### 7.10 Anchor and FX
- SEP-1 discovery, SEP-10 web auth (verify contract-account support; consider SEP-45; ADR), SEP-24 and/or SEP-6 for NGN deposits (traders) and withdrawals (suppliers' payouts, organizers' fees, refunds to bank).
- `AnchorProvider` abstraction; support multiple anchors.
- FX quotes shown are indicative; anchor quote at execution is final; quotes expire in 60 s.

### 7.11 Notifications
- SMS (primary), WhatsApp (opt-in, also used for share links), Web Push, email (suppliers/admin).
- Templates EN + PCM, ≤ 160 chars for SMS, quiet hours 21:00–07:00 WAT except delivery-day messages.
- Key templates: joined, price break reached, deadline soon, pool filled/expired (refund), supplier accepted, dispatched, goods ready for pickup, confirm pickup, settled (your refund), dispute updates.

### 7.12 Security (backend)
- Helmet, CORS allow-list, CSRF for cookie sessions, JWT 15 min + rotating refresh.
- Secrets from a secret manager; Pino redaction (phone, BVN, NIN, CAC, account numbers).
- Evidence access: only pool participants, the arbiter assigned, and admins, via 5-minute signed URLs; uploads virus-scanned; EXIF location stripped from public product images (kept for evidence, access-controlled).
- Admin/arbiter: hardware-key 2FA, IP allow-list, full audit.
- Dependabot/Renovate, `npm audit`, CodeQL. Daily DB backup, PITR, monthly restore test.

---

## 8. Repo: `sorobanpool-frontend`

### 8.1 Stack
- **Next.js** (App Router) + TypeScript strict; Tailwind + shadcn/ui (Radix).
- TanStack Query; Zustand for small client state.
- PWA (service worker via a maintained Next-compatible lib — verify), installable, offline shell.
- `next-intl` with `en` and `pcm`; ready for `ha`, `yo`, `ig`.
- Passkeys: `@simplewebauthn/browser` + chosen passkey smart-wallet SDK.
- Arbiter/admin signing: a multi-wallet Stellar kit (e.g. Stellar Wallets Kit — verify) with Freighter and hardware-wallet support.
- Image handling: Next Image with backend thumbnails; camera capture for evidence (`<input capture>`), client-side compression before upload.
- Tests: Vitest + Testing Library, Playwright, Lighthouse CI, axe.

### 8.2 Layout
```
sorobanpool-frontend/
├── app/
│   ├── (trader)/
│   │   ├── page.tsx               # Home: my pools, pools near me, ready for pickup
│   │   ├── onboarding/
│   │   ├── explore/               # offers + open pools near me
│   │   ├── p/[slug]/              # public pool page (WhatsApp landing)
│   │   ├── pools/[id]/            # join/commit, status timeline, pickup, refund, report problem
│   │   ├── organize/              # create pool wizard, my organized pools, delivery confirmation
│   │   ├── disputes/
│   │   ├── history/
│   │   └── settings/
│   ├── supplier/                  # portal: onboarding, offers, pools to accept, dispatch, payouts, bond, ratings
│   ├── arbiter/                   # queue, dispute workspace
│   └── admin/
├── components/
│   ├── ui/
│   ├── money/                     # <Amount>, <NairaEstimate>, <PriceBreaks>, <RefundEstimate>
│   ├── pool/                      # <ProgressToMoq>, <TierLadder>, <PoolTimeline>, <Countdown>, <ShareButton>
│   ├── evidence/                  # <EvidenceCapture>, <EvidenceGallery>
│   └── feedback/                  # <TxStatus>, <ErrorExplainer>
├── lib/ (api client from OpenAPI, passkey, stellar, errors map, format)
├── messages/en.json, messages/pcm.json
├── public/ (manifest, icons)
└── tests/e2e/
```

### 8.3 UX principles
- **No crypto jargon** for traders/organizers/suppliers: "pay", "money held safely until delivery", "refund", "confirm with your fingerprint/face". Dollar value appears only in an info sheet and in the price-fixed explanation.
- The **core trust message** appears on every pool page and payment screen: *"Your money is held safely. The supplier is paid only after the goods arrive. If the group doesn't fill, you get everything back automatically."*
- **Tier ladder** UI: shows each price break, where the pool is now, units to the next break; motivates sharing.
- Before paying, always show: units, max you pay now, expected final price, expected refund, deadline, hub, pickup window, supplier rating, organizer, what happens if it fails, and dispute rights.
- **Pool timeline** with clear current step: Joining → Filled → Supplier accepted → On the way → Arrived at hub → Collected → Done.
- WhatsApp-first sharing: one-tap share with share card image and pre-filled Pidgin/English text.
- Big tap targets (≥ 48px), 360px-wide layouts, light theme default for sunlight, WCAG 2.1 AA.
- Performance budget: ≤ 170 KB gzipped JS on trader routes; LCP < 2.5 s on slow 3G (Lighthouse CI). Public pool page (`/p/[slug]`) server-rendered and fast for link previews.
- Offline: cached shell + last known pool states; no double submits (idempotency keys).
- Contract error codes mapped to plain language with a "what to do now" line.

### 8.4 Key screens
**Trader:** Home; Explore (offers near me, open pools); Pool page (join, tier ladder, progress, members count, countdown, share); Pay (units, method, summary, confirm); My pool status (timeline, pickup details, I've collected, refund status, Report a problem); Dispute (reason, units, photos, deposit explanation); History; Settings (language, notifications, bank for refunds, help).

**Organizer:** Create pool wizard (choose offer → deadline → hub & pickup window → fee → market-only → review → confirm → share); Organized pools dashboard (progress, members, messages); Delivery confirmation (count received units, capture photos/video, confirm); Pickup checklist (members, units each, mark collected on their behalf only with member's in-app confirmation).

**Supplier portal:** Onboarding/KYB; Offers (create/edit with NGN tiers and live USDC preview, refresh FX); Pools (filled → accept/reject within countdown; accepted → dispatch with waybill photo; delivered; settled); Payouts (to bank via anchor or keep USDC); Bond; Ratings and dispute history.

**Arbiter console:** Queue sorted by SLA; dispute workspace with pool terms snapshot, timeline, all evidence, both parties' statements, outcome picker with payout preview, reasoning note (hashed on-chain, text stored off-chain), sign with wallet.

**Admin console:** KYB/KYC review, offer moderation, categories, suspensions, arbiters, parameters (multisig XDR), risk overview (GMV, escrow held, failure rate, dispute rate by supplier/category/state, organizer–supplier pair concentration to spot collusion), system health, audit log.

### 8.5 Frontend security
- No secrets in bundle; strict CSP; sanitize user text; passkeys only on production RP ID; supplier, arbiter and admin areas behind role checks server-side; admin/arbiter on a separate subdomain.

---

## 9. Environments and configuration

| Env | Network | Purpose |
|---|---|---|
| `local` | Stellar local quickstart or testnet | Dev |
| `staging` | testnet | Integration, demos, reviewers |
| `production` | mainnet | Only after Milestone 6 |

Minimum env vars:

**contracts:** `NETWORK`, `RPC_URL`, `NETWORK_PASSPHRASE`, `DEPLOYER_SECRET` (local only), `ADMIN_ADDRESS`, `USDC_CONTRACT_ID`, `TREASURY_ADDRESS`, `MAINNET_CONFIRM`.

**backend:** `ROLE`, `DATABASE_URL`, `REDIS_URL`, `S3_ENDPOINT`, `S3_BUCKET_PUBLIC`, `S3_BUCKET_EVIDENCE`, `S3_ACCESS_KEY_REF`, `S3_SECRET_KEY_REF`, `JWT_SECRET`, `JWT_REFRESH_SECRET`, `ENCRYPTION_KEY_ID`, `STELLAR_NETWORK`, `RPC_URL`, `HORIZON_URL`, `NETWORK_PASSPHRASE`, `DEPLOYMENTS_FILE`, `SPONSOR_SECRET_REF`, `ATTESTOR_SECRET_REF`, `KYC_PROVIDER`, `KYC_API_KEY_REF`, `SMS_PROVIDER`, `SMS_API_KEY_REF`, `WHATSAPP_API_KEY_REF`, `ANCHOR_DOMAIN`, `FX_SOURCES`, `WEBAUTHN_RP_ID`, `WEBAUTHN_ORIGIN`, `PUBLIC_APP_URL`, `SENTRY_DSN`, `OTEL_EXPORTER_OTLP_ENDPOINT`.

**frontend:** `NEXT_PUBLIC_API_URL`, `NEXT_PUBLIC_WS_URL`, `NEXT_PUBLIC_STELLAR_NETWORK`, `NEXT_PUBLIC_RP_ID`, `NEXT_PUBLIC_APP_NAME`, `NEXT_PUBLIC_SENTRY_DSN`.

`*_REF` values point to secret-manager entries in staging/production.

---

## 10. Cross-repo contracts

1. **Contracts → others:** pinned TS bindings + `deployments/<network>.json` (IDs, wasm hashes, commit).
2. **Events:** `docs/events.md` + `events.json`; backend indexer has fixture tests for every event.
3. **Errors:** `errors.json` → frontend `lib/errors/map.ts`; CI fails if any code lacks EN or PCM text.
4. **Pricing math parity:** a shared JSON test-vector file (`pricing-vectors.json`, generated by contracts CI from property tests) that backend quote logic and frontend display logic must reproduce exactly.
5. **API:** backend OpenAPI + generated TS client package for frontend.

All versioned with semver; breaking changes need migration notes in each affected repo.

---

## 11. Testing strategy

### 11.1 Contracts — see 6.8.

### 11.2 Backend
- Unit tests (quotes, FX conversion, catalog validation, share cards).
- **Pricing parity:** backend quote/refund/shortfall calculations match `pricing-vectors.json` (≥ 500 vectors) and a local-network contract run in CI.
- Integration via Testcontainers (Postgres, Redis, MinIO, Stellar local).
- Indexer replay tests; keeper job tests with time-shifted local network.
- Provider mocks with record/replay fixtures (KYC/KYB, SMS, WhatsApp, anchor).

### 11.3 Frontend
- Component tests for money, tier ladder, refund estimate, timeline states.
- i18n completeness; axe accessibility on key screens.

### 11.4 End-to-end (staging/testnet, Playwright, nightly + release)
Trader onboarding; supplier onboarding with KYB sandbox and offer publish; organizer creates pool and shares; multiple traders commit across a price break; pool expires (refunds); pool fills → accept → dispatch → delivery confirm → pickups → settle with tier refunds; shortfall delivery; dispute → each outcome; dispute timeout; fail_accept; fail_delivery; naira deposit and supplier naira payout via testnet anchor; admin suspension. Use a staging param profile with short windows (minutes) for time-dependent paths.

### 11.5 Load
- k6: 1,000 concurrent viewers on one public pool page, 100 commits within 5 minutes to one pool, 200 pools settling in the same hour. API p95 < 300 ms for non-chain endpoints; public pool page TTFB < 500 ms.

---

## 12. CI/CD and deployment

### 12.1 CI (every PR)
- **contracts:** fmt, clippy `-D warnings`, wasm build, tests, coverage, wasm size report, bindings freshness, export `errors.json`, `events.json`, `pricing-vectors.json`.
- **backend:** lint, typecheck, unit + integration tests, Prisma migration check, OpenAPI breaking-change diff, Docker build, CodeQL.
- **frontend:** lint, typecheck, unit tests, build, Lighthouse CI budgets, Playwright smoke on preview.

### 12.2 Deploy
- **Contracts:** `scripts/deploy.sh NETWORK=testnet` → upload, deploy with constructors, `init.sh` wiring, write `deployments/testnet.json`, open PR. Mainnet needs `MAINNET_CONFIRM=yes`, clean tree, release tag, audit report hash in `deployments/mainnet.json`, admin multisig for init.
- **Backend:** Docker images; staging on a simple container host (e.g. Fly.io/Render/Railway), production on a provider with private networking (ADR). Managed Postgres, Redis, S3-compatible storage. Migrations as release step.
- **Frontend:** Vercel (previews per PR, staging, production).
- Release tags `vX.Y.Z` across repos; conventional-commit changelogs.

### 12.3 Observability
- Sentry, OpenTelemetry, Prometheus/Grafana (or provider equivalent), public status page.
- Alerts: indexer lag; keeper failures; sponsor balance; attestor spike; reconcile mismatch; escrow held > configured threshold; failure rate 7d > 5%; dispute rate 7d > 8%; arbitration SLA breaches; FX source divergence; API error rate > 2%.

---

## 13. Compliance, risk and disclosures (before public launch)

This lists what must exist; it is **not** legal advice. Engage a Nigerian fintech/commercial lawyer before mainnet.

1. **Escrow / payments regulation:** determine whether holding funds in escrow and facilitating payments requires a CBN payment licence (e.g. as a payment service provider or via a licensed partner) and how a smart-contract escrow is treated. Keep a `licensedPartner` integration point in the backend.
2. **Digital asset rules:** confirm SEC Nigeria's current position on VASPs/stablecoins and which regulated entity (e.g. the anchor) covers the fiat ↔ USDC leg.
3. **KYC/KYB/AML:** trader KYC thresholds, supplier KYB, sanctions/PEP screening for suppliers above thresholds, transaction monitoring (unusual pool sizes, repeated organizer–supplier pairs, rapid in/out), suspicious activity runbook.
4. **Consumer protection (FCCPC) and marketplace terms:** clear refund rules, dispute process and timelines, organizer and platform fee disclosure, supplier obligations (accurate descriptions, quantity, quality, delivery times).
5. **Product rules:** category allow-list only; no regulated goods in v1 (NAFDAC-regulated drugs, alcohol, tobacco, agrochemicals, infant formula, etc.). Food items must be sealed/packaged from verified suppliers.
6. **Tax:** suppliers responsible for VAT/invoicing; platform provides payout statements; confirm platform's own VAT/withholding obligations on fees.
7. **Data protection (NDPA 2023):** privacy policy, consent capture, minimisation, retention schedule (evidence retained 1 year after pool close unless in dispute/legal hold), DPO contact, breach runbook. No PII on-chain.
8. **FX disclosure:** prices fixed in dollar value at offer refresh; naira estimates are indicative; anchor rate at payment is final.
9. **Terms of Service** for traders, organizers, suppliers, arbiters; versioned acceptance stored.
10. **Brand clearance:** SDF consent for "Soroban" in the name, or rename (Section 1.5).

---

## 14. Engineering conventions

### 14.1 Git
- `main` protected; branches `feat/…`, `fix/…`, `chore/…`; Conventional Commits; squash merges.
- PR template: summary, linked issue, screenshots (UI), test evidence, risk notes, docs updated.

### 14.2 Code style
- Rust: rustfmt, clippy; no `unwrap()`/`expect()` in contract code paths.
- TS: ESLint + Prettier, strict; no `any` without justification comment.

### 14.3 Docs per repo
`README.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `docs/architecture.md`, `docs/decisions/`.

### 14.4 ADRs
`docs/decisions/NNNN-title.md` (Context, Decision, Alternatives, Consequences). Required for: smart wallet choice; fee sponsorship; anchor; KYC/KYB provider; object storage and hosting; SEP-10 vs SEP-45; pool ID scheme; member-limit per pool; refund push vs pull defaults.

### 14.5 Issues for open-source contributors (Drips Wave and beyond)
- Labels: `good first issue`, `contracts`, `backend`, `frontend`, `docs`, `i18n`, `security`, `complexity:S|M|L`.
- Each issue: context, acceptance criteria, likely files, test requirements.
- Escrow math, settlement, disputes, bond, relayer, attestor and auth work is maintainer-reviewed and never `good first issue`.
- Great contributor areas: i18n (Hausa, Yoruba, Igbo), share card designs, accessibility, docs/examples, test vectors, admin dashboards, indexer projections.

---

## 15. Milestones (build order)

Each ends with a tagged release, a staging demo, and updated docs.

### M0 — Foundations (week 1)
- Org + 3 repos, licence (Apache-2.0 suggested — confirm with founder), templates, CI skeletons, `.env.example`, ADR folder.
- Empty contract compiles; Nest app with health, Postgres, Redis, MinIO via docker-compose; Next app with i18n, Tailwind, PWA manifest, deployed preview.
- **Done when:** all three CIs green on `main`.

### M1 — Core contracts (weeks 2–4)
- `sp_common`, `config`, `registry`, `reputation`, `group_buy` (full state machine incl. tiers, shortfall, settlement, refunds), `disputes`, `supplier_bond` (with `advance_enabled=false` default but fully tested).
- Docs: `state-machine.md`, `pricing-math.md`, `events.md`, `errors.md`, `storage-and-ttl.md`, `threat-model.md` v1.
- Testnet deploy; bindings and artefacts published.
- **Done when:** scenario and property tests in 6.8 pass; coverage targets met.

### M2 — Backend core (weeks 4–6)
- Auth (OTP + passkeys), wallets, verification adapters (sandbox) + attestor, catalog with FX, pools APIs, relayer, indexer, keeper jobs, evidence uploads, notifications (SMS sandbox), share cards.
- **Done when:** a script can onboard a supplier and 5 traders, publish an offer, create a pool, commit across a price break, fill, accept, dispatch, confirm delivery with a shortfall, settle and refund — all via API on testnet.

### M3 — Trader & organizer PWA (weeks 6–8)
- Onboarding, explore, public pool page, join/pay (USDC path), timeline, pickup, refunds, organizer wizard, delivery confirmation, EN + PCM.
- **Done when:** E2E for 5.1, 5.3–5.5, 5.7 pass on throttled mobile; Lighthouse budgets met.

### M4 — Supplier portal + disputes (weeks 8–10)
- Supplier onboarding/KYB flow, offers with tiers and FX preview, accept/dispatch, payouts view, bond; dispute flows for traders/organizers/suppliers; arbiter console.
- **Done when:** E2E for 5.2, 5.6, 5.8, 5.9 pass.

### M5 — Naira rails + admin + risk (weeks 10–12)
- Anchor integration (testnet anchor) for trader deposits, supplier/organizer payouts, refunds to bank; admin console; risk dashboards; statements.
- **Done when:** naira-path flows work end to end on testnet; admin can run KYB → offer moderation → suspension cycle.

### M6 — Hardening & audit (weeks 12–16)
- Internal security review vs threat model; fuzz/property expansion; external contract audit; backend/frontend pen test; load test; DR drill; compliance checklist (Section 13) signed off; brand clearance.
- **Done when:** audit findings closed or accepted with rationale; audit hash in `deployments/mainnet.json`.

### M7 — Pilot
- Mainnet with conservative caps (S0/O0/T0–T1 limits, advances disabled), one market (e.g. one Abuja market association), 3–5 verified suppliers, 2–3 categories (e.g. rice, cooking oil, beverages). Weekly review of disputes and failures; widen caps from data.

---

## 16. Definition of done (any feature)

- Requirements met; deviations documented (ADR or PR description).
- Tests written and passing; E2E updated if a flow changed; pricing vectors updated if math changed.
- No new lint/type errors; no secrets; no PII in logs or on-chain.
- Docs updated (README, state machine, events/errors, API, i18n EN + PCM).
- At least one maintainer review; two for security-sensitive code (escrow, settlement, disputes, bond, auth, relayer, attestor).

---

## 17. Open questions (resolve early; record as ADRs)

1. Passkey smart-wallet implementation (maturity, audit status, SDK) vs an embedded/MPC wallet for v1.
2. NGN anchor(s) supporting contract (C-) accounts for SEP-24/6; fees, limits, settlement speed; supplier payout speed in naira.
3. Licensing structure for escrow/payments in Nigeria (own licence vs licensed partner).
4. Do suppliers want USDC-fixed pricing, or must we support naira-fixed pricing with FX buffers (e.g. collect 2% extra, refund the difference) until a regulated NGN stablecoin on Stellar exists?
5. Arbitration model: in-house ops only for v1, or a vetted external arbiter pool (market association elders, trade-dispute mediators)?
6. Should organizers be required to hold a small bond as well (anti-collusion), or is reputation + member confirmations sufficient for v1?
7. Perishables (tomatoes, peppers, fresh produce): include in pilot with shorter windows, or exclude until v2?
8. Final product name (Section 1.5).

---

*End of brief.*
