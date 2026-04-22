#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct FactoryContract;

#[contractimpl]
impl FactoryContract {
    pub fn create_contract(_env: Env) {
        // Basic factory contract functionality
    }
    
    pub fn get_contract_count(_env: Env) -> u32 {
        0
    }
}

#[cfg(test)]
mod test {
    use soroban_sdk::Env;

    use super::*;

    #[test]
    fn test_contract_count() {
        let env = Env::default();
        assert_eq!(FactoryContract::get_contract_count(&env), 0);
    }
}
