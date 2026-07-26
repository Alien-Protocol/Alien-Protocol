
//! Internal accounting state and custody reconciliation for the collateral vault.

use soroban_sdk::{contracttype, Address, Env};
use shared::VaultError;
use crate::token::SafeTokenClient;

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum DataKey {
    TotalCollateral(Address),
    UserPosition(Address, Address),
    ReentrancyLock,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct CustodyReport {
    pub recorded_liability: i128,
    pub actual_balance: i128,
    pub has_deficit: bool,
}

/// Helper to manage total internal liabilities for a token asset.
pub struct Liabilities;

impl Liabilities {
    pub fn get(env: &Env, token: &Address) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::TotalCollateral(token.clone()))
            .unwrap_or(0)
    }

    pub fn add(env: &Env, token: &Address, amount: i128) -> Result<i128, VaultError> {
        let current = Self::get(env, token);
        let new_total = current.checked_add(amount).ok_or(VaultError::MathOverflow)?;
        env.storage()
            .instance()
            .set(&DataKey::TotalCollateral(token.clone()), &new_total);
        Ok(new_total)
    }

    pub fn sub(env: &Env, token: &Address, amount: i128) -> Result<i128, VaultError> {
        let current = Self::get(env, token);
        let new_total = current.checked_sub(amount).ok_or(VaultError::MathOverflow)?;
        env.storage()
            .instance()
            .set(&DataKey::TotalCollateral(token.clone()), &new_total);
        Ok(new_total)
    }
}

/// Reconciles vault custody balance against recorded internal liabilities.
pub fn reconcile_custody(env: &Env, token: &Address) -> CustodyReport {
    let client = SafeTokenClient::new(env, token);
    let actual_balance = client.balance(&env.current_contract_address());
    let recorded_liability = Liabilities::get(env, token);

    CustodyReport {
        recorded_liability,
        actual_balance,
        has_deficit: actual_balance < recorded_liability,
    }
}
