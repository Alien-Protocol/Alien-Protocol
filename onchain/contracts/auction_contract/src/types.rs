use soroban_sdk::{contracttype, Address, BytesN};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuctionStatus {
    Open,
    Closed,
    Claimed,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InstanceKey {
    Status,
    HighestBidder,
    FactoryContract,
    EndTime,
    HighestBid,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuctionKey {
    Seller(u32),
    Asset(u32),
    MinBid(u32),
    MinBidIncrement(u32),
    EndTime(u32),
    HighestBidder(u32),
    HighestBid(u32),
    OutbidAmount(u32, Address),
    BidRefunded(u32, Address),
    Status(u32),
    Claimed(u32),
    UsernameHash(u32),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuctionConfig {
    pub username_hash: BytesN<32>,
    pub start_time: u64,
    pub end_time: u64,
    pub min_bid: i128,
    pub min_bid_increment: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuctionState {
    pub config: AuctionConfig,
    pub status: AuctionStatus,
    pub highest_bidder: Option<Address>,
    pub highest_bid: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bid {
    pub bidder: Address,
    pub amount: i128,
    pub timestamp: u64,
}
