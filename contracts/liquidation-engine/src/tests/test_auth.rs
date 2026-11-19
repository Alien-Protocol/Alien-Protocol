#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, MockAuth, MockAuthInvoke};
use soroban_sdk::{Address, Env, IntoVal};

#[test]
fn test_liquidate_requires_liquidator_auth() {
    let env = Env::default();
    let contract_id = env.register(LiquidationContract, ());
    let client = LiquidationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let liquidator = Address::generate(&env);
    let wrong = Address::generate(&env);
    let user = Address::generate(&env);

    let vault = Address::generate(&env);
    let pool = Address::generate(&env);
    let oracle = Address::generate(&env);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "initialize",
            args: (
                admin.clone(),
                vault.clone(),
                pool.clone(),
                oracle.clone(),
            )
                .into_val(&env),
            sub_invokes: &[],
        },
    }]);

    client.initialize(&admin, &vault, &pool, &oracle);

    env.mock_auths(&[MockAuth {
        address: &wrong,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "liquidate",
            args: (
                liquidator.clone(),
                user.clone(),
                100_i128,
            )
                .into_val(&env),
            sub_invokes: &[],
        },
    }]);

    assert!(
        client
            .try_liquidate(&liquidator, &user, &100)
            .is_err()
    );

    env.mock_auths(&[MockAuth {
        address: &liquidator,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "liquidate",
            args: (
                liquidator.clone(),
                user.clone(),
                100_i128,
            )
                .into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, liquidator);
}