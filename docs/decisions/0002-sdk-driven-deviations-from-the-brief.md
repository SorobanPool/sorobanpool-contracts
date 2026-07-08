# 0002 SDK-driven deviations from the brief

- `Params.withdraw_lock_before_deadline_secs` is `withdraw_lock_secs`: soroban-sdk 28 limits struct field names to 30 characters.
- `Dispute.outcome` is `Vec<Outcome>` (empty until resolved) because `Option<Outcome>` is not supported for an enum with a tuple variant.
- `group_buy.accept` takes `advance_bp` (0 = no advance).
- `config.pause` takes the `caller` (admin or guardian); `config` also exposes `admin`, `usdc`, `treasury`, `is_attestor`.
- `supplier_bond.slash` sends slashed funds to the caller (`group_buy`).
- `Pool` / `Commitment` gain bookkeeping fields (`final_price`, `total_paid`, `diverted`, `extra_refund_pool`, `owed_total`, `refunds_paid`, allocation cursors; `Commitment.refund_claimed` is an amount, plus `extra_credit`, `delivery_confirmed`).
- Contract crates talk through client traits in `sp_common::clients`, so none depends on another.
- Reputation tier thresholds are constants in v1 (the brief makes them admin-configurable within caps).
- WASM is built with `stellar contract build`; a plain `cargo build --target wasm32v1-none` is rejected by this SDK version.
