#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, symbol_short, token,
    Address, BytesN, Env, Symbol, Vec,
};
use sp_common::clients::{BondClient, ConfigClient, RegistryClient, ReputationClient};
use sp_common::events::emit;
use sp_common::keys;
use sp_common::math::{
    bp_of, ceiling_price, cumulative_alloc, final_unit_price, tier_index, validate_tiers,
};
use sp_common::ttl::{bump_instance, bump_persistent};
use sp_common::types::{Commitment, Params, Pool, PoolState, PoolTerms, Role, Status};

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    NotInitialised = 300,
    Paused = 301,
    BadTerms = 302,
    CategoryNotAllowed = 303,
    NotVerified = 304,
    BadDeadline = 305,
    FeeTooHigh = 306,
    PoolTooLarge = 307,
    PoolNotFound = 308,
    WrongState = 309,
    DeadlinePassed = 310,
    DeadlineNotReached = 311,
    ZeroUnits = 312,
    NotAllowedMember = 313,
    ClusterMismatch = 314,
    OverMemberLimit = 315,
    OverMemberShare = 316,
    OverPoolMax = 317,
    OverTierCap = 318,
    TooManyMembers = 319,
    AlreadyCommitted = 320,
    NotCommitted = 321,
    WithdrawLocked = 322,
    WouldLowerTier = 323,
    BelowMoq = 324,
    NotOrganizer = 325,
    NotSupplier = 326,
    NotAdmin = 327,
    CannotCancel = 328,
    AcceptWindowPassed = 329,
    AcceptWindowOpen = 330,
    AdvanceDisabled = 331,
    AdvanceTooHigh = 332,
    DeliveryDeadlineNotPassed = 333,
    BadReceivedUnits = 334,
    ConfirmDeadlineNotPassed = 335,
    AlreadyConfirmed = 336,
    AllocationPending = 337,
    SettleTooEarly = 338,
    NothingToClaim = 339,
    NotDisputes = 340,
    FreezeTooLarge = 341,
    DisputeWindowClosed = 342,
    Overflow = 343,
    NotFinal = 344,
    SupplierIsParty = 345,
}

#[contracttype]
#[derive(Clone)]
enum Key {
    Config,
    NextId,
    Pool(u64),
    Commit(u64, Address),
    Page(u64, u32),
}

const C: Symbol = symbol_short!("group_buy");
const PAGE: u32 = 50;
const COLLECTIVE_CONFIRM_BP: u32 = 5000;

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
fn registry(env: &Env) -> RegistryClient<'_> {
    RegistryClient::new(env, &cfg(env).get_address(&keys::REGISTRY))
}
fn reputation(env: &Env) -> ReputationClient<'_> {
    ReputationClient::new(env, &cfg(env).get_address(&keys::REPUTATION))
}
fn bond(env: &Env) -> BondClient<'_> {
    BondClient::new(env, &cfg(env).get_address(&keys::BOND))
}
fn usdc(env: &Env) -> token::Client<'_> {
    token::Client::new(env, &cfg(env).usdc())
}
fn me(env: &Env) -> Address {
    env.current_contract_address()
}
fn now(env: &Env) -> u64 {
    env.ledger().timestamp()
}
fn rep(env: &Env, subject: &Address, role: Role, ev: Symbol) {
    reputation(env).record(&me(env), subject, &role, &ev);
}
fn ck<T>(env: &Env, v: Option<T>) -> T {
    v.unwrap_or_else(|| fail(env, Error::Overflow))
}

fn load_pool(env: &Env, id: u64) -> Pool {
    let k = Key::Pool(id);
    let p: Pool = env
        .storage()
        .persistent()
        .get(&k)
        .unwrap_or_else(|| fail(env, Error::PoolNotFound));
    bump_persistent(env, &k);
    p
}
fn save_pool(env: &Env, p: &Pool) {
    let k = Key::Pool(p.id);
    env.storage().persistent().set(&k, p);
    bump_persistent(env, &k);
}
fn load_commit(env: &Env, id: u64, m: &Address) -> Option<Commitment> {
    let k = Key::Commit(id, m.clone());
    let c: Option<Commitment> = env.storage().persistent().get(&k);
    if c.is_some() {
        bump_persistent(env, &k);
    }
    c
}
fn save_commit(env: &Env, id: u64, c: &Commitment) {
    let k = Key::Commit(id, c.member.clone());
    env.storage().persistent().set(&k, c);
    bump_persistent(env, &k);
}
fn push_member(env: &Env, id: u64, idx: u32, m: &Address) {
    let k = Key::Page(id, idx / PAGE);
    let mut page: Vec<Address> = env
        .storage()
        .persistent()
        .get(&k)
        .unwrap_or_else(|| Vec::new(env));
    page.push_back(m.clone());
    env.storage().persistent().set(&k, &page);
    bump_persistent(env, &k);
}
fn member_at(env: &Env, id: u64, idx: u32) -> Option<Address> {
    let page: Vec<Address> = env.storage().persistent().get(&Key::Page(id, idx / PAGE))?;
    page.get(idx % PAGE)
}

fn require_paused_not(env: &Env, scope: Symbol) {
    if cfg(env).is_paused(&scope) {
        fail::<()>(env, Error::Paused);
    }
}

fn cap_at(env: &Env, caps: &Vec<i128>, tier: u32) -> i128 {
    caps.get(tier).unwrap_or_else(|| fail(env, Error::Overflow))
}

/// Final price is fixed on Filled.
fn fill(env: &Env, pool: &mut Pool) {
    pool.state = PoolState::Filled;
    pool.filled_at = now(env);
    pool.final_price = ck(env, final_unit_price(&pool.terms.tiers, pool.total_units));
    emit(
        env,
        C,
        symbol_short!("filled"),
        pool.id,
        (pool.total_units, pool.final_price),
    );
}

/// Moves a pool to a refundable end state. Slashes the supplier bond if an advance was paid.
fn end_refundable(env: &Env, pool: &mut Pool, state: PoolState, blame_supplier: bool) {
    if pool.advance_paid > 0 {
        let slashed = bond(env).slash(&me(env), &pool.terms.supplier, &pool.advance_paid);
        pool.escrow_balance = ck(env, pool.escrow_balance.checked_add(slashed));
        pool.advance_paid = 0;
    }
    pool.state = state;
    pool.owed_total = pool.total_paid;
    if blame_supplier {
        rep(
            env,
            &pool.terms.supplier,
            Role::Supplier,
            symbol_short!("failed"),
        );
    }
}

fn confirm_window(params: &Params, pool: &Pool) -> u64 {
    if pool.terms.perishable {
        params.perishable_confirm_window_secs
    } else {
        params.confirm_window_secs
    }
}

fn delivery_deadline(params: &Params, pool: &Pool) -> u64 {
    pool.accepted_at + pool.terms.lead_time_secs + params.delivery_grace_secs
}

fn confirm_deadline_of(params: &Params, pool: &Pool) -> u64 {
    let base = if pool.dispatched_at > 0 {
        pool.dispatched_at
    } else {
        pool.accepted_at
    };
    base + pool.terms.lead_time_secs + params.organizer_confirm_window_secs
}

fn is_final_refundable(s: &PoolState) -> bool {
    matches!(
        s,
        PoolState::Expired | PoolState::Failed | PoolState::Cancelled | PoolState::Settled
    )
}

fn alloc_units(pool: &Pool, c: &Commitment) -> u32 {
    if pool.received_units == pool.total_units {
        c.units
    } else {
        c.allocated_units
    }
}

/// Total refund owed to a member (claimed amounts are tracked separately).
fn refund_amount(env: &Env, pool: &Pool, c: &Commitment) -> i128 {
    match pool.state {
        PoolState::Expired | PoolState::Failed | PoolState::Cancelled => c.paid,
        PoolState::Settled => {
            let alloc = alloc_units(pool, c) as i128;
            let base = c.paid - ck(env, alloc.checked_mul(pool.final_price));
            let share = if pool.extra_refund_pool > 0 && pool.total_units > 0 {
                ck(
                    env,
                    sp_common::math::mul_div_floor(
                        pool.extra_refund_pool,
                        c.units as i128,
                        pool.total_units as i128,
                    ),
                )
            } else {
                0
            };
            base + share + c.extra_credit
        }
        _ => 0,
    }
}

/// Fees and supplier net for releasing `amount` to the supplier.
fn split_payable(env: &Env, pool: &Pool, amount: i128) -> (i128, i128, i128) {
    let org = ck(env, bp_of(amount, pool.organizer_fee_bp));
    let plat = ck(env, bp_of(amount, pool.platform_fee_bp));
    (amount - org - plat, plat, org)
}

fn add_units(env: &Env, member: &Address, pool_id: u64, units: u32, is_increase: bool) -> i128 {
    member.require_auth();
    require_paused_not(env, symbol_short!("commit"));
    let params = cfg(env).get_params();
    let mut pool = load_pool(env, pool_id);
    if pool.state != PoolState::Open {
        fail::<()>(env, Error::WrongState);
    }
    if now(env) >= pool.fill_deadline {
        fail::<()>(env, Error::DeadlinePassed);
    }
    if units == 0 {
        fail::<()>(env, Error::ZeroUnits);
    }
    if *member == pool.terms.supplier {
        fail::<()>(env, Error::SupplierIsParty);
    }
    let reg = registry(env);
    let st = reg.status(member, &Role::Trader);
    if st != Status::Registered && st != Status::Verified {
        fail::<()>(env, Error::NotAllowedMember);
    }
    if let Some(cluster) = pool.cluster.clone() {
        if reg.cluster_of(member) != Some(cluster) {
            fail::<()>(env, Error::ClusterMismatch);
        }
    }

    let existing = load_commit(env, pool_id, member);
    let (mut c, is_listed) = match existing {
        Some(c) => (c, true),
        None => (
            Commitment {
                member: member.clone(),
                units: 0,
                paid: 0,
                allocated_units: 0,
                refund_claimed: 0,
                extra_credit: 0,
                picked_up: false,
                delivery_confirmed: false,
                committed_at: now(env),
            },
            false,
        ),
    };
    if is_increase && c.units == 0 {
        fail::<()>(env, Error::NotCommitted);
    }
    if !is_increase && c.units > 0 {
        fail::<()>(env, Error::AlreadyCommitted);
    }

    let new_units = ck(env, c.units.checked_add(units));
    if new_units > pool.terms.max_per_member {
        fail::<()>(env, Error::OverMemberLimit);
    }
    let share_cap = ck(
        env,
        bp_of(pool.terms.max_units as i128, params.max_member_share_bp),
    ) as u32;
    let only_member = pool.member_count == 0 || (pool.member_count == 1 && c.units > 0);
    if !only_member && new_units > share_cap.max(1) {
        fail::<()>(env, Error::OverMemberShare);
    }
    let new_total = ck(env, pool.total_units.checked_add(units));
    if new_total > pool.terms.max_units {
        fail::<()>(env, Error::OverPoolMax);
    }
    if !is_listed && pool.listed_members >= params.max_members_per_pool {
        fail::<()>(env, Error::TooManyMembers);
    }

    let price = ck(env, ceiling_price(&pool.terms.tiers, pool.total_units));
    let amount = ck(env, (units as i128).checked_mul(price));
    let new_paid = ck(env, c.paid.checked_add(amount));
    let lvl = reg.level(member, &Role::Trader);
    let rep_tier = reputation(env).tier(member, &Role::Trader);
    let trader_tier = if lvl == 0 {
        0
    } else if rep_tier >= 2 {
        2
    } else {
        1
    };
    if new_paid > cap_at(env, &params.trader_tier_caps, trader_tier) {
        fail::<()>(env, Error::OverTierCap);
    }

    // effects
    let before_tier = tier_index(&pool.terms.tiers, pool.total_units);
    if c.units == 0 {
        pool.member_count += 1;
    }
    if !is_listed {
        push_member(env, pool_id, pool.listed_members, member);
        pool.listed_members += 1;
    }
    c.units = new_units;
    c.paid = new_paid;
    pool.total_units = new_total;
    pool.total_paid = ck(env, pool.total_paid.checked_add(amount));
    pool.escrow_balance = ck(env, pool.escrow_balance.checked_add(amount));
    save_commit(env, pool_id, &c);
    let after_tier = tier_index(&pool.terms.tiers, new_total);
    if after_tier != before_tier {
        emit(
            env,
            C,
            symbol_short!("tier_up"),
            pool_id,
            (after_tier.unwrap_or(0), new_total),
        );
    }
    let ev = if is_increase {
        symbol_short!("commit_up")
    } else {
        symbol_short!("committed")
    };
    emit(env, C, ev, pool_id, (member.clone(), units, amount));
    if new_total == pool.terms.max_units {
        fill(env, &mut pool);
    }
    save_pool(env, &pool);

    // interaction
    usdc(env).transfer(member, me(env), &amount);
    amount
}

fn release_member_payout(env: &Env, to: &Address, amount: i128) {
    if amount > 0 {
        usdc(env).transfer(&me(env), to, &amount);
    }
}

#[contract]
pub struct GroupBuy;

#[contractimpl]
impl GroupBuy {
    pub fn __constructor(env: Env, config: Address) {
        env.storage().instance().set(&Key::Config, &config);
        env.storage().instance().set(&Key::NextId, &1u64);
    }

    pub fn create_pool(
        env: Env,
        organizer: Address,
        terms: PoolTerms,
        hub_hash: BytesN<32>,
        organizer_fee_bp: u32,
        cluster: Option<Symbol>,
        fill_deadline: u64,
    ) -> u64 {
        organizer.require_auth();
        require_paused_not(&env, symbol_short!("create"));
        let c = cfg(&env);
        let params = c.get_params();
        if !validate_tiers(&terms.tiers, terms.moq, terms.max_units)
            || terms.max_per_member == 0
            || terms.max_per_member > terms.max_units
            || terms.lead_time_secs == 0
            || terms.supplier == organizer
        {
            fail::<()>(&env, Error::BadTerms);
        }
        if !c.is_category_allowed(&terms.category) {
            fail::<()>(&env, Error::CategoryNotAllowed);
        }
        if organizer_fee_bp > params.max_organizer_fee_bp {
            fail::<()>(&env, Error::FeeTooHigh);
        }
        let t = now(&env);
        if fill_deadline <= t || fill_deadline > t + params.max_fill_window_secs {
            fail::<()>(&env, Error::BadDeadline);
        }
        let reg = registry(&env);
        if reg.status(&organizer, &Role::Organizer) != Status::Verified
            || reg.status(&terms.supplier, &Role::Supplier) != Status::Verified
        {
            fail::<()>(&env, Error::NotVerified);
        }
        let first = terms
            .tiers
            .get(0)
            .unwrap_or_else(|| fail(&env, Error::BadTerms));
        let max_value = ck(
            &env,
            (terms.max_units as i128).checked_mul(first.unit_price),
        );
        let r = reputation(&env);
        let s_cap = cap_at(
            &env,
            &params.supplier_tier_caps,
            r.tier(&terms.supplier, &Role::Supplier),
        );
        let o_cap = cap_at(
            &env,
            &params.organizer_tier_caps,
            r.tier(&organizer, &Role::Organizer),
        );
        if max_value > s_cap || max_value > o_cap {
            fail::<()>(&env, Error::PoolTooLarge);
        }

        let id: u64 = env.storage().instance().get(&Key::NextId).unwrap_or(1);
        env.storage().instance().set(&Key::NextId, &(id + 1));
        bump_instance(&env);
        let pool = Pool {
            id,
            organizer: organizer.clone(),
            terms: terms.clone(),
            hub_hash: hub_hash.clone(),
            organizer_fee_bp,
            platform_fee_bp: params.platform_fee_bp,
            cluster,
            fill_deadline,
            created_at: t,
            filled_at: 0,
            accepted_at: 0,
            dispatched_at: 0,
            delivered_at: 0,
            total_units: 0,
            received_units: 0,
            final_price: 0,
            total_paid: 0,
            escrow_balance: 0,
            frozen_amount: 0,
            diverted: 0,
            extra_refund_pool: 0,
            advance_paid: 0,
            member_count: 0,
            listed_members: 0,
            picked_units: 0,
            confirm_units: 0,
            alloc_cursor: 0,
            alloc_cum: 0,
            alloc_done: false,
            refund_cursor: 0,
            owed_total: 0,
            refunds_paid: 0,
            state: PoolState::Open,
        };
        save_pool(&env, &pool);
        emit(
            &env,
            C,
            symbol_short!("pool_new"),
            id,
            (organizer, terms.supplier, terms.offer_hash, hub_hash),
        );
        id
    }

    /// Organizer if nothing is committed yet; admin any time before Dispatched.
    pub fn cancel_pool(env: Env, caller: Address, pool_id: u64) {
        caller.require_auth();
        let mut pool = load_pool(&env, pool_id);
        if caller == pool.organizer {
            if pool.state != PoolState::Open || pool.total_units != 0 {
                fail::<()>(&env, Error::CannotCancel);
            }
        } else if caller == cfg(&env).admin() {
            if !matches!(
                pool.state,
                PoolState::Open | PoolState::Filled | PoolState::Accepted
            ) {
                fail::<()>(&env, Error::CannotCancel);
            }
        } else {
            fail::<()>(&env, Error::NotAdmin);
        }
        end_refundable(&env, &mut pool, PoolState::Cancelled, false);
        save_pool(&env, &pool);
        emit(&env, C, symbol_short!("cancelled"), pool_id, caller);
    }

    pub fn commit(env: Env, member: Address, pool_id: u64, units: u32) -> i128 {
        add_units(&env, &member, pool_id, units, false)
    }

    pub fn increase(env: Env, member: Address, pool_id: u64, extra_units: u32) -> i128 {
        add_units(&env, &member, pool_id, extra_units, true)
    }

    /// Allowed while Open and before the lock, and only if it does not lower the pool's price
    /// tier (ADR 0001): that keeps every member's paid amount at or above the final bill.
    pub fn withdraw_commitment(env: Env, member: Address, pool_id: u64) {
        member.require_auth();
        let params = cfg(&env).get_params();
        let mut pool = load_pool(&env, pool_id);
        if pool.state != PoolState::Open {
            fail::<()>(&env, Error::WrongState);
        }
        if now(&env) + params.withdraw_lock_secs > pool.fill_deadline {
            fail::<()>(&env, Error::WithdrawLocked);
        }
        let mut c =
            load_commit(&env, pool_id, &member).unwrap_or_else(|| fail(&env, Error::NotCommitted));
        if c.units == 0 {
            fail::<()>(&env, Error::NotCommitted);
        }
        let remaining = pool.total_units - c.units;
        if tier_index(&pool.terms.tiers, remaining)
            != tier_index(&pool.terms.tiers, pool.total_units)
        {
            fail::<()>(&env, Error::WouldLowerTier);
        }
        let refund = c.paid;
        pool.total_units = remaining;
        pool.total_paid -= refund;
        pool.escrow_balance -= refund;
        pool.member_count -= 1;
        c.units = 0;
        c.paid = 0;
        save_commit(&env, pool_id, &c);
        save_pool(&env, &pool);
        emit(
            &env,
            C,
            symbol_short!("withdrawn"),
            pool_id,
            (member.clone(), refund),
        );
        usdc(&env).transfer(&me(&env), &member, &refund);
    }

    pub fn close_early(env: Env, organizer: Address, pool_id: u64) {
        organizer.require_auth();
        let mut pool = load_pool(&env, pool_id);
        if organizer != pool.organizer {
            fail::<()>(&env, Error::NotOrganizer);
        }
        if pool.state != PoolState::Open {
            fail::<()>(&env, Error::WrongState);
        }
        if pool.total_units < pool.terms.moq {
            fail::<()>(&env, Error::BelowMoq);
        }
        fill(&env, &mut pool);
        save_pool(&env, &pool);
    }

    /// Permissionless once the fill deadline has passed.
    pub fn close(env: Env, pool_id: u64) {
        let mut pool = load_pool(&env, pool_id);
        if pool.state != PoolState::Open {
            fail::<()>(&env, Error::WrongState);
        }
        if now(&env) < pool.fill_deadline {
            fail::<()>(&env, Error::DeadlineNotReached);
        }
        if pool.total_units >= pool.terms.moq {
            fill(&env, &mut pool);
        } else {
            end_refundable(&env, &mut pool, PoolState::Expired, false);
            emit(&env, C, symbol_short!("expired"), pool_id, pool.total_units);
        }
        save_pool(&env, &pool);
    }

    /// `advance_bp` is 0 for no advance; advances are off unless `advance_enabled`.
    pub fn accept(env: Env, supplier: Address, pool_id: u64, advance_bp: u32) {
        supplier.require_auth();
        require_paused_not(&env, symbol_short!("accept"));
        let params = cfg(&env).get_params();
        let mut pool = load_pool(&env, pool_id);
        if supplier != pool.terms.supplier {
            fail::<()>(&env, Error::NotSupplier);
        }
        if pool.state != PoolState::Filled {
            fail::<()>(&env, Error::WrongState);
        }
        if now(&env) > pool.filled_at + params.accept_window_secs {
            fail::<()>(&env, Error::AcceptWindowPassed);
        }
        pool.state = PoolState::Accepted;
        pool.accepted_at = now(&env);
        let mut advance = 0i128;
        if advance_bp > 0 {
            if !params.advance_enabled {
                fail::<()>(&env, Error::AdvanceDisabled);
            }
            let tier = reputation(&env).tier(&supplier, &Role::Supplier);
            if advance_bp > params.supplier_advance_bp.get(tier).unwrap_or(0) {
                fail::<()>(&env, Error::AdvanceTooHigh);
            }
            let base = ck(
                &env,
                (pool.total_units as i128).checked_mul(pool.final_price),
            );
            advance = ck(&env, bp_of(base, advance_bp));
            bond(&env).reserve_for_advance(&me(&env), &supplier, &advance);
            pool.advance_paid = advance;
            pool.escrow_balance -= advance;
        }
        save_pool(&env, &pool);
        emit(
            &env,
            C,
            symbol_short!("accepted"),
            pool_id,
            supplier.clone(),
        );
        if advance > 0 {
            emit(&env, C, symbol_short!("advance"), pool_id, advance);
            usdc(&env).transfer(&me(&env), &supplier, &advance);
        }
    }

    pub fn reject(env: Env, supplier: Address, pool_id: u64) {
        supplier.require_auth();
        let mut pool = load_pool(&env, pool_id);
        if supplier != pool.terms.supplier {
            fail::<()>(&env, Error::NotSupplier);
        }
        if pool.state != PoolState::Filled {
            fail::<()>(&env, Error::WrongState);
        }
        end_refundable(&env, &mut pool, PoolState::Failed, true);
        save_pool(&env, &pool);
        emit(&env, C, symbol_short!("rejected"), pool_id, supplier);
    }

    /// Permissionless after the accept window.
    pub fn fail_accept(env: Env, pool_id: u64) {
        let params = cfg(&env).get_params();
        let mut pool = load_pool(&env, pool_id);
        if pool.state != PoolState::Filled {
            fail::<()>(&env, Error::WrongState);
        }
        if now(&env) <= pool.filled_at + params.accept_window_secs {
            fail::<()>(&env, Error::AcceptWindowOpen);
        }
        end_refundable(&env, &mut pool, PoolState::Failed, true);
        save_pool(&env, &pool);
        emit(
            &env,
            C,
            symbol_short!("failed"),
            pool_id,
            symbol_short!("no_accept"),
        );
    }

    pub fn dispatch(env: Env, supplier: Address, pool_id: u64, waybill_hash: Option<BytesN<32>>) {
        supplier.require_auth();
        let mut pool = load_pool(&env, pool_id);
        if supplier != pool.terms.supplier {
            fail::<()>(&env, Error::NotSupplier);
        }
        if pool.state != PoolState::Accepted {
            fail::<()>(&env, Error::WrongState);
        }
        pool.state = PoolState::Dispatched;
        pool.dispatched_at = now(&env);
        save_pool(&env, &pool);
        emit(&env, C, symbol_short!("dispatch"), pool_id, waybill_hash);
    }

    /// Permissionless after `accepted_at + lead_time + grace`.
    pub fn fail_delivery(env: Env, pool_id: u64) {
        let params = cfg(&env).get_params();
        let mut pool = load_pool(&env, pool_id);
        if !matches!(pool.state, PoolState::Accepted | PoolState::Dispatched) {
            fail::<()>(&env, Error::WrongState);
        }
        if now(&env) <= delivery_deadline(&params, &pool) {
            fail::<()>(&env, Error::DeliveryDeadlineNotPassed);
        }
        end_refundable(&env, &mut pool, PoolState::Failed, true);
        save_pool(&env, &pool);
        emit(
            &env,
            C,
            symbol_short!("failed"),
            pool_id,
            symbol_short!("no_deliv"),
        );
    }

    pub fn confirm_delivery(
        env: Env,
        organizer: Address,
        pool_id: u64,
        received_units: u32,
        evidence: BytesN<32>,
    ) {
        organizer.require_auth();
        let params = cfg(&env).get_params();
        let mut pool = load_pool(&env, pool_id);
        if organizer != pool.organizer {
            fail::<()>(&env, Error::NotOrganizer);
        }
        if !matches!(pool.state, PoolState::Accepted | PoolState::Dispatched) {
            fail::<()>(&env, Error::WrongState);
        }
        if received_units > pool.total_units {
            fail::<()>(&env, Error::BadReceivedUnits);
        }
        let on_time = now(&env) <= delivery_deadline(&params, &pool);
        pool.state = PoolState::Delivered;
        pool.delivered_at = now(&env);
        pool.received_units = received_units;
        pool.alloc_done = received_units == pool.total_units;
        rep(
            &env,
            &pool.terms.supplier,
            Role::Supplier,
            if on_time {
                symbol_short!("on_time")
            } else {
                symbol_short!("late")
            },
        );
        emit(
            &env,
            C,
            symbol_short!("delivered"),
            pool_id,
            (received_units, evidence),
        );
        if received_units < pool.total_units {
            rep(
                &env,
                &pool.terms.supplier,
                Role::Supplier,
                symbol_short!("short"),
            );
            emit(
                &env,
                C,
                symbol_short!("shortfall"),
                pool_id,
                pool.total_units - received_units,
            );
        }
        save_pool(&env, &pool);
    }

    /// Collective fallback when the organizer has not confirmed in time: members holding at
    /// least 50% of units can confirm that the full order arrived.
    pub fn member_confirm_delivery(env: Env, member: Address, pool_id: u64) {
        member.require_auth();
        let params = cfg(&env).get_params();
        let mut pool = load_pool(&env, pool_id);
        if !matches!(pool.state, PoolState::Accepted | PoolState::Dispatched) {
            fail::<()>(&env, Error::WrongState);
        }
        if now(&env) <= confirm_deadline_of(&params, &pool) {
            fail::<()>(&env, Error::ConfirmDeadlineNotPassed);
        }
        let mut c =
            load_commit(&env, pool_id, &member).unwrap_or_else(|| fail(&env, Error::NotCommitted));
        if c.units == 0 {
            fail::<()>(&env, Error::NotCommitted);
        }
        if c.delivery_confirmed {
            fail::<()>(&env, Error::AlreadyConfirmed);
        }
        c.delivery_confirmed = true;
        pool.confirm_units += c.units;
        save_commit(&env, pool_id, &c);
        if (pool.confirm_units as u64) * 10_000
            >= (COLLECTIVE_CONFIRM_BP as u64) * (pool.total_units as u64)
        {
            let on_time = now(&env) <= delivery_deadline(&params, &pool);
            pool.state = PoolState::Delivered;
            pool.delivered_at = now(&env);
            pool.received_units = pool.total_units;
            pool.alloc_done = true;
            rep(
                &env,
                &pool.terms.supplier,
                Role::Supplier,
                if on_time {
                    symbol_short!("on_time")
                } else {
                    symbol_short!("late")
                },
            );
            emit(
                &env,
                C,
                symbol_short!("delivered"),
                pool_id,
                (pool.total_units, BytesN::from_array(&env, &[0u8; 32])),
            );
        }
        save_pool(&env, &pool);
    }

    /// Permissionless, paged. Needed only after a shortfall; fills `allocated_units`.
    pub fn allocate_shortfall(env: Env, pool_id: u64, max: u32) -> bool {
        let mut pool = load_pool(&env, pool_id);
        if pool.state != PoolState::Delivered {
            fail::<()>(&env, Error::WrongState);
        }
        let was_done = pool.alloc_done;
        let mut done = 0u32;
        while !pool.alloc_done && done < max {
            if pool.alloc_cursor >= pool.listed_members {
                pool.alloc_done = true;
                break;
            }
            let m = member_at(&env, pool_id, pool.alloc_cursor)
                .unwrap_or_else(|| fail(&env, Error::Overflow));
            if let Some(mut c) = load_commit(&env, pool_id, &m) {
                if c.units > 0 {
                    c.allocated_units = ck(
                        &env,
                        cumulative_alloc(
                            pool.alloc_cum,
                            c.units,
                            pool.received_units,
                            pool.total_units,
                        ),
                    );
                    pool.alloc_cum += c.units;
                    save_commit(&env, pool_id, &c);
                }
            }
            pool.alloc_cursor += 1;
            done += 1;
            if pool.alloc_cursor >= pool.listed_members {
                pool.alloc_done = true;
            }
        }
        save_pool(&env, &pool);
        if pool.alloc_done && !was_done {
            emit(
                &env,
                C,
                symbol_short!("alloc_ok"),
                pool_id,
                pool.received_units,
            );
        }
        pool.alloc_done
    }

    pub fn confirm_pickup(env: Env, member: Address, pool_id: u64) {
        member.require_auth();
        let mut pool = load_pool(&env, pool_id);
        if !matches!(pool.state, PoolState::Delivered | PoolState::Settled) {
            fail::<()>(&env, Error::WrongState);
        }
        let mut c =
            load_commit(&env, pool_id, &member).unwrap_or_else(|| fail(&env, Error::NotCommitted));
        if c.units == 0 {
            fail::<()>(&env, Error::NotCommitted);
        }
        if c.picked_up {
            fail::<()>(&env, Error::AlreadyConfirmed);
        }
        c.picked_up = true;
        pool.picked_units += c.units;
        save_commit(&env, pool_id, &c);
        save_pool(&env, &pool);
        rep(&env, &member, Role::Trader, symbol_short!("pickup"));
        rep(&env, &member, Role::Trader, symbol_short!("settled"));
        emit(&env, C, symbol_short!("pickup"), pool_id, member);
    }

    /// Permissionless once the confirmation window has closed, or early when members holding
    /// `early_release_weight_bp` of units have confirmed pickup. Frozen amounts stay in escrow.
    pub fn settle(env: Env, pool_id: u64) {
        let params = cfg(&env).get_params();
        let mut pool = load_pool(&env, pool_id);
        if pool.state != PoolState::Delivered {
            fail::<()>(&env, Error::WrongState);
        }
        if !pool.alloc_done {
            fail::<()>(&env, Error::AllocationPending);
        }
        let window_over = now(&env) >= pool.delivered_at + confirm_window(&params, &pool);
        let early = pool.total_units > 0
            && (pool.picked_units as u64) * 10_000
                >= (params.early_release_weight_bp as u64) * (pool.total_units as u64);
        if !window_over && !early {
            fail::<()>(&env, Error::SettleTooEarly);
        }
        let gross = ck(
            &env,
            (pool.received_units as i128).checked_mul(pool.final_price),
        );
        let payable = gross - pool.frozen_amount - pool.diverted;
        if payable < 0 {
            fail::<()>(&env, Error::Overflow);
        }
        let (net, plat, org) = split_payable(&env, &pool, payable);
        let supplier_net = net - pool.advance_paid;
        if supplier_net < 0 {
            fail::<()>(&env, Error::Overflow);
        }
        let advance = pool.advance_paid;
        pool.state = PoolState::Settled;
        pool.owed_total = pool.total_paid - gross + pool.diverted;
        pool.escrow_balance -= supplier_net + plat + org;
        save_pool(&env, &pool);
        emit(
            &env,
            C,
            symbol_short!("settled"),
            pool_id,
            (supplier_net, plat, org),
        );
        if advance > 0 {
            bond(&env).release_advance(&me(&env), &pool.terms.supplier, &advance);
        }
        release_member_payout(&env, &pool.terms.supplier, supplier_net);
        release_member_payout(&env, &cfg(&env).treasury(), plat);
        release_member_payout(&env, &pool.organizer, org);
        rep(
            &env,
            &pool.terms.supplier,
            Role::Supplier,
            symbol_short!("settled"),
        );
        rep(
            &env,
            &pool.organizer,
            Role::Organizer,
            symbol_short!("settled"),
        );
    }

    pub fn claim_refund(env: Env, member: Address, pool_id: u64) -> i128 {
        member.require_auth();
        let mut pool = load_pool(&env, pool_id);
        if !is_final_refundable(&pool.state) {
            fail::<()>(&env, Error::NotFinal);
        }
        let mut c = load_commit(&env, pool_id, &member)
            .unwrap_or_else(|| fail(&env, Error::NothingToClaim));
        let due = refund_amount(&env, &pool, &c) - c.refund_claimed;
        if due <= 0 {
            fail::<()>(&env, Error::NothingToClaim);
        }
        c.refund_claimed += due;
        pool.escrow_balance -= due;
        pool.refunds_paid += due;
        save_commit(&env, pool_id, &c);
        save_pool(&env, &pool);
        emit(
            &env,
            C,
            symbol_short!("refund"),
            pool_id,
            (member.clone(), due),
        );
        usdc(&env).transfer(&me(&env), &member, &due);
        due
    }

    /// Permissionless bounded batch push of refunds. Returns members processed.
    pub fn push_refunds(env: Env, pool_id: u64, max: u32) -> u32 {
        let mut pool = load_pool(&env, pool_id);
        if !is_final_refundable(&pool.state) {
            fail::<()>(&env, Error::NotFinal);
        }
        let mut visited = 0u32;
        while visited < max && pool.refund_cursor < pool.listed_members {
            let m = member_at(&env, pool_id, pool.refund_cursor)
                .unwrap_or_else(|| fail(&env, Error::Overflow));
            pool.refund_cursor += 1;
            visited += 1;
            if let Some(mut c) = load_commit(&env, pool_id, &m) {
                let due = refund_amount(&env, &pool, &c) - c.refund_claimed;
                if due > 0 {
                    c.refund_claimed += due;
                    pool.escrow_balance -= due;
                    pool.refunds_paid += due;
                    save_commit(&env, pool_id, &c);
                    usdc(&env).transfer(&me(&env), &m, &due);
                    emit(&env, C, symbol_short!("refund"), pool_id, (m, due));
                }
            }
        }
        save_pool(&env, &pool);
        visited
    }

    /// Sends rounding dust (escrow beyond what is still owed) to the treasury.
    pub fn sweep_dust(env: Env, pool_id: u64) -> i128 {
        let mut pool = load_pool(&env, pool_id);
        if !is_final_refundable(&pool.state) || pool.frozen_amount != 0 {
            fail::<()>(&env, Error::NotFinal);
        }
        let outstanding = pool.owed_total - pool.refunds_paid;
        let dust = pool.escrow_balance - outstanding;
        if dust <= 0 {
            return 0;
        }
        pool.escrow_balance -= dust;
        save_pool(&env, &pool);
        usdc(&env).transfer(&me(&env), cfg(&env).treasury(), &dust);
        dust
    }

    // ---- hooks callable only by `disputes` ----

    pub fn freeze(env: Env, caller: Address, pool_id: u64, amount: i128) {
        caller.require_auth();
        if caller != cfg(&env).get_address(&keys::DISPUTES) {
            fail::<()>(&env, Error::NotDisputes);
        }
        let params = cfg(&env).get_params();
        let mut pool = load_pool(&env, pool_id);
        // Supplier dispute against an organizer/members who will not confirm: the order is
        // treated as delivered in full and the claimed amount is frozen for arbitration.
        if matches!(pool.state, PoolState::Accepted | PoolState::Dispatched)
            && now(&env) > confirm_deadline_of(&params, &pool)
        {
            pool.state = PoolState::Delivered;
            pool.delivered_at = now(&env);
            pool.received_units = pool.total_units;
            pool.alloc_done = true;
        }
        if pool.state != PoolState::Delivered {
            fail::<()>(&env, Error::WrongState);
        }
        if now(&env) >= pool.delivered_at + confirm_window(&params, &pool) {
            fail::<()>(&env, Error::DisputeWindowClosed);
        }
        let gross = ck(
            &env,
            (pool.received_units as i128).checked_mul(pool.final_price),
        );
        if amount <= 0 || pool.frozen_amount + amount > gross - pool.diverted {
            fail::<()>(&env, Error::FreezeTooLarge);
        }
        pool.frozen_amount += amount;
        save_pool(&env, &pool);
    }

    pub fn apply_outcome(
        env: Env,
        caller: Address,
        pool_id: u64,
        member: Option<Address>,
        to_member: i128,
        to_supplier: i128,
    ) {
        caller.require_auth();
        if caller != cfg(&env).get_address(&keys::DISPUTES) {
            fail::<()>(&env, Error::NotDisputes);
        }
        let mut pool = load_pool(&env, pool_id);
        if !matches!(pool.state, PoolState::Delivered | PoolState::Settled) {
            fail::<()>(&env, Error::WrongState);
        }
        if to_member < 0 || to_supplier < 0 || to_member + to_supplier > pool.frozen_amount {
            fail::<()>(&env, Error::FreezeTooLarge);
        }
        pool.frozen_amount -= to_member + to_supplier;
        if to_member > 0 {
            match member {
                Some(m) => {
                    let mut c = load_commit(&env, pool_id, &m)
                        .unwrap_or_else(|| fail(&env, Error::NotCommitted));
                    c.extra_credit += to_member;
                    save_commit(&env, pool_id, &c);
                }
                None => pool.extra_refund_pool += to_member,
            }
            if pool.state == PoolState::Settled {
                pool.owed_total += to_member;
            } else {
                pool.diverted += to_member;
            }
        }
        let mut payout = (0i128, 0i128, 0i128);
        if to_supplier > 0 && pool.state == PoolState::Settled {
            payout = split_payable(&env, &pool, to_supplier);
            pool.escrow_balance -= to_supplier;
        }
        save_pool(&env, &pool);
        if payout.0 > 0 || payout.1 > 0 || payout.2 > 0 {
            release_member_payout(&env, &pool.terms.supplier, payout.0);
            release_member_payout(&env, &cfg(&env).treasury(), payout.1);
            release_member_payout(&env, &pool.organizer, payout.2);
        }
    }

    // ---- views ----

    pub fn pool(env: Env, id: u64) -> Pool {
        load_pool(&env, id)
    }

    pub fn commitment(env: Env, pool_id: u64, member: Address) -> Option<Commitment> {
        load_commit(&env, pool_id, &member)
    }

    pub fn members(env: Env, pool_id: u64, start: u32, limit: u32) -> Vec<Address> {
        let pool = load_pool(&env, pool_id);
        let mut out = Vec::new(&env);
        let end = start.saturating_add(limit).min(pool.listed_members);
        for i in start..end {
            if let Some(m) = member_at(&env, pool_id, i) {
                out.push_back(m);
            }
        }
        out
    }

    pub fn current_tier(env: Env, pool_id: u64) -> (u32, i128) {
        let pool = load_pool(&env, pool_id);
        let idx = tier_index(&pool.terms.tiers, pool.total_units).unwrap_or(0);
        let price = pool.terms.tiers.get(idx).map(|t| t.unit_price).unwrap_or(0);
        (idx, price)
    }

    pub fn quote_commit(env: Env, pool_id: u64, units: u32) -> i128 {
        let pool = load_pool(&env, pool_id);
        let price = ck(&env, ceiling_price(&pool.terms.tiers, pool.total_units));
        ck(&env, (units as i128).checked_mul(price))
    }

    /// (supplier, platform, organizer, refunds) if the pool settled now.
    pub fn payout_preview(env: Env, pool_id: u64) -> (i128, i128, i128, i128) {
        let pool = load_pool(&env, pool_id);
        let units = if pool.state == PoolState::Delivered || pool.state == PoolState::Settled {
            pool.received_units
        } else {
            pool.total_units
        };
        let price = if pool.final_price > 0 {
            pool.final_price
        } else {
            ck(
                &env,
                final_unit_price(&pool.terms.tiers, pool.total_units.max(pool.terms.moq)),
            )
        };
        let gross = ck(&env, (units as i128).checked_mul(price));
        let (net, plat, org) = split_payable(&env, &pool, gross);
        (net, plat, org, pool.total_paid - gross)
    }

    pub fn allocated_units_of(env: Env, pool_id: u64, member: Address) -> u32 {
        let pool = load_pool(&env, pool_id);
        if !matches!(pool.state, PoolState::Delivered | PoolState::Settled) {
            return 0;
        }
        load_commit(&env, pool_id, &member)
            .map(|c| alloc_units(&pool, &c))
            .unwrap_or(0)
    }

    pub fn confirm_deadline(env: Env, pool_id: u64) -> u64 {
        let params = cfg(&env).get_params();
        confirm_deadline_of(&params, &load_pool(&env, pool_id))
    }

    pub fn refundable_of(env: Env, pool_id: u64, member: Address) -> i128 {
        let pool = load_pool(&env, pool_id);
        match load_commit(&env, pool_id, &member) {
            Some(c) => (refund_amount(&env, &pool, &c) - c.refund_claimed).max(0),
            None => 0,
        }
    }
}

#[cfg(test)]
mod test;
