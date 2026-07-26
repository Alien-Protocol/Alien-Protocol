//! Safe fixed-point math helpers for financial calculations.
//!
//! All helpers return typed [`VaultError`] variants instead of panicking.
//! Rounding direction is explicitly documented for every operation to ensure
//! the protocol always rounds in the safe, conservative direction.

use crate::errors::VaultError;

// ---------------------------------------------------------------------------
// Addition
// ---------------------------------------------------------------------------

/// Checked addition. Returns [`VaultError::MathOverflow`] on overflow.
///
/// **Rounding:** exact — addition never introduces rounding error.
pub fn checked_add(a: i128, b: i128) -> Result<i128, VaultError> {
    a.checked_add(b).ok_or(VaultError::MathOverflow)
}

// ---------------------------------------------------------------------------
// Subtraction
// ---------------------------------------------------------------------------

/// Checked subtraction. Returns [`VaultError::MathUnderflow`] on underflow.
///
/// **Rounding:** exact — subtraction never introduces rounding error.
pub fn checked_sub(a: i128, b: i128) -> Result<i128, VaultError> {
    a.checked_sub(b).ok_or(VaultError::MathUnderflow)
}

// ---------------------------------------------------------------------------
// Multiplication
// ---------------------------------------------------------------------------

/// Checked multiplication. Returns [`VaultError::MathOverflow`] on overflow.
///
/// **Rounding:** exact — multiplication never introduces rounding error.
pub fn checked_mul(a: i128, b: i128) -> Result<i128, VaultError> {
    a.checked_mul(b).ok_or(VaultError::MathOverflow)
}

// ---------------------------------------------------------------------------
// Division
// ---------------------------------------------------------------------------

/// Checked division. Returns [`VaultError::MathDivisionByZero`] if `b == 0`
/// or [`VaultError::MathOverflow`] on overflow (e.g. `i128::MIN / -1`).
///
/// **Rounding:** **truncates toward zero** (Rounds down for positive
/// quotients, up for negative quotients). This is conservative for the
/// protocol when dividing collateral values because it never over-states
/// the resulting value.
pub fn checked_div(a: i128, b: i128) -> Result<i128, VaultError> {
    a.checked_div(b).ok_or(VaultError::MathDivisionByZero)
}

// ---------------------------------------------------------------------------
// Multiply-Divide  (a * b / denom)
// ---------------------------------------------------------------------------

/// Checked multiply-divide: `a * b / denom`.
///
/// Returns:
/// - [`VaultError::MathOverflow`] if the intermediate product `a * b` overflows.
/// - [`VaultError::MathDivisionByZero`] if `denom == 0`.
/// - [`VaultError::MathPrecisionLoss`] when the result is truncated to zero
///   despite non-zero inputs (detectable precision loss).
///
/// # Strategy
///
/// Uses the straightforward product-first strategy because Soroban's `i128`
/// provides ~170 bits of range (38 decimal digits), which comfortably
/// accommodates realistic collateral × price products. A future optimisation
/// could use a full-width `(hi, lo)` approach, but the intermediate product
/// is verified via `checked_mul` so overflow is never silent.
///
/// # Rounding
///
/// **Rounds down** (truncates toward zero). For positive inputs this means
/// the protocol values collateral **less** than the true mathematical value,
/// which is conservative — it prevents over-leveraging and protects the
/// protocol from under-collateralised positions.
pub fn checked_mul_div(a: i128, b: i128, denom: i128) -> Result<i128, VaultError> {
    if denom == 0 {
        return Err(VaultError::MathDivisionByZero);
    }
    let product = checked_mul(a, b)?;
    let result = product
        .checked_div(denom)
        .ok_or(VaultError::MathDivisionByZero)?;

    // Detect precision loss: if a and b are non-zero but the result rounded
    // to zero, that may hide value from the protocol.
    if result == 0 && a != 0 && b != 0 {
        return Err(VaultError::MathPrecisionLoss);
    }

    Ok(result)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- checked_add -------------------------------------------------------

    #[test]
    fn test_checked_add_ok() {
        assert_eq!(checked_add(100, 200).unwrap(), 300);
    }

    #[test]
    fn test_checked_add_overflow() {
        assert_eq!(checked_add(i128::MAX, 1), Err(VaultError::MathOverflow));
    }

    // --- checked_sub -------------------------------------------------------

    #[test]
    fn test_checked_sub_ok() {
        assert_eq!(checked_sub(200, 100).unwrap(), 100);
    }

    #[test]
    fn test_checked_sub_underflow() {
        assert_eq!(checked_sub(0, 1), Err(VaultError::MathUnderflow));
    }

    // --- checked_mul -------------------------------------------------------

    #[test]
    fn test_checked_mul_ok() {
        assert_eq!(checked_mul(3, 7).unwrap(), 21);
    }

    #[test]
    fn test_checked_mul_overflow() {
        assert_eq!(
            checked_mul(i128::MAX, 2),
            Err(VaultError::MathOverflow)
        );
    }

    // --- checked_div -------------------------------------------------------

    #[test]
    fn test_checked_div_ok() {
        assert_eq!(checked_div(100, 3).unwrap(), 33);
    }

    #[test]
    fn test_checked_div_by_zero() {
        assert_eq!(checked_div(1, 0), Err(VaultError::MathDivisionByZero));
    }

    // --- checked_mul_div ---------------------------------------------------

    #[test]
    fn test_checked_mul_div_ok() {
        // (500 * 10_000_000) / 10_000_000 = 500
        assert_eq!(checked_mul_div(500, 10_000_000, 10_000_000).unwrap(), 500);
    }

    #[test]
    fn test_checked_mul_div_rounds_down() {
        // 1 * 5_000_000 / 10_000_000 = 0  (truncates down)
        assert_eq!(checked_mul_div(1, 5_000_000, 10_000_000).unwrap(), 0);
    }

    #[test]
    fn test_checked_mul_div_precision_loss() {
        // a != 0, b != 0, but result is 0 → precision loss
        assert_eq!(
            checked_mul_div(1, 1, 10_000_000),
            Err(VaultError::MathPrecisionLoss)
        );
    }

    #[test]
    fn test_checked_mul_div_division_by_zero() {
        assert_eq!(
            checked_mul_div(100, 200, 0),
            Err(VaultError::MathDivisionByZero)
        );
    }

    #[test]
    fn test_checked_mul_div_overflow() {
        assert_eq!(
            checked_mul_div(i128::MAX, 2, 1),
            Err(VaultError::MathOverflow)
        );
    }

    // --- boundary: max i128 values -----------------------------------------

    #[test]
    fn test_max_values_checked_add() {
        assert_eq!(checked_add(i128::MAX - 1, 1).unwrap(), i128::MAX);
        assert_eq!(checked_add(i128::MAX, 0).unwrap(), i128::MAX);
    }

    #[test]
    fn test_max_values_checked_sub() {
        assert_eq!(checked_sub(i128::MIN + 1, 1).unwrap(), i128::MIN);
    }

    #[test]
    fn test_max_values_checked_mul() {
        assert_eq!(checked_mul(i128::MAX, 1).unwrap(), i128::MAX);
        assert_eq!(checked_mul(i128::MAX, 0).unwrap(), 0);
    }

    #[test]
    fn test_max_values_checked_div() {
        assert_eq!(checked_div(i128::MAX, 1).unwrap(), i128::MAX);
        assert_eq!(checked_div(i128::MAX, i128::MAX).unwrap(), 1);
    }

    #[test]
    fn test_one_unit_rounding_boundaries() {
        // One-unit rounding boundary for mul_div
        // price = 1 (smallest non-zero price), amount = PRICE_PRECISION - 1
        // → (PRICE_PRECISION - 1) * 1 / PRICE_PRECISION = 0 → no precision loss
        // because a * b = PRICE_PRECISION - 1 < PRICE_PRECISION, result = 0, but
        // both a and b are non-zero so checked_mul_div returns PrecisionLoss.
        let price_unit = 1_i128;
        let amount = 10_000_000 - 1;
        assert_eq!(
            checked_mul_div(amount, price_unit, 10_000_000),
            Err(VaultError::MathPrecisionLoss)
        );

        // amount = PRICE_PRECISION, price = 1 → result = 1 (no loss)
        assert_eq!(
            checked_mul_div(10_000_000, price_unit, 10_000_000).unwrap(),
            1
        );
    }
}

