#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{token, Address, Env};
use collateral_vault::{VaultContract, VaultContractClient};

fuzz_target!(|amount: i128| {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(VaultContract, (&admin,));
    let client = VaultContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    let pool = Address::generate(&env);
    let oracle = Address::generate(&env);
    let liquidation_engine = Address::generate(&env);

    client.initialize(&admin, &pool, &oracle, &liquidation_engine);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_id = token_contract.address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);

    client.add_supported_asset(&token_id, &shared::AssetConfig::default());

    if amount > 0 {
        token_admin_client.mint(&user, &amount);
    }

    // Must never panic on arbitrary i128
    let res = client.try_deposit(&user, &token_id, &amount);

    if amount <= 0 {
        assert!(res.is_err(), "Deposit must fail for non-positive amount");
        let bal = client.get_position_balance(&user, &token_id);
        assert_eq!(bal, 0, "Non-positive deposit must never credit");
    }
});
