use soroban_sdk::{contracttype, Address};

pub use shared::PriceData;

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
}
