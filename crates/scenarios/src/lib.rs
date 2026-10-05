//! Test harness wiring all SorobanPool contracts together with a real USDC token.
use config::{Config, ConfigClient};
use disputes::{Disputes, DisputesClient};
use group_buy::{GroupBuy, GroupBuyClient};
use registry::{Registry, RegistryClient};
use reputation::{Reputation, ReputationClient};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{symbol_short, token, vec, Address, BytesN, Env, Symbol};
use sp_common::keys;
use sp_common::types::{PoolTerms, Role, Tier};
use supplier_bond::{SupplierBond, SupplierBondClient};

pub const USDC: i128 = 10_000_000;
pub const T0: u64 = 1_000_000;
pub const DAY: u64 = 24 * 3600;

pub struct Ctx {
    pub env: Env,
    pub admin: Address,
    pub treasury: Address,
    pub attestor: Address,
    pub arbiter: Address,
    pub token: token::Client<'static>,
    pub mint: token::StellarAssetClient<'static>,
    pub cfg: ConfigClient<'static>,
    pub reg: RegistryClient<'static>,
    pub rep: ReputationClient<'static>,
    pub bond: SupplierBondClient<'static>,
    pub gb: GroupBuyClient<'static>,
    pub dis: DisputesClient<'static>,
    pub supplier: Address,
    pub organizer: Address,
}

pub fn h(env: &Env, b: u8) -> BytesN<32> {
    BytesN::from_array(env, &[b; 32])
}

pub fn setup() -> Ctx {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(T0);
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(Address::generate(&env));
    let cfg_id = env.register(Config, (&admin, sac.address(), &treasury));
    let cfg = ConfigClient::new(&env, &cfg_id);
    let reg_id = env.register(Registry, (&cfg_id,));
    let rep_id = env.register(Reputation, (&cfg_id,));
    let bond_id = env.register(SupplierBond, (&cfg_id,));
    let gb_id = env.register(GroupBuy, (&cfg_id,));
    let dis_id = env.register(Disputes, (&cfg_id,));
    cfg.set_address(&keys::REGISTRY, &reg_id);
    cfg.set_address(&keys::REPUTATION, &rep_id);
    cfg.set_address(&keys::BOND, &bond_id);
    cfg.set_address(&keys::GROUP_BUY, &gb_id);
    cfg.set_address(&keys::DISPUTES, &dis_id);
    cfg.set_category(&symbol_short!("rice"), &true);
    let attestor = Address::generate(&env);
    let arbiter = Address::generate(&env);
    cfg.set_attestor(&attestor, &true);
    cfg.set_arbiter(&arbiter, &true);
    let mut c = Ctx {
        token: token::Client::new(&env, &sac.address()),
        mint: token::StellarAssetClient::new(&env, &sac.address()),
        reg: RegistryClient::new(&env, &reg_id),
        rep: ReputationClient::new(&env, &rep_id),
        bond: SupplierBondClient::new(&env, &bond_id),
        gb: GroupBuyClient::new(&env, &gb_id),
        dis: DisputesClient::new(&env, &dis_id),
        cfg,
        supplier: Address::generate(&env),
        organizer: Address::generate(&env),
        env,
        admin,
        treasury,
        attestor,
        arbiter,
    };
    c.verify(&c.supplier.clone(), Role::Supplier);
    c.verify(&c.organizer.clone(), Role::Organizer);
    c.mint.mint(&c.supplier, &(1_000 * USDC));
    c.supplier = c.supplier.clone();
    c
}

impl Ctx {
    pub fn verify(&self, who: &Address, role: Role) {
        self.reg.register(who, &role, &h(&self.env, 1), &None);
        self.reg
            .attest(&self.attestor, who, &role, &h(&self.env, 2), &1);
    }

    /// KYC'd trader (T1) funded with 1,000 USDC.
    pub fn trader(&self) -> Address {
        let t = Address::generate(&self.env);
        self.verify(&t, Role::Trader);
        self.mint.mint(&t, &(1_000 * USDC));
        t
    }

    pub fn now(&self) -> u64 {
        self.env.ledger().timestamp()
    }

    pub fn set_time(&self, t: u64) {
        self.env.ledger().set_timestamp(t);
    }

    pub fn advance(&self, secs: u64) {
        self.set_time(self.now() + secs);
    }

    /// MOQ 100 @ 1.0, 200 @ 0.9, 400 @ 0.8 USDC; max 500; 250 per member.
    pub fn terms(&self) -> PoolTerms {
        PoolTerms {
            supplier: self.supplier.clone(),
            offer_hash: h(&self.env, 9),
            unit_label_hash: h(&self.env, 8),
            category: symbol_short!("rice"),
            tiers: vec![
                &self.env,
                Tier {
                    min_units: 100,
                    unit_price: USDC,
                },
                Tier {
                    min_units: 200,
                    unit_price: USDC * 9 / 10,
                },
                Tier {
                    min_units: 400,
                    unit_price: USDC * 8 / 10,
                },
            ],
            moq: 100,
            max_units: 500,
            max_per_member: 250,
            lead_time_secs: 5 * DAY,
            perishable: false,
        }
    }

    pub fn pool(&self, organizer_fee_bp: u32) -> u64 {
        self.gb.create_pool(
            &self.organizer,
            &self.terms(),
            &h(&self.env, 7),
            &organizer_fee_bp,
            &None::<Symbol>,
            &(self.now() + 3 * DAY),
        )
    }

    pub fn bal(&self, who: &Address) -> i128 {
        self.token.balance(who)
    }

    /// Fill past the deadline with the standard four-trader scenario (420 units, tier 3).
    pub fn fill_standard(&self, pool: u64) -> [Address; 4] {
        let t = [self.trader(), self.trader(), self.trader(), self.trader()];
        self.gb.commit(&t[0], &pool, &60);
        self.gb.commit(&t[1], &pool, &60);
        self.gb.commit(&t[2], &pool, &100);
        self.gb.commit(&t[3], &pool, &200);
        self.advance(3 * DAY);
        self.gb.close(&pool);
        t
    }

    /// Standard pool taken to Delivered with `received` units.
    pub fn deliver(&self, pool: u64, received: u32) {
        self.gb.accept(&self.supplier, &pool, &0);
        self.gb.dispatch(&self.supplier, &pool, &None);
        self.gb
            .confirm_delivery(&self.organizer, &pool, &received, &h(&self.env, 5));
    }
}
