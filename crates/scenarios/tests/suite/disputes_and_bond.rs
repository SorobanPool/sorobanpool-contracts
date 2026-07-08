use scenarios::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::Address;
use sp_common::types::{DisputeReason, Outcome, PoolState, Role};

const PRICE: i128 = USDC * 8 / 10;
const DEPOSIT: i128 = USDC / 2; // 1% of the claim is below the 0.5 USDC minimum

/// Standard pool delivered in full; returns members. Dispute window is open.
fn delivered() -> (Ctx, u64, [Address; 4]) {
    let c = setup();
    let pool = c.pool(0);
    let t = c.fill_standard(pool);
    c.deliver(pool, 420);
    (c, pool, t)
}

fn open(c: &Ctx, pool: u64, who: &Address, units: u32) -> u64 {
    c.dis
        .open(who, &pool, &DisputeReason::Damaged, &units, &h(&c.env, 3))
}

fn drain(c: &Ctx, pool: u64) {
    c.gb.push_refunds(&pool, &50);
    c.gb.sweep_dust(&pool);
}

#[test]
fn open_freezes_amount_and_takes_deposit() {
    let (c, pool, t) = delivered();
    let before = c.bal(&t[0]);
    let id = open(&c, pool, &t[0], 10);
    assert_eq!(c.gb.pool(&pool).frozen_amount, 10 * PRICE);
    assert_eq!(before - c.bal(&t[0]), DEPOSIT);
    assert_eq!(c.dis.open_disputes_of(&pool).len(), 1);
    assert_eq!(c.dis.dispute(&id).claimed_amount, 10 * PRICE);
    assert_eq!(c.rep.stats(&t[0], &Role::Trader).disputes_opened, 1);
}

#[test]
fn only_parties_with_valid_claims_can_open() {
    let (c, pool, t) = delivered();
    let stranger = c.trader();
    assert!(c
        .dis
        .try_open(&stranger, &pool, &DisputeReason::Short, &1, &h(&c.env, 3))
        .is_err());
    assert!(c
        .dis
        .try_open(&t[0], &pool, &DisputeReason::Short, &0, &h(&c.env, 3))
        .is_err());
    assert!(c
        .dis
        .try_open(&t[0], &pool, &DisputeReason::Short, &61, &h(&c.env, 3))
        .is_err()); // owns 60
    c.advance(48 * 3600);
    assert!(c
        .dis
        .try_open(&t[0], &pool, &DisputeReason::Short, &1, &h(&c.env, 3))
        .is_err()); // window closed
}

#[test]
fn release_to_supplier_pays_supplier_and_forfeits_deposit() {
    let (c, pool, t) = delivered();
    let id = open(&c, pool, &t[0], 10);
    c.gb.confirm_pickup(&t[1], &pool);
    c.advance(48 * 3600);
    c.gb.settle(&pool); // frozen part held back
    let mid = c.bal(&c.supplier);
    c.dis
        .resolve(&c.arbiter, &id, &Outcome::ReleaseToSupplier, &h(&c.env, 4));
    let released = 10 * PRICE;
    let fees = released * 150 / 10_000;
    assert_eq!(c.bal(&c.supplier) - mid, released - fees + DEPOSIT);
    assert_eq!(c.rep.stats(&t[0], &Role::Trader).disputes_lost, 1);
    for m in &t {
        let _ = c.gb.try_claim_refund(m, &pool);
    }
    drain(&c, pool);
    assert_eq!(c.bal(&c.gb.address), 0);
}

#[test]
fn refund_member_credits_member_and_returns_deposit() {
    let (c, pool, t) = delivered();
    let id = open(&c, pool, &t[0], 10);
    let paid_before = c.bal(&t[0]);
    c.dis
        .resolve(&c.arbiter, &id, &Outcome::RefundMember(6), &h(&c.env, 4));
    assert_eq!(c.bal(&t[0]) - paid_before, DEPOSIT); // deposit back
    c.advance(48 * 3600);
    c.gb.settle(&pool);
    // tier refund 12 USDC + 6 units * 0.8
    let expected = 12 * USDC + 6 * PRICE;
    assert_eq!(c.gb.claim_refund(&t[0], &pool), expected);
    for m in &t[1..] {
        c.gb.claim_refund(m, &pool);
    }
    assert_eq!(c.rep.stats(&t[0], &Role::Trader).disputes_won, 1);
    drain(&c, pool);
    assert_eq!(c.bal(&c.gb.address), 0);
}

#[test]
fn split_divides_the_claim() {
    let (c, pool, t) = delivered();
    let id = open(&c, pool, &t[2], 20); // 16 USDC claimed
    c.dis
        .resolve(&c.arbiter, &id, &Outcome::Split(2500), &h(&c.env, 4));
    c.advance(48 * 3600);
    c.gb.settle(&pool);
    assert_eq!(c.gb.claim_refund(&t[2], &pool), 20 * USDC + 4 * USDC); // tier refund + 25% of 16
    for m in [&t[0], &t[1], &t[3]] {
        c.gb.claim_refund(m, &pool);
    }
    drain(&c, pool);
    assert_eq!(c.bal(&c.gb.address), 0);
}

#[test]
fn refund_pool_is_shared_by_units() {
    let (c, pool, t) = delivered();
    let id = open(&c, pool, &t[3], 100); // 80 USDC claimed
    c.dis
        .resolve(&c.arbiter, &id, &Outcome::RefundPool, &h(&c.env, 4));
    c.advance(48 * 3600);
    c.gb.settle(&pool);
    let mut total = 0;
    for m in &t {
        total += c.gb.claim_refund(m, &pool);
    }
    assert!(total >= 64 * USDC + 80 * USDC - 4); // floors may leave a few stroops
    drain(&c, pool);
    assert_eq!(c.bal(&c.gb.address), 0);
}

#[test]
fn timeout_refunds_the_disputer_when_arbiter_is_silent() {
    let (c, pool, t) = delivered();
    let id = open(&c, pool, &t[0], 10);
    assert!(c.dis.try_timeout(&id).is_err()); // SLA not reached
    c.advance(5 * DAY + 1);
    assert!(c
        .dis
        .try_resolve(&c.arbiter, &id, &Outcome::ReleaseToSupplier, &h(&c.env, 4))
        .is_err());
    c.dis.timeout(&id);
    assert!(c.dis.try_timeout(&id).is_err());
    c.gb.settle(&pool);
    assert_eq!(c.gb.claim_refund(&t[0], &pool), 12 * USDC + 10 * PRICE);
    for m in &t[1..] {
        c.gb.claim_refund(m, &pool);
    }
    drain(&c, pool);
    assert_eq!(c.bal(&c.gb.address), 0);
}

#[test]
fn arbiter_conflicts_and_permissions() {
    let (c, pool, t) = delivered();
    let id = open(&c, pool, &t[0], 5);
    let nobody = c.trader();
    assert!(c
        .dis
        .try_resolve(&nobody, &id, &Outcome::ReleaseToSupplier, &h(&c.env, 4))
        .is_err());
    c.cfg.set_arbiter(&t[1], &true); // a pool member is also an arbiter
    assert!(c
        .dis
        .try_resolve(&t[1], &id, &Outcome::ReleaseToSupplier, &h(&c.env, 4))
        .is_err());
    assert!(c
        .dis
        .try_resolve(&c.arbiter, &id, &Outcome::RefundMember(6), &h(&c.env, 4))
        .is_err()); // more than claimed
    assert!(c
        .dis
        .try_resolve(&c.arbiter, &id, &Outcome::Split(10_001), &h(&c.env, 4))
        .is_err());
    c.dis
        .resolve(&c.arbiter, &id, &Outcome::ReleaseToSupplier, &h(&c.env, 4));
    assert!(c
        .dis
        .try_resolve(&c.arbiter, &id, &Outcome::ReleaseToSupplier, &h(&c.env, 4))
        .is_err());
}

#[test]
fn evidence_from_parties_only() {
    let (c, pool, t) = delivered();
    let id = open(&c, pool, &t[0], 5);
    c.dis.add_evidence(&c.supplier, &id, &h(&c.env, 6));
    c.dis.add_evidence(&c.organizer, &id, &h(&c.env, 7));
    c.dis.add_evidence(&t[0], &id, &h(&c.env, 8));
    assert!(c.dis.try_add_evidence(&t[1], &id, &h(&c.env, 9)).is_err());
    assert_eq!(c.dis.dispute(&id).evidence.len(), 4);
}

#[test]
fn supplier_can_force_arbitration_when_nobody_confirms_delivery() {
    let c = setup();
    let pool = c.pool(0);
    let t = c.fill_standard(pool);
    c.gb.accept(&c.supplier, &pool, &0);
    c.gb.dispatch(&c.supplier, &pool, &None);
    assert!(c
        .dis
        .try_open(
            &c.supplier,
            &pool,
            &DisputeReason::NotDelivered,
            &420,
            &h(&c.env, 3)
        )
        .is_err());
    c.advance(5 * DAY + 2 * DAY + 1); // organizer confirmation window over (fail_delivery not yet called)
    let id = c.dis.open(
        &c.supplier,
        &pool,
        &DisputeReason::NotDelivered,
        &420,
        &h(&c.env, 3),
    );
    assert_eq!(c.gb.pool(&pool).state, PoolState::Delivered);
    c.dis
        .resolve(&c.arbiter, &id, &Outcome::ReleaseToSupplier, &h(&c.env, 4));
    c.advance(48 * 3600);
    c.gb.settle(&pool);
    let gross = 420 * PRICE;
    let fees = gross * 150 / 10_000;
    assert_eq!(c.bal(&c.supplier), 1_000 * USDC + gross - fees); // deposit came from, and returned to, the supplier
    for m in &t {
        c.gb.claim_refund(m, &pool);
    }
    drain(&c, pool);
    assert_eq!(c.bal(&c.gb.address), 0);
}

// ---- supplier advance and bond ----

fn enable_advance(c: &Ctx) {
    let mut p = c.cfg.get_params();
    p.advance_enabled = true;
    p.supplier_advance_bp.set(0, 2000);
    c.cfg.set_params(&p);
}

#[test]
fn advance_is_off_by_default_and_needs_bond() {
    let c = setup();
    let pool = c.pool(0);
    c.fill_standard(pool);
    assert!(c.gb.try_accept(&c.supplier, &pool, &1000).is_err()); // disabled
    enable_advance(&c);
    assert!(c.gb.try_accept(&c.supplier, &pool, &1000).is_err()); // no bond posted
    assert!(c.gb.try_accept(&c.supplier, &pool, &2500).is_err()); // above tier limit
    c.bond.deposit(&c.supplier, &(100 * USDC));
    c.gb.accept(&c.supplier, &pool, &1000);
    let advance = 420 * PRICE / 10;
    assert_eq!(c.gb.pool(&pool).advance_paid, advance);
    assert_eq!(c.bond.bond_of(&c.supplier), (100 * USDC, advance));
}

#[test]
fn advance_is_netted_at_settlement_and_bond_released() {
    let c = setup();
    enable_advance(&c);
    c.bond.deposit(&c.supplier, &(100 * USDC));
    let pool = c.pool(0);
    let t = c.fill_standard(pool);
    c.gb.accept(&c.supplier, &pool, &1000);
    c.gb.dispatch(&c.supplier, &pool, &None);
    c.gb.confirm_delivery(&c.organizer, &pool, &420, &h(&c.env, 5));
    c.advance(48 * 3600);
    c.gb.settle(&pool);
    let gross = 420 * PRICE;
    let fees = gross * 150 / 10_000;
    assert_eq!(c.bal(&c.supplier), 1_000 * USDC - 100 * USDC + gross - fees);
    assert_eq!(c.bond.bond_of(&c.supplier), (100 * USDC, 0));
    for m in &t {
        c.gb.claim_refund(m, &pool);
    }
    assert_eq!(c.bal(&c.gb.address), 0);
}

#[test]
fn failed_delivery_slashes_bond_so_members_are_made_whole() {
    let c = setup();
    enable_advance(&c);
    c.bond.deposit(&c.supplier, &(100 * USDC));
    let pool = c.pool(0);
    let t = c.fill_standard(pool);
    c.gb.accept(&c.supplier, &pool, &1000);
    c.gb.dispatch(&c.supplier, &pool, &None);
    c.advance(5 * DAY + 2 * DAY + 1);
    c.gb.fail_delivery(&pool);
    let advance = 420 * PRICE / 10;
    assert_eq!(c.bond.bond_of(&c.supplier), (100 * USDC - advance, 0));
    let mut total = 0;
    for m in &t {
        total += c.gb.claim_refund(m, &pool);
    }
    assert_eq!(total, 400 * USDC); // everyone gets everything back
    assert_eq!(c.bal(&c.gb.address), 0);
}

#[test]
fn nothing_is_dust_for_random_sized_commitments() {
    let c = setup();
    let pool = c.pool(0);
    let sizes = [7u32, 13, 29, 41, 53, 67, 71, 89];
    let members: Vec<Address> = sizes.iter().map(|_| c.trader()).collect();
    for (m, s) in members.iter().zip(sizes) {
        c.gb.commit(m, &pool, &s);
    }
    c.advance(3 * DAY);
    c.gb.close(&pool);
    c.deliver(pool, 301); // short of 370
    c.gb.allocate_shortfall(&pool, &20);
    c.advance(48 * 3600);
    c.gb.settle(&pool);
    for m in &members {
        let _ = c.gb.try_claim_refund(m, &pool);
    }
    drain(&c, pool);
    assert_eq!(c.bal(&c.gb.address), 0);
    let _ = Address::generate(&c.env);
}
