#![no_main]

use lending_pool::{PoolContract, PoolContractClient};
use libfuzzer_sys::fuzz_target;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env};

fuzz_target!(|amount: i128| {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PoolContract, ());
    let client = PoolContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let oracle = Address::generate(&env);
    let vault = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(token_admin);
    let borrow_asset = sac.address();

    let _ = client.try_initialize(&admin, &vault, &oracle, &borrow_asset, &500);

    // Arbitrary i128 borrow amount must never panic the host
    let _ = client.try_borrow(&user, &borrow_asset, &amount);
});
