//! Regression tests for fund-safety findings from the November 2026 internal review (brief section 15, M6).
use scenarios::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::Address;
use sp_common::types::{DisputeReason, Outcome, PoolState, Role, Status};

const PRICE: i128 = USDC * 8 / 10;

fn delivered() -> (Ctx, u64, [soroban_sdk::Address; 4]) {
    let c = setup();
    let pool = c.pool(0);
    let t = c.fill_standard(pool);
    c.deliver(pool, 420);
    (c, pool, t)
}

/// Finding 1: one member could open many disputes over the same allocated units. Each one froze more of the
/// supplier's payable and each paid out on RefundMember or SLA timeout, so a member with 60 units could drain
/// the whole pool's payout when the arbiter missed the SLA.
#[test]
fn a_member_cannot_dispute_the_same_pool_twice() {
    let (c, pool, t) = delivered();
    c.dis
        .open(&t[0], &pool, &DisputeReason::Damaged, &60, &h(&c.env, 3));
    let again = c
        .dis
        .try_open(&t[0], &pool, &DisputeReason::Short, &1, &h(&c.env, 3));
    assert!(
        again.is_err(),
        "second dispute by the same opener must be refused"
    );
    assert_eq!(c.gb.pool(&pool).frozen_amount, 60 * PRICE);
}

#[test]
fn a_resolved_dispute_cannot_be_reopened_for_more_money() {
    let (c, pool, t) = delivered();
    let id = c
        .dis
        .open(&t[0], &pool, &DisputeReason::Damaged, &60, &h(&c.env, 3));
    c.dis
        .resolve(&c.arbiter, &id, &Outcome::RefundMember(60), &h(&c.env, 4));
    assert!(c
        .dis
        .try_open(&t[0], &pool, &DisputeReason::Damaged, &60, &h(&c.env, 3))
        .is_err());
}

#[test]
fn sla_timeouts_cannot_pay_a_member_more_than_their_allocation() {
    let (c, pool, t) = delivered();
    // The attack: open the same 60-unit claim repeatedly, then let every dispute time out.
    let mut ids = soroban_sdk::Vec::<u64>::new(&c.env);
    for _ in 0..4 {
        if let Ok(Ok(id)) =
            c.dis
                .try_open(&t[0], &pool, &DisputeReason::Damaged, &60, &h(&c.env, 3))
        {
            ids.push_back(id);
        }
    }
    c.advance(6 * DAY);
    for id in ids.iter() {
        c.dis.timeout(&id);
    }
    c.gb.settle(&pool);
    let before = c.bal(&t[0]);
    c.gb.claim_refund(&t[0], &pool);
    // Paid 60 at the tier-1 ceiling (1.0); owes nothing for the 60 disputed units: refund is exactly what was paid.
    assert_eq!(c.bal(&t[0]) - before, 60 * USDC);
}

#[test]
fn different_parties_can_each_open_one_dispute() {
    let (c, pool, t) = delivered();
    c.mint.mint(&c.organizer, &(10 * USDC)); // the organizer pays a deposit too
    c.dis
        .open(&t[0], &pool, &DisputeReason::Damaged, &10, &h(&c.env, 3));
    c.dis
        .open(&t[1], &pool, &DisputeReason::Damaged, &10, &h(&c.env, 3));
    c.dis.open(
        &c.organizer,
        &pool,
        &DisputeReason::Short,
        &10,
        &h(&c.env, 3),
    );
    assert_eq!(c.dis.open_disputes_of(&pool).len(), 3);
}

/// Finding 2: a dispute resolved after settlement credits the member, but push_refunds had already walked every
/// member, so the keeper never paid the credit; only a manual claim could.
#[test]
fn credits_from_disputes_resolved_after_settlement_are_pushed() {
    let (c, pool, t) = delivered();
    let id = c
        .dis
        .open(&t[0], &pool, &DisputeReason::Damaged, &10, &h(&c.env, 3));
    c.advance(2 * DAY);
    c.gb.settle(&pool);
    c.gb.push_refunds(&pool, &50); // pays the tier refunds; cursor reaches the end
    let before = c.bal(&t[0]);
    c.dis
        .resolve(&c.arbiter, &id, &Outcome::RefundMember(10), &h(&c.env, 4));
    c.gb.push_refunds(&pool, &50);
    assert_eq!(c.bal(&t[0]) - before, 10 * PRICE + USDC / 2); // credit + deposit returned
    assert_eq!(c.gb.refundable_of(&pool, &t[0]), 0);
    assert_eq!(c.gb.pool(&pool).state, PoolState::Settled);
}

/// Brief 5.9: a supplier is suspended after two failures within 90 days.
#[test]
fn two_supplier_failures_within_90_days_suspend_the_supplier() {
    let c = setup();
    let p1 = c.pool(0);
    c.fill_standard(p1);
    c.gb.reject(&c.supplier, &p1);
    assert_eq!(c.reg.status(&c.supplier, &Role::Supplier), Status::Verified);

    c.advance(30 * DAY);
    let p2 = c.pool(0);
    c.fill_standard(p2);
    c.advance(2 * DAY); // past the 24h accept window
    c.gb.fail_accept(&p2);
    assert_eq!(
        c.reg.status(&c.supplier, &Role::Supplier),
        Status::Suspended
    );
    assert!(c
        .gb
        .try_create_pool(
            &c.organizer,
            &c.terms(),
            &h(&c.env, 7),
            &0,
            &None,
            &(c.now() + 3 * DAY)
        )
        .is_err());
    assert_eq!(c.rep.stats(&c.supplier, &Role::Supplier).pools_failed, 2);
}

#[test]
fn failures_more_than_90_days_apart_do_not_suspend() {
    let c = setup();
    let p1 = c.pool(0);
    c.fill_standard(p1);
    c.gb.reject(&c.supplier, &p1);
    c.advance(100 * DAY);
    let p2 = c.pool(0);
    c.fill_standard(p2);
    c.gb.reject(&c.supplier, &p2);
    assert_eq!(c.reg.status(&c.supplier, &Role::Supplier), Status::Verified);
}

// ── Issue #148: revoked attestor ──────────────────────────────────────────────

/// A disabled attestor must not be able to write new attestations, but any
/// records it wrote before revocation must remain intact (see docs/threat-model.md).
#[test]
fn revoked_attestor_cannot_attest_but_existing_records_remain() {
    let c = setup();

    // 1. Attestor writes a Verified record for a fresh user.
    let user = c.trader();
    assert_eq!(c.reg.status(&user, &Role::Trader), Status::Verified);

    // 2. Disable the attestor in config.
    c.cfg.set_attestor(&c.attestor, &false);
    assert!(!c.cfg.is_attestor(&c.attestor));

    // 3. The existing attestation must still be in place — revocation is
    //    not retroactive.
    assert_eq!(
        c.reg.status(&user, &Role::Trader),
        Status::Verified,
        "existing Verified record must survive attestor revocation"
    );

    // 4. The disabled attestor must not be able to attest a new user.
    let new_user = Address::generate(&c.env);
    c.reg
        .register(&new_user, &Role::Trader, &h(&c.env, 1), &None);
    let result = c
        .reg
        .try_attest(&c.attestor, &new_user, &Role::Trader, &h(&c.env, 2), &1);
    assert!(
        result.is_err(),
        "disabled attestor must be refused on a new attest call"
    );

    // 5. The new user's record must remain Registered (not Verified).
    assert_eq!(
        c.reg.status(&new_user, &Role::Trader),
        Status::Registered,
        "new user must not be Verified after a failed attest from a revoked attestor"
    );
}
