use soroban_sdk::{Address, Env};

use crate::{errors, events, storage, types};

pub fn create_auction(
    env: &Env,
    id: u32,
    seller: Address,
    asset: Address,
    min_bid: i128,
    end_time: u64,
) -> Result<(), errors::AuctionError> {
    seller.require_auth();
    if storage::auction_exists(env, id) {
        return Err(errors::AuctionError::AuctionNotOpen);
    }
    if end_time <= env.ledger().timestamp() {
        return Err(errors::AuctionError::AuctionNotClosed);
    }
    if min_bid <= 0 {
        return Err(errors::AuctionError::BidTooLow);
    }
    storage::auction_set_seller(env, id, &seller);
    storage::auction_set_asset(env, id, &asset);
    storage::auction_set_min_bid(env, id, min_bid);
    storage::auction_set_end_time(env, id, end_time);
    storage::auction_set_status(env, id, types::AuctionStatus::Open);

    let username_hash = soroban_sdk::BytesN::from_array(env, &[0u8; 32]);
    storage::auction_set_username_hash(env, id, &username_hash);
    events::emit_auction_created(env, &username_hash, end_time, min_bid);

    Ok(())
}

pub fn place_bid(
    env: &Env,
    id: u32,
    bidder: Address,
    amount: i128,
) -> Result<(), errors::AuctionError> {
    bidder.require_auth();
    if !storage::auction_exists(env, id) {
        return Err(errors::AuctionError::AuctionNotOpen);
    }
    if storage::auction_get_status(env, id) != types::AuctionStatus::Open {
        return Err(errors::AuctionError::AuctionNotOpen);
    }
    let end_time = storage::auction_get_end_time(env, id);
    if env.ledger().timestamp() >= end_time {
        return Err(errors::AuctionError::AuctionNotOpen);
    }
    let min_bid = storage::auction_get_min_bid(env, id);
    let highest_bid = storage::auction_get_highest_bid(env, id);
    if storage::auction_get_highest_bidder(env, id)
        .as_ref()
        .map(|h| h == &bidder)
        .unwrap_or(false)
    {
        return Err(errors::AuctionError::Unauthorized);
    }
    if amount < min_bid || amount <= highest_bid {
        return Err(errors::AuctionError::BidTooLow);
    }
    let asset = storage::auction_get_asset(env, id)?;
    let token = soroban_sdk::token::Client::new(env, &asset);
    token.transfer(&bidder, env.current_contract_address(), &amount);
    if let Some(prev_bidder) = storage::auction_get_highest_bidder(env, id) {
        storage::auction_set_outbid_amount(env, id, &prev_bidder, highest_bid);
        let username_hash = storage::auction_get_username_hash(env, id);
        events::emit_bid_refunded(env, &username_hash, &prev_bidder, highest_bid);
    }
    storage::auction_set_highest_bidder(env, id, &bidder);
    storage::auction_set_highest_bid(env, id, amount);

    let username_hash = storage::auction_get_username_hash(env, id);
    events::emit_bid_placed(env, &username_hash, &bidder, amount);

    Ok(())
}

pub fn close_auction_by_id(env: &Env, id: u32) -> Result<(), errors::AuctionError> {
    if !storage::auction_exists(env, id) {
        return Err(errors::AuctionError::AuctionNotOpen);
    }
    let end_time = storage::auction_get_end_time(env, id);
    if env.ledger().timestamp() < end_time {
        return Err(errors::AuctionError::AuctionNotClosed);
    }
    storage::auction_set_status(env, id, types::AuctionStatus::Closed);

    let username_hash = storage::auction_get_username_hash(env, id);
    let winner = storage::auction_get_highest_bidder(env, id);
    let winning_bid = storage::auction_get_highest_bid(env, id) as u128;
    events::emit_auction_closed(env, &username_hash, winner, winning_bid);

    Ok(())
}

pub fn claim(env: &Env, id: u32, claimant: Address) -> Result<(), errors::AuctionError> {
    claimant.require_auth();
    let status = storage::auction_get_status(env, id);
    if status != types::AuctionStatus::Closed {
        return Err(errors::AuctionError::NotClosed);
    }
    if storage::auction_is_claimed(env, id) {
        return Err(errors::AuctionError::AlreadyClaimed);
    }
    let winner = storage::auction_get_highest_bidder(env, id);
    if winner.as_ref().map(|w| w == &claimant).unwrap_or(false) {
        let asset = storage::auction_get_asset(env, id)?;
        let token = soroban_sdk::token::Client::new(env, &asset);
        let winning_bid = storage::auction_get_highest_bid(env, id);
        let seller = storage::auction_get_seller(env, id)?;
        token.transfer(&env.current_contract_address(), &seller, &winning_bid);
        storage::auction_set_claimed(env, id);
        Ok(())
    } else {
        Err(errors::AuctionError::NotWinner)
    }
}
