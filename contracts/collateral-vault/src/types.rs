use soroban_sdk::{contracttype, Address, Vec};

/// Represents a single collateral asset held by a user.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct CollateralAsset {
    pub asset: Address,
    pub amount: i128,
}

/// Represents a user's collateral position across all assets.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Position {
    pub user: Address,
    pub collateral: Vec<CollateralAsset>,
}

/// A page of positions returned by the paginated view.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PositionsPage {
    pub positions: Vec<Position>,
    /// `u32::MAX` when there are no more pages.
    pub next_cursor: u32,
}

/// A page of asset addresses returned by the paginated asset view.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct AssetsPage {
    pub assets: Vec<Address>,
    /// `u32::MAX` when there are no more pages.
    pub next_cursor: u32,
}

/// Sentinel value returned in `next_cursor` when there are no more pages.
pub const NO_NEXT_CURSOR: u32 = u32::MAX;

/// Maximum number of items that may be requested in a single paginated call.
pub const MAX_PAGE_LIMIT: u32 = 50;

/// Storage keys for persistent contract state.
///
/// Slot-based collections use O(1) add/remove via swap-and-pop:
/// - `*Count` — current length
/// - `*At(slot)` — item at slot
/// - `*Slot(item)` — reverse lookup (item → slot)
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum DataKey {
    // ── Admin / config ────────────────────────────────────────────────────────
    Admin,
    Paused,
    LendingPool,
    Oracle,
    LiquidationEngine,
    Pool,

    // ── Supported-asset slot index ────────────────────────────────────────────
    SupportedAsset(Address),
    SupportedAssetCount,
    SupportedAssetAt(u32),
    SupportedAssetSlot(Address),

    // ── Per-(user,asset) balance ──────────────────────────────────────────────
    Position(Address, Address),

    // ── Per-user asset slot index ─────────────────────────────────────────────
    UserAssetCount(Address),
    UserAssetAt(Address, u32),
    UserAssetSlot(Address, Address),

    // ── Global user/position slot index ──────────────────────────────────────
    PositionCount,
    PositionAt(u32),
    PositionSlot(Address),

    // ── Contract / storage versioning ────────────────────────────────────────
    ContractVersion,
    StorageSchemaVersion,
}

/// Price data from the oracle.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PriceData {
    pub price: i128,
    pub timestamp: u64,
}
