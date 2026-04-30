use soroban_sdk::{contracttype, Address, Bytes, BytesN, Symbol};

#[contracttype]
#[derive(Clone)]
pub struct AddressMetadata {
    pub label: Symbol,
}

#[contracttype]
#[derive(Clone)]
pub struct ResolveData {
    pub wallet: Address,
    pub memo: Option<u64>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChainType {
    Evm,
    Bitcoin,
    Solana,
    Cosmos,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrivacyMode {
    Normal,
    Shielded,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Permission {
    SetMemo = 1,
    SetPrivacyMode = 2,
    AddChainAddress = 3,
    RemoveChainAddress = 4,
    AddStellarAddress = 5,
    RemoveStellarAddress = 6,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PermissionSet {
    pub permissions: soroban_sdk::Vec<Permission>,
}

pub type Proof = Bytes;

#[contracttype]
#[derive(Clone)]
pub struct PublicSignals {
    pub commitment: BytesN<32>,
    pub old_root: BytesN<32>,
    pub new_root: BytesN<32>,
}
