use soroban_sdk::contracterror;

/// Shared error code ranges to prevent cross-contract code collisions.
/// Each contract has a dedicated range of 100 error codes.
///
/// Ranges:
/// - AuctionError: 1000-1099
/// - EscrowError: 2000-2099
/// - FactoryError: 3000-3099
/// - CoreError: 4000-4099
/// - ChainAddressError: 5000-5099

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum AuctionError {
    NotWinner = 1001,
    AlreadyClaimed = 1002,
    NotClosed = 1003,
    NoFactoryContract = 1004,
    Unauthorized = 1005,
    InvalidState = 1006,
    BidTooLow = 1007,
    AuctionNotOpen = 1008,
    AuctionNotClosed = 1009,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum EscrowError {
    /// The vault balance is insufficient to cover the requested amount.
    InsufficientBalance = 2001,
    /// The release timestamp must be in the future relative to the current ledger time.
    PastReleaseTime = 2002,
    /// The commitment is not registered in the Registration contract.
    CommitmentNotRegistered = 2003,
    /// The requested amount must be strictly greater than 0.
    InvalidAmount = 2004,
    /// The specified vault commitment was not found in the persistent storage.
    VaultNotFound = 2005,
    /// The payment counter has reached its maximum value (u32::MAX), preventing new IDs.
    PaymentCounterOverflow = 2006,
    /// The specified scheduled payment was not found.
    PaymentNotFound = 2007,
    /// The scheduled payment has already been executed.
    PaymentAlreadyExecuted = 2008,
    /// The scheduled payment is not yet due for execution.
    PaymentNotYetDue = 2009,
    /// The vault is inactive and cannot process new payments.
    VaultInactive = 2010,
    /// The interval must be strictly greater than 0.
    InvalidInterval = 2011,
    /// The auto-pay counter has reached its maximum value (u32::MAX), preventing new IDs.
    AutoPayCounterOverflow = 2012,
    /// The specified auto-pay rule was not found.
    AutoPayNotFound = 2013,
    /// The interval has not yet elapsed since the last payment.
    IntervalNotElapsed = 2014,
    /// A vault already exists for this commitment.
    VaultAlreadyExists = 2015,
    /// The contract has already been initialized.
    AlreadyInitialized = 2016,
    /// Self-payment is not allowed (from == to).
    SelfPaymentNotAllowed = 2017,
    /// Caller is not the escrow admin.
    Unauthorized = 2018,
    /// The contract is paused; money-moving operations are blocked.
    ContractPaused = 2019,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum FactoryError {
    Unauthorized = 3001,
    AlreadyDeployed = 3002,
    CoreContractNotConfigured = 3003,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum CoreError {
    /// The requested resource was not found.
    NotFound = 4001,
    /// The SMT root has not been set yet.
    RootNotSet = 4002,
    /// Commitment is already registered.
    DuplicateCommitment = 4003,
    /// public_signals.old_root does not match the current on-chain SMT root.
    StaleRoot = 4004,
    /// The supplied Groth16 proof is invalid.
    InvalidProof = 4005,
    /// The username is registered but has no primary Stellar address linked.
    NoAddressLinked = 4006,
    /// Caller is not the registered owner of the commitment.
    Unauthorized = 4007,
    /// new_owner is the same as the current owner.
    SameOwner = 4008,
    /// initialize() has already been called on this contract instance.
    AlreadyInitialized = 4009,
    /// Commitment is already registered via register().
    AlreadyRegistered = 4010,
    /// The new SMT root matches the existing on-chain root.
    RootUnchanged = 4011,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ChainAddressError {
    /// Caller is not the owner of the username commitment.
    Unauthorized = 5001,
    /// The username commitment is not registered.
    NotRegistered = 5002,
    /// The address format is invalid for the given chain type.
    InvalidAddress = 5003,
}
