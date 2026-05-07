use soroban_sdk::{contracttype, Address, String, Symbol};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Owner,
    UsernameHash,
    PrimaryAddress,
    Wallet(Symbol),
    WalletLabels,
    EscrowCounter,
    Escrow(u32),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChainType {
    Stellar,
    Bitcoin,
    Ethereum,
    Tron,
    Bnb,
    Solana,
    Other,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WalletEntry {
    pub label: Symbol,
    pub address: String,
    pub chain: ChainType,
    pub added_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowStatus {
    Active,
    Released,
    Refunded,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowRecord {
    pub id: u32,
    pub asset: Address,
    pub amount: i128,
    pub recipient: Address,
    pub release_at: u64,
    pub status: EscrowStatus,
    pub created_at: u64,
    pub note: String,
}
