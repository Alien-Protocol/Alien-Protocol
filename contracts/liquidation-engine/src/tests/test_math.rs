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
        assert_eq!(
            calculate_bonus(env, -100),
            Err(EngineError::InvalidAmount)
        );
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
        assert_eq!(
            calculate_bonus(env, i128::MAX),
            Err(EngineError::Overflow)
        );
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
            assert_eq!(
                calculate_bonus(env.clone(), repaid).unwrap(),
                expected,
                "repaid={}",
                repaid
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
        let remaining_debt = debt - 3_856;                   // 4_244
        assert_eq!(remaining_collateral, 5_836);
        assert_eq!(remaining_debt, 4_244);

        // post-HF = floor(5_836 * 8_000 / 4_244) = floor(10998.6) = 11_000 (int div)
        let post_hf = (remaining_collateral * lt_bps) / remaining_debt;
        assert_eq!(post_hf, 11_000);
    }

    // -----------------------------------------------------------------------
    // Regression tests: denominator_sub must use floor, not ceil.
    //
    // The repayment formula solves for R in:
    //   (C - R*(1+b)) * lt / (D - R) = TARGET
    //
    // which gives:
    //   denominator = TARGET - floor((BPS_DENOM + BONUS) * lt / BPS_DENOM)
    //
    // Using ceil_div here overstates denominator_sub, shrinks the denominator,
    // and inflates R — more collateral is seized than needed to restore the
    // target health factor.
    // -----------------------------------------------------------------------

    /// denominator_sub computed with floor division (correct).
    fn denom_sub_floor(lt_bps: i128) -> i128 {
        (shared::BPS_DENOMINATOR + shared::LIQUIDATION_BONUS_BPS as i128) * lt_bps
            / shared::BPS_DENOMINATOR
    }

    /// denominator_sub computed with ceil division (old, buggy).
    fn denom_sub_ceil(lt_bps: i128) -> i128 {
        let temp =
            (shared::BPS_DENOMINATOR + shared::LIQUIDATION_BONUS_BPS as i128) * lt_bps;
        shared::ceil_div(temp, shared::BPS_DENOMINATOR).unwrap()
    }

    #[test]
    fn test_denominator_sub_exact_division_floor_eq_ceil() {
        // (10_800 * 8_000) % 10_000 == 0 → both round to 8_640
        let lt_bps = 8_000i128;
        assert_eq!(denom_sub_floor(lt_bps), denom_sub_ceil(lt_bps));
        assert_eq!(denom_sub_floor(lt_bps), 8_640);
    }

    #[test]
    fn test_denominator_sub_non_exact_floor_lt_ceil() {
        // (10_800 * 7_777) = 83_991_600; 83_991_600 / 10_000 = 8_399.16
        // floor = 8_399, ceil = 8_400
        let lt_bps = 7_777i128;
        assert_eq!(denom_sub_floor(lt_bps), 8_399);
        assert_eq!(denom_sub_ceil(lt_bps), 8_400);
        assert!(denom_sub_floor(lt_bps) < denom_sub_ceil(lt_bps));
    }

    #[test]
    fn test_denominator_sub_regression_ceil_inflates_repayment() {
        // Fixture: collateral=10_000, debt=8_000, lt_bps=7_777
        // HF_now = floor(10_000 * 7_777 / 8_000) = 9_721  (liquidatable)
        let collateral = 10_000i128;
        let debt = 8_000i128;
        let lt_bps = 7_777i128;
        let target = shared::TARGET_HF_BPS;  // 11_000
        let bps = shared::BPS_DENOMINATOR;   // 10_000

        // numerator = 11_000*8_000 - 10_000*7_777 = 88_000_000 - 77_770_000 = 10_230_000
        let numerator = target * debt - collateral * lt_bps;
        assert_eq!(numerator, 10_230_000);

        // Fixed (floor): denominator = 11_000 - 8_399 = 2_601
        // R = ceil(10_230_000 / 2_601) = ceil(3932.33) = 3_933
        let denom_fixed = target - denom_sub_floor(lt_bps);
        assert_eq!(denom_fixed, 2_601);
        let r_fixed = shared::ceil_div(numerator, denom_fixed).unwrap();
        assert_eq!(r_fixed, 3_933);

        // Buggy (ceil): denominator = 11_000 - 8_400 = 2_600
        // R = ceil(10_230_000 / 2_600) = ceil(3934.61) = 3_935
        let denom_buggy = target - denom_sub_ceil(lt_bps);
        assert_eq!(denom_buggy, 2_600);
        let r_buggy = shared::ceil_div(numerator, denom_buggy).unwrap();
        assert_eq!(r_buggy, 3_935);

        // Buggy path demands 2 more units of repayment than necessary.
        assert!(r_buggy > r_fixed, "ceil path must produce more repayment");

        // Verify the fixed R restores HF to at least HEALTHY_HF_BPS.
        let seized = r_fixed * (bps + 800) / bps;
        let post_hf = (collateral - seized) * lt_bps / (debt - r_fixed);
        assert!(
            post_hf >= shared::HEALTHY_HF_BPS,
            "post-HF {} < HEALTHY_HF_BPS after fixed repayment",
            post_hf
        );
    }

    #[test]
    fn test_denominator_sub_regression_multiple_lt_values() {
        // For lt_bps where (10_800 * lt_bps) is divisible by 10_000:
        //   floor == ceil (no bug manifestation).
        // For all others: ceil == floor + 1 (bug inflates R by one denominator unit).
        let cases: &[(i128, bool /* exact? */)] = &[
            (5_000, true),
            (6_250, true),
            (7_000, true),
            (7_500, true),
            (8_000, true),
            (8_500, true),
            (7_777, false),
            (6_543, false),
            (9_001, false),
        ];
        for &(lt_bps, exact) in cases {
            let f = denom_sub_floor(lt_bps);
            let c = denom_sub_ceil(lt_bps);
            if exact {
                assert_eq!(f, c, "lt_bps={}: expected floor==ceil", lt_bps);
            } else {
                assert_eq!(c, f + 1, "lt_bps={}: expected ceil==floor+1", lt_bps);
            }
        }
    }
}
