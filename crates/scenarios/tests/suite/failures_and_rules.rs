use scenarios::*;
use soroban_sdk::testutils::{Address as _, Events as _};
use soroban_sdk::{symbol_short, Address, Symbol};
use sp_common::types::{caps, PoolState, Role, Tier};

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
    assert!(
        format!("{:?}", c.env.events().all()).contains("alloc_ok"),
        "alloc_ok event emitted when allocation finishes"
    );

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
fn shortfall_allocation_handles_zero_pages_and_retries_for_30_members() {
    let c = setup();
    let pool = c.pool(0);
    let members: Vec<Address> = (0..30).map(|_| c.trader()).collect();
    for member in &members {
        c.gb.commit(member, &pool, &10);
    }
    c.advance(3 * DAY);
    c.gb.close(&pool);
    c.deliver(pool, 250);

    let zero_page_error = c
        .gb
        .try_allocate_shortfall(&pool, &0)
        .err()
        .and_then(|r| r.ok())
        .map(|e| e.get_code());
    assert_eq!(
        zero_page_error,
        Some(group_buy::Error::ZeroPageSize as u32)
    );

    assert!(!c.gb.allocate_shortfall(&pool, &10));
    assert!(!c.gb.allocate_shortfall(&pool, &10));
    assert!(c.gb.allocate_shortfall(&pool, &10));

    let completed = c.gb.pool(&pool);
    assert_eq!(completed.alloc_cursor, 30);
    assert_eq!(completed.alloc_cum, 300);
    assert_eq!(
        members
            .iter()
            .map(|member| c.gb.allocated_units_of(&pool, member))
            .sum::<u32>(),
        250
    );
    assert!(c.gb.allocate_shortfall(&pool, &10));
    assert_eq!(c.gb.pool(&pool), completed);
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
fn commit_withdraw_re_commit_with_2h_lock() {
    let c = setup();
    let pool = c.pool(0);
    let params = c.cfg.get_params();
    let member = c.trader();

    // Start with a pool that has enough commitments to establish a tier
    // Add other members first to establish the pool
    let other1 = c.trader();
    let other2 = c.trader();
    c.gb.commit(&other1, &pool, &150); // 150 units
    c.gb.commit(&other2, &pool, &100); // 250 total - tier 2

    // Now our test member joins
    c.gb.commit(&member, &pool, &100); // 350 total - still tier 2
    let initial_paid = 100 * 9_000_000; // tier 2 price (0.90 USDC = 9M stroops)
    assert_eq!(c.gb.commitment(&pool, &member).unwrap().paid, initial_paid);

    let p = c.gb.pool(&pool);
    let lock_starts_at = p.fill_deadline - params.withdraw_lock_secs;

    // Withdraw just before the lock (deadline - lock - 1 second) - should succeed
    c.set_time(lock_starts_at - 1);

    // Check that withdrawal won't lower tier (350 -> 250 stays in tier 2)
    assert_eq!(c.gb.pool(&pool).total_units, 350);

    let balance_before = c.bal(&member);
    c.gb.withdraw_commitment(&member, &pool);
    assert_eq!(c.bal(&member) - balance_before, initial_paid);
    assert_eq!(c.gb.commitment(&pool, &member).unwrap().units, 0);
    assert_eq!(c.gb.pool(&pool).total_units, 250);

    // Re-commit after withdrawal - should charge current tier ceiling price
    // Pool is at 250 units (tier 2), so new commits pay tier 2 ceiling price
    let (tier_idx, tier_price) = c.gb.current_tier(&pool);
    assert_eq!(tier_idx, 1); // tier 2 (index 1)
    assert_eq!(tier_price, 9_000_000); // 0.9 USDC = 9M stroops

    c.gb.commit(&member, &pool, &50);
    let new_paid = c.gb.commitment(&pool, &member).unwrap().paid;
    assert_eq!(new_paid, 50 * tier_price); // Charged at tier 2 ceiling price

    // Now try to withdraw at deadline - lock + 1 (inside the lock window) - should fail
    c.set_time(lock_starts_at + 1);
    assert!(c.gb.try_withdraw_commitment(&member, &pool).is_err());
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
    c.gb.reject(&c.supplier, &pool, &None::<Symbol>);
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

fn settled_delivery(offset_from_lead: i64) -> sp_common::types::RepStats {
    let c = setup();
    let pool = c.pool(0);
    c.fill_standard(pool);
    c.gb.accept(&c.supplier, &pool, &0);
    let lead = c.gb.pool(&pool).terms.lead_time_secs;
    if offset_from_lead >= 0 {
        c.advance(lead + offset_from_lead as u64);
    } else {
        c.advance(lead - (-offset_from_lead) as u64);
    }
    c.gb.dispatch(&c.supplier, &pool, &None);
    c.gb.confirm_delivery(&c.organizer, &pool, &420, &h(&c.env, 6));
    c.advance(48 * DAY + 1);
    c.gb.settle(&pool);
    c.rep.stats(&c.supplier, &Role::Supplier)
}

#[test]
fn delivery_on_time_boundary_and_grace_are_tracked_separately() {
    let before = settled_delivery(-1);
    assert_eq!((before.on_time, before.late), (1, 0));
    let exact = settled_delivery(0);
    assert_eq!((exact.on_time, exact.late), (1, 0));
    let late = settled_delivery(DAY as i64);
    assert_eq!((late.on_time, late.late), (0, 1));
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
    assert_eq!(
        c.gb.try_create_pool(&c.supplier, &c.terms(), &h(&c.env, 7), &0, &None, &dl)
            .err(),
        Some(Ok(soroban_sdk::Error::from_contract_error(
            group_buy::Error::OrganizerIsSupplier as u32
        )))
    );
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
fn every_pause_scope_preserves_expiry_refunds() {
    let scopes = [
        symbol_short!("create"),
        symbol_short!("commit"),
        symbol_short!("accept"),
        symbol_short!("all"),
    ];
    for scope in scopes {
        let c = setup();
        let pool = c.pool(0);
        let trader = c.trader();
        c.gb.commit(&trader, &pool, &10);
        c.cfg.pause(&c.admin, &scope);
        c.advance(3 * DAY);
        c.gb.close(&pool);
        assert_eq!(c.gb.claim_refund(&trader, &pool), 10 * USDC);
    }
}

#[test]
fn suspended_participants_keep_existing_pool_exits_but_not_new_activity() {
    let c = setup();
    let pool = c.pool(0);
    let trader = c.trader();
    c.gb.commit(&trader, &pool, &10);
    c.reg
        .suspend(&c.admin, &trader, &Role::Trader, &symbol_short!("test"));
    assert!(c.gb.try_commit(&trader, &pool, &1).is_err());
    c.advance(3 * DAY);
    c.gb.close(&pool);
    assert_eq!(c.gb.claim_refund(&trader, &pool), 10 * USDC);

    let c = setup();
    let pool = c.pool(0);
    c.fill_standard(pool);
    c.gb.accept(&c.supplier, &pool, &0);
    c.reg.suspend(
        &c.admin,
        &c.supplier,
        &Role::Supplier,
        &symbol_short!("test"),
    );
    c.gb.dispatch(&c.supplier, &pool, &None);

    let c = setup();
    c.reg.suspend(
        &c.admin,
        &c.organizer,
        &Role::Organizer,
        &symbol_short!("test"),
    );
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
fn admin_cancel_pool_in_open_filled_accepted_with_full_refunds() {
    let c = setup();

    // Test 1: Admin cancel in Open state with commitments
    let pool_open = c.pool(0);
    let members_open: Vec<Address> = (0..3).map(|_| c.trader()).collect();
    c.gb.commit(&members_open[0], &pool_open, &50);
    c.gb.commit(&members_open[1], &pool_open, &75);
    c.gb.commit(&members_open[2], &pool_open, &25);

    let total_paid_open = 50 * USDC + 75 * USDC + 25 * USDC;
    let pool_open_state = c.gb.pool(&pool_open);
    assert_eq!(pool_open_state.state, PoolState::Open);
    assert_eq!(pool_open_state.total_paid, total_paid_open);

    // Admin cancels the Open pool
    c.gb.cancel_pool(&c.admin, &pool_open);
    let cancelled_pool = c.gb.pool(&pool_open);
    assert_eq!(cancelled_pool.state, PoolState::Cancelled);
    assert_eq!(cancelled_pool.owed_total, total_paid_open);

    // All members get full refund, no fees taken
    assert_eq!(c.gb.refundable_of(&pool_open, &members_open[0]), 50 * USDC);
    assert_eq!(c.gb.refundable_of(&pool_open, &members_open[1]), 75 * USDC);
    assert_eq!(c.gb.refundable_of(&pool_open, &members_open[2]), 25 * USDC);

    for member in &members_open {
        let refund = c.gb.claim_refund(member, &pool_open);
        let commitment = c.gb.commitment(&pool_open, member).unwrap();
        assert_eq!(refund, commitment.paid); // Full refund equals what they paid
    }

    // Test 2: Admin cancel in Filled state
    let pool_filled = c.pool(0);
    let members_filled = c.fill_standard(pool_filled); // Fills to 420 units (tier 3)
    let pool_filled_state = c.gb.pool(&pool_filled);
    assert_eq!(pool_filled_state.state, PoolState::Filled);
    let total_paid_filled = pool_filled_state.total_paid;

    // Admin cancels the Filled pool
    c.gb.cancel_pool(&c.admin, &pool_filled);
    let cancelled_filled = c.gb.pool(&pool_filled);
    assert_eq!(cancelled_filled.state, PoolState::Cancelled);
    assert_eq!(cancelled_filled.owed_total, total_paid_filled);

    // All members get full refund
    for member in &members_filled {
        let commitment = c.gb.commitment(&pool_filled, member).unwrap();
        let refundable = c.gb.refundable_of(&pool_filled, member);
        assert_eq!(refundable, commitment.paid); // Full refund
        let actual_refund = c.gb.claim_refund(member, &pool_filled);
        assert_eq!(actual_refund, commitment.paid);
    }

    // Test 3: Admin cancel in Accepted state
    let pool_accepted = c.pool(0);
    let members_accepted = c.fill_standard(pool_accepted);
    c.gb.accept(&c.supplier, &pool_accepted, &0); // Supplier accepts
    let pool_accepted_state = c.gb.pool(&pool_accepted);
    assert_eq!(pool_accepted_state.state, PoolState::Accepted);
    let total_paid_accepted = pool_accepted_state.total_paid;

    // Admin cancels the Accepted pool
    c.gb.cancel_pool(&c.admin, &pool_accepted);
    let cancelled_accepted = c.gb.pool(&pool_accepted);
    assert_eq!(cancelled_accepted.state, PoolState::Cancelled);
    assert_eq!(cancelled_accepted.owed_total, total_paid_accepted);

    // All members get full refund
    for member in &members_accepted {
        let commitment = c.gb.commitment(&pool_accepted, member).unwrap();
        let refundable = c.gb.refundable_of(&pool_accepted, member);
        assert_eq!(refundable, commitment.paid); // Full refund
        let actual_refund = c.gb.claim_refund(member, &pool_accepted);
        assert_eq!(actual_refund, commitment.paid);
    }

    // Test 4: Admin cancel is rejected after Dispatched
    let pool_dispatched = c.pool(0);
    let _ = c.fill_standard(pool_dispatched);
    c.gb.accept(&c.supplier, &pool_dispatched, &0);
    c.gb.dispatch(&c.supplier, &pool_dispatched, &None);
    assert_eq!(c.gb.pool(&pool_dispatched).state, PoolState::Dispatched);

    assert!(c.gb.try_cancel_pool(&c.admin, &pool_dispatched).is_err());
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

// ── Issue #138: fill_deadline boundary tests ──────────────────────────────────

/// Helper: attempt create_pool with the given fill_deadline and return the error
/// code if the call fails, or None if it succeeds.
fn try_dl(c: &scenarios::Ctx, deadline: u64) -> Option<u32> {
    c.gb.try_create_pool(
        &c.organizer,
        &c.terms(),
        &h(&c.env, 7),
        &0,
        &None,
        &deadline,
    )
    .err()
    .and_then(|r| r.ok())
    .map(|e| e.get_code())
}

#[test]
fn fill_deadline_equal_to_now_is_rejected_with_deadline_in_past() {
    let c = setup();
    // fill_deadline == now: not strictly in the future.
    let err = try_dl(&c, c.now());
    assert_eq!(err, Some(group_buy::Error::DeadlineInPast as u32));
}

#[test]
fn fill_deadline_one_second_after_now_is_accepted() {
    let c = setup();
    // The smallest valid deadline: now + 1.
    assert!(c
        .gb
        .try_create_pool(
            &c.organizer,
            &c.terms(),
            &h(&c.env, 7),
            &0,
            &None,
            &(c.now() + 1),
        )
        .is_ok());
}

#[test]
fn fill_deadline_at_max_window_is_accepted() {
    let c = setup();
    // Default max_fill_window_secs is 7 days; deadline == now + window is valid.
    let p = c.cfg.get_params();
    let at_cap = c.now() + p.max_fill_window_secs;
    assert!(c
        .gb
        .try_create_pool(&c.organizer, &c.terms(), &h(&c.env, 7), &0, &None, &at_cap,)
        .is_ok());
}

#[test]
fn fill_deadline_one_second_beyond_max_window_is_rejected_with_deadline_too_far() {
    let c = setup();
    let p = c.cfg.get_params();
    let beyond = c.now() + p.max_fill_window_secs + 1;
    let err = try_dl(&c, beyond);
    assert_eq!(err, Some(group_buy::Error::DeadlineTooFar as u32));
}

#[test]
fn fill_deadline_errors_are_distinct() {
    // DeadlineInPast and DeadlineTooFar must have different discriminants so the
    // frontend and backend can route them to different messages.
    assert_ne!(
        group_buy::Error::DeadlineInPast as u32,
        group_buy::Error::DeadlineTooFar as u32,
    );
}

#[test]
fn max_fill_window_can_be_raised_to_14_day_hard_cap() {
    let c = setup();
    // Raise max_fill_window_secs to the 14-day hard cap.
    let mut p = c.cfg.get_params();
    p.max_fill_window_secs = caps::FILL_WINDOW_SECS; // 14 * 24 * 3600
    c.cfg.set_params(&p);

    // A deadline right at the new cap must succeed.
    let at_hard_cap = c.now() + caps::FILL_WINDOW_SECS;
    assert!(c
        .gb
        .try_create_pool(
            &c.organizer,
            &c.terms(),
            &h(&c.env, 7),
            &0,
            &None,
            &at_hard_cap,
        )
        .is_ok());

    // One second beyond the hard cap must still fail (DeadlineTooFar).
    let err = try_dl(&c, at_hard_cap + 1);
    assert_eq!(err, Some(group_buy::Error::DeadlineTooFar as u32));
}

#[test]
fn create_pool_fails_when_organizer_is_supplier() {
    let c = setup();
    let dl = c.now() + DAY;
    // Organizer address matches supplier in terms
    let err =
        c.gb.try_create_pool(&c.supplier, &c.terms(), &h(&c.env, 7), &0, &None, &dl)
            .err();
    assert_eq!(
        err,
        Some(Ok(soroban_sdk::Error::from_contract_error(
            group_buy::Error::OrganizerIsSupplier as u32
        )))
    );

    // Terms supplier set to organizer
    let mut terms = c.terms();
    terms.supplier = c.organizer.clone();
    let err2 =
        c.gb.try_create_pool(&c.organizer, &terms, &h(&c.env, 7), &0, &None, &dl)
            .err();
    assert_eq!(
        err2,
        Some(Ok(soroban_sdk::Error::from_contract_error(
            group_buy::Error::OrganizerIsSupplier as u32
        )))
    );
}

#[test]
fn commit_fails_when_member_is_pool_supplier() {
    let c = setup();
    let pool = c.pool(0);
    // Even if supplier is also verified as a Trader
    c.verify(&c.supplier, Role::Trader);
    let err = c.gb.try_commit(&c.supplier, &pool, &10).err();
    assert_eq!(
        err,
        Some(Ok(soroban_sdk::Error::from_contract_error(
            group_buy::Error::SupplierIsParty as u32
        )))
    );
}

#[test]
fn organizer_member_share_cap_and_sole_member_exemption() {
    let c = setup();
    let mut terms = c.terms();
    terms.max_units = 100;
    terms.max_per_member = 100;
    terms.moq = 10;
    terms.tiers = soroban_sdk::vec![
        &c.env,
        Tier {
            min_units: 10,
            unit_price: USDC,
        },
    ];
    let dl = c.now() + 2 * DAY;
    let pool =
        c.gb.create_pool(&c.organizer, &terms, &h(&c.env, 7), &0, &None, &dl);

    c.mint.mint(&c.organizer, &(1_000 * USDC));

    // 1. Sole-member exemption: organizer can commit >50% (60 units of 100) as the only member
    c.gb.commit(&c.organizer, &pool, &60);
    assert_eq!(c.gb.commitment(&pool, &c.organizer).unwrap().units, 60);

    // 2. When other members exist:
    let pool2 =
        c.gb.create_pool(&c.organizer, &terms, &h(&c.env, 8), &0, &None, &dl);
    let trader = c.trader();
    c.gb.commit(&trader, &pool2, &10);

    // Organizer committing 51 units (one unit over the 50% cap of 100) is rejected
    let err_over = c.gb.try_commit(&c.organizer, &pool2, &51).err();
    assert_eq!(
        err_over,
        Some(Ok(soroban_sdk::Error::from_contract_error(
            group_buy::Error::OverMemberShare as u32
        )))
    );

    // Organizer committing 50 units (at the 50% cap) succeeds
    assert!(c.gb.try_commit(&c.organizer, &pool2, &50).is_ok());
    assert_eq!(c.gb.commitment(&pool2, &c.organizer).unwrap().units, 50);

    // Organizer increasing by 1 unit (from 50 to 51) while other members exist is rejected
    let err_inc = c.gb.try_increase(&c.organizer, &pool2, &1).err();
    assert_eq!(
        err_inc,
        Some(Ok(soroban_sdk::Error::from_contract_error(
            group_buy::Error::OverMemberShare as u32
        )))
    );

    // 3. Ordinary member behaviour is unchanged: sole-member exemption and 50% cap
    let pool3 =
        c.gb.create_pool(&c.organizer, &terms, &h(&c.env, 9), &0, &None, &dl);
    let trader1 = c.trader();
    // Sole member trader can commit 60 units (>50% of 100)
    c.gb.commit(&trader1, &pool3, &60);
    assert_eq!(c.gb.commitment(&pool3, &trader1).unwrap().units, 60);

    let pool4 =
        c.gb.create_pool(&c.organizer, &terms, &h(&c.env, 10), &0, &None, &dl);
    let trader2 = c.trader();
    let trader3 = c.trader();
    c.gb.commit(&trader2, &pool4, &10);
    // trader3 committing 51 units fails
    let err_trader_over = c.gb.try_commit(&trader3, &pool4, &51).err();
    assert_eq!(
        err_trader_over,
        Some(Ok(soroban_sdk::Error::from_contract_error(
            group_buy::Error::OverMemberShare as u32
        )))
    );
    // trader3 committing 50 units succeeds
    assert!(c.gb.try_commit(&trader3, &pool4, &50).is_ok());
}

#[test]
fn organizer_fee_at_hard_cap_boundaries() {
    let c = setup();
    let default_max = c.cfg.get_params().max_organizer_fee_bp;
    assert_eq!(default_max, 100);

    // 1. Boundary: 0 bp is accepted
    let p0 = c.pool(0);
    assert_eq!(c.gb.pool(&p0).organizer_fee_bp, 0);

    // 2. Boundary: configured maximum (100 bp) is accepted
    let p_max = c.pool(default_max);
    assert_eq!(c.gb.pool(&p_max).organizer_fee_bp, default_max);

    // 3. Boundary: configured maximum + 1 (101 bp) is rejected with FeeTooHigh
    let err_over_default =
        c.gb.try_create_pool(
            &c.organizer,
            &c.terms(),
            &h(&c.env, 11),
            &(default_max + 1),
            &None,
            &(c.now() + 3 * DAY),
        )
        .err();
    assert_eq!(
        err_over_default,
        Some(Ok(soroban_sdk::Error::from_contract_error(
            group_buy::Error::FeeTooHigh as u32
        )))
    );

    // 4. Admin raises max_organizer_fee_bp to the 200 bp hard cap
    let mut p = c.cfg.get_params();
    p.max_organizer_fee_bp = caps::ORGANIZER_FEE_BP;
    c.cfg.set_params(&p);
    assert_eq!(c.cfg.get_params().max_organizer_fee_bp, 200);

    // Configured maximum at hard cap (200 bp) is accepted
    let p_cap = c.pool(caps::ORGANIZER_FEE_BP);
    assert_eq!(c.gb.pool(&p_cap).organizer_fee_bp, 200);

    // One above the hard cap (201 bp) is rejected with FeeTooHigh
    let err_over_cap =
        c.gb.try_create_pool(
            &c.organizer,
            &c.terms(),
            &h(&c.env, 12),
            &(caps::ORGANIZER_FEE_BP + 1),
            &None,
            &(c.now() + 3 * DAY),
        )
        .err();
    assert_eq!(
        err_over_cap,
        Some(Ok(soroban_sdk::Error::from_contract_error(
            group_buy::Error::FeeTooHigh as u32
        )))
    );

    // 5. Raising above the 200 bp hard cap fails in set_params
    let mut p_too_high = c.cfg.get_params();
    p_too_high.max_organizer_fee_bp = caps::ORGANIZER_FEE_BP + 1;
    assert!(c.cfg.try_set_params(&p_too_high).is_err());
}

#[test]
fn reject_reason_allow_list_and_event_payload() {
    let c = setup();

    // 1. Rejection with disallowed reason is rejected with InvalidRejectReason
    let pool_bad = c.pool(0);
    c.fill_standard(pool_bad);
    let bad_reason = symbol_short!("invalid");
    let err_bad_reason =
        c.gb.try_reject(&c.supplier, &pool_bad, &Some(bad_reason))
            .err();
    assert_eq!(
        err_bad_reason,
        Some(Ok(soroban_sdk::Error::from_contract_error(
            group_buy::Error::InvalidRejectReason as u32
        )))
    );

    // 2. Rejection with allowed reason (no_stock) succeeds and emits reason
    let pool_good = c.pool(0);
    c.fill_standard(pool_good);
    let good_reason = symbol_short!("no_stock");
    c.gb.reject(&c.supplier, &pool_good, &Some(good_reason));
    assert_eq!(c.gb.pool(&pool_good).state, PoolState::Failed);
    assert!(
        format!("{:?}", c.env.events().all()).contains("no_stock"),
        "pool_rejected includes the reason in its payload"
    );

    // 3. Rejection with None defaults to 'other' and succeeds
    let pool_none = c.pool(0);
    c.fill_standard(pool_none);
    c.gb.reject(&c.supplier, &pool_none, &None);
    assert_eq!(c.gb.pool(&pool_none).state, PoolState::Failed);
    assert!(
        format!("{:?}", c.env.events().all()).contains("other"),
        "pool_rejected includes the default other reason in its payload"
    );

// Issue #41: Test fail_delivery against the exact delivery deadline
#[test]
fn confirm_delivery_succeeds_one_second_before_deadline() {
    let c = setup();
    let pool = c.pool(0);
    c.fill_standard(pool);
    c.gb.accept(&c.supplier, &pool, &0);
    c.gb.dispatch(&c.supplier, &pool, &None);
    
    let p = c.gb.pool(&pool);
    let lead_time = p.terms.lead_time_secs;
    let delivery_grace = c.cfg.get_params().delivery_grace_secs;
    let deadline = p.accepted_at + lead_time + delivery_grace;
    
    // Jump to 1 second before the deadline
    c.set_time(deadline - 1);
    
    // confirm_delivery should succeed
    c.gb.confirm_delivery(&c.organizer, &pool, &420, &h(&c.env, 7));
    assert_eq!(c.gb.pool(&pool).state, PoolState::Delivered);
}

#[test]
fn fail_delivery_fails_one_second_before_deadline() {
    let c = setup();
    let pool = c.pool(0);
    c.fill_standard(pool);
    c.gb.accept(&c.supplier, &pool, &0);
    c.gb.dispatch(&c.supplier, &pool, &None);
    
    let p = c.gb.pool(&pool);
    let lead_time = p.terms.lead_time_secs;
    let delivery_grace = c.cfg.get_params().delivery_grace_secs;
    let deadline = p.accepted_at + lead_time + delivery_grace;
    
    // Jump to 1 second before the deadline
    c.set_time(deadline - 1);
    
    // fail_delivery should fail (DeliveryDeadlineNotPassed)
    assert!(c.gb.try_fail_delivery(&pool).is_err());
}

#[test]
fn fail_delivery_succeeds_at_exact_deadline_and_beyond() {
    let c = setup();
    let pool = c.pool(0);
    c.fill_standard(pool);
    c.gb.accept(&c.supplier, &pool, &0);
    c.gb.dispatch(&c.supplier, &pool, &None);
    
    let p = c.gb.pool(&pool);
    let lead_time = p.terms.lead_time_secs;
    let delivery_grace = c.cfg.get_params().delivery_grace_secs;
    let deadline = p.accepted_at + lead_time + delivery_grace;
    
    // Jump to exact deadline
    c.set_time(deadline);
    
    // fail_delivery should succeed
    c.gb.fail_delivery(&pool);
    assert_eq!(c.gb.pool(&pool).state, PoolState::Failed);
}

#[test]
fn escrow_accounting_consistent_after_exact_deadline_paths() {
    let c = setup();
    
    // Path 1: confirm just before deadline
    let pool1 = c.pool(0);
    let t1 = c.fill_standard(pool1);
    c.gb.accept(&c.supplier, &pool1, &0);
    c.gb.dispatch(&c.supplier, &pool1, &None);
    let p1 = c.gb.pool(&pool1);
    let deadline1 = p1.accepted_at + p1.terms.lead_time_secs + c.cfg.get_params().delivery_grace_secs;
    c.set_time(deadline1 - 1);
    c.gb.confirm_delivery(&c.organizer, &pool1, &420, &h(&c.env, 8));
    c.advance(48 * 3600 + 1);
    c.gb.settle(&pool1);
    let escrow1 = c.gb.pool(&pool1).escrow_balance;
    
    // Path 2: fail at deadline
    let pool2 = c.pool(0);
    c.fill_standard(pool2);
    c.gb.accept(&c.supplier, &pool2, &0);
    c.gb.dispatch(&c.supplier, &pool2, &None);
    let p2 = c.gb.pool(&pool2);
    let deadline2 = p2.accepted_at + p2.terms.lead_time_secs + c.cfg.get_params().delivery_grace_secs;
    c.set_time(deadline2);
    c.gb.fail_delivery(&pool2);
    c.gb.push_refunds(&pool2, &10);
    let escrow2 = c.gb.pool(&pool2).escrow_balance;
    
    // Both paths should leave zero escrow balance
    assert_eq!(escrow1, 0, "confirmed delivery path should clear escrow");
    assert_eq!(escrow2, 0, "failed delivery path should clear escrow");
}

// Issue #40: Test fail_accept racing a late supplier accept
#[test]
fn accept_succeeds_one_second_before_window_closes() {
    let c = setup();
    let pool = c.pool(0);
    c.fill_standard(pool);
    
    let p = c.gb.pool(&pool);
    let accept_window = c.cfg.get_params().accept_window_secs;
    let window_end = p.filled_at + accept_window;
    
    // Jump to 1 second before window closes
    c.set_time(window_end - 1);
    
    // accept should succeed
    c.gb.accept(&c.supplier, &pool, &0);
    assert_eq!(c.gb.pool(&pool).state, PoolState::Accepted);
}

#[test]
fn fail_accept_is_rejected_one_second_before_window_closes() {
    let c = setup();
    let pool = c.pool(0);
    c.fill_standard(pool);
    
    let p = c.gb.pool(&pool);
    let accept_window = c.cfg.get_params().accept_window_secs;
    let window_end = p.filled_at + accept_window;
    
    // Jump to 1 second before window closes
    c.set_time(window_end - 1);
    
    // fail_accept should fail (AcceptWindowOpen)
    assert!(c.gb.try_fail_accept(&pool).is_err());
}

#[test]
fn accept_fails_at_exact_window_boundary() {
    let c = setup();
    let pool = c.pool(0);
    c.fill_standard(pool);
    
    let p = c.gb.pool(&pool);
    let accept_window = c.cfg.get_params().accept_window_secs;
    let window_end = p.filled_at + accept_window;
    
    // Jump to exact window boundary
    c.set_time(window_end);
    
    // accept should fail (AcceptWindowPassed)
    assert!(c.gb.try_accept(&c.supplier, &pool, &0).is_err());
}

#[test]
fn fail_accept_succeeds_at_exact_window_boundary() {
    let c = setup();
    let pool = c.pool(0);
    c.fill_standard(pool);
    
    let p = c.gb.pool(&pool);
    let accept_window = c.cfg.get_params().accept_window_secs;
    let window_end = p.filled_at + accept_window;
    
    // Jump to exact window boundary
    c.set_time(window_end);
    
    // fail_accept should succeed
    c.gb.fail_accept(&pool);
    assert_eq!(c.gb.pool(&pool).state, PoolState::Failed);
}
