use crate::errors::SharedError;
use crate::types::{
    accrue_linear_interest, borrow_limit_from_collateral, ceil_div, health_factor_bps, Debt,
};

#[test]
fn test_accrue_linear_interest_one_year_eight_percent() {
    assert_eq!(
        accrue_linear_interest(1_000_000_000, 800, 31_536_000).unwrap(),
        80_000_000
    );
}

#[test]
fn test_accrue_linear_interest_zero_elapsed_is_zero() {
    assert_eq!(accrue_linear_interest(1_000_000_000, 800, 0).unwrap(), 0);
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

#[test]
fn test_health_factor_zero_debt_is_max() {
    assert_eq!(health_factor_bps(10_000, 0, 8_000).unwrap(), i128::MAX);
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
