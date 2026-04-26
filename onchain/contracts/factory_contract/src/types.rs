use soroban_sdk::{contracttype, Address, BytesN};

/// Storage keys used by the factory contract.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Key for the auction contract address.
    AuctionContract,
    /// Key for the core contract address.
    CoreContract,
    /// Key for a username record identified by its hash.
    Username(BytesN<32>),
    /// Key for the deploy configuration.
    Config,
}

/// A record representing a deployed username and its associated metadata.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UsernameRecord {
    /// The hash of the username.
    pub username_hash: BytesN<32>,
    /// The current owner of the username.
    pub owner: Address,
    /// The ledger timestamp when the username was registered.
    pub registered_at: u64,
    /// The core contract associated with this username.
    pub core_contract: Address,
}

/// Configuration used when deploying new username contracts.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeployConfig {
    /// The WASM hash of the core contract to deploy.
    pub core_contract_wasm_hash: BytesN<32>,
    /// The admin address with deployment privileges.
    pub admin: Address,
}
