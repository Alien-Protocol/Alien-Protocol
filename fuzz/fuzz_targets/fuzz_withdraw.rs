#![no_main]

use collateral_vault::{VaultContract, VaultContractClient};
use libfuzzer_sys::fuzz_target;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env};

fuzz_target!(|amount: i128| {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let oracle = Address::generate(&env);
    let pool = Address::generate(&env);
    let engine = Address::generate(&env);

    let _ = client.try_initialize(&admin, &pool, &oracle, &engine);

    let token_admin = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(token_admin);
    let asset = sac.address();
    let _ = client.try_add_supported_asset(&asset);

    // Arbitrary i128 withdraw amount must never panic the host
    let _ = client.try_withdraw(&user, &asset, &amount);
});
