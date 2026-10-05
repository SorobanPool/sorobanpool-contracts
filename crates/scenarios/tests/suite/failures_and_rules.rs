use scenarios::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{symbol_short, Address};
use sp_common::types::{PoolState, Role};

#[test]
fn shortfall_is_allocated_pro_rata_and_supplier_paid_for_received_only() {
    let c = setup();
    let pool = c.pool(0);
    let t = c.fill_standard(pool);
    c.deliver(pool, 400); // 20 short of 420
    assert!(c.gb.try_settle(&pool).is_err()); // allocation pending
    c.advance(48 * 3600);
    assert!(c.gb.try_settle(&pool).is_err());
    assert!(c.gb.allocate_shortfall(&pool, &10));

    let alloc: u32 = t.iter().map(|m| c.gb.allocated_units_of(&pool, m)).sum();
    assert_eq!(alloc, 400);
    c.gb.settle(&pool);
    let gross = 400 * USDC * 8 / 10;
    let plat = gross * 150 / 10_000;
    assert_eq!(c.bal(&c.supplier), 1_000 * USDC + gross - plat);

    for m in &t {
        c.gb.claim_refund(m, &pool);
    }
    assert_eq!(c.bal(&c.gb.address), 0);
    assert_eq!(
        c.rep.stats(&c.supplier, &Role::Supplier).short_deliveries,
        1
    );
}

#[test]
fn allocation_can_be_paged() {
    let c = setup();
    let pool = c.pool(0);
    c.fill_standard(pool);
    c.deliver(pool, 333);
    assert!(!c.gb.allocate_shortfall(&pool, &2));
    assert!(c.gb.allocate_shortfall(&pool, &2));
}

#[test]
fn withdrawal_lock_and_tier_protection() {
    let c = setup();
    let pool = c.pool(0);
    let a = c.trader();
    let b = c.trader();
    c.gb.commit(&a, &pool, &150);
    c.gb.commit(&b, &pool, &100); // total 250 -> tier 2 (0.9)
                                  // b leaving would drop 250 -> 150 (tier 1): blocked, so a later member's lower price stays covered.
    assert_eq!(
        c.gb.try_withdraw_commitment(&b, &pool).err(),
        Some(Ok(soroban_sdk::Error::from_contract_error(
            group_buy::Error::WouldLowerTier as u32
        )))
    );
    let d = c.trader();
    c.gb.commit(&d, &pool, &10);
    // d leaving keeps the tier (250 -> 250? 260 -> 250 both tier 2): allowed and refunded in full.
    let before = c.bal(&d);
    c.gb.withdraw_commitment(&d, &pool);
    assert_eq!(c.bal(&d) - before, 9 * USDC);
    assert_eq!(c.gb.pool(&pool).total_units, 250);
    // after leaving, d may join again
    c.gb.commit(&d, &pool, &5);
    // inside the 2h lock before the deadline nobody can withdraw
    c.set_time(c.gb.pool(&pool).fill_deadline - 3600);
    assert!(c.gb.try_withdraw_commitment(&d, &pool).is_err());
}

#[test]
fn escrow_always_covers_final_bill_after_allowed_withdrawals() {
    let c = setup();
    let pool = c.pool(0);
    let m: Vec<Address> = (0..5).map(|_| c.trader()).collect();
    c.gb.commit(&m[0], &pool, &120);
    c.gb.commit(&m[1], &pool, &90);
    c.gb.commit(&m[2], &pool, &40); // 250 total
    let _ = c.gb.try_withdraw_commitment(&m[1], &pool); // lowers tier -> rejected
    c.gb.commit(&m[3], &pool, &160); // 410 -> tier 3
    let _ = c.gb.try_withdraw_commitment(&m[2], &pool); // 410 -> 370 lowers tier -> rejected
    c.advance(3 * DAY);
    c.gb.close(&pool);
    let p = c.gb.pool(&pool);
    assert!(p.escrow_balance >= p.total_units as i128 * p.final_price);
}

#[test]
fn fail_accept_after_window_refunds_everyone() {
    let c = setup();
    let pool = c.pool(0);
    let t = c.fill_standard(pool);
    assert!(c.gb.try_fail_accept(&pool).is_err());
    c.advance(DAY + 1);
    assert!(c.gb.try_accept(&c.supplier, &pool, &0).is_err());
    c.gb.fail_accept(&pool);
    assert_eq!(c.gb.pool(&pool).state, PoolState::Failed);
    for m in &t {
        c.gb.claim_refund(m, &pool);
    }
    assert_eq!(c.bal(&c.gb.address), 0);
    assert_eq!(c.rep.stats(&c.supplier, &Role::Supplier).pools_failed, 1);
}

#[test]
fn supplier_reject_refunds() {
    let c = setup();
    let pool = c.pool(0);
    c.fill_standard(pool);
    c.gb.reject(&c.supplier, &pool);
    assert_eq!(c.gb.pool(&pool).state, PoolState::Failed);
    assert_eq!(c.gb.push_refunds(&pool, &10), 4);
    assert_eq!(c.bal(&c.gb.address), 0);
}

#[test]
fn fail_delivery_after_deadline_refunds_everyone() {
    let c = setup();
    let pool = c.pool(0);
    c.fill_standard(pool);
    c.gb.accept(&c.supplier, &pool, &0);
    c.gb.dispatch(&c.supplier, &pool, &None);
    assert!(c.gb.try_fail_delivery(&pool).is_err());
    c.advance(5 * DAY + 2 * DAY + 1); // lead time + grace
    c.gb.fail_delivery(&pool);
    assert_eq!(c.gb.push_refunds(&pool, &10), 4);
    assert_eq!(c.bal(&c.gb.address), 0);
}

#[test]
fn collective_member_confirmation_when_organizer_is_silent() {
    let c = setup();
    let pool = c.pool(0);
    let t = c.fill_standard(pool);
    c.gb.accept(&c.supplier, &pool, &0);
    c.gb.dispatch(&c.supplier, &pool, &None);
    assert!(c.gb.try_member_confirm_delivery(&t[0], &pool).is_err()); // too early
    c.advance(DAY * 8);
    // delivery deadline has not been enforced yet, so members can confirm
    c.gb.member_confirm_delivery(&t[3], &pool); // 200 of 420 units: under 50%
    assert_eq!(c.gb.pool(&pool).state, PoolState::Dispatched);
    assert!(c.gb.try_member_confirm_delivery(&t[3], &pool).is_err()); // no double count
    c.gb.member_confirm_delivery(&t[2], &pool); // 300 of 420: delivered
    let p = c.gb.pool(&pool);
    assert_eq!(p.state, PoolState::Delivered);
    assert_eq!(p.received_units, 420);
}

#[test]
fn rules_and_limits() {
    let c = setup();
    let pool = c.pool(0);
    let a = c.trader();
    // supplier cannot join their own pool
    assert!(c.gb.try_commit(&c.supplier, &pool, &10).is_err());
    // unregistered users cannot commit
    let ghost = Address::generate(&c.env);
    c.mint.mint(&ghost, &(100 * USDC));
    assert!(c.gb.try_commit(&ghost, &pool, &10).is_err());
    // zero units, over per-member limit, over pool max
    assert!(c.gb.try_commit(&a, &pool, &0).is_err());
    assert!(c.gb.try_commit(&a, &pool, &251).is_err());
    c.gb.commit(&a, &pool, &250);
    // second commit must use increase
    assert!(c.gb.try_commit(&a, &pool, &1).is_err());
    assert!(c.gb.try_increase(&a, &pool, &1).is_err()); // per-member cap 250
                                                        // 50% of max pool per member unless alone
    let b = c.trader();
    c.gb.commit(&b, &pool, &100);
    let big = c.trader();
    assert!(c.gb.try_commit(&big, &pool, &251).is_err());
    // commit after deadline fails
    c.advance(3 * DAY);
    assert!(c.gb.try_commit(&big, &pool, &10).is_err());
}

#[test]
fn trader_tier_cap_applies_without_kyc() {
    let c = setup();
    let pool = c.pool(0);
    let t0 = Address::generate(&c.env);
    c.reg.register(&t0, &Role::Trader, &h(&c.env, 1), &None);
    c.mint.mint(&t0, &(1_000 * USDC));
    assert!(c.gb.try_commit(&t0, &pool, &51).is_err()); // T0 cap is 50 USDC
    c.gb.commit(&t0, &pool, &50);
}

#[test]
fn create_pool_validation() {
    let c = setup();
    let dl = c.now() + 3 * DAY;
    let mk = |terms, fee, dl| {
        c.gb.try_create_pool(&c.organizer, &terms, &h(&c.env, 7), &fee, &None, &dl)
    };
    assert!(mk(c.terms(), 0, dl).is_ok());
    assert!(mk(c.terms(), 101, dl).is_err()); // organizer fee above the max
    assert!(mk(c.terms(), 0, c.now()).is_err()); // deadline in the past
    assert!(mk(c.terms(), 0, c.now() + 8 * DAY).is_err()); // beyond max fill window
    let mut bad = c.terms();
    bad.category = symbol_short!("beer");
    assert!(mk(bad, 0, dl).is_err()); // category not allowed
    let mut bad = c.terms();
    bad.moq = 99;
    assert!(mk(bad, 0, dl).is_err()); // moq must equal the first tier
    let mut big = c.terms();
    big.max_units = 600;
    assert!(mk(big, 0, dl).is_err()); // value above O0 cap (500 USDC)
                                      // unverified organizer
    let rogue = Address::generate(&c.env);
    c.reg
        .register(&rogue, &Role::Organizer, &h(&c.env, 1), &None);
    assert!(c
        .gb
        .try_create_pool(&rogue, &c.terms(), &h(&c.env, 7), &0, &None, &dl)
        .is_err());
    // supplier cannot organize their own pool
    assert!(c
        .gb
        .try_create_pool(&c.supplier, &c.terms(), &h(&c.env, 7), &0, &None, &dl)
        .is_err());
}

#[test]
fn cluster_restriction() {
    let c = setup();
    let pool = c.gb.create_pool(
        &c.organizer,
        &c.terms(),
        &h(&c.env, 7),
        &0,
        &Some(symbol_short!("wuse")),
        &(c.now() + DAY),
    );
    let insider = Address::generate(&c.env);
    c.reg.register(
        &insider,
        &Role::Trader,
        &h(&c.env, 1),
        &Some(symbol_short!("wuse")),
    );
    c.mint.mint(&insider, &(100 * USDC));
    c.gb.commit(&insider, &pool, &10);
    let outsider = c.trader();
    assert!(c.gb.try_commit(&outsider, &pool, &10).is_err());
}

#[test]
fn pause_blocks_new_activity_but_never_refunds() {
    let c = setup();
    let pool = c.pool(0);
    let a = c.trader();
    c.gb.commit(&a, &pool, &10);
    c.cfg.pause(&c.admin, &symbol_short!("all"));
    assert!(c.gb.try_commit(&a, &pool, &1).is_err());
    assert!(c
        .gb
        .try_create_pool(
            &c.organizer,
            &c.terms(),
            &h(&c.env, 7),
            &0,
            &None,
            &(c.now() + DAY)
        )
        .is_err());
    c.advance(3 * DAY);
    c.gb.close(&pool); // expiry still works
    assert_eq!(c.gb.claim_refund(&a, &pool), 10 * USDC);
}

#[test]
fn cancel_rules() {
    let c = setup();
    let pool = c.pool(0);
    let a = c.trader();
    c.gb.commit(&a, &pool, &10);
    assert!(c.gb.try_cancel_pool(&c.organizer, &pool).is_err()); // has commitments
    let stranger = c.trader();
    assert!(c.gb.try_cancel_pool(&stranger, &pool).is_err());
    c.gb.cancel_pool(&c.admin, &pool);
    assert_eq!(c.gb.claim_refund(&a, &pool), 10 * USDC);
    let empty = c.pool(0);
    c.gb.cancel_pool(&c.organizer, &empty);
    assert_eq!(c.gb.pool(&empty).state, PoolState::Cancelled);
}

#[test]
fn settled_pool_updates_reputation() {
    let c = setup();
    let pool = c.pool(0);
    let t = c.fill_standard(pool);
    c.deliver(pool, 420);
    c.gb.confirm_pickup(&t[0], &pool);
    c.advance(48 * 3600);
    c.gb.settle(&pool);
    let s = c.rep.stats(&c.supplier, &Role::Supplier);
    assert_eq!((s.pools_settled, s.on_time, s.late), (1, 1, 0));
    assert_eq!(c.rep.stats(&c.organizer, &Role::Organizer).pools_settled, 1);
    assert_eq!(c.rep.stats(&t[0], &Role::Trader).pickups_confirmed, 1);
}
