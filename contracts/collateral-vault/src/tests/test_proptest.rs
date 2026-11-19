#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{token, Address, Env};

#[contract]
pub struct MockLendingPool;

#[contractimpl]
impl MockLendingPool {
    pub fn get_user_debt(env: Env, _user: Address) -> i128 {
        env.storage().persistent().get(&"debt").unwrap_or(0)
    }

    pub fn set_user_debt(env: Env, debt: i128) {
        env.storage().persistent().set(&"debt", &debt);
    }
}

#[contract]
pub struct MockOracle;

#[contractimpl]
impl MockOracle {
    pub fn get_price(env: Env, asset: Address) -> Option<types::PriceData> {
        env.storage().persistent().get(&asset)
    }

    pub fn get_price_or_fail(env: Env, asset: Address) -> types::PriceData {
        match env.storage().persistent().get(&asset) {
            Some(pd) => pd,
            None => panic!("price not found"),
        }
    }

    pub fn set_price(env: Env, asset: Address, price: i128, timestamp: u64) {
        let price_data = types::PriceData {
            price,
            timestamp,
            write_timestamp: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&asset, &price_data);
    }
}

fn setup_env() -> (
    Env,
    VaultContractClient<'static>,
    Address,
    Address,
    Address,
    token::Client<'static>,
    token::StellarAssetClient<'static>,
    MockLendingPoolClient<'static>,
    MockOracleClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1000);

    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    let oracle_id = env.register(MockOracle, ());
    let oracle_client = MockOracleClient::new(&env, &oracle_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);

    let pool_id = env.register(MockLendingPool, ());
    let pool_client = MockLendingPoolClient::new(&env, &pool_id);

    let liquidation_engine = Address::generate(&env);

    client.initialize(&admin, &pool_id, &oracle_id, &liquidation_engine);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_contract_id = token_contract.address();
    let token_client = token::Client::new(&env, &token_contract_id);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract_id);

    client.add_supported_asset(&token_contract_id);

    // Default price: 1 token = 100 USD (7 decimals)
    oracle_client.set_price(&token_contract_id, &1_000_000_000, &1000);

    (
        env,
        client,
        admin,
        user,
        token_contract_id,
        token_client,
        token_admin_client,
        pool_client,
        oracle_client,
    )
}

#[test]
fn prop_deposit_withdraw_round_trip_zero_debt() {
    let (
        _env,
        client,
        _admin,
        user,
        asset,
        token_client,
        token_admin,
        _pool_client,
        _oracle_client,
    ) = setup_env();

    let test_amounts = [1i128, 100, 10_000, 500_000, 1_000_000_000, 10_000_000_000];

    for amount in test_amounts {
        token_admin.mint(&user, &amount);
        let initial_token_bal = token_client.balance(&user);

        // Deposit
        let res_dep = client.try_deposit(&user, &asset, &amount);
        assert!(res_dep.is_ok(), "Deposit failed for amount {}", amount);

        let vault_bal = client.get_position_balance(&user, &asset);
        assert_eq!(vault_bal, amount);

        // Withdraw round-trip when debt is 0
        let res_wd = client.try_withdraw(&user, &asset, &amount);
        assert!(res_wd.is_ok(), "Withdraw failed for amount {}", amount);

        let final_vault_bal = client.get_position_balance(&user, &asset);
        assert_eq!(
            final_vault_bal, 0,
            "Position not cleared after full withdraw"
        );

        let final_token_bal = token_client.balance(&user);
        assert_eq!(final_token_bal, initial_token_bal);
    }
}

#[test]
fn prop_non_positive_deposit_never_credits() {
    let (
        _env,
        client,
        _admin,
        user,
        asset,
        _token_client,
        _token_admin,
        _pool_client,
        _oracle_client,
    ) = setup_env();

    let test_amounts = [
        0i128,
        -1,
        -100,
        -10_000,
        -1_000_000_000,
        i128::MIN + 1,
        i128::MIN,
    ];

    for amount in test_amounts {
        let res = client.try_deposit(&user, &asset, &amount);
        assert!(
            res.is_err(),
            "Deposit must fail for non-positive amount {}",
            amount
        );

        let vault_bal = client.get_position_balance(&user, &asset);
        assert_eq!(
            vault_bal, 0,
            "Balance must never credit for non-positive amount {}",
            amount
        );
    }
}
