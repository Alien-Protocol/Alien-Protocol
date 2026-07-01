// SPDX-License-Identifier: Apache-2.0
use soroban_sdk::contracterror;

#[contracterror]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum OracleError {
    // Core required error types
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    AlreadyAdmin = 4,
    PriceNotFound = 5,
    StalePrice = 6,
    InvalidPrice = 7,
    InvalidTimestamp = 8,
    AlreadyAuthorized = 9,
    FeederNotFound = 10,
    InvalidThreshold = 11,
    // Existing error types retained for compatibility
    OraclePaused = 12,
    AlreadyPaused = 13,
    NotPaused = 14,
    // Additional placeholders for potential future use
    UnknownFeed = 15,
    InvalidPayload = 16,
    FeedNotWritten = 17,
}
