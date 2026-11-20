//! Event helper. Topics are `(contract_symbol, event_symbol, key)`; payloads are
//! documented in docs/events.md and the indexer depends on them.
#![allow(deprecated)]
use soroban_sdk::{symbol_short, Env, IntoVal, Symbol, Val};

// Contract symbols
pub const CONTRACT_CONFIG: Symbol = symbol_short!("config");
pub const CONTRACT_REGISTRY: Symbol = symbol_short!("registry");
pub const CONTRACT_REP: Symbol = symbol_short!("rep");
pub const CONTRACT_BOND: Symbol = symbol_short!("bond");
pub const CONTRACT_GROUP_BUY: Symbol = symbol_short!("group_buy");
pub const CONTRACT_DISPUTES: Symbol = symbol_short!("disputes");
pub const CONTRACT_WALLET: Symbol = symbol_short!("wallet");

// Config events
pub const PARAMS: Symbol = symbol_short!("params");
pub const PAUSED: Symbol = symbol_short!("paused");
pub const UNPAUSED: Symbol = symbol_short!("unpaused");
pub const UPGRADED: Symbol = symbol_short!("upgraded");
pub const ADMIN_PROPOSED_STR: &str = "admin_proposed";
pub const ADMIN_CHANGED_STR: &str = "admin_changed";

pub fn admin_proposed(env: &Env) -> Symbol {
    Symbol::new(env, ADMIN_PROPOSED_STR)
}

pub fn admin_changed(env: &Env) -> Symbol {
    Symbol::new(env, ADMIN_CHANGED_STR)
}

// Registry events
pub const USER_REG: Symbol = symbol_short!("user_reg");
pub const USER_ATT: Symbol = symbol_short!("user_att");
pub const USER_REV: Symbol = symbol_short!("user_rev");
pub const USER_SUS: Symbol = symbol_short!("user_sus");
pub const USER_UNS: Symbol = symbol_short!("user_uns");

// Reputation events
pub const REP_UPD: Symbol = symbol_short!("rep_upd");

// Supplier bond events
pub const DEPOSIT: Symbol = symbol_short!("deposit");
pub const WITHDRAW: Symbol = symbol_short!("withdraw");
pub const SLASHED: Symbol = symbol_short!("slashed");

// Group buy events
pub const POOL_NEW: Symbol = symbol_short!("pool_new");
pub const COMMITTED: Symbol = symbol_short!("committed");
pub const COMMIT_UP: Symbol = symbol_short!("commit_up");
pub const WITHDRAWN: Symbol = symbol_short!("withdrawn");
pub const TIER_UP: Symbol = symbol_short!("tier_up");
pub const FILLED: Symbol = symbol_short!("filled");
pub const EXPIRED: Symbol = symbol_short!("expired");
pub const CANCELLED: Symbol = symbol_short!("cancelled");
pub const ACCEPTED: Symbol = symbol_short!("accepted");
pub const REJECTED: Symbol = symbol_short!("rejected");
pub const ADVANCE: Symbol = symbol_short!("advance");
pub const DISPATCH: Symbol = symbol_short!("dispatch");
pub const FAILED: Symbol = symbol_short!("failed");
pub const DELIVERED: Symbol = symbol_short!("delivered");
pub const SHORTFALL: Symbol = symbol_short!("shortfall");
pub const ALLOC_OK: Symbol = symbol_short!("alloc_ok");
pub const PICKUP: Symbol = symbol_short!("pickup");
pub const SETTLED: Symbol = symbol_short!("settled");
pub const REFUND: Symbol = symbol_short!("refund");

// Dispute events
pub const D_OPEN: Symbol = symbol_short!("d_open");
pub const D_EVID: Symbol = symbol_short!("d_evid");
pub const D_RESOLVE: Symbol = symbol_short!("d_resolve");
pub const D_TIMEOUT: Symbol = symbol_short!("d_timeout");

// Wallet events
pub const KEY_ADD: Symbol = symbol_short!("key_add");
pub const KEY_RM: Symbol = symbol_short!("key_rm");

/// Complete list of event names matching artifacts/events.json
pub const ALL_EVENTS: &[&str] = &[
    "accepted",
    "admin_changed",
    "admin_proposed",
    "advance",
    "alloc_ok",
    "cancelled",
    "commit_up",
    "committed",
    "d_evid",
    "d_open",
    "d_resolve",
    "d_timeout",
    "delivered",
    "deposit",
    "dispatch",
    "expired",
    "failed",
    "filled",
    "key_add",
    "key_rm",
    "params",
    "paused",
    "pickup",
    "pool_new",
    "refund",
    "rejected",
    "rep_upd",
    "settled",
    "shortfall",
    "slashed",
    "tier_up",
    "unpaused",
    "upgraded",
    "user_att",
    "user_reg",
    "user_rev",
    "user_sus",
    "user_uns",
    "withdraw",
    "withdrawn",
];

pub fn emit<K: IntoVal<Env, Val>, D: IntoVal<Env, Val>>(
    env: &Env,
    contract: Symbol,
    event: Symbol,
    key: K,
    data: D,
) {
    env.events().publish((contract, event, key), data);
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate std;
    use std::vec::Vec;

    #[test]
    fn test_all_events_match_artifacts() {
        let json_str = include_str!("../../../artifacts/events.json");
        // Extract event names from JSON
        let mut artifact_events = Vec::new();
        for line in json_str.lines() {
            if let Some(pos) = line.find("\"event\": \"") {
                let rest = &line[pos + 10..];
                if let Some(end) = rest.find('\"') {
                    artifact_events.push(&rest[..end]);
                }
            }
        }
        artifact_events.sort();
        artifact_events.dedup();

        let mut const_events = ALL_EVENTS.to_vec();
        const_events.sort();
        const_events.dedup();

        assert_eq!(
            const_events, artifact_events,
            "Event constants in sp_common::events do not match artifacts/events.json"
        );
    }
}
