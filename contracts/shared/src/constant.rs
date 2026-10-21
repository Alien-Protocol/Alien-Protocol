/// Standard quote-token precision (7 decimals), i.e. 1.0 quote unit.
///
/// This is the scaling factor used throughout the protocol to represent quote
/// asset amounts in integer fixed-point form. All ratios and percentages are
/// expressed in basis points (BPS) relative to `BPS_DENOMINATOR`.
pub const PROTOCOL_QUOTE_PRECISION: i128 = 10_000_000;

/// Number of basis points in 100% (1 BPS = 0.01%).
///
/// Used to convert BPS-encoded ratios (e.g. LTVs, thresholds, close factors)
/// into scaled integer multipliers.
pub const BPS_DENOMINATOR: i128 = 10_000;

/// Seconds in a non-leap 365-day year, used for APR accrual math.
pub const SECONDS_PER_YEAR: u64 = 31_536_000;

/// Default v1 borrow APR (expressed in BPS) applied to newly opened pools.
pub const V1_BORROW_APR_BPS: u32 = 800;

/// Default maximum loan-to-value ratio (70%) for collateral positions.
pub const DEFAULT_MAX_LTV_BPS: u32 = 7_000;

/// Default liquidation threshold (80%). A position is liquidatable once its
/// health factor falls below `HEALTHY_HF_BPS`.
pub const DEFAULT_LIQUIDATION_THRESHOLD_BPS: u32 = 8_000;

/// Health factor of 1.0 (100%). At or above this value a position is
/// considered healthy and not liquidatable.
pub const HEALTHY_HF_BPS: i128 = 10_000;

/// Post-liquidation target health factor (1.10).
///
/// When the liquidation solver selects a repay amount, it aims to leave the
/// position just above healthy rather than at the threshold. This headroom
/// (1.10 vs 1.00) absorbs price movement between transaction simulation and
/// execution and reduces the chance of a position remaining immediately
/// reliquidatable. It is also the target used by the liquidation solver to
/// derive the repay amount from the close factor and bonus.
pub const TARGET_HF_BPS: i128 = 11_000;

/// Liquidation bonus (8%) granted to liquidators on repaid debt.
///
/// Expressed in BPS and applied to the repay amount when computing the
/// collateral seized. This incentivizes liquidators to clear bad debt while
/// protecting the protocol from over-seizing collateral.
pub const LIQUIDATION_BONUS_BPS: u32 = 800;

/// Maximum fraction of outstanding debt that a single liquidation may repay
/// (50%), expressed in BPS.
///
/// The close factor caps the repay amount so that large positions cannot be
/// closed in a single transaction, limiting capital requirements for liquidators
/// and spreading liquidation pressure. The liquidation solver combines this
/// cap with `TARGET_HF_BPS` to determine the effective repay amount.
pub const CLOSE_FACTOR_BPS: u32 = 5_000;

/// Minimum remaining debt below which a residual balance is treated as dust
/// and forced to zero during liquidation / repayment.
///
/// Dust here means a debt remainder so small that it is economically
/// insignificant relative to transaction fees and cannot be repaid without
/// costing more than the debt itself. Leaving such a remainder would block
/// position closure and create a perpetually uncloseable position.
///
/// One quote unit (1.0 in quote units, i.e. `PROTOCOL_QUOTE_PRECISION`) is chosen
/// deliberately: it is the smallest non-zero amount representable at the
/// protocol's quote precision, so any remaining debt at or below this threshold
/// is below the granularity of the accounting system and effectively zero.
/// Tiering the threshold to the precision (and not an arbitrary larger value)
/// keeps the dust rule consistent with the unit of account and avoids writing
/// off small but non-trivial balances.
///
/// Code paths that depend on this constant:
/// - The liquidation solver's dust fallback: when the computed repay would leave
///   a remaining debt at or below this value, the solver instead repays the full
///   debt and closes the position.
/// - Per TODO §1.3, this should also gate repay dust in the pool: repays that
///   would leave less than or equal to this amount outstanding are treated as
///   full repayments.
pub const MIN_REMAINING_DEBT: i128 = PROTOCOL_QUOTE_PRECISION;
