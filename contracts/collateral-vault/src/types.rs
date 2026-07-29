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
    /// The owner of this position.
    pub user: Address,
    /// All collateral assets held by this user.
    pub collateral: Vec<CollateralAsset>,
}

/// A page of positions returned by the paginated view.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PositionsPage {
    /// The positions in this page.
    pub positions: Vec<Position>,
    /// The slot offset to pass as `cursor` on the next call.
    /// Equal to `u32::MAX` when this is the last page (no more items).
    pub next_cursor: u32,
}

/// A page of asset addresses returned by the paginated asset view.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct AssetsPage {
    /// Asset addresses in this page.
    pub assets: Vec<Address>,
    /// The slot offset to pass as `cursor` on the next call.
    /// Equal to `u32::MAX` when this is the last page (no more items).
    pub next_cursor: u32,
}

/// Sentinel value returned in `next_cursor` when there are no more pages.
pub const NO_NEXT_CURSOR: u32 = u32::MAX;

/// Maximum number of items that may be requested in a single paginated call.
pub const MAX_PAGE_LIMIT: u32 = 50;

/// Storage keys for persistent contract state.
///
/// Indexed collections use a slot-based layout for O(1) add/remove:
/// - `*Count` stores the current length (u32)
/// - `*At(slot)` stores the item at that slot
/// - `*Slot(item)` stores the slot index for an item (reverse lookup)
///
/// This avoids rewriting a monolithic `Vec<T>` on every mutation.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum DataKey {
    /// Admin address key
    Admin,
    /// Paused state key
    Paused,
    /// Lending pool address key
    LendingPool,

    // ── Supported-asset index ────────────────────────────────────────────────
    /// Whether a specific asset address is supported (bool sentinel)
    SupportedAsset(Address),
    /// Number of entries in the supported-asset index
    SupportedAssetCount,
    /// Supported asset at slot `n`
    SupportedAssetAt(u32),
    /// Slot of a supported asset (reverse lookup)
    SupportedAssetSlot(Address),

    // ── Per-(user,asset) balance ─────────────────────────────────────────────
    /// Balance for (user, asset)
    Position(Address, Address),

    // ── Per-user asset index ─────────────────────────────────────────────────
    /// Number of assets tracked for a user
    UserAssetCount(Address),
    /// Asset at slot `n` for user
    UserAssetAt(Address, u32),
    /// Slot of an asset in a user's index (reverse lookup)
    UserAssetSlot(Address, Address),

    // ── Global user / position index ─────────────────────────────────────────
    /// Number of users in the position index
    PositionCount,
    /// User address at slot `n` in the position index
    PositionAt(u32),
    /// Slot of a user in the position index (reverse lookup)
    PositionSlot(Address),

    // ── Other contract addresses ─────────────────────────────────────────────
    /// Oracle adapter address
    Oracle,
    /// Liquidation engine address
    LiquidationEngine,
    /// Lending pool address (alternative key)
    Pool,
    /// Contract version key
    ContractVersion,
    /// Storage schema version key
    StorageSchemaVersion,
}

/// Price data from the oracle.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PriceData {
    pub price: i128,
    pub timestamp: u64,
}
