#![cfg(test)]

//! Security tests that replace blanket `mock_all_auths` with explicit
//! `mock_auths` entries and assert exact authorization trees via `env.auths()`.

extern crate alloc;

use super::super::*;
use soroban_sdk::testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, MockAuth, MockAuthInvoke};
use soroban_sdk::{token, Address, Env, IntoVal, Symbol};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn base_setup() -> (Env, VaultContractClient<'static>, Address, Address, Address, token::StellarAssetClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    client.initialize(&admin, &oracle);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = token_contract.address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);

    client.add_supported_asset(&token_id);

    (env, client, admin, token_id, oracle, token_admin_client)
}

// ---------------------------------------------------------------------------
// deposit — authorization tree
// ---------------------------------------------------------------------------

#[test]
fn test_deposit_exact_auth_tree() {
    let env = Env::default();
    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    let user = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin, &oracle);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = token_contract.address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);
    client.add_supported_asset(&token_id);
    token_admin_client.mint(&user, &1000);

    // Use mock_auths to set exact authorized invocations
    env.mock_auths(&[MockAuth {
        address: &user,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "deposit",
            args: (&user, &token_id, &500_i128).into_val(&env),
            sub_invokes: &[MockAuthInvoke {
                contract: &token_id,
                fn_name: "transfer",
                args: (&user, &contract_id, &500_i128).into_val(&env),
                sub_invokes: &[],
            }],
        },
    }]);

    client.deposit(&user, &token_id, &500);

    // Verify exact auth tree was recorded
    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    let (auth_addr, auth_inv) = auths.first().unwrap();
    assert_eq!(*auth_addr, user);
    assert_eq!(
        auth_inv,
        &AuthorizedInvocation {
            function: AuthorizedFunction::Contract((
                contract_id.clone(),
                Symbol::new(&env, "deposit"),
                (&user, &token_id, &500_i128).into_val(&env),
            )),
            sub_invocations: alloc::vec![AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    token_id.clone(),
                    Symbol::new(&env, "transfer"),
                    (&user, &contract_id, &500_i128).into_val(&env),
                )),
                sub_invocations: alloc::vec![],
            }],
        }
    );
}

#[test]
fn test_deposit_wrong_signer_fails() {
    let (env, client, _admin, token_id, _oracle, token_admin) = base_setup();
    let user = Address::generate(&env);
    let attacker = Address::generate(&env);

    token_admin.mint(&user, &1000);

    // Only grant auth to attacker — deposit requires user auth, so must fail
    env.mock_auths(&[MockAuth {
        address: &attacker,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "deposit",
            args: (&user, &token_id, &500_i128).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let res = client.try_deposit(&user, &token_id, &500);
    assert!(res.is_err(), "deposit must fail when wrong signer authorizes");
}

#[test]
fn test_deposit_no_auth_fails() {
    let (env, client, _admin, token_id, _oracle, token_admin) = base_setup();
    let user = Address::generate(&env);
    token_admin.mint(&user, &1000);

    env.mock_auths(&[]);
    let res = client.try_deposit(&user, &token_id, &500);
    assert!(res.is_err());
}

#[test]
fn test_deposit_invalid_amount_rollback() {
    let (env, client, _admin, token_id, _oracle, token_admin) = base_setup();
    let user = Address::generate(&env);
    token_admin.mint(&user, &1000);

    let balance_before = client.get_position_balance(&user, &token_id);
    let res = client.try_deposit(&user, &token_id, &0);
    assert!(res.is_err());
    assert_eq!(
        client.get_position_balance(&user, &token_id),
        balance_before,
        "storage must not change on invalid input"
    );
}

// ---------------------------------------------------------------------------
// withdraw — authorization tree
// ---------------------------------------------------------------------------

#[test]
fn test_withdraw_exact_auth_tree() {
    let env = Env::default();
    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    let user = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin, &oracle);
    let token_admin_addr = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin_addr.clone());
    let token_id = token_contract.address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);
    client.add_supported_asset(&token_id);
    token_admin_client.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    env.mock_auths(&[MockAuth {
        address: &user,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "withdraw",
            args: (&user, &token_id, &200_i128).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    client.withdraw(&user, &token_id, &200);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    let (auth_addr, auth_inv) = auths.first().unwrap();
    assert_eq!(*auth_addr, user);
    assert_eq!(
        auth_inv.function,
        AuthorizedFunction::Contract((
            contract_id.clone(),
            Symbol::new(&env, "withdraw"),
            (&user, &token_id, &200_i128).into_val(&env),
        ))
    );
}

#[test]
fn test_withdraw_wrong_signer_fails() {
    let (env, client, _admin, token_id, _oracle, token_admin) = base_setup();
    let user = Address::generate(&env);
    let attacker = Address::generate(&env);
    token_admin.mint(&user, &1000);

    env.mock_all_auths();
    client.deposit(&user, &token_id, &500);

    env.mock_auths(&[MockAuth {
        address: &attacker,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "withdraw",
            args: (&user, &token_id, &200_i128).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let res = client.try_withdraw(&user, &token_id, &200);
    assert!(res.is_err());
}

#[test]
fn test_withdraw_no_position_rollback() {
    let (env, client, _admin, token_id, _oracle, _token_admin) = base_setup();
    let user = Address::generate(&env);

    let index_before = client.get_position_index();
    let res = client.try_withdraw(&user, &token_id, &100);
    assert!(res.is_err());
    assert_eq!(
        client.get_position_index(),
        index_before,
        "index must not change on failed withdraw"
    );
}

// ---------------------------------------------------------------------------
// pause / unpause — admin-only
// ---------------------------------------------------------------------------

#[test]
fn test_pause_requires_admin_auth() {
    let (env, client, admin, _token_id, _oracle, _token_admin) = base_setup();

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "pause",
            args: ().into_val(&env),
            sub_invokes: &[],
        },
    }]);

    client.pause();

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    let (auth_addr, auth_inv) = auths.first().unwrap();
    assert_eq!(*auth_addr, admin);
    assert_eq!(
        auth_inv.function,
        AuthorizedFunction::Contract((
            client.address.clone(),
            Symbol::new(&env, "pause"),
            ().into_val(&env),
        ))
    );
}

#[test]
fn test_pause_non_admin_fails() {
    let (env, client, _admin, _token_id, _oracle, _token_admin) = base_setup();
    let non_admin = Address::generate(&env);

    env.mock_auths(&[MockAuth {
        address: &non_admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "pause",
            args: ().into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let res = client.try_pause();
    assert!(res.is_err(), "non-admin must not be able to pause");
}

#[test]
fn test_unpause_requires_admin_auth() {
    let (env, client, admin, _token_id, _oracle, _token_admin) = base_setup();

    env.mock_all_auths();
    client.pause();

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "unpause",
            args: ().into_val(&env),
            sub_invokes: &[],
        },
    }]);

    client.unpause();

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    let (auth_addr, auth_inv) = auths.first().unwrap();
    assert_eq!(*auth_addr, admin);
    assert_eq!(
        auth_inv.function,
        AuthorizedFunction::Contract((
            client.address.clone(),
            Symbol::new(&env, "unpause"),
            ().into_val(&env),
        ))
    );
}

// ---------------------------------------------------------------------------
// set_admin — auth tree + error variants
// ---------------------------------------------------------------------------

#[test]
fn test_set_admin_requires_current_admin_auth() {
    let (env, client, admin, _token_id, _oracle, _token_admin) = base_setup();
    let new_admin = Address::generate(&env);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "set_admin",
            args: (&new_admin,).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    client.set_admin(&new_admin);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    let (auth_addr, auth_inv) = auths.first().unwrap();
    assert_eq!(*auth_addr, admin);
    assert_eq!(
        auth_inv.function,
        AuthorizedFunction::Contract((
            client.address.clone(),
            Symbol::new(&env, "set_admin"),
            (&new_admin,).into_val(&env),
        ))
    );
    assert_eq!(client.get_admin(), Some(new_admin));
}

#[test]
fn test_set_admin_non_admin_fails() {
    let (env, client, _admin, _token_id, _oracle, _token_admin) = base_setup();
    let attacker = Address::generate(&env);
    let new_admin = Address::generate(&env);

    env.mock_auths(&[MockAuth {
        address: &attacker,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "set_admin",
            args: (&new_admin,).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let res = client.try_set_admin(&new_admin);
    assert!(res.is_err());
}

#[test]
fn test_set_admin_same_address_returns_error() {
    let (env, client, admin, _token_id, _oracle, _token_admin) = base_setup();

    env.mock_all_auths();
    let res = client.try_set_admin(&admin);
    assert_eq!(res, Err(Ok(VaultError::AlreadyAdmin)));
}

#[test]
fn test_set_admin_rollback_on_same_address() {
    let (env, client, admin, _token_id, _oracle, _token_admin) = base_setup();

    env.mock_all_auths();
    let _ = client.try_set_admin(&admin);
    // Admin must remain unchanged
    assert_eq!(client.get_admin(), Some(admin));
}

// ---------------------------------------------------------------------------
// add_supported_asset / remove_supported_asset — admin-only
// ---------------------------------------------------------------------------

#[test]
fn test_add_asset_requires_admin_auth() {
    let (env, client, admin, _token_id, _oracle, _token_admin) = base_setup();
    let new_asset = Address::generate(&env);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "add_supported_asset",
            args: (&new_asset,).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    client.add_supported_asset(&new_asset);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    let (auth_addr, _) = auths.first().unwrap();
    assert_eq!(*auth_addr, admin);
}

#[test]
fn test_add_asset_non_admin_fails() {
    let (env, client, _admin, _token_id, _oracle, _token_admin) = base_setup();
    let attacker = Address::generate(&env);
    let new_asset = Address::generate(&env);

    env.mock_auths(&[MockAuth {
        address: &attacker,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "add_supported_asset",
            args: (&new_asset,).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let res = client.try_add_supported_asset(&new_asset);
    assert!(res.is_err());
}

#[test]
fn test_remove_asset_requires_admin_auth() {
    let (env, client, admin, token_id, _oracle, _token_admin) = base_setup();

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "remove_supported_asset",
            args: (&token_id,).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    client.remove_supported_asset(&token_id);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    let (auth_addr, _) = auths.first().unwrap();
    assert_eq!(*auth_addr, admin);
}

#[test]
fn test_remove_asset_non_admin_fails() {
    let (env, client, _admin, token_id, _oracle, _token_admin) = base_setup();
    let attacker = Address::generate(&env);

    env.mock_auths(&[MockAuth {
        address: &attacker,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "remove_supported_asset",
            args: (&token_id,).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let res = client.try_remove_supported_asset(&token_id);
    assert!(res.is_err());
}

// ---------------------------------------------------------------------------
// seize_collateral — engine-only
// ---------------------------------------------------------------------------

#[test]
fn test_seize_requires_registered_engine_auth() {
    let (env, client, _admin, token_id, _oracle, token_admin_sac) = base_setup();
    let engine = Address::generate(&env);
    let user = Address::generate(&env);

    env.mock_all_auths();
    client.set_liquidation_engine(&engine);
    token_admin_sac.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    env.mock_auths(&[MockAuth {
        address: &engine,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "seize_collateral",
            args: (&engine, &user, &token_id, &200_i128).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    client.seize_collateral(&engine, &user, &token_id, &200);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    let (auth_addr, auth_inv) = auths.first().unwrap();
    assert_eq!(*auth_addr, engine);
    assert_eq!(
        auth_inv.function,
        AuthorizedFunction::Contract((
            client.address.clone(),
            Symbol::new(&env, "seize_collateral"),
            (&engine, &user, &token_id, &200_i128).into_val(&env),
        ))
    );
}

#[test]
fn test_seize_unregistered_engine_fails() {
    let (env, client, _admin, token_id, _oracle, token_admin_sac) = base_setup();
    let engine = Address::generate(&env);
    let malicious = Address::generate(&env);
    let user = Address::generate(&env);

    env.mock_all_auths();
    client.set_liquidation_engine(&engine);
    token_admin_sac.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let res = client.try_seize_collateral(&malicious, &user, &token_id, &200);
    assert!(res.is_err());
}

#[test]
fn test_seize_no_position_fails() {
    let (env, client, _admin, token_id, _oracle, _token_admin_sac) = base_setup();
    let engine = Address::generate(&env);
    let user = Address::generate(&env);

    env.mock_all_auths();
    client.set_liquidation_engine(&engine);

    let res = client.try_seize_collateral(&engine, &user, &token_id, &100);
    assert!(res.is_err());
    assert!(!client.get_position_index().contains(&user));
}

#[test]
fn test_seize_excess_amount_rollback() {
    let (env, client, _admin, token_id, _oracle, token_admin_sac) = base_setup();
    let engine = Address::generate(&env);
    let user = Address::generate(&env);

    env.mock_all_auths();
    client.set_liquidation_engine(&engine);
    token_admin_sac.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let balance_before = client.get_position_balance(&user, &token_id);
    let res = client.try_seize_collateral(&engine, &user, &token_id, &600);
    assert!(res.is_err());
    assert_eq!(client.get_position_balance(&user, &token_id), balance_before);
}

// ---------------------------------------------------------------------------
// set_oracle / set_liquidation_engine — admin-only
// ---------------------------------------------------------------------------

#[test]
fn test_set_oracle_requires_admin_auth() {
    let (env, client, admin, _token_id, _oracle, _token_admin) = base_setup();
    let new_oracle = Address::generate(&env);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "set_oracle",
            args: (&new_oracle,).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    client.set_oracle(&new_oracle);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    let (auth_addr, _) = auths.first().unwrap();
    assert_eq!(*auth_addr, admin);
}

#[test]
fn test_set_oracle_non_admin_fails() {
    let (env, client, _admin, _token_id, _oracle, _token_admin) = base_setup();
    let attacker = Address::generate(&env);
    let new_oracle = Address::generate(&env);

    env.mock_auths(&[MockAuth {
        address: &attacker,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "set_oracle",
            args: (&new_oracle,).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let res = client.try_set_oracle(&new_oracle);
    assert!(res.is_err());
}

#[test]
fn test_set_liquidation_engine_requires_admin_auth() {
    let (env, client, admin, _token_id, _oracle, _token_admin) = base_setup();
    let engine = Address::generate(&env);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "set_liquidation_engine",
            args: (&engine,).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    client.set_liquidation_engine(&engine);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    let (auth_addr, _) = auths.first().unwrap();
    assert_eq!(*auth_addr, admin);
}
