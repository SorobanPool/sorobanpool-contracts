# Storage and TTL

- **Instance:** config pointers (`config` address), id counters, config parameters/admin/treasury.
- **Persistent:** pools, commitments, member pages (50 addresses per page), disputes and per-pool dispute lists, bonds, registry records, reputation stats, config flags (attestors, arbiters, categories, pause scopes).
- **Temporary:** none for money-related data.

Every read or write of a persistent entry extends its TTL (`sp_common::ttl`, threshold ~30 days, extend to ~60 days). Instance TTL is extended on id allocation and parameter reads. The backend `ttl-extend` job extends everything belonging to non-final pools, open disputes, bonds, active users, and the contract instances and code.

Final pools (Settled/Expired/Failed/Cancelled with all refunds paid) may be left to archive after 180 days. Unclaimed refunds must never be archived silently: the backend keeps extending any pool with `owed_total > refunds_paid`.

Per-pool member limit is 200 (`max_members_per_pool`). Settlement is O(1); refunds and shortfall allocation are paged.
