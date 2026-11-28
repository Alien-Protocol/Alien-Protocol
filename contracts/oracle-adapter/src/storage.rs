use soroban_sdk::{Address, Env, Symbol, Vec};

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

pub fn get_asset_feed(env: &Env, asset: &Address) -> Option<Symbol> {
    env.storage()
        .persistent()
        .get(&DataKey::AssetFeed(asset.clone()))
}

pub fn set_asset_feed(env: &Env, asset: &Address, feed_id: &Symbol) {
    if let Some(old_feed) = get_asset_feed(env, asset) {
        if old_feed != *feed_id {
            let old_list = get_feed_assets(env, &old_feed);
            let mut new_old_list = Vec::new(env);
            for a in old_list.iter() {
                if a != *asset {
                    new_old_list.push_back(a);
                }
            }
            env.storage()
                .persistent()
                .set(&DataKey::FeedAssets(old_feed), &new_old_list);
        }
    }

    env.storage()
        .persistent()
        .set(&DataKey::AssetFeed(asset.clone()), feed_id);

    let mut list = get_feed_assets(env, feed_id);
    let mut found = false;
    for a in list.iter() {
        if a == *asset {
            found = true;
            break;
        }
    }
    if !found {
        list.push_back(asset.clone());
        env.storage()
            .persistent()
            .set(&DataKey::FeedAssets(feed_id.clone()), &list);
    }
}

pub fn get_feed_assets(env: &Env, feed_id: &Symbol) -> Vec<Address> {
    env.storage()
        .persistent()
        .get(&DataKey::FeedAssets(feed_id.clone()))
        .unwrap_or_else(|| Vec::new(env))
}
