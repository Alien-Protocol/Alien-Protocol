//! Cross-contract client re-exports for the collateral-vault.
//!
//! This module is the **only** place inside the vault crate that references
//! cross-contract client types.  All other vault modules that need to call an
//! external contract must import the client from here rather than from
//! `shared::interfaces` directly.
//!
//! Adding a new external call to the vault means:
//! 1. Add or update the corresponding trait in `shared::interfaces`.
//! 2. Add a `pub use` line here.
//! 3. Update `INTERFACE_VERSION` in `shared::interfaces` if the signature changed.

pub use shared::interfaces::{LendingPoolClient, OracleAdapterClient};
// Re-export interface traits for consumers that need them for type bounds.
#[allow(unused_imports)]
pub use shared::interfaces::{LendingPoolInterface, OracleInterface};
