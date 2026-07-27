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

/// Identifies which operation category to pause or unpause.
/// Each variant maps to a single bit in the pause bitmask stored in persistent
/// storage, making the system compact and extensible for future operations.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub enum PauseFlag {
    Deposit,
    Borrow,
    Withdraw,
    Liquidation,
    Recovery,
}

impl PauseFlag {
    /// Returns the bit mask for this pause flag.
    pub fn bit(&self) -> u32 {
        match self {
            PauseFlag::Deposit => 1 << 0,
            PauseFlag::Borrow => 1 << 1,
            PauseFlag::Withdraw => 1 << 2,
            PauseFlag::Liquidation => 1 << 3,
            PauseFlag::Recovery => 1 << 4,
        }
    }
}

/// Returns a human-readable symbol for a pause flag (used in events).
pub fn pause_flag_symbol(env: &soroban_sdk::Env, flag: &PauseFlag) -> soroban_sdk::Symbol {
    match flag {
        PauseFlag::Deposit => soroban_sdk::Symbol::new(env, "deposit"),
        PauseFlag::Borrow => soroban_sdk::Symbol::new(env, "borrow"),
        PauseFlag::Withdraw => soroban_sdk::Symbol::new(env, "withdraw"),
        PauseFlag::Liquidation => soroban_sdk::Symbol::new(env, "liquidation"),
        PauseFlag::Recovery => soroban_sdk::Symbol::new(env, "recovery"),
    }
}

/// Storage keys for persistent contract state.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum DataKey {
    /// Admin address key
    Admin,
    /// Bitmask of paused operations (replaces single Paused boolean)
    PauseMask,
    /// Lending pool address key
    LendingPool,
    /// Supported asset key: stores whether a specific asset is supported
    SupportedAsset(Address),
    /// List of all supported assets
    SupportedAssets,
    /// Position key: stores balance for a user's position in an asset
    Position(Address, Address),
    /// Position index key: tracks all users with active positions
    PositionIndex,
    /// Tracks which assets a user has ever deposited into
    UserAssets(Address),
    /// Oracle adapter address
    Oracle,
    /// Liquidation engine address
    LiquidationEngine,
    /// Lending pool address (alternative key)
    Pool,
}

/// Price data from the oracle.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PriceData {
    pub price: i128,
    pub timestamp: u64,
}
