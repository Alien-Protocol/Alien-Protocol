use soroban_sdk::{contracttype, Address, Symbol};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PriceData {
    pub price: i128,
    pub timestamp: u64,
    pub write_timestamp: u64,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Price data for a given asset, recorded in persistent storage and keyed by the asset address.
    Price(Address),
    /// Administrator address, stored in instance storage.
    Admin,
    /// Maximum acceptable price age in seconds, stored in instance storage.
    StalenessThreshold,
    /// Whether the oracle is paused, stored in instance storage.
    Paused,
    /// Authorization flag for a price feeder, stored in persistent storage and keyed by the feeder address.
    AuthorizedFeeder(Address),
    /// Mapping from asset address to RedStone feed ID, stored in persistent storage.
    AssetFeed(Address),
    /// Mapping from RedStone feed ID to asset address, stored in persistent storage.
    FeedAsset(Symbol),
}
