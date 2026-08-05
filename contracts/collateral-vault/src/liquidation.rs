use soroban_sdk::{token, Address, Env};

use crate::errors::VaultError;
use crate::events;
use crate::position::checked_debit;
use crate::storage;

/// Execute a collateral seizure. Caller (`lib.rs`) is responsible for calling
/// `liquidation_engine.require_auth()` before this function.
///
/// Validates engine registration, pause state, and delegates amount/balance
/// validation and storage updates to `checked_debit`. On success, transfers
/// tokens to the liquidation engine and emits `CollateralSeized`.
pub fn execute_seize_authorized(
    env: &Env,
    liquidation_engine: Address,
    user: Address,
    asset: Address,
    amount: i128,
) -> Result<(), VaultError> {
    let registered_engine = storage::get_liquidation_engine(env).ok_or(VaultError::Unauthorized)?;
    if liquidation_engine != registered_engine {
        return Err(VaultError::Unauthorized);
    }

    if storage::is_paused(env) {
        return Err(VaultError::VaultPaused);
    }

    let _new_balance = checked_debit(env, &user, &asset, amount)?;

    let token_client = token::Client::new(env, &asset);
    token_client.transfer(
        &env.current_contract_address(),
        &liquidation_engine,
        &amount,
    );

    events::CollateralSeized {
        user,
        asset,
        amount,
        liquidation_engine,
    }
    .publish(env);

    Ok(())
}
