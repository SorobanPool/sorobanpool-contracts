# 0001 Withdrawals must not lower the price tier

**Context.** A member pays the price of the tier the pool is in before their units, so their payment is an upper bound on the final price only if the tier never goes down. The brief allows withdrawals up to 2h before the deadline. A withdrawal that drops the pool to a lower tier would leave later members (who paid the cheaper price) unable to cover the higher final price.

**Decision.** `withdraw_commitment` is rejected with `WouldLowerTier` if `tier(total − units) != tier(total)`. Tiers are therefore monotonic non-decreasing for the life of a pool.

**Alternatives.** Re-charge remaining members on withdrawal (needs their auth and breaks pull payments); recompute prices at close and let late payers be short (puts escrowed funds at risk); disallow withdrawal entirely (hurts trader trust).

**Consequences.** A member whose units hold a tier cannot leave; the UI must explain this. Coverage invariant `escrow ≥ total_units × final_price` always holds.
