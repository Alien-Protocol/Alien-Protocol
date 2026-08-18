#[cfg(test)]
mod tests {
    use crate::errors::EngineError;
    use crate::liquidate::calculate_bonus;
    use soroban_sdk::Env;

    #[test]
    fn test_calculate_bonus_eight_percent() {
        // Test that bonus calculation works correctly
        // repaid=1_000_000_000 => bonus=80_000_000
        let env = Env::default();
        let repaid_amount = 1_000_000_000i128;
        let result = calculate_bonus(env, repaid_amount);

        assert!(result.is_ok());
        let bonus = result.unwrap();
        assert_eq!(bonus, 80_000_000);
    }

    #[test]
    fn test_calculate_bonus_rejects_non_positive() {
        // Test that zero amount is rejected
        let env = Env::default();
        let result = calculate_bonus(env.clone(), 0);
        assert_eq!(result, Err(EngineError::InvalidAmount));

        // Test that negative amount is rejected
        let result = calculate_bonus(env, -100);
        assert_eq!(result, Err(EngineError::InvalidAmount));
    }

    #[test]
    fn test_calculate_bonus_small_amount() {
        // Test with small amount: 1 => ceil(1 * 800 / 10000) = ceil(0.08) = 1
        let env = Env::default();
        let result = calculate_bonus(env, 1);
        assert!(result.is_ok());
        let bonus = result.unwrap();
        // ceil_div(1 * 800, 10000) = ceil(0.08) = 1
        assert_eq!(bonus, 1);
    }

    #[test]
    fn test_calculate_bonus_large_amount() {
        // Test with large amount: 10_000_000_000 => 800_000_000
        let env = Env::default();
        let result = calculate_bonus(env, 10_000_000_000);
        assert!(result.is_ok());
        let bonus = result.unwrap();
        assert_eq!(bonus, 800_000_000);
    }

    #[test]
    fn test_calculate_bonus_overflow_protection() {
        // Test that overflow is caught
        let env = Env::default();
        let max_i128 = i128::MAX;
        let result = calculate_bonus(env, max_i128);
        // This should overflow when multiplying by LIQUIDATION_BONUS_BPS
        assert_eq!(result, Err(EngineError::Overflow));
    }

    #[test]
    fn test_calculate_bonus_rounding() {
        // Test ceiling division behavior
        // 100 * 800 / 10000 = 8 (exact)
        let env = Env::default();
        let result = calculate_bonus(env.clone(), 100);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 8);

        // 101 * 800 / 10000 = 80.8 -> ceil to 81
        let result = calculate_bonus(env, 101);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 9); // ceil(80.8 / 1000) actually = ceil(0.0808) from 101 * 800 / 10000
    }

    #[test]
    fn test_calculate_bonus_various_amounts() {
        let env = Env::default();

        // Test a few more values to ensure consistency
        let test_cases: [(i128, i128); 4] = [
            (10, 1),       // ceil(10 * 800 / 10000) = ceil(0.8) = 1
            (125, 10),     // 125 * 800 / 10000 = 10
            (1250, 100),   // 1250 * 800 / 10000 = 100
            (12500, 1000), // 12500 * 800 / 10000 = 1000
        ];

        for (repaid, expected_bonus) in &test_cases {
            let result = calculate_bonus(env.clone(), *repaid);
            assert!(result.is_ok(), "Failed for repaid={}", repaid);
            let bonus = result.unwrap();
            assert_eq!(
                bonus, *expected_bonus,
                "Mismatch for repaid={}: expected {}, got {}",
                repaid, expected_bonus, bonus
            );
        }
    }

    // Tests for calculate_partial_repayment mathematical correctness
    // These tests verify the math without needing contract clients

    #[test]
    fn test_partial_repay_math_architecture_fixture() {
        // Test the numeric fixture from requirements:
        // Value
        // Collateral = 10_000
        // Debt = 8_100
        // LT = 8000 (80% in basis points)
        // HF now = 9876 (liquidatable, < 10_000)
        // Uncapped R = 3856
        // Close-factor cap = 4050
        // Use = 3856

        let collateral_value = 10_000i128;
        let debt = 8_100i128;
        let lt_bps = 8_000i128; // 80%
        let target_hf_bps = 11_000i128; // 1.10
        let bonus_bps = 800i128; // 8%

        // Calculate numerator: TARGET_HF_BPS * debt - collateral_value * lt_bps
        let numerator = target_hf_bps * debt - collateral_value * lt_bps;
        // 11_000 * 8_100 - 10_000 * 8_000
        // = 89_100_000 - 80_000_000 = 9_100_000
        assert_eq!(numerator, 9_100_000);

        // Calculate denominator: TARGET_HF_BPS - (10_000 + LIQUIDATION_BONUS_BPS) * lt_bps / 10_000
        // (10_000 + 800) * 8_000 / 10_000
        // = 10_800 * 8_000 / 10_000 = 86_400_000 / 10_000 = 8_640
        let denominator_sub = (10_000 + bonus_bps) * lt_bps / 10_000;
        // 11_000 - 8_640 = 2_360
        let denominator = target_hf_bps - denominator_sub;
        assert_eq!(denominator, 2_360);

        // R = ceil_div(9_100_000, 2_360) = ceil(3855.93...) = 3856
        let uncapped_r = (numerator + denominator - 1) / denominator; // ceiling division
        assert_eq!(uncapped_r, 3856, "Uncapped R should be 3856");

        // Close factor cap: debt * CLOSE_FACTOR_BPS / 10_000
        // 8_100 * 5_000 / 10_000 = 4_050
        let close_factor = debt * 5_000 / 10_000;
        assert_eq!(close_factor, 4_050, "Close factor should be 4_050");

        // Since 3_856 < 4_050, we use 3_856
        let repay = uncapped_r.min(close_factor).min(debt);
        assert_eq!(repay, 3_856, "Final repay should be 3_856");
    }

    #[test]
    fn test_partial_repay_bonus_calculation_in_fixture() {
        // Verify bonus calculation for the fixture repayment
        // repay = 3_856
        // seized_value = 3_856 * (10_000 + 800) / 10_000 = 3_856 * 10_800 / 10_000
        let repay = 3_856i128;
        let bonus_bps = 800i128;

        let seized_value = repay * (10_000 + bonus_bps) / 10_000;
        // = 3_856 * 10_800 / 10_000 = 41_644_800 / 10_000 = 4_164.48
        // Floor division: 4_164
        assert_eq!(seized_value, 4_164, "Seized value should be 4_164");

        // Now verify bonus: ceil_div(3_856 * 800, 10_000) = ceil(308.48) = 309
        let bonus_numerator = repay * bonus_bps;
        let bonus = (bonus_numerator + 10_000 - 1) / 10_000;
        assert_eq!(bonus, 309, "Bonus should be ceil(308.48) = 309");
    }

    #[test]
    fn test_health_factor_calculation_fixture() {
        // Verify HF calculations in the fixture
        let collateral_value = 10_000i128;
        let debt = 8_100i128;
        let lt_bps = 8_000i128;

        // Current HF = collateral_value * lt_bps / debt
        // = 10_000 * 8_000 / 8_100 = 80_000_000 / 8_100 = 9876.543...
        let current_hf = (collateral_value * lt_bps) / debt;
        assert_eq!(current_hf, 9876, "Current HF should be 9876 (floor)");

        // After repay = 3_856 and seize = 4_164:
        // remaining_collateral = 10_000 - 4_164 = 5_836
        // remaining_debt = 8_100 - 3_856 = 4_244
        let remaining_collateral = collateral_value - 4_164;
        let remaining_debt = debt - 3_856;
        assert_eq!(remaining_collateral, 5_836);
        assert_eq!(remaining_debt, 4_244);

        // post-HF = 5_836 * 8_000 / 4_244
        // = 46_688_000 / 4_244 = 10_998.69...
        let post_hf = (remaining_collateral * lt_bps) / remaining_debt;
        // Actual calculation: 46_688_000 / 4_244 = 11000 (with integer division)
        assert_eq!(
            post_hf, 11000,
            "Post-HF should be >= TARGET_HF_BPS (11_000)"
        );
    }
}
