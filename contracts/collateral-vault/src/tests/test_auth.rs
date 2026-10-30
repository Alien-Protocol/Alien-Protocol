#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::{Address as _, MockAuth, MockAuthInvoke};
use soroban_sdk::{contract, contractimpl, token, Address, Env, IntoVal};

#[contract]
pub struct MockLendingPool;

#[contractimpl]
impl MockLendingPool {
    pub fn get_user_debt(_env: Env, _user: Address) -> i128 {
        0
    }

    pub fn is_liquidatable(_env: Env, _user: Address) -> bool {
        true
    }
}

fn setup_auth_env() -> (
    Env,
    VaultContractClient<'static>,
    Address,
    Address,
    Address,
    Address,
    token::Client<'static>,
    token::StellarAssetClient<'static>,
    Address,
) {
    let env = Env::default();

    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let oracle = Address::generate(&env);
    let lending_pool = env.register(MockLendingPool, ());
    let liquidation_engine = Address::generate(&env);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "initialize",
            args: (
                admin.clone(),
                lending_pool.clone(),
                oracle.clone(),
                liquidation_engine.clone(),
            )
                .into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.initialize(&admin, &lending_pool, &oracle, &liquidation_engine);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_contract_id = token_contract.address();
    let token_client = token::Client::new(&env, &token_contract_id);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract_id);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "add_supported_asset",
            args: (token_contract_id.clone(),).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.add_supported_asset(&token_contract_id);

    env.mock_auths(&[MockAuth {
        address: &token_admin,
        invoke: &MockAuthInvoke {
            contract: &token_contract_id,
            fn_name: "mint",
            args: (user.clone(), 1000_i128).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    token_admin_client.mint(&user, &1000);

    (
        env,
        client,
        admin,
        user,
        oracle,
        token_contract_id,
        token_client,
        token_admin_client,
        liquidation_engine,
    )
}

#[test]
fn test_deposit_requires_user_auth() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, _token_admin, _engine) =
        setup_auth_env();

    // 1. Without auth, try_deposit fails
    let res_no_auth = client.try_deposit(&user, &token_id, &500);
    assert!(res_no_auth.is_err());

    // 2. With wrong signer auth, try_deposit fails
    let wrong = Address::generate(&env);
    env.mock_auths(&[MockAuth {
        address: &wrong,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "deposit",
            args: (user.clone(), token_id.clone(), 500_i128).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    let res_wrong = client.try_deposit(&user, &token_id, &500);
    assert!(res_wrong.is_err());

    // 3. With exact user MockAuth and inner SAC transfer sub-invoke, deposit succeeds
    env.mock_auths(&[MockAuth {
        address: &user,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "deposit",
            args: (user.clone(), token_id.clone(), 500_i128).into_val(&env),
            sub_invokes: &[MockAuthInvoke {
                contract: &token_id,
                fn_name: "transfer",
                args: (user.clone(), client.address.clone(), 500_i128).into_val(&env),
                sub_invokes: &[],
            }],
        },
    }]);
    let res_ok = client.try_deposit(&user, &token_id, &500);
    assert!(res_ok.is_ok());

    // Assert env.auths() immediately after the call under test
    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, user);
}

#[test]
fn test_withdraw_rejects_other_signer() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, _token_admin, _engine) =
        setup_auth_env();

    // Deposit with user auth
    env.mock_auths(&[MockAuth {
        address: &user,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "deposit",
            args: (user.clone(), token_id.clone(), 500_i128).into_val(&env),
            sub_invokes: &[MockAuthInvoke {
                contract: &token_id,
                fn_name: "transfer",
                args: (user.clone(), client.address.clone(), 500_i128).into_val(&env),
                sub_invokes: &[],
            }],
        },
    }]);
    client.deposit(&user, &token_id, &500);

    // Other signer attempts to withdraw user collateral
    let malicious = Address::generate(&env);
    env.mock_auths(&[MockAuth {
        address: &malicious,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "withdraw",
            args: (user.clone(), token_id.clone(), 200_i128).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    let res_other = client.try_withdraw(&user, &token_id, &200);
    assert!(res_other.is_err(), "other signer must be rejected");

    // Legitimate user MockAuth succeeds
    env.mock_auths(&[MockAuth {
        address: &user,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "withdraw",
            args: (user.clone(), token_id.clone(), 200_i128).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    let res_ok = client.try_withdraw(&user, &token_id, &200);
    assert!(res_ok.is_ok());

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, user);
}

#[test]
fn test_seize_requires_registered_engine_auth() {
    let (
        env,
        client,
        _admin,
        user,
        _oracle,
        token_id,
        _token_client,
        _token_admin,
        registered_engine,
    ) = setup_auth_env();

    // Deposit collateral
    env.mock_auths(&[MockAuth {
        address: &user,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "deposit",
            args: (user.clone(), token_id.clone(), 500_i128).into_val(&env),
            sub_invokes: &[MockAuthInvoke {
                contract: &token_id,
                fn_name: "transfer",
                args: (user.clone(), client.address.clone(), 500_i128).into_val(&env),
                sub_invokes: &[],
            }],
        },
    }]);
    client.deposit(&user, &token_id, &500);

    // Fake engine calls seize
    let fake_engine = Address::generate(&env);
    env.mock_auths(&[MockAuth {
        address: &fake_engine,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "seize_collateral",
            args: (
                fake_engine.clone(),
                user.clone(),
                token_id.clone(),
                200_i128,
            )
                .into_val(&env),
            sub_invokes: &[],
        },
    }]);
    let res_fake = client.try_seize_collateral(&fake_engine, &user, &token_id, &200);
    assert_eq!(res_fake, Err(Ok(VaultError::Unauthorized)));

    // Registered engine without auth fails
    let res_no_auth = client.try_seize_collateral(&registered_engine, &user, &token_id, &200);
    assert!(res_no_auth.is_err());

    // Registered engine with MockAuth succeeds
    env.mock_auths(&[MockAuth {
        address: &registered_engine,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "seize_collateral",
            args: (
                registered_engine.clone(),
                user.clone(),
                token_id.clone(),
                200_i128,
            )
                .into_val(&env),
            sub_invokes: &[],
        },
    }]);
    let res_ok = client.try_seize_collateral(&registered_engine, &user, &token_id, &200);
    assert!(res_ok.is_ok());

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, registered_engine);
}
