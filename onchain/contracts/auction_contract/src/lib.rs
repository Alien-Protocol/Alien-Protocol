#![no_std]
use soroban_sdk::{contract, contractimpl, Address, BytesN, Env};

pub mod errors;
pub mod events;
pub mod indexed;
pub mod singleton;
pub mod storage;
pub mod types;

use crate::errors::AuctionError;
use crate::events::{AUCTION_CLOSED, AUCTION_CREATED, BID_PLACED, BID_REFUNDED, USERNAME_CLAIMED};
use crate::types::AuctionStatus;

/// Internal helper to ensure event symbols are included in the WASM.
#[allow(dead_code)]
fn _touch_event_symbols() {
    let _ = (
        AUCTION_CREATED,
        BID_PLACED,
        AUCTION_CLOSED,
        USERNAME_CLAIMED,
        BID_REFUNDED,
    );
}

#[allow(clippy::missing_docs_in_private_items)]
fn require_status(
    status: AuctionStatus,
    expected: AuctionStatus,
    err: AuctionError,
) -> Result<(), AuctionError> {
    if status != expected {
        return Err(err);
    }
    Ok(())
}

#[cfg(test)]
mod test;

#[contract]
pub struct AuctionContract;

#[contractimpl]
impl AuctionContract {
    pub fn close_auction(env: Env, username_hash: BytesN<32>) -> Result<(), errors::AuctionError> {
        singleton::close_auction(&env, username_hash)
    }

    pub fn claim_username(
        env: Env,
        username_hash: BytesN<32>,
        claimer: Address,
    ) -> Result<(), errors::AuctionError> {
        singleton::claim_username(&env, username_hash, claimer)
    }
    pub fn create_auction(
        env: Env,
        id: u32,
        seller: Address,
        asset: Address,
        min_bid: i128,
        end_time: u64,
    ) -> Result<(), errors::AuctionError> {
        indexed::create_auction(&env, id, seller, asset, min_bid, end_time)
    }

    pub fn place_bid(
        env: Env,
        id: u32,
        bidder: Address,
        amount: i128,
    ) -> Result<(), errors::AuctionError> {
        indexed::place_bid(&env, id, bidder, amount)
    }

    pub fn refund_bid(env: Env, id: u32, bidder: Address) -> Result<(), errors::AuctionError> {
        bidder.require_auth();

        let status = storage::auction_get_status(&env, id);
        if status != types::AuctionStatus::Closed {
            return Err(errors::AuctionError::NotClosed);
        }

        let highest_bidder = storage::auction_get_highest_bidder(&env, id);
        if highest_bidder
            .as_ref()
            .map(|h| h == &bidder)
            .unwrap_or(false)
        {
            return Err(errors::AuctionError::NotWinner);
        }

        if storage::auction_is_bid_refunded(&env, id, &bidder) {
            return Err(errors::AuctionError::AlreadyClaimed);
        }

        let amount = storage::auction_get_outbid_amount(&env, id, &bidder);
        if amount <= 0 {
            return Err(errors::AuctionError::InvalidState);
        }

        // Transfer asset back to bidder (single transfer)
        let asset = storage::auction_get_asset(&env, id)?;

        let token = soroban_sdk::token::Client::new(&env, &asset);
        token.transfer(&env.current_contract_address(), &bidder, &amount);

        storage::auction_set_bid_refunded(&env, id, &bidder);
        storage::auction_set_outbid_amount(&env, id, &bidder, 0);

        // Emit a single refund event
        events::emit_bid_refunded(
            &env,
            &storage::auction_get_username_hash(&env, id),
            &bidder,
            amount,
        );

        Ok(())
    }

    pub fn close_auction_by_id(env: Env, id: u32) -> Result<(), errors::AuctionError> {
        indexed::close_auction_by_id(&env, id)
    }

    pub fn claim(env: Env, id: u32, claimant: Address) -> Result<(), errors::AuctionError> {
        indexed::claim(&env, id, claimant)
    }

    #[allow(clippy::type_complexity)]
    pub fn get_auction_info(
        env: Env,
        id: u32,
    ) -> Result<
        Option<(
            Address,
            Address,
            i128,
            u64,
            i128,
            Option<Address>,
            types::AuctionStatus,
            bool,
        )>,
        errors::AuctionError,
    > {
        if !storage::auction_exists(&env, id) {
            return Ok(None);
        }
        Ok(Some((
            storage::auction_get_seller(&env, id)?,
            storage::auction_get_asset(&env, id)?,
            storage::auction_get_min_bid(&env, id),
            storage::auction_get_end_time(&env, id),
            storage::auction_get_highest_bid(&env, id),
            storage::auction_get_highest_bidder(&env, id),
            storage::auction_get_status(&env, id),
            storage::auction_is_claimed(&env, id),
        )))
    }

    pub fn get_auction(env: Env, hash: BytesN<32>) -> Option<types::AuctionState> {
        storage::get_auction(&env, &hash)
    }

    pub fn has_auction(env: Env, hash: BytesN<32>) -> bool {
        storage::has_auction(&env, &hash)
    }

    pub fn get_bid(env: Env, hash: BytesN<32>, bidder: Address) -> Option<types::Bid> {
        storage::get_bid(&env, &hash, &bidder)
    }

    pub fn get_all_bidders(env: Env, hash: BytesN<32>) -> soroban_sdk::Vec<Address> {
        storage::get_all_bidders(&env, &hash)
    }
}
