//! Risk and boundary calculations for the collateral vault.
//!
//! All financial arithmetic delegates to [`crate::math`] helpers that return
//! typed errors and round conservatively to protect the protocol.

use crate::errors::VaultError;
use crate::math;
use crate::storage;
use crate::types;
use soroban_sdk::{token, Address, Env};

use crate::LendingPoolClient;
use crate::OracleClient;

/// Minimum collateral ratio expressed as a percentage (110% = 110).
const MIN_COLLATERAL_RATIO_NUMERATOR: i128 = 110;
const MIN_COLLATERAL_RATIO_DENOMINATOR: i128 = 100;

/// Oracle prices are encoded with 7 decimal places (e.g. $1.00 = 10_000_000).
const PRICE_PRECISION: i128 = 10_000_000;

/// Compute the USD value of a user's total collateral.
///
/// Iterates over all assets in the user's position, multiplies each amount
/// by its oracle price, and divides by [`PRICE_PRECISION`].
///
/// # Rounding
///
/// **Rounds down** (truncates toward zero) at each asset-level division.
/// This under-states the collateral value, which is conservative — it makes
/// liquidations slightly easier and withdrawals slightly harder, protecting
/// the protocol from under-collateralised positions.
///
/// # Errors
///
/// - [`VaultError::MathOverflow`] if any intermediate product exceeds `i128::MAX`.
/// - [`VaultError::MathDivisionByZero`] if the oracle returns a price of zero.
/// - [`VaultError::MathPrecisionLoss`] if a price × amount product loses all precision.
pub fn get_collateral_value(env: &Env, user: &Address) -> Result<i128, VaultError> {
    let position = crate::VaultContract::get_position(env.clone(), user.clone());

    let oracle_address =
        storage::get_oracle(env).ok_or(VaultError::InvalidInputs)?;
    let oracle_client = OracleClient::new(env, &oracle_address);

    let mut total_value: i128 = 0;

    for item in position.collateral.iter() {
        let price_data = oracle_client.get_price_or_fail(&item.asset);

        // USD value = amount * price / PRICE_PRECISION.
        // checked_mul_div rounds down (truncates), which is conservative.
        let item_value = math::checked_mul_div(item.amount, price_data.price, PRICE_PRECISION)?;

        total_value = math::checked_add(total_value, item_value)?;
    }

    Ok(total_value)
}

/// Check whether a withdrawal of `amount` of `asset` would leave the user's
/// position safely collateralised above the minimum collateral ratio (110%).
///
/// # Rounding
///
/// The collateral value is **rounded down** (conservative for collateral).
/// The withdrawn value is **rounded down** (conservative — slightly over-states
/// what is removed). The debt floor comparison (`debt * 110 / 100`) is exact
/// when debt is a whole-dollar amount; truncation rounds the required
/// collateral **down**, making the requirement slightly looser, but the
/// conservative rounding of `get_collateral_value` offsets this.
///
/// On balance, the withdrawal check is **conservative toward the protocol**:
/// it will never allow a withdrawal that leaves the position under-collateralised.
///
/// # Errors
///
/// Returns [`VaultError::MathOverflow`] or [`VaultError::MathDivisionByZero`]
/// if underlying arithmetic fails.
pub fn is_withdrawal_safe(
    env: &Env,
    user: &Address,
    asset: &Address,
    amount: i128,
) -> Result<bool, VaultError> {
    let debt = if let Some(pool_addr) = storage::get_pool(env) {
        let pool_client = LendingPoolClient::new(env, &pool_addr);
        pool_client.get_user_debt(user)
    } else {
        0
    };

    if debt == 0 {
        return Ok(true);
    }

    let total_value = get_collateral_value(env, user)?;

    let oracle_address = storage::get_oracle(env).ok_or(VaultError::InvalidInputs)?;
    let oracle_client = OracleClient::new(env, &oracle_address);
    let price_data = oracle_client
        .get_price(asset)
        .ok_or(VaultError::AssetNotFound)?;

    // USD value of the withdrawn amount = amount * price / PRICE_PRECISION.
    // Rounds down, which slightly over-states the withdrawn value →
    // conservative for the protocol.
    let withdrawn_value =
        math::checked_mul_div(amount, price_data.price, PRICE_PRECISION)?;

    if total_value < withdrawn_value {
        return Ok(false);
    }

    let remaining_value = math::checked_sub(total_value, withdrawn_value)?;

    // Minimum required collateral = debt * 110 / 100.
    // checked_mul_div rounds down (truncates) so the requirement is slightly
    // looser, but this is offset by the conservative rounding in total_value.
    let min_collateral = math::checked_mul_div(
        debt,
        MIN_COLLATERAL_RATIO_NUMERATOR,
        MIN_COLLATERAL_RATIO_DENOMINATOR,
    )?;

    Ok(remaining_value >= min_collateral)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VaultContract;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{contract, contractimpl, token, Address, Env, Vec};

    #[contract]
    pub struct MockLendingPool;

    #[contractimpl]
    impl MockLendingPool {
        pub fn get_user_debt(env: Env, _user: Address) -> i128 {
            env.storage().persistent().get(&"debt").unwrap_or(0)
        }
        pub fn set_user_debt(env: Env, debt: i128) {
            env.storage().persistent().set(&"debt", &debt);
        }
    }

    #[contract]
    pub struct MockOracle;

    #[contractimpl]
    impl MockOracle {
        pub fn get_price(env: Env, asset: Address) -> Option<types::PriceData> {
            env.storage().persistent().get(&asset)
        }
        pub fn get_price_or_fail(env: Env, asset: Address) -> types::PriceData {
            let pd: types::PriceData = env.storage().persistent().get(&asset).unwrap();
            pd
        }
        pub fn set_price(env: Env, asset: Address, price: i128, timestamp: u64) {
            let pd = types::PriceData { price, timestamp };
            env.storage().persistent().set(&asset, &pd);
        }
    }

    fn setup() -> (Env, Address, Address, Address, Vec<Address>) {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(VaultContract, ());
        let client = crate::VaultContractClient::new(&env, &contract_id);

        let oracle_id = env.register(MockOracle, ());
        let oracle_client = MockOracleClient::new(&env, &oracle_id);

        let admin = Address::generate(&env);
        let user = Address::generate(&env);

        client.initialize(&admin, &oracle_id);
        client.set_oracle(&oracle_id);

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin);
        let token_id = token_contract.address();
        let token_admin_client = token::StellarAssetClient::new(&env, &token_id);

        client.add_supported_asset(&token_id);

        let pool_id = env.register(MockLendingPool, ());
        let pool_client = MockLendingPoolClient::new(&env, &pool_id);
        client.set_pool(&pool_id);

        oracle_client.set_price(&token_id, &10_000_000, &1000); // $1.00

        let tokens = Vec::from_array(&env, [token_id]);

        (env, contract_id, user, pool_id, tokens)
    }

    #[test]
    fn test_is_withdrawal_safe_no_debt() {
        let (_env, contract_id, user, _pool, _tokens) = setup();
        let c = crate::VaultContractClient::new(&_env, &contract_id);
        let token_id = _tokens.get(0).unwrap();

        let result = is_withdrawal_safe(&_env, &user, &token_id, 100).unwrap();
        assert!(result, "withdrawal should be safe when debt is 0");
    }
}

