#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env};
use liquidation_engine::{LiquidationEngine, LiquidationEngineClient};

fuzz_target!(|amount: i128| {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(LiquidationEngine, ());
    let client = LiquidationEngineClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let vault = Address::generate(&env);
    let pool = Address::generate(&env);
    let oracle = Address::generate(&env);

    client.initialize(&admin, &vault, &pool, &oracle);

    let liquidator = Address::generate(&env);
    let user = Address::generate(&env);

    // Call try_liquidate with arbitrary i128 amount: must never panic/abort
    let res = client.try_liquidate(&liquidator, &user, &amount);
    if amount <= 0 {
        assert!(res.is_err(), "Liquidate must fail for non-positive amount");
    }
});
