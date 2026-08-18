#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::{Address as _, MockAuth, MockAuthInvoke};
use soroban_sdk::{token, Address, Bytes, BytesN, Env, IntoVal};

const TEST_WASM: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../target/wasm32-unknown-unknown/release/collateral_vault.wasm"
));

fn setup_env() -> (
    Env,
    Address,
    VaultContractClient<'static>,
    Address,
    Address,
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
    let lending_pool = Address::generate(&env);
    let liquidation_engine = Address::generate(&env);

    client.initialize(&admin, &lending_pool, &oracle, &liquidation_engine);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_id = token_contract.address();
    let token_client = token::Client::new(&env, &token_id);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);

    client.add_supported_asset(&token_id);

    (
        env,
        contract_id,
        client,
        admin,
        user,
        oracle,
        token_id,
        token_client,
        token_admin_client,
    )
}

#[test]
#[ignore] 
fn test_upgrade_and_migrate_preserve_state() {
    let (env, contract_id, client, admin, user, oracle, token_id, token_client, token_admin) =
        setup_env();

    token_admin.mint(&user, &1_000);
    client.deposit(&user, &token_id, &500);

    env.as_contract(&contract_id, || {
        storage::set_contract_version(&env, 1);
        storage::set_storage_schema_version(&env, 1);
    });

    let wasm = Bytes::from_slice(&env, TEST_WASM);
    let wasm_hash = env.deployer().upload_contract_wasm(wasm);

    client.upgrade(&wasm_hash);
    client.migrate(&2);

    assert_eq!(client.get_contract_version(), 2);
    assert_eq!(client.get_storage_schema_version(), 2);
    assert_eq!(client.get_admin(), Some(admin));
    env.as_contract(&contract_id, || {
        assert_eq!(storage::get_oracle(&env), Some(oracle));
    });
    assert!(client.is_supported_asset(&token_id));
    assert_eq!(client.get_position_balance(&user, &token_id), 500);
    assert_eq!(token_client.balance(&user), 500);
    assert_eq!(token_client.balance(&contract_id), 500);
    assert_eq!(client.get_position(&user).collateral.len(), 1);
}

#[test]
fn test_upgrade_rejects_unauthorized_address() {
    let env = Env::default();
    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    let attacker = Address::generate(&env);
    let lending_pool = Address::generate(&env);
    let liquidation_engine = Address::generate(&env);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "initialize",
            args: (
                admin.clone(),
                lending_pool.clone(),
                oracle.clone(),
                liquidation_engine.clone(),
            )
                .into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.initialize(&admin, &lending_pool, &oracle, &liquidation_engine);

    let wasm_hash = BytesN::from_array(&env, &[9u8; 32]);

    env.mock_auths(&[MockAuth {
        address: &attacker,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "upgrade",
            args: (wasm_hash.clone(),).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let result = client.try_upgrade(&wasm_hash);
    assert!(result.is_err());
}

#[test]
fn test_migration_rejects_repeated_and_out_of_order_versions() {
    let (env, contract_id, _client, _admin, _user, _oracle, _token_id, _token_client, _token_admin) =
        setup_env();

    env.as_contract(&contract_id, || {
        storage::set_storage_schema_version(&env, 1);
    });

    let first = VaultContractClient::new(&env, &contract_id).try_migrate(&2);
    assert!(first.is_ok());

    let repeated = VaultContractClient::new(&env, &contract_id).try_migrate(&2);
    assert!(matches!(
        repeated,
        Err(Ok(VaultError::MigrationAlreadyApplied))
    ));

    let out_of_order = VaultContractClient::new(&env, &contract_id).try_migrate(&1);
    assert!(matches!(
        out_of_order,
        Err(Ok(VaultError::MigrationOutOfOrder))
    ));

    let skipped = VaultContractClient::new(&env, &contract_id).try_migrate(&3);
    assert!(matches!(skipped, Err(Ok(VaultError::MigrationSkipped))));
}
