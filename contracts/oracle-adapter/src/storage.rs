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

pub fn is_initialized(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

pub fn has_feeder(env: &Env, feeder: &Address) -> bool {
    env.storage()
        .instance()
        .has(&DataKey::Feeder(feeder.clone()))
}

pub fn add_feeder(env: &Env, feeder: &Address) {
    env.storage()
        .instance()
        .set(&DataKey::Feeder(feeder.clone()), &true);
}

pub fn remove_feeder(env: &Env, feeder: &Address) {
    env.storage()
        .instance()
        .remove(&DataKey::Feeder(feeder.clone()));
}
