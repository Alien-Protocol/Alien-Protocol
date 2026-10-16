#![allow(unused)]
use soroban_sdk::{contracttype, Address};

pub use shared::PriceData;

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Price(Address),
    Admin,
    StalenessThreshold,
    Paused,
    AuthorizedFeeder(Address),
}
