#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct Disputes;

#[contractimpl]
impl Disputes {
    /// Placeholder until M1; lets CI build and test the empty contract.
    pub fn version(_env: Env) -> u32 {
        1
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn version_is_one() {
        let env = Env::default();
        let id = env.register(Disputes, ());
        let client = DisputesClient::new(&env, &id);
        assert_eq!(client.version(), 1);
    }
}
