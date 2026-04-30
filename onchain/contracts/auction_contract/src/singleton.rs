use soroban_sdk::{vec, Address, BytesN, Env, IntoVal, Symbol};

use crate::{errors::AuctionError, events, storage, types};

pub fn close_auction(env: &Env, username_hash: BytesN<32>) -> Result<(), AuctionError> {
    let status = storage::get_status(env);
    crate::require_status(
        status,
        types::AuctionStatus::Open,
        AuctionError::AuctionNotOpen,
    )?;

    let current_time = env.ledger().timestamp();
    let end_time = storage::get_end_time(env);

    if current_time < end_time {
        return Err(AuctionError::AuctionNotClosed);
    }

    storage::set_status(env, types::AuctionStatus::Closed);

    let winner = storage::get_highest_bidder(env);
    let winning_bid = storage::get_highest_bid(env);

    events::emit_auction_closed(env, &username_hash, winner, winning_bid);

    Ok(())
}

pub fn claim_username(
    env: &Env,
    username_hash: BytesN<32>,
    claimer: Address,
) -> Result<(), AuctionError> {
    claimer.require_auth();

    let status = storage::get_status(env);

    if status == types::AuctionStatus::Claimed {
        return Err(AuctionError::AlreadyClaimed);
    }

    crate::require_status(
        status,
        types::AuctionStatus::Closed,
        AuctionError::NotClosed,
    )?;

    let highest_bidder = storage::get_highest_bidder(env);
    if !highest_bidder.map(|h| h == claimer).unwrap_or(false) {
        return Err(AuctionError::NotWinner);
    }

    storage::set_status(env, types::AuctionStatus::Claimed);

    let factory_addr = storage::get_factory_contract(env).ok_or(AuctionError::NoFactoryContract)?;

    env.invoke_contract::<()>(
        &factory_addr,
        &Symbol::new(env, "deploy_username"),
        vec![env, username_hash.into_val(env), claimer.into_val(env)],
    );

    events::emit_username_claimed(env, &username_hash, &claimer);

    Ok(())
}
