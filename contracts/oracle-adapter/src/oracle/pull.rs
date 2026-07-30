extern crate alloc;

use crate::oracle::processor;
use crate::OracleError;
use soroban_sdk::{Bytes, Env, Symbol, Vec};

pub fn get_prices(
    env: Env,
    feed_ids: Vec<Symbol>,
    payload: Bytes,
) -> Result<(u64, Vec<i128>), OracleError> {
    let (timestamp, validated_prices) =
        processor::process_redstone_payload(&env, feed_ids, payload)?;

    let mut prices = Vec::new(&env);
    for price_bytes in validated_prices.iter() {
        prices.push_back(processor::parse_price_be256(&price_bytes)?);
    }

    Ok((timestamp, prices))
}
