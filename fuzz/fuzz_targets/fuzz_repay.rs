#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{token, Address, Env};
use lending_pool::{PoolContract, PoolContractClient};

fuzz_target!(|amount: i128| {
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
    let token_id = token_contract.address();

    client.initialize(&admin, &vault, &oracle, &token_id, &1_000);

    // Call try_repay with arbitrary i128 amount: must never panic/abort
    let res = client.try_repay(&user, &token_id, &amount);
    if amount <= 0 {
        assert!(res.is_err(), "Repay must fail for non-positive amount");
    }
});
