extern crate alloc;
use alloc::vec;
use alloc::vec::Vec as RustVec;

use crate::config::{ ledger_timestamp_to_ms, SIGNER_THRESHOLD, TRUSTED_SIGNER_BYTES };
use crate::OracleError;
use soroban_sdk::{ Bytes, Env, Symbol, TryFromVal as _, Vec };

use redstone::{
    core::{ config::Config, processor::process_payload },
    soroban::{ SorobanCrypto, SorobanRedStoneConfig },
    FeedId,
    SignerAddress,
};

/// Internal payload processor - verifies RedStone payload signatures and returns verified prices
/// This is the security-sensitive core function that all oracle features call
pub fn get_prices_from_payload(
    env: &Env,
    feed_ids: Vec<Symbol>,
    payload: Bytes
) -> Result<(u64, Vec<i128>), OracleError> {
    // Convert compile-time signer bytes to SignerAddress
    let mut redstone_signers: RustVec<SignerAddress> = RustVec::new();
    for signer_bytes in TRUSTED_SIGNER_BYTES.iter() {
        redstone_signers.push(SignerAddress::from(signer_bytes.to_vec()));
    }

    // Convert feed IDs to RedStone FeedId format
    let mut redstone_feed_ids: RustVec<FeedId> = RustVec::new();
    for sym in feed_ids.iter() {
        let symbol_str = soroban_sdk::SymbolStr::try_from_val(env, &sym.to_symbol_val())
            .map_err(|_| OracleError::MalformedPayload)?;
        let rust_str: &str = symbol_str.as_ref();

        let mut feed_id_bytes = [0u8; 32];
        let bytes = rust_str.as_bytes();
        let len = bytes.len().min(32);
        feed_id_bytes[..len].copy_from_slice(&bytes[..len]);

        let feed_id = FeedId::from(feed_id_bytes.to_vec());
        redstone_feed_ids.push(feed_id);
    }

    // Convert ledger timestamp to milliseconds
    let block_timestamp_ms = ledger_timestamp_to_ms(env.ledger().timestamp());

    // Create RedStone config with compile-time values
    let config = Config::try_new(
        SIGNER_THRESHOLD,
        redstone_signers,
        redstone_feed_ids,
        block_timestamp_ms.into(),
        None,
        None
    ).map_err(|_| OracleError::MalformedPayload)?;

    // Convert payload bytes
    let mut payload_buf = vec![0u8; payload.len() as usize];
    payload.copy_into_slice(&mut payload_buf);
    let redstone_payload = redstone::Bytes::from(payload_buf);

    // Process payload with signature verification
    let crypto = SorobanCrypto::new(env);
    let mut redstone_config = SorobanRedStoneConfig::from((config, crypto));

    let validated = process_payload(&mut redstone_config, redstone_payload).map_err(
        |_| OracleError::InvalidPayload
    )?;

    // Extract prices for requested feeds
    let mut prices = Vec::new(env);
    for sym in feed_ids.iter() {
        let symbol_str = soroban_sdk::SymbolStr::try_from_val(env, &sym.to_symbol_val())
            .map_err(|_| OracleError::MalformedPayload)?;
        let rust_str: &str = symbol_str.as_ref();

        let mut feed_id_bytes = [0u8; 32];
        let bytes = rust_str.as_bytes();
        let len = bytes.len().min(32);
        feed_id_bytes[..len].copy_from_slice(&bytes[..len]);

        let target_feed_id = FeedId::from(feed_id_bytes.to_vec());

        let mut found = false;
        for fv in validated.values.iter() {
            if fv.feed == target_feed_id {
                // RedStone returns prices as big-endian unsigned 256-bit integers
                // We need to convert to i128, ensuring the value fits
                let val_bytes = fv.value.as_be_bytes();

                // Check that the value fits in i128 (upper 16 bytes must be zero and sign bit not set)
                let fits = val_bytes[0..16].iter().all(|&b| b == 0) && val_bytes[16] < 128;
                if !fits {
                    return Err(OracleError::InvalidPayload);
                }

                // Extract lower 16 bytes as i128
                let mut buf = [0u8; 16];
                buf.copy_from_slice(&val_bytes[16..32]);
                let price = i128::from_be_bytes(buf);

                if price <= 0 {
                    return Err(OracleError::InvalidPayload);
                }

                prices.push_back(price);
                found = true;
                break;
            }
        }

        if !found {
            return Err(OracleError::UnknownFeed);
        }
    }

    // Return timestamp in milliseconds and prices
    Ok((validated.timestamp.as_millis(), prices))
}

pub fn get_prices(
    env: Env,
    feed_ids: Vec<Symbol>,
    payload: Bytes
) -> Result<(u64, Vec<i128>), OracleError> {
    get_prices_from_payload(&env, feed_ids, payload)
}
