#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, symbol_short, token,
    Address, Env, Symbol,
};
use sp_common::clients::ConfigClient;
use sp_common::events::emit;
use sp_common::keys;
use sp_common::ttl::bump_persistent;

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    InvalidAmount = 500,
    InsufficientFreeBond = 501,
    NotGroupBuy = 502,
    NotInitialised = 503,
    ReleaseExceedsReserved = 504,
}

#[contracttype]
#[derive(Clone)]
enum Key {
    Config,
    Bond(Address),
}

#[contracttype]
#[derive(Clone, Default)]
struct BondRec {
    total: i128,
    reserved: i128,
}

const C: Symbol = symbol_short!("bond");

fn cfg(env: &Env) -> ConfigClient<'_> {
    let a: Address = env
        .storage()
        .instance()
        .get(&Key::Config)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialised));
    ConfigClient::new(env, &a)
}

fn load(env: &Env, s: &Address) -> BondRec {
    env.storage()
        .persistent()
        .get(&Key::Bond(s.clone()))
        .unwrap_or_default()
}

fn save(env: &Env, s: &Address, b: &BondRec) {
    let k = Key::Bond(s.clone());
    env.storage().persistent().set(&k, b);
    bump_persistent(env, &k);
}

fn require_group_buy(env: &Env, caller: &Address) {
    caller.require_auth();
    if *caller != cfg(env).get_address(&keys::GROUP_BUY) {
        panic_with_error!(env, Error::NotGroupBuy);
    }
}

#[contract]
pub struct SupplierBond;

#[contractimpl]
impl SupplierBond {
    pub fn __constructor(env: Env, config: Address) {
        env.storage().instance().set(&Key::Config, &config);
    }

    pub fn deposit(env: Env, supplier: Address, amount: i128) {
        supplier.require_auth();
        if amount <= 0 {
            panic_with_error!(&env, Error::InvalidAmount);
        }
        let mut b = load(&env, &supplier);
        b.total += amount;
        save(&env, &supplier, &b);
        token::Client::new(&env, &cfg(&env).usdc()).transfer(
            &supplier,
            env.current_contract_address(),
            &amount,
        );
        emit(&env, C, symbol_short!("deposit"), supplier, amount);
    }

    /// Only the free part of the bond (total minus outstanding advances).
    pub fn withdraw(env: Env, supplier: Address, amount: i128) {
        supplier.require_auth();
        let mut b = load(&env, &supplier);
        if amount <= 0 {
            panic_with_error!(&env, Error::InvalidAmount);
        }
        if amount > b.total - b.reserved {
            panic_with_error!(&env, Error::InsufficientFreeBond);
        }
        b.total -= amount;
        save(&env, &supplier, &b);
        token::Client::new(&env, &cfg(&env).usdc()).transfer(
            &env.current_contract_address(),
            &supplier,
            &amount,
        );
        emit(&env, C, symbol_short!("withdraw"), supplier, amount);
    }

    pub fn reserve_for_advance(env: Env, caller: Address, supplier: Address, amount: i128) {
        require_group_buy(&env, &caller);
        let mut b = load(&env, &supplier);
        if amount <= 0 || amount > b.total - b.reserved {
            panic_with_error!(&env, Error::InsufficientFreeBond);
        }
        b.reserved += amount;
        save(&env, &supplier, &b);
    }

    pub fn release_advance(env: Env, caller: Address, supplier: Address, amount: i128) {
        require_group_buy(&env, &caller);
        let mut b = load(&env, &supplier);
        if amount < 0 || amount > b.reserved {
            panic_with_error!(&env, Error::ReleaseExceedsReserved);
        }
        b.reserved -= amount;
        save(&env, &supplier, &b);
    }

    /// Slashes up to `amount` of the bond to the caller (group_buy) so members can be refunded.
    /// Returns the amount actually slashed.
    pub fn slash(env: Env, caller: Address, supplier: Address, amount: i128) -> i128 {
        require_group_buy(&env, &caller);
        let mut b = load(&env, &supplier);
        let slashed = if amount < b.total { amount } else { b.total };
        b.total -= slashed;
        b.reserved = if amount < b.reserved {
            b.reserved - amount
        } else {
            0
        };
        save(&env, &supplier, &b);
        if slashed > 0 {
            token::Client::new(&env, &cfg(&env).usdc()).transfer(
                &env.current_contract_address(),
                &caller,
                &slashed,
            );
        }
        emit(&env, C, symbol_short!("slashed"), supplier, slashed);
        slashed
    }

    pub fn bond_of(env: Env, supplier: Address) -> (i128, i128) {
        let b = load(&env, &supplier);
        (b.total, b.reserved)
    }
}

#[cfg(test)]
mod test;
