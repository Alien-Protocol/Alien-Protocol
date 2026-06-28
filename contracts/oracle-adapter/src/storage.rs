use soroban_sdk::{Address, Env};

use crate::types::{DataKey, PriceData};

pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Admin)
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

pub fn get_staleness_threshold(env: &Env) -> Option<u64> {
    env.storage().instance().get(&DataKey::StalenessThreshold)
}

pub fn set_staleness_threshold(env: &Env, threshold: u64) {
    env.storage()
        .instance()
        .set(&DataKey::StalenessThreshold, &threshold);
}

pub fn get_price(env: &Env, asset: &Address) -> Option<PriceData> {
    env.storage()
        .persistent()
        .get(&DataKey::Price(asset.clone()))
}

pub fn set_price(env: &Env, asset: &Address, data: &PriceData) {
    env.storage()
        .persistent()
        .set(&DataKey::Price(asset.clone()), data);
}

pub fn is_authorized_feeder(env: &Env, feeder: &Address) -> bool {
    env.storage()
        .persistent()
        .get(&DataKey::AuthorizedFeeder(feeder.clone()))
        .unwrap_or(false)
}

pub fn set_authorized_feeder(env: &Env, feeder: &Address) {
    env.storage()
        .persistent()
        .set(&DataKey::AuthorizedFeeder(feeder.clone()), &true);
}

pub fn is_initialized(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

pub fn is_paused(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&DataKey::Paused)
        .unwrap_or(false)
}

pub fn set_paused(env: &Env, paused: bool) {
    env.storage().instance().set(&DataKey::Paused, &paused);
}

pub fn has_authorized_feeder(env: &Env, feeder: &Address) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::AuthorizedFeeder(feeder.clone()))
}

pub fn remove_authorized_feeder(env: &Env, feeder: &Address) {
    env.storage()
        .persistent()
        .remove(&DataKey::AuthorizedFeeder(feeder.clone()));
}
