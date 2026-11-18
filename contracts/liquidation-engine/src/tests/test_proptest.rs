#![cfg(test)]

use crate::errors::EngineError;
use crate::liquidate::calculate_bonus;
use soroban_sdk::Env;

#[test]
fn prop_bonus_strictly_monotonic_in_repayment() {
    let env = Env::default();
    let amounts = [
        1i128,
        10,
        100,
        1_000,
        10_000,
        100_000,
        1_000_000,
        10_000_000,
        100_000_000,
        1_000_000_000,
        50_000_000_000,
    ];

    for i in 0..amounts.len() - 1 {
        let b1 = calculate_bonus(env.clone(), amounts[i]).unwrap();
        let b2 = calculate_bonus(env.clone(), amounts[i + 1]).unwrap();
        assert!(
            b1 <= b2,
            "Bonus must be monotonic: bonus({})={} > bonus({})={}",
            amounts[i],
            b1,
            amounts[i + 1],
            b2
        );
    }
}

#[test]
fn prop_bonus_within_expected_bounds() {
    let env = Env::default();
    let amounts = [
        1i128,
        2,
        3,
        7,
        13,
        100,
        101,
        999,
        1_000,
        10_000,
        123_456,
        1_000_000,
        99_999_999,
    ];

    for amount in amounts {
        let bonus = calculate_bonus(env.clone(), amount).unwrap();
        let floor_val = (amount * 800) / 10_000;
        let expected_ceil = if (amount * 800) % 10_000 == 0 {
            floor_val
        } else {
            floor_val + 1
        };

        assert_eq!(
            bonus, expected_ceil,
            "Bonus for {} must match exact ceil_div calculation",
            amount
        );
        assert!(
            bonus >= floor_val,
            "Bonus must never be less than floor calculation"
        );
    }
}

#[test]
fn prop_non_positive_repay_rejected() {
    let env = Env::default();
    let test_amounts = [0i128, -1, -100, -1_000_000, i128::MIN + 1, i128::MIN];

    for amount in test_amounts {
        let res = calculate_bonus(env.clone(), amount);
        assert_eq!(
            res,
            Err(EngineError::InvalidAmount),
            "Non-positive amount {} must return InvalidAmount",
            amount
        );
    }
}
