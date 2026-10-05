use super::*;
use soroban_sdk::testutils::Address as _;

fn setup() -> (Env, ConfigClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let id = env.register(
        Config,
        (&admin, Address::generate(&env), Address::generate(&env)),
    );
    (env.clone(), ConfigClient::new(&env, &id), admin)
}

#[test]
fn defaults_match_brief() {
    let (_e, c, _) = setup();
    let p = c.get_params();
    assert_eq!(p.platform_fee_bp, 150);
    assert!(!p.advance_enabled);
    assert_eq!(p.supplier_tier_caps.len(), 4);
}

#[test]
fn hard_caps_are_enforced() {
    let (_e, c, _) = setup();
    let mut p = c.get_params();
    p.platform_fee_bp = 501;
    assert!(c.try_set_params(&p).is_err());
    let mut p = c.get_params();
    p.max_fill_window_secs = 15 * 24 * 3600;
    assert!(c.try_set_params(&p).is_err());
    let mut p = c.get_params();
    p.supplier_advance_bp.set(2, 4001);
    assert!(c.try_set_params(&p).is_err());
    let mut p = c.get_params();
    p.platform_fee_bp = 500;
    c.set_params(&p);
    assert_eq!(c.get_params().platform_fee_bp, 500);
}

#[test]
fn pause_scopes_and_guardian() {
    let (e, c, admin) = setup();
    let guardian = Address::generate(&e);
    c.set_guardian(&guardian);
    let commit = symbol_short!("commit");
    assert!(!c.is_paused(&commit));
    c.pause(&guardian, &commit);
    assert!(c.is_paused(&commit));
    assert!(!c.is_paused(&symbol_short!("create")));
    c.unpause(&commit);
    assert!(!c.is_paused(&commit));
    c.pause(&admin, &symbol_short!("all"));
    assert!(c.is_paused(&symbol_short!("create")));
    let stranger = Address::generate(&e);
    assert!(c.try_pause(&stranger, &commit).is_err());
}

#[test]
fn registries() {
    let (e, c, _) = setup();
    let a = Address::generate(&e);
    assert!(!c.is_attestor(&a) && !c.is_arbiter(&a));
    c.set_attestor(&a, &true);
    c.set_arbiter(&a, &true);
    assert!(c.is_attestor(&a) && c.is_arbiter(&a));
    let cat = symbol_short!("rice");
    assert!(!c.is_category_allowed(&cat));
    c.set_category(&cat, &true);
    assert!(c.is_category_allowed(&cat));
    assert!(c.try_get_address(&symbol_short!("nope")).is_err());
    c.set_address(&symbol_short!("x"), &a);
    assert_eq!(c.get_address(&symbol_short!("x")), a);
}
