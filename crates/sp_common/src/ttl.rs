use soroban_sdk::{Env, IntoVal, Val};

/// ~30 days / ~60 days in ledgers (5s each).
pub const TTL_THRESHOLD: u32 = 518_400;
pub const TTL_EXTEND_TO: u32 = 1_036_800;

pub fn bump_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(TTL_THRESHOLD, TTL_EXTEND_TO);
}

pub fn bump_persistent<K: IntoVal<Env, Val>>(env: &Env, key: &K) {
    env.storage()
        .persistent()
        .extend_ttl(key, TTL_THRESHOLD, TTL_EXTEND_TO);
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::storage::Instance as _;
    use soroban_sdk::testutils::storage::Persistent as _;
    use soroban_sdk::testutils::Ledger as _;
    use soroban_sdk::{contract, contractimpl, symbol_short};

    #[contract]
    pub struct TestContract;

    #[contractimpl]
    impl TestContract {
        pub fn init(env: Env) {
            let key = symbol_short!("test_key");
            env.storage().persistent().set(&key, &100u32);
        }

        pub fn touch_persistent(env: Env) {
            let key = symbol_short!("test_key");
            bump_persistent(&env, &key);
        }

        pub fn touch_instance(env: Env) {
            bump_instance(&env);
        }
    }

    #[test]
    fn test_constants_values() {
        assert_eq!(TTL_THRESHOLD, 518_400);
        assert_eq!(TTL_EXTEND_TO, 1_036_800);
        const { assert!(TTL_EXTEND_TO > TTL_THRESHOLD) };
    }

    #[test]
    fn test_persistent_ttl_extension_below_threshold() {
        let env = Env::default();
        let contract_id = env.register(TestContract, ());
        let client = TestContractClient::new(&env, &contract_id);

        client.init();
        let key = symbol_short!("test_key");

        // Newly written entry starts with low default TTL (< TTL_THRESHOLD)
        let initial_ttl =
            env.as_contract(&contract_id, || env.storage().persistent().get_ttl(&key));
        assert!(initial_ttl < TTL_THRESHOLD);

        // Bumping persistent extends it up to TTL_EXTEND_TO
        client.touch_persistent();

        let extended_ttl =
            env.as_contract(&contract_id, || env.storage().persistent().get_ttl(&key));
        assert_eq!(extended_ttl, TTL_EXTEND_TO);

        // Advance ledger so remaining TTL drops below TTL_THRESHOLD
        let ledgers_to_advance = TTL_EXTEND_TO - TTL_THRESHOLD + 1000;
        let new_seq = env.ledger().sequence() + ledgers_to_advance;
        env.ledger().set_sequence_number(new_seq);

        let current_ttl =
            env.as_contract(&contract_id, || env.storage().persistent().get_ttl(&key));
        assert!(
            current_ttl < TTL_THRESHOLD,
            "Expected TTL {} < {}",
            current_ttl,
            TTL_THRESHOLD
        );

        // Call helper to bump persistent TTL back to TTL_EXTEND_TO
        client.touch_persistent();

        let bumped_ttl = env.as_contract(&contract_id, || env.storage().persistent().get_ttl(&key));
        assert_eq!(bumped_ttl, TTL_EXTEND_TO);
    }

    #[test]
    fn test_persistent_ttl_long_ttl_untouched() {
        let env = Env::default();
        let contract_id = env.register(TestContract, ());
        let client = TestContractClient::new(&env, &contract_id);

        client.init();
        let key = symbol_short!("test_key");

        // Bump to TTL_EXTEND_TO first
        client.touch_persistent();
        let ttl_after_bump =
            env.as_contract(&contract_id, || env.storage().persistent().get_ttl(&key));
        assert_eq!(ttl_after_bump, TTL_EXTEND_TO);

        // Advance only a small number of ledgers so TTL remains well above TTL_THRESHOLD
        let small_advance = 100u32;
        env.ledger()
            .set_sequence_number(env.ledger().sequence() + small_advance);

        let before_touch =
            env.as_contract(&contract_id, || env.storage().persistent().get_ttl(&key));
        assert!(before_touch > TTL_THRESHOLD);
        assert_eq!(before_touch, TTL_EXTEND_TO - small_advance);

        // Calling bump_persistent when TTL is >= TTL_THRESHOLD must leave it untouched
        client.touch_persistent();

        let after_touch =
            env.as_contract(&contract_id, || env.storage().persistent().get_ttl(&key));
        assert_eq!(
            before_touch, after_touch,
            "Long TTL should remain untouched"
        );
    }

    #[test]
    fn test_instance_ttl_extension_below_threshold() {
        let env = Env::default();
        let contract_id = env.register(TestContract, ());
        let client = TestContractClient::new(&env, &contract_id);

        client.init();

        // Initial instance TTL is low in test env (< TTL_THRESHOLD)
        let initial_ttl = env.as_contract(&contract_id, || env.storage().instance().get_ttl());
        assert!(initial_ttl < TTL_THRESHOLD);

        // Calling bump_instance extends it to TTL_EXTEND_TO
        client.touch_instance();

        let extended_ttl = env.as_contract(&contract_id, || env.storage().instance().get_ttl());
        assert_eq!(extended_ttl, TTL_EXTEND_TO);

        // Advance ledger so instance TTL drops below threshold
        let ledgers_to_advance = TTL_EXTEND_TO - TTL_THRESHOLD + 500;
        env.ledger()
            .set_sequence_number(env.ledger().sequence() + ledgers_to_advance);

        let current_ttl = env.as_contract(&contract_id, || env.storage().instance().get_ttl());
        assert!(current_ttl < TTL_THRESHOLD);

        client.touch_instance();

        let bumped_ttl = env.as_contract(&contract_id, || env.storage().instance().get_ttl());
        assert_eq!(bumped_ttl, TTL_EXTEND_TO);
    }
}
