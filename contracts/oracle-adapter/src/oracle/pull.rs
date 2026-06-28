extern crate alloc;
use alloc::vec;
use alloc::vec::Vec as RustVec;

use crate::oracle::storage;
use crate::OracleError;
use soroban_sdk::{Bytes, Env, Symbol, TryFromVal as _, Vec};

use redstone::{
    core::{config::Config, processor::process_payload},
    soroban::{SorobanCrypto, SorobanRedStoneConfig},
    FeedId, SignerAddress,
};

pub fn get_prices(
    env: Env,
    feed_ids: Vec<Symbol>,
    payload: Bytes,
) -> Result<(u64, Vec<i128>), OracleError> {
    if !storage::is_redstone_initialized(&env) {
        return Err(OracleError::NotInitialized);
    }

    let threshold = match storage::get_redstone_threshold(&env) {
        Some(t) => t as u8,
        None => return Err(OracleError::NotInitialized),
    };

    let stored_signers = match storage::get_redstone_signers(&env) {
        Some(s) => s,
        None => return Err(OracleError::NotInitialized),
    };

    if stored_signers.is_empty() {
        return Err(OracleError::NotInitialized);
    }

    let mut redstone_signers: RustVec<SignerAddress> = RustVec::new();
    for s in stored_signers.iter() {
        let mut buf = vec![0u8; s.len() as usize];
        s.copy_into_slice(&mut buf);
        redstone_signers.push(SignerAddress::from(buf));
    }

    let mut redstone_feed_ids: RustVec<FeedId> = RustVec::new();
    for sym in feed_ids.iter() {
        if sym != Symbol::new(&env, "XLM")
            && sym != Symbol::new(&env, "USDC")
            && sym != Symbol::new(&env, "BTC")
            && sym != Symbol::new(&env, "ETH")
        {
            return Err(OracleError::UnknownFeed);
        }

        let symbol_str = soroban_sdk::SymbolStr::try_from_val(&env, &sym.to_symbol_val()).unwrap();
        let rust_str: &str = symbol_str.as_ref();
        let feed_id = FeedId::from(rust_str.as_bytes().to_vec());
        redstone_feed_ids.push(feed_id);
    }

    let block_timestamp_ms = env.ledger().timestamp() * 1000;

    let config = match Config::try_new(
        threshold,
        redstone_signers,
        redstone_feed_ids,
        block_timestamp_ms.into(),
        None,
        None,
    ) {
        Ok(c) => c,
        Err(_) => return Err(OracleError::InvalidPayload),
    };

    let mut payload_buf = vec![0u8; payload.len() as usize];
    payload.copy_into_slice(&mut payload_buf);
    let redstone_payload = redstone::Bytes::from(payload_buf);

    let crypto = SorobanCrypto::new(&env);
    let mut redstone_config = SorobanRedStoneConfig::from((config, crypto));

    let validated = match process_payload(&mut redstone_config, redstone_payload) {
        Ok(v) => v,
        Err(_) => return Err(OracleError::InvalidPayload),
    };

    let mut prices = Vec::new(&env);
    for sym in feed_ids.iter() {
        let symbol_str = soroban_sdk::SymbolStr::try_from_val(&env, &sym.to_symbol_val()).unwrap();
        let rust_str: &str = symbol_str.as_ref();
        let target_feed_id = FeedId::from(rust_str.as_bytes().to_vec());

        let mut found = false;
        for fv in validated.values.iter() {
            if fv.feed == target_feed_id {
                let val_bytes = fv.value.as_be_bytes();
                let mut fits = true;
                for i in 0..16 {
                    if val_bytes[i] != 0 {
                        fits = false;
                    }
                }
                if val_bytes[16] >= 128 {
                    fits = false;
                }
                if !fits {
                    return Err(OracleError::InvalidPayload);
                }

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

    Ok((validated.timestamp.as_millis(), prices))
}
