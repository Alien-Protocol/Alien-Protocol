use soroban_sdk::contracterror;

#[contracterror]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PoolError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    AlreadyAdmin = 4,
    InvalidAddress = 5,
    InvalidRate = 6,
    AlreadyPaused = 7,
    NotPaused = 8,
    InvalidAmount = 9,
    InsufficientLiquidity = 10,
    InsufficientSupply = 11,
    PoolPaused = 12,
    Overflow = 13,
    UnsupportedAsset = 14,
    ExceedsBorrowLimit = 15,
    NoCollateral = 16,
    BorrowPaused = 17,
    NoDebt = 18,
    BelowMinDebt = 19,
    RepayPaused = 20,
    NotImplemented = 21,
}

impl From<shared::SharedError> for PoolError {
    fn from(err: shared::SharedError) -> Self {
        match err {
            shared::SharedError::Overflow => PoolError::Overflow,
            shared::SharedError::InvalidAmount => PoolError::InvalidAmount,
            shared::SharedError::InvalidBps => PoolError::InvalidRate,
            shared::SharedError::NotImplemented => PoolError::NotImplemented,
            shared::SharedError::DivisionByZero => PoolError::Overflow,
        }
    }
}
