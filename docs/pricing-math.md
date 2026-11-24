# Pricing math

All amounts are `i128` USDC stroops (7 decimals). Every operation is checked; overflow panics.

**Tiers.** `(min_units, unit_price)`, 1–5 entries, ascending `min_units`, strictly descending price, tier 1 `min_units == MOQ`.

**Commit.** A member committing `n` units when the pool holds `t` units pays `n × price(tier_at(t))` (tier 1 price while `t < MOQ`): the highest price they could end up paying. The tier is monotonic non-decreasing because a withdrawal that would lower the tier is rejected (ADR 0001), so `escrow ≥ total_units × final_price` always holds.

**Final price.** Fixed at `Filled`: price of the tier reached by `total_units`.

**Refund (no shortfall).** `paid − units × final_price`.

**Shortfall.** If `received < total`, members are walked in commit order with running cumulative units `cum`:
`alloc_i = floor((cum+u_i)·R/T) − floor(cum·R/T)` (R = received, T = total). Allocations sum to exactly `R`, deterministic, no leftover units. Refund = `paid − alloc × final_price`; the supplier is paid for `R × final_price`.

**Settlement.** `gross = R × final_price`; `payable = gross − frozen − diverted`; organizer fee and platform fee are `floor(payable × bp / 10000)`; supplier gets `payable − fees − advance_paid`. Sum of member refunds `= total_paid − gross` exactly, so no dust arises from tiers or shortfall.

**Dispute amounts.** `claimed_amount = claimed_units × final_price`. Deposit `= min(claimed, max(claimed × deposit_bp, deposit_min))`: the 1% floor of `deposit_min` applies below the crossover, the percentage applies above it, and the claim itself is a hard ceiling so a tiny claim never requires a larger deposit.
- `ReleaseToSupplier`: all to supplier (fees applied when paid).
- `RefundMember(u)`: `u × final_price` to the member (or pool-wide if the opener is not a member), rest to supplier.
- `Split(bp)`: supplier receives `floor(claim × (10000−bp)/10000)`, the group side receives the remainder; `bp > 10000` is rejected. `bp = 0` moves the whole claim to the supplier (as `ReleaseToSupplier`), `bp = 10000` moves the whole claim to the group side.
- `RefundPool`: whole claim shared pro-rata by units (floor; remainder stays as unswept dust).

**Dust.** Only the floor remainder of `RefundPool` shares can remain; `sweep_dust` sends escrow beyond `owed_total − refunds_paid` to the treasury once nothing is frozen.

---

## Worked examples

### Worked Example 1: Full delivery with tier progression (happy path)

- **Parameters**:
  - Tiers: MOQ 100 @ 1.00 USDC, 200 @ 0.90 USDC, 400 @ 0.80 USDC.
  - Fees: Platform fee = 1.5% (150 bp), Organizer fee = 1.0% (100 bp).
- **Commits**:
  - A commits 60 units (pool at 0 -> 60, < MOQ) @ 1.00 USDC: pays 60.00 USDC.
  - B commits 60 units (pool at 60 -> 120, tier 1 reached) @ 1.00 USDC: pays 60.00 USDC.
  - C commits 100 units (pool at 120 -> 220, tier 2 reached) @ 1.00 USDC: pays 100.00 USDC.
  - D commits 200 units (pool at 220 -> 420, tier 3 reached) @ 0.90 USDC: pays 180.00 USDC.
  - Total committed: 420 units. Total paid into escrow: 400.00 USDC.
- **Settlement**:
  - Final price: 0.80 USDC (tier 3).
  - Gross: `420 × 0.80 = 336.00 USDC`.
  - Platform fee: `floor(336.00 × 150 / 10000) = 5.04 USDC`.
  - Organizer fee: `floor(336.00 × 100 / 10000) = 3.36 USDC`.
  - Supplier net: `336.00 − 5.04 − 3.36 = 327.60 USDC`.
- **Member Refunds**:
  - A: `60.00 − (60 × 0.80) = 12.00 USDC`.
  - B: `60.00 − (60 × 0.80) = 12.00 USDC`.
  - C: `100.00 − (100 × 0.80) = 20.00 USDC`.
  - D: `180.00 − (200 × 0.80) = 20.00 USDC`.
  - Total refunds: `12 + 12 + 20 + 20 = 64.00 USDC`.
  - Invariant: `total_paid (400.00) = gross (336.00) + refunds (64.00)`. Exactly 0 dust. (See `happy_path.rs`).

---

### Worked Example 2: Shortfall with tier change and open dispute (RefundMember outcome)

> This worked example is mirrored identically by the integration test `shortfall_with_tier_change_and_dispute_worked_example` in [`crates/scenarios/tests/suite/disputes_and_bond.rs`](../crates/scenarios/tests/suite/disputes_and_bond.rs).

- **Parameters**:
  - Tiers: MOQ 100 @ 1.00 USDC (10,000,000 stroops), 200 @ 0.90 USDC (9,000,000 stroops), 400 @ 0.80 USDC (8,000,000 stroops).
  - Fees: Platform fee = 1.5% (150 bp), Organizer fee = 1.0% (100 bp).
- **Commits & Tier Progression**:
  - Member A commits 60 units (pool total 60, below MOQ): pays `60 × 1.00 = 60.00 USDC` (600,000,000 stroops).
  - Member B commits 60 units (pool total 120, tier 1 active): pays `60 × 1.00 = 60.00 USDC` (600,000,000 stroops).
  - Member C commits 100 units (pool total 220, tier 2 active): pays `100 × 1.00 = 100.00 USDC` (1,000,000,000 stroops).
  - Member D commits 200 units (pool total 420, tier 3 active): pays `200 × 0.90 = 180.00 USDC` (1,800,000,000 stroops).
  - Total committed: $T = 420$ units. Total paid into escrow: 400.00 USDC (4,000,000,000 stroops).
  - Pool closes and locks final price at Tier 3: **0.80 USDC** (8,000,000 stroops).
- **Shortfall Delivery & Allocation**:
  - Supplier delivers only $R = 350$ units (shortfall = 70 units).
  - Shortfall allocation walks members in commit order using integer arithmetic:
    `alloc_i = floor((cum + u_i) × R / T) - floor(cum × R / T)`
    - **Member A** ($u_A = 60$, running $cum = 0 \to 60$):
      $\lfloor 60 \times 350 / 420 \rfloor - 0 = \lfloor 21000 / 420 \rfloor = 50$ units.
    - **Member B** ($u_B = 60$, running $cum = 60 \to 120$):
      $\lfloor 120 \times 350 / 420 \rfloor - 50 = \lfloor 42000 / 420 \rfloor - 50 = 100 - 50 = 50$ units.
    - **Member C** ($u_C = 100$, running $cum = 120 \to 220$):
      $\lfloor 220 \times 350 / 420 \rfloor - 100 = \lfloor 77000 / 420 \rfloor - 100 = \lfloor 183.333\dots \rfloor - 100 = 183 - 100 = 83$ units.
      *(Rounding choice: integer floor truncates 0.333).*
    - **Member D** ($u_D = 200$, running $cum = 220 \to 420$):
      $\lfloor 420 \times 350 / 420 \rfloor - 183 = 350 - 183 = 167$ units.
    - Sum of allocated units: $50 + 50 + 83 + 167 = 350 = R$ exactly.
- **Open Dispute (Prior to Settlement)**:
  - Member A opens a dispute on 10 of their 50 received units.
  - Claimed & Frozen amount: $10 \times 0.80 = 8.00\text{ USDC}$ ($80,000,000$ stroops).
- **Settlement (with Open Dispute)**:
  - Gross delivery value: $R \times \text{final\_price} = 350 \times 0.80 = 280.00\text{ USDC}$ ($2,800,000,000$ stroops).
  - Net payable at settlement: $\text{Gross} - \text{Frozen} = 280.00 - 8.00 = 272.00\text{ USDC}$ ($2,720,000,000$ stroops).
  - Platform fee (1.5%): $\lfloor 272.00 \times 150 / 10000 \rfloor = 4.08\text{ USDC}$ ($40,800,000$ stroops).
  - Organizer fee (1.0%): $\lfloor 272.00 \times 100 / 10000 \rfloor = 2.72\text{ USDC}$ ($27,200,000$ stroops).
  - Supplier payout at settlement: $272.00 - 4.08 - 2.72 = 265.20\text{ USDC}$ ($2,652,000,000$ stroops).
- **Dispute Resolution**:
  - Arbiter resolves dispute with outcome `RefundMember(10)`.
  - Member A receives a dispute refund credit for $10 \times 0.80 = 8.00\text{ USDC}$.
  - Frozen amount drops from 8.00 USDC to 0.
- **Member Refunds**:
  - Base refund per member is $\text{paid} - (\text{allocated\_units} \times \text{final\_price})$.
  - Member A: $60.00 - (50 \times 0.80) + 8.00\text{ (dispute)} = 60.00 - 40.00 + 8.00 = 28.00\text{ USDC}$.
  - Member B: $60.00 - (50 \times 0.80) = 60.00 - 40.00 = 20.00\text{ USDC}$.
  - Member C: $100.00 - (83 \times 0.80) = 100.00 - 66.40 = 33.60\text{ USDC}$.
  - Member D: $180.00 - (167 \times 0.80) = 180.00 - 133.60 = 46.40\text{ USDC}$.
  - Total refunds: $28.00 + 20.00 + 33.60 + 46.40 = 128.00\text{ USDC}$.
- **Escrow Accounting Audit**:
  - Inflow: Total member deposits = 400.00 USDC.
  - Outflow: Supplier (265.20) + Platform fee (4.08) + Organizer fee (2.72) + Refunds (128.00) = 400.00 USDC.
  - Remaining escrow = 0 stroops. Exactly 0 dust.

---

### Worked Example 3: Shortfall with tier change and open dispute (Split outcome)

- **Parameters**:
  - Tiers: MOQ 100 @ 2.00 USDC (20,000,000 stroops), 250 @ 1.80 USDC (18,000,000 stroops).
  - Fees: Platform fee = 1.5% (150 bp), Organizer fee = 2.0% (200 bp).
- **Commits & Tier Progression**:
  - Member A commits 80 units (pool total 80, < MOQ): pays `80 × 2.00 = 160.00 USDC`.
  - Member B commits 70 units (pool total 150, Tier 1): pays `70 × 2.00 = 140.00 USDC`.
  - Member C commits 150 units (pool total 300, Tier 2 @ 1.80): pays `150 × 2.00 = 300.00 USDC`.
  - Total committed: $T = 300$ units. Total paid into escrow: 600.00 USDC.
  - Final price locked at Tier 2: **1.80 USDC** (18,000,000 stroops).
- **Shortfall Delivery & Allocation**:
  - Supplier delivers only $R = 200$ units (shortfall = 100 units).
  - Allocation arithmetic:
    - **Member A** ($u_A = 80$): $\lfloor 80 \times 200 / 300 \rfloor = \lfloor 53.333\dots \rfloor = 53$ units. *(Rounding: floor truncates 0.333)*.
    - **Member B** ($u_B = 70$): $\lfloor 150 \times 200 / 300 \rfloor - 53 = 100 - 53 = 47$ units.
    - **Member C** ($u_C = 150$): $200 - 100 = 100$ units.
    - Allocations: $53 + 47 + 100 = 200 = R$ exactly.
- **Open Dispute (Prior to Settlement)**:
  - Member B opens a dispute on 20 of their 47 received units.
  - Claimed amount: $20 \times 1.80 = 36.00\text{ USDC}$ ($360,000,000$ stroops).
  - Frozen amount: 36.00 USDC.
- **Settlement (with Open Dispute)**:
  - Gross delivery value: $200 \times 1.80 = 360.00\text{ USDC}$.
  - Net payable at settlement: $\text{Gross} - \text{Frozen} = 360.00 - 36.00 = 324.00\text{ USDC}$.
  - Platform fee (1.5%): $\lfloor 324.00 \times 150 / 10000 \rfloor = 4.86\text{ USDC}$ ($48,600,000$ stroops).
  - Organizer fee (2.0%): $\lfloor 324.00 \times 200 / 10000 \rfloor = 6.48\text{ USDC}$ ($64,800,000$ stroops).
  - Supplier payout at settlement: $324.00 - 4.86 - 6.48 = 312.66\text{ USDC}$ ($3,126,600,000$ stroops).
- **Dispute Resolution (Split 50/50)**:
  - Arbiter resolves dispute with outcome `Split(5000)` (50.00% split).
  - Member refund portion: $\lfloor 36.00 \times 5000 / 10000 \rfloor = 18.00\text{ USDC}$.
  - Supplier portion: $36.00 - 18.00 = 18.00\text{ USDC}$.
  - Fees deducted on supplier's dispute release:
    - Platform fee: $\lfloor 18.00 \times 150 / 10000 \rfloor = 0.27\text{ USDC}$.
    - Organizer fee: $\lfloor 18.00 \times 200 / 10000 \rfloor = 0.36\text{ USDC}$.
    - Supplier net dispute payout: $18.00 - 0.27 - 0.36 = 17.37\text{ USDC}$.
- **Member Refunds**:
  - Member A: $160.00 - (53 \times 1.80) = 160.00 - 95.40 = 64.60\text{ USDC}$.
  - Member B: $140.00 - (47 \times 1.80) + 18.00\text{ (dispute)} = 140.00 - 84.60 + 18.00 = 73.40\text{ USDC}$.
  - Member C: $300.00 - (100 \times 1.80) = 300.00 - 180.00 = 120.00\text{ USDC}$.
  - Total refunds: $64.60 + 73.40 + 120.00 = 258.00\text{ USDC}$.
- **Escrow Accounting Audit**:
  - Inflow: Total member deposits = 600.00 USDC.
  - Outflow:
    - Supplier: $312.66\text{ (initial)} + 17.37\text{ (dispute)} = 330.03\text{ USDC}$.
    - Platform fee: $4.86 + 0.27 = 5.13\text{ USDC}$.
    - Organizer fee: $6.48 + 0.36 = 6.84\text{ USDC}$.
    - Member refunds: 258.00 USDC.
    - Sum: $330.03 + 5.13 + 6.84 + 258.00 = 600.00\text{ USDC}$. Exactly 0 dust.

---

## Escrow invariant (issue #143)

At every reachable state the following must hold:

```
escrow_balance  >=  (owed_total − refunds_paid)  +  frozen_amount
```

| Term | Meaning |
|---|---|
| `escrow_balance` | Tokens the contract currently holds for the pool |
| `owed_total − refunds_paid` | Outstanding refund obligations not yet paid out |
| `frozen_amount` | Tokens locked in open disputes |

The right-hand side is the **minimum** the contract must hold. Any breach means tokens
were released in excess of obligations and must be treated as a critical (P0) incident.

**After full settlement and `sweep_dust`** both sides equal zero: all obligations are paid,
all dust is swept, and the contract holds nothing for that pool.

**Read-only view:** `group_buy.escrow_invariant(pool_id) -> (expected, actual)` returns
both sides of the inequality without mutating state. The reconcile backend job and any
external monitor should call this view after every block that touches the pool.

**Property test:** `crates/scenarios/tests/suite/properties.rs::escrow_invariant_holds_in_all_states`
asserts `actual >= expected` after every operation in a random commit/deliver/settle sequence.
