extern crate alloc;
use alloc::vec;
use alloc::vec::Vec as RustVec;

use crate::OracleError;
use redstone::core::{config::Config, processor::process_payload};
use redstone::network::error::Error as RedstoneError;
use redstone::soroban::{SorobanCrypto, SorobanRedStoneConfig};
use redstone::{FeedId, SignerAddress};
use soroban_sdk::{Bytes, Env, Symbol, SymbolStr, TryFromVal as _, Vec};

const REDSTONE_SIGNER_THRESHOLD: u8 = 3;
const REDSTONE_MAX_TIMESTAMP_DELAY_MS: u64 = 15 * 60 * 1000; // 15 minutes
const REDSTONE_MAX_TIMESTAMP_AHEAD_MS: u64 = 3 * 60 * 1000; // 3 minutes

const REDSTONE_SIGNERS: [[u8; 20]; 5] = [
    [
        0x8b, 0xb8, 0xf3, 0x2d, 0xf0, 0x4c, 0x8b, 0x65, 0x49, 0x87, 0xda, 0xae, 0xd5, 0x3d, 0x6b,
        0x60, 0x91, 0xe3, 0xb7, 0x74,
    ],
    [
        0xde, 0xb2, 0x2f, 0x54, 0x73, 0x8d, 0x54, 0x97, 0x6c, 0x4c, 0x0f, 0xe5, 0xce, 0x6d, 0x40,
        0x8e, 0x40, 0xd8, 0x84, 0x99,
    ],
    [
        0x51, 0xce, 0x04, 0xbe, 0x4b, 0x3e, 0x32, 0x57, 0x2c, 0x4e, 0xc9, 0x13, 0x52, 0x21, 0xd0,
        0x69, 0x1b, 0xa7, 0xd2, 0x02,
    ],
    [
        0xdd, 0x68, 0x2d, 0xae, 0xc5, 0xa9, 0x0d, 0xd2, 0x95, 0xd1, 0x4d, 0xa4, 0xb0, 0xbe, 0xc9,
        0x28, 0x10, 0x17, 0xb5, 0xbe,
    ],
    [
        0x9c, 0x5a, 0xe8, 0x9c, 0x4a, 0xf6, 0xaa, 0x32, 0xce, 0x58, 0x58, 0x8d, 0xba, 0xf9, 0x0d,
        0x18, 0xa8, 0x55, 0xb6, 0xde,
    ],
];

fn build_signers() -> RustVec<SignerAddress> {
    let mut signers = RustVec::new();
    for signer in REDSTONE_SIGNERS.iter() {
        signers.push(SignerAddress::from(signer.to_vec()));
    }
    signers
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

    let config = Config::try_new(
        REDSTONE_SIGNER_THRESHOLD,
        build_signers(),
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
