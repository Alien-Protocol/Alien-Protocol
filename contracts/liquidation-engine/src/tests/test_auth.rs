#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::{Address as _, MockAuth, MockAuthInvoke};
use soroban_sdk::{contract, contractimpl, Address, Env, IntoVal};

#[contract]
struct MockVaultContract;

#[contractimpl]
impl MockVaultContract {
    pub fn get_liquidation_engine(_env: Env) -> Option<Address> {
        None
    }
}

#[contract]
struct MockPoolContract;

#[contractimpl]
impl MockPoolContract {
    pub fn get_borrow_asset(_env: Env) -> Option<Address> {
        None
    }
}

#[contract]
struct MockOracleContract;

#[contractimpl]
impl MockOracleContract {}

#[test]
fn test_liquidate_requires_liquidator_auth() {
    let env = Env::default();

    let contract_id = env.register(LiquidationContract, ());
    let client = LiquidationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let liquidator = Address::generate(&env);
    let user = Address::generate(&env);

    let vault_id = env.register(MockVaultContract, ());
    let pool_id = env.register(MockPoolContract, ());
    let oracle_id = env.register(MockOracleContract, ());

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "initialize",
            args: (
                admin.clone(),
                vault_id.clone(),
                pool_id.clone(),
                oracle_id.clone(),
            )
                .into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.initialize(&admin, &vault_id, &pool_id, &oracle_id);

    // 1. Without auth, try_liquidate fails
    let res_no_auth = client.try_liquidate(&liquidator, &user, &1000);
    assert!(res_no_auth.is_err(), "must fail without liquidator auth");

    // 2. With other signer auth, try_liquidate fails
    let other = Address::generate(&env);
    env.mock_auths(&[MockAuth {
        address: &other,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "liquidate",
            args: (liquidator.clone(), user.clone(), 1000_i128).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    let res_wrong = client.try_liquidate(&liquidator, &user, &1000);
    assert!(res_wrong.is_err(), "must fail with wrong signer auth");
}
