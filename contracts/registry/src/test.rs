use super::*;
use config::{Config, ConfigClient};
use soroban_sdk::testutils::Address as _;

struct Ctx {
    env: Env,
    reg: RegistryClient<'static>,
    cfg: ConfigClient<'static>,
    admin: Address,
}

fn setup() -> Ctx {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let cfg_id = env.register(
        Config,
        (&admin, Address::generate(&env), Address::generate(&env)),
    );
    let reg_id = env.register(Registry, (&cfg_id,));
    Ctx {
        reg: RegistryClient::new(&env, &reg_id),
        cfg: ConfigClient::new(&env, &cfg_id),
        env,
        admin,
    }
}

fn h(env: &Env, b: u8) -> BytesN<32> {
    BytesN::from_array(env, &[b; 32])
}

#[test]
fn register_attest_suspend_cycle() {
    let c = setup();
    let user = Address::generate(&c.env);
    let attestor = Address::generate(&c.env);
    assert_eq!(c.reg.status(&user, &Role::Trader), Status::Unregistered);
    c.reg.register(
        &user,
        &Role::Trader,
        &h(&c.env, 1),
        &Some(symbol_short!("wuse")),
    );
    assert_eq!(c.reg.status(&user, &Role::Trader), Status::Registered);
    assert_eq!(c.reg.cluster_of(&user), Some(symbol_short!("wuse")));
    assert!(c
        .reg
        .try_register(&user, &Role::Trader, &h(&c.env, 1), &None)
        .is_err());

    assert!(c
        .reg
        .try_attest(&attestor, &user, &Role::Trader, &h(&c.env, 2), &1)
        .is_err());
    c.cfg.set_attestor(&attestor, &true);
    assert!(c
        .reg
        .try_attest(&attestor, &user, &Role::Trader, &h(&c.env, 2), &0)
        .is_err());
    c.reg
        .attest(&attestor, &user, &Role::Trader, &h(&c.env, 2), &1);
    assert_eq!(c.reg.status(&user, &Role::Trader), Status::Verified);
    assert_eq!(c.reg.level(&user, &Role::Trader), 1);

    let stranger = Address::generate(&c.env);
    assert!(c
        .reg
        .try_suspend(&stranger, &user, &Role::Trader, &symbol_short!("fraud"))
        .is_err());
    c.reg
        .suspend(&c.admin, &user, &Role::Trader, &symbol_short!("fraud"));
    assert_eq!(c.reg.status(&user, &Role::Trader), Status::Suspended);
    assert!(c
        .reg
        .try_attest(&attestor, &user, &Role::Trader, &h(&c.env, 3), &2)
        .is_err());
    c.reg.unsuspend(&c.admin, &user, &Role::Trader);
    assert_eq!(c.reg.status(&user, &Role::Trader), Status::Verified);

    c.reg
        .revoke(&c.admin, &user, &Role::Trader, &symbol_short!("kyc_bad"));
    assert_eq!(c.reg.status(&user, &Role::Trader), Status::Registered);
    assert_eq!(c.reg.level(&user, &Role::Trader), 0);
}

#[test]
fn roles_are_independent() {
    let c = setup();
    let user = Address::generate(&c.env);
    c.reg.register(&user, &Role::Trader, &h(&c.env, 1), &None);
    assert_eq!(c.reg.status(&user, &Role::Supplier), Status::Unregistered);
    assert!(c.reg.try_unsuspend(&c.admin, &user, &Role::Trader).is_err());
}
