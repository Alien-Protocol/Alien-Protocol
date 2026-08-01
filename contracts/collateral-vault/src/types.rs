//! Collateral-vault types.
//!
//! Cross-contract DTOs ([`PriceData`], [`CollateralAsset`], [`Position`]) are
//! re-exported from `shared::types` — they have a single authoritative
//! definition there.
//!
//! [`DataKey`] is vault-internal storage and must **not** be moved to `shared`;
//! changing its discriminants would silently corrupt persistent ledger state on
//! already-deployed contracts.

use soroban_sdk::{contracttype, Address};

// ── Cross-contract DTOs (canonical definitions live in `shared`) ─────────────

/// Re-exported canonical price DTO.  See [`shared::types::PriceData`].
/// Used by test modules that construct mock oracle contracts returning this type.
#[allow(unused_imports)]
pub use shared::types::PriceData;

/// Re-exported canonical collateral-asset record.  See [`shared::types::CollateralAsset`].
pub use shared::types::CollateralAsset;

/// Re-exported canonical user position.  See [`shared::types::Position`].
pub use shared::types::Position;

// ── Vault-internal storage keys ──────────────────────────────────────────────

/// Storage keys for persistent vault state.
///
/// Discriminants are stable: changing them would corrupt existing ledger data.
/// Add new variants at the end only.
/// Storage keys for persistent contract state.
///
/// Each external dependency has exactly one canonical key:
/// - `Admin` — contract administrator
/// - `LendingPool` — lending pool / debt source
/// - `Oracle` — price oracle adapter
/// - `LiquidationEngine` — authorized liquidation engine
/// - `Paused` — circuit-breaker flag
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum DataKey {
    /// Contract administrator address.
    Admin,
    /// Whether the vault is currently paused.
    Paused,
    /// Address of the configured lending-pool contract (legacy key).
    /// Canonical lending pool address
    LendingPool,
    /// Whether a specific asset is on the supported-asset allowlist.
    SupportedAsset(Address),
    /// Ordered list of all supported asset addresses.
    SupportedAssets,
    /// Per-user, per-asset balance: `(user, asset) → i128`.
    Position(Address, Address),
    /// Ordered index of all users that have an active position.
    PositionIndex,
    /// Assets a user has ever deposited (used to rebuild their `Position`).
    UserAssets(Address),
    /// Address of the configured oracle-adapter contract.
    Oracle,
    /// Address of the configured liquidation-engine contract.
    LiquidationEngine,
    /// Address of the configured lending-pool contract (primary key).
    Pool,
    /// Monotonically increasing contract bytecode version.
    ContractVersion,
    /// Monotonically increasing storage-schema version.
    StorageSchemaVersion,
}
