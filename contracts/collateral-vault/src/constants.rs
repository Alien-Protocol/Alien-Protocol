/// Denominator for all fixed-point basis-point values.
/// 10_000 bps = 100%.
pub const BPS_DENOMINATOR: i128 = 10_000;

/// Oracle prices are encoded with 7 decimal places (e.g. $1.00 = 10_000_000).
/// Dividing `amount * price` by this constant yields the USD-denominated value.
pub const PRICE_PRECISION: i128 = 10_000_000;

// ---------------------------------------------------------------------------
// LTV bounds
// ---------------------------------------------------------------------------

/// Minimum allowed loan-to-value ratio in basis points (1% = 100 bps).
pub const MIN_LTV_BPS: i128 = 100;

/// Maximum allowed loan-to-value ratio in basis points (99% = 9_900 bps).
/// Must always be strictly below MAX_LIQ_THRESHOLD_BPS so the invariant
/// ltv_bps < liquidation_threshold_bps can be satisfied.
pub const MAX_LTV_BPS: i128 = 9_900;

// ---------------------------------------------------------------------------
// Liquidation-threshold bounds
// ---------------------------------------------------------------------------

/// Minimum allowed liquidation threshold in basis points (1% = 100 bps).
pub const MIN_LIQ_THRESHOLD_BPS: i128 = 100;

/// Maximum allowed liquidation threshold in basis points (100% = 10_000 bps).
pub const MAX_LIQ_THRESHOLD_BPS: i128 = 10_000;

// ---------------------------------------------------------------------------
// Liquidation-bonus bounds
// ---------------------------------------------------------------------------

/// Minimum allowed liquidation bonus in basis points (0% = 0 bps).
pub const MIN_LIQ_BONUS_BPS: i128 = 0;

/// Maximum allowed liquidation bonus in basis points (50% = 5_000 bps).
pub const MAX_LIQ_BONUS_BPS: i128 = 5_000;
