//! Oracle-adapter-specific types.
//!
//! [`PriceData`] is re-exported from `shared::types` — it is the single
//! authoritative definition used by every contract that exchanges price
//! information.  The [`DataKey`] enum is oracle-internal storage and must not
//! be moved to `shared`.

#![allow(unused_imports)]
use soroban_sdk::{contracttype, Address};

/// Re-export the canonical price DTO from the shared crate so that callers of
/// this module can import it from one place without knowing the origin crate.
pub use shared::types::PriceData;

/// Storage keys used exclusively by the oracle-adapter contract.
///
/// These are internal to oracle-adapter and must not be referenced by other
/// contracts directly.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Stored price for a specific asset.
    Price(Address),
    /// Administrator address.
    Admin,
    /// Maximum age (seconds) a price may have before it is considered stale.
    StalenessThreshold,
    /// Whether the oracle is currently paused.
    Paused,
    /// An address that is authorised to push price updates.
    AuthorizedFeeder(Address),
}
