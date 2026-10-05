# Pool state machine

```
Open ──close/close_early/max reached──▶ Filled ──accept──▶ Accepted ──dispatch──▶ Dispatched
 │                                        │                  │                        │
 │ close (deadline, total < MOQ)          │ reject /         │ fail_delivery /        │ confirm_delivery /
 ▼                                        │ fail_accept      │ cancel (admin)         │ member_confirm_delivery
Expired                                   ▼                  ▼                        ▼
                                        Failed             Failed/Cancelled        Delivered ──settle──▶ Settled
Open/Filled/Accepted ──cancel_pool──▶ Cancelled                                      (frozen disputes held back)
```
Refundable end states: `Expired`, `Failed`, `Cancelled` (full refund) and `Settled` (price-tier / shortfall refunds and dispute credits). Refunds are pull (`claim_refund`) with a permissionless bounded push (`push_refunds`).

| Transition | Who | Preconditions |
|---|---|---|
| `create_pool` | organizer | Verified organizer and supplier, allowed category, valid tiers, deadline ≤ max fill window, pool value within organizer and supplier tier caps, fee ≤ max |
| `commit` / `increase` | member | Open, before deadline, not the supplier, trader registered, cluster match, per-member / share / pool max / trader tier cap |
| `withdraw_commitment` | member | Open, ≥ lock (2h) before deadline, **must not lower the price tier** (ADR 0001) |
| `close_early` | organizer | Open, total ≥ MOQ |
| `close` | anyone | Open, now ≥ deadline → Filled if total ≥ MOQ else Expired |
| `accept` | supplier | Filled, within accept window; optional advance (off by default) |
| `reject` | supplier | Filled → Failed |
| `fail_accept` | anyone | Filled, accept window passed → Failed |
| `dispatch` | supplier | Accepted |
| `confirm_delivery` | organizer | Accepted or Dispatched, `received ≤ total` |
| `member_confirm_delivery` | members | After organizer-confirm deadline; ≥ 50% of units confirm → Delivered in full |
| `fail_delivery` | anyone | Accepted/Dispatched after `accepted_at + lead + grace` → Failed (bond slashed if advance paid) |
| `allocate_shortfall` | anyone | Delivered with `received < total`; paged; required before `settle` |
| `confirm_pickup` | member | Delivered or Settled |
| `settle` | anyone | Delivered, allocation done, confirmation window over **or** pickup weight ≥ early-release weight |
| `claim_refund` / `push_refunds` | member / anyone | Any refundable end state |
| `sweep_dust` | anyone | Refundable end state, nothing frozen; sends only escrow beyond what is still owed |
| `freeze` / `apply_outcome` | `disputes` contract only | Delivered (within the confirmation window) / Delivered or Settled |

Known overlap: with default parameters the organizer-confirmation deadline and the delivery deadline coincide (`lead_time + 48h`). From that moment a supplier may force arbitration (`disputes.open` → treated as delivered in full) and anyone may call `fail_delivery`; whichever lands first wins. Operators should set `organizer_confirm_window_secs < delivery_grace_secs` if they want the supplier route to open first.
