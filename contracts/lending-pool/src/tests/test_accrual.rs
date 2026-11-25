#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{Address, Env};

fn setup_env() -> (
    Env,
    PoolContractClient<'static>,
    Address,
    Address,
    Address,
    Address,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PoolContract, ());
    let client = PoolContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let vault = Address::generate(&env);
    let oracle = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_contract_id = token_contract.address();

    client.initialize(&admin, &vault, &oracle, &token_contract_id, &800);

    (env, client, admin, user, vault, token_contract_id)
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
fn test_get_debt_empty_user_is_zero() {
    let (_env, client, _admin, user, _vault, _token_id) = setup_env();

    let debt = client.get_debt(&user);
    assert_eq!(debt.principal, 0);
    assert_eq!(debt.accrued_interest, 0);
    assert_eq!(debt.interest_rate_bps, 0);
    assert_eq!(debt.last_accrual_at, 0);
    assert_eq!(debt.total().unwrap(), 0);

    let user_debt = client.get_user_debt(&user);
    assert_eq!(user_debt, 0);
}

#[test]
fn test_accrue_one_year_eight_percent() {
    let (env, client, _admin, user, _vault, _token_id) = setup_env();

    let start_time = 1_000u64;
    env.ledger().set_timestamp(start_time);

    let principal = 1_000_000_000i128;
    let rate_bps = 800u32; // 8% APR
    set_user_debt(
        &env,
        &client.address,
        &user,
        principal,
        0,
        rate_bps,
        start_time,
    );

    // Zero elapsed adds zero interest
    let debt = client.accrue_interest(&user);
    assert_eq!(debt.principal, principal);
    assert_eq!(debt.accrued_interest, 0);
    assert_eq!(debt.last_accrual_at, start_time);

    // Advance timestamp by exactly one year (31_536_000 seconds)
    let one_year = shared::constant::SECONDS_PER_YEAR;
    env.ledger().set_timestamp(start_time + one_year);

    let updated = client.accrue_interest(&user);
    assert_eq!(updated.principal, principal);
    assert_eq!(updated.accrued_interest, 80_000_000);
    assert_eq!(updated.last_accrual_at, start_time + one_year);
    assert_eq!(updated.total().unwrap(), 1_080_000_000);

    // Also verify for principal 100_000_000 at 8% APR adds exactly 8_000_000
    let user2 = Address::generate(&env);
    set_user_debt(
        &env,
        &client.address,
        &user2,
        100_000_000,
        0,
        rate_bps,
        start_time,
    );
    let updated2 = client.accrue_interest(&user2);
    assert_eq!(updated2.principal, 100_000_000);
    assert_eq!(updated2.accrued_interest, 8_000_000);
    assert_eq!(updated2.total().unwrap(), 108_000_000);
}

#[test]
fn test_accrue_twice_in_same_ledger_does_not_double_count() {
    let (env, client, _admin, user, _vault, _token_id) = setup_env();

    let start_time = 1_000u64;
    env.ledger().set_timestamp(start_time);

    let principal = 1_000_000_000i128;
    let rate_bps = 800u32;
    set_user_debt(
        &env,
        &client.address,
        &user,
        principal,
        0,
        rate_bps,
        start_time,
    );

    let one_year = shared::constant::SECONDS_PER_YEAR;
    env.ledger().set_timestamp(start_time + one_year);

    let first = client.accrue_interest(&user);
    assert_eq!(first.accrued_interest, 80_000_000);
    assert_eq!(first.last_accrual_at, start_time + one_year);

    // Second accrual in the same ledger timestamp
    let second = client.accrue_interest(&user);
    assert_eq!(second.accrued_interest, 80_000_000);
    assert_eq!(second.principal, principal);
    assert_eq!(second.last_accrual_at, start_time + one_year);
}

#[test]
fn test_get_debt_accrues_before_read() {
    let (env, client, _admin, user, _vault, _token_id) = setup_env();

    let start_time = 1_000u64;
    env.ledger().set_timestamp(start_time);

    let principal = 1_000_000_000i128;
    let rate_bps = 800u32;
    set_user_debt(
        &env,
        &client.address,
        &user,
        principal,
        0,
        rate_bps,
        start_time,
    );

    let one_year = shared::constant::SECONDS_PER_YEAR;
    env.ledger().set_timestamp(start_time + one_year);

    // get_debt accrues interest before returning
    let debt = client.get_debt(&user);
    assert_eq!(debt.principal, principal);
    assert_eq!(debt.accrued_interest, 80_000_000);
    assert_eq!(debt.last_accrual_at, start_time + one_year);

    // Verify storage was updated
    let debt_cached = client.get_debt(&user);
    assert_eq!(debt_cached.accrued_interest, 80_000_000);
}

#[test]
fn test_get_user_debt_matches_debt_total() {
    let (env, client, _admin, user, _vault, _token_id) = setup_env();

    // Empty debt returns 0
    assert_eq!(client.get_user_debt(&user), 0);

    let start_time = 1_000u64;
    env.ledger().set_timestamp(start_time);

    let principal = 1_000_000_000i128;
    let initial_interest = 5_000_000i128;
    let rate_bps = 800u32;
    set_user_debt(
        &env,
        &client.address,
        &user,
        principal,
        initial_interest,
        rate_bps,
        start_time,
    );

    let one_year = shared::constant::SECONDS_PER_YEAR;
    env.ledger().set_timestamp(start_time + one_year);

    let user_debt = client.get_user_debt(&user);
    let debt = client.get_debt(&user);

    assert_eq!(user_debt, debt.total().unwrap());
    assert_eq!(user_debt, principal + initial_interest + 80_000_000);
}
