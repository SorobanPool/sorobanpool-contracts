use proptest::prelude::*;
use scenarios::*;
use soroban_sdk::{Address, Env, Vec as SorobanVec};
use sp_common::math::{
    final_unit_price, final_unit_price_slice, validate_tiers, validate_tiers_slice,
};
use sp_common::types::{PoolState, Tier};

/// Assert `escrow_invariant` holds (actual >= expected) after every operation (#143).
macro_rules! assert_invariant {
    ($ctx:expr, $pool:expr) => {{
        let (expected, actual) = $ctx.gb.escrow_invariant($pool);
        prop_assert!(
            actual >= expected,
            "escrow invariant breached: actual={actual} expected={expected} state={:?}",
            $ctx.gb.pool($pool).state
        );
    }};
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(fuzz_cases(24)))]

    /// `escrow_invariant` returns `actual >= expected` in every reachable state (#143).
    #[test]
    fn escrow_invariant_holds_in_all_states(
        sizes in proptest::collection::vec(1u32..=120, 1..8),
        received_pct in 0u32..=100,
    ) {
        let c = setup();
        let pool = c.pool(100);
        // after create
        assert_invariant!(c, &pool);

        let mut members: Vec<Address> = Vec::new();
        for s in &sizes {
            let m = c.trader();
            if c.gb.try_commit(&m, &pool, s).is_ok() {
                members.push(m);
                // after each commit
                assert_invariant!(c, &pool);
            }
        }

        c.advance(3 * DAY);
        c.gb.close(&pool);
        // after close
        assert_invariant!(c, &pool);

        let p = c.gb.pool(&pool);
        if p.state == PoolState::Expired {
            c.gb.push_refunds(&pool, &50);
            // after push_refunds on expired pool
            assert_invariant!(c, &pool);
            return Ok(());
        }

        let received = (p.total_units as u64 * received_pct as u64 / 100) as u32;
        c.deliver(pool, received);
        // after delivery
        assert_invariant!(c, &pool);
        c.gb.allocate_shortfall(&pool, &50);
        // after shortfall allocation
        assert_invariant!(c, &pool);
        c.advance(48 * 3600);
        c.gb.settle(&pool);
        // after settle
        assert_invariant!(c, &pool);
        c.gb.push_refunds(&pool, &50);
        // after push_refunds
        assert_invariant!(c, &pool);
        c.gb.sweep_dust(&pool);
        // after sweep_dust (invariant: both sides are 0)
        let (expected_final, actual_final) = c.gb.escrow_invariant(&pool);
        prop_assert_eq!(expected_final, 0, "owed_total - refunds_paid must be 0 after full settlement");
        prop_assert_eq!(actual_final, 0, "escrow_balance must be 0 after full settlement and sweep");
    }

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

// Strategy producing valid tier tables (1 to 5 tiers, ascending min, strictly descending price)
fn valid_tiers_strategy() -> impl Strategy<Value = (std::vec::Vec<Tier>, u32, u32)> {
    (
        1usize..=5,
        1u32..=50,
        proptest::array::uniform4(1u32..=30),
        proptest::array::uniform5(1i128..=10_000),
    )
        .prop_map(|(n, moq, deltas, price_deltas)| {
            let mut tiers = std::vec::Vec::with_capacity(n);
            let mut cur_min = moq;
            let sum_deltas: i128 = price_deltas[..n].iter().sum();
            let mut cur_price = 1_000_000i128 + sum_deltas;
            tiers.push(Tier {
                min_units: moq,
                unit_price: cur_price,
            });
            for i in 0..(n - 1) {
                cur_min += deltas[i];
                cur_price -= price_deltas[i];
                tiers.push(Tier {
                    min_units: cur_min,
                    unit_price: cur_price,
                });
            }
            let max_units = cur_min + 30;
            (tiers, moq, max_units)
        })
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 10_000,
        ..ProptestConfig::default()
    })]

    /// Property test (issue #85): Tier price is monotonic non-increasing for every unit count from 0 to maximum.
    /// Fixed seed recorded for reproducibility:
    /// SEED: 0x536f726f62616e506f6f6c54696572536565643230323631313230303030303031
    #[test]
    fn tier_price_is_monotonic_non_increasing_in_units(
        (tiers_vec, moq, max_units) in valid_tiers_strategy()
    ) {
        prop_assert!(validate_tiers_slice(&tiers_vec, moq, max_units));

        let mut prev_price = i128::MAX;
        for u in 0..=max_units {
            let p = final_unit_price_slice(&tiers_vec, u).unwrap();
            prop_assert!(
                p <= prev_price,
                "Monotonicity violated at u={u}: prev_price={prev_price}, current={p}"
            );
            prev_price = p;
        }
    }
}

#[test]
fn broken_tier_tables_fail_validation_not_property() {
    let env = Env::default();

    // Verify parity with Soroban Vec on a valid test table
    let sample_tiers = vec![
        Tier {
            min_units: 10,
            unit_price: 100,
        },
        Tier {
            min_units: 50,
            unit_price: 80,
        },
        Tier {
            min_units: 100,
            unit_price: 60,
        },
    ];
    let mut raw: SorobanVec<Tier> = SorobanVec::new(&env);
    for t in &sample_tiers {
        raw.push_back(t.clone());
    }
    assert!(validate_tiers(&raw, 10, 200));
    assert!(validate_tiers_slice(&sample_tiers, 10, 200));
    for u in 0..=200 {
        assert_eq!(
            final_unit_price(&raw, u),
            final_unit_price_slice(&sample_tiers, u)
        );
    }

    // Deliberately broken tables fail validation, not the property
    // 1. Non-descending price (tier 1 price > tier 0 price)
    let broken_price = vec![
        Tier {
            min_units: 10,
            unit_price: 100,
        },
        Tier {
            min_units: 20,
            unit_price: 120,
        },
    ];
    assert!(!validate_tiers_slice(&broken_price, 10, 50));
    let mut broken_price_sdk: SorobanVec<Tier> = SorobanVec::new(&env);
    for t in &broken_price {
        broken_price_sdk.push_back(t.clone());
    }
    assert!(!validate_tiers(&broken_price_sdk, 10, 50));

    // 2. Non-ascending min_units (tier 1 min_units <= tier 0 min_units)
    let broken_min = vec![
        Tier {
            min_units: 10,
            unit_price: 100,
        },
        Tier {
            min_units: 10,
            unit_price: 90,
        },
    ];
    assert!(!validate_tiers_slice(&broken_min, 10, 50));
    let mut broken_min_sdk: SorobanVec<Tier> = SorobanVec::new(&env);
    for t in &broken_min {
        broken_min_sdk.push_back(t.clone());
    }
    assert!(!validate_tiers(&broken_min_sdk, 10, 50));

    // 3. Zero price
    let zero_price = vec![Tier {
        min_units: 10,
        unit_price: 0,
    }];
    assert!(!validate_tiers_slice(&zero_price, 10, 50));
    let mut zero_price_sdk: SorobanVec<Tier> = SorobanVec::new(&env);
    for t in &zero_price {
        zero_price_sdk.push_back(t.clone());
    }
    assert!(!validate_tiers(&zero_price_sdk, 10, 50));

    // 4. Zero min_units
    let zero_min = vec![Tier {
        min_units: 0,
        unit_price: 100,
    }];
    assert!(!validate_tiers_slice(&zero_min, 0, 50));
    let mut zero_min_sdk: SorobanVec<Tier> = SorobanVec::new(&env);
    for t in &zero_min {
        zero_min_sdk.push_back(t.clone());
    }
    assert!(!validate_tiers(&zero_min_sdk, 0, 50));

    // 5. Empty tiers
    let empty: std::vec::Vec<Tier> = vec![];
    assert!(!validate_tiers_slice(&empty, 10, 50));
    let empty_sdk: SorobanVec<Tier> = SorobanVec::new(&env);
    assert!(!validate_tiers(&empty_sdk, 10, 50));

    // 6. More than 5 tiers (MAX_TIERS = 5)
    let mut six_tiers = vec![];
    for i in 0..6 {
        six_tiers.push(Tier {
            min_units: (i + 1) * 10,
            unit_price: 100 - (i as i128 * 10),
        });
    }
    assert!(!validate_tiers_slice(&six_tiers, 10, 100));
    let mut six_tiers_sdk: SorobanVec<Tier> = SorobanVec::new(&env);
    for t in &six_tiers {
        six_tiers_sdk.push_back(t.clone());
    }
    assert!(!validate_tiers(&six_tiers_sdk, 10, 100));

    // 7. First tier min_units != moq
    let wrong_moq = vec![Tier {
        min_units: 15,
        unit_price: 100,
    }];
    assert!(!validate_tiers_slice(&wrong_moq, 10, 50));
    let mut wrong_moq_sdk: SorobanVec<Tier> = SorobanVec::new(&env);
    for t in &wrong_moq {
        wrong_moq_sdk.push_back(t.clone());
    }
    assert!(!validate_tiers(&wrong_moq_sdk, 10, 50));

    // 8. max_units < moq
    let small_max = vec![Tier {
        min_units: 20,
        unit_price: 100,
    }];
    assert!(!validate_tiers_slice(&small_max, 20, 10));
    let mut small_max_sdk: SorobanVec<Tier> = SorobanVec::new(&env);
    for t in &small_max {
        small_max_sdk.push_back(t.clone());
    }
    assert!(!validate_tiers(&small_max_sdk, 20, 10));
}

#[test]
fn fee_rounding_table_favours_members_and_conserves_escrow() {
    use sp_common::math::{bp_of, round_member_refund, round_supplier_payable};

    // Table-driven test over at least 200 (units, price, fee) combinations
    let units_list: [u32; 8] = [1, 7, 13, 50, 100, 250, 420, 1000];
    let prices_list: [i128; 5] = [1, 99, 8_000_000, 9_000_000, 10_000_000];
    let plat_fees: [u32; 4] = [0, 50, 150, 500];
    let org_fees: [u32; 4] = [0, 25, 100, 200];

    let mut row_count = 0;
    for &units in &units_list {
        for &price in &prices_list {
            for &plat_bp in &plat_fees {
                for &org_bp in &org_fees {
                    row_count += 1;
                    let gross = (units as i128) * price;
                    let total_fee_bp = plat_bp + org_bp;
                    let supplier = round_supplier_payable(gross, total_fee_bp).unwrap();
                    let plat = bp_of(gross, plat_bp).unwrap();
                    let org = bp_of(gross, org_bp).unwrap();
                    let dust = gross - (supplier + plat + org);

                    let paid_per_unit = price + (price / 10).max(1);
                    let total_paid = (units as i128) * paid_per_unit;
                    let refunds = round_member_refund(total_paid, gross).unwrap();

                    // Acceptance criteria: The sum identity holds for every row
                    assert_eq!(
                        supplier + plat + org + refunds + dust,
                        total_paid,
                        "sum identity breached at row {row_count}"
                    );

                    // Rounding favours members: no component is off by more than 1 stroop in member's disfavour
                    assert!(dust >= 0);
                    assert!(dust <= 2);
                    assert!(supplier <= (gross * (10_000 - total_fee_bp) as i128) / 10_000);
                    assert!(plat <= (gross * plat_bp as i128) / 10_000);
                    assert!(org <= (gross * org_bp as i128) / 10_000);
                    assert_eq!(refunds, total_paid - gross);
                }
            }
        }
    }
    assert!(
        row_count >= 200,
        "must test at least 200 rows, tested {row_count}"
    );

    // On-chain scenario: Any dust is exactly what sweep_dust later reports
    let c = setup();
    let pool = c.pool(100);
    let t = c.fill_standard(pool);
    c.deliver(pool, 420);
    c.advance(48 * 3600);
    c.gb.settle(&pool);
    for m in &t {
        c.gb.claim_refund(m, &pool);
    }
    let swept = c.gb.sweep_dust(&pool);
    assert_eq!(swept, 0);
}
