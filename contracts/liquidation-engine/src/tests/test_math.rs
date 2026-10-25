#[cfg(test)]
mod tests {
    use crate::errors::EngineError;
    use crate::liquidate::calculate_bonus;
    use soroban_sdk::Env;

    #[test]
    fn test_calculate_bonus_eight_percent() {
        // repaid=1_000_000_000 => bonus = ceil(1_000_000_000 * 800 / 10_000) = 80_000_000
        let env = Env::default();
        let result = calculate_bonus(env, 1_000_000_000i128);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 80_000_000);
    }

    #[test]
    fn test_calculate_bonus_rejects_non_positive() {
        let env = Env::default();
        assert_eq!(
            calculate_bonus(env.clone(), 0),
            Err(EngineError::InvalidAmount)
        );
        assert_eq!(calculate_bonus(env, -100), Err(EngineError::InvalidAmount));
    }

    #[test]
    fn test_calculate_bonus_small_amount() {
        // ceil_div(1 * 800, 10_000) = ceil(0.08) = 1
        let env = Env::default();
        assert_eq!(calculate_bonus(env, 1).unwrap(), 1);
    }

    #[test]
    fn test_calculate_bonus_large_amount() {
        // 10_000_000_000 * 800 / 10_000 = 800_000_000 (exact)
        let env = Env::default();
        assert_eq!(calculate_bonus(env, 10_000_000_000).unwrap(), 800_000_000);
    }

    #[test]
    fn test_calculate_bonus_overflow_protection() {
        let env = Env::default();
        assert_eq!(calculate_bonus(env, i128::MAX), Err(EngineError::Overflow));
    }

    #[test]
    fn test_calculate_bonus_rounding() {
        let env = Env::default();
        // 100 * 800 / 10_000 = 8 (exact)
        assert_eq!(calculate_bonus(env.clone(), 100).unwrap(), 8);
        // 101 * 800 = 80_800; ceil(80_800 / 10_000) = ceil(8.08) = 9
        assert_eq!(calculate_bonus(env, 101).unwrap(), 9);
    }

    #[test]
    fn test_calculate_bonus_various_amounts() {
        let env = Env::default();
        let cases: &[(i128, i128)] = &[
            (10, 1),       // ceil(8_000 / 10_000) = 1
            (125, 10),     // 100_000 / 10_000 = 10, exact
            (1250, 100),   // 1_000_000 / 10_000 = 100, exact
            (12500, 1000), // 10_000_000 / 10_000 = 1_000, exact
        ];

        for &(repaid, expected) in cases {
            let result = calculate_bonus(env.clone(), repaid);
            assert!(result.is_ok(), "Failed for repaid={repaid}");

            let bonus = result.unwrap();
            assert_eq!(
                bonus, expected,
                "Mismatch for repaid={repaid}: expected {expected}, got {bonus}"
            );
        }
    }

    // -----------------------------------------------------------------------
    // calculate_partial_repayment — pure arithmetic fixtures
    // -----------------------------------------------------------------------

    #[test]
    fn test_partial_repay_math_architecture_fixture() {
        // Collateral=10_000, Debt=8_100, LT=8_000 bps, TARGET=11_000, BONUS=800
        let collateral_value = 10_000i128;
        let debt = 8_100i128;
        let lt_bps = 8_000i128;
        let target_hf_bps = 11_000i128;
        let bonus_bps = 800i128;

        // numerator = 11_000*8_100 - 10_000*8_000 = 89_100_000 - 80_000_000 = 9_100_000
        let numerator = target_hf_bps * debt - collateral_value * lt_bps;
        assert_eq!(numerator, 9_100_000);

        // denominator_sub = (10_000+800)*8_000 / 10_000 = 86_400_000/10_000 = 8_640 (exact)
        let denominator_sub = (10_000 + bonus_bps) * lt_bps / 10_000;
        let denominator = target_hf_bps - denominator_sub;
        assert_eq!(denominator, 2_360);

        // R = ceil(9_100_000 / 2_360) = ceil(3855.93) = 3_856
        let uncapped_r = (numerator + denominator - 1) / denominator;
        assert_eq!(uncapped_r, 3_856);

        // close_factor = 8_100*5_000/10_000 = 4_050
        let close_factor = debt * 5_000 / 10_000;
        assert_eq!(close_factor, 4_050);

        // 3_856 < 4_050 → repay = 3_856
        assert_eq!(uncapped_r.min(close_factor).min(debt), 3_856);
    }

    #[test]
    fn test_partial_repay_bonus_calculation_in_fixture() {
        let repay = 3_856i128;
        // seized_value = floor(3_856 * 10_800 / 10_000) = floor(4164.48) = 4_164
        assert_eq!(repay * (10_000 + 800) / 10_000, 4_164);
        // bonus = ceil(3_856 * 800 / 10_000) = ceil(308.48) = 309
        assert_eq!((repay * 800 + 10_000 - 1) / 10_000, 309);
    }

    #[test]
    fn test_health_factor_calculation_fixture() {
        let collateral_value = 10_000i128;
        let debt = 8_100i128;
        let lt_bps = 8_000i128;

        // HF = floor(10_000 * 8_000 / 8_100) = floor(9876.54) = 9_876
        assert_eq!((collateral_value * lt_bps) / debt, 9876);

        // After repay=3_856, seize=4_164:
        let remaining_collateral = collateral_value - 4_164; // 5_836
        let remaining_debt = debt - 3_856; // 4_244
        assert_eq!(remaining_collateral, 5_836);
        assert_eq!(remaining_debt, 4_244);

        // post-HF = floor(5_836 * 8_000 / 4_244) = floor(10998.6) = 11_000 (int div)
        let post_hf = (remaining_collateral * lt_bps) / remaining_debt;
        assert_eq!(post_hf, 11_000);
    }

    // -----------------------------------------------------------------------
    // Regression tests: denominator_sub uses ceil_div so that repayment is
    // sufficient to reach TARGET_HF_BPS after calculate_bonus rounds up.
    // -----------------------------------------------------------------------

    #[test]
    fn test_partial_repayment_non_exact_fixture_reaches_target_hf() {
        // Fixture: Collateral=10_000, Debt=8_000, LT=7_777 bps, TARGET=11_000, BONUS=800
        // Current HF = floor(10_000 * 7_777 / 8_000) = 9_721 (liquidatable)
        let collateral = 10_000i128;
        let debt = 8_000i128;
        let lt_bps = 7_777i128;
        let target = shared::TARGET_HF_BPS; // 11_000
        let bps = shared::BPS_DENOMINATOR; // 10_000
        let bonus_bps = shared::LIQUIDATION_BONUS_BPS as i128; // 800

        let numerator = target * debt - collateral * lt_bps; // 10_230_000
        assert_eq!(numerator, 10_230_000);

        // denominator_sub uses ceil_div: ceil(10_800 * 7_777 / 10_000) = ceil(8_399.16) = 8_400
        let temp = (bps + bonus_bps) * lt_bps;
        let denominator_sub = shared::ceil_div(temp, bps).unwrap();
        assert_eq!(denominator_sub, 8_400);

        let denominator = target - denominator_sub;
        assert_eq!(denominator, 2_600);

        // R = ceil(10_230_000 / 2_600) = ceil(3934.61) = 3_935
        let repay = shared::ceil_div(numerator, denominator).unwrap();
        assert_eq!(repay, 3_935);

        // calculate_bonus rounds up: ceil(3_935 * 800 / 10_000) = ceil(314.8) = 315
        let env = Env::default();
        let bonus = calculate_bonus(env, repay).unwrap();
        assert_eq!(bonus, 315);

        let seized = repay + bonus;
        assert_eq!(seized, 4_250);

        let remaining_collateral = collateral - seized;
        assert_eq!(remaining_collateral, 5_750);

        let remaining_debt = debt - repay;
        assert_eq!(remaining_debt, 4_065);

        // post-HF = 5_750 * 7_777 / 4_065 = 11_000 >= TARGET_HF_BPS
        let post_hf = remaining_collateral * lt_bps / remaining_debt;
        assert!(
            post_hf >= target,
            "post-HF {post_hf} must reach TARGET_HF_BPS {target}"
        );
        assert_eq!(post_hf, 11_000);
    }
}
