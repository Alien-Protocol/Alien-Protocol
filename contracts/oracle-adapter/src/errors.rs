use soroban_sdk::contracterror;

#[contracterror]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum OracleError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    AlreadyAdmin = 3,
    OraclePaused = 4,
    AlreadyPaused = 5,
    FeederNotFound = 6,
    NotPaused = 7,
    AlreadyAuthorized = 8,
    Unauthorized = 9,
    UnknownFeed = 10,
    InvalidPayload = 11,
    FeedNotWritten = 12,
    PriceNotFound = 13,
    StalePrice = 14,
    InvalidPrice = 15,
    InvalidTimestamp = 16,
    InvalidThreshold = 17,
}
