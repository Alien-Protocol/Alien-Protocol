use crate::constant::{SECONDS_PER_YEAR, V1_BORROW_APR_BPS};
use crate::errors::SharedError;
use crate::types::{
    accrue_linear_interest, borrow_limit_from_collateral, ceil_div, health_factor_bps, Debt,
};

/// Health factor at or above this value is considered healthy (1.0 = 10_000 bps).
const HEALTHY_HF_BPS: i128 = 10_000;

#[test]
fn test_accrue_linear_interest_one_year_eight_percent() {
    assert_eq!(
        accrue_linear_interest(1_000_000_000, V1_BORROW_APR_BPS, SECONDS_PER_YEAR).unwrap(),
        80_000_000
    );
}

#[test]
fn test_accrue_linear_interest_zero_principal_is_zero() {
    assert_eq!(
        accrue_linear_interest(0, V1_BORROW_APR_BPS, SECONDS_PER_YEAR).unwrap(),
        0
    );
}

#[test]
fn test_accrue_linear_interest_zero_elapsed_is_zero() {
    assert_eq!(
        accrue_linear_interest(1_000_000_000, V1_BORROW_APR_BPS, 0).unwrap(),
        0
    );
}

#[test]
fn test_accrue_linear_interest_overflow_is_error() {
    assert_eq!(
        accrue_linear_interest(i128::MAX, V1_BORROW_APR_BPS, SECONDS_PER_YEAR),
        Err(SharedError::Overflow)
    );
}

#[test]
fn test_accrue_linear_interest_rejects_negative_principal() {
    assert_eq!(
        accrue_linear_interest(-1, 800, 31_536_000),
        Err(SharedError::InvalidAmount)
    );
}

#[test]
fn test_debt_total_is_principal_plus_accrued() {
    let debt = Debt {
        principal: 1_000_000_000,
        accrued_interest: 80_000_000,
        interest_rate_bps: 800,
        last_accrual_at: 0,
    };
    assert_eq!(debt.total().unwrap(), 1_080_000_000);
}

#[test]
fn test_health_factor_bps_example_from_arch() {
    assert_eq!(health_factor_bps(10_000, 7_500, 8_000).unwrap(), 10_666);
}

/// Zero debt is always healthy and reported as the maximum representable value.
#[test]
fn test_health_factor_zero_debt_is_max() {
    assert_eq!(health_factor_bps(10_000, 0, 8_000).unwrap(), i128::MAX);
}

/// HF exactly at the 1.0 boundary must report precisely HEALTHY_HF_BPS.
#[test]
fn test_health_factor_at_exactly_one_boundary() {
    // collateral * liquidation_threshold_bps / 10_000 == debt
    // 10_000 * 8_000 / 10_000 = 8_000 == debt
    assert_eq!(
        health_factor_bps(10_000, 8_000, 8_000).unwrap(),
        HEALTHY_HF_BPS
    );
}

/// A clearly-liquidatable position (HF < 1.0).
#[test]
fn test_health_factor_below_one_is_liquidatable() {
    // 10_000 * 8_000 / 10_000 = 8_000 basis points < HEALTHY_HF_BPS
    let hf = health_factor_bps(10_000, 10_000, 8_000).unwrap();
    assert!(hf < HEALTHY_HF_BPS);
    assert_eq!(hf, 8_000);
}

/// A clearly-healthy position (HF > 1.0).
#[test]
fn test_health_factor_above_one_is_healthy() {
    // 10_000 * 8_000 / 4_000 = 20_000 basis points > HEALTHY_HF_BPS
    let hf = health_factor_bps(10_000, 4_000, 8_000).unwrap();
    assert!(hf > HEALTHY_HF_BPS);
    assert_eq!(hf, 20_000);
}

/// A position just below 1.0 must remain below HEALTHY_HF_BPS (no rounding up
/// past the boundary, which would hide liquidatability).
#[test]
fn test_health_factor_rounding_at_boundary() {
    // collateral * threshold / debt, scaled to bps:
    // 9_999 * 8_000 / 8_000 = 9_999 basis points
    let hf = health_factor_bps(9_999, 8_000, 8_000).unwrap();
    assert_eq!(hf, 9_999);
    assert!(hf < HEALTHY_HF_BPS);
}

#[test]
fn test_health_factor_rejects_negative_debt() {
    assert_eq!(
        health_factor_bps(10_000, -1, 8_000),
        Err(SharedError::InvalidAmount)
    );
}

#[test]
fn test_health_factor_rejects_negative_collateral() {
    assert_eq!(
        health_factor_bps(-1, 7_500, 8_000),
        Err(SharedError::InvalidAmount)
    );
}

#[test]
fn test_borrow_limit_seventy_percent() {
    assert_eq!(
        borrow_limit_from_collateral(10_000_000, 7_000).unwrap(),
        7_000_000
    );
}

#[test]
fn test_ceil_div_rounds_up() {
    assert_eq!(ceil_div(10, 3).unwrap(), 4);
    assert_eq!(ceil_div(9, 3).unwrap(), 3);
    assert_eq!(ceil_div(1, 3).unwrap(), 1);
    assert_eq!(ceil_div(0, 3).unwrap(), 0);
    assert_eq!(ceil_div(5, -2).unwrap(), -2);
}

#[test]
fn test_ceil_div_rejects_zero_denominator() {
    assert_eq!(ceil_div(1, 0), Err(SharedError::DivisionByZero));
}

/// `ceil_div` is a mathematical ceiling for every sign combination: when the
/// division is not exact, the quotient rounds up toward `+∞`. Negative
/// numerators are therefore not rejected — they round toward `+∞` like any
/// other case (for example `ceil(-10 / 3) == -3`).
#[test]
fn test_ceil_div_negative_numerator_rounds_toward_positive_infinity() {
    // ceil(-10 / 3) = ceil(-3.33) = -3
    assert_eq!(ceil_div(-10, 3).unwrap(), -3);
    // ceil(-7 / 2) = ceil(-3.5) = -3
    assert_eq!(ceil_div(-7, 2).unwrap(), -3);
    // ceil(-1 / 3) = ceil(-0.33) = 0
    assert_eq!(ceil_div(-1, 3).unwrap(), 0);
}

/// Exact division with a negative numerator is returned unchanged.
#[test]
fn test_ceil_div_negative_numerator_exact_division() {
    assert_eq!(ceil_div(-9, 3).unwrap(), -3);
    assert_eq!(ceil_div(-6, -3).unwrap(), 2);
}

/// A negative numerator and a negative denominator also round up toward `+∞`.
#[test]
fn test_ceil_div_negative_numerator_and_denominator_rounds_up() {
    // ceil(-10 / -3) = ceil(3.33) = 4
    assert_eq!(ceil_div(-10, -3).unwrap(), 4);
    // ceil(-1 / -2) = ceil(0.5) = 1
    assert_eq!(ceil_div(-1, -2).unwrap(), 1);
}
