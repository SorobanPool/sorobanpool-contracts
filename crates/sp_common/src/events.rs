//! Event helper. Topics are `(contract_symbol, event_symbol, key)`; payloads are
//! documented in docs/events.md and the indexer depends on them.
#![allow(deprecated)]
use soroban_sdk::{Env, IntoVal, Symbol, Val};

pub fn emit<K: IntoVal<Env, Val>, D: IntoVal<Env, Val>>(
    env: &Env,
    contract: Symbol,
    event: Symbol,
    key: K,
    data: D,
) {
    env.events().publish((contract, event, key), data);
}
