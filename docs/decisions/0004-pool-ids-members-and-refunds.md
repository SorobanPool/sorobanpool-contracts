# 0004 Pool IDs, member limit, refund defaults

- **Pool ID scheme:** sequential `u64` from an instance counter (simple, indexer friendly, not secret).
- **Member limit:** 200 per pool (`max_members_per_pool`); the list includes withdrawn members, which keeps commit order stable for shortfall allocation.
- **Refunds:** pull by default (`claim_refund`), with permissionless bounded `push_refunds(max)`; the keeper pushes in batches of 25.
