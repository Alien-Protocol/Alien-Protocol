use soroban_sdk::contracterror;

#[contracterror]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SharedError {
    Overflow = 1,
    InvalidAmount = 2,
    InvalidBps = 3,
    NotImplemented = 4,
    DivisionByZero = 5,
}
