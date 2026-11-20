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
    InvalidCorrection = 603,
    CorrectionTooLarge = 604,
    CorrectionUnderflow = 605,
}

#[contracttype]
#[derive(Clone)]
enum Key {
    Config,
    Stats(Address, Role),
}

const C: Symbol = symbol_short!("rep");
const MONTH_SECS: u64 = 30 * 24 * 3600;
const MAX_CORRECTION: u32 = 1_000;

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
            s.pools_settled = s.pools_settled.saturating_add(1);
        } else if event == symbol_short!("failed") {
            s.pools_failed = s.pools_failed.saturating_add(1);
            s.last_failed_at = env.ledger().timestamp();
        } else if event == symbol_short!("on_time") {
            s.on_time = s.on_time.saturating_add(1);
        } else if event == symbol_short!("late") {
            s.late = s.late.saturating_add(1);
        } else if event == symbol_short!("d_open") {
            s.disputes_opened = s.disputes_opened.saturating_add(1);
        } else if event == symbol_short!("d_won") {
            s.disputes_won = s.disputes_won.saturating_add(1);
        } else if event == symbol_short!("d_lost") {
            s.disputes_lost = s.disputes_lost.saturating_add(1);
        } else if event == symbol_short!("short") {
            s.short_deliveries = s.short_deliveries.saturating_add(1);
        } else if event == symbol_short!("pickup") {
            s.pickups_confirmed = s.pickups_confirmed.saturating_add(1);
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

    /// Admin-only bounded decrement. Counters saturate at u32::MAX when recorded normally.
    pub fn correct(
        env: Env,
        admin: Address,
        subject: Address,
        role: Role,
        counter: Symbol,
        delta: u32,
        reason: Symbol,
    ) {
        admin.require_auth();
        let cfg_addr: Address = env
            .storage()
            .instance()
            .get(&Key::Config)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialised));
        let cfg = ConfigClient::new(&env, &cfg_addr);
        if admin != cfg.admin() {
            panic_with_error!(&env, Error::NotAuthorizedCaller);
        }
        if delta == 0 {
            panic_with_error!(&env, Error::InvalidCorrection);
        }
        if delta > MAX_CORRECTION {
            panic_with_error!(&env, Error::CorrectionTooLarge);
        }
        let k = Key::Stats(subject.clone(), role);
        let mut s: RepStats = env.storage().persistent().get(&k).unwrap_or_default();
        let counter_ref = match counter.clone() {
            c if c == symbol_short!("settled") => &mut s.pools_settled,
            c if c == symbol_short!("failed") => &mut s.pools_failed,
            c if c == symbol_short!("on_time") => &mut s.on_time,
            c if c == symbol_short!("late") => &mut s.late,
            c if c == symbol_short!("d_open") => &mut s.disputes_opened,
            c if c == symbol_short!("d_won") => &mut s.disputes_won,
            c if c == symbol_short!("d_lost") => &mut s.disputes_lost,
            c if c == symbol_short!("short") => &mut s.short_deliveries,
            c if c == symbol_short!("pickup") => &mut s.pickups_confirmed,
            _ => panic_with_error!(&env, Error::InvalidCorrection),
        };
        if *counter_ref < delta {
            panic_with_error!(&env, Error::CorrectionUnderflow);
        }
        *counter_ref -= delta;
        env.storage().persistent().set(&k, &s);
        bump_persistent(&env, &k);
        emit(&env, C, Symbol::new(&env, "rep_corrected"), subject, (role, counter, delta, reason));
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
