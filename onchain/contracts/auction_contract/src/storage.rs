use crate::errors::AuctionError;
use crate::types::{AuctionState, AuctionStatus, Bid, InstanceKey};
use shared::storage as shared_storage;
use soroban_sdk::{contracttype, Address, BytesN, Env, Vec};

/// TTL constants for persistent storage entries.
/// PERSISTENT_BUMP_AMOUNT: 30 days × 24h × 3600s / 5s per ledger = 518_400 ledgers
#[allow(dead_code)]
pub(crate) const PERSISTENT_BUMP_AMOUNT: u32 = 518_400; // 30 * 24 * 3600 / 5
/// PERSISTENT_LIFETIME_THRESHOLD: 7 days × 24h × 3600s / 5s per ledger = 120_960 ledgers
#[allow(dead_code)]
pub(crate) const PERSISTENT_LIFETIME_THRESHOLD: u32 = 120_960; // 7 * 24 * 3600 / 5

#[contracttype]
pub enum DataKey {
    Auction(BytesN<32>),
    Bid(BytesN<32>, Address),
    AllBidders(BytesN<32>),
}

pub fn get_status(env: &Env) -> AuctionStatus {
    shared_storage::get_instance(env, &InstanceKey::Status).unwrap_or(AuctionStatus::Open)
}

pub fn set_status(env: &Env, status: AuctionStatus) {
    shared_storage::set_instance(env, &InstanceKey::Status, &status);
}

pub fn get_highest_bidder(env: &Env) -> Option<Address> {
    shared_storage::get_instance(env, &InstanceKey::HighestBidder)
}

pub fn set_highest_bidder(env: &Env, bidder: &Address) {
    shared_storage::set_instance(env, &InstanceKey::HighestBidder, bidder);
}

pub fn get_factory_contract(env: &Env) -> Option<Address> {
    shared_storage::get_instance(env, &InstanceKey::FactoryContract)
}

pub fn set_factory_contract(env: &Env, factory: &Address) {
    shared_storage::set_instance(env, &InstanceKey::FactoryContract, factory);
}

pub fn get_end_time(env: &Env) -> u64 {
    shared_storage::get_instance(env, &InstanceKey::EndTime).unwrap_or(0)
}

pub fn set_end_time(env: &Env, end_time: u64) {
    shared_storage::set_instance(env, &InstanceKey::EndTime, &end_time);
}

pub fn get_highest_bid(env: &Env) -> u128 {
    shared_storage::get_instance(env, &InstanceKey::HighestBid).unwrap_or(0)
}

pub fn set_highest_bid(env: &Env, bid: u128) {
    shared_storage::set_instance(env, &InstanceKey::HighestBid, &bid);
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
    shared_storage::set_persistent(env, &key, &status);
}

pub fn auction_get_seller(env: &Env, id: u32) -> Result<Address, AuctionError> {
    env.storage()
        .persistent()
        .get(&AuctionKey::Seller(id))
        .ok_or(AuctionError::InvalidState)
}

pub fn auction_set_seller(env: &Env, id: u32, seller: &Address) {
    let key = AuctionKey::Seller(id);
    shared_storage::set_persistent(env, &key, seller);
}

pub fn auction_get_asset(env: &Env, id: u32) -> Result<Address, AuctionError> {
    env.storage()
        .persistent()
        .get(&AuctionKey::Asset(id))
        .ok_or(AuctionError::InvalidState)
}

pub fn auction_set_asset(env: &Env, id: u32, asset: &Address) {
    let key = AuctionKey::Asset(id);
    shared_storage::set_persistent(env, &key, asset);
}

pub fn auction_get_min_bid(env: &Env, id: u32) -> i128 {
    env.storage()
        .persistent()
        .get(&AuctionKey::MinBid(id))
        .unwrap_or(0)
}

pub fn auction_set_min_bid(env: &Env, id: u32, min_bid: i128) {
    let key = AuctionKey::MinBid(id);
    shared_storage::set_persistent(env, &key, &min_bid);
}

pub fn auction_get_min_bid_increment(env: &Env, id: u32) -> i128 {
    env.storage()
        .persistent()
        .get(&AuctionKey::MinBidIncrement(id))
        .unwrap_or(0)
}

pub fn auction_set_min_bid_increment(env: &Env, id: u32, min_bid_increment: i128) {
    let key = AuctionKey::MinBidIncrement(id);
    env.storage().persistent().set(&key, &min_bid_increment);
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
    shared_storage::set_persistent(env, &key, &end_time);
}

pub fn auction_get_highest_bidder(env: &Env, id: u32) -> Option<Address> {
    env.storage()
        .persistent()
        .get(&AuctionKey::HighestBidder(id))
}

pub fn auction_set_highest_bidder(env: &Env, id: u32, bidder: &Address) {
    let key = AuctionKey::HighestBidder(id);
    shared_storage::set_persistent(env, &key, bidder);
}

pub fn auction_get_highest_bid(env: &Env, id: u32) -> i128 {
    env.storage()
        .persistent()
        .get(&AuctionKey::HighestBid(id))
        .unwrap_or(0)
}

pub fn auction_set_highest_bid(env: &Env, id: u32, bid: i128) {
    let key = AuctionKey::HighestBid(id);
    shared_storage::set_persistent(env, &key, &bid);
}

pub fn auction_is_claimed(env: &Env, id: u32) -> bool {
    env.storage()
        .persistent()
        .get(&AuctionKey::Claimed(id))
        .unwrap_or(false)
}

pub fn auction_set_claimed(env: &Env, id: u32) {
    let key = AuctionKey::Claimed(id);
    shared_storage::set_persistent(env, &key, &true);
}

pub fn auction_get_username_hash(env: &Env, id: u32) -> BytesN<32> {
    env.storage()
        .persistent()
        .get(&AuctionKey::UsernameHash(id))
        .unwrap_or(BytesN::from_array(env, &[0; 32]))
}

pub fn auction_set_username_hash(env: &Env, id: u32, username_hash: &BytesN<32>) {
    let key = AuctionKey::UsernameHash(id);
    shared_storage::set_persistent(env, &key, username_hash);
}

pub fn auction_get_outbid_amount(env: &Env, id: u32, bidder: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&AuctionKey::OutbidAmount(id, bidder.clone()))
        .unwrap_or(0)
}

pub fn auction_set_outbid_amount(env: &Env, id: u32, bidder: &Address, amount: i128) {
    let key = AuctionKey::OutbidAmount(id, bidder.clone());
    shared_storage::set_persistent(env, &key, &amount);
}

pub fn auction_is_bid_refunded(env: &Env, id: u32, bidder: &Address) -> bool {
    env.storage()
        .persistent()
        .get(&AuctionKey::BidRefunded(id, bidder.clone()))
        .unwrap_or(false)
}

pub fn auction_set_bid_refunded(env: &Env, id: u32, bidder: &Address) {
    let key = AuctionKey::BidRefunded(id, bidder.clone());
    shared_storage::set_persistent(env, &key, &true);
}

pub fn get_auction(env: &Env, hash: &BytesN<32>) -> Option<AuctionState> {
    shared_storage::get_persistent(env, &DataKey::Auction(hash.clone()))
}

pub fn set_auction(env: &Env, hash: &BytesN<32>, state: &AuctionState) {
    let key = DataKey::Auction(hash.clone());
    shared_storage::set_persistent(env, &key, state);
}

pub fn has_auction(env: &Env, hash: &BytesN<32>) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Auction(hash.clone()))
}

pub fn get_bid(env: &Env, hash: &BytesN<32>, bidder: &Address) -> Option<Bid> {
    shared_storage::get_persistent(env, &DataKey::Bid(hash.clone(), bidder.clone()))
}

pub fn set_bid(env: &Env, hash: &BytesN<32>, bidder: &Address, bid: &Bid) {
    let key = DataKey::Bid(hash.clone(), bidder.clone());
    shared_storage::set_persistent(env, &key, bid);
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
        shared_storage::set_persistent(env, &key, &bidders);
    }
}
