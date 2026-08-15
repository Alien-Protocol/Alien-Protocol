use soroban_sdk::contracterror;

#[contracterror]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum EngineError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    AlreadyAdmin = 4,
    InvalidAddress = 5,
    InvalidAmount = 6,
    NotLiquidatable = 7,
    NoPosition = 8,
    Overflow = 9,
    NotImplemented = 10,
}
