//! Read paths and guard rails not exercised by the flow tests.
use scenarios::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::Address;
use sp_common::types::PoolState;

#[test]
fn views_report_consistent_figures() {
    let c = setup();
    let pool = c.pool(100);
    assert_eq!(c.gb.quote_commit(&pool, &10), 10 * USDC);
    let [a, b, cc, d] = c.fill_standard(pool);
    let listed = c.gb.members(&pool, &0, &10);
    assert_eq!(listed.len(), 4);
    assert_eq!(c.gb.members(&pool, &2, &1).len(), 1);
    assert_eq!(c.gb.members(&pool, &10, &5).len(), 0);
    assert!(
        listed.contains(&a) && listed.contains(&b) && listed.contains(&cc) && listed.contains(&d)
    );

    let (net, plat, org, refunds) = c.gb.payout_preview(&pool);
    let gross = 336 * USDC;
    assert_eq!(net + plat + org, gross);
    assert_eq!(refunds, 400 * USDC - gross);

    c.deliver(pool, 420);
    let (net2, plat2, org2, _) = c.gb.payout_preview(&pool);
    assert_eq!((net, plat, org), (net2, plat2, org2));
    assert!(c.gb.confirm_deadline(&pool) > c.now());
    assert_eq!(c.gb.refundable_of(&pool, &a), 0);
    assert_eq!(c.gb.refundable_of(&pool, &Address::generate(&c.env)), 0);
    for m in [&a, &b, &cc, &d] {
        c.gb.confirm_pickup(m, &pool);
    }
    c.gb.settle(&pool);
    assert_eq!(c.gb.refundable_of(&pool, &a), 12 * USDC);
    c.gb.claim_refund(&a, &pool);
    assert_eq!(c.gb.refundable_of(&pool, &a), 0);
}

#[test]
fn payout_preview_before_close_uses_moq_floor() {
    let c = setup();
    let pool = c.pool(0);
    let t = c.trader();
    c.gb.commit(&t, &pool, &10);
    let (net, plat, org, refunds) = c.gb.payout_preview(&pool);
    assert_eq!(net + plat + org, 10 * USDC);
    assert_eq!(refunds, 0);
}

#[test]
fn sweep_dust_requires_a_final_pool_and_is_idempotent() {
    let c = setup();
    let pool = c.pool(0);
    assert!(c.gb.try_sweep_dust(&pool).is_err());
    let t = c.fill_standard(pool);
    assert!(c.gb.try_sweep_dust(&pool).is_err());
    c.deliver(pool, 420);
    for m in &t {
        c.gb.confirm_pickup(m, &pool);
    }
    c.gb.settle(&pool);
    for m in &t {
        c.gb.claim_refund(m, &pool);
    }
    assert_eq!(c.gb.sweep_dust(&pool), 0);
    assert_eq!(c.gb.pool(&pool).state, PoolState::Settled);
}

#[test]
fn state_guards_reject_out_of_order_calls() {
    let c = setup();
    let pool = c.pool(0);
    let stranger = c.trader();
    // nothing committed yet
    assert!(c.gb.try_close_early(&c.organizer, &pool).is_err());
    assert!(c.gb.try_withdraw_commitment(&stranger, &pool).is_err());
    assert!(c.gb.try_member_confirm_delivery(&stranger, &pool).is_err());
    assert!(c.gb.try_confirm_pickup(&stranger, &pool).is_err());
    assert!(c.gb.try_settle(&pool).is_err());
    assert!(c.gb.try_claim_refund(&stranger, &pool).is_err());
    assert!(c.gb.try_push_refunds(&pool, &5).is_err());
    assert!(c.gb.try_accept(&c.supplier, &pool, &0).is_err());
    assert!(c.gb.try_reject(&c.supplier, &pool).is_err());
    assert!(c.gb.try_fail_accept(&pool).is_err());
    assert!(c.gb.try_dispatch(&c.supplier, &pool, &None).is_err());
    assert!(c.gb.try_fail_delivery(&pool).is_err());
    assert!(c
        .gb
        .try_confirm_delivery(&c.organizer, &pool, &1, &h(&c.env, 5))
        .is_err());
    assert!(c.gb.try_allocate_shortfall(&pool, &5).is_err());
    // below MOQ the organizer cannot close early
    c.gb.commit(&stranger, &pool, &10);
    assert!(c.gb.try_close_early(&c.organizer, &pool).is_err());
    // wrong callers on a filled pool
    c.gb.commit(&c.trader(), &pool, &100);
    c.gb.close_early(&c.organizer, &pool);
    assert!(c.gb.try_close_early(&c.organizer, &pool).is_err());
    assert!(c.gb.try_accept(&stranger, &pool, &0).is_err());
    assert!(c.gb.try_reject(&stranger, &pool).is_err());
    assert!(c.gb.try_commit(&stranger, &pool, &1).is_err());
    assert!(c.gb.try_cancel_pool(&stranger, &pool).is_err());
    c.gb.accept(&c.supplier, &pool, &0);
    assert!(c.gb.try_dispatch(&stranger, &pool, &None).is_err());
    c.gb.dispatch(&c.supplier, &pool, &None);
    assert!(c
        .gb
        .try_confirm_delivery(&stranger, &pool, &110, &h(&c.env, 5))
        .is_err());
    assert!(c
        .gb
        .try_confirm_delivery(&c.organizer, &pool, &9999, &h(&c.env, 5))
        .is_err());
    assert!(c
        .gb
        .try_confirm_pickup(&Address::generate(&c.env), &pool)
        .is_err());
}

#[test]
fn commit_limits_are_enforced() {
    let c = setup();
    let pool = c.pool(0);
    let t = c.trader();
    assert!(c.gb.try_commit(&t, &pool, &251).is_err()); // over per-member share
    assert!(c.gb.try_increase(&t, &pool, &1).is_err()); // never committed
    c.gb.commit(&t, &pool, &250);
    assert!(c.gb.try_increase(&t, &pool, &1).is_err());
    let u = c.trader();
    c.gb.commit(&u, &pool, &250);
    let v = c.trader();
    assert!(c.gb.try_commit(&v, &pool, &1).is_err()); // pool max 500
}

#[test]
fn double_pickup_confirmation_is_rejected() {
    let c = setup();
    let pool = c.pool(0);
    let t = c.fill_standard(pool);
    c.deliver(pool, 420);
    c.gb.confirm_pickup(&t[0], &pool);
    assert!(c.gb.try_confirm_pickup(&t[0], &pool).is_err());
}
