#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::{Address as _, Events, Ledger};
use soroban_sdk::{contract, contractimpl, token, Address, Env, Symbol};

const ORACLE_STALE_THRESHOLD: u64 = 300;

#[contract]
pub struct MockLendingPool;

#[contractimpl]
impl MockLendingPool {
    pub fn get_user_debt(env: Env, _user: Address) -> i128 {
        env.storage().persistent().get(&"debt").unwrap_or(0)
    }

    pub fn set_user_debt(env: Env, debt: i128) {
        env.storage().persistent().set(&"debt", &debt);
    }

    pub fn is_liquidatable(_env: Env, _user: Address) -> bool {
        false
    }
}

#[contract]
pub struct MockOracle;

#[contractimpl]
impl MockOracle {
    pub fn get_price(env: Env, asset: Address) -> Option<types::PriceData> {
        env.storage().persistent().get(&asset)
    }

    pub fn get_price_or_fail(env: Env, asset: Address) -> types::PriceData {
        let price_data: types::PriceData = match env.storage().persistent().get(&asset) {
            Some(pd) => pd,
            None => panic!("price not found"),
        };
        let current_time = env.ledger().timestamp();
        let age = match current_time.checked_sub(price_data.timestamp) {
            Some(delta) => delta,
            None => panic!("stale price"),
        };
        if age > ORACLE_STALE_THRESHOLD {
            panic!("stale price");
        }
        price_data
    }

    pub fn set_price(env: Env, asset: Address, price: i128, timestamp: u64) {
        let price_data = types::PriceData { price, timestamp };
        env.storage().persistent().set(&asset, &price_data);
    }
}

fn setup_env() -> (
    Env,
    VaultContractClient<'static>,
    Address,
    Address,
    token::Client<'static>,
    token::StellarAssetClient<'static>,
    Address,
) {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1000);

    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    let oracle_id = env.register(MockOracle, ());

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let lending_pool = env.register(MockLendingPool, ());
    let liquidation_engine = Address::generate(&env);

    client.initialize(&admin, &lending_pool, &oracle_id, &liquidation_engine);
    client.set_oracle(&oracle_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_id = token_contract.address();
    let token_client = token::Client::new(&env, &token_id);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);

    client.add_supported_asset(&token_id);

    (
        env,
        client,
        admin,
        user,
        token_client,
        token_admin_client,
        token_id,
    )
}

fn setup_env_with_engine() -> (
    Env,
    VaultContractClient<'static>,
    Address,
    Address,
    token::Client<'static>,
    token::StellarAssetClient<'static>,
    Address,
    Address,
) {
    let (env, client, admin, user, token_client, token_admin_client, token_id) = setup_env();

    let engine = Address::generate(&env);
    client.set_liquidation_engine(&engine);

    (
        env,
        client,
        admin,
        user,
        token_client,
        token_admin_client,
        token_id,
        engine,
    )
}

// ---------------------------------------------------------------------------
// Deposits
// ---------------------------------------------------------------------------

#[test]
fn test_deposit_allowed_when_not_paused() {
    let (_env, client, _admin, user, _token_client, token_admin, token_id) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);
    assert_eq!(client.get_position_balance(&user, &token_id), 500);
}

#[test]
fn test_deposit_paused() {
    let (env, client, _admin, user, _token_client, token_admin, token_id) = setup_env();

    token_admin.mint(&user, &1000);
    client.pause_operation(&PauseFlag::Deposit, &Symbol::new(&env, "test"));

    let res = client.try_deposit(&user, &token_id, &500);
    assert!(res.is_err());
}

#[test]
fn test_deposit_paused_unpause_allows() {
    let (env, client, _admin, user, _token_client, token_admin, token_id) = setup_env();

    token_admin.mint(&user, &1000);
    client.pause_operation(&PauseFlag::Deposit, &Symbol::new(&env, "test"));
    client.unpause_operation(&PauseFlag::Deposit);

    client.deposit(&user, &token_id, &500);
    assert_eq!(client.get_position_balance(&user, &token_id), 500);
}

// ---------------------------------------------------------------------------
// Borrowing
// ---------------------------------------------------------------------------

#[test]
fn test_borrow_not_paused_by_default() {
    let (_env, client, _admin, _user, _token_client, _token_admin, _token_id) = setup_env();

    // Unpausing something that is not paused should fail -> confirms it starts unpaused
    let res = client.try_unpause_operation(&PauseFlag::Borrow);
    assert!(res.is_err());
}

#[test]
fn test_borrow_pause_toggle() {
    let (env, client, _admin, _user, _token_client, _token_admin, _token_id) = setup_env();

    client.pause_operation(&PauseFlag::Borrow, &Symbol::new(&env, "test"));

    // Pausing again should fail (already paused)
    let res = client.try_pause_operation(&PauseFlag::Borrow, &Symbol::new(&env, "test2"));
    assert!(res.is_err());

    client.unpause_operation(&PauseFlag::Borrow);

    // Unpausing again should fail (already unpaused)
    let res = client.try_unpause_operation(&PauseFlag::Borrow);
    assert!(res.is_err());
}

// ---------------------------------------------------------------------------
// Withdrawals
// ---------------------------------------------------------------------------

#[test]
fn test_withdraw_allowed_when_not_paused() {
    let (_env, client, _admin, user, _token_client, token_admin, token_id) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);
    client.withdraw(&user, &token_id, &200);
    assert_eq!(client.get_position_balance(&user, &token_id), 300);
}

#[test]
fn test_withdraw_paused() {
    let (env, client, _admin, user, _token_client, token_admin, token_id) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    client.pause_operation(&PauseFlag::Withdraw, &Symbol::new(&env, "test"));

    let res = client.try_withdraw(&user, &token_id, &100);
    assert!(res.is_err());
}

#[test]
fn test_withdraw_allowed_when_deposit_paused() {
    let (env, client, _admin, user, _token_client, token_admin, token_id) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    // Pause deposits but NOT withdrawals
    client.pause_operation(&PauseFlag::Deposit, &Symbol::new(&env, "no_deposits"));

    // Deposit should fail
    let res = client.try_deposit(&user, &token_id, &100);
    assert!(res.is_err());

    // Withdrawal should still work
    client.withdraw(&user, &token_id, &100);
    assert_eq!(client.get_position_balance(&user, &token_id), 400);
}

// ---------------------------------------------------------------------------
// Liquidations
// ---------------------------------------------------------------------------

#[test]
fn test_liquidation_allowed_when_not_paused() {
    let (_env, client, _admin, user, _token_client, token_admin, token_id, engine) =
        setup_env_with_engine();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    client.seize_collateral(&engine, &user, &token_id, &200);
    assert_eq!(client.get_position_balance(&user, &token_id), 300);
}

#[test]
fn test_liquidation_paused() {
    let (env, client, _admin, user, _token_client, token_admin, token_id, engine) =
        setup_env_with_engine();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    client.pause_operation(&PauseFlag::Liquidation, &Symbol::new(&env, "halt_liq"));

    let res = client.try_seize_collateral(&engine, &user, &token_id, &200);
    assert!(res.is_err());
}

#[test]
fn test_liquidation_allowed_when_deposit_paused() {
    let (env, client, _admin, user, _token_client, token_admin, token_id, engine) =
        setup_env_with_engine();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    // Pause deposits
    client.pause_operation(&PauseFlag::Deposit, &Symbol::new(&env, "no_deposits"));

    // Liquidation should still work (liquidations protect solvency)
    client.seize_collateral(&engine, &user, &token_id, &200);
    assert_eq!(client.get_position_balance(&user, &token_id), 300);
}

// ---------------------------------------------------------------------------
// Recovery
// ---------------------------------------------------------------------------

#[test]
fn test_recovery_pause_toggle_authorized() {
    let (env, client, _admin, _user, _token_client, _token_admin, _token_id) = setup_env();

    client.pause_operation(&PauseFlag::Recovery, &Symbol::new(&env, "recover"));
    client.unpause_operation(&PauseFlag::Recovery);

    // Verify we can re-pause
    client.pause_operation(&PauseFlag::Recovery, &Symbol::new(&env, "recover2"));
}

#[test]
fn test_recovery_pause_unauthorized_fails() {
    let (env, client, _admin, _user, _token_client, _token_admin, _token_id) = setup_env();

    // Clear all auths so require_auth fails
    env.set_auths(&[]);

    let res = client.try_pause_operation(&PauseFlag::Recovery, &Symbol::new(&env, "hack"));
    assert!(res.is_err());
}

#[test]
fn test_recovery_config_operations_unaffected_by_pause() {
    let (env, client, _admin, _user, _token_client, _token_admin, _token_id) = setup_env();

    // Pause recovery
    client.pause_operation(&PauseFlag::Recovery, &Symbol::new(&env, "test"));

    // Config operations (set_oracle, set_pool, etc.) should still work
    // because they require admin auth and are not gated by any pause flag
    let new_oracle = Address::generate(&env);
    client.set_oracle(&new_oracle);
}

// ---------------------------------------------------------------------------
// Read methods - always accessible
// ---------------------------------------------------------------------------

#[test]
fn test_read_methods_always_accessible() {
    let (env, client, _admin, user, _token_client, token_admin, token_id) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    // Pause all operations
    client.pause_operation(&PauseFlag::Deposit, &Symbol::new(&env, "all"));
    client.pause_operation(&PauseFlag::Borrow, &Symbol::new(&env, "all"));
    client.pause_operation(&PauseFlag::Withdraw, &Symbol::new(&env, "all"));
    client.pause_operation(&PauseFlag::Liquidation, &Symbol::new(&env, "all"));
    client.pause_operation(&PauseFlag::Recovery, &Symbol::new(&env, "all"));

    // All read methods must still work
    assert!(client.get_admin().is_some());
    assert_eq!(client.get_position_balance(&user, &token_id), 500);
    assert!(client.get_position_index().contains(&user));
    let position = client.get_position(&user);
    assert_eq!(position.collateral.len(), 1);
    assert!(client.is_supported_asset(&token_id));
    let positions = client.get_all_positions();
    assert_eq!(positions.len(), 1);
}

// ---------------------------------------------------------------------------
// Authorization
// ---------------------------------------------------------------------------

#[test]
fn test_emergency_role_succeeds() {
    let (env, client, _admin, _user, _token_client, _token_admin, _token_id) = setup_env();

    // Admin (emergency role) can pause and unpause
    client.pause_operation(&PauseFlag::Deposit, &Symbol::new(&env, "auth_test"));
    let paused = env.as_contract(&client.address, || {
        storage::is_operation_paused(&env, &PauseFlag::Deposit)
    });
    assert!(paused);

    client.unpause_operation(&PauseFlag::Deposit);
    let paused = env.as_contract(&client.address, || {
        storage::is_operation_paused(&env, &PauseFlag::Deposit)
    });
    assert!(!paused);
}

#[test]
fn test_non_emergency_role_fails() {
    let (env, client, _admin, _user, _token_client, _token_admin, _token_id) = setup_env();

    // Clear all auths so require_auth fails for admin
    env.set_auths(&[]);

    let res = client.try_pause_operation(&PauseFlag::Deposit, &Symbol::new(&env, "hack"));
    assert!(res.is_err());
}

#[test]
fn test_unpause_unauthorized_fails() {
    let (env, client, _admin, _user, _token_client, _token_admin, _token_id) = setup_env();

    client.pause_operation(&PauseFlag::Deposit, &Symbol::new(&env, "setup"));

    // Clear all auths
    env.set_auths(&[]);

    let res = client.try_unpause_operation(&PauseFlag::Deposit);
    assert!(res.is_err());
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

#[test]
fn test_pause_event_emitted() {
    let (env, client, _admin, _user, _token_client, _token_admin, _token_id) = setup_env();

    client.pause_operation(&PauseFlag::Deposit, &Symbol::new(&env, "security_issue"));

    let all_events = env.events().all();
    let last_event = all_events.last().unwrap();
    assert_eq!(last_event.0, client.address);

    use soroban_sdk::TryFromVal;
    let event_symbol = Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(event_symbol, Symbol::new(&env, "operation_paused"));
}

#[test]
fn test_unpause_event_emitted() {
    let (env, client, _admin, _user, _token_client, _token_admin, _token_id) = setup_env();

    client.pause_operation(&PauseFlag::Withdraw, &Symbol::new(&env, "test"));
    client.unpause_operation(&PauseFlag::Withdraw);

    let all_events = env.events().all();
    let last_event = all_events.last().unwrap();
    assert_eq!(last_event.0, client.address);

    use soroban_sdk::TryFromVal;
    let event_symbol = Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(event_symbol, Symbol::new(&env, "operation_unpaused"));
}

// ---------------------------------------------------------------------------
// Edge cases: multiple flags active simultaneously
// ---------------------------------------------------------------------------

#[test]
fn test_multiple_pause_flags_active() {
    let (env, client, _admin, user, _token_client, token_admin, token_id) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    // Pause deposits and withdrawals simultaneously
    client.pause_operation(&PauseFlag::Deposit, &Symbol::new(&env, "multi"));
    client.pause_operation(&PauseFlag::Withdraw, &Symbol::new(&env, "multi"));

    // Both should be blocked
    let res1 = client.try_deposit(&user, &token_id, &100);
    assert!(res1.is_err());

    let res2 = client.try_withdraw(&user, &token_id, &100);
    assert!(res2.is_err());

    // Verify internal mask state
    let flags = env.as_contract(&client.address, || {
        (
            storage::is_operation_paused(&env, &PauseFlag::Deposit),
            storage::is_operation_paused(&env, &PauseFlag::Withdraw),
            storage::is_operation_paused(&env, &PauseFlag::Liquidation),
            storage::is_operation_paused(&env, &PauseFlag::Borrow),
            storage::is_operation_paused(&env, &PauseFlag::Recovery),
        )
    });
    assert!(flags.0); // Deposit
    assert!(flags.1); // Withdraw
    assert!(!flags.2); // Liquidation
    assert!(!flags.3); // Borrow
    assert!(!flags.4); // Recovery

    // Unpause only deposits
    client.unpause_operation(&PauseFlag::Deposit);

    // Deposits work again, withdrawals still blocked
    client.deposit(&user, &token_id, &100);
    let res3 = client.try_withdraw(&user, &token_id, &100);
    assert!(res3.is_err());

    let flags = env.as_contract(&client.address, || {
        (
            !storage::is_operation_paused(&env, &PauseFlag::Deposit),
            storage::is_operation_paused(&env, &PauseFlag::Withdraw),
        )
    });
    assert!(flags.0); // Deposit unpaused
    assert!(flags.1); // Withdraw still paused
}

// ---------------------------------------------------------------------------
// Edge cases: repeated pause/unpause
// ---------------------------------------------------------------------------

#[test]
fn test_double_pause_same_operation_fails() {
    let (env, client, _admin, _user, _token_client, _token_admin, _token_id) = setup_env();

    client.pause_operation(&PauseFlag::Deposit, &Symbol::new(&env, "first"));

    let res = client.try_pause_operation(&PauseFlag::Deposit, &Symbol::new(&env, "second"));
    assert!(res.is_err());
}

#[test]
fn test_double_unpause_same_operation_fails() {
    let (env, client, _admin, _user, _token_client, _token_admin, _token_id) = setup_env();

    client.pause_operation(&PauseFlag::Deposit, &Symbol::new(&env, "test"));
    client.unpause_operation(&PauseFlag::Deposit);

    let res = client.try_unpause_operation(&PauseFlag::Deposit);
    assert!(res.is_err());
}

// ---------------------------------------------------------------------------
// Edge cases: incomplete configuration after unpause
// ---------------------------------------------------------------------------

#[test]
fn test_incomplete_config_after_unpause_blocks_deposit() {
    let (env, client, _admin, user, _token_client, token_admin, token_id) = setup_env();

    token_admin.mint(&user, &1000);

    // Pause deposits
    client.pause_operation(&PauseFlag::Deposit, &Symbol::new(&env, "test"));

    // Unpause
    client.unpause_operation(&PauseFlag::Deposit);

    // Remove the supported asset to simulate incomplete config
    client.remove_supported_asset(&token_id);

    // Deposit should fail due to unsupported asset, not pause
    let res = client.try_deposit(&user, &token_id, &500);
    assert!(res.is_err());
}

#[test]
fn test_incomplete_config_after_unpause_blocks_withdraw() {
    let (env, client, _admin, user, _token_client, token_admin, token_id) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    // Pause withdrawals
    client.pause_operation(&PauseFlag::Withdraw, &Symbol::new(&env, "test"));

    // Unpause
    client.unpause_operation(&PauseFlag::Withdraw);

    // Delist with open position fails
    let res = client.try_remove_supported_asset(&token_id);
    assert_eq!(res, Err(Ok(VaultError::AssetHasOpenPositions)));

    // Full withdraw to reduce position balance to 0
    client.withdraw(&user, &token_id, &500);

    // Delist after full withdraw succeeds
    client.remove_supported_asset(&token_id);

    // Withdraw should fail due to unsupported asset
    let res = client.try_withdraw(&user, &token_id, &100);
    assert!(res.is_err());
}

// ---------------------------------------------------------------------------
// All operations independent
// ---------------------------------------------------------------------------

#[test]
fn test_all_operations_independent() {
    let (env, client, _admin, user, _token_client, token_admin, token_id) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    // Pause every operation
    client.pause_operation(&PauseFlag::Deposit, &Symbol::new(&env, "full"));
    client.pause_operation(&PauseFlag::Borrow, &Symbol::new(&env, "full"));
    client.pause_operation(&PauseFlag::Withdraw, &Symbol::new(&env, "full"));
    client.pause_operation(&PauseFlag::Liquidation, &Symbol::new(&env, "full"));
    client.pause_operation(&PauseFlag::Recovery, &Symbol::new(&env, "full"));

    // All flags should be set
    let all_paused = env.as_contract(&client.address, || {
        storage::is_operation_paused(&env, &PauseFlag::Deposit)
            && storage::is_operation_paused(&env, &PauseFlag::Borrow)
            && storage::is_operation_paused(&env, &PauseFlag::Withdraw)
            && storage::is_operation_paused(&env, &PauseFlag::Liquidation)
            && storage::is_operation_paused(&env, &PauseFlag::Recovery)
    });
    assert!(all_paused);

    // Unpause one at a time and verify isolation
    client.unpause_operation(&PauseFlag::Deposit);
    let flags = env.as_contract(&client.address, || {
        (
            storage::is_operation_paused(&env, &PauseFlag::Deposit),
            storage::is_operation_paused(&env, &PauseFlag::Borrow),
            storage::is_operation_paused(&env, &PauseFlag::Withdraw),
            storage::is_operation_paused(&env, &PauseFlag::Liquidation),
            storage::is_operation_paused(&env, &PauseFlag::Recovery),
        )
    });
    assert!(!flags.0); // Deposit unpaused
    assert!(flags.1); // Borrow still paused
    assert!(flags.2); // Withdraw still paused
    assert!(flags.3); // Liquidation still paused
    assert!(flags.4); // Recovery still paused

    client.unpause_operation(&PauseFlag::Borrow);
    let flags = env.as_contract(&client.address, || {
        (
            storage::is_operation_paused(&env, &PauseFlag::Borrow),
            storage::is_operation_paused(&env, &PauseFlag::Withdraw),
        )
    });
    assert!(!flags.0); // Borrow unpaused
    assert!(flags.1); // Withdraw still paused

    client.unpause_operation(&PauseFlag::Withdraw);
    let paused = env.as_contract(&client.address, || {
        storage::is_operation_paused(&env, &PauseFlag::Withdraw)
    });
    assert!(!paused);

    client.unpause_operation(&PauseFlag::Liquidation);
    let paused = env.as_contract(&client.address, || {
        storage::is_operation_paused(&env, &PauseFlag::Liquidation)
    });
    assert!(!paused);

    client.unpause_operation(&PauseFlag::Recovery);
    let paused = env.as_contract(&client.address, || {
        storage::is_operation_paused(&env, &PauseFlag::Recovery)
    });
    assert!(!paused);

    // All unpaused now - verify deposit/withdraw work
    client.deposit(&user, &token_id, &100);
    assert_eq!(client.get_position_balance(&user, &token_id), 600);
}
