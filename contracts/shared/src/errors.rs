//! Shared protocol-level error codes for Alien Protocol.
//!
//! # Error ownership
//!
//! `ProtocolError` is the single authoritative source for error codes that
//! cross contract boundaries or appear in shared interface return types.
//!
//! Each individual contract also keeps its own `*Error` enum (e.g. `VaultError`,
//! `OracleError`) for errors that are strictly internal to that contract and
//! never need to be decoded by a caller on the other side of a cross-contract
//! call.  When a new error must be visible across contracts it belongs here.

use soroban_sdk::contracterror;

/// Cross-contract protocol errors.
///
/// Discriminants are stable: once published they must not change.
/// Add new variants at the end to maintain wire compatibility.
#[contracterror]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ProtocolError {
    /// Contract has already been initialized and cannot be re-initialized.
    AlreadyInitialized = 1,
    /// Caller is not authorized to perform this operation.
    Unauthorized = 2,
    /// One or more input values are out of range or logically invalid.
    InvalidInputs = 3,
    /// The referenced asset is not on the supported-asset list.
    UnsupportedAsset = 4,
    /// The user has no collateral position in the vault.
    NoPosition = 5,
    /// The vault is paused; state-mutating operations are temporarily disabled.
    VaultPaused = 6,
    /// The withdrawal would push the collateral ratio below the protocol minimum.
    BelowMinCollateralRatio = 7,
    /// No price exists for the requested asset in the oracle.
    PriceNotFound = 8,
    /// The stored price is older than the staleness threshold.
    StalePrice = 9,
    /// The oracle contract address has not been configured in the vault.
    OracleNotConfigured = 10,
}
