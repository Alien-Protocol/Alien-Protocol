#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct CoreContract;

#[contractimpl]
impl CoreContract {
    pub fn initialize(_env: Env) {
        // Basic initialization function
    }
    
    pub fn get_version(_env: Env) -> u32 {
        1
    }
}

#[cfg(test)]
mod test {
    use soroban_sdk::Env;

    use super::*;

    #[test]
    fn test_version() {
        let env = Env::default();
        assert_eq!(CoreContract::get_version(&env), 1);
    }
}
