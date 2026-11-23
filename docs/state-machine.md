# Pool state machine

> For step-by-step sequence diagrams of each flow (happy path, expiry, shortfall, dispute, supplier failure) see [`docs/flows.md`](flows.md).

```
Open ──close/close_early/max reached──▶ Filled ──accept──▶ Accepted ──dispatch──▶ Dispatched
 │                                        │                  │                        │
 │ close (deadline, total < MOQ)          │ reject /         │ fail_delivery /        │ confirm_delivery /
 ▼                                        │ fail_accept      │ cancel (admin)         │ member_confirm_delivery
Expired                                   ▼                  ▼                        ▼
                                        Failed             Failed/Cancelled        Delivered ──settle──▶ Settled
Open/Filled/Accepted ──cancel_pool──▶ Cancelled                                      (frozen disputes held back)
```
Refundable end states: `Expired`, `Failed`, `Cancelled` (full refund) and `Settled` (price-tier / shortfall refunds and dispute credits). Refunds are pull (`claim_refund`) with a permissionless bounded push (`push_refunds`). A dispute credit applied after settlement rewinds the push cursor so the keeper pays it too.

Supplier failures (`reject`, `fail_accept`, `fail_delivery`) record `failed` in `reputation`; a second failure within 90 days of the previous one suspends the supplier in `registry` (brief 5.9).

Registry roles may be closed by an admin and re-registered with a fresh profile hash. The
new registration resets registry verification only; reputation records remain separate.

## State transitions (`group_buy`)

The table below lists every state-changing function in [`contracts/group_buy/src/lib.rs`](../contracts/group_buy/src/lib.rs), its source and destination states, allowed callers, and required preconditions.

| From State | To State | Function | Caller (Who) | Preconditions |
|---|---|---|---|---|
| *Uninitialized* | `Open` (contract initialized) | [`__constructor`](../contracts/group_buy/src/lib.rs) | Deployer | Valid `config` address, contract not yet initialized. |
| *None* | `Open` | [`create_pool`](../contracts/group_buy/src/lib.rs) | Organizer | Organizer verified in `registry`, supplier verified and active, allowed category, valid tiers (1-5 ascending MOQ/descending price), deadline ≤ max fill window, pool value within caps, fee ≤ max. |
| `Open`, `Filled`, `Accepted` | `Cancelled` | [`cancel_pool`](../contracts/group_buy/src/lib.rs) | Admin (via Config) | Config admin authorization; transitions pool to `Cancelled`; enables 100% principal refunds. |
| `Open` | `Open` / `Filled` | [`commit`](../contracts/group_buy/src/lib.rs) | Member | Pool is `Open`, current time < deadline, member is registered trader and matches pool cluster, units within member and pool limits. Transitions to `Filled` if `max_units` reached. |
| `Open` | `Open` / `Filled` | [`increase`](../contracts/group_buy/src/lib.rs) | Member | Pool is `Open`, current time < deadline, member already committed, additional units fit limits. Transitions to `Filled` if `max_units` reached. |
| `Open` | `Open` | [`withdraw_commitment`](../contracts/group_buy/src/lib.rs) | Member | Pool is `Open`, current time ≤ deadline - lock window (2h), remaining total must not drop below any lower tier boundary (ADR 0001). |
| `Open` | `Filled` | [`close_early`](../contracts/group_buy/src/lib.rs) | Organizer | Pool is `Open`, organizer authorization, total committed units ≥ MOQ. |
| `Open` | `Filled` / `Expired` | [`close`](../contracts/group_buy/src/lib.rs) | Anyone | Pool is `Open`, current time ≥ fill deadline. Reaches `Filled` if `total >= MOQ`, else `Expired`. |
| `Filled` | `Accepted` | [`accept`](../contracts/group_buy/src/lib.rs) | Supplier | Pool is `Filled`, supplier authorization, current time ≤ accept deadline. Optional advance paid if supplier bond covers it. |
| `Filled` | `Failed` | [`reject`](../contracts/group_buy/src/lib.rs) | Supplier | Pool is `Filled`, supplier authorization. Marks supplier failure in `reputation`. |
| `Filled` | `Failed` | [`fail_accept`](../contracts/group_buy/src/lib.rs) | Anyone | Pool is `Filled`, current time > accept deadline. Supplier failed SLA. |
| `Accepted` | `Dispatched` | [`dispatch`](../contracts/group_buy/src/lib.rs) | Supplier | Pool is `Accepted`, supplier authorization. |
| `Accepted`, `Dispatched` | `Failed` | [`fail_delivery`](../contracts/group_buy/src/lib.rs) | Anyone | Pool is `Accepted` or `Dispatched`, current time > `accepted_at + lead_time + grace_period`. Slashes supplier bond if advance was disbursed. |
| `Accepted`, `Dispatched` | `Delivered` | [`confirm_delivery`](../contracts/group_buy/src/lib.rs) | Organizer | Pool is `Accepted` or `Dispatched`, organizer authorization, `received_units <= total_units`. |
| `Accepted`, `Dispatched` | `Delivered` | [`member_confirm_delivery`](../contracts/group_buy/src/lib.rs) | Member | Pool is `Accepted` or `Dispatched`, current time > organizer confirm window; confirms sum to ≥ 50% of units. |
| `Delivered` | `Delivered` | [`allocate_shortfall`](../contracts/group_buy/src/lib.rs) | Anyone | Pool is `Delivered` with `received < total`; processes members in paged batches of up to 50 until all members allocated. |
| `Delivered`, `Settled` | `Delivered`, `Settled` | [`confirm_pickup`](../contracts/group_buy/src/lib.rs) | Member | Pool is `Delivered` or `Settled`, member committed to pool; records member pickup acknowledgement. |
| `Delivered` | `Settled` | [`settle`](../contracts/group_buy/src/lib.rs) | Anyone | Pool is `Delivered`, shortfall allocation complete (if any), and confirmation window passed (or early release pickup weight met). Disburses payable amount minus frozen disputes. |
| `Expired`, `Failed`, `Cancelled`, `Settled` | `Expired`, `Failed`, `Cancelled`, `Settled` | [`claim_refund`](../contracts/group_buy/src/lib.rs) | Member | Pool in refundable end state. Member pulls their owed refund (overpayment tier savings, shortfall, or full refund). |
| `Expired`, `Failed`, `Cancelled`, `Settled` | `Expired`, `Failed`, `Cancelled`, `Settled` | [`push_refunds`](../contracts/group_buy/src/lib.rs) | Anyone (Keeper) | Pool in refundable end state. Pushes refunds to a batch of up to `batch_size` members starting at `refund_cursor`. |
| `Expired`, `Failed`, `Cancelled`, `Settled` | `Expired`, `Failed`, `Cancelled`, `Settled` | [`sweep_dust`](../contracts/group_buy/src/lib.rs) | Anyone | Pool in refundable end state, `frozen_amount == 0`, contract holds excess escrow beyond `owed_total - refunds_paid`. Sends excess dust to treasury. |
| `Delivered` | `Delivered` | [`freeze`](../contracts/group_buy/src/lib.rs) | `disputes` contract | Caller is authorized disputes contract address, pool is `Delivered` within confirm window, locks `amount` into `frozen_amount`. |
| `Delivered`, `Settled` | `Delivered`, `Settled` | [`apply_outcome`](../contracts/group_buy/src/lib.rs) | `disputes` contract | Caller is authorized disputes contract address, unfreezes dispute amount, applies refund credit or releases to supplier. |

## State transitions (`disputes`)

The table below covers every state-changing function in [`contracts/disputes/src/lib.rs`](../contracts/disputes/src/lib.rs).

| From State | To State | Function | Caller (Who) | Preconditions |
|---|---|---|---|---|
| *Uninitialized* | Initialized | [`__constructor`](../contracts/disputes/src/lib.rs) | Deployer | Valid `config` address, contract not yet initialized. |
| *None* | `Open` | [`open`](../contracts/disputes/src/lib.rs) | Member / Supplier / Organizer | Pool is `Delivered`, within dispute window, valid units/reason, caller posts required dispute deposit in USDC. Calls `group_buy.freeze`. |
| `Open` | `Open` | [`add_evidence`](../contracts/disputes/src/lib.rs) | Dispute party | Dispute is `Open`, caller is opener or respondent, evidence hash (32 bytes) provided. |
| `Open` | `Resolved` | [`resolve`](../contracts/disputes/src/lib.rs) | Arbiter | Dispute is `Open`, caller is active arbiter in `config`. Resolves outcome (`ReleaseToSupplier`, `RefundMember`, `Split`, `RefundPool`). Calls `group_buy.apply_outcome` and refunds or slashes deposit. |
| `Open` | `TimedOut` | [`timeout`](../contracts/disputes/src/lib.rs) | Anyone | Dispute is `Open`, current time > `created_at + sla_timeout_secs`. Settles in favor of non-defaulting party. |

## Read-only views (`group_buy` and `disputes`)

Read-only inspection methods do not mutate contract state:

- **`group_buy` views**:
  - [`pool`](../contracts/group_buy/src/lib.rs): returns `Pool` record by pool id.
  - [`commitment`](../contracts/group_buy/src/lib.rs): returns `Commitment` for a member in a pool.
  - [`members`](../contracts/group_buy/src/lib.rs): returns paged member addresses for a pool.
  - [`current_tier`](../contracts/group_buy/src/lib.rs): returns `(tier_index, unit_price)` based on current total units.
  - [`quote_commit`](../contracts/group_buy/src/lib.rs): calculates required escrow deposit for adding `units`.
  - [`payout_preview`](../contracts/group_buy/src/lib.rs): returns breakdown of supplier payout and fees before settlement.
  - [`allocated_units_of`](../contracts/group_buy/src/lib.rs): returns received unit allocation for a member post-delivery.
  - [`confirm_deadline`](../contracts/group_buy/src/lib.rs): returns delivery confirmation deadline timestamp.
  - [`dispute_deadline`](../contracts/group_buy/src/lib.rs): returns dispute window end timestamp (perishable-aware).
  - [`refundable_of`](../contracts/group_buy/src/lib.rs): returns total outstanding claimable refund for a member.
  - [`refunds_pending`](../contracts/group_buy/src/lib.rs): returns `(remaining_members, remaining_amount)` for keeper planning.
  - [`escrow_invariant`](../contracts/group_buy/src/lib.rs): returns `(expected, actual)` escrow balances for integrity monitoring.
- **`disputes` views**:
  - [`dispute`](../contracts/disputes/src/lib.rs): returns `Dispute` record by dispute id.
  - [`open_disputes_of`](../contracts/disputes/src/lib.rs): returns list of open dispute ids for a pool.

Known overlap: with default parameters the organizer-confirmation deadline and the delivery deadline coincide (`lead_time + 48h`). From that moment a supplier may force arbitration (`disputes.open` → treated as delivered in full) and anyone may call `fail_delivery`; whichever lands first wins. Operators should set `organizer_confirm_window_secs < delivery_grace_secs` if they want the supplier route to open first.
