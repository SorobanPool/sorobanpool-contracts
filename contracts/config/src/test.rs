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

#[test]
fn snapshot_matches_individual_views() {
    let (e, c, admin) = setup();
    // un-paused baseline: snapshot params equal get_params, all flags false
    let snap = c.snapshot();
    let p = c.get_params();
    assert_eq!(snap.params.platform_fee_bp, p.platform_fee_bp);
    assert_eq!(snap.params.max_members_per_pool, p.max_members_per_pool);
    assert!(!snap.paused_all);
    assert!(!snap.paused_create);
    assert!(!snap.paused_commit);
    assert!(!snap.paused_accept);
    assert_eq!(snap.paused_create, c.is_paused(&symbol_short!("create")));
    assert_eq!(snap.paused_commit, c.is_paused(&symbol_short!("commit")));
    assert_eq!(snap.paused_accept, c.is_paused(&symbol_short!("accept")));

    // pause "commit" individually
    c.pause(&admin, &symbol_short!("commit"));
    let snap2 = c.snapshot();
    assert!(!snap2.paused_all);
    assert!(!snap2.paused_create);
    assert!(snap2.paused_commit);
    assert!(!snap2.paused_accept);
    assert_eq!(snap2.paused_commit, c.is_paused(&symbol_short!("commit")));
    assert_eq!(snap2.paused_create, c.is_paused(&symbol_short!("create")));

    // pause "all" — all scope flags should become true
    c.pause(&admin, &symbol_short!("all"));
    let snap3 = c.snapshot();
    assert!(snap3.paused_all);
    assert!(snap3.paused_create);
    assert!(snap3.paused_commit);
    assert!(snap3.paused_accept);

    // unpause "all": flags go back to their own storage values
    c.unpause(&symbol_short!("all"));
    let snap4 = c.snapshot();
    assert!(!snap4.paused_all);
    assert!(!snap4.paused_create);
    assert!(snap4.paused_commit); // still paused from earlier
    assert!(!snap4.paused_accept);

    // params roundtrip through set_params
    let mut np = c.get_params();
    np.platform_fee_bp = 200;
    c.set_params(&np);
    let snap5 = c.snapshot();
    assert_eq!(snap5.params.platform_fee_bp, 200);
    assert_eq!(snap5.params.platform_fee_bp, c.get_params().platform_fee_bp);
    // pause flags still match after params change
    assert_eq!(snap5.paused_commit, c.is_paused(&symbol_short!("commit")));
    assert_eq!(snap5.paused_create, c.is_paused(&symbol_short!("create")));
}

#[test]
fn attestor_and_arbiter_list_views_and_limits() {
    let (e, c, _) = setup();
    assert_eq!(c.attestors().len(), 0);
    assert_eq!(c.arbiters().len(), 0);

    let a1 = Address::generate(&e);
    let a2 = Address::generate(&e);

    // 1. Add attestor and arbiter
    c.set_attestor(&a1, &true);
    c.set_arbiter(&a1, &true);
    assert_eq!(c.attestors().len(), 1);
    assert_eq!(c.arbiters().len(), 1);
    assert_eq!(c.attestors().get(0).unwrap(), a1);
    assert_eq!(c.arbiters().get(0).unwrap(), a1);

    // 2. Duplicate add does not duplicate entry
    c.set_attestor(&a1, &true);
    c.set_arbiter(&a1, &true);
    assert_eq!(c.attestors().len(), 1);
    assert_eq!(c.arbiters().len(), 1);

    // 3. Add second entry
    c.set_attestor(&a2, &true);
    c.set_arbiter(&a2, &true);
    assert_eq!(c.attestors().len(), 2);
    assert_eq!(c.arbiters().len(), 2);

    // 4. Disable removes entry
    c.set_attestor(&a1, &false);
    c.set_arbiter(&a1, &false);
    assert!(!c.is_attestor(&a1));
    assert!(!c.is_arbiter(&a1));
    assert_eq!(c.attestors().len(), 1);
    assert_eq!(c.arbiters().len(), 1);
    assert_eq!(c.attestors().get(0).unwrap(), a2);
    assert_eq!(c.arbiters().get(0).unwrap(), a2);

    // 5. Size limits: MAX_ATTESTORS and MAX_ARBITERS
    for _ in 1..MAX_ATTESTORS {
        let addr = Address::generate(&e);
        c.set_attestor(&addr, &true);
    }
    assert_eq!(c.attestors().len(), MAX_ATTESTORS);
    let extra_attestor = Address::generate(&e);
    assert!(c.try_set_attestor(&extra_attestor, &true).is_err());

    for _ in 1..MAX_ARBITERS {
        let addr = Address::generate(&e);
        c.set_arbiter(&addr, &true);
    }
    assert_eq!(c.arbiters().len(), MAX_ARBITERS);
    let extra_arbiter = Address::generate(&e);
    assert!(c.try_set_arbiter(&extra_arbiter, &true).is_err());
}
