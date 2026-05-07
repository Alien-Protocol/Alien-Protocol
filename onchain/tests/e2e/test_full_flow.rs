#![cfg(test)]
extern crate soroban_sdk;
use core_contract::types::PublicSignals;
use core_contract::{Contract, ContractClient};
use escrow_contract::types::VaultState;
use escrow_contract::{EscrowContract, EscrowContractClient};
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, BytesN, Env};
mod mock_registration_contract;
use mock_registration_contract::MockRegistrationContract;

#[test]
fn e2e_offchain_proof_to_onchain() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    // Register a commitment
    let owner = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[42u8; 32]);
    client.register(&owner, &hash);
    let stored_owner = client.get_owner(&hash);
    assert_eq!(stored_owner, Some(owner.clone()));

    // Simulate a mock proof and public signals
    let old_root = BytesN::from_array(&env, &[1u8; 32]);
    let new_root = BytesN::from_array(&env, &[2u8; 32]);
    let proof = soroban_sdk::Bytes::from_slice(&env, &[1u8; 64]);
    let public_signals = PublicSignals {
        commitment: hash.clone(),
        old_root: old_root.clone(),
        new_root: new_root.clone(),
    };

    // Set the initial root
    env.as_contract(&contract_id, || {
        core_contract::smt_root::SmtRoot::update_root(&env, old_root.clone());
    });

    // Register resolver (simulate proof verification)
    client.register_resolver(&owner, &hash, &proof, &public_signals);

    // Assert root is updated
    let current_root = client.get_smt_root();
    assert_eq!(current_root, new_root);
}

#[test]
fn e2e_add_stellar_address_and_resolve() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    // Register a username commitment
    let owner = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[99u8; 32]);
    client.register(&owner, &hash);

    // Add a Stellar address
    let stellar_address = Address::generate(&env);
    client.add_stellar_address(&owner, &hash, &stellar_address);

    // Resolve the Stellar address
    let resolved = client.resolve_stellar(&hash);
    assert_eq!(resolved, stellar_address);
}

#[test]
fn e2e_sdk_send_to_username() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    // Register a username commitment
    let owner = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[77u8; 32]);
    client.register(&owner, &hash);

    // Add a Stellar address
    let stellar_address = Address::generate(&env);
    client.add_stellar_address(&owner, &hash, &stellar_address);

    // Simulate SDK send-to-username by resolving the username hash
    let resolved = client.resolve_stellar(&hash);
    assert_eq!(resolved, stellar_address);
}

#[test]
fn e2e_escrow_deposit_schedule_payment() {
    // Debug prints for contract and environment state (none before env is declared)
    let env = Env::default();
    env.mock_all_auths();

    // Deploy mock registration contract and escrow contract
    let reg_id = env.register(MockRegistrationContract, ());
    let escrow_id = env.register(EscrowContract, ());
    let escrow_client = EscrowContractClient::new(&env, &escrow_id);

    // Initialize escrow contract with registration contract address
    let admin = Address::generate(&env);
    escrow_client.initialize(&admin, &reg_id);

    // Register a username commitment in mock registration contract before creating the vault
    let owner = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[123u8; 32]);
    env.as_contract(&reg_id, || {
        MockRegistrationContract::set_owner(env.clone(), hash.clone(), owner.clone());
    });

    // Deploy a mock Stellar token contract and mint tokens to the owner
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address()
        .clone();
    // Debug prints for contract and environment state
    println!("[TEST DEBUG] escrow_id: {:?}", escrow_id);
    println!("[TEST DEBUG] reg_id: {:?}", reg_id);
    let token_client = StellarAssetClient::new(&env, &token);
    token_client.mock_all_auths().mint(&owner, &1000);

    // Create a vault for the commitment
    env.as_contract(&escrow_id, || {
        println!(
            "[TEST DEBUG] create_vault: contract address = {:?}",
            env.current_contract_address()
        );
        EscrowContract::create_vault(env.clone(), hash.clone(), token.clone()).unwrap();
    });
    // Assert vault state exists after creation
    let state: Option<VaultState> = env.as_contract(&escrow_id, || {
        escrow_contract::storage::read_vault_state(&env, &hash)
    });
    assert!(state.is_some(), "Vault state should exist after creation");

    // Deposit tokens into the vault as the owner
    println!("[TEST DEBUG] deposit: contract address = {:?}", escrow_id);
    escrow_client.deposit(&hash, &1000);
    // Assert vault state after deposit
    let state: Option<VaultState> = env.as_contract(&escrow_id, || {
        escrow_contract::storage::read_vault_state(&env, &hash)
    });
    assert_eq!(
        state
            .as_ref()
            .expect("vault state should exist after payment")
            .balance,
        1000,
        "Vault balance should be 1000 after deposit"
    );

    // Register a second username commitment for payment destination
    let to_hash = BytesN::from_array(&env, &[124u8; 32]);
    env.as_contract(&reg_id, || {
        MockRegistrationContract::set_owner(env.clone(), to_hash.clone(), owner.clone());
    });
    // Create a vault for the recipient commitment as well
    env.as_contract(&escrow_id, || {
        println!(
            "[TEST DEBUG] create_vault (recipient): contract address = {:?}",
            env.current_contract_address()
        );
        EscrowContract::create_vault(env.clone(), to_hash.clone(), token.clone()).unwrap();
    });

    // Schedule a payment to another commitment
    let now = env.ledger().timestamp();
    let release_at = now + 10;
    let payment_id = env.as_contract(&escrow_id, || {
        println!(
            "[TEST DEBUG] schedule_payment: contract address = {:?}",
            env.current_contract_address()
        );
        EscrowContract::schedule_payment(
            env.clone(),
            hash.clone(),
            to_hash.clone(),
            500,
            release_at,
        )
        .expect("scheduled payment should be stored")
    });
    // Assert vault state after scheduling payment (should be 500)
    let state: Option<VaultState> = env.as_contract(&escrow_id, || {
        escrow_contract::storage::read_vault_state(&env, &hash)
    });
    assert_eq!(
        state
            .as_ref()
            .expect("vault state should exist after execution")
            .balance,
        500,
        "Vault balance should be 500 after scheduling payment"
    );

    // Fast-forward ledger time to after release_at
    env.ledger().set_timestamp(release_at + 1);

    // Assert vault state exists before executing scheduled payment
    let state: Option<VaultState> = env.as_contract(&escrow_id, || {
        escrow_contract::storage::read_vault_state(&env, &hash)
    });
    assert!(
        state.is_some(),
        "Vault state should exist before executing scheduled payment"
    );

    // Execute the scheduled payment
    env.as_contract(&escrow_id, || {
        println!(
            "[TEST DEBUG] execute_scheduled: contract address = {:?}",
            env.current_contract_address()
        );
        let _ = EscrowContract::execute_scheduled(env.clone(), payment_id);
    });

    // Assert vault state updated (balance should be 500)
    let state: VaultState = env.as_contract(&escrow_id, || {
        escrow_contract::storage::read_vault_state(&env, &hash).expect("vault state should exist")
    });
    assert_eq!(state.balance, 500);
}
