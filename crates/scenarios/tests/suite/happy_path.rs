use scenarios::*;
use sp_common::types::PoolState;

#[test]
fn full_flow_with_tier_refunds_leaves_no_dust() {
    let c = setup();
    let pool = c.pool(100);
    let [a, b, cc, d] = c.fill_standard(pool);

    // Members paid the ceiling price at commit time.
    assert_eq!(c.gb.commitment(&pool, &a).unwrap().paid, 60 * USDC);
    assert_eq!(c.gb.commitment(&pool, &cc).unwrap().paid, 100 * USDC);
    assert_eq!(c.gb.commitment(&pool, &d).unwrap().paid, 180 * USDC);
    let p = c.gb.pool(&pool);
    assert_eq!(p.state, PoolState::Filled);
    assert_eq!(p.total_units, 420);
    assert_eq!(p.final_price, USDC * 8 / 10);
    assert_eq!(c.bal(&c.gb.address), 400 * USDC);

    c.deliver(pool, 420);
    for m in [&a, &b, &cc, &d] {
        c.gb.confirm_pickup(m, &pool);
    }
    // 100% pickup weight releases early.
    c.gb.settle(&pool);

    let gross = 336 * USDC; // 420 * 0.8
    let plat = gross * 150 / 10_000;
    let org = gross * 100 / 10_000;
    assert_eq!(c.bal(&c.treasury), plat);
    assert_eq!(c.bal(&c.organizer), org);
    assert_eq!(c.bal(&c.supplier), 1_000 * USDC + gross - plat - org);

    // Tier-difference refunds: 12, 12, 20, 20 USDC.
    assert_eq!(c.gb.claim_refund(&a, &pool), 12 * USDC);
    assert_eq!(c.gb.claim_refund(&b, &pool), 12 * USDC);
    assert_eq!(c.gb.claim_refund(&cc, &pool), 20 * USDC);
    assert_eq!(c.gb.claim_refund(&d, &pool), 20 * USDC);
    assert_eq!(c.bal(&c.gb.address), 0);
    assert_eq!(c.gb.pool(&pool).state, PoolState::Settled);
    assert!(c.gb.try_claim_refund(&a, &pool).is_err());
}

#[test]
fn settles_after_window_without_pickups() {
    let c = setup();
    let pool = c.pool(0);
    c.fill_standard(pool);
    c.deliver(pool, 420);
    assert!(c.gb.try_settle(&pool).is_err());
    c.advance(48 * 3600);
    c.gb.settle(&pool);
    assert_eq!(c.gb.pool(&pool).state, PoolState::Settled);
}

#[test]
fn push_refunds_pays_everyone_in_batches() {
    let c = setup();
    let pool = c.pool(0);
    let t = c.fill_standard(pool);
    c.deliver(pool, 420);
    c.advance(48 * 3600);
    c.gb.settle(&pool);
    let before: i128 = t.iter().map(|m| c.bal(m)).sum();
    assert_eq!(c.gb.push_refunds(&pool, &3), 3);
    assert_eq!(c.gb.push_refunds(&pool, &3), 1);
    assert_eq!(c.gb.push_refunds(&pool, &3), 0);
    let after: i128 = t.iter().map(|m| c.bal(m)).sum();
    assert_eq!(after - before, 64 * USDC);
    assert_eq!(c.gb.sweep_dust(&pool), 0);
}

#[test]
fn expired_pool_refunds_in_full() {
    let c = setup();
    let pool = c.pool(0);
    let a = c.trader();
    let b = c.trader();
    c.gb.commit(&a, &pool, &40);
    c.gb.commit(&b, &pool, &30);
    assert!(c.gb.try_close(&pool).is_err()); // before the deadline
    c.advance(3 * DAY);
    c.gb.close(&pool);
    assert_eq!(c.gb.pool(&pool).state, PoolState::Expired);
    assert_eq!(c.gb.claim_refund(&a, &pool), 40 * USDC);
    assert_eq!(c.gb.push_refunds(&pool, &10), 2);
    assert_eq!(c.bal(&b), 1_000 * USDC);
    assert_eq!(c.bal(&c.gb.address), 0);
}

#[test]
fn close_early_and_auto_fill_at_max() {
    let c = setup();
    let pool = c.pool(0);
    let a = c.trader();
    c.gb.commit(&a, &pool, &100);
    assert!(c.gb.try_close_early(&c.organizer, &pool).is_ok());
    assert_eq!(c.gb.pool(&pool).state, PoolState::Filled);

    let pool2 = c.pool(0);
    let t = [c.trader(), c.trader()];
    c.gb.commit(&t[0], &pool2, &250);
    c.gb.commit(&t[1], &pool2, &250);
    assert_eq!(c.gb.pool(&pool2).state, PoolState::Filled);
}
