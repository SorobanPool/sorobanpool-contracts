//! Randomised operation sequences: whatever order members commit, increase, withdraw and time passes,
//! no token is created or destroyed, the escrow contract holds exactly what it records, and rejected calls fail
//! with a typed contract error, never a host panic.
use proptest::prelude::*;
use scenarios::*;
use soroban_sdk::testutils::Events as _;
use soroban_sdk::{symbol_short, xdr, Address, Symbol, TryFromVal};
use sp_common::types::PoolState;

/// Collect every address that the escrow contract (`gb_id`) transferred tokens *to*.
///
/// The Stellar Asset Contract emits a `transfer` event with topics
/// `[Symbol("transfer"), from: Address, to: Address]`.  We filter to transfers
/// whose `from` is the group_buy contract so we only examine escrow outflows.
fn escrow_outflow_destinations(c: &Ctx, gb_id: &Address) -> Vec<Address> {
    let mut out = Vec::new();
    for event in c.env.events().all().events() {
        let xdr::ContractEventBody::V0(ref v0) = event.body;
        if v0.topics.len() >= 3 {
            if let (Ok(sym), Ok(from), Ok(to)) = (
                Symbol::try_from_val(&c.env, &v0.topics[0]),
                Address::try_from_val(&c.env, &v0.topics[1]),
                Address::try_from_val(&c.env, &v0.topics[2]),
            ) {
                if sym == symbol_short!("transfer") && &from == gb_id {
                    out.push(to);
                }
            }
        }
    }
    out
}

#[derive(Debug, Clone)]
enum Op {
    Commit(usize, u32),
    Increase(usize, u32),
    Withdraw(usize),
    Advance(u64),
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        4 => (0usize..6, 0u32..=300).prop_map(|(i, u)| Op::Commit(i, u)),
        2 => (0usize..6, 0u32..=200).prop_map(|(i, u)| Op::Increase(i, u)),
        2 => (0usize..6).prop_map(Op::Withdraw),
        1 => (0u64..=DAY).prop_map(Op::Advance),
    ]
}

/// A rejected call must be a contract error (`Err(Ok(_))`), not an `InvokeError` from a panic or trap.
fn typed<T, E: std::fmt::Debug, C: std::fmt::Debug>(
    r: Result<Result<T, E>, Result<C, soroban_sdk::InvokeError>>,
) -> Result<(), TestCaseError> {
    match r {
        Ok(_) | Err(Ok(_)) => Ok(()),
        Err(Err(e)) => Err(TestCaseError::fail(format!(
            "host-level failure, not a contract error: {e:?}"
        ))),
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(40))]

    #[test]
    fn random_sequences_conserve_tokens_and_fail_cleanly(ops in proptest::collection::vec(op(), 1..25), received_pct in 0u32..=100) {
        let c = setup();
        let pool = c.pool(100);
        let members: Vec<Address> = (0..6).map(|_| c.trader()).collect();
        let everyone: Vec<&Address> = members.iter().chain([&c.supplier, &c.organizer, &c.treasury, &c.gb.address]).collect();
        let total = |c: &Ctx| -> i128 { everyone.iter().map(|a| c.bal(a)).sum() };
        let start = total(&c);

        for o in &ops {
            match o {
                Op::Commit(i, u) => typed(c.gb.try_commit(&members[*i], &pool, u))?,
                Op::Increase(i, u) => typed(c.gb.try_increase(&members[*i], &pool, u))?,
                Op::Withdraw(i) => typed(c.gb.try_withdraw_commitment(&members[*i], &pool))?,
                Op::Advance(s) => c.advance(*s),
            }
            prop_assert_eq!(total(&c), start);
            let p = c.gb.pool(&pool);
            prop_assert_eq!(c.bal(&c.gb.address), p.escrow_balance);
            prop_assert!(p.total_units <= p.terms.max_units);
        }

        c.advance(4 * DAY);
        typed(c.gb.try_close(&pool))?;
        let p = c.gb.pool(&pool);
        if p.state == PoolState::Filled {
            // A pool that filled early may have sat past the accept window while time advanced: the supplier is then
            // too late and the keeper's fail_accept refunds everyone.
            if c.gb.try_accept(&c.supplier, &pool, &0).is_ok() {
                c.gb.dispatch(&c.supplier, &pool, &None);
                let received = (p.total_units as u64 * received_pct as u64 / 100) as u32;
                c.gb.confirm_delivery(&c.organizer, &pool, &received, &h(&c.env, 5));
                c.gb.allocate_shortfall(&pool, &50);
                c.advance(48 * 3600);
                c.gb.settle(&pool);
            } else {
                c.gb.fail_accept(&pool);
                prop_assert_eq!(c.gb.pool(&pool).state, PoolState::Failed);
            }
        }
        c.gb.push_refunds(&pool, &50);
        c.gb.sweep_dust(&pool);
        prop_assert_eq!(total(&c), start);
        prop_assert_eq!(c.bal(&c.gb.address), 0);
    }
}

// ── #156: escrow outflows must only reach the allowed destination set ─────────
//
// Allowed recipients of a transfer *from* the group_buy escrow:
//   • members      — refunds (tier credit, shortfall, expiry, failure)
//   • supplier     — net payment at settle
//   • organizer    — organizer fee at settle
//   • treasury     — platform fee at settle (and dust sweep)
//
// No other address may ever receive a transfer originating from escrow.
// This property is checked over random commit/increase/withdraw sequences
// that run the pool to completion (Settled, Expired, or Failed).

proptest! {
    #![proptest_config(ProptestConfig::with_cases(30))]

    #[test]
    fn escrow_only_pays_allowed_destinations(
        ops in proptest::collection::vec(op(), 1..20),
        received_pct in 0u32..=100,
    ) {
        let c = setup();
        let pool = c.pool(100);
        let members: Vec<Address> = (0..6).map(|_| c.trader()).collect();

        // Allowed set: every address that may legitimately receive escrow funds.
        let allowed: Vec<Address> = members
            .iter()
            .cloned()
            .chain([
                c.supplier.clone(),
                c.organizer.clone(),
                c.treasury.clone(),
            ])
            .collect();

        let gb_id = c.gb.address.clone();

        for o in &ops {
            match o {
                Op::Commit(i, u)   => typed(c.gb.try_commit(&members[*i], &pool, u))?,
                Op::Increase(i, u) => typed(c.gb.try_increase(&members[*i], &pool, u))?,
                Op::Withdraw(i)    => typed(c.gb.try_withdraw_commitment(&members[*i], &pool))?,
                Op::Advance(s)     => c.advance(*s),
            }
        }

        c.advance(4 * DAY);
        typed(c.gb.try_close(&pool))?;
        let p = c.gb.pool(&pool);
        if p.state == PoolState::Filled {
            if c.gb.try_accept(&c.supplier, &pool, &0).is_ok() {
                c.gb.dispatch(&c.supplier, &pool, &None);
                let received = (p.total_units as u64 * received_pct as u64 / 100) as u32;
                c.gb.confirm_delivery(&c.organizer, &pool, &received, &h(&c.env, 5));
                c.gb.allocate_shortfall(&pool, &50);
                c.advance(48 * 3600);
                c.gb.settle(&pool);
            } else {
                c.gb.fail_accept(&pool);
            }
        }
        c.gb.push_refunds(&pool, &50);
        c.gb.sweep_dust(&pool);

        // Assert: every address that received tokens from escrow is in the allowed set.
        for dest in escrow_outflow_destinations(&c, &gb_id) {
            prop_assert!(
                allowed.iter().any(|a| a == &dest),
                "escrow paid an unexpected destination: {:?}",
                dest
            );
        }
    }
}

/// Crafted bad-destination regression: a manually constructed scenario that
/// exercises every outflow path (refund, tier-credit, settle, dust sweep) and
/// verifies none reaches an address outside the allowed set.
#[test]
fn escrow_outflows_cover_all_paths_and_stay_in_allowed_set() {
    let c = setup();
    let pool = c.pool(100); // 1% organizer fee

    let members: Vec<Address> = (0..4).map(|_| c.trader()).collect();

    let allowed: Vec<Address> = members
        .iter()
        .cloned()
        .chain([c.supplier.clone(), c.organizer.clone(), c.treasury.clone()])
        .collect();

    let gb_id = c.gb.address.clone();

    // Commit across two tiers so a tier-credit refund is triggered at settle.
    c.gb.commit(&members[0], &pool, &100); // tier 1 price
    c.gb.commit(&members[1], &pool, &100); // still tier 1
    c.gb.commit(&members[2], &pool, &100); // crosses into tier 2 (200 units)
    c.gb.commit(&members[3], &pool, &120); // crosses into tier 3 (420 units)

    c.advance(3 * DAY);
    c.gb.close(&pool);

    // Shortfall delivery: supplier delivers 400 of 420 → 20-unit shortfall refund.
    c.gb.accept(&c.supplier, &pool, &0);
    c.gb.dispatch(&c.supplier, &pool, &None);
    c.gb.confirm_delivery(&c.organizer, &pool, &400, &h(&c.env, 5));
    c.gb.allocate_shortfall(&pool, &50);
    c.advance(48 * 3600);
    c.gb.settle(&pool);

    // Pull refunds for all members (tier credits + shortfall credits).
    for m in &members {
        c.gb.claim_refund(m, &pool);
    }
    c.gb.sweep_dust(&pool);

    for dest in escrow_outflow_destinations(&c, &gb_id) {
        assert!(
            allowed.iter().any(|a| a == &dest),
            "escrow paid an unexpected destination: {dest:?}"
        );
    }
    // Escrow must be empty after all exits.
    assert_eq!(c.bal(&c.gb.address), 0);
}

/// Regression from a CI fuzz seed: a pool that fills early and then idles past the accept window cannot be accepted;
/// the keeper's fail_accept refunds every member in full.
#[test]
fn early_filled_pool_past_accept_window_is_refunded() {
    let c = setup();
    let pool = c.pool(0);
    let (a, b) = (c.trader(), c.trader());
    c.gb.commit(&a, &pool, &250);
    c.gb.commit(&b, &pool, &250); // max_units 500: filled immediately
    assert_eq!(c.gb.pool(&pool).state, PoolState::Filled);
    c.advance(2 * DAY);
    assert!(c.gb.try_accept(&c.supplier, &pool, &0).is_err());
    c.gb.fail_accept(&pool);
    assert_eq!(c.gb.pool(&pool).state, PoolState::Failed);
    c.gb.push_refunds(&pool, &10);
    assert_eq!(
        (c.bal(&a), c.bal(&b), c.bal(&c.gb.address)),
        (1_000 * USDC, 1_000 * USDC, 0)
    );
}
