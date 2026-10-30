//! Storage scopes for the oracle adapter.
//!
//! Decision: keep `Admin`, `StalenessThreshold`, and `Paused` in instance
//! storage rather than standardizing on the other contracts' persistent admin
//! keys. These are a small, bounded set of contract-wide settings used by the
//! oracle's authorization, freshness, and pause checks. Instance storage embeds
//! them in the contract instance entry, which is already loaded for an
//! invocation, avoiding a separate ledger entry for each setting. Keeping this
//! always-loaded data small is the tradeoff; per-asset prices and per-address
//! feeder permissions can grow, so they remain in persistent storage.
//!
//! # TTL and archival
//!
//! Instance keys share the contract instance entry's TTL; they have no
//! independent per-key lifetime. This is acceptable because the singleton
//! configuration should live and be restored with the oracle instance, not
//! expire separately while the oracle is otherwise callable. Archival is not
//! permanent deletion or permission to reinitialize: restore the archived
//! instance, including its configuration, before executing contract calls.
//!
//! These helpers do not extend TTLs, and ordinary reads/writes are not a TTL
//! maintenance policy. Deployment operators must maintain the instance and
//! contract code lifetimes (for example, through explicit TTL extension) and
//! arrange restoration when needed. Persistent prices and feeder permissions
//! have their own entry TTLs and need separate maintenance/restoration; keeping
//! the instance alive does not extend those entries. No keys, values, or storage
//! scopes are changed by this documentation decision.

use soroban_sdk::{Address, Env, Symbol};

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
    env.storage()
        .persistent()
        .set(&DataKey::AssetFeed(asset.clone()), feed_id);
    env.storage()
        .persistent()
        .set(&DataKey::FeedAsset(feed_id.clone()), asset);
}

pub fn get_feed_asset(env: &Env, feed_id: &Symbol) -> Option<Address> {
    env.storage()
        .persistent()
        .get(&DataKey::FeedAsset(feed_id.clone()))
}
