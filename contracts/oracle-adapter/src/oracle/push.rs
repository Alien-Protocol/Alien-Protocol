extern crate alloc;

use crate::oracle::{processor, storage};
use crate::OracleError;
use soroban_sdk::{contractevent, Address, Bytes, Env, Symbol, Vec};

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct FeedPriceUpdated {
    pub feed_id: Symbol,
    pub price: i128,
    pub timestamp: u64,
}

pub fn write_prices(
    env: Env,
    caller: Address,
    feed_ids: Vec<Symbol>,
    payload: Bytes,
) -> Result<(), OracleError> {
    // Authenticate caller (same as set_price)
    let admin = match crate::storage::get_admin(&env) {
        Some(addr) => addr,
        None => return Err(OracleError::NotInitialized),
    };
    let is_admin = caller == admin;
    let is_authorized_feeder = crate::storage::is_authorized_feeder(&env, &caller);

    if is_admin || is_authorized_feeder {
        caller.require_auth();
    } else {
        return Err(OracleError::Unauthorized);
    }

    if crate::storage::is_paused(&env) {
        return Err(OracleError::OraclePaused);
    }

    let (timestamp, validated_prices) =
        processor::process_redstone_payload(&env, feed_ids.clone(), payload)?;
    let write_timestamp = env.ledger().timestamp();

    for (index, sym) in feed_ids.iter().enumerate() {
        let price_bytes = validated_prices
            .get(index.try_into().map_err(|_| OracleError::InvalidPayload)?)
            .ok_or(OracleError::InvalidPayload)?;

        let price = processor::parse_price_be256(&price_bytes)?;

        let existing = storage::get_feed_price(&env, &sym);
        if let Some(ref ext_data) = existing {
            if timestamp <= ext_data.timestamp {
                continue;
            }
        }

        let price_data = crate::types::PriceData {
            price,
            timestamp,
            write_timestamp,
        };
        storage::set_feed_price(&env, &sym, &price_data);

        FeedPriceUpdated {
            feed_id: sym.clone(),
            price,
            timestamp,
        }
        .publish(&env);
    }

    Ok(())
}

pub fn read_prices(
    env: Env,
    feed_ids: Vec<Symbol>,
) -> Result<Vec<crate::types::PriceData>, OracleError> {
    let mut result = Vec::new(&env);
    for sym in feed_ids.iter() {
        if sym != Symbol::new(&env, "XLM")
            && sym != Symbol::new(&env, "USDC")
            && sym != Symbol::new(&env, "BTC")
            && sym != Symbol::new(&env, "ETH")
        {
            return Err(OracleError::UnknownFeed);
        }

        match storage::get_feed_price(&env, &sym) {
            Some(price_data) => {
                result.push_back(price_data);
            }
            None => {
                return Err(OracleError::FeedNotWritten);
            }
        }
    }
    Ok(result)
}
