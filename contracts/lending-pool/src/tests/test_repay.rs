
#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{contract, contractimpl, token, Address, Env, Symbol};

#[contract]
pub struct MockVaultContract;

#[contractimpl]
impl MockVaultContract {
    pub fn set_health_factor(env: Env, user: Address, hf: i128) {
        env.storage().persistent().set(&user, &hf);
    }

    pub fn get_health_factor(env: Env, user: Address) -> i128 {
        env.storage().persistent().get(&user).unwrap_or(15_000)
    }

    pub fn get_collateral_value(_env: Env, _user: Address) -> i128 {
        0
    }

    pub fn get_position(_env: Env, _user: Address) -> shared::Position {
        unimplemented!()
    }

    pub fn get_asset_config(_env: Env, _asset: Address) -> shared::AssetConfig {
        unimplemented!()
    }
}

fn setup_env() -> (
    Env,
    PoolContractClient<'static>,
    Address,
    Address,
    MockVaultContractClient<'static>,
    Address,
    Address,
    token::Client<'static>,
    token::StellarAssetClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PoolContract, ());
    let client = PoolContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);

    let vault_id = env.register(MockVaultContract, ());
    let vault_client = MockVaultContractClient::new(&env, &vault_id);
    let oracle = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_contract_id = token_contract.address();
    let token_client = token::Client::new(&env, &token_contract_id);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract_id);

    let interest_rate_bps = 1000; // 10% annual interest
    client.initialize(
        &admin,
        &vault_id,
        &oracle,
        &token_contract_id,
        &interest_rate_bps,
    );

    (
        env,
        client,
        admin,
        user,
        vault_client,
        vault_id,
        token_contract_id,
        token_client,
        token_admin_client,
    )
}

fn set_user_debt(
    env: &Env,
    pool_address: &Address,
    user: &Address,
    principal: i128,
    accrued_interest: i128,
    interest_rate_bps: u32,
    last_accrual_at: u64,
) {
    env.as_contract(pool_address, || {
        let debt = Debt {
            principal,
            accrued_interest,
            interest_rate_bps,
            last_accrual_at,
        };
        storage::set_debt(env, user, &debt);
        let current_borrowed = storage::get_total_borrowed(env);
        storage::set_total_borrowed(env, current_borrowed + principal);
    });
}

#[test]
fn test_repay_interest_before_principal() {
    let (env, client, _admin, user, _vault, _vault_id, token_id, token_client, token_admin) =
        setup_env();

    let initial_principal = 100_000_000i128;
    let initial_interest = 10_000_000i128;
    let now = env.ledger().timestamp();
    set_user_debt(
        &env,
        &client.address,
        &user,
        initial_principal,
        initial_interest,
        1000,
        now,
    );

    token_admin.mint(&user, &5_000_000);

    client.repay(&user, &token_id, &5_000_000);

    let updated_debt = client.get_debt(&user);
    assert_eq!(updated_debt.accrued_interest, 5_000_000);
    assert_eq!(updated_debt.principal, 100_000_000);
    assert_eq!(token_client.balance(&user), 0);
    assert_eq!(token_client.balance(&client.address), 5_000_000);

    env.as_contract(&client.address, || {
        assert_eq!(storage::get_total_borrowed(&env), 100_000_000);
    });
}

#[test]
fn test_repay_covers_interest_then_principal() {
    let (env, client, _admin, user, _vault, _vault_id, token_id, token_client, token_admin) =
        setup_env();

    let initial_principal = 100_000_000i128;
    let initial_interest = 10_000_000i128;
    let now = env.ledger().timestamp();
    set_user_debt(
        &env,
        &client.address,
        &user,
        initial_principal,
        initial_interest,
        1000,
        now,
    );

    token_admin.mint(&user, &30_000_000);

    client.repay(&user, &token_id, &30_000_000);

    let updated_debt = client.get_debt(&user);
    assert_eq!(updated_debt.accrued_interest, 0);
    assert_eq!(updated_debt.principal, 80_000_000);
    assert_eq!(token_client.balance(&user), 0);
    assert_eq!(token_client.balance(&client.address), 30_000_000);

    env.as_contract(&client.address, || {
        assert_eq!(storage::get_total_borrowed(&env), 80_000_000);
    });
}

#[test]
fn test_repay_full_clears_debt_and_index() {
    let (env, client, _admin, user, _vault, _vault_id, token_id, token_client, token_admin) =
        setup_env();

    let initial_principal = 100_000_000i128;
    let initial_interest = 10_000_000i128;
    let now = env.ledger().timestamp();
    set_user_debt(
        &env,
        &client.address,
        &user,
        initial_principal,
        initial_interest,
        1000,
        now,
    );

    token_admin.mint(&user, &150_000_000);

    client.repay(&user, &token_id, &150_000_000);

    env.as_contract(&client.address, || {
        assert_eq!(storage::get_debt(&env, &user), None);
        assert_eq!(storage::get_total_borrowed(&env), 0);
    });

    assert_eq!(client.get_user_debt(&user), 0);
    assert_eq!(token_client.balance(&user), 40_000_000);
    assert_eq!(token_client.balance(&client.address), 110_000_000);
}

#[test]
fn test_repay_zero_or_negative_fails() {
    let (_env, client, _admin, user, _vault, _vault_id, token_id, _token_client, token_admin) =
        setup_env();

    token_admin.mint(&user, &1000);

    let res_zero = client.try_repay(&user, &token_id, &0);
    assert!(res_zero.is_err());

    let res_neg = client.try_repay(&user, &token_id, &-100);
    assert!(res_neg.is_err());
}

#[test]
fn test_repay_when_paused_fails() {
    let (env, client, _admin, user, _vault, _vault_id, token_id, _token_client, token_admin) =
        setup_env();

    let now = env.ledger().timestamp();
    set_user_debt(&env, &client.address, &user, 100_000_000, 0, 1000, now);
    token_admin.mint(&user, &100_000_000);

    client.pause_operation(&PauseFlag::Repay, &Symbol::new(&env, "test"));

    let res = client.try_repay(&user, &token_id, &50_000_000);
    assert!(res.is_err());
}

#[test]
fn test_repay_wrong_asset_fails() {
    let (env, client, _admin, user, _vault, _vault_id, _token_id, _token_client, _token_admin) =
        setup_env();

    let wrong_token_admin = Address::generate(&env);
    let wrong_token = env
        .register_stellar_asset_contract_v2(wrong_token_admin)
        .address();

    let res = client.try_repay(&user, &wrong_token, &1000);
    assert!(res.is_err());
}

#[test]
fn test_repay_dust_remaining_fails_unless_full_close() {
    let (env, client, _admin, user, _vault, _vault_id, token_id, _token_client, token_admin) =
        setup_env();

    let total_debt = shared::MIN_REMAINING_DEBT + 50;
    let now = env.ledger().timestamp();
    set_user_debt(&env, &client.address, &user, total_debt, 0, 1000, now);
    token_admin.mint(&user, &total_debt);

    let res_dust = client.try_repay(&user, &token_id, &100);
    assert!(res_dust.is_err());

    let res_full = client.try_repay(&user, &token_id, &total_debt);
    assert!(res_full.is_ok());
}

#[test]
fn test_is_liquidatable_false_when_no_debt() {
    let (_env, client, _admin, user, vault, _vault_id, _token_id, _token_client, _token_admin) =
        setup_env();

    vault.set_health_factor(&user, &5000);

    assert!(!client.is_liquidatable(&user));
}

#[test]
fn test_is_liquidatable_true_when_hf_below_one() {
    let (env, client, _admin, user, vault, _vault_id, _token_id, _token_client, _token_admin) =
        setup_env();

    let now = env.ledger().timestamp();
    set_user_debt(&env, &client.address, &user, 100_000_000, 0, 1000, now);

    vault.set_health_factor(&user, &10_500);
    assert!(!client.is_liquidatable(&user));

    vault.set_health_factor(&user, &9_500);
    assert!(client.is_liquidatable(&user));
}
