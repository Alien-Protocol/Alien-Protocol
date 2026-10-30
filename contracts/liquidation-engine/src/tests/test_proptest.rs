#![cfg(test)]

use crate::{liquidate, LiquidationContract, LiquidationContractClient};
use proptest::prelude::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env};

proptest! {
    #[test]
    fn prop_bonus_calculation_strictly_eight_percent(
        repay in 100i128..=1_000_000_000i128,
    ) {
        let env = Env::default();
        let bonus = liquidate::calculate_bonus(env, repay).unwrap();
        let expected = shared::ceil_div(repay * 800, 10_000).unwrap();
        prop_assert_eq!(bonus, expected);
        // Invariant: bonus is at least 8% (floor) and at most ceil(8%)
        prop_assert!(bonus >= repay * 800 / 10_000);
    }

    #[test]
    fn prop_liquidate_rejects_non_positive_max_repay(
        repay in any::<i128>().prop_filter("non-positive only", |x| *x <= 0),
    ) {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(LiquidationContract, ());
        let client = LiquidationContractClient::new(&env, &contract_id);

        let liquidator = Address::generate(&env);
        let user = Address::generate(&env);

        let res = client.try_liquidate(&liquidator, &user, &repay);
        prop_assert!(res.is_err());
    }
}
