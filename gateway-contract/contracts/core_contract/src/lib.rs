#![no_std]
use soroban_sdk::{contract, contractimpl, Env, Symbol};

#[contract]
pub struct CoreContract;

#[contractimpl]
impl CoreContract {
    pub fn hello(env: Env, to: Symbol) -> Symbol {
        let hello = soroban_sdk::symbol!("Hello");
        hello.concat(to)
    }
}

#[cfg(test)]
mod test {
    use soroban_sdk::symbol;

    use super::*;

    #[test]
    fn test_hello() {
        let env = Env::default();
        let contract_id = env.register_contract(None, CoreContract);
        let client = CoreContractClient::new(&env, &contract_id);

        let word = symbol!("world");
        let expected = symbol!("Hello, world!");
        assert_eq!(client.hello(&word), expected);
    }
}
