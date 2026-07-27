/// config.rs — typed accessors for required contract-configuration addresses.
///
/// Every function in this module returns `Result<T, VaultError>` so callers
/// never need to reach for `expect` or `unwrap`.  The mapping between a missing
/// storage key and the corresponding error variant is kept in one place here,
/// making the error surface easy to audit.
use crate::{errors::VaultError, storage};
use soroban_sdk::{Address, Env};

/// Return the admin address, or `VaultError::NotInitialized` if the vault has
/// not been initialized yet.
pub fn require_admin(env: &Env) -> Result<Address, VaultError> {
    storage::get_admin(env).ok_or(VaultError::NotInitialized)
}

/// Return the oracle address, or `VaultError::OracleNotConfigured` if it has
/// never been set.
pub fn require_oracle(env: &Env) -> Result<Address, VaultError> {
    storage::get_oracle(env).ok_or(VaultError::OracleNotConfigured)
}

/// Return the liquidation-engine address, or
/// `VaultError::LiquidationEngineNotSet` if it has never been set.
pub fn require_liquidation_engine(env: &Env) -> Result<Address, VaultError> {
    storage::get_liquidation_engine(env).ok_or(VaultError::LiquidationEngineNotSet)
}

/// Return the lending-pool address, or `VaultError::PoolNotSet` if it has
/// never been set.
pub fn require_pool(env: &Env) -> Result<Address, VaultError> {
    storage::get_pool(env).ok_or(VaultError::PoolNotSet)
}
