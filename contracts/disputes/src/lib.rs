#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, symbol_short, token,
    Address, BytesN, Env, Symbol, Vec,
};
use sp_common::clients::{ConfigClient, GroupBuyClient, ReputationClient};
use sp_common::events::emit;
use sp_common::keys;
use sp_common::math::{bp_of, mul_div_floor};
use sp_common::ttl::{bump_instance, bump_persistent};
use sp_common::types::{Dispute, DisputeReason, Outcome, Role};

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    NotInitialised = 400,
    NotAParty = 401,
    BadClaim = 402,
    NotFound = 403,
    AlreadyResolved = 404,
    NotArbiter = 405,
    ArbiterConflict = 406,
    SlaPassed = 407,
    SlaNotReached = 408,
    BadOutcome = 409,
    TooMuchEvidence = 410,
    Overflow = 411,
}

#[contracttype]
#[derive(Clone)]
enum Key {
    Config,
    NextId,
    Dispute(u64),
    PoolList(u64),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Member,
    Organizer,
    Supplier,
}

const C: Symbol = symbol_short!("disputes");
const MAX_EVIDENCE: u32 = 20;

fn fail<T>(env: &Env, e: Error) -> T {
    panic_with_error!(env, e)
}
fn cfg(env: &Env) -> ConfigClient<'_> {
    let a: Address = env
        .storage()
        .instance()
        .get(&Key::Config)
        .unwrap_or_else(|| fail(env, Error::NotInitialised));
    ConfigClient::new(env, &a)
}
fn gb(env: &Env) -> GroupBuyClient<'_> {
    GroupBuyClient::new(env, &cfg(env).get_address(&keys::GROUP_BUY))
}
fn me(env: &Env) -> Address {
    env.current_contract_address()
}
fn ck<T>(env: &Env, v: Option<T>) -> T {
    v.unwrap_or_else(|| fail(env, Error::Overflow))
}
fn load(env: &Env, id: u64) -> Dispute {
    let k = Key::Dispute(id);
    let d: Dispute = env
        .storage()
        .persistent()
        .get(&k)
        .unwrap_or_else(|| fail(env, Error::NotFound));
    bump_persistent(env, &k);
    d
}
fn save(env: &Env, d: &Dispute) {
    let k = Key::Dispute(d.id);
    env.storage().persistent().set(&k, d);
    bump_persistent(env, &k);
}
fn rep(env: &Env, subject: &Address, role: Role, ev: Symbol) {
    let r = ReputationClient::new(env, &cfg(env).get_address(&keys::REPUTATION));
    r.record(&me(env), subject, &role, &ev);
}
fn role_of(side: Side) -> Role {
    match side {
        Side::Member => Role::Trader,
        Side::Organizer => Role::Organizer,
        Side::Supplier => Role::Supplier,
    }
}

fn side_of(env: &Env, pool_id: u64, who: &Address) -> Option<Side> {
    let g = gb(env);
    let pool = g.pool(&pool_id);
    if *who == pool.terms.supplier {
        Some(Side::Supplier)
    } else if *who == pool.organizer {
        Some(Side::Organizer)
    } else if g
        .commitment(&pool_id, who)
        .map(|c| c.units > 0)
        .unwrap_or(false)
    {
        Some(Side::Member)
    } else {
        None
    }
}

fn remove_from_pool_list(env: &Env, pool_id: u64, id: u64) {
    let k = Key::PoolList(pool_id);
    let list: Vec<u64> = env
        .storage()
        .persistent()
        .get(&k)
        .unwrap_or_else(|| Vec::new(env));
    let mut out = Vec::new(env);
    for x in list.iter() {
        if x != id {
            out.push_back(x);
        }
    }
    env.storage().persistent().set(&k, &out);
}

/// Pays out a deposit and applies the money outcome to `group_buy`.
fn finish(
    env: &Env,
    d: &mut Dispute,
    side: Side,
    outcome: Outcome,
    to_member: i128,
    to_supplier: i128,
    deposit_back: bool,
) {
    let g = gb(env);
    let pool = g.pool(&d.pool_id);
    let target = if side == Side::Member {
        Some(d.opener.clone())
    } else {
        None
    };
    g.apply_outcome(&me(env), &d.pool_id, &target, &to_member, &to_supplier);
    d.resolved = true;
    d.outcome = Vec::from_array(env, [outcome]);
    save(env, d);
    remove_from_pool_list(env, d.pool_id, d.id);
    let usdc = token::Client::new(env, &cfg(env).usdc());
    if d.deposit > 0 {
        let to = if deposit_back {
            d.opener.clone()
        } else if side == Side::Supplier {
            pool.organizer.clone()
        } else {
            pool.terms.supplier.clone()
        };
        usdc.transfer(&me(env), &to, &d.deposit);
    }
    let opener_won = deposit_back;
    if side != Side::Supplier && !opener_won {
        rep(env, &d.opener, role_of(side), symbol_short!("d_lost"));
    } else if opener_won {
        rep(env, &d.opener, role_of(side), symbol_short!("d_won"));
    } else {
        rep(env, &d.opener, role_of(side), symbol_short!("d_lost"));
    }
    // The supplier loses when the group side takes at least half of the claim.
    if side != Side::Supplier && to_member > 0 && to_member * 2 >= d.claimed_amount {
        rep(
            env,
            &pool.terms.supplier,
            Role::Supplier,
            symbol_short!("d_lost"),
        );
    }
}

#[contract]
pub struct Disputes;

#[contractimpl]
impl Disputes {
    pub fn __constructor(env: Env, config: Address) {
        env.storage().instance().set(&Key::Config, &config);
        env.storage().instance().set(&Key::NextId, &1u64);
    }

    pub fn open(
        env: Env,
        opener: Address,
        pool_id: u64,
        reason: DisputeReason,
        claimed_units: u32,
        evidence: BytesN<32>,
    ) -> u64 {
        opener.require_auth();
        let g = gb(&env);
        let pool = g.pool(&pool_id);
        let side = side_of(&env, pool_id, &opener).unwrap_or_else(|| fail(&env, Error::NotAParty));
        let limit = match side {
            Side::Member => g.allocated_units_of(&pool_id, &opener),
            Side::Organizer => pool.received_units,
            Side::Supplier => pool.total_units,
        };
        if claimed_units == 0 || claimed_units > limit {
            fail::<()>(&env, Error::BadClaim);
        }
        let claimed_amount = ck(&env, (claimed_units as i128).checked_mul(pool.final_price));
        let params = cfg(&env).get_params();
        let pct = ck(&env, bp_of(claimed_amount, params.dispute_deposit_bp));
        let deposit = pct.max(params.dispute_deposit_min);

        let id: u64 = env.storage().instance().get(&Key::NextId).unwrap_or(1);
        env.storage().instance().set(&Key::NextId, &(id + 1));
        bump_instance(&env);

        // Freezes first: group_buy rejects if the window is closed or the amount is too large.
        g.freeze(&me(&env), &pool_id, &claimed_amount);
        if deposit > 0 {
            token::Client::new(&env, &cfg(&env).usdc()).transfer(&opener, me(&env), &deposit);
        }
        let d = Dispute {
            id,
            pool_id,
            opener: opener.clone(),
            reason,
            claimed_units,
            claimed_amount,
            deposit,
            evidence: Vec::from_array(&env, [evidence]),
            opened_at: env.ledger().timestamp(),
            resolved: false,
            outcome: Vec::new(&env),
        };
        save(&env, &d);
        let lk = Key::PoolList(pool_id);
        let mut list: Vec<u64> = env
            .storage()
            .persistent()
            .get(&lk)
            .unwrap_or_else(|| Vec::new(&env));
        list.push_back(id);
        env.storage().persistent().set(&lk, &list);
        bump_persistent(&env, &lk);
        rep(&env, &opener, role_of(side), symbol_short!("d_open"));
        emit(
            &env,
            C,
            symbol_short!("d_open"),
            id,
            (pool_id, opener, claimed_amount),
        );
        id
    }

    pub fn add_evidence(env: Env, party: Address, dispute_id: u64, evidence: BytesN<32>) {
        party.require_auth();
        let mut d = load(&env, dispute_id);
        if d.resolved {
            fail::<()>(&env, Error::AlreadyResolved);
        }
        if side_of(&env, d.pool_id, &party).is_none()
            || (party != d.opener && {
                let pool = gb(&env).pool(&d.pool_id);
                party != pool.terms.supplier && party != pool.organizer
            })
        {
            fail::<()>(&env, Error::NotAParty);
        }
        if d.evidence.len() >= MAX_EVIDENCE {
            fail::<()>(&env, Error::TooMuchEvidence);
        }
        d.evidence.push_back(evidence.clone());
        save(&env, &d);
        emit(
            &env,
            C,
            symbol_short!("d_evid"),
            dispute_id,
            (party, evidence),
        );
    }

    pub fn resolve(
        env: Env,
        arbiter: Address,
        dispute_id: u64,
        outcome: Outcome,
        reasoning_hash: BytesN<32>,
    ) {
        arbiter.require_auth();
        let c = cfg(&env);
        if !c.is_arbiter(&arbiter) {
            fail::<()>(&env, Error::NotArbiter);
        }
        let mut d = load(&env, dispute_id);
        if d.resolved {
            fail::<()>(&env, Error::AlreadyResolved);
        }
        let params = c.get_params();
        if env.ledger().timestamp() > d.opened_at + params.arbitration_sla_secs {
            fail::<()>(&env, Error::SlaPassed);
        }
        // An arbiter cannot rule on a pool they organized, supplied or joined.
        if side_of(&env, d.pool_id, &arbiter).is_some() {
            fail::<()>(&env, Error::ArbiterConflict);
        }
        let side = side_of(&env, d.pool_id, &d.opener).unwrap_or(Side::Member);
        let a = d.claimed_amount;
        let (to_member, to_supplier, opener_won) = match outcome {
            Outcome::ReleaseToSupplier => (0, a, side == Side::Supplier),
            Outcome::RefundMember(units) => {
                if units > d.claimed_units {
                    fail::<()>(&env, Error::BadOutcome);
                }
                let pool = gb(&env).pool(&d.pool_id);
                let m = ck(&env, (units as i128).checked_mul(pool.final_price));
                (m, a - m, side != Side::Supplier && units > 0)
            }
            Outcome::Split(bp) => {
                if bp > 10_000 {
                    fail::<()>(&env, Error::BadOutcome);
                }
                let m = ck(&env, mul_div_floor(a, bp as i128, 10_000));
                (m, a - m, true)
            }
            Outcome::RefundPool => (a, 0, side != Side::Supplier),
        };
        finish(
            &env,
            &mut d,
            side,
            outcome,
            to_member,
            to_supplier,
            opener_won,
        );
        emit(
            &env,
            C,
            symbol_short!("d_resolve"),
            dispute_id,
            (arbiter, reasoning_hash),
        );
    }

    /// Permissionless once the arbitration SLA has passed: the frozen amount goes to the disputer.
    pub fn timeout(env: Env, dispute_id: u64) {
        let mut d = load(&env, dispute_id);
        if d.resolved {
            fail::<()>(&env, Error::AlreadyResolved);
        }
        let params = cfg(&env).get_params();
        if env.ledger().timestamp() <= d.opened_at + params.arbitration_sla_secs {
            fail::<()>(&env, Error::SlaNotReached);
        }
        let side = side_of(&env, d.pool_id, &d.opener).unwrap_or(Side::Member);
        let a = d.claimed_amount;
        if side == Side::Supplier {
            finish(&env, &mut d, side, Outcome::ReleaseToSupplier, 0, a, true);
        } else {
            let units = d.claimed_units;
            finish(&env, &mut d, side, Outcome::RefundMember(units), a, 0, true);
        }
        emit(&env, C, symbol_short!("d_timeout"), dispute_id, true);
    }

    pub fn dispute(env: Env, id: u64) -> Dispute {
        load(&env, id)
    }

    pub fn open_disputes_of(env: Env, pool_id: u64) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&Key::PoolList(pool_id))
            .unwrap_or_else(|| Vec::new(&env))
    }
}
