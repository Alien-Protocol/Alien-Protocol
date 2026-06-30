extern crate alloc;
use alloc::vec;
use alloc::vec::Vec as RustVec;

use crate::oracle::storage;
use crate::OracleError;
use soroban_sdk::{contractevent, Address, Bytes, Env, Symbol, TryFromVal as _, Vec};

use redstone::{
    core::{config::Config, processor::process_payload},
    soroban::{SorobanCrypto, SorobanRedStoneConfig},
    FeedId, SignerAddress,
};

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

    // Convert compile-time signer bytes to SignerAddress
    let mut redstone_signers: RustVec<SignerAddress> = RustVec::new();
    for signer_bytes in crate::config::TRUSTED_SIGNER_BYTES.iter() {
        redstone_signers.push(SignerAddress::from(signer_bytes.to_vec()));
    }

    let mut redstone_feed_ids: RustVec<FeedId> = RustVec::new();
    for sym in feed_ids.iter() {
        let symbol_str = soroban_sdk::SymbolStr::try_from_val(&env, &sym.to_symbol_val())
            .map_err(|_| OracleError::MalformedPayload)?;
        let rust_str: &str = symbol_str.as_ref();

        let mut feed_id_bytes = [0u8; 32];
        let bytes = rust_str.as_bytes();
        let len = bytes.len().min(32);
        feed_id_bytes[..len].copy_from_slice(&bytes[..len]);

        let feed_id = FeedId::from(feed_id_bytes.to_vec());
        redstone_feed_ids.push(feed_id);
    }

    let block_timestamp_ms = crate::config::ledger_timestamp_to_ms(env.ledger().timestamp());

    let config = Config::try_new(
        crate::config::SIGNER_THRESHOLD,
        redstone_signers,
        redstone_feed_ids,
        block_timestamp_ms.into(),
        None,
        None,
    )
    .map_err(|_| OracleError::MalformedPayload)?;

    let mut payload_buf = vec![0u8; payload.len() as usize];
    payload.copy_into_slice(&mut payload_buf);
    let redstone_payload = redstone::Bytes::from(payload_buf);

    let crypto = SorobanCrypto::new(&env);
    let mut redstone_config = SorobanRedStoneConfig::from((config, crypto));

    let validated = process_payload(&mut redstone_config, redstone_payload)
        .map_err(|_| OracleError::InvalidPayload)?;

    let new_timestamp = validated.timestamp.as_millis();
    let write_timestamp = env.ledger().timestamp();

    for sym in feed_ids.iter() {
        let symbol_str = soroban_sdk::SymbolStr::try_from_val(&env, &sym.to_symbol_val()).unwrap();
        let rust_str: &str = symbol_str.as_ref();

        let mut feed_id_bytes = [0u8; 32];
        let bytes = rust_str.as_bytes();
        let len = bytes.len().min(32);
        feed_id_bytes[..len].copy_from_slice(&bytes[..len]);

        let target_feed_id = FeedId::from(feed_id_bytes.to_vec());

        let mut found_fv = None;
        for fv in validated.values.iter() {
            if fv.feed == target_feed_id {
                found_fv = Some(fv);
                break;
            }
        }
        let fv = match found_fv {
            Some(v) => v,
            None => return Err(OracleError::UnknownFeed),
        };

        let existing = storage::get_feed_price(&env, &sym);
        if let Some(ref ext_data) = existing {
            // A second write with an older timestamp is silently rejected without overwriting existing data
            if new_timestamp <= ext_data.timestamp {
                continue;
            }
        }

        let val_bytes = fv.value.as_be_bytes();
        let fits = val_bytes[0..16].iter().all(|&b| b == 0) && val_bytes[16] < 128;
        if !fits {
            return Err(OracleError::InvalidPayload);
        }

        let mut buf = [0u8; 16];
        buf.copy_from_slice(&val_bytes[16..32]);
        let price = i128::from_be_bytes(buf);

        if price <= 0 {
            return Err(OracleError::InvalidPayload);
        }

        let price_data = crate::types::PriceData {
            price,
            timestamp: new_timestamp,
            write_timestamp,
        };
        storage::set_feed_price(&env, &sym, &price_data);

        FeedPriceUpdated {
            feed_id: sym.clone(),
            price,
            timestamp: new_timestamp,
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
