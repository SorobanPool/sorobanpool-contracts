use super::*;
use config::{Config, ConfigClient};
use soroban_sdk::testutils::Address as _;

struct Ctx {
    env: Env,
    bond: SupplierBondClient<'static>,
    gb: Address,
    usdc: token::Client<'static>,
    supplier: Address,
}

fn setup() -> Ctx {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(issuer);
    let cfg_id = env.register(Config, (&admin, sac.address(), Address::generate(&env)));
    let gb = Address::generate(&env);
    ConfigClient::new(&env, &cfg_id).set_address(&keys::GROUP_BUY, &gb);
    let bond_id = env.register(SupplierBond, (&cfg_id,));
    let supplier = Address::generate(&env);
    token::StellarAssetClient::new(&env, &sac.address()).mint(&supplier, &1_000);
    Ctx {
        bond: SupplierBondClient::new(&env, &bond_id),
        usdc: token::Client::new(&env, &sac.address()),
        env,
        gb,
        supplier,
    }
}

#[test]
fn deposit_withdraw_free_only() {
    let c = setup();
    c.bond.deposit(&c.supplier, &500);
    assert_eq!(c.bond.bond_of(&c.supplier), (500, 0));
    c.bond.reserve_for_advance(&c.gb, &c.supplier, &200);
    assert!(c.bond.try_withdraw(&c.supplier, &301).is_err());
    c.bond.withdraw(&c.supplier, &300);
    assert_eq!(c.usdc.balance(&c.supplier), 800);
    assert_eq!(c.bond.bond_of(&c.supplier), (200, 200));
}

#[test]
fn reserve_needs_free_bond_and_group_buy() {
    let c = setup();
    c.bond.deposit(&c.supplier, &100);
    assert!(c
        .bond
        .try_reserve_for_advance(&c.gb, &c.supplier, &101)
        .is_err());
    let stranger = Address::generate(&c.env);
    assert!(c
        .bond
        .try_reserve_for_advance(&stranger, &c.supplier, &10)
        .is_err());
    c.bond.reserve_for_advance(&c.gb, &c.supplier, &60);
    assert!(c.bond.try_release_advance(&c.gb, &c.supplier, &61).is_err());
    c.bond.release_advance(&c.gb, &c.supplier, &60);
    assert_eq!(c.bond.bond_of(&c.supplier), (100, 0));
}

#[test]
fn slash_pays_caller_and_caps_at_total() {
    let c = setup();
    c.bond.deposit(&c.supplier, &100);
    c.bond.reserve_for_advance(&c.gb, &c.supplier, &80);
    assert_eq!(c.bond.slash(&c.gb, &c.supplier, &80), 80);
    assert_eq!(c.usdc.balance(&c.gb), 80);
    assert_eq!(c.bond.bond_of(&c.supplier), (20, 0));
    assert_eq!(c.bond.slash(&c.gb, &c.supplier, &50), 20); // capped at what is left
}
