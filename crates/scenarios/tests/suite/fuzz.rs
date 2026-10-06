//! Randomised operation sequences: whatever order members commit, increase, withdraw and time passes,
//! no token is created or destroyed, the escrow contract holds exactly what it records, and rejected calls fail
//! with a typed contract error, never a host panic.
use proptest::prelude::*;
use scenarios::*;
use soroban_sdk::Address;
use sp_common::types::PoolState;

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
            c.deliver(pool, (p.total_units as u64 * received_pct as u64 / 100) as u32);
            c.gb.allocate_shortfall(&pool, &50);
            c.advance(48 * 3600);
            c.gb.settle(&pool);
        }
        c.gb.push_refunds(&pool, &50);
        c.gb.sweep_dust(&pool);
        prop_assert_eq!(total(&c), start);
        prop_assert_eq!(c.bal(&c.gb.address), 0);
    }
}
