#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{token, Address, Env, Symbol};

fn setup_env() -> (
    Env,
    PoolContractClient<'static>,
    Address,
    Address,
    Address,
    Address,
    token::Client<'static>,
    token::StellarAssetClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(PoolContract, (&admin,));
    let client = PoolContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    let vault = Address::generate(&env);
    let oracle = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_contract_id = token_contract.address();
    let token_client = token::Client::new(&env, &token_contract_id);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract_id);

    client.initialize(&admin, &vault, &oracle, &token_contract_id, &500);

    (
        env,
        client,
        admin,
        user,
        vault,
        token_contract_id,
        token_client,
        token_admin_client,
    )
}

#[test]
fn test_pause_and_unpause_borrow() {
    let (env, client, _admin, _user, _vault, _token_id, _token_client, _token_admin) = setup_env();

    client.pause_operation(&PauseFlag::Borrow, &Symbol::new(&env, "test"));

    let res = client.try_unpause_operation(&PauseFlag::Borrow);
    assert!(res.is_ok());
}

#[test]
fn test_double_pause_fails() {
    let (env, client, _admin, _user, _vault, _token_id, _token_client, _token_admin) = setup_env();

    client.pause_operation(&PauseFlag::Borrow, &Symbol::new(&env, "test"));

    let res = client.try_pause_operation(&PauseFlag::Borrow, &Symbol::new(&env, "test"));
    assert!(res.is_err());
}

#[test]
fn test_unpause_when_not_paused_fails() {
    let (_env, client, _admin, _user, _vault, _token_id, _token_client, _token_admin) = setup_env();

    let res = client.try_unpause_operation(&PauseFlag::Borrow);
    assert!(res.is_err());
}

#[test]
fn test_pool_is_operation_paused_matches_mask() {
    let (env, client, _admin, _user, _vault, _token_id, _token_client, _token_admin) = setup_env();

    let flags = [
        PauseFlag::Supply,
        PauseFlag::Borrow,
        PauseFlag::Repay,
        PauseFlag::WithdrawLiquidity,
    ];

    // Initially, all operations should be unpaused
    for flag in flags.iter() {
        assert!(!client.is_operation_paused(flag));
    }

    let reason = Symbol::new(&env, "pause_test");

    // Pause each operation one by one and assert getter matches mask
    for (i, flag) in flags.iter().enumerate() {
        client.pause_operation(flag, &reason);

        // Checked flag must now be paused
        assert!(client.is_operation_paused(flag));

        // Verify all flags against expectation
        for (j, check_flag) in flags.iter().enumerate() {
            if j <= i {
                assert!(client.is_operation_paused(check_flag));
            } else {
                assert!(!client.is_operation_paused(check_flag));
            }
        }
    }

    // Unpause each operation one by one and assert getter reflects changes
    for (i, flag) in flags.iter().enumerate() {
        client.unpause_operation(flag);

        // Checked flag must now be unpaused
        assert!(!client.is_operation_paused(flag));

        // Verify remaining flags
        for (j, check_flag) in flags.iter().enumerate() {
            if j <= i {
                assert!(!client.is_operation_paused(check_flag));
            } else {
                assert!(client.is_operation_paused(check_flag));
            }
        }
    }

    // All operations are unpaused again
    for flag in flags.iter() {
        assert!(!client.is_operation_paused(flag));
    }
}
