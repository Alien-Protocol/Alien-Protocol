//! Canonical cross-contract data-transfer objects (DTOs) for Alien Protocol.
//!
//! These are the single authoritative definitions for every type that crosses
//! a contract boundary.  No contract may define its own copy of these structs.
//!
//! Ownership:
//!   - [`PriceData`]       — produced by `oracle-adapter`, consumed by `collateral-vault`
//!   - [`CollateralAsset`] — produced and stored by `collateral-vault`
//!   - [`Position`]        — produced and stored by `collateral-vault`, read by liquidation engine

#![allow(unused_imports)]
use soroban_sdk::{contracttype, Address, Vec};

/// A single price observation returned by the oracle.
///
/// Prices are encoded with 7 decimal places (e.g. USD 1.00 = 10_000_000).
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PriceData {
    /// Asset price, scaled by 10^7.
    pub price: i128,
    /// Unix timestamp of the price observation (seconds).
    pub timestamp: u64,
    /// Unix timestamp at which the price was written to the ledger (seconds).
    pub write_timestamp: u64,
}

/// A single collateral asset held by a user inside the vault.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct CollateralAsset {
    /// The Stellar asset contract address.
    pub asset: Address,
    /// Amount held, expressed in the asset's native smallest unit.
    pub amount: i128,
}

/// The complete collateral position for one user across all supported assets.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Position {
    /// The owner of this position.
    pub user: Address,
    /// All collateral assets currently held by this user.
    pub collateral: Vec<CollateralAsset>,
}
