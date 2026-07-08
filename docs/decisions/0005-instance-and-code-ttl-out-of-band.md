# 0005 Contract instance and code TTL are extended out-of-band

**Context.** The first implementation called `env.storage().instance().extend_ttl(..)` in `config.get_params`, `group_buy.create_pool` and `disputes.open`. In the Soroban SDK that extends the contract instance *and* its wasm code. Measured on testnet with the simulated resource fee: `config.get_params` 350,055,587 stroops (~35 XLM), `create_pool` 1,589,512,927 stroops (~159 XLM), versus 2,868,948 for a `registry.register` and ~14,000 for a read-only call. The cost was constant in pool size (1 to 5 tiers moved it by under 0.1%), so it was code rent, charged to whichever user transaction happened to find the TTL below threshold, and so to the sponsor.

**Decision.** Contract calls never extend instance or code TTL. Persistent data entries are still bumped on touch (small, cheap). The backend `ttl-extend` job extends each contract's instance and code with `ExtendFootprintTtl` when remaining life is below ~20 days, and monitors the remaining life.

**Alternatives.** Keep bumping on user paths with a higher threshold (still charges the unlucky caller large rent); smaller wasm (helps, does not remove the effect); letting users pay (the sponsor model forbids it).

**Consequences.** The sponsor balance must cover periodic code rent; the `sponsor-balance` alert should account for it. If `ttl-extend` fails for long enough the contracts archive and need a restore, so its failure is a paging alert. Pending: measure the same fee on mainnet-like network settings before pricing the product.
