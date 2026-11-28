#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::{Address as _, MockAuth, MockAuthInvoke};
use soroban_sdk::{token, Address, Bytes, BytesN, Env, IntoVal};

// Requires the wasm32v1-none artifact to exist before `cargo test` compiles this
// crate (include_bytes! reads it at compile time), so run:
//   cargo build -p collateral-vault --target wasm32v1-none --release
// first. CI already does this in the "Build collateral-vault WASM for upgrade
// tests" step of .github/workflows/contract.yml.
//
// Building for wasm32v1-none (rather than wasm32-unknown-unknown) is also what
// keeps this test working on soroban-env-host v23: since Rust 1.82, the
// general wasm32-unknown-unknown target enables the WASM reference-types and
// multivalue proposals by default, which the Soroban host's module validator
// rejects on upload_contract_wasm. wasm32v1-none pins the original WASM MVP
// feature set (no reference-types/multivalue), so uploads validate cleanly.
const TEST_WASM: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../target/wasm32v1-none/release/collateral_vault.wasm"
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

    let admin = Address::generate(&env);
    let contract_id = env.register(VaultContract, (&admin,));
    let client = VaultContractClient::new(&env, &contract_id);

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

    client.add_supported_asset(&token_id, &shared::AssetConfig::default());

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

    client.upgrade(&wasm_hash, &3);
    client.finalize_upgrade();
    assert!(client.try_finalize_upgrade().is_err());
    client.migrate(&2);

    assert_eq!(client.get_contract_version(), 3);
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
    let admin = Address::generate(&env);
    let contract_id = env.register(VaultContract, (&admin,));
    let client = VaultContractClient::new(&env, &contract_id);

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
            args: (wasm_hash.clone(), 3u32).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let result = client.try_upgrade(&wasm_hash, &3);
    assert!(result.is_err());
}

#[test]
fn test_migration_rejects_repeated_and_out_of_order_versions() {
    let (env, contract_id, client, admin, user, _oracle, token_id, token_client, token_admin) =
        setup_env();

    token_admin.mint(&user, &1_000);
    client.deposit(&user, &token_id, &500);

    env.as_contract(&contract_id, || {
        storage::set_storage_schema_version(&env, 1);
    });

    let first = client.try_migrate(&2);
    assert!(first.is_ok());
    assert_eq!(client.get_storage_schema_version(), 2);

    let repeated = client.try_migrate(&2);
    assert!(matches!(
        repeated,
        Err(Ok(VaultError::MigrationAlreadyApplied))
    ));
    assert_eq!(client.get_storage_schema_version(), 2);
    assert_eq!(client.get_admin(), Some(admin.clone()));
    assert_eq!(client.get_position_balance(&user, &token_id), 500);
    assert_eq!(token_client.balance(&user), 500);

    let out_of_order = client.try_migrate(&1);
    assert!(matches!(
        out_of_order,
        Err(Ok(VaultError::MigrationOutOfOrder))
    ));
    assert_eq!(client.get_storage_schema_version(), 2);
    assert_eq!(client.get_admin(), Some(admin.clone()));
    assert_eq!(client.get_position_balance(&user, &token_id), 500);
    assert_eq!(token_client.balance(&user), 500);

    let skipped = client.try_migrate(&4);
    assert!(matches!(skipped, Err(Ok(VaultError::MigrationSkipped))));
    assert_eq!(client.get_storage_schema_version(), 2);
    assert_eq!(client.get_admin(), Some(admin));
    assert_eq!(client.get_position_balance(&user, &token_id), 500);
    assert_eq!(token_client.balance(&user), 500);
}

#[test]
fn test_migrate_preserves_state_native() {
    let (env, contract_id, client, admin, user, oracle, token_id, token_client, token_admin) =
        setup_env();

    token_admin.mint(&user, &1_000);
    client.deposit(&user, &token_id, &500);

    // Set schema version to 1 in storage to exercise migration natively without WASM
    env.as_contract(&contract_id, || {
        storage::set_contract_version(&env, 1);
        storage::set_storage_schema_version(&env, 1);
    });

    // Record state BEFORE migration
    let before_contract_version = client.get_contract_version();
    let before_schema_version = client.get_storage_schema_version();
    let before_admin = client.get_admin();
    let before_position_balance = client.get_position_balance(&user, &token_id);
    let before_position = client.get_position(&user);
    let before_all_positions = client.get_all_positions();
    let before_position_index = client.get_position_index();
    let before_user_token_balance = token_client.balance(&user);
    let before_vault_token_balance = token_client.balance(&contract_id);
    let before_oracle = env.as_contract(&contract_id, || storage::get_oracle(&env));
    let before_is_supported = client.is_supported_asset(&token_id);

    assert_eq!(before_contract_version, 3);
    assert_eq!(before_schema_version, 1);
    assert_eq!(before_admin, Some(admin.clone()));
    assert_eq!(before_oracle, Some(oracle.clone()));
    assert_eq!(before_position_balance, 500);
    assert_eq!(before_user_token_balance, 500);
    assert_eq!(before_vault_token_balance, 500);

    // Call migrate with valid target schema version (2) as the authorized admin
    let migrate_res = client.try_migrate(&2);
    assert!(migrate_res.is_ok());

    // Assert AFTER: schema version equals target
    assert_eq!(client.get_storage_schema_version(), 2);

    // Assert AFTER: admin, position balance, aggregates, and configuration are unchanged
    assert_eq!(client.get_admin(), before_admin);
    assert_eq!(
        client.get_position_balance(&user, &token_id),
        before_position_balance
    );
    assert_eq!(client.get_position(&user), before_position);
    assert_eq!(client.get_all_positions(), before_all_positions);
    assert_eq!(client.get_position_index(), before_position_index);
    assert_eq!(token_client.balance(&user), before_user_token_balance);
    assert_eq!(
        token_client.balance(&contract_id),
        before_vault_token_balance
    );
    assert_eq!(client.is_supported_asset(&token_id), before_is_supported);
    env.as_contract(&contract_id, || {
        assert_eq!(storage::get_oracle(&env), before_oracle);
    });
}

#[test]
fn test_migrate_rejects_unauthorized_address() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(VaultContract, (&admin,));
    let client = VaultContractClient::new(&env, &contract_id);

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

    env.as_contract(&contract_id, || {
        storage::set_storage_schema_version(&env, 1);
    });

    env.mock_auths(&[MockAuth {
        address: &attacker,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "migrate",
            args: (2u32,).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let result = client.try_migrate(&2);
    assert!(result.is_err());

    // Schema version and admin remain unchanged afterwards
    assert_eq!(client.get_storage_schema_version(), 1);
    assert_eq!(client.get_admin(), Some(admin));
}

#[test]
fn finalization_checks_the_version_of_the_installed_code() {
    let (env, _, vault, _, _, _, _, _, _) = setup_env();
    let hash = env
        .deployer()
        .upload_contract_wasm(Bytes::from_slice(&env, TEST_WASM));
    vault.upgrade(&hash, &4);
    assert_eq!(vault.get_contract_version(), 3);
    assert_eq!(
        vault.try_finalize_upgrade(),
        Err(Ok(VaultError::InvalidInputs))
    );
    vault.upgrade(&hash, &3);
    vault.finalize_upgrade();
}
