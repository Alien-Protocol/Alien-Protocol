use crate::errors::EscrowError;
use crate::types::{AutoPay, DataKey, LegacyVault, ScheduledPayment, VaultConfig, VaultState};
use crate::EscrowContract;
use crate::EscrowContractClient;
use soroban_sdk::testutils::{Address as _, Events as _, Ledger, MockAuth, MockAuthInvoke};
use soroban_sdk::token::{Client as TokenClient, StellarAssetClient};

use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, Error, IntoVal};

#[contract]
pub struct MockRegistrationContract;

#[contractimpl]
impl MockRegistrationContract {
    pub fn set_owner(env: Env, commitment: BytesN<32>, owner: Address) {
        env.storage().persistent().set(&commitment, &owner);
    }

    pub fn get_owner(env: Env, commitment: BytesN<32>) -> Option<Address> {
        env.storage().persistent().get(&commitment)
    }
}

fn setup_test(
    env: &Env,
) -> (
    Address,
    EscrowContractClient<'_>,
    Address,
    Address,
    BytesN<32>,
    BytesN<32>,
) {
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(env, &contract_id);

    let token_admin = Address::generate(env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address()
        .clone();

    let from = BytesN::from_array(env, &[0u8; 32]);
    let to = BytesN::from_array(env, &[1u8; 32]);

    (contract_id, client, token, token_admin, from, to)
}

fn create_vault(
    env: &Env,
    contract_id: &Address,
    id: &BytesN<32>,
    owner: &Address,
    token: &Address,
    balance: i128,
) {
    let config = VaultConfig {
        owner: owner.clone(),
        token: token.clone(),
        created_at: 0,
    };
    let state = VaultState {
        balance,
        is_active: true,
    };
    env.as_contract(contract_id, || {
        env.storage()
            .persistent()
            .set(&DataKey::VaultConfig(id.clone()), &config);
        env.storage()
            .persistent()
            .set(&DataKey::VaultState(id.clone()), &state);
    });
}

fn create_legacy_vault(
    env: &Env,
    contract_id: &Address,
    id: &BytesN<32>,
    owner: &Address,
    token: &Address,
    balance: i128,
) {
    let legacy = LegacyVault {
        owner: owner.clone(),
        token: token.clone(),
        created_at: 0,
        balance,
        is_active: true,
    };
    env.as_contract(contract_id, || {
        env.storage()
            .persistent()
            .set(&DataKey::Vault(id.clone()), &legacy);
    });
}

fn mint_token(env: &Env, token: &Address, token_admin: &Address, to: &Address, amount: i128) {
    let admin_client = StellarAssetClient::new(env, token);
    admin_client.mock_all_auths().mint(to, &amount);
    assert_eq!(admin_client.admin(), *token_admin);
}

#[test]
fn test_legacy_vault_key_fallback_and_migration() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, token_admin, from, _to) = setup_test(&env);

    let owner = Address::generate(&env);
    create_legacy_vault(&env, &contract_id, &from, &owner, &token, 1000);

    mint_token(&env, &token, &token_admin, &owner, 500);

    assert_eq!(client.get_balance(&from), Some(1000));

    client.deposit(&from, &200);
    assert_eq!(client.get_balance(&from), Some(1200));

    env.as_contract(&contract_id, || {
        let config: Option<VaultConfig> = env
            .storage()
            .persistent()
            .get(&DataKey::VaultConfig(from.clone()));
        assert_eq!(config, None);

        let state: VaultState = env
            .storage()
            .persistent()
            .get(&DataKey::VaultState(from.clone()))
            .expect("vault state should exist");
        assert_eq!(state.balance, 1200);
        assert!(state.is_active);

        let legacy: LegacyVault = env
            .storage()
            .persistent()
            .get(&DataKey::Vault(from.clone()))
            .expect("legacy vault should exist");
        assert_eq!(legacy.owner, owner);
        assert_eq!(legacy.token, token);
    });
}

#[test]
fn test_get_scheduled_payment_returns_all_fields_after_schedule() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _, from, to) = setup_test(&env);

    let initial_balance = 1_000i128;
    let amount = 400i128;
    let release_at = 2_000u64;

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &token,
        initial_balance,
    );
    env.ledger().set_timestamp(1_000);

    let payment_id = client.schedule_payment(&from, &to, &amount, &release_at);

    let result = client.get_scheduled_payment(&payment_id);
    assert!(
        result.is_some(),
        "expected Some(ScheduledPayment) for a known payment_id"
    );

    let payment = result.expect("scheduled payment should exist");
    assert_eq!(payment.from, from);
    assert_eq!(payment.to, to);
    assert_eq!(payment.amount, amount);
    assert_eq!(payment.release_at, release_at);
    assert!(!payment.executed, "payment should not be executed yet");
}

#[test]
fn test_get_scheduled_payment_returns_none_for_unknown_id() {
    let env = Env::default();
    let (_, client, _, _, _, _) = setup_test(&env);

    let result = client.get_scheduled_payment(&99_999u32);
    assert!(result.is_none(), "expected None for an unknown payment_id");
}

#[test]
fn test_schedule_payment_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _, from, to) = setup_test(&env);

    let initial_balance = 1000i128;
    let amount = 400i128;
    let release_at = 2000u64;

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &token,
        initial_balance,
    );
    env.ledger().set_timestamp(1000);

    let payment_id = client.schedule_payment(&from, &to, &amount, &release_at);
    assert_eq!(payment_id, 0);

    env.as_contract(&contract_id, || {
        let state: VaultState = env
            .storage()
            .persistent()
            .get(&DataKey::VaultState(from.clone()))
            .expect("vault state should exist");
        assert_eq!(state.balance, initial_balance - amount);

        let config: VaultConfig = env
            .storage()
            .persistent()
            .get(&DataKey::VaultConfig(from.clone()))
            .expect("vault config should exist");
        assert_eq!(config.token, token);

        let payment: ScheduledPayment = env
            .storage()
            .persistent()
            .get(&DataKey::ScheduledPayment(payment_id))
            .expect("scheduled payment should exist");
        assert_eq!(payment.from, from);
        assert_eq!(payment.to, to);
        assert_eq!(payment.amount, amount);
        assert_eq!(payment.release_at, release_at);
        assert!(!payment.executed);
    });
}

#[test]
fn test_schedule_payment_inactive_vault() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _, from, to) = setup_test(&env);

    let config = VaultConfig {
        owner: Address::generate(&env),
        token: token.clone(),
        created_at: 0,
    };
    let state = VaultState {
        balance: 1000,
        is_active: false,
    };
    env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .set(&DataKey::VaultConfig(from.clone()), &config);
        env.storage()
            .persistent()
            .set(&DataKey::VaultState(from.clone()), &state);
    });

    env.ledger().set_timestamp(1000);

    let result = client.try_schedule_payment(&from, &to, &100, &2000);
    assert_eq!(result, Err(Ok(EscrowError::VaultInactive)));
}

#[test]
fn test_schedule_payment_past_release_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, _, _, from, to) = setup_test(&env);

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &Address::generate(&env),
        1000,
    );
    env.ledger().set_timestamp(2000);

    let result = client.try_schedule_payment(&from, &to, &100, &1000);
    assert_eq!(result, Err(Ok(EscrowError::PastReleaseTime)));
}

#[test]
fn test_schedule_payment_insufficient_balance_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, _, _, from, to) = setup_test(&env);

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &Address::generate(&env),
        100,
    );
    env.ledger().set_timestamp(1000);

    let result = client.try_schedule_payment(&from, &to, &200, &2000);
    assert_eq!(result, Err(Ok(EscrowError::InsufficientBalance)));
}

#[test]
fn test_schedule_payment_returns_incrementing_ids() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _, from, to) = setup_test(&env);

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &token,
        10000,
    );
    env.ledger().set_timestamp(1000);

    let id0 = client.schedule_payment(&from, &to, &100, &2000);
    let id1 = client.schedule_payment(&from, &to, &200, &3000);

    assert_eq!(id0, 0);
    assert_eq!(id1, 1);
}

#[test]
fn test_payment_counter_overflow_returns_error() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _, from, to) = setup_test(&env);

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &token,
        1000,
    );
    env.ledger().set_timestamp(1000);

    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&DataKey::PaymentCounter, &u32::MAX);
    });

    let result = client.try_schedule_payment(&from, &to, &100, &2000);
    assert_eq!(result, Err(Ok(EscrowError::PaymentCounterOverflow)));
}

#[test]
fn test_execute_scheduled_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _token_admin, from, to) = setup_test(&env);

    let from_owner = Address::generate(&env);
    let to_owner = Address::generate(&env);
    let amount = 400i128;
    let release_at = 2000u64;

    create_vault(&env, &contract_id, &from, &from_owner, &token, 1000);
    create_vault(&env, &contract_id, &to, &to_owner, &token, 0);

    env.ledger().set_timestamp(1000);
    let payment_id = client.schedule_payment(&from, &to, &amount, &release_at);

    let token_admin_client = StellarAssetClient::new(&env, &token);
    token_admin_client.mint(&contract_id, &amount);

    env.ledger().set_timestamp(2500);
    client.execute_scheduled(&payment_id);

    let events = env.events().all();
    let escrow_events = events
        .iter()
        .filter(|(event_contract, _, _)| event_contract == &contract_id)
        .count();
    assert!(escrow_events > 0); // schedule + execute events

    env.as_contract(&contract_id, || {
        let payment: ScheduledPayment = env
            .storage()
            .persistent()
            .get(&DataKey::ScheduledPayment(payment_id))
            .expect("scheduled payment should exist");
        assert!(payment.executed);
    });

    let token_client = TokenClient::new(&env, &token);
    assert_eq!(token_client.balance(&to_owner), amount);
    assert_eq!(token_client.balance(&contract_id), 0);
}

#[test]
fn test_execute_scheduled_early_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _, from, to) = setup_test(&env);

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &token,
        1000,
    );
    create_vault(&env, &contract_id, &to, &Address::generate(&env), &token, 0);

    env.ledger().set_timestamp(1000);
    let payment_id = client.schedule_payment(&from, &to, &100, &2000);

    let result = client.try_execute_scheduled(&payment_id);
    assert_eq!(result, Err(Ok(EscrowError::PaymentNotYetDue)));
}

fn setup_with_registration<'a>(
    env: &'a Env,
    commitment_seed: u8,
) -> (
    EscrowContractClient<'a>,
    Address,
    Address,
    Address,
    BytesN<32>,
) {
    let reg_id = env.register(MockRegistrationContract, ());
    let reg_client = MockRegistrationContractClient::new(env, &reg_id);

    let commitment = BytesN::from_array(env, &[commitment_seed; 32]);
    let owner = Address::generate(env);
    let token_admin = Address::generate(env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address()
        .clone();

    reg_client.set_owner(&commitment, &owner);

    let escrow_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(env, &escrow_id);
    let admin = Address::generate(env);
    client.initialize(&admin, &reg_id);

    (client, escrow_id, owner, token, commitment)
}

#[test]
fn test_create_vault_success() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, escrow_id, owner, token, commitment) = setup_with_registration(&env, 0xAA);

    client.create_vault(&commitment, &token);

    env.as_contract(&escrow_id, || {
        let config: VaultConfig = env
            .storage()
            .persistent()
            .get(&DataKey::VaultConfig(commitment.clone()))
            .expect("vault config should exist");
        assert_eq!(config.owner, owner);
        assert_eq!(config.token, token);

        let state: VaultState = env
            .storage()
            .persistent()
            .get(&DataKey::VaultState(commitment.clone()))
            .expect("vault state should exist");
        assert_eq!(state.balance, 0);
        assert!(state.is_active);
    });
}

#[test]
fn test_create_vault_already_exists() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, _, token, commitment) = setup_with_registration(&env, 0xBB);

    client.create_vault(&commitment, &token);

    let result = client.try_create_vault(&commitment, &token);
    assert!(matches!(
        result,
        Err(Ok(err)) if err == Error::from_contract_error(EscrowError::VaultAlreadyExists as u32)
    ));
}

#[test]
#[should_panic]
fn test_create_vault_not_owner() {
    let env = Env::default();

    let reg_id = env.register(MockRegistrationContract, ());
    let reg_client = MockRegistrationContractClient::new(&env, &reg_id);

    let commitment = BytesN::from_array(&env, &[0xCCu8; 32]);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address()
        .clone();

    reg_client.set_owner(&commitment, &owner);

    let escrow_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &escrow_id);

    env.as_contract(&escrow_id, || {
        env.storage()
            .instance()
            .set(&DataKey::RegistrationContract, &reg_id);
    });

    client.create_vault(&commitment, &token);
}

#[test]
fn test_execute_scheduled_double_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _token_admin, from, to) = setup_test(&env);

    let to_owner = Address::generate(&env);
    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &token,
        1000,
    );
    create_vault(&env, &contract_id, &to, &to_owner, &token, 0);

    env.ledger().set_timestamp(1000);
    let payment_id = client.schedule_payment(&from, &to, &100, &1500);

    let token_admin_client = StellarAssetClient::new(&env, &token);
    token_admin_client.mint(&contract_id, &100);

    env.ledger().set_timestamp(1600);
    client.execute_scheduled(&payment_id);

    let result = client.try_execute_scheduled(&payment_id);
    assert_eq!(result, Err(Ok(EscrowError::PaymentAlreadyExecuted)));
}

#[test]
fn test_execute_scheduled_not_found_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, _, _, _, _) = setup_test(&env);

    let invalid_id = 999;
    let result = client.try_execute_scheduled(&invalid_id);
    assert_eq!(result, Err(Ok(EscrowError::PaymentNotFound)));
}

#[test]
fn test_deposit_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _token_admin, from, _to) = setup_test(&env);

    let owner = Address::generate(&env);
    let initial_balance = 100i128;
    let deposit_amount = 50i128;

    create_vault(&env, &contract_id, &from, &owner, &token, initial_balance);

    let token_admin_client = StellarAssetClient::new(&env, &token);
    token_admin_client.mint(&owner, &deposit_amount);

    client.deposit(&from, &deposit_amount);

    env.as_contract(&contract_id, || {
        let state: VaultState = env
            .storage()
            .persistent()
            .get(&DataKey::VaultState(from.clone()))
            .expect("vault state should exist");
        assert_eq!(state.balance, initial_balance + deposit_amount);
    });

    let token_client = TokenClient::new(&env, &token);
    assert_eq!(token_client.balance(&contract_id), deposit_amount);
    assert_eq!(token_client.balance(&owner), 0);
}

#[test]
fn test_deposit_non_existent_vault() {
    let env = Env::default();
    env.mock_all_auths();
    let (_contract_id, client, _token, _token_admin, from, _to) = setup_test(&env);

    let result = client.try_deposit(&from, &100);
    assert_eq!(result, Err(Ok(EscrowError::VaultNotFound)));
}

#[test]
fn test_deposit_inactive_vault() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _token_admin, from, _to) = setup_test(&env);

    let owner = Address::generate(&env);
    let config = VaultConfig {
        owner: owner.clone(),
        token: token.clone(),
        created_at: 0,
    };
    let state = VaultState {
        balance: 1000,
        is_active: false,
    };
    env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .set(&DataKey::VaultConfig(from.clone()), &config);
        env.storage()
            .persistent()
            .set(&DataKey::VaultState(from.clone()), &state);
    });

    let result = client.try_deposit(&from, &100);
    assert_eq!(result, Err(Ok(EscrowError::VaultInactive)));
}

#[test]
fn test_deposit_invalid_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _token_admin, from, _to) = setup_test(&env);

    let owner = Address::generate(&env);
    create_vault(&env, &contract_id, &from, &owner, &token, 100);

    let result0 = client.try_deposit(&from, &0);
    assert_eq!(result0, Err(Ok(EscrowError::InvalidAmount)));

    let result_neg = client.try_deposit(&from, &-50);
    assert_eq!(result_neg, Err(Ok(EscrowError::InvalidAmount)));
}

#[test]
fn test_deposit_zero_amount_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _token_admin, from, _to) = setup_test(&env);

    let owner = Address::generate(&env);
    create_vault(&env, &contract_id, &from, &owner, &token, 100);

    let result = client.try_deposit(&from, &0);
    assert_eq!(result, Err(Ok(EscrowError::InvalidAmount)));
}

#[test]
fn test_withdraw_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _token_admin, from, _) = setup_test(&env);

    let owner = Address::generate(&env);
    let initial_balance = 1_000i128;
    let withdraw_amount = 400i128;

    create_vault(&env, &contract_id, &from, &owner, &token, initial_balance);

    let token_admin_client = StellarAssetClient::new(&env, &token);
    token_admin_client.mint(&contract_id, &withdraw_amount);

    client.withdraw(&from, &withdraw_amount);

    env.as_contract(&contract_id, || {
        let state: VaultState = env
            .storage()
            .persistent()
            .get(&DataKey::VaultState(from.clone()))
            .expect("vault state should exist");
        assert_eq!(state.balance, initial_balance - withdraw_amount);
        assert!(state.is_active);
    });

    let token_client = TokenClient::new(&env, &token);
    assert_eq!(token_client.balance(&owner), withdraw_amount);
    assert_eq!(token_client.balance(&contract_id), 0);
}

#[test]
fn test_withdraw_insufficient_balance() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _token_admin, from, _) = setup_test(&env);

    let owner = Address::generate(&env);
    create_vault(&env, &contract_id, &from, &owner, &token, 100);

    let result = client.try_withdraw(&from, &200);
    assert!(matches!(
        result,
        Err(Ok(err)) if err == Error::from_contract_error(EscrowError::InsufficientBalance as u32)
    ));
}

#[test]
#[should_panic]
fn test_deposit_not_owner() {
    let env = Env::default();
    let (contract_id, client, token, _token_admin, from, _to) = setup_test(&env);

    let owner = Address::generate(&env);
    create_vault(&env, &contract_id, &from, &owner, &token, 100);

    client.deposit(&from, &100);
}

#[test]
fn test_get_balance_vault_not_found() {
    let env = Env::default();
    let (_, client, _, _, _, _) = setup_test(&env);

    let unknown = BytesN::from_array(&env, &[99u8; 32]);
    assert_eq!(client.get_balance(&unknown), None);
}

#[test]
fn test_get_balance_after_deposit() {
    let env = Env::default();
    let (contract_id, client, token, _, from, _) = setup_test(&env);

    let balance = 5_000i128;
    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &token,
        balance,
    );

    assert_eq!(client.get_balance(&from), Some(balance));
}

#[test]
fn test_get_balance_after_payment() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _, from, to) = setup_test(&env);

    let initial = 1_000i128;
    let amount = 300i128;

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &token,
        initial,
    );
    create_vault(&env, &contract_id, &to, &Address::generate(&env), &token, 0);

    env.ledger().set_timestamp(1000);
    client.schedule_payment(&from, &to, &amount, &2000);

    assert_eq!(client.get_balance(&from), Some(initial - amount));
}

#[test]
fn test_deposit_increases_balance() {
    let env = Env::default();
    let (contract_id, client, token, token_admin, from, _) = setup_test(&env);
    let owner = Address::generate(&env);
    let amount = 100_i128;

    create_vault(&env, &contract_id, &from, &owner, &token, 0);
    mint_token(&env, &token, &token_admin, &owner, amount);

    client.mock_all_auths().deposit(&from, &amount);

    assert_eq!(client.get_balance(&from), Some(amount));
    let token_client = TokenClient::new(&env, &token);
    assert_eq!(token_client.balance(&owner), 0);
    assert_eq!(token_client.balance(&contract_id), amount);
}

#[test]
fn test_deposit_zero_panics() {
    let env = Env::default();
    let (contract_id, client, token, _, from, _) = setup_test(&env);
    let owner = Address::generate(&env);

    create_vault(&env, &contract_id, &from, &owner, &token, 0);
    let result = client.mock_all_auths().try_deposit(&from, &0);
    assert_eq!(result, Err(Ok(EscrowError::InvalidAmount)));
}

#[test]
#[should_panic]
fn test_deposit_non_owner_panics() {
    let env = Env::default();
    let (contract_id, client, token, token_admin, from, _) = setup_test(&env);
    let owner = Address::generate(&env);
    let non_owner = Address::generate(&env);
    let amount = 100_i128;

    create_vault(&env, &contract_id, &from, &owner, &token, 0);
    mint_token(&env, &token, &token_admin, &owner, amount);

    client
        .mock_auths(&[MockAuth {
            address: &non_owner,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "deposit",
                args: (from.clone(), amount).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .deposit(&from, &amount);
}

#[test]
fn test_deposit_vault_not_found_panics() {
    let env = Env::default();
    let (_, client, _, _, _, _) = setup_test(&env);
    let commitment = BytesN::from_array(&env, &[9u8; 32]);

    let result = client.mock_all_auths().try_deposit(&commitment, &100);
    assert_eq!(result, Err(Ok(EscrowError::VaultNotFound)));
}

#[test]
fn test_withdraw_success_with_token_transfer() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _token_admin, from, _to) = setup_test(&env);

    let owner = Address::generate(&env);
    let initial_balance = 100i128;
    let withdraw_amount = 40i128;

    create_vault(&env, &contract_id, &from, &owner, &token, initial_balance);

    let token_admin_client = StellarAssetClient::new(&env, &token);
    token_admin_client.mint(&contract_id, &initial_balance);

    env.as_contract(&contract_id, || {
        let state: VaultState = env
            .storage()
            .persistent()
            .get(&DataKey::VaultState(from.clone()))
            .expect("vault state should exist");
        assert_eq!(state.balance, initial_balance);
    });

    client.withdraw(&from, &withdraw_amount);

    env.as_contract(&contract_id, || {
        let state: VaultState = env
            .storage()
            .persistent()
            .get(&DataKey::VaultState(from.clone()))
            .expect("vault state should exist");
        assert_eq!(state.balance, initial_balance - withdraw_amount);
    });

    let token_client = TokenClient::new(&env, &token);
    assert_eq!(token_client.balance(&owner), withdraw_amount);
    assert_eq!(
        token_client.balance(&contract_id),
        initial_balance - withdraw_amount
    );
}

#[test]
fn test_withdraw_non_existent_vault() {
    let env = Env::default();
    env.mock_all_auths();
    let (_contract_id, client, _token, _token_admin, from, _to) = setup_test(&env);

    let result = client.try_withdraw(&from, &100);
    assert!(matches!(
        result,
        Err(Ok(err)) if err == Error::from_contract_error(EscrowError::VaultNotFound as u32)
    ));
}

#[test]
fn test_withdraw_inactive_vault() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _token_admin, from, _to) = setup_test(&env);

    let owner = Address::generate(&env);
    let config = VaultConfig {
        owner: owner.clone(),
        token: token.clone(),
        created_at: 0,
    };
    let state = VaultState {
        balance: 1000,
        is_active: false,
    };
    env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .set(&DataKey::VaultConfig(from.clone()), &config);
        env.storage()
            .persistent()
            .set(&DataKey::VaultState(from.clone()), &state);
    });

    let result = client.try_withdraw(&from, &100);
    assert!(matches!(
        result,
        Err(Ok(err)) if err == Error::from_contract_error(EscrowError::VaultInactive as u32)
    ));
}

#[test]
fn test_withdraw_invalid_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _token_admin, from, _to) = setup_test(&env);

    let owner = Address::generate(&env);
    create_vault(&env, &contract_id, &from, &owner, &token, 100);

    let result0 = client.try_withdraw(&from, &0);
    assert!(matches!(
        result0,
        Err(Ok(err)) if err == Error::from_contract_error(EscrowError::InvalidAmount as u32)
    ));

    let result_neg = client.try_withdraw(&from, &-50);
    assert!(matches!(
        result_neg,
        Err(Ok(err)) if err == Error::from_contract_error(EscrowError::InvalidAmount as u32)
    ));
}

#[test]
fn test_withdraw_overdraft() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _token_admin, from, _to) = setup_test(&env);

    let owner = Address::generate(&env);
    let balance = 50i128;
    create_vault(&env, &contract_id, &from, &owner, &token, balance);

    let result = client.try_withdraw(&from, &100);
    assert!(matches!(
        result,
        Err(Ok(err)) if err == Error::from_contract_error(EscrowError::InsufficientBalance as u32)
    ));

    env.as_contract(&contract_id, || {
        let state: VaultState = env
            .storage()
            .persistent()
            .get(&DataKey::VaultState(from.clone()))
            .expect("vault state should exist");
        assert_eq!(state.balance, balance);
    });
}

#[test]
#[should_panic]
fn test_withdraw_not_owner() {
    let env = Env::default();
    let (contract_id, client, token, _token_admin, from, _to) = setup_test(&env);

    let owner = Address::generate(&env);
    create_vault(&env, &contract_id, &from, &owner, &token, 100);

    client.withdraw(&from, &50);
}

#[test]
fn test_auto_pay_multiple_vaults_no_interference() {
    use crate::storage::{read_auto_pay, write_auto_pay};
    use crate::types::AutoPay;

    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, _client, token, _token_admin, _from, _to) = setup_test(&env);

    let vault_a = BytesN::from_array(&env, &[0xAAu8; 32]);
    let vault_b = BytesN::from_array(&env, &[0xBBu8; 32]);

    let rule_a = AutoPay {
        from: vault_a.clone(),
        to: vault_b.clone(),
        token: token.clone(),
        amount: 100,
        interval: 86_400,
        last_paid: 0,
    };
    let rule_b = AutoPay {
        from: vault_b.clone(),
        to: vault_a.clone(),
        token: token.clone(),
        amount: 200,
        interval: 43_200,
        last_paid: 0,
    };

    env.as_contract(&contract_id, || {
        write_auto_pay(&env, &vault_a, 0, &rule_a);
        write_auto_pay(&env, &vault_b, 0, &rule_b);
    });

    env.as_contract(&contract_id, || {
        let stored_a = read_auto_pay(&env, &vault_a, 0).expect("rule for vault_a not found");
        assert_eq!(stored_a.amount, 100);
        assert_eq!(stored_a.interval, 86_400);

        let stored_b = read_auto_pay(&env, &vault_b, 0).expect("rule for vault_b not found");
        assert_eq!(stored_b.amount, 200);
        assert_eq!(stored_b.interval, 43_200);

        assert_ne!(stored_a.amount, stored_b.amount);
        assert_ne!(stored_a.from, stored_b.from);
    });
}

#[test]
fn test_trigger_auto_pay_inactive_vault_returns_vault_inactive() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _token_admin, from, to) = setup_test(&env);

    let owner = Address::generate(&env);
    let config = VaultConfig {
        owner: owner.clone(),
        token: token.clone(),
        created_at: 0,
    };
    let state = VaultState {
        balance: 1000,
        is_active: false,
    };
    let auto_pay = AutoPay {
        from: from.clone(),
        to: to.clone(),
        token: token.clone(),
        amount: 100,
        interval: 1,
        last_paid: 0,
    };

    env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .set(&DataKey::VaultConfig(from.clone()), &config);
        env.storage()
            .persistent()
            .set(&DataKey::VaultState(from.clone()), &state);
        env.storage()
            .persistent()
            .set(&DataKey::AutoPay(from.clone(), 0u64), &auto_pay);
    });

    env.ledger().set_timestamp(1000);

    let result = client.try_trigger_auto_pay(&from, &0);
    assert!(matches!(
        result,
        Err(Ok(err)) if err == Error::from_contract_error(EscrowError::VaultInactive as u32)
    ));
}

#[test]
fn test_cancel_vault_refunds_balance() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _token_admin, from, _) = setup_test(&env);
    let owner = Address::generate(&env);

    let initial_balance = 100i128;
    create_vault(&env, &contract_id, &from, &owner, &token, initial_balance);

    let token_admin_client = StellarAssetClient::new(&env, &token);
    token_admin_client
        .mock_all_auths()
        .mint(&contract_id, &initial_balance);

    assert_eq!(client.get_balance(&from), Some(initial_balance));

    client.cancel_vault(&from);

    assert_eq!(client.get_balance(&from), Some(0));

    env.as_contract(&contract_id, || {
        let state: VaultState = env
            .storage()
            .persistent()
            .get(&DataKey::VaultState(from.clone()))
            .expect("vault state should exist");
        assert!(!state.is_active);
        assert_eq!(state.balance, 0);
    });
}

#[test]
fn test_cancel_vault_empty_balance() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _, from, _) = setup_test(&env);
    let owner = Address::generate(&env);

    create_vault(&env, &contract_id, &from, &owner, &token, 0);

    client.cancel_vault(&from);

    env.as_contract(&contract_id, || {
        let state: VaultState = env
            .storage()
            .persistent()
            .get(&DataKey::VaultState(from.clone()))
            .expect("vault state should exist");
        assert!(!state.is_active);
        assert_eq!(state.balance, 0);
    });
}

#[test]
fn test_cancel_vault_blocks_deposit() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _, from, _) = setup_test(&env);
    let owner = Address::generate(&env);

    create_vault(&env, &contract_id, &from, &owner, &token, 0);

    client.cancel_vault(&from);

    let amount = 50i128;
    let result = client.try_deposit(&from, &amount);
    assert_eq!(result, Err(Ok(EscrowError::VaultInactive)));
}

#[test]
fn test_cancel_vault_blocks_schedule() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _, from, to) = setup_test(&env);

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &token,
        0,
    );
    create_vault(&env, &contract_id, &to, &Address::generate(&env), &token, 0);

    client.cancel_vault(&from);

    env.ledger().set_timestamp(1000);

    let result = client.try_schedule_payment(&from, &to, &100, &2000);
    assert_eq!(result, Err(Ok(EscrowError::VaultInactive)));
}

#[test]
#[should_panic]
fn test_cancel_vault_non_owner_panics() {
    let env = Env::default();
    let (contract_id, client, token, _, from, _) = setup_test(&env);
    let owner = Address::generate(&env);
    let non_owner = Address::generate(&env);

    create_vault(&env, &contract_id, &from, &owner, &token, 100);

    client
        .mock_auths(&[MockAuth {
            address: &non_owner,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "cancel_vault",
                args: (from.clone(),).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .cancel_vault(&from);
}

#[test]
fn test_get_auto_pay_count_returns_zero_before_any_rules() {
    let env = Env::default();
    let (_, client, _, _, _, _) = setup_test(&env);

    assert_eq!(client.get_auto_pay_count(), 0);
}

#[test]
fn test_get_auto_pay_count_increments_after_setup() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _, from, to) = setup_test(&env);

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &token,
        1000,
    );

    assert_eq!(client.get_auto_pay_count(), 0);

    client.setup_auto_pay(&from, &to, &100, &86_400);

    assert_eq!(client.get_auto_pay_count(), 1);
}

#[test]
fn test_get_auto_pay_count_increments_with_multiple_rules() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _, from, to) = setup_test(&env);

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &token,
        1000,
    );

    client.setup_auto_pay(&from, &to, &100, &86_400);
    assert_eq!(client.get_auto_pay_count(), 1);

    client.setup_auto_pay(&from, &to, &200, &43_200);
    assert_eq!(client.get_auto_pay_count(), 2);

    client.setup_auto_pay(&from, &to, &50, &3_600);
    assert_eq!(client.get_auto_pay_count(), 3);
}

#[test]
fn test_initialize_twice_returns_already_initialized() {
    let env = Env::default();
    env.mock_all_auths();

    let reg_id = env.register(MockRegistrationContract, ());
    let escrow_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &escrow_id);
    let admin = Address::generate(&env);

    client.initialize(&admin, &reg_id);

    let result = client.try_initialize(&admin, &reg_id);
    assert!(matches!(
        result,
        Err(Ok(err)) if err == Error::from_contract_error(EscrowError::AlreadyInitialized as u32)
    ));
}

#[test]
fn test_get_auto_pay_returns_rule_after_setup() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _token_admin, from, to) = setup_test(&env);

    let amount = 250i128;
    let interval = 86_400u64;

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &token,
        1_000,
    );

    let rule_id = client.setup_auto_pay(&from, &to, &amount, &interval);

    let result = client.get_auto_pay(&from, &rule_id);
    assert!(
        result.is_some(),
        "expected Some(AutoPay) after setup_auto_pay"
    );

    let rule = result.expect("auto-pay rule should exist");
    assert_eq!(rule.from, from);
    assert_eq!(rule.to, to);
    assert_eq!(rule.amount, amount);
    assert_eq!(rule.interval, interval);
    assert_eq!(rule.last_paid, 0);
}

#[test]
fn test_get_auto_pay_returns_none_for_unknown_rule() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _token_admin, from, _to) = setup_test(&env);

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &token,
        1_000,
    );

    let result = client.get_auto_pay(&from, &999u32);
    assert!(
        result.is_none(),
        "expected None for an unregistered rule_id"
    );
}

#[test]
fn test_auto_pay_self_payment_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _token_admin, from, _to) = setup_test(&env);

    let owner = Address::generate(&env);
    create_vault(&env, &contract_id, &from, &owner, &token, 1000);

    let result = client.try_setup_auto_pay(&from, &from, &100, &86400);
    assert!(matches!(
        result,
Err(Ok(err)) if err == EscrowError::SelfPaymentNotAllowed    ));
}

#[test]
fn test_cancel_auto_pay_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _token_admin, from, to) = setup_test(&env);

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &token,
        1_000,
    );

    let rule_id = client.setup_auto_pay(&from, &to, &100, &86_400);

    assert!(
        client.get_auto_pay(&from, &rule_id).is_some(),
        "rule must exist before cancel_auto_pay"
    );

    client.cancel_auto_pay(&from, &rule_id);

    assert!(
        client.get_auto_pay(&from, &rule_id).is_none(),
        "get_auto_pay must return None after cancel_auto_pay"
    );
}

#[test]
fn test_cancel_auto_pay_then_trigger_panics_with_not_found() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _token_admin, from, to) = setup_test(&env);
    let owner = Address::generate(&env);
    create_vault(&env, &contract_id, &from, &owner, &token, 1_000);

    let token_admin_client = StellarAssetClient::new(&env, &token);
    token_admin_client.mint(&contract_id, &500);

    let rule_id = client.setup_auto_pay(&from, &to, &100, &1);
    client.cancel_auto_pay(&from, &rule_id);

    env.ledger().set_timestamp(10_000);

    let result = client.try_trigger_auto_pay(&from, &rule_id);
    assert!(
        matches!(
            result,
            Err(Ok(err)) if err == Error::from_contract_error(EscrowError::AutoPayNotFound as u32)
        ),
        "expected AutoPayNotFound after cancel, got: {:?}",
        result
    );
}

#[test]
#[should_panic]
fn test_cancel_auto_pay_non_owner_panics() {
    let env = Env::default();
    let (contract_id, client, token, _token_admin, from, to) = setup_test(&env);

    let owner = Address::generate(&env);
    let non_owner = Address::generate(&env);

    create_vault(&env, &contract_id, &from, &owner, &token, 1_000);

    let rule_id = client
        .mock_all_auths()
        .setup_auto_pay(&from, &to, &100, &86_400);

    client
        .mock_auths(&[MockAuth {
            address: &non_owner,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "cancel_auto_pay",
                args: (from.clone(), rule_id).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .cancel_auto_pay(&from, &rule_id);
}

#[test]
fn test_cancel_auto_pay_nonexistent_rule_returns_not_found() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _token_admin, from, _to) = setup_test(&env);

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &token,
        1_000,
    );

    let result = client.try_cancel_auto_pay(&from, &999u32);
    assert!(
        matches!(
            result,
            Err(Ok(err)) if err == Error::from_contract_error(EscrowError::AutoPayNotFound as u32)
        ),
        "expected AutoPayNotFound for a rule that was never registered, got: {:?}",
        result
    );
}

#[test]
fn test_cancel_auto_pay_double_cancel_returns_not_found() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _token_admin, from, to) = setup_test(&env);

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &token,
        1_000,
    );

    let rule_id = client.setup_auto_pay(&from, &to, &100, &86_400);

    client.cancel_auto_pay(&from, &rule_id);

    let result = client.try_cancel_auto_pay(&from, &rule_id);
    assert!(
        matches!(
            result,
            Err(Ok(err)) if err == Error::from_contract_error(EscrowError::AutoPayNotFound as u32)
        ),
        "expected AutoPayNotFound on double-cancel, got: {:?}",
        result
    );
}

#[test]
fn test_cancel_auto_pay_does_not_affect_sibling_rules() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _token_admin, from, to) = setup_test(&env);

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &token,
        2_000,
    );

    let rule_a = client.setup_auto_pay(&from, &to, &100, &86_400);
    let rule_b = client.setup_auto_pay(&from, &to, &200, &43_200);

    client.cancel_auto_pay(&from, &rule_a);

    assert!(
        client.get_auto_pay(&from, &rule_a).is_none(),
        "cancelled rule_a must return None"
    );

    let surviving = client.get_auto_pay(&from, &rule_b);
    assert!(
        surviving.is_some(),
        "rule_b must survive cancellation of rule_a"
    );
    assert_eq!(
        surviving.expect("surviving payment should exist").amount,
        200,
        "rule_b amount must be unchanged after rule_a cancel"
    );
}

#[test]
fn test_cancel_auto_pay_cross_vault_isolation() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _token_admin, from, to) = setup_test(&env);

    let vault_a = from.clone();
    let vault_b = to.clone();

    create_vault(
        &env,
        &contract_id,
        &vault_a,
        &Address::generate(&env),
        &token,
        1_000,
    );
    create_vault(
        &env,
        &contract_id,
        &vault_b,
        &Address::generate(&env),
        &token,
        1_000,
    );

    let rule_a = client.setup_auto_pay(&vault_a, &vault_b, &100, &86_400);
    let rule_b = client.setup_auto_pay(&vault_b, &vault_a, &200, &43_200);

    assert_eq!(rule_a, 0, "first rule on vault_a must have id 0");

    client.cancel_auto_pay(&vault_a, &rule_a);

    assert!(
        client.get_auto_pay(&vault_a, &rule_a).is_none(),
        "vault_a rule must be deleted"
    );

    assert!(
        client.get_auto_pay(&vault_b, &rule_b).is_some(),
        "vault_b rule must survive cancellation on vault_a"
    );
}

#[test]
fn test_cancel_auto_pay_vault_not_found() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, _, _, _, _) = setup_test(&env);

    let nonexistent_commitment = BytesN::from_array(&env, &[0xFFu8; 32]);

    let result = client.try_cancel_auto_pay(&nonexistent_commitment, &0u32);
    assert!(
        matches!(
            result,
            Err(Ok(err)) if err == Error::from_contract_error(EscrowError::VaultNotFound as u32)
        ),
        "expected VaultNotFound for nonexistent vault, got: {:?}",
        result
    );
}

#[test]
fn test_is_vault_active_returns_some_true_for_active_vault() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _, from, _) = setup_test(&env);

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &token,
        0,
    );

    assert_eq!(
        client.is_vault_active(&from),
        Some(true),
        "active vault must return Some(true)"
    );
}

#[test]
fn test_is_vault_active_returns_some_false_for_cancelled_vault() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, token, _, from, _) = setup_test(&env);

    create_vault(
        &env,
        &contract_id,
        &from,
        &Address::generate(&env),
        &token,
        0,
    );

    client.cancel_vault(&from);

    assert_eq!(
        client.is_vault_active(&from),
        Some(false),
        "cancelled vault must return Some(false)"
    );
}

#[test]
fn test_is_vault_active_returns_none_for_nonexistent_vault() {
    let env = Env::default();
    let (_, client, _, _, _, _) = setup_test(&env);

    let nonexistent = BytesN::from_array(&env, &[0xDEu8; 32]);

    assert_eq!(
        client.is_vault_active(&nonexistent),
        None,
        "nonexistent vault must return None"
    );
}

// ── Admin & Pause tests ──────────────────────────────────────────────────────

#[test]
fn test_initialize_stores_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let reg_id = env.register(MockRegistrationContract, ());
    let escrow_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &escrow_id);
    let admin = Address::generate(&env);

    client.initialize(&admin, &reg_id);

    assert_eq!(
        client.get_admin(),
        Some(admin),
        "admin must be stored after initialize"
    );
}

#[test]
fn test_is_paused_false_after_initialize() {
    let env = Env::default();
    env.mock_all_auths();

    let reg_id = env.register(MockRegistrationContract, ());
    let escrow_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &escrow_id);
    let admin = Address::generate(&env);

    client.initialize(&admin, &reg_id);

    assert!(
        !client.is_paused(),
        "contract must not be paused after initialize"
    );
}

#[test]
fn test_set_paused_by_admin_toggles_state() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, _, _, _) = setup_with_registration(&env, 0x01);

    assert!(!client.is_paused());

    client.set_paused(&true);
    assert!(
        client.is_paused(),
        "contract must be paused after set_paused(true)"
    );

    client.set_paused(&false);
    assert!(
        !client.is_paused(),
        "contract must be unpaused after set_paused(false)"
    );
}

#[test]
#[should_panic]
fn test_set_paused_by_non_admin_panics() {
    let env = Env::default();

    let reg_id = env.register(MockRegistrationContract, ());
    let escrow_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &escrow_id);
    let admin = Address::generate(&env);
    let non_admin = Address::generate(&env);

    client.mock_all_auths().initialize(&admin, &reg_id);

    client
        .mock_auths(&[MockAuth {
            address: &non_admin,
            invoke: &MockAuthInvoke {
                contract: &escrow_id,
                fn_name: "set_paused",
                args: (true,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .set_paused(&true);
}

#[test]
fn test_rotate_admin_by_admin_succeeds() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, _, _, _) = setup_with_registration(&env, 0x02);

    let new_admin = Address::generate(&env);
    client.rotate_admin(&new_admin);

    assert_eq!(
        client.get_admin(),
        Some(new_admin),
        "admin must be updated after rotate_admin"
    );
}

#[test]
#[should_panic]
fn test_rotate_admin_by_non_admin_panics() {
    let env = Env::default();

    let reg_id = env.register(MockRegistrationContract, ());
    let escrow_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &escrow_id);
    let admin = Address::generate(&env);
    let non_admin = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.mock_all_auths().initialize(&admin, &reg_id);

    client
        .mock_auths(&[MockAuth {
            address: &non_admin,
            invoke: &MockAuthInvoke {
                contract: &escrow_id,
                fn_name: "rotate_admin",
                args: (new_admin.clone(),).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .rotate_admin(&new_admin);
}

#[test]
fn test_rotate_admin_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let reg_id = env.register(MockRegistrationContract, ());
    let escrow_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &escrow_id);
    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.initialize(&admin, &reg_id);
    client.rotate_admin(&new_admin);

    let events = env.events().all();
    let has_escrow_event = events.iter().any(|(contract, _, _)| contract == escrow_id);
    assert!(
        has_escrow_event,
        "expected at least one escrow event after rotate_admin"
    );
}

#[test]
fn test_set_paused_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, escrow_id, _, _, _) = setup_with_registration(&env, 0x03);

    client.set_paused(&true);

    let events = env.events().all();
    let has_escrow_event = events.iter().any(|(contract, _, _)| contract == escrow_id);
    assert!(
        has_escrow_event,
        "expected at least one escrow event after set_paused"
    );
}

#[test]
fn test_create_vault_blocked_when_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, _, token, commitment) = setup_with_registration(&env, 0x04);

    client.set_paused(&true);

    let result = client.try_create_vault(&commitment, &token);
    assert!(
        matches!(result, Err(Ok(err)) if err == Error::from_contract_error(EscrowError::ContractPaused as u32)),
        "create_vault must be blocked while paused"
    );
}

#[test]
fn test_deposit_blocked_when_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, escrow_id, owner, token, commitment) = setup_with_registration(&env, 0x05);

    create_vault(&env, &escrow_id, &commitment, &owner, &token, 100);

    client.set_paused(&true);

    let result = client.try_deposit(&commitment, &50);
    assert!(
        matches!(result, Err(Ok(err)) if err == EscrowError::ContractPaused),
        "deposit must be blocked while paused"
    );
}

#[test]
fn test_withdraw_blocked_when_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, escrow_id, owner, token, commitment) = setup_with_registration(&env, 0x06);

    create_vault(&env, &escrow_id, &commitment, &owner, &token, 100);

    client.set_paused(&true);

    let result = client.try_withdraw(&commitment, &50);
    assert!(
        matches!(result, Err(Ok(err)) if err == Error::from_contract_error(EscrowError::ContractPaused as u32)),
        "withdraw must be blocked while paused"
    );
}

#[test]
fn test_schedule_payment_blocked_when_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, escrow_id, owner, token, from) = setup_with_registration(&env, 0x07);
    let to = BytesN::from_array(&env, &[0xEEu8; 32]);

    create_vault(&env, &escrow_id, &from, &owner, &token, 1000);

    client.set_paused(&true);

    env.ledger().set_timestamp(1000);
    let result = client.try_schedule_payment(&from, &to, &100, &2000);
    assert!(
        matches!(result, Err(Ok(err)) if err == EscrowError::ContractPaused),
        "schedule_payment must be blocked while paused"
    );
}

#[test]
fn test_execute_scheduled_blocked_when_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, escrow_id, owner, token, from) = setup_with_registration(&env, 0x08);
    let to = BytesN::from_array(&env, &[0xEFu8; 32]);
    let to_owner = Address::generate(&env);

    create_vault(&env, &escrow_id, &from, &owner, &token, 1000);
    create_vault(&env, &escrow_id, &to, &to_owner, &token, 0);

    env.ledger().set_timestamp(1000);
    let payment_id = client.schedule_payment(&from, &to, &100, &1500);

    client.set_paused(&true);

    env.ledger().set_timestamp(2000);
    let result = client.try_execute_scheduled(&payment_id);
    assert!(
        matches!(result, Err(Ok(err)) if err == EscrowError::ContractPaused),
        "execute_scheduled must be blocked while paused"
    );
}

#[test]
fn test_trigger_auto_pay_blocked_when_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, escrow_id, owner, token, from) = setup_with_registration(&env, 0x09);
    let to = BytesN::from_array(&env, &[0xF0u8; 32]);

    create_vault(&env, &escrow_id, &from, &owner, &token, 1000);

    let rule_id = client.setup_auto_pay(&from, &to, &100, &1);

    client.set_paused(&true);

    env.ledger().set_timestamp(10_000);
    let result = client.try_trigger_auto_pay(&from, &rule_id);
    assert!(
        matches!(result, Err(Ok(err)) if err == Error::from_contract_error(EscrowError::ContractPaused as u32)),
        "trigger_auto_pay must be blocked while paused"
    );
}

#[test]
fn test_read_only_queries_work_while_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, escrow_id, owner, token, commitment) = setup_with_registration(&env, 0x0A);

    create_vault(&env, &escrow_id, &commitment, &owner, &token, 500);

    client.set_paused(&true);

    assert!(client.is_paused(), "is_paused must return true");
    assert!(
        client.get_admin().is_some(),
        "get_admin must work while paused"
    );
    assert_eq!(
        client.get_balance(&commitment),
        Some(500),
        "get_balance must work while paused"
    );
    assert_eq!(
        client.is_vault_active(&commitment),
        Some(true),
        "is_vault_active must work while paused"
    );
    assert_eq!(
        client.get_auto_pay_count(),
        0,
        "get_auto_pay_count must work while paused"
    );
}

#[test]
fn test_recovery_after_unpause() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, escrow_id, owner, token, commitment) = setup_with_registration(&env, 0x0B);

    create_vault(&env, &escrow_id, &commitment, &owner, &token, 0);

    let token_admin_client = soroban_sdk::token::StellarAssetClient::new(&env, &token);
    token_admin_client.mint(&owner, &200);

    client.set_paused(&true);

    let result = client.try_deposit(&commitment, &100);
    assert!(
        matches!(result, Err(Ok(err)) if err == EscrowError::ContractPaused),
        "deposit must be blocked while paused"
    );

    client.set_paused(&false);

    client.deposit(&commitment, &100);
    assert_eq!(
        client.get_balance(&commitment),
        Some(100),
        "deposit must succeed after unpause"
    );
}
