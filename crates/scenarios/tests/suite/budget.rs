//! Resource budget (brief 6.8) at the worst case of 200 members.
//! Limits are the conservative per-transaction figures: 100M CPU instructions, 40 MiB memory.
//! Native test execution under-reports VM costs, so these are regression guards, not a substitute for simulation.
use scenarios::*;
use soroban_sdk::Address;

const CPU_LIMIT: u64 = 100_000_000;
const MEM_LIMIT: u64 = 40 * 1024 * 1024;
const MEMBERS: usize = 200;

fn measure<R>(c: &Ctx, label: &str, pct: u64, f: impl FnOnce() -> R) -> R {
    let mut b = c.env.cost_estimate().budget();
    b.reset_tracker();
    let r = f();
    let (cpu, mem) = (b.cpu_instruction_cost(), b.memory_bytes_cost());
    println!(
        "{label}: cpu {cpu} ({}%), mem {mem} ({}%)",
        cpu * 100 / CPU_LIMIT,
        mem * 100 / MEM_LIMIT
    );
    assert!(cpu < CPU_LIMIT * pct / 100, "{label} cpu {cpu} over {pct}%");
    assert!(mem < MEM_LIMIT * pct / 100, "{label} mem {mem} over {pct}%");
    r
}

#[test]
fn worst_case_200_members_stays_within_budget() {
    let c = setup();
    let mut b = c.env.cost_estimate().budget();
    b.reset_unlimited();
    let pool = c.pool(100);
    let members: Vec<Address> = (0..MEMBERS).map(|_| c.trader()).collect();
    for m in &members[..MEMBERS - 1] {
        c.gb.commit(m, &pool, &2);
    }
    let last = &members[MEMBERS - 1];
    measure(&c, "commit", 50, || c.gb.commit(last, &pool, &2));
    c.advance(3 * DAY);
    c.gb.close(&pool);
    c.deliver(pool, 400);
    for m in &members {
        c.gb.confirm_pickup(m, &pool);
    }
    measure(&c, "settle", 50, || c.gb.settle(&pool));
    measure(&c, "claim_refund", 50, || {
        c.gb.claim_refund(&members[0], &pool)
    });
    let pushed = measure(&c, "push_refunds(25)", 80, || c.gb.push_refunds(&pool, &25));
    assert_eq!(pushed, 25);
}
