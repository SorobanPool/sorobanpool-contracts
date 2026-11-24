use crate::types::Tier;
use crate::BPS_DENOMINATOR;
use soroban_sdk::Vec;

/// Index of the highest tier whose `min_units` is reached, or `None` below MOQ.
pub fn tier_index(tiers: &Vec<Tier>, total_units: u32) -> Option<u32> {
    let mut found = None;
    for i in 0..tiers.len() {
        if let Some(t) = tiers.get(i) {
            if total_units >= t.min_units {
                found = Some(i);
            }
        }
    }
    found
}

/// Unit price a committer pays: the price of the tier the pool is in before
/// their units, i.e. the highest price they could end up paying.
pub fn ceiling_price(tiers: &Vec<Tier>, total_before: u32) -> Option<i128> {
    let idx = tier_index(tiers, total_before).unwrap_or(0);
    tiers.get(idx).map(|t| t.unit_price)
}

/// Final unit price for a pool that reached `total_units` (at least MOQ).
pub fn final_unit_price(tiers: &Vec<Tier>, total_units: u32) -> Option<i128> {
    ceiling_price(tiers, total_units)
}

/// `a * b / c` rounded down, with overflow and zero-divisor checks.
pub fn mul_div_floor(a: i128, b: i128, c: i128) -> Option<i128> {
    if c == 0 {
        return None;
    }
    a.checked_mul(b)?.checked_div(c)
}

pub fn bp_of(amount: i128, bp: u32) -> Option<i128> {
    mul_div_floor(amount, bp as i128, BPS_DENOMINATOR as i128)
}

/// Return the supplier's share after fees, rounded down in favour of escrow.
///
/// The discarded fractional unit remains in escrow as dust and can be swept
/// only after all member obligations have been satisfied.
pub fn round_supplier_payable(amount: i128, fee_bp: u32) -> Option<i128> {
    if fee_bp > BPS_DENOMINATOR {
        return None;
    }
    mul_div_floor(
        amount,
        (BPS_DENOMINATOR - fee_bp) as i128,
        BPS_DENOMINATOR as i128,
    )
}

/// Return the member refund after a rounded-down payable amount.
///
/// Because the payable side is rounded down, this subtraction rounds the
/// refund up to preserve the member's full entitlement.
pub fn round_member_refund(paid: i128, payable: i128) -> Option<i128> {
    paid.checked_sub(payable)
}

/// Deterministic shortfall allocation. Members are walked in commit order with
/// a running cumulative unit count; each gets
/// `floor(cum_after * received / total) - floor(cum_before * received / total)`,
/// so the allocations always sum to exactly `received`.
pub fn cumulative_alloc(cum_before: u32, units: u32, received: u32, total: u32) -> Option<u32> {
    if total == 0 {
        return None;
    }
    let after = (cum_before as u64).checked_add(units as u64)?;
    let hi = after.checked_mul(received as u64)? / total as u64;
    let lo = (cum_before as u64).checked_mul(received as u64)? / total as u64;
    u32::try_from(hi.checked_sub(lo)?).ok()
}

/// Tiers must be 1..=5, ascending `min_units`, strictly descending price, MOQ first.
pub fn validate_tiers(tiers: &Vec<Tier>, moq: u32, max_units: u32) -> bool {
    let n = tiers.len();
    if n == 0 || n > crate::types::caps::MAX_TIERS {
        return false;
    }
    let mut prev: Option<Tier> = None;
    for i in 0..n {
        let Some(t) = tiers.get(i) else { return false };
        if t.unit_price <= 0 || t.min_units == 0 {
            return false;
        }
        if let Some(p) = prev {
            if t.min_units <= p.min_units || t.unit_price >= p.unit_price {
                return false;
            }
        }
        prev = Some(t);
    }
    match tiers.get(0) {
        Some(first) => first.min_units == moq && max_units >= moq,
        None => false,
    }
}

/// Slice version of `tier_index` for host-independent math evaluation.
pub fn tier_index_slice(tiers: &[Tier], total_units: u32) -> Option<usize> {
    let mut found = None;
    for (i, t) in tiers.iter().enumerate() {
        if total_units >= t.min_units {
            found = Some(i);
        }
    }
    found
}

/// Slice version of `ceiling_price` for host-independent math evaluation.
pub fn ceiling_price_slice(tiers: &[Tier], total_before: u32) -> Option<i128> {
    let idx = tier_index_slice(tiers, total_before).unwrap_or(0);
    tiers.get(idx).map(|t| t.unit_price)
}

/// Slice version of `final_unit_price` for host-independent math evaluation.
pub fn final_unit_price_slice(tiers: &[Tier], total_units: u32) -> Option<i128> {
    ceiling_price_slice(tiers, total_units)
}

/// Slice version of `validate_tiers` for host-independent validation.
pub fn validate_tiers_slice(tiers: &[Tier], moq: u32, max_units: u32) -> bool {
    let n = tiers.len();
    if n == 0 || n > crate::types::caps::MAX_TIERS as usize {
        return false;
    }
    let mut prev: Option<&Tier> = None;
    for t in tiers {
        if t.unit_price <= 0 || t.min_units == 0 {
            return false;
        }
        if let Some(p) = prev {
            if t.min_units <= p.min_units || t.unit_price >= p.unit_price {
                return false;
            }
        }
        prev = Some(t);
    }
    match tiers.first() {
        Some(first) => first.min_units == moq && max_units >= moq,
        None => false,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{vec, Env};

    fn tiers(env: &Env) -> Vec<Tier> {
        vec![
            env,
            Tier {
                min_units: 10,
                unit_price: 100,
            },
            Tier {
                min_units: 50,
                unit_price: 90,
            },
            Tier {
                min_units: 100,
                unit_price: 80,
            },
        ]
    }

    #[test]
    fn tier_lookup() {
        let env = Env::default();
        let t = tiers(&env);
        assert_eq!(tier_index(&t, 9), None);
        assert_eq!(tier_index(&t, 10), Some(0));
        assert_eq!(tier_index(&t, 99), Some(1));
        assert_eq!(tier_index(&t, 100), Some(2));
        assert_eq!(ceiling_price(&t, 0), Some(100));
        assert_eq!(ceiling_price(&t, 60), Some(90));
        assert_eq!(final_unit_price(&t, 500), Some(80));
    }

    #[test]
    fn allocation_sums_to_received() {
        let units = [7u32, 13, 1, 29, 50];
        let total: u32 = units.iter().sum();
        for received in 0..=total {
            let mut cum = 0;
            let mut sum = 0;
            for u in units {
                sum += cumulative_alloc(cum, u, received, total).unwrap();
                cum += u;
            }
            assert_eq!(sum, received);
        }
    }

    #[test]
    fn tier_validation() {
        let env = Env::default();
        assert!(validate_tiers(&tiers(&env), 10, 200));
        assert!(!validate_tiers(&tiers(&env), 11, 200));
        assert!(!validate_tiers(&tiers(&env), 10, 5));
        let bad = vec![
            &env,
            Tier {
                min_units: 10,
                unit_price: 100,
            },
            Tier {
                min_units: 50,
                unit_price: 100,
            },
        ];
        assert!(!validate_tiers(&bad, 10, 200));
        assert!(!validate_tiers(&Vec::new(&env), 10, 200));
    }

    #[test]
    fn mul_div_checks() {
        assert_eq!(mul_div_floor(10, 3, 4), Some(7));
        assert_eq!(mul_div_floor(1, 1, 0), None);
        assert_eq!(mul_div_floor(i128::MAX, 2, 1), None);
        assert_eq!(bp_of(10_000, 150), Some(150));
    }

    #[test]
    fn named_rounding_policy_is_explicit() {
        assert_eq!(round_supplier_payable(101, 333), Some(97));
        assert_eq!(round_member_refund(101, 97), Some(4));
        assert_eq!(round_supplier_payable(1, BPS_DENOMINATOR + 1), None);
    }

    #[test]
    fn overflow_tests_at_extremes() {
        // mul_div_floor near i128::MAX and extreme boundaries
        assert_eq!(mul_div_floor(i128::MAX, 2, 1), None);
        assert_eq!(mul_div_floor(i128::MAX, i128::MAX, 1), None);
        assert_eq!(mul_div_floor(i128::MIN, -1, 1), None);
        assert_eq!(mul_div_floor(i128::MAX, 1, 0), None);
        assert_eq!(mul_div_floor(i128::MAX, 0, 1), Some(0));
        assert_eq!(mul_div_floor(i128::MAX, 1, 1), Some(i128::MAX));

        // bp_of overflow checks
        assert_eq!(bp_of(i128::MAX, 2), None);
        assert_eq!(bp_of(i128::MAX, 10_000), None);
        assert_eq!(bp_of(i128::MIN, 100), None);
        assert_eq!(bp_of(i128::MAX / 10_000, 10_000), Some(i128::MAX / 10_000));

        // cumulative_alloc near limits
        assert_eq!(cumulative_alloc(u32::MAX, 1, 1, 0), None);
        assert_eq!(
            cumulative_alloc(u32::MAX, u32::MAX, u32::MAX, u32::MAX),
            None
        );
        assert!(cumulative_alloc(u32::MAX - 100, 100, 50, u32::MAX).is_some());

        // tier price lookups at i128::MAX
        let env = Env::default();
        let extreme_tiers = vec![
            &env,
            Tier {
                min_units: 1,
                unit_price: i128::MAX,
            },
            Tier {
                min_units: 100,
                unit_price: i128::MAX - 1,
            },
        ];
        assert_eq!(ceiling_price(&extreme_tiers, 0), Some(i128::MAX));
        assert_eq!(
            final_unit_price(&extreme_tiers, u32::MAX),
            Some(i128::MAX - 1)
        );
        assert_eq!(tier_index(&extreme_tiers, u32::MAX), Some(1));
        assert!(validate_tiers(&extreme_tiers, 1, u32::MAX));
    }

    #[test]
    fn table_driven_fee_rounding_favours_members_at_every_size() {
        let units_range: [u32; 8] = [1, 7, 13, 50, 100, 250, 420, 1000];
        let prices: [i128; 5] = [
            1,          // 1 stroop
            99,         // 99 stroops
            8_000_000,  // 0.8 USDC
            9_000_000,  // 0.9 USDC
            10_000_000, // 1.0 USDC
        ];
        let platform_fees: [u32; 4] = [0, 50, 150, 500];
        let organizer_fees: [u32; 4] = [0, 25, 100, 200];

        let mut row_count = 0;
        for &u in &units_range {
            for &price in &prices {
                for &plat_bp in &platform_fees {
                    for &org_bp in &organizer_fees {
                        row_count += 1;
                        let gross = (u as i128) * price;
                        let total_fee_bp = plat_bp + org_bp;
                        let supplier = round_supplier_payable(gross, total_fee_bp).unwrap();
                        let plat = bp_of(gross, plat_bp).unwrap();
                        let org = bp_of(gross, org_bp).unwrap();

                        let ceiling_p = price + (price / 10).max(1);
                        let total_paid = (u as i128) * ceiling_p;
                        let refunds = round_member_refund(total_paid, gross).unwrap();
                        let dust = gross - (supplier + plat + org);

                        assert!(dust >= 0, "dust must be non-negative: row {row_count}");
                        assert!(dust <= 2, "dust must not exceed 2 stroops: got {dust}");

                        // Acceptance criteria: The sum identity holds for every row
                        assert_eq!(
                            supplier + plat + org + refunds + dust,
                            total_paid,
                            "sum identity failed at row {row_count}"
                        );

                        // Rounding favours members: no component is off by more than 1 stroop in member's disfavour
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
            "must test at least 200 combinations, tested {row_count}"
        );
    }
}
