use crate::SharedError;
use soroban_sdk::{Env, I256};

// Cancel the quote precision before multiplying so two i128 operands fit in I256.
fn scales(decimals: u32, price_decimals: u32) -> Result<(i128, i128), SharedError> {
    if !(1..=18).contains(&decimals) || !(1..=18).contains(&price_decimals) {
        return Err(SharedError::InvalidAmount);
    }
    let sum = decimals + price_decimals;
    Ok((
        10_i128.pow(7_u32.saturating_sub(sum)),
        10_i128.pow(sum.saturating_sub(7)),
    ))
}

pub fn quote_value(
    env: &Env,
    amount: i128,
    price: i128,
    decimals: u32,
    price_decimals: u32,
) -> Result<i128, SharedError> {
    if amount < 0 || price <= 0 {
        return Err(SharedError::InvalidAmount);
    }
    let (up, down) = scales(decimals, price_decimals)?;
    I256::from_i128(env, amount)
        .mul(&I256::from_i128(env, price))
        .div(&I256::from_i128(env, down))
        .to_i128()
        .and_then(|n| n.checked_mul(up))
        .ok_or(SharedError::Overflow)
}

pub fn tokens_for_quote(
    env: &Env,
    value: i128,
    price: i128,
    decimals: u32,
    price_decimals: u32,
) -> Result<i128, SharedError> {
    if value < 0 || price <= 0 {
        return Err(SharedError::InvalidAmount);
    }
    let (up, down) = scales(decimals, price_decimals)?;
    let n = I256::from_i128(env, value).mul(&I256::from_i128(env, down));
    let d = I256::from_i128(env, price).mul(&I256::from_i128(env, up));
    n.add(&d.sub(&I256::from_i32(env, 1)))
        .div(&d)
        .to_i128()
        .ok_or(SharedError::Overflow)
}
