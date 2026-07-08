#![no_std]
//! Shared types, math, cross-contract clients and TTL helpers for SorobanPool.

pub mod clients;
pub mod events;
pub mod math;
pub mod ttl;
pub mod types;

/// Basis points denominator.
pub const BPS_DENOMINATOR: u32 = 10_000;

/// Well-known keys for `config.get_address`.
pub mod keys {
    use soroban_sdk::{symbol_short, Symbol};
    pub const REGISTRY: Symbol = symbol_short!("registry");
    pub const GROUP_BUY: Symbol = symbol_short!("group_buy");
    pub const DISPUTES: Symbol = symbol_short!("disputes");
    pub const BOND: Symbol = symbol_short!("bond");
    pub const REPUTATION: Symbol = symbol_short!("rep");
}
