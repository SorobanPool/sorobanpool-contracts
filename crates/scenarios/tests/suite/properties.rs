use proptest::prelude::*;
use scenarios::*;
use soroban_sdk::Address;
use sp_common::types::PoolState;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    /// Escrow covers the bill, every stroop is accounted for, nobody overpays, and no dust is left.
    #[test]
    fn money_is_conserved(
        sizes in proptest::collection::vec(1u32..=120, 1..8),
        received_pct in 0u32..=100,
    ) {
        let c = setup();
        let pool = c.pool(100);
        let mut members: Vec<Address> = Vec::new();
        let mut last_price = i128::MAX;
        for s in &sizes {
            let m = c.trader();
            if c.gb.try_commit(&m, &pool, s).is_ok() {
                members.push(m);
                // final price never rises as units are added
                let (_, price) = c.gb.current_tier(&pool);
                prop_assert!(price <= last_price);
                last_price = price;
            }
        }
        let mut paid_total = 0i128;
        for m in &members {
            let cm = c.gb.commitment(&pool, m).unwrap();
            paid_total += cm.paid;
            // never more than units * tier-1 price
            prop_assert!(cm.paid <= cm.units as i128 * USDC);
        }
        c.advance(3 * DAY);
        c.gb.close(&pool);
        let p = c.gb.pool(&pool);
        prop_assert_eq!(c.bal(&c.gb.address), paid_total);

        if p.state == PoolState::Expired {
            c.gb.push_refunds(&pool, &50);
            prop_assert_eq!(c.bal(&c.gb.address), 0);
            return Ok(());
        }
        // coverage invariant
        prop_assert!(p.escrow_balance >= p.total_units as i128 * p.final_price);

        let received = (p.total_units as u64 * received_pct as u64 / 100) as u32;
        c.deliver(pool, received);
        c.gb.allocate_shortfall(&pool, &50);
        c.advance(48 * 3600);
        c.gb.settle(&pool);
        let supplier_got = c.bal(&c.supplier) - 1_000 * USDC;
        let fees = c.bal(&c.treasury) + c.bal(&c.organizer);
        c.gb.push_refunds(&pool, &50);
        c.gb.sweep_dust(&pool);
        let refunded: i128 = members.iter().map(|m| c.bal(m) - 1_000 * USDC).sum::<i128>() + paid_total;
        prop_assert_eq!(supplier_got + fees + refunded, paid_total);
        prop_assert_eq!(c.bal(&c.gb.address), 0);
        let sum_alloc: u32 = members.iter().map(|m| c.gb.allocated_units_of(&pool, m)).sum();
        prop_assert_eq!(sum_alloc, received);
    }
}
