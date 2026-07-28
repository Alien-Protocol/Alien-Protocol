//! Liquidation domain.
//!
//! Both `seize_collateral` and `authorize_liquidation` delegate balance
//! mutation to `position::debit_position` — the single source of truth
//! for that invariant.

use crate::clients::LendingPoolClient;
use crate::errors::VaultError;
use crate::events;
use crate::position::{debit_position, require_not_paused, require_position};
use crate::storage;
use soroban_sdk::{token, Address, Env};

/// Transfer `amount` of `asset` from `user`'s position to `liquidation_engine`.
///
/// Only the registered liquidation engine may call this.
pub fn seize_collateral(
    env: &Env,
    liquidation_engine: Address,
    user: Address,
    asset: Address,
    amount: i128,
) {
    liquidation_engine.require_auth();

    let registered_engine =
        storage::get_liquidation_engine(env).expect("liquidation engine not authorized");
    if liquidation_engine != registered_engine {
        soroban_sdk::panic_with_error!(env, VaultError::Unauthorized);
    }

    require_not_paused(env);
    require_position(env, &user);

    // debit_position handles balance check, asset cleanup, and user-index cleanup
    debit_position(env, &user, &asset, amount);

    let token_client = token::Client::new(env, &asset);
    token_client.transfer(&env.current_contract_address(), &liquidation_engine, &amount);

    events::CollateralSeized {
        user,
        asset,
        amount,
        liquidation_engine,
    }
    .publish(env);
}

/// Returns `true` when the registered lending pool reports that `user` is
/// liquidatable. Panics if the caller is not the registered engine.
pub fn authorize_liquidation(env: &Env, liquidation_engine: Address, user: Address) -> bool {
    let stored_engine =
        storage::get_liquidation_engine(env).expect("Liquidation engine not set");
    if liquidation_engine != stored_engine {
        soroban_sdk::panic_with_error!(env, VaultError::Unauthorized);
    }

    liquidation_engine.require_auth();
    require_position(env, &user);

    let pool_address = storage::get_pool(env).expect("Lending pool not set");
    let pool_client = LendingPoolClient::new(env, &pool_address);
    pool_client.is_liquidatable(&user)
}
