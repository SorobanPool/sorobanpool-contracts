#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, symbol_short, vec,
    Address, BytesN, Env, Symbol, Vec,
};
use sp_common::events::emit;
use sp_common::ttl::bump_instance;
use sp_common::types::{caps, Params};

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    NotAdmin = 100,
    NotAdminOrGuardian = 101,
    ParamsInvalid = 102,
    ParamsExceedHardCap = 103,
    AddressNotSet = 104,
    NoGuardian = 105,
}

#[contracttype]
enum Key {
    Admin,
    Usdc,
    Treasury,
    Params,
    Guardian,
    Addr(Symbol),
    Attestor(Address),
    Arbiter(Address),
    Category(Symbol),
    Paused(Symbol),
}

const C: Symbol = symbol_short!("config");

fn default_params(env: &Env) -> Params {
    let usdc = 10_000_000i128; // 1 USDC (7 decimals)
    Params {
        platform_fee_bp: 150,
        max_organizer_fee_bp: 100,
        max_fill_window_secs: 7 * 24 * 3600,
        accept_window_secs: 24 * 3600,
        delivery_grace_secs: 48 * 3600,
        confirm_window_secs: 48 * 3600,
        perishable_confirm_window_secs: 72 * 3600,
        organizer_confirm_window_secs: 48 * 3600,
        early_release_weight_bp: 6000,
        dispute_deposit_bp: 100,
        dispute_deposit_min: usdc / 2,
        arbitration_sla_secs: 5 * 24 * 3600,
        withdraw_lock_secs: 2 * 3600,
        max_member_share_bp: 5000,
        max_members_per_pool: 200,
        advance_enabled: false,
        supplier_tier_caps: vec![
            env,
            2_000 * usdc,
            10_000 * usdc,
            30_000 * usdc,
            100_000 * usdc,
        ],
        organizer_tier_caps: vec![env, 500 * usdc, 5_000 * usdc, 25_000 * usdc],
        trader_tier_caps: vec![env, 50 * usdc, 500 * usdc, 2_500 * usdc],
        supplier_advance_bp: vec![env, 0u32, 0, 2000, 3000],
    }
}

fn validate(p: &Params) -> Result<(), Error> {
    if p.platform_fee_bp > caps::PLATFORM_FEE_BP
        || p.max_organizer_fee_bp > caps::ORGANIZER_FEE_BP
        || p.max_fill_window_secs > caps::FILL_WINDOW_SECS
        || p.arbitration_sla_secs > caps::ARBITRATION_SLA_SECS
        || p.dispute_deposit_bp > caps::DISPUTE_DEPOSIT_BP
    {
        return Err(Error::ParamsExceedHardCap);
    }
    for i in 0..p.supplier_advance_bp.len() {
        if p.supplier_advance_bp.get(i).unwrap_or(u32::MAX) > caps::ADVANCE_BP {
            return Err(Error::ParamsExceedHardCap);
        }
    }
    let ok = p.max_fill_window_secs > 0
        && p.accept_window_secs > 0
        && p.confirm_window_secs > 0
        && p.perishable_confirm_window_secs >= p.confirm_window_secs
        && p.arbitration_sla_secs > 0
        && p.early_release_weight_bp > 0
        && p.early_release_weight_bp <= 10_000
        && p.max_member_share_bp > 0
        && p.max_member_share_bp <= 10_000
        && p.max_members_per_pool > 0
        && p.dispute_deposit_min >= 0
        && p.supplier_tier_caps.len() == 4
        && p.supplier_advance_bp.len() == 4
        && p.organizer_tier_caps.len() == 3
        && p.trader_tier_caps.len() == 3;
    if ok {
        Ok(())
    } else {
        Err(Error::ParamsInvalid)
    }
}

fn admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&Key::Admin)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotAdmin))
}

fn require_admin(env: &Env) {
    admin(env).require_auth();
}

#[contract]
pub struct Config;

#[contractimpl]
impl Config {
    pub fn __constructor(env: Env, admin: Address, usdc: Address, treasury: Address) {
        let s = env.storage().instance();
        s.set(&Key::Admin, &admin);
        s.set(&Key::Usdc, &usdc);
        s.set(&Key::Treasury, &treasury);
        s.set(&Key::Params, &default_params(&env));
    }

    pub fn admin(env: Env) -> Address {
        admin(&env)
    }
    pub fn usdc(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&Key::Usdc)
            .unwrap_or_else(|| panic_with_error!(&env, Error::AddressNotSet))
    }
    pub fn treasury(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&Key::Treasury)
            .unwrap_or_else(|| panic_with_error!(&env, Error::AddressNotSet))
    }

    pub fn set_params(env: Env, params: Params) {
        require_admin(&env);
        if let Err(e) = validate(&params) {
            panic_with_error!(&env, e);
        }
        env.storage().instance().set(&Key::Params, &params);
        bump_instance(&env);
        emit(
            &env,
            C,
            symbol_short!("params"),
            symbol_short!("updated"),
            params.platform_fee_bp,
        );
    }

    pub fn get_params(env: Env) -> Params {
        bump_instance(&env);
        env.storage()
            .instance()
            .get(&Key::Params)
            .unwrap_or_else(|| panic_with_error!(&env, Error::ParamsInvalid))
    }

    pub fn set_address(env: Env, key: Symbol, addr: Address) {
        require_admin(&env);
        env.storage().instance().set(&Key::Addr(key), &addr);
    }

    pub fn get_address(env: Env, key: Symbol) -> Address {
        env.storage()
            .instance()
            .get(&Key::Addr(key))
            .unwrap_or_else(|| panic_with_error!(&env, Error::AddressNotSet))
    }

    pub fn set_attestor(env: Env, a: Address, enabled: bool) {
        require_admin(&env);
        env.storage().persistent().set(&Key::Attestor(a), &enabled);
    }
    pub fn is_attestor(env: Env, a: Address) -> bool {
        env.storage()
            .persistent()
            .get(&Key::Attestor(a))
            .unwrap_or(false)
    }

    pub fn set_arbiter(env: Env, a: Address, enabled: bool) {
        require_admin(&env);
        env.storage().persistent().set(&Key::Arbiter(a), &enabled);
    }
    pub fn is_arbiter(env: Env, a: Address) -> bool {
        env.storage()
            .persistent()
            .get(&Key::Arbiter(a))
            .unwrap_or(false)
    }

    pub fn set_category(env: Env, cat: Symbol, allowed: bool) {
        require_admin(&env);
        env.storage()
            .persistent()
            .set(&Key::Category(cat), &allowed);
    }
    pub fn is_category_allowed(env: Env, cat: Symbol) -> bool {
        env.storage()
            .persistent()
            .get(&Key::Category(cat))
            .unwrap_or(false)
    }

    pub fn set_guardian(env: Env, g: Address) {
        require_admin(&env);
        env.storage().instance().set(&Key::Guardian, &g);
    }

    /// Scopes: "create", "commit", "accept", "all". Admin or guardian.
    /// Pausing never blocks refunds, expiry, failures, dispute timeouts or settlement.
    pub fn pause(env: Env, caller: Address, scope: Symbol) {
        caller.require_auth();
        let guardian: Option<Address> = env.storage().instance().get(&Key::Guardian);
        if caller != admin(&env) && Some(caller.clone()) != guardian {
            panic_with_error!(&env, Error::NotAdminOrGuardian);
        }
        env.storage()
            .persistent()
            .set(&Key::Paused(scope.clone()), &true);
        emit(&env, C, symbol_short!("paused"), scope, caller);
    }

    pub fn unpause(env: Env, scope: Symbol) {
        require_admin(&env);
        env.storage()
            .persistent()
            .set(&Key::Paused(scope.clone()), &false);
        emit(&env, C, symbol_short!("unpaused"), scope, true);
    }

    pub fn is_paused(env: Env, scope: Symbol) -> bool {
        let get = |s: Symbol| {
            env.storage()
                .persistent()
                .get(&Key::Paused(s))
                .unwrap_or(false)
        };
        get(symbol_short!("all")) || get(scope)
    }

    pub fn upgrade(env: Env, wasm_hash: BytesN<32>) {
        require_admin(&env);
        env.deployer()
            .update_current_contract(soroban_sdk::ContractExecutable::Wasm(wasm_hash.clone()));
        emit(
            &env,
            C,
            symbol_short!("upgraded"),
            symbol_short!("config"),
            wasm_hash,
        );
    }
}

#[allow(dead_code)]
fn _unused(_: Vec<u32>) {}

#[cfg(test)]
mod test;
