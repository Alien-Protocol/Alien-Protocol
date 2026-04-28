use soroban_sdk::{contracttype, Address, BytesN};

/// Storage keys used by the factory contract.
#[contracttype]
#[derive(Clone)]
/// Storage keys for the factory contract.
pub enum DataKey {
    /// The contract owner.
    Owner,
    /// The contract admin.
    Admin,
    /// The contract operator.
    Operator,
    /// The auction contract address.
    AuctionContract,
    /// The core resolver contract address.
    CoreContract,
    /// A username record mapping.
    Username(BytesN<32>),
    /// Deployment configuration.
    Config,
}

/// A record representing a deployed username and its associated metadata.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
/// A record representing a deployed username.
pub struct UsernameRecord {
    /// SHA-256 hash of the username.
    pub username_hash: BytesN<32>,
    /// Address of the username owner.
    pub owner: Address,
    /// Ledger timestamp when the username was registered.
    pub registered_at: u64,
    /// Address of the core resolver contract.
    pub core_contract: Address,
}

/// Configuration used when deploying new username contracts.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
/// Deployment configuration for the factory.
pub struct DeployConfig {
    /// WASM hash of the core contract.
    pub core_contract_wasm_hash: BytesN<32>,
    /// Admin address for the deployment.
    pub admin: Address,
}
