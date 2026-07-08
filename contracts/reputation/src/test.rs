use super::*;
use config::{Config, ConfigClient};
use soroban_sdk::testutils::{Address as _, Ledger};

fn setup() -> (Env, ReputationClient<'static>, Address) {
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
    (env.clone(), ReputationClient::new(&env, &id), gb)
}

fn rec(c: &ReputationClient, gb: &Address, who: &Address, role: Role, ev: Symbol, n: u32) {
    for _ in 0..n {
        c.record(gb, who, &role, &ev);
    }
}

#[test]
fn only_pool_contracts_may_record() {
    let (env, c, _) = setup();
    let stranger = Address::generate(&env);
    let who = Address::generate(&env);
    assert!(c
        .try_record(&stranger, &who, &Role::Supplier, &symbol_short!("settled"))
        .is_err());
}

#[test]
fn unknown_event_rejected() {
    let (env, c, gb) = setup();
    let who = Address::generate(&env);
    assert!(c
        .try_record(&gb, &who, &Role::Supplier, &symbol_short!("bogus"))
        .is_err());
}

#[test]
fn supplier_tiers() {
    let (env, c, gb) = setup();
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
    let (env, c, gb) = setup();
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
