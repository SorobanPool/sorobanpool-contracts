# Pricing math

All amounts are `i128` USDC stroops (7 decimals). Every operation is checked; overflow panics.

**Tiers.** `(min_units, unit_price)`, 1–5 entries, ascending `min_units`, strictly descending price, tier 1 `min_units == MOQ`.

**Commit.** A member committing `n` units when the pool holds `t` units pays `n × price(tier_at(t))` (tier 1 price while `t < MOQ`): the highest price they could end up paying. The tier is monotonic non-decreasing because a withdrawal that would lower the tier is rejected (ADR 0001), so `escrow ≥ total_units × final_price` always holds.

**Final price.** Fixed at `Filled`: price of the tier reached by `total_units`.

**Refund (no shortfall).** `paid − units × final_price`.

**Shortfall.** If `received < total`, members are walked in commit order with running cumulative units `cum`:
`alloc_i = floor((cum+u_i)·R/T) − floor(cum·R/T)` (R = received, T = total). Allocations sum to exactly `R`, deterministic, no leftover units. Refund = `paid − alloc × final_price`; the supplier is paid for `R × final_price`.

**Settlement.** `gross = R × final_price`; `payable = gross − frozen − diverted`; organizer fee and platform fee are `floor(payable × bp / 10000)`; supplier gets `payable − fees − advance_paid`. Sum of member refunds `= total_paid − gross` exactly, so no dust arises from tiers or shortfall.

**Worked example** (MOQ 100 @ 1.00, 200 @ 0.90, 400 @ 0.80; fees 1.5% + 1%):
A 60 → pays 60, B 60 → pays 60, C 100 (pool at 120, tier 1) → pays 100, D 200 (pool at 220, tier 2) → pays 180. Total 420 units, final price 0.80, total paid 400.
Gross 336; platform 5.04; organizer 3.36; supplier 327.60 + (advance 0). Refunds: A 12, B 12, C 20, D 20 = 64 = 400 − 336. ✔ (see `happy_path.rs`).

**Dispute amounts.** `claimed_amount = claimed_units × final_price`. Deposit `= max(claimed × deposit_bp, deposit_min)`.
- `ReleaseToSupplier`: all to supplier (fees applied when paid).
- `RefundMember(u)`: `u × final_price` to the member (or pool-wide if the opener is not a member), rest to supplier.
- `Split(bp)`: `floor(claim × bp/10000)` to the group side, rest to supplier.
- `RefundPool`: whole claim shared pro-rata by units (floor; remainder stays as unswept dust).

**Dust.** Only the floor remainder of `RefundPool` shares can remain; `sweep_dust` sends escrow beyond `owed_total − refunds_paid` to the treasury once nothing is frozen.
