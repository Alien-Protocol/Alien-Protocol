//! Storage lifetime and TTL policy for the collateral-vault contract.
//!
//! # Storage classification
//!
//! Soroban provides three storage classes with different lifetime semantics:
//!
//! | Storage class | Lifetime                        | Used for                              |
//! |---------------|---------------------------------|---------------------------------------|
//! | `instance`    | Tied to contract instance TTL   | Contract-wide configuration           |
//! | `persistent`  | Finite TTL, survives checkpoints| Per-user positions, asset registries  |
//! | `temporary`   | Expires automatically           | Not used in this contract             |
//!
//! ## DataKey → storage class mapping
//!
//! | DataKey variant          | Storage class | TTL policy                          |
//! |--------------------------|---------------|-------------------------------------|
//! | `Admin`                  | instance      | Extended with instance TTL          |
//! | `Paused`                 | instance      | Extended with instance TTL          |
//! | `LendingPool`            | instance      | Extended with instance TTL          |
//! | `Oracle`                 | instance      | Extended with instance TTL          |
//! | `LiquidationEngine`      | instance      | Extended with instance TTL          |
//! | `Pool`                   | instance      | Extended with instance TTL          |
//! | `SupportedAssets`        | instance      | Extended with instance TTL          |
//! | `SupportedAsset(Address)`| persistent    | Extended per key on every read/write|
//! | `Position(Address,Address)`| persistent  | Extended per key on every read/write|
//! | `PositionIndex`          | persistent    | Extended on every read/write        |
//! | `UserAssets(Address)`    | persistent    | Extended per key on every read/write|
//!
//! Configuration keys (`Admin`, `Paused`, `LendingPool`, `Oracle`,
//! `LiquidationEngine`, `Pool`, `SupportedAssets`) are stored in `instance`
//! storage because they are contract-wide, change infrequently, and share the
//! contract-instance lifetime.  Keeping them in `instance` means a single
//! `extend_ttl` call covers all of them simultaneously.
//!
//! Per-user data (`Position`, `UserAssets`) and shared indexes
//! (`PositionIndex`, `SupportedAsset`) are stored in `persistent` storage
//! because they are keyed per user or asset and must survive independently.
//!
//! # TTL extension strategy
//!
//! Extensions are **conditional**: an extension is only performed when the
//! remaining TTL of the entry has fallen below `TTL_THRESHOLD_*`.  This avoids
//! paying unnecessary resource fees on every invocation when the TTL is already
//! healthy.
//!
//! The `extend_ttl` call is a no-op when the current TTL is already ≥ the
//! target, so it is safe to call unconditionally — but the threshold check
//! makes the intent explicit and avoids surprising behaviour.
//!
//! # Restoration path for archived persistent entries
//!
//! When a persistent entry's TTL reaches zero it becomes **archived**.  Archived
//! entries are inaccessible to contract reads but are not permanently deleted.
//!
//! To restore an archived entry:
//! 1. Identify the archived `LedgerKey` (contract address + `DataKey` variant).
//! 2. Submit a `RestoreFootprint` transaction on Stellar that includes the
//!    archived key in its read-write footprint.
//! 3. The entry is restored to the minimum persistent TTL and is accessible again.
//!
//! **Operational responsibility**: the protocol does not perform automatic
//! restoration.  Operators, keepers, or users are responsible for submitting
//! `RestoreFootprint` transactions before interacting with archived positions.
//! The contract admin should monitor instance and contract-code TTL and extend
//! them at least every 14 days using a keeper bot or manual transaction.
//!
//! # Ledger arithmetic
//!
//! Stellar produces approximately one ledger every 6 seconds:
//! - 1 day  ≈  14_400 ledgers
//! - 7 days ≈ 100_800 ledgers
//! - 30 days ≈ 432_000 ledgers

/// Minimum remaining TTL (in ledgers) for **instance** storage below which an
/// extension is triggered.  Approximately 7 days at 6 s/ledger.
///
/// When the contract instance TTL falls below this value on any invocation that
/// touches instance storage, the TTL is extended to [`TTL_TARGET_INSTANCE`].
pub const TTL_THRESHOLD_INSTANCE: u32 = 100_800;

/// Target TTL (in ledgers) to extend **instance** storage to when the threshold
/// is crossed.  Approximately 30 days at 6 s/ledger.
///
/// Must be strictly greater than [`TTL_THRESHOLD_INSTANCE`].
pub const TTL_TARGET_INSTANCE: u32 = 432_000;

/// Minimum remaining TTL (in ledgers) for **persistent** per-key entries below
/// which an extension is triggered.  Approximately 3 days at 6 s/ledger.
///
/// When a persistent entry's TTL falls below this value on any read or write
/// that touches that specific key, the TTL is extended to
/// [`TTL_TARGET_PERSISTENT`].
pub const TTL_THRESHOLD_PERSISTENT: u32 = 43_200;

/// Target TTL (in ledgers) to extend **persistent** per-key entries to when the
/// threshold is crossed.  Approximately 30 days at 6 s/ledger.
///
/// Must be strictly greater than [`TTL_THRESHOLD_PERSISTENT`].
pub const TTL_TARGET_PERSISTENT: u32 = 432_000;
