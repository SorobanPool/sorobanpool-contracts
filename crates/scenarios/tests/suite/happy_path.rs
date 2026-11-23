use scenarios::*;
use sp_common::types::{PoolState, Tier};

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

// Issue #147: refunds_pending view tracks keeper progress across batches
#[test]
fn refunds_pending_decrements_with_each_push_batch() {
    let c = setup();
    let pool = c.pool(0);
    let _t = c.fill_standard(pool);
    c.deliver(pool, 420);
    c.advance(48 * 3600);
    c.gb.settle(&pool);

    // Before any batch: all 4 members are pending.
    let (rem, amt) = c.gb.refunds_pending(&pool);
    assert_eq!(rem, 4, "all members should be pending before first push");
    assert!(amt > 0, "pending amount should be positive");

    // Push 3: 1 member left.
    c.gb.push_refunds(&pool, &3);
    let (rem2, amt2) = c.gb.refunds_pending(&pool);
    assert_eq!(rem2, 1);
    assert!(amt2 < amt);

    // Push the final batch: 0 members left, 0 amount.
    c.gb.push_refunds(&pool, &3);
    let (rem3, amt3) = c.gb.refunds_pending(&pool);
    assert_eq!(rem3, 0);
    assert_eq!(amt3, 0);
}

// refunds_pending returns (0, 0) when pool is not in a refundable state
#[test]
fn refunds_pending_is_zero_for_non_refundable_pool() {
    let c = setup();
    let pool = c.pool(0);
    c.fill_standard(pool);
    // Pool is Filled — not yet in a refundable state.
    let (rem, amt) = c.gb.refunds_pending(&pool);
    assert_eq!(rem, 0);
    assert_eq!(amt, 0);
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

#[test]
fn early_release_at_exact_weight_threshold_and_rounding() {
    let c = setup();
    let mut terms = c.terms();
    terms.moq = 10;
    terms.max_units = 10;
    terms.max_per_member = 5;
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

    let m1 = c.trader();
    let m2 = c.trader();
    let m3 = c.trader();

    c.gb.commit(&m1, &pool, &5);
    c.gb.commit(&m2, &pool, &1);
    c.gb.commit(&m3, &pool, &4);

    // Pool reaches 10 units -> Filled
    assert_eq!(c.gb.pool(&pool).state, PoolState::Filled);

    // Deliver
    c.deliver(pool, 10);
    assert_eq!(c.gb.pool(&pool).state, PoolState::Delivered);

    // Confirmation window is still active:
    // With 0 confirmed pickups: settle fails with SettleTooEarly
    let err0 = c.gb.try_settle(&pool).err();
    assert_eq!(
        err0,
        Some(Ok(soroban_sdk::Error::from_contract_error(
            group_buy::Error::SettleTooEarly as u32
        )))
    );

    // 5 confirmed units (one unit below 60% of 10) fails with SettleTooEarly
    c.gb.confirm_pickup(&m1, &pool);
    let err5 = c.gb.try_settle(&pool).err();
    assert_eq!(
        err5,
        Some(Ok(soroban_sdk::Error::from_contract_error(
            group_buy::Error::SettleTooEarly as u32
        )))
    );

    // 6 confirmed units (exactly 60% of 10: 5 + 1) succeeds
    c.gb.confirm_pickup(&m2, &pool);
    assert!(c.gb.try_settle(&pool).is_ok());
    assert_eq!(c.gb.pool(&pool).state, PoolState::Settled);

    // --- 7-unit pool to test rounding ---
    let mut terms7 = c.terms();
    terms7.moq = 7;
    terms7.max_units = 7;
    terms7.max_per_member = 3;
    terms7.tiers = soroban_sdk::vec![
        &c.env,
        Tier {
            min_units: 7,
            unit_price: USDC,
        },
    ];
    let pool7 =
        c.gb.create_pool(&c.organizer, &terms7, &h(&c.env, 8), &0, &None, &dl);

    let u1 = c.trader();
    let u2 = c.trader();
    let u3 = c.trader();
    let u4 = c.trader();

    c.gb.commit(&u1, &pool7, &3);
    c.gb.commit(&u2, &pool7, &1);
    c.gb.commit(&u3, &pool7, &1);
    c.gb.commit(&u4, &pool7, &2);

    assert_eq!(c.gb.pool(&pool7).state, PoolState::Filled);
    c.deliver(pool7, 7);
    assert_eq!(c.gb.pool(&pool7).state, PoolState::Delivered);

    // 60% of 7 is 4.2 units.
    // 4 confirmed units (3 + 1 = 4, 40_000 < 42_000) is one unit below rounded threshold (5 units)
    c.gb.confirm_pickup(&u1, &pool7);
    c.gb.confirm_pickup(&u2, &pool7);
    let err4 = c.gb.try_settle(&pool7).err();
    assert_eq!(
        err4,
        Some(Ok(soroban_sdk::Error::from_contract_error(
            group_buy::Error::SettleTooEarly as u32
        )))
    );

    // 5 confirmed units (3 + 1 + 1 = 5, 50_000 >= 42_000) reaches the rounded-up 60% threshold
    c.gb.confirm_pickup(&u3, &pool7);
    assert!(c.gb.try_settle(&pool7).is_ok());
    assert_eq!(c.gb.pool(&pool7).state, PoolState::Settled);
}

#[test]
fn increase_across_price_tier_boundary_charges_pre_tier_ceiling_and_refunds_difference() {
    let c = setup();
    let pool = c.pool(0);

    let a = c.trader();
    let b = c.trader();

    // 1. Member A commits 100 units. Pool reaches MOQ (100 units, Tier 1: 1.00 USDC).
    // Charged at Tier 1 ceiling price: 100 * 1.00 = 100 USDC.
    c.gb.commit(&a, &pool, &100);
    assert_eq!(c.gb.commitment(&pool, &a).unwrap().paid, 100 * USDC);
    assert_eq!(c.gb.pool(&pool).total_units, 100);

    // 2. Member B commits 40 units while pool is in Tier 1 (100..200 units).
    // Pre-commit total is 100 (Tier 1 ceiling price: 1.00 USDC).
    // Charged: 40 * 1.00 = 40 USDC. Pool total becomes 140 (still Tier 1).
    c.gb.commit(&b, &pool, &40);
    assert_eq!(c.gb.commitment(&pool, &b).unwrap().paid, 40 * USDC);
    assert_eq!(c.gb.pool(&pool).total_units, 140);

    // 3. Member B increases by 70 units:
    // Moves total units from 140 (Tier 1) to 210 (Tier 2: 200..400 units, 0.90 USDC).
    // Pre-increase tier is Tier 1, with ceiling price 1.00 USDC.
    // Charged amount must equal extra_units x ceiling price of pre-increase tier = 70 x 1.00 = 70 USDC.
    let charged = c.gb.increase(&b, &pool, &70);
    assert_eq!(charged, 70 * USDC);

    // Verify Member B commitment state: 110 units, 110 USDC paid
    let b_commit = c.gb.commitment(&pool, &b).unwrap();
    assert_eq!(b_commit.units, 110);
    assert_eq!(b_commit.paid, 110 * USDC);

    // 4. Assert escrow coverage holds after the increase:
    let p = c.gb.pool(&pool);
    assert_eq!(p.total_units, 210);
    assert_eq!(p.total_paid, 210 * USDC);
    assert_eq!(p.escrow_balance, 210 * USDC);
    assert_eq!(c.bal(&c.gb.address), p.escrow_balance);

    // Escrow balance covers total_units * ceiling_price
    let ceiling_p = sp_common::math::ceiling_price(&p.terms.tiers, p.total_units).unwrap();
    assert!(p.escrow_balance >= (p.total_units as i128) * ceiling_p);

    // Escrow invariant holds
    let (expected, actual) = c.gb.escrow_invariant(&pool);
    assert!(
        actual >= expected,
        "escrow invariant breached after increase: actual={actual} expected={expected}"
    );

    // 5. Close pool and verify final price is Tier 2 price (0.90 USDC)
    c.gb.close_early(&c.organizer, &pool);
    let p_filled = c.gb.pool(&pool);
    assert_eq!(p_filled.state, PoolState::Filled);
    assert_eq!(p_filled.final_price, USDC * 9 / 10);

    // 6. Deliver all units and members confirm pickup
    c.deliver(pool, 210);
    c.gb.confirm_pickup(&a, &pool);
    c.gb.confirm_pickup(&b, &pool);

    // 7. Settle the pool
    c.gb.settle(&pool);
    assert_eq!(c.gb.pool(&pool).state, PoolState::Settled);

    // 8. Final refund equals paid - (units x final tier price) for both:
    // Earlier member A: paid 100 USDC for 100 units @ final price 0.90 USDC -> refund = 100 - 90 = 10 USDC
    let final_price = p_filled.final_price;
    let expected_refund_a = 100 * USDC - ((100 as i128) * final_price);
    let refund_a = c.gb.claim_refund(&a, &pool);
    assert_eq!(refund_a, expected_refund_a);
    assert_eq!(refund_a, 10 * USDC);

    // Increasing member B: paid 110 USDC for 110 units @ final price 0.90 USDC -> refund = 110 - 99 = 11 USDC
    let expected_refund_b = 110 * USDC - ((110 as i128) * final_price);
    let refund_b = c.gb.claim_refund(&b, &pool);
    assert_eq!(refund_b, expected_refund_b);
    assert_eq!(refund_b, 11 * USDC);

    // Escrow is fully settled with no dust remaining
    assert_eq!(c.bal(&c.gb.address), 0);
}

// Issue #53: members may claim while the keeper advances through refund pages.
#[test]
fn claim_refund_and_push_refunds_interleave_without_double_payment() {
    let c = setup();
    let pool = c.pool(0);
    let members: Vec<_> = (0..12).map(|_| c.trader()).collect();
    let initial_balances: Vec<i128> = members.iter().map(|m| c.bal(m)).collect();
    let starting_total = initial_balances.iter().sum::<i128>() + c.bal(&c.gb.address);

    // 1 + 2 + ... + 12 = 78 units: below the 100-unit MOQ, so closing
    // expires the pool and each of the 12 members is owed their full deposit.
    let mut deposits = Vec::new();
    for (index, member) in members.iter().enumerate() {
        let units = u32::try_from(index + 1).unwrap();
        deposits.push(c.gb.commit(member, &pool, &units));
    }
    let total_deposited: i128 = deposits.iter().sum();
    assert_eq!(total_deposited, 78 * USDC);
    assert_eq!(c.bal(&c.gb.address), total_deposited);

    c.advance(3 * DAY);
    c.gb.close(&pool);
    assert_eq!(c.gb.pool(&pool).state, PoolState::Expired);
    assert_eq!(c.gb.pool(&pool).owed_total, total_deposited);

    // Check each member's refund ledger AND their real token balance at
    // every interleaving boundary. An individual may receive their deposit
    // exactly once, whether by claim or by keeper batch.
    let assert_state = |paid: &[bool; 12]| {
        let mut refunded_total = 0i128;
        let mut held_by_members = 0i128;
        for (index, member) in members.iter().enumerate() {
            let expected_refund = if paid[index] { deposits[index] } else { 0 };
            let commitment = c.gb.commitment(&pool, member).unwrap();
            assert_eq!(
                commitment.refund_claimed, expected_refund,
                "refund_claimed mismatch for member {index}"
            );
            assert_eq!(
                c.gb.refundable_of(&pool, member),
                deposits[index] - expected_refund,
                "outstanding refund mismatch for member {index}"
            );
            let balance = c.bal(member);
            assert_eq!(
                balance,
                initial_balances[index] - deposits[index] + expected_refund,
                "member {index} was underpaid or paid twice"
            );
            refunded_total += expected_refund;
            held_by_members += balance;
        }
        let pool_state = c.gb.pool(&pool);
        assert_eq!(pool_state.refunds_paid, refunded_total);
        assert_eq!(pool_state.escrow_balance, total_deposited - refunded_total);
        assert_eq!(c.bal(&c.gb.address), pool_state.escrow_balance);
        assert_eq!(
            held_by_members + c.bal(&c.gb.address),
            starting_total,
            "USDC conservation must hold throughout the interleaving"
        );
        let (expected, actual) = c.gb.escrow_invariant(&pool);
        assert_eq!(expected, actual);
    };

    let mut paid = [false; 12];
    assert_state(&paid);

    // Three members claim before the keeper visits any entries.
    for index in [0, 2, 8] {
        assert_eq!(c.gb.claim_refund(&members[index], &pool), deposits[index]);
        paid[index] = true;
    }
    assert_state(&paid);

    // The keeper visits positions 0–3, skipping the already-paid 0 and 2.
    // Its return value counts visited entries, NOT token transfers.
    assert_eq!(c.gb.push_refunds(&pool, &4), 4);
    for index in [1, 3] {
        paid[index] = true;
    }
    assert_state(&paid);
    assert_eq!(c.gb.pool(&pool).refund_cursor, 4);

    // Another three members claim while the keeper cursor is partway through.
    // Exactly six of twelve members will have claimed manually.
    for index in [4, 6, 10] {
        assert_eq!(c.gb.claim_refund(&members[index], &pool), deposits[index]);
        paid[index] = true;
    }
    assert_state(&paid);

    // Re-claiming after a manual claim or after a keeper push must produce
    // the precise contract error, not another transfer.
    let nothing_to_claim =
        soroban_sdk::Error::from_contract_error(group_buy::Error::NothingToClaim as u32);
    for index in [0, 1, 2, 3, 4, 6, 8, 10] {
        assert_eq!(
            c.gb.try_claim_refund(&members[index], &pool),
            Err(Ok(nothing_to_claim)),
            "member {index} could claim a second refund"
        );
    }
    assert_state(&paid);

    // Continue through claimed and unclaimed entries in bounded batches.
    assert_eq!(c.gb.push_refunds(&pool, &3), 3);
    paid[5] = true; // indices 4 and 6 were already claimed
    assert_state(&paid);

    assert_eq!(c.gb.push_refunds(&pool, &3), 3);
    paid[7] = true;
    paid[9] = true; // index 8 was already claimed
    assert_state(&paid);

    assert_eq!(c.gb.push_refunds(&pool, &12), 2);
    paid[11] = true; // index 10 was already claimed
    assert_state(&paid);

    assert!(paid.iter().all(|is_paid| *is_paid));
    assert_eq!(c.gb.push_refunds(&pool, &12), 0);
    for member in &members {
        assert_eq!(
            c.gb.try_claim_refund(member, &pool),
            Err(Ok(nothing_to_claim))
        );
    }
    assert_state(&paid);

    assert_eq!(c.gb.pool(&pool).refund_cursor, 12);
    assert_eq!(c.gb.pool(&pool).refunds_paid, total_deposited);
    assert_eq!(c.gb.refunds_pending(&pool), (0, 0));
    assert_eq!(c.gb.sweep_dust(&pool), 0);
    assert_eq!(c.bal(&c.gb.address), 0);
}
