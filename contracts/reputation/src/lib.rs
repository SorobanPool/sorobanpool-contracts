#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, symbol_short, Address,
    Env, Symbol,
};
use sp_common::clients::ConfigClient;
use sp_common::events::emit;
use sp_common::keys;
use sp_common::ttl::bump_persistent;
use sp_common::types::{RepStats, Role};

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    NotAuthorizedCaller = 600,
    UnknownEvent = 601,
    NotInitialised = 602,
}

#[contracttype]
#[derive(Clone)]
enum Key {
    Config,
    Stats(Address, Role),
}

const C: Symbol = symbol_short!("rep");
const MONTH_SECS: u64 = 30 * 24 * 3600;

fn rate_bp(num: u32, den: u32) -> u32 {
    if den == 0 {
        return 0;
    }
    ((num as u64) * 10_000 / den as u64) as u32
}

/// Tier rules from brief section 3.8. Thresholds are constants in v1.
fn compute_tier(role: Role, s: &RepStats, now: u64) -> u32 {
    let lost_bp = rate_bp(s.disputes_lost, s.pools_settled);
    match role {
        Role::Supplier => {
            let on_time_bp = rate_bp(s.on_time, s.on_time + s.late);
            let months = if s.first_active_at == 0 {
                0
            } else {
                now.saturating_sub(s.first_active_at) / MONTH_SECS
            };
            if s.pools_settled >= 50 && lost_bp < 200 && on_time_bp >= 9500 && months >= 6 {
                3
            } else if s.pools_settled >= 15 && lost_bp < 500 && on_time_bp >= 9000 {
                2
            } else if s.pools_settled >= 3 && s.disputes_lost == 0 {
                1
            } else {
                0
            }
        }
        Role::Organizer => {
            if s.pools_settled >= 15 && lost_bp < 500 {
                2
            } else if s.pools_settled >= 3 && s.disputes_lost == 0 {
                1
            } else {
                0
            }
        }
        // Trader tier 2 needs reputation; tier 1 (KYC) is decided by group_buy from registry level.
        Role::Trader => {
            if s.pools_settled >= 5 && s.disputes_lost == 0 {
                2
            } else {
                0
            }
        }
    }
}

#[contract]
pub struct Reputation;

#[contractimpl]
impl Reputation {
    pub fn __constructor(env: Env, config: Address) {
        env.storage().instance().set(&Key::Config, &config);
    }

    /// Events: settled, failed, on_time, late, d_open, d_won, d_lost, short, pickup.
    /// Caller must be the group_buy or disputes contract.
    pub fn record(env: Env, caller: Address, subject: Address, role: Role, event: Symbol) {
        caller.require_auth();
        let cfg_addr: Address = env
            .storage()
            .instance()
            .get(&Key::Config)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialised));
        let cfg = ConfigClient::new(&env, &cfg_addr);
        if caller != cfg.get_address(&keys::GROUP_BUY) && caller != cfg.get_address(&keys::DISPUTES)
        {
            panic_with_error!(&env, Error::NotAuthorizedCaller);
        }
        let k = Key::Stats(subject.clone(), role);
        let mut s: RepStats = env.storage().persistent().get(&k).unwrap_or_default();
        if s.first_active_at == 0 {
            s.first_active_at = env.ledger().timestamp();
        }
        if event == symbol_short!("settled") {
            s.pools_settled += 1;
        } else if event == symbol_short!("failed") {
            s.pools_failed += 1;
        } else if event == symbol_short!("on_time") {
            s.on_time += 1;
        } else if event == symbol_short!("late") {
            s.late += 1;
        } else if event == symbol_short!("d_open") {
            s.disputes_opened += 1;
        } else if event == symbol_short!("d_won") {
            s.disputes_won += 1;
        } else if event == symbol_short!("d_lost") {
            s.disputes_lost += 1;
        } else if event == symbol_short!("short") {
            s.short_deliveries += 1;
        } else if event == symbol_short!("pickup") {
            s.pickups_confirmed += 1;
        } else {
            panic_with_error!(&env, Error::UnknownEvent);
        }
        env.storage().persistent().set(&k, &s);
        bump_persistent(&env, &k);
        let tier = compute_tier(role, &s, env.ledger().timestamp());
        emit(
            &env,
            C,
            symbol_short!("rep_upd"),
            subject,
            (role, event, tier),
        );
    }

    pub fn stats(env: Env, subject: Address, role: Role) -> RepStats {
        env.storage()
            .persistent()
            .get(&Key::Stats(subject, role))
            .unwrap_or_default()
    }

    pub fn tier(env: Env, subject: Address, role: Role) -> u32 {
        let s = Self::stats(env.clone(), subject, role);
        compute_tier(role, &s, env.ledger().timestamp())
    }
}

#[cfg(test)]
mod test;
