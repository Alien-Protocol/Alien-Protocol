
//! Safe wrapper for interacting with Stellar/Soroban token contracts.
//! Ensures interface compliance and safe token transfers with checks-effects-interactions integrity.

use soroban_sdk::{token, Address, Env};
use shared::VaultError;

/// Safe wrapper over the standard Stellar token client.
pub struct SafeTokenClient<'a> {
    env: &'a Env,
    token: &'a Address,
    client: token::Client<'a>,
}

impl<'a> SafeTokenClient<'a> {
    pub fn new(env: &'a Env, token: &'a Address) -> Self {
        let client = token::Client::new(env, token);
        Self { env, token, client }
    }

    /// Verifies that a listed asset implements the expected Stellar token interface
    /// by invoking standard read methods (e.g. `decimals`).
    pub fn verify_interface(&self) -> Result<(), VaultError> {
        // Attempt a standard view call on the token client.
        // If the contract does not implement the token interface, this call will fail.
        let _decimals = self.client.decimals();
        Ok(())
    }

    /// Safely transfers tokens from an investor/user to the vault.
    pub fn transfer_from(
        &self,
        from: &Address,
        to: &Address,
        amount: i128,
    ) -> Result<(), VaultError> {
        if amount <= 0 {
            return Err(VaultError::InvalidAmount);
        }
        self.client.transfer_from(from, to, &amount);
        Ok(())
    }

    /// Safely transfers tokens from the vault to a user.
    pub fn transfer(&self, to: &Address, amount: i128) -> Result<(), VaultError> {
        if amount <= 0 {
            return Err(VaultError::InvalidAmount);
        }
        self.client.transfer(to, &amount);
        Ok(())
    }

    /// Fetches the actual on-chain balance held by an address.
    pub fn balance(&self, account: &Address) -> i128 {
        self.client.balance(account)
    }
}
