#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, MockAuth, MockAuthInvoke};
use soroban_sdk::{Address, Env, IntoVal};

#[test]
fn test_deposit_requires_user_auth() {
    let env = Env::default();
    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let wrong = Address::generate(&env);
    let oracle = Address::generate(&env);
    let pool = Address::generate(&env);
    let engine = Address::generate(&env);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "initialize",
            args: (
                admin.clone(),
                pool.clone(),
                oracle.clone(),
                engine.clone(),
            )
                .into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.initialize(&admin, &pool, &oracle, &engine);

    let token_admin = Address::generate(&env);
    let token_contract =
        env.register_stellar_asset_contract_v2(token_admin);
    let asset = token_contract.address();
    let sac_admin = token::StellarAssetClient::new(&env, &asset);

    env.mock_all_auths();
    client.add_supported_asset(&asset);
    sac_admin.mint(&user, &1000);

    env.set_auths(&[]);
    assert!(client.try_deposit(&wrong, &asset, &100).is_err());

    env.mock_auths(&[MockAuth {
        address: &user,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "deposit",
            args: (user.clone(), asset.clone(), 100_i128).into_val(&env),
            sub_invokes: &[MockAuthInvoke {
                contract: &asset,
                fn_name: "transfer",
                args: (
                    user.clone(),
                    contract_id.clone(),
                    100_i128,
                )
                    .into_val(&env),
                sub_invokes: &[],
            }],
        },
    }]);

    assert!(client.try_deposit(&user, &asset, &100).is_ok());

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, user);
}

#[test]
fn test_withdraw_rejects_other_signer() {
    let env = Env::default();
    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let wrong = Address::generate(&env);

    let oracle = Address::generate(&env);
    let pool = Address::generate(&env);
    let engine = Address::generate(&env);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "initialize",
            args: (
                admin.clone(),
                pool.clone(),
                oracle.clone(),
                engine.clone(),
            )
                .into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.initialize(&admin, &pool, &oracle, &engine);

    env.mock_auths(&[MockAuth {
        address: &wrong,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "withdraw",
            args: (user.clone(), Address::generate(&env), 100_i128)
                .into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let asset = Address::generate(&env);
    assert!(
        client
            .try_withdraw(&user, &asset, &100)
            .is_err()
    );
}
