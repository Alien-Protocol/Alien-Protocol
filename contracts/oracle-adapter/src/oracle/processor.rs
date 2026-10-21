extern crate alloc;
use alloc::vec;
use alloc::vec::Vec as RustVec;

use super::storage;
use crate::OracleError;
use redstone::core::{config::Config, processor::process_payload};
use redstone::network::error::Error as RedstoneError;
use redstone::soroban::{SorobanCrypto, SorobanRedStoneConfig};
use redstone::{FeedId, SignerAddress};
use soroban_sdk::{Bytes, Env, Symbol, SymbolStr, TryFromVal as _, Vec};

const REDSTONE_MAX_TIMESTAMP_DELAY_MS: u64 = 15 * 60 * 1000; // 15 minutes
const REDSTONE_MAX_TIMESTAMP_AHEAD_MS: u64 = 3 * 60 * 1000; // 3 minutes

fn load_redstone_config(env: &Env) -> Result<(u8, RustVec<SignerAddress>), OracleError> {
    if !storage::is_redstone_initialized(env) {
        return Err(OracleError::NotInitialized);
    }

    let stored_signers = storage::get_redstone_signers(env).ok_or(OracleError::NotInitialized)?;
    let stored_threshold =
        storage::get_redstone_threshold(env).ok_or(OracleError::NotInitialized)?;

    if stored_threshold == 0
        || stored_threshold > stored_signers.len()
        || stored_threshold > (u8::MAX as u32)
    {
        return Err(OracleError::InvalidThreshold);
    }

    let threshold = stored_threshold as u8;

    let mut signers = RustVec::new();
    for s in stored_signers.iter() {
        let mut buf = vec![0u8; s.len() as usize];
        s.copy_into_slice(&mut buf);
        signers.push(SignerAddress::from(buf));
    }

    Ok((threshold, signers))
}

fn feed_id_from_symbol(env: &Env, symbol: &Symbol) -> Result<FeedId, OracleError> {
    let symbol_str = SymbolStr::try_from_val(env, &symbol.to_symbol_val())
        .map_err(|_| OracleError::UnknownFeed)?;
    let rust_str: &str = symbol_str.as_ref();

    match rust_str {
        "XLM" | "USDC" | "BTC" | "ETH" => Ok(FeedId::from(rust_str.as_bytes().to_vec())),
        _ => Err(OracleError::UnknownFeed),
    }
}

fn map_redstone_error(error: RedstoneError) -> OracleError {
    match error {
        RedstoneError::TimestampTooOld(..) | RedstoneError::TimestampTooFuture(..) => {
            OracleError::InvalidTimestamp
        }
        _ => OracleError::InvalidPayload,
    }
}

pub(crate) fn parse_price_be256(value: &Bytes) -> Result<i128, OracleError> {
    if value.len() != 32 {
        return Err(OracleError::InvalidPayload);
    }

    let mut bytes = [0u8; 32];
    value.copy_into_slice(&mut bytes);

    let leading_ok = bytes[0..16].iter().all(|&b| b == 0) && bytes[16] < 128;
    if !leading_ok {
        return Err(OracleError::InvalidPayload);
    }

    let mut price_bytes = [0u8; 16];
    price_bytes.copy_from_slice(&bytes[16..32]);
    let price = i128::from_be_bytes(price_bytes);

    if price <= 0 {
        return Err(OracleError::InvalidPayload);
    }

    Ok(price)
}

pub(crate) fn process_redstone_payload(
    env: &Env,
    feed_ids: Vec<Symbol>,
    payload: Bytes,
) -> Result<(u64, Vec<Bytes>), OracleError> {
    let mut requested_feed_ids: RustVec<FeedId> = RustVec::new();
    let mut unique_feed_ids: RustVec<FeedId> = RustVec::new();

    for symbol in feed_ids.iter() {
        let feed_id = feed_id_from_symbol(env, &symbol)?;
        requested_feed_ids.push(feed_id);
        if !unique_feed_ids.iter().any(|existing| existing == &feed_id) {
            unique_feed_ids.push(feed_id);
        }
    }

    let block_timestamp_ms = env
        .ledger()
        .timestamp()
        .checked_mul(1000)
        .ok_or(OracleError::InvalidTimestamp)?;

    let (threshold, signers) = load_redstone_config(env)?;

    let config = Config::try_new(
        threshold,
        signers,
        unique_feed_ids,
        block_timestamp_ms.into(),
        Some(REDSTONE_MAX_TIMESTAMP_DELAY_MS.into()),
        Some(REDSTONE_MAX_TIMESTAMP_AHEAD_MS.into()),
    )
    .map_err(map_redstone_error)?;

    let mut payload_buf = vec![0u8; payload.len() as usize];
    payload.copy_into_slice(&mut payload_buf);
    let redstone_payload = redstone::Bytes::from(payload_buf);
    let crypto = SorobanCrypto::new(env);
    let mut redstone_config = SorobanRedStoneConfig::from((config, crypto));

    let validated =
        process_payload(&mut redstone_config, redstone_payload).map_err(map_redstone_error)?;
    let timestamp_ms = validated.timestamp.as_millis();

    let mut prices = Vec::new(env);
    for feed_id in requested_feed_ids.iter() {
        let mut found = false;
        for feed_value in validated.values.iter() {
            if feed_value.feed == *feed_id {
                prices.push_back(Bytes::from_slice(env, feed_value.value.as_be_bytes()));
                found = true;
                break;
            }
        }
        if !found {
            return Err(OracleError::UnknownFeed);
        }
    }

    Ok((timestamp_ms, prices))
}
