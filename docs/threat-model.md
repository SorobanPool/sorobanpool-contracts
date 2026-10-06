# Threat model (v1)

Trust boundaries: contracts are the source of truth for escrow; the backend cannot move funds; the frontend is untrusted; user exits never depend on the backend (`close`, `fail_accept`, `fail_delivery`, `dispute.timeout`, `settle`, refunds are permissionless and time-checked). Pausing never blocks refunds, expiry, failure paths, dispute timeouts or settlement.

| Threat | Mitigation | Residual risk |
|---|---|---|
| Organizer–supplier collusion (fake delivery confirmation) | Member dispute window, pickup confirmations, early-release weight, collective member confirmation, reputation, admin concentration dashboard | Colluders can still defraud members who do not dispute within 48h; keep pool caps low in the pilot |
| Tier sabotage (last-minute withdrawal) | 2h withdrawal lock; withdrawals that would lower the price tier are rejected (ADR 0001) | A member may be unable to withdraw while holding the pool at its tier |
| Underfunded escrow after tier changes | Ceiling-price commits + monotonic tiers; coverage invariant property-tested | – |
| Sybil members inflating tiers | T0 cap 50 USDC, KYC for larger amounts, per-member cap of 50% of `max_units` | A funded attacker can still buy tier breaks with their own money |
| Arbiter capture | Admin-managed arbiter set, on-chain conflict check (cannot rule on a pool they organized, supplied or joined), SLA timeout defaults to the disputer, reasoning hashes public | Arbiters can still rule wrongly within their set; use a multisig admin |
| Griefing disputes | Deposit (1% or 0.5 USDC min) forfeited on a full rejection; only the claimed amount is frozen | – |
| Supplier no-show with advance | Bond ≥ outstanding advances; `fail_delivery` slashes the bond into escrow; advances disabled in v1 | Bond slashing caps at the bond held |
| Attestor key compromise | Attestor can only call `registry.attest`; admin can revoke; backend daily caps and spike alerts | Fake verifications until revoked |
| FX movement between offer refresh and pool close | Prices fixed in USDC at pool creation; short fill windows; disclosed | Supplier bears or passes on naira FX risk |
| Supplier/organizer race at the delivery deadline | Documented in `state-machine.md` | First caller wins |
| Admin key / upgrade abuse | Admin should be a multisig; `upgrade` emits an event; hard caps live in contract code | Upgrade authority is powerful |
| Re-entrancy via token callbacks | State is updated before transfers; USDC is a Stellar Asset Contract | – |
| Storage archival of live data | TTL bumping on touch plus backend job | A pool left untouched for long periods needs the keeper |
| Dust theft | `sweep_dust` only moves escrow beyond `owed_total − refunds_paid`, blocked while anything is frozen | – |

Covered since: resource budget tests at a 200-member pool (`crates/scenarios/tests/suite/budget.rs`), ≥90% line coverage on `group_buy`/`disputes`/`supplier_bond` enforced in CI, randomised operation-sequence fuzzing (`crates/scenarios/tests/suite/fuzz.rs`). Not covered yet: external audit, formal verification, coverage-guided fuzzing (needs nightly Rust).
