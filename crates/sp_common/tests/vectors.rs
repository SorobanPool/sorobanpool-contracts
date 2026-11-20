//! Generates `vectors/pricing-vectors.json` from the contract math. The backend quote logic and
//! the frontend display logic must reproduce every vector exactly. Run with
//! `UPDATE_VECTORS=1 cargo test -p sp_common --test vectors` after changing the math.
use sp_common::math::{
    ceiling_price_slice, cumulative_alloc, final_unit_price_slice, tier_index_slice,
    validate_tiers_slice,
};
use sp_common::types::Tier;
use std::fmt::Write;

struct Lcg(u64);
impl Lcg {
    fn next(&mut self, lo: u64, hi: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        lo + (self.0 >> 33) % (hi - lo + 1)
    }
}

fn emit_vector(group: &str, tiers: &[Tier], moq: u32, units: &[u32], received: u32) -> String {
    let mut tiers_json = String::new();
    for (i, t) in tiers.iter().enumerate() {
        let _ = write!(
            tiers_json,
            "{}[{},{}]",
            if i > 0 { "," } else { "" },
            t.min_units,
            t.unit_price
        );
    }
    let mut total = 0u32;
    let mut paid = std::vec::Vec::new();
    for u in units {
        let p = ceiling_price_slice(tiers, total).unwrap() * (*u as i128);
        paid.push(p);
        total += u;
    }
    let filled = total >= moq;
    let final_price = if filled {
        final_unit_price_slice(tiers, total).unwrap()
    } else {
        0
    };
    let mut cum = 0;
    let mut alloc = std::vec::Vec::new();
    let mut refunds = std::vec::Vec::new();
    for (i, u) in units.iter().enumerate() {
        if filled {
            let a = cumulative_alloc(cum, *u, received, total).unwrap();
            alloc.push(a);
            refunds.push(paid[i] - a as i128 * final_price);
        } else {
            alloc.push(0);
            refunds.push(paid[i]);
        }
        cum += u;
    }
    let tier = tier_index_slice(tiers, total)
        .map(|t| t as i64)
        .unwrap_or(-1);
    let list = |v: &[String]| v.join(",");
    format!(
        "{{\"group\":\"{}\",\"tiers\":[{}],\"moq\":{},\"units\":[{}],\"paid\":[\"{}\"],\"total\":{},\"tier\":{},\"filled\":{},\"finalPrice\":\"{}\",\"received\":{},\"alloc\":[{}],\"refunds\":[\"{}\"]}}",
        group,
        tiers_json,
        moq,
        list(&units.iter().map(|x| x.to_string()).collect::<std::vec::Vec<_>>()),
        paid.iter().map(|x| x.to_string()).collect::<std::vec::Vec<_>>().join("\",\""),
        total,
        tier,
        filled,
        final_price,
        received,
        list(&alloc.iter().map(|x| x.to_string()).collect::<std::vec::Vec<_>>()),
        refunds.iter().map(|x| x.to_string()).collect::<std::vec::Vec<_>>().join("\",\""),
    )
}

fn generate() -> String {
    let mut rng = Lcg(42);
    let mut out = String::from("{\"version\":1,\"vectors\":[\n");
    let mut count = 0;

    // 1. 600 base vectors
    let mut n = 0;
    while n < 600 {
        let ntiers = rng.next(1, 5) as usize;
        let moq = rng.next(5, 60) as u32;
        let mut min_units = moq;
        let mut price = rng.next(5_000_000, 50_000_000) as i128;
        let mut tiers: std::vec::Vec<Tier> = std::vec::Vec::new();
        for _ in 0..ntiers {
            tiers.push(Tier {
                min_units,
                unit_price: price,
            });
            min_units += rng.next(5, 80) as u32;
            price -= rng.next(1, (price as u64 / 6).max(2)) as i128;
        }
        let max_units = min_units + 100;
        if !validate_tiers_slice(&tiers, moq, max_units) {
            continue;
        }
        let members = rng.next(1, 8) as usize;
        let mut total = 0u32;
        let mut units = std::vec::Vec::new();
        for _ in 0..members {
            let u = rng.next(1, 70) as u32;
            if total + u > max_units {
                break;
            }
            units.push(u);
            total += u;
        }
        if units.is_empty() {
            continue;
        }
        let filled = total >= moq;
        let received = if filled {
            rng.next(0, total as u64) as u32
        } else {
            0
        };
        let line = emit_vector("random", &tiers, moq, &units, received);
        let _ = writeln!(out, "{}{}", if count > 0 { "," } else { "" }, line);
        count += 1;
        n += 1;
    }

    // 2. Edge group: single_unit (25 vectors)
    let mut n = 0;
    while n < 25 {
        let moq = 1;
        let ntiers = rng.next(1, 3) as usize;
        let mut min_units = 1;
        let mut price = rng.next(10_000_000, 30_000_000) as i128;
        let mut tiers = std::vec::Vec::new();
        for _ in 0..ntiers {
            tiers.push(Tier {
                min_units,
                unit_price: price,
            });
            min_units += rng.next(2, 10) as u32;
            price -= rng.next(500_000, 2_000_000) as i128;
        }
        let max_units = 10;
        if !validate_tiers_slice(&tiers, moq, max_units) {
            continue;
        }
        let member_count = rng.next(1, 2) as usize;
        let units: std::vec::Vec<u32> = (0..member_count).map(|_| 1).collect();
        let total: u32 = units.iter().sum();
        let received = if rng.next(0, 1) == 1 { total } else { 0 };
        let line = emit_vector("single_unit", &tiers, moq, &units, received);
        let _ = writeln!(out, "{}{}", if count > 0 { "," } else { "" }, line);
        count += 1;
        n += 1;
    }

    // 3. Edge group: five_tier_equal_gaps (25 vectors)
    let mut n = 0;
    while n < 25 {
        let moq = rng.next(10, 50) as u32;
        let gap = rng.next(10, 40) as u32;
        let price_step = rng.next(1_000_000, 5_000_000) as i128;
        let base_price = 50_000_000i128;
        let mut tiers = std::vec::Vec::new();
        for i in 0..5 {
            tiers.push(Tier {
                min_units: moq + i as u32 * gap,
                unit_price: base_price - i as i128 * price_step,
            });
        }
        let max_units = moq + 4 * gap + 100;
        if !validate_tiers_slice(&tiers, moq, max_units) {
            continue;
        }
        let members = rng.next(2, 6) as usize;
        let mut units = std::vec::Vec::new();
        let mut total = 0;
        for _ in 0..members {
            let u = rng.next(5, 50) as u32;
            if total + u > max_units {
                break;
            }
            units.push(u);
            total += u;
        }
        if units.is_empty() {
            continue;
        }
        let received = rng.next(0, total as u64) as u32;
        let line = emit_vector("five_tier_equal_gaps", &tiers, moq, &units, received);
        let _ = writeln!(out, "{}{}", if count > 0 { "," } else { "" }, line);
        count += 1;
        n += 1;
    }

    // 4. Edge group: maximum_size (25 vectors)
    let mut n = 0;
    while n < 25 {
        let moq = 50;
        let tiers = vec![
            Tier {
                min_units: 50,
                unit_price: 30_000_000,
            },
            Tier {
                min_units: 150,
                unit_price: 25_000_000,
            },
            Tier {
                min_units: 300,
                unit_price: 20_000_000,
            },
        ];
        let _max_units = 10_000;
        let members = rng.next(50, 150) as usize; // high member count
        let mut units = std::vec::Vec::new();
        let mut total = 0;
        for _ in 0..members {
            let u = rng.next(1, 10) as u32;
            units.push(u);
            total += u;
        }
        let received = (total as u64 * rng.next(50, 100) / 100) as u32;
        let line = emit_vector("maximum_size", &tiers, moq, &units, received);
        let _ = writeln!(out, "{}{}", if count > 0 { "," } else { "" }, line);
        count += 1;
        n += 1;
    }

    // 5. Edge group: shortfall_1_unit (25 vectors)
    let mut n = 0;
    while n < 25 {
        let ntiers = rng.next(2, 4) as usize;
        let moq = rng.next(10, 30) as u32;
        let mut min_units = moq;
        let mut price = 30_000_000i128;
        let mut tiers = std::vec::Vec::new();
        for _ in 0..ntiers {
            tiers.push(Tier {
                min_units,
                unit_price: price,
            });
            min_units += rng.next(15, 40) as u32;
            price -= rng.next(2_000_000, 5_000_000) as i128;
        }
        let _max_units = min_units + 200;
        let members = rng.next(2, 7) as usize;
        let mut units = std::vec::Vec::new();
        let mut total = 0;
        for _ in 0..members {
            let u = rng.next(5, 30) as u32;
            units.push(u);
            total += u;
        }
        if total < moq || total < 2 {
            continue;
        }
        // Exactly 1 unit shortfall
        let received = total - 1;
        let line = emit_vector("shortfall_1_unit", &tiers, moq, &units, received);
        let _ = writeln!(out, "{}{}", if count > 0 { "," } else { "" }, line);
        count += 1;
        n += 1;
    }

    out.push_str("]}\n");
    out
}

#[test]
fn vectors_are_current() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vectors/pricing-vectors.json"
    );
    let fresh = generate();
    if std::env::var("UPDATE_VECTORS").is_ok() {
        std::fs::write(path, &fresh).unwrap();
    }
    let on_disk =
        std::fs::read_to_string(path).expect("run with UPDATE_VECTORS=1 to create the file");
    assert_eq!(
        on_disk, fresh,
        "pricing-vectors.json is stale; regenerate with UPDATE_VECTORS=1"
    );
    assert!(fresh.matches("\"tiers\"").count() >= 700);
    assert!(fresh.matches("\"group\":\"single_unit\"").count() >= 25);
    assert!(fresh.matches("\"group\":\"five_tier_equal_gaps\"").count() >= 25);
    assert!(fresh.matches("\"group\":\"maximum_size\"").count() >= 25);
    assert!(fresh.matches("\"group\":\"shortfall_1_unit\"").count() >= 25);
}
