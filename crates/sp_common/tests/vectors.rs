//! Generates `vectors/pricing-vectors.json` from the contract math. The backend quote logic and
//! the frontend display logic must reproduce every vector exactly. Run with
//! `UPDATE_VECTORS=1 cargo test -p sp_common --test vectors` after changing the math.
use soroban_sdk::{Env, Vec};
use sp_common::math::{
    ceiling_price, cumulative_alloc, final_unit_price, tier_index, validate_tiers,
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

fn generate() -> String {
    let env = Env::default();
    let mut rng = Lcg(42);
    let mut out = String::from("{\"version\":1,\"vectors\":[\n");
    let mut n = 0;
    while n < 600 {
        let ntiers = rng.next(1, 5) as usize;
        let moq = rng.next(5, 60) as u32;
        let mut min_units = moq;
        let mut price = rng.next(5_000_000, 50_000_000) as i128;
        let mut raw: Vec<Tier> = Vec::new(&env);
        let mut tiers_json = String::new();
        for i in 0..ntiers {
            raw.push_back(Tier {
                min_units,
                unit_price: price,
            });
            let _ = write!(
                tiers_json,
                "{}[{},{}]",
                if i > 0 { "," } else { "" },
                min_units,
                price
            );
            min_units += rng.next(5, 80) as u32;
            price -= rng.next(1, (price as u64 / 6).max(2)) as i128;
        }
        let max_units = min_units + 100;
        if !validate_tiers(&raw, moq, max_units) {
            continue;
        }
        let members = rng.next(1, 8) as usize;
        let mut total = 0u32;
        let mut units = std::vec::Vec::new();
        let mut paid = std::vec::Vec::new();
        for _ in 0..members {
            let u = rng.next(1, 70) as u32;
            if total + u > max_units {
                break;
            }
            let p = ceiling_price(&raw, total).unwrap() * u as i128;
            units.push(u);
            paid.push(p);
            total += u;
        }
        if units.is_empty() {
            continue;
        }
        let filled = total >= moq;
        let final_price = if filled {
            final_unit_price(&raw, total).unwrap()
        } else {
            0
        };
        let received = if filled {
            rng.next(0, total as u64) as u32
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
        let tier = tier_index(&raw, total).map(|t| t as i64).unwrap_or(-1);
        let list = |v: &[String]| v.join(",");
        let _ = writeln!(
            out,
            "{}{{\"tiers\":[{}],\"moq\":{},\"units\":[{}],\"paid\":[\"{}\"],\"total\":{},\"tier\":{},\"filled\":{},\"finalPrice\":\"{}\",\"received\":{},\"alloc\":[{}],\"refunds\":[\"{}\"]}}",
            if n > 0 { "," } else { "" },
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
        );
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
    assert!(fresh.matches("\"tiers\"").count() >= 500);
}
