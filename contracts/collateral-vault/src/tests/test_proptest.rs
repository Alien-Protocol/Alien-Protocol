#![cfg(test)]

use super::super::*;
use proptest::prelude::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{contract, contractimpl, token, Address, Env};

#[contract]
pub struct MockPropLendingPool;

#[contractimpl]
impl MockPropLendingPool {
    pub fn get_user_debt(_env: Env, _user: Address) -> i128 {
        0
    }

    pub fn is_liquidatable(_env: Env, _user: Address) -> bool {
        false
    }
}

fn setup_vault_env() -> (
    Env,
    VaultContractClient<'static>,
    Address,
    Address,
    token::Client<'static>,
    token::StellarAssetClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let oracle = Address::generate(&env);
    let pool = env.register(MockPropLendingPool, ());
    let engine = Address::generate(&env);

    client.initialize(&admin, &pool, &oracle, &engine);

    let token_admin = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(token_admin);
    let asset = sac.address();
    let token_client = token::Client::new(&env, &asset);
    let token_admin_client = token::StellarAssetClient::new(&env, &asset);

    client.add_supported_asset(&asset);

    (env, client, user, asset, token_client, token_admin_client)
}

proptest! {
    #[test]
    fn prop_deposit_withdraw_round_trip_zero_debt(
        amount in 1i128..=1_000_000_000i128,
    ) {
        let (_env, client, user, asset, token_client, token_admin) = setup_vault_env();

        token_admin.mint(&user, &amount);
        let initial_balance = token_client.balance(&user);
        prop_assert_eq!(initial_balance, amount);

        // Deposit amount
        client.deposit(&user, &asset, &amount);
        prop_assert_eq!(client.get_position_balance(&user, &asset), amount);
        prop_assert_eq!(token_client.balance(&user), 0);

        // Withdraw amount back with zero debt
        client.withdraw(&user, &asset, &amount);
        prop_assert_eq!(client.get_position_balance(&user, &asset), 0);
        prop_assert_eq!(token_client.balance(&user), amount);
    }

    #[test]
    fn prop_non_positive_deposit_never_credits(
        amount in any::<i128>().prop_filter("non-positive only", |x| *x <= 0),
    ) {
        let (_env, client, user, asset, token_client, _token_admin) = setup_vault_env();

        let initial_position = client.get_position_balance(&user, &asset);
        let res = client.try_deposit(&user, &asset, &amount);
        prop_assert!(res.is_err());
        prop_assert_eq!(client.get_position_balance(&user, &asset), initial_position);
        prop_assert_eq!(token_client.balance(&user), 0);
    }
}
