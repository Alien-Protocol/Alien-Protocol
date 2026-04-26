use crate::errors::AuctionError;
use crate::types::{AuctionState, AuctionStatus, Bid, InstanceKey};
use soroban_sdk::{contracttype, Address, BytesN, Env, Vec};

/// The amount of ledger entries to bump persistent storage by.
pub(crate) const PERSISTENT_BUMP_AMOUNT: u32 = 518_400; // 30 * 24 * 3600 / 5
/// The threshold for persistent storage TTL to trigger an auto-bump.
pub(crate) const PERSISTENT_LIFETIME_THRESHOLD: u32 = 120_960; // 7 * 24 * 3600 / 5

#[contracttype]
pub enum DataKey {
    Auction(BytesN<32>),
    Bid(BytesN<32>, Address),
    AllBidders(BytesN<32>),
}

pub fn get_status(env: &Env) -> AuctionStatus {
    env.storage()
        .instance()
        .get(&InstanceKey::Status)
        .unwrap_or(AuctionStatus::Open)
}

pub fn set_status(env: &Env, status: AuctionStatus) {
    env.storage().instance().set(&InstanceKey::Status, &status);
}

pub fn get_highest_bidder(env: &Env) -> Option<Address> {
    env.storage().instance().get(&InstanceKey::HighestBidder)
}

pub fn set_highest_bidder(env: &Env, bidder: &Address) {
    env.storage()
        .instance()
        .set(&InstanceKey::HighestBidder, bidder);
}

pub fn get_factory_contract(env: &Env) -> Option<Address> {
    env.storage().instance().get(&InstanceKey::FactoryContract)
}

pub fn set_factory_contract(env: &Env, factory: &Address) {
    env.storage()
        .instance()
        .set(&InstanceKey::FactoryContract, factory);
}

pub fn get_end_time(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&InstanceKey::EndTime)
        .unwrap_or(0)
}

pub fn set_end_time(env: &Env, end_time: u64) {
    env.storage()
        .instance()
        .set(&InstanceKey::EndTime, &end_time);
}

pub fn get_highest_bid(env: &Env) -> u128 {
    env.storage()
        .instance()
        .get(&InstanceKey::HighestBid)
        .unwrap_or(0)
}

pub fn set_highest_bid(env: &Env, bid: u128) {
    env.storage().instance().set(&InstanceKey::HighestBid, &bid);
}

use crate::types::AuctionKey;

pub fn auction_exists(env: &Env, id: u32) -> bool {
    env.storage().persistent().has(&AuctionKey::Status(id))
}

pub fn auction_get_status(env: &Env, id: u32) -> crate::types::AuctionStatus {
    env.storage()
        .persistent()
        .get(&AuctionKey::Status(id))
        .unwrap_or(crate::types::AuctionStatus::Open)
}

pub fn auction_set_status(env: &Env, id: u32, status: crate::types::AuctionStatus) {
    let key = AuctionKey::Status(id);
    env.storage().persistent().set(&key, &status);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn auction_get_seller(env: &Env, id: u32) -> Result<Address, AuctionError> {
    env.storage()
        .persistent()
        .get(&AuctionKey::Seller(id))
        .ok_or(AuctionError::InvalidState)
}

pub fn auction_set_seller(env: &Env, id: u32, seller: &Address) {
    let key = AuctionKey::Seller(id);
    env.storage().persistent().set(&key, seller);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn auction_get_asset(env: &Env, id: u32) -> Result<Address, AuctionError> {
    env.storage()
        .persistent()
        .get(&AuctionKey::Asset(id))
        .ok_or(AuctionError::InvalidState)
}

pub fn auction_set_asset(env: &Env, id: u32, asset: &Address) {
    let key = AuctionKey::Asset(id);
    env.storage().persistent().set(&key, asset);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn auction_get_min_bid(env: &Env, id: u32) -> i128 {
    env.storage()
        .persistent()
        .get(&AuctionKey::MinBid(id))
        .unwrap_or(0)
}

pub fn auction_set_min_bid(env: &Env, id: u32, min_bid: i128) {
    let key = AuctionKey::MinBid(id);
    env.storage().persistent().set(&key, &min_bid);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn auction_get_end_time(env: &Env, id: u32) -> u64 {
    env.storage()
        .persistent()
        .get(&AuctionKey::EndTime(id))
        .unwrap_or(0)
}

pub fn auction_set_end_time(env: &Env, id: u32, end_time: u64) {
    let key = AuctionKey::EndTime(id);
    env.storage().persistent().set(&key, &end_time);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn auction_get_highest_bidder(env: &Env, id: u32) -> Option<Address> {
    env.storage()
        .persistent()
        .get(&AuctionKey::HighestBidder(id))
}

pub fn auction_set_highest_bidder(env: &Env, id: u32, bidder: &Address) {
    let key = AuctionKey::HighestBidder(id);
    env.storage().persistent().set(&key, bidder);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn auction_get_highest_bid(env: &Env, id: u32) -> i128 {
    env.storage()
        .persistent()
        .get(&AuctionKey::HighestBid(id))
        .unwrap_or(0)
}

pub fn auction_set_highest_bid(env: &Env, id: u32, bid: i128) {
    let key = AuctionKey::HighestBid(id);
    env.storage().persistent().set(&key, &bid);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn auction_is_claimed(env: &Env, id: u32) -> bool {
    env.storage()
        .persistent()
        .get(&AuctionKey::Claimed(id))
        .unwrap_or(false)
}

pub fn auction_set_claimed(env: &Env, id: u32) {
    let key = AuctionKey::Claimed(id);
    env.storage().persistent().set(&key, &true);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn auction_get_username_hash(env: &Env, id: u32) -> BytesN<32> {
    env.storage()
        .persistent()
        .get(&AuctionKey::UsernameHash(id))
        .unwrap_or(BytesN::from_array(env, &[0; 32]))
}

pub fn auction_set_username_hash(env: &Env, id: u32, username_hash: &BytesN<32>) {
    let key = AuctionKey::UsernameHash(id);
    env.storage().persistent().set(&key, username_hash);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn auction_get_outbid_amount(env: &Env, id: u32, bidder: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&AuctionKey::OutbidAmount(id, bidder.clone()))
        .unwrap_or(0)
}

pub fn auction_set_outbid_amount(env: &Env, id: u32, bidder: &Address, amount: i128) {
    let key = AuctionKey::OutbidAmount(id, bidder.clone());
    env.storage().persistent().set(&key, &amount);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn auction_is_bid_refunded(env: &Env, id: u32, bidder: &Address) -> bool {
    env.storage()
        .persistent()
        .get(&AuctionKey::BidRefunded(id, bidder.clone()))
        .unwrap_or(false)
}

pub fn auction_set_bid_refunded(env: &Env, id: u32, bidder: &Address) {
    let key = AuctionKey::BidRefunded(id, bidder.clone());
    env.storage().persistent().set(&key, &true);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn get_auction(env: &Env, hash: &BytesN<32>) -> Option<AuctionState> {
    env.storage()
        .persistent()
        .get(&DataKey::Auction(hash.clone()))
}

pub fn set_auction(env: &Env, hash: &BytesN<32>, state: &AuctionState) {
    let key = DataKey::Auction(hash.clone());
    env.storage().persistent().set(&key, state);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn has_auction(env: &Env, hash: &BytesN<32>) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Auction(hash.clone()))
}

pub fn get_bid(env: &Env, hash: &BytesN<32>, bidder: &Address) -> Option<Bid> {
    env.storage()
        .persistent()
        .get(&DataKey::Bid(hash.clone(), bidder.clone()))
}

pub fn set_bid(env: &Env, hash: &BytesN<32>, bidder: &Address, bid: &Bid) {
    let key = DataKey::Bid(hash.clone(), bidder.clone());
    env.storage().persistent().set(&key, bid);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn get_all_bidders(env: &Env, hash: &BytesN<32>) -> Vec<Address> {
    env.storage()
        .persistent()
        .get(&DataKey::AllBidders(hash.clone()))
        .unwrap_or(Vec::new(env))
}

pub fn add_bidder(env: &Env, hash: &BytesN<32>, bidder: Address) {
    let mut bidders = get_all_bidders(env, hash);
    if !bidders.contains(&bidder) {
        bidders.push_back(bidder);
        let key = DataKey::AllBidders(hash.clone());
        env.storage().persistent().set(&key, &bidders);
        env.storage().persistent().extend_ttl(
            &key,
            PERSISTENT_LIFETIME_THRESHOLD,
            PERSISTENT_BUMP_AMOUNT,
        );
    }
}
