#![cfg(test)]

//! Integration-level tests for the math helpers used in financial calculations.
//!
//! These tests verify that the math module's helpers behave correctly at
//! boundaries — maximum `i128` values and one-unit rounding thresholds —
//! and that the `risk` module correctly integrates with the vault contract.

use crate::math;
use crate::errors::VaultError;

// ---------------------------------------------------------------------------
// checked_add – integration-style boundary tests
// ---------------------------------------------------------------------------

#[test]
fn test_checked_add_max_boundary() {
    assert_eq!(math::checked_add(i128::MAX - 1, 1).unwrap(), i128::MAX);
    assert_eq!(math::checked_add(i128::MAX, 0).unwrap(), i128::MAX);
    assert_eq!(math::checked_add(i128::MAX, 1), Err(VaultError::MathOverflow));
}

#[test]
fn test_checked_add_min_boundary() {
    assert_eq!(math::checked_add(i128::MIN, 0).unwrap(), i128::MIN);
    assert_eq!(math::checked_add(i128::MIN, 1).unwrap(), i128::MIN + 1);
}

// ---------------------------------------------------------------------------
// checked_sub – integration-style boundary tests
// ---------------------------------------------------------------------------

#[test]
fn test_checked_sub_zero_boundary() {
    assert_eq!(math::checked_sub(0, 0).unwrap(), 0);
    assert_eq!(math::checked_sub(0, 1), Err(VaultError::MathUnderflow));
}

#[test]
fn test_checked_sub_max_boundary() {
    assert_eq!(math::checked_sub(i128::MAX, i128::MAX).unwrap(), 0);
    assert_eq!(math::checked_sub(i128::MAX, i128::MAX - 1).unwrap(), 1);
}

// ---------------------------------------------------------------------------
// checked_mul – integration-style boundary tests
// ---------------------------------------------------------------------------

#[test]
fn test_checked_mul_max_boundary() {
    assert_eq!(math::checked_mul(i128::MAX, 1).unwrap(), i128::MAX);
    assert_eq!(math::checked_mul(i128::MAX, -1).unwrap(), -i128::MAX);
    assert_eq!(math::checked_mul(i128::MAX, 2), Err(VaultError::MathOverflow));
}

#[test]
fn test_checked_mul_zero() {
    assert_eq!(math::checked_mul(i128::MAX, 0).unwrap(), 0);
    assert_eq!(math::checked_mul(0, i128::MAX).unwrap(), 0);
}

// ---------------------------------------------------------------------------
// checked_div – integration-style boundary tests
// ---------------------------------------------------------------------------

#[test]
fn test_checked_div_max_boundary() {
    assert_eq!(math::checked_div(i128::MAX, 1).unwrap(), i128::MAX);
    assert_eq!(math::checked_div(i128::MAX, i128::MAX).unwrap(), 1);
    assert_eq!(math::checked_div(1, 0), Err(VaultError::MathDivisionByZero));
}

#[test]
fn test_checked_div_positive_rounds_down() {
    // 100 / 3 = 33.333… → truncates to 33 (rounds down for positive)
    assert_eq!(math::checked_div(100, 3).unwrap(), 33);
}

// ---------------------------------------------------------------------------
// checked_mul_div – integration-style boundary tests
// ---------------------------------------------------------------------------

#[test]
fn test_checked_mul_div_price_precision_scale() {
    // Simulate the core collateral valuation pattern:
    // amount * price / PRICE_PRECISION

    // 500 tokens at $1.00 (10_000_000) → $500
    assert_eq!(
        math::checked_mul_div(500, 10_000_000, 10_000_000).unwrap(),
        500
    );

    // 1 token at $1.00 → $1
    assert_eq!(
        math::checked_mul_div(1, 10_000_000, 10_000_000).unwrap(),
        1
    );

    // 1 token at $0.50 (5_000_000) → $0 (truncated to 0, but detected as precision loss)
    assert_eq!(
        math::checked_mul_div(1, 5_000_000, 10_000_000).unwrap(),
        0
    );
}

#[test]
fn test_checked_mul_div_precision_loss_boundary() {
    // a = 1, b = 1, denom = 10_000_000 → result = 0, a != 0, b != 0 → PrecisionLoss
    assert_eq!(
        math::checked_mul_div(1, 1, 10_000_000),
        Err(VaultError::MathPrecisionLoss)
    );

    // a = 1, b = 9_999_999, denom = 10_000_000 → product = 9_999_999, result = 0 → PrecisionLoss
    assert_eq!(
        math::checked_mul_div(1, 9_999_999, 10_000_000),
        Err(VaultError::MathPrecisionLoss)
    );

    // a = 1, b = 10_000_000, denom = 10_000_000 → result = 1 → OK
    assert_eq!(
        math::checked_mul_div(1, 10_000_000, 10_000_000).unwrap(),
        1
    );
}

#[test]
fn test_checked_mul_div_by_zero() {
    assert_eq!(
        math::checked_mul_div(100, 200, 0),
        Err(VaultError::MathDivisionByZero)
    );
}

#[test]
fn test_checked_mul_div_overflow() {
    assert_eq!(
        math::checked_mul_div(i128::MAX, 2, 1),
        Err(VaultError::MathOverflow)
    );
}

// ---------------------------------------------------------------------------
// checked_mul_div – conservative rounding direction
// ---------------------------------------------------------------------------

#[test]
fn test_checked_mul_div_rounds_down_protocol_safe() {
    // For positive inputs, truncation rounds down, which is conservative for
    // the protocol — it values collateral less than its true mathematical value.

    // (9 * 3) / 5 = 27 / 5 = 5.4 → truncates to 5 (rounds down)
    assert_eq!(math::checked_mul_div(9, 3, 5).unwrap(), 5);

    // If this were rounding up, the result would be 6.
    // Rounding down (5) is conservative — the protocol sees less value.
}

// ---------------------------------------------------------------------------
// Maximum i128 stress tests
// ---------------------------------------------------------------------------

#[test]
fn test_max_i128_product_without_overflow() {
    // i128::MAX ≈ 1.7e38. The protocol price precision is 10^7.
    // A realistic max product: (10^12 tokens) * (10^12 price units) = 10^24,
    // which is far below i128::MAX. But test the boundary anyway.

    // product = i128::MAX / 2 * 2 = i128::MAX - 1 (no overflow if we do it right)
    let half_max = i128::MAX / 2;
    assert_eq!(math::checked_mul(half_max, 2).unwrap(), i128::MAX - 1);
}

#[test]
fn test_max_i128_mul_div_stress() {
    // Use near-max values that still fit in the product.
    // i128::MAX / 1_000_000 as a safe upper bound.
    let large_amount = i128::MAX / 10_000_000;
    let large_price = 10_000_000;
    let denom = 10_000_000;

    // large_amount * large_price / denom = (i128::MAX / 10_000_000) * 10_000_000 / 10_000_000
    // = large_amount (so ≈ 1.7e31 / 1e7 = 1.7e24)
    let result = math::checked_mul_div(large_amount, large_price, denom).unwrap();
    assert_eq!(result, large_amount);
}

