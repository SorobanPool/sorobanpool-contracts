# Storage and TTL

- **Instance:** config pointers (`config` address), id counters, config parameters/admin/treasury, attestor list (`attestors()`, bounded at `MAX_ATTESTORS = 20`), arbiter list (`arbiters()`, bounded at `MAX_ARBITERS = 20`).
- **Persistent:** pools, commitments, member pages (50 addresses per page), disputes and per-pool dispute lists, bonds, registry records, reputation stats, config flags (attestors, arbiters, categories, pause scopes).
- **Temporary:** none for money-related data.

### Config Admin Lists Storage Growth
Enumerable attestor and arbiter lists (`attestors()`, `arbiters()`) are stored in `config` instance storage as `Vec<Address>`. To ensure bounded instance storage footprint and predictable serialization cost, each list is strictly limited to 20 entries (`MAX_ATTESTORS = 20`, `MAX_ARBITERS = 20`). Exceeding either limit panics with named errors `TooManyAttestors` or `TooManyArbiters`. Disabling an attestor or arbiter automatically removes it from the respective enumerable list.

Every read or write of a persistent entry extends its TTL (`sp_common::ttl`, threshold ~30 days, extend to ~60 days).

**Instance and code TTL are never extended by contract calls.** In the Soroban SDK, extending instance storage also extends the contract's wasm code, charged as rent on the whole code size. An early version bumped the instance on `config.get_params` and `group_buy.create_pool`; on testnet a single `get_params` then cost ~35 XLM and `create_pool` ~159 XLM (measured with the simulated resource fee), against ~0.003 XLM for a registry write. The backend `ttl-extend` job (ExtendFootprintTtl on each contract's instance and code entries, when remaining life drops below ~20 days) now keeps contracts alive and pays that rent deliberately, once per period, from the sponsor account. It also extends everything belonging to non-final pools, open disputes, bonds and active users.

Final pools (Settled/Expired/Failed/Cancelled with all refunds paid) may be left to archive after 180 days. Unclaimed refunds must never be archived silently: the backend keeps extending any pool with `owed_total > refunds_paid`.

Per-pool member limit is 200 (`max_members_per_pool`). Settlement is O(1); refunds and shortfall allocation are paged.
