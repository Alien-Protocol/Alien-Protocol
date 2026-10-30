#![no_main]

use libfuzzer_sys::fuzz_target;
use liquidation_engine::{LiquidationContract, LiquidationContractClient};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env};

fuzz_target!(|max_repay_amount: i128| {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LiquidationContract, ());
    let client = LiquidationContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let liquidator = Address::generate(&env);
    let user = Address::generate(&env);
    let vault = Address::generate(&env);
    let pool = Address::generate(&env);
    let oracle = Address::generate(&env);

    let _ = client.try_initialize(&admin, &vault, &pool, &oracle);

    // Arbitrary i128 max_repay_amount must never panic the host
    let _ = client.try_liquidate(&liquidator, &user, &max_repay_amount);
});
