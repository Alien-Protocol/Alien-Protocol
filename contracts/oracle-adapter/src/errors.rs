use soroban_sdk::contracterror;

#[contracterror]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum OracleError {
    NotInitialized = 1,
    AlreadyAdmin = 2,
    OraclePaused = 3,
    AlreadyPaused = 4,
    FeederNotFound = 5,
    NotPaused = 6,
    AlreadyAuthorized = 7,
    Unauthorized = 8,
    UnknownFeed = 9,
    InvalidPayload = 10,
    FeedNotWritten = 11,
    PriceNotFound = 12,
    StalePrice = 13,
}
