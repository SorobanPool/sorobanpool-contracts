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
