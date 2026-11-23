use super::*;
use config::{Config, ConfigClient};
use soroban_sdk::testutils::{Address as _, Ledger};

fn setup() -> (Env, ReputationClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);
    let admin = Address::generate(&env);
    let cfg_id = env.register(
        Config,
        (&admin, Address::generate(&env), Address::generate(&env)),
    );
    let cfg = ConfigClient::new(&env, &cfg_id);
    let gb = Address::generate(&env);
    cfg.set_address(&keys::GROUP_BUY, &gb);
    cfg.set_address(&keys::DISPUTES, &Address::generate(&env));
    let id = env.register(Reputation, (&cfg_id,));
    (env.clone(), ReputationClient::new(&env, &id), gb, admin)
}

/// Same wiring as `setup`, but without mocked authorisations so that
/// `require_auth` failures surface.
fn setup_unmocked() -> (Env, ReputationClient<'static>, Address) {
    let env = Env::default();
    env.ledger().set_timestamp(1_000);
    let admin = Address::generate(&env);
    let cfg_id = env.register(
        Config,
        (&admin, Address::generate(&env), Address::generate(&env)),
    );
    let cfg = ConfigClient::new(&env, &cfg_id);
    let gb = Address::generate(&env);
    env.mock_all_auths();
    cfg.set_address(&keys::GROUP_BUY, &gb);
    cfg.set_address(&keys::DISPUTES, &Address::generate(&env));
    env.mock_auths(&[]);
    let id = env.register(Reputation, (&cfg_id,));
    (env.clone(), ReputationClient::new(&env, &id), gb)
}

fn rec(c: &ReputationClient, gb: &Address, who: &Address, role: Role, ev: Symbol, n: u32) {
    for _ in 0..n {
        c.record(gb, who, &role, &ev);
    }
}

#[test]
fn only_pool_contracts_may_record() {
    let (env, c, _, _) = setup();
    let stranger = Address::generate(&env);
    let who = Address::generate(&env);
    assert!(c
        .try_record(&stranger, &who, &Role::Supplier, &symbol_short!("settled"))
        .is_err());
}

#[test]
fn registered_caller_without_auth_is_rejected() {
    let (env, c, gb) = setup_unmocked();
    let who = Address::generate(&env);
    assert!(c
        .try_record(&gb, &who, &Role::Supplier, &symbol_short!("settled"))
        .is_err());
}

#[test]
fn authorised_registered_caller_records() {
    let (env, c, gb, _) = setup();
    let who = Address::generate(&env);
    c.record(&gb, &who, &Role::Supplier, &symbol_short!("settled"));
    assert_eq!(c.stats(&who, &Role::Supplier).pools_settled, 1);
}

#[test]
fn unknown_event_rejected() {
    let (env, c, gb, _) = setup();
    let who = Address::generate(&env);
    assert!(c
        .try_record(&gb, &who, &Role::Supplier, &symbol_short!("bogus"))
        .is_err());
}

#[test]
fn supplier_tiers() {
    let (env, c, gb, _) = setup();
    let s = Address::generate(&env);
    assert_eq!(c.tier(&s, &Role::Supplier), 0);
    rec(&c, &gb, &s, Role::Supplier, symbol_short!("settled"), 3);
    assert_eq!(c.tier(&s, &Role::Supplier), 1);
    rec(&c, &gb, &s, Role::Supplier, symbol_short!("d_lost"), 1);
    assert_eq!(c.tier(&s, &Role::Supplier), 0); // a lost dispute blocks S1
    let s2 = Address::generate(&env);
    rec(&c, &gb, &s2, Role::Supplier, symbol_short!("settled"), 15);
    rec(&c, &gb, &s2, Role::Supplier, symbol_short!("on_time"), 14);
    rec(&c, &gb, &s2, Role::Supplier, symbol_short!("late"), 1);
    assert_eq!(c.tier(&s2, &Role::Supplier), 2); // 93% on time, 0% lost
    rec(&c, &gb, &s2, Role::Supplier, symbol_short!("settled"), 35);
    rec(&c, &gb, &s2, Role::Supplier, symbol_short!("on_time"), 35);
    assert_eq!(c.tier(&s2, &Role::Supplier), 2); // not yet 6 months
    env.ledger().set_timestamp(1_000 + 7 * 30 * 24 * 3600);
    assert_eq!(c.tier(&s2, &Role::Supplier), 3);
}

#[test]
fn organizer_and_trader_tiers() {
    let (env, c, gb, _) = setup();
    let o = Address::generate(&env);
    rec(&c, &gb, &o, Role::Organizer, symbol_short!("settled"), 3);
    assert_eq!(c.tier(&o, &Role::Organizer), 1);
    rec(&c, &gb, &o, Role::Organizer, symbol_short!("settled"), 12);
    assert_eq!(c.tier(&o, &Role::Organizer), 2);
    let t = Address::generate(&env);
    rec(&c, &gb, &t, Role::Trader, symbol_short!("settled"), 4);
    assert_eq!(c.tier(&t, &Role::Trader), 0);
    rec(&c, &gb, &t, Role::Trader, symbol_short!("settled"), 1);
    assert_eq!(c.tier(&t, &Role::Trader), 2);
    assert_eq!(c.stats(&t, &Role::Trader).first_active_at, 1_000);
}

#[test]
fn correction_is_admin_only_and_bounded() {
    let (env, c, gb, admin) = setup();
    let who = Address::generate(&env);
    rec(&c, &gb, &who, Role::Supplier, symbol_short!("settled"), 2);
    c.correct(
        &admin,
        &who,
        &Role::Supplier,
        &symbol_short!("settled"),
        &1,
        &symbol_short!("fix"),
    );
    assert_eq!(c.stats(&who, &Role::Supplier).pools_settled, 1);
    let stranger = Address::generate(&env);
    assert!(c
        .try_correct(
            &stranger,
            &who,
            &Role::Supplier,
            &symbol_short!("settled"),
            &1,
            &symbol_short!("fix")
        )
        .is_err());
    assert!(c
        .try_correct(
            &admin,
            &who,
            &Role::Supplier,
            &symbol_short!("settled"),
            &1001,
            &symbol_short!("fix")
        )
        .is_err());
    assert!(c
        .try_correct(
            &admin,
            &who,
            &Role::Supplier,
            &symbol_short!("settled"),
            &2,
            &symbol_short!("fix")
        )
        .is_err());
}

#[test]
fn recording_at_u32_max_saturates() {
    let (env, c, gb, _) = setup();
    let who = Address::generate(&env);
    let seeded = RepStats {
        pools_settled: u32::MAX,
        ..Default::default()
    };
    env.as_contract(&c.address, || {
        env.storage()
            .persistent()
            .set(&Key::Stats(who.clone(), Role::Supplier), &seeded);
    });
    c.record(&gb, &who, &Role::Supplier, &symbol_short!("settled"));
    assert_eq!(c.stats(&who, &Role::Supplier).pools_settled, u32::MAX);
}

#[test]
fn tier_threshold_boundaries_are_explicit() {
    const SUPPLIER_S2_SETTLED: u32 = 15;
    const ORGANIZER_S2_SETTLED: u32 = 15;
    const TRADER_S2_SETTLED: u32 = 5;
    const LOST_BP_LIMIT: u32 = 500;
    const ON_TIME_BP_LIMIT: u32 = 9_000;

    let supplier_cases = [
        (SUPPLIER_S2_SETTLED - 1, 0, 100, 0, 1),
        (SUPPLIER_S2_SETTLED, 0, 90, 10, 2),
        (1_000, LOST_BP_LIMIT / 10 - 1, ON_TIME_BP_LIMIT / 10, 100, 2),
        (1_000, LOST_BP_LIMIT / 10, ON_TIME_BP_LIMIT / 10, 100, 0),
        (1_000, 0, ON_TIME_BP_LIMIT / 10 - 1, 101, 1),
        (1_000, 0, ON_TIME_BP_LIMIT / 10, 100, 2),
    ];
    for (settled, lost, on_time, late, expected) in supplier_cases {
        let stats = RepStats {
            pools_settled: settled,
            disputes_lost: lost,
            on_time,
            late,
            ..Default::default()
        };
        assert_eq!(compute_tier(Role::Supplier, &stats, 0), expected);
    }

    let organizer_cases = [
        (ORGANIZER_S2_SETTLED - 1, 0, 1),
        (ORGANIZER_S2_SETTLED, 0, 2),
        (1_000, LOST_BP_LIMIT / 10 - 1, 2),
        (1_000, LOST_BP_LIMIT / 10, 0),
    ];
    for (settled, lost, expected) in organizer_cases {
        let stats = RepStats {
            pools_settled: settled,
            disputes_lost: lost,
            ..Default::default()
        };
        assert_eq!(compute_tier(Role::Organizer, &stats, 0), expected);
    }

    let trader_cases = [
        (TRADER_S2_SETTLED - 1, 0, 0),
        (TRADER_S2_SETTLED, 0, 2),
        (TRADER_S2_SETTLED, 1, 0),
    ];
    for (settled, lost, expected) in trader_cases {
        let stats = RepStats {
            pools_settled: settled,
            disputes_lost: lost,
            ..Default::default()
        };
        assert_eq!(compute_tier(Role::Trader, &stats, 0), expected);
    }
}
