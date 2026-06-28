use crate::types::PriceData;
use soroban_sdk::{contracttype, Bytes, Env, Symbol, Vec};

#[contracttype]
#[derive(Clone)]
pub enum OracleDataKey {
    FeedPrice(Symbol),
    RedStoneSigners,
    RedStoneSignerThreshold,
}

pub fn get_feed_price(env: &Env, feed_id: &Symbol) -> Option<PriceData> {
    env.storage()
        .persistent()
        .get(&OracleDataKey::FeedPrice(feed_id.clone()))
}

pub fn set_feed_price(env: &Env, feed_id: &Symbol, data: &PriceData) {
    env.storage()
        .persistent()
        .set(&OracleDataKey::FeedPrice(feed_id.clone()), data);
}

pub fn get_redstone_signers(env: &Env) -> Option<Vec<Bytes>> {
    env.storage()
        .instance()
        .get(&OracleDataKey::RedStoneSigners)
}

pub fn set_redstone_signers(env: &Env, signers: &Vec<Bytes>) {
    env.storage()
        .instance()
        .set(&OracleDataKey::RedStoneSigners, signers);
}

pub fn get_redstone_threshold(env: &Env) -> Option<u32> {
    env.storage()
        .instance()
        .get(&OracleDataKey::RedStoneSignerThreshold)
}

pub fn set_redstone_threshold(env: &Env, threshold: u32) {
    env.storage()
        .instance()
        .set(&OracleDataKey::RedStoneSignerThreshold, &threshold);
}

pub fn is_redstone_initialized(env: &Env) -> bool {
    env.storage().instance().has(&OracleDataKey::RedStoneSigners)
}

