# 0003 Per-member share cap is relative to `max_units`

**Context.** The brief forbids any member holding more than 50% of a pool unless they are the only member. Checking the live share on every commit is O(members) or needs stale tracking, and makes early small joiners impossible once a whale commits.

**Decision.** A member (other than the sole member) may hold at most `max_member_share_bp` of the pool's `max_units`. O(1), deterministic.

**Consequences.** A single large member plus one tiny sybil can still reach a tier; trader tier caps and KYC bound that.
