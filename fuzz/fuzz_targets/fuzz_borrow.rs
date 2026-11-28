#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{contract, contractimpl, Address, Env};
use lending_pool::{PoolContract, PoolContractClient};

#[contract]
pub struct MockVault;

#[contractimpl]
impl MockVault {
    pub fn can_originate(_env: Env, _pool: Address) -> bool {
        true
    }

    pub fn get_position(env: Env, user: Address) -> shared::Position {
        let mut collateral = soroban_sdk::Vec::new(&env);
        collateral.push_back(shared::CollateralAsset {
            asset: Address::generate(&env),
            amount: 10_000_000,
        });
        shared::Position { user, collateral }
    }

    pub fn get_asset_config(_env: Env, _asset: Address) -> shared::AssetConfig {
        shared::AssetConfig {
            token_decimals: 7,
            oracle_price_decimals: 7,
            max_ltv_bps: 8_000,
            liquidation_threshold_bps: 8_500,
        }
    }

    pub fn get_collateral_value(_env: Env, _user: Address) -> i128 {
        100_000_000
    }

    pub fn get_health_factor(_env: Env, _user: Address) -> i128 {
        15_000
    }
}

fuzz_target!(|amount: i128| {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(PoolContract, (&admin,));
    let client = PoolContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    let vault_id = env.register(MockVault, ());
    let oracle = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_id = token_contract.address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);

    client.initialize(&admin, &vault_id, &oracle, &token_id, &1_000);

    let lp = Address::generate(&env);
    token_admin_client.mint(&lp, &100_000_000);
    let _ = client.try_supply(&lp, &100_000_000);

    // Call try_borrow with arbitrary i128 amount: must never panic/abort
    let res = client.try_borrow(&user, &token_id, &amount);
    if amount <= 0 {
        assert!(res.is_err(), "Borrow must fail for non-positive amount");
    }
});
