#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, MockAuth, MockAuthInvoke};
use soroban_sdk::{Address, Env, IntoVal};

#[test]
fn test_borrow_requires_user_auth() {
    let env = Env::default();
    let contract_id = env.register(PoolContract, ());
    let client = PoolContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let wrong = Address::generate(&env);
    let vault = Address::generate(&env);
    let oracle = Address::generate(&env);

    let sac_admin = Address::generate(&env);
    let asset_contract =
        env.register_stellar_asset_contract_v2(sac_admin);
    let asset = asset_contract.address();

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "initialize",
            args: (
                admin.clone(),
                vault.clone(),
                oracle.clone(),
                asset.clone(),
                1000_u32,
            )
                .into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.initialize(&admin, &vault, &oracle, &asset, &1000);

    env.set_auths(&[]);

    let wrong_result =
        client.try_borrow(&wrong, &asset, &100);
    assert!(wrong_result.is_err());

    env.mock_auths(&[MockAuth {
        address: &user,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "borrow",
            args: (user.clone(), asset.clone(), 100_i128).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, user);
}

#[test]
fn test_repay_for_requires_payer_not_user() {
    let env = Env::default();
    let contract_id = env.register(PoolContract, ());
    let client = PoolContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let payer = Address::generate(&env);
    let user = Address::generate(&env);
    let vault = Address::generate(&env);
    let oracle = Address::generate(&env);

    let sac_admin = Address::generate(&env);
    let asset_contract =
        env.register_stellar_asset_contract_v2(sac_admin);
    let asset = asset_contract.address();

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "initialize",
            args: (
                admin.clone(),
                vault.clone(),
                oracle.clone(),
                asset.clone(),
                1000_u32,
            )
                .into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.initialize(&admin, &vault, &oracle, &asset, &1000);

    let amount = 100_i128;

    env.mock_auths(&[MockAuth {
        address: &payer,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "repay_for",
            args: (
                payer.clone(),
                user.clone(),
                asset.clone(),
                amount,
            )
                .into_val(&env),
            sub_invokes: &[MockAuthInvoke {
                contract: &asset,
                fn_name: "transfer",
                args: (
                    payer.clone(),
                    contract_id.clone(),
                    amount,
                )
                    .into_val(&env),
                sub_invokes: &[],
            }],
        },
    }]);

    let _ = client.try_repay_for(
        &payer,
        &user,
        &asset,
        &amount,
    );

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, payer);

    let sub_invokes = &auths[0].1.sub_invocations;
    assert_eq!(sub_invokes.len(), 1);
    assert_eq!(sub_invokes[0].contract, asset);
    assert_eq!(sub_invokes[0].function_name, "transfer");
}