#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, symbol_short, Address,
    BytesN, Env, Symbol,
};
use sp_common::clients::ConfigClient;
use sp_common::events::emit;
use sp_common::keys;
use sp_common::ttl::bump_persistent;
use sp_common::types::{Role, Status};

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    AlreadyRegistered = 200,
    NotRegistered = 201,
    NotAttestor = 202,
    NotAdmin = 203,
    NotAuthorizedCaller = 204,
    InvalidLevel = 205,
    BadStatus = 206,
}

#[contracttype]
#[derive(Clone)]
enum Key {
    Config,
    Rec(Address, Role),
    Cluster(Address),
}

#[contracttype]
#[derive(Clone)]
struct Rec {
    status: Status,
    level: u32,
    profile_hash: BytesN<32>,
    ver_hash: Option<BytesN<32>>,
}

const C: Symbol = symbol_short!("registry");

fn cfg(env: &Env) -> ConfigClient<'_> {
    let a: Address = env
        .storage()
        .instance()
        .get(&Key::Config)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotAdmin));
    ConfigClient::new(env, &a)
}

fn load(env: &Env, user: &Address, role: &Role) -> Rec {
    let k = Key::Rec(user.clone(), *role);
    let r: Rec = env
        .storage()
        .persistent()
        .get(&k)
        .unwrap_or_else(|| panic_with_error!(env, Error::NotRegistered));
    bump_persistent(env, &k);
    r
}

fn save(env: &Env, user: &Address, role: &Role, rec: &Rec) {
    let k = Key::Rec(user.clone(), *role);
    env.storage().persistent().set(&k, rec);
    bump_persistent(env, &k);
}

fn require_admin(env: &Env, admin: &Address) {
    admin.require_auth();
    if *admin != cfg(env).admin() {
        panic_with_error!(env, Error::NotAdmin);
    }
}

#[contract]
pub struct Registry;

#[contractimpl]
impl Registry {
    pub fn __constructor(env: Env, config: Address) {
        env.storage().instance().set(&Key::Config, &config);
    }

    pub fn register(
        env: Env,
        user: Address,
        role: Role,
        profile_hash: BytesN<32>,
        cluster: Option<Symbol>,
    ) {
        user.require_auth();
        let k = Key::Rec(user.clone(), role);
        if env.storage().persistent().has(&k) {
            panic_with_error!(&env, Error::AlreadyRegistered);
        }
        save(
            &env,
            &user,
            &role,
            &Rec {
                status: Status::Registered,
                level: 0,
                profile_hash,
                ver_hash: None,
            },
        );
        if let Some(c) = cluster {
            let ck = Key::Cluster(user.clone());
            env.storage().persistent().set(&ck, &c);
            bump_persistent(&env, &ck);
        }
        emit(&env, C, symbol_short!("user_reg"), user, role);
    }

    /// Writes a verification hash and level. Level 0 is not a verification.
    pub fn attest(
        env: Env,
        attestor: Address,
        user: Address,
        role: Role,
        ver_hash: BytesN<32>,
        level: u32,
    ) {
        attestor.require_auth();
        if !cfg(&env).is_attestor(&attestor) {
            panic_with_error!(&env, Error::NotAttestor);
        }
        if level == 0 {
            panic_with_error!(&env, Error::InvalidLevel);
        }
        let mut rec = load(&env, &user, &role);
        if rec.status == Status::Suspended || rec.status == Status::Closed {
            panic_with_error!(&env, Error::BadStatus);
        }
        rec.status = Status::Verified;
        rec.level = level;
        rec.ver_hash = Some(ver_hash);
        save(&env, &user, &role, &rec);
        emit(&env, C, symbol_short!("user_att"), user, (role, level));
    }

    pub fn revoke(env: Env, admin: Address, user: Address, role: Role, reason: Symbol) {
        require_admin(&env, &admin);
        let mut rec = load(&env, &user, &role);
        rec.status = Status::Registered;
        rec.level = 0;
        rec.ver_hash = None;
        save(&env, &user, &role, &rec);
        emit(&env, C, symbol_short!("user_rev"), user, (role, reason));
    }

    /// Admin, or the `group_buy` / `disputes` contracts.
    pub fn suspend(env: Env, caller: Address, user: Address, role: Role, reason: Symbol) {
        caller.require_auth();
        let c = cfg(&env);
        let ok = caller == c.admin()
            || caller == c.get_address(&keys::GROUP_BUY)
            || caller == c.get_address(&keys::DISPUTES);
        if !ok {
            panic_with_error!(&env, Error::NotAuthorizedCaller);
        }
        let mut rec = load(&env, &user, &role);
        rec.status = Status::Suspended;
        save(&env, &user, &role, &rec);
        emit(&env, C, symbol_short!("user_sus"), user, (role, reason));
    }

    pub fn unsuspend(env: Env, admin: Address, user: Address, role: Role) {
        require_admin(&env, &admin);
        let mut rec = load(&env, &user, &role);
        if rec.status != Status::Suspended {
            panic_with_error!(&env, Error::BadStatus);
        }
        rec.status = if rec.level > 0 {
            Status::Verified
        } else {
            Status::Registered
        };
        save(&env, &user, &role, &rec);
        emit(&env, C, symbol_short!("user_uns"), user, role);
    }

    pub fn status(env: Env, user: Address, role: Role) -> Status {
        match env
            .storage()
            .persistent()
            .get::<_, Rec>(&Key::Rec(user, role))
        {
            Some(r) => r.status,
            None => Status::Unregistered,
        }
    }

    pub fn level(env: Env, user: Address, role: Role) -> u32 {
        env.storage()
            .persistent()
            .get::<_, Rec>(&Key::Rec(user, role))
            .map(|r| r.level)
            .unwrap_or(0)
    }

    pub fn cluster_of(env: Env, user: Address) -> Option<Symbol> {
        env.storage().persistent().get(&Key::Cluster(user))
    }
}

#[cfg(test)]
mod test;
