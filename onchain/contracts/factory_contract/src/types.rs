use soroban_sdk::{contracttype, Address, BytesN};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Owner,
    Admin,
    Operator,
    AuctionContract,
    CoreContract(BytesN<32>),
    Username(BytesN<32>),
    Config,
    CoreWasm
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UsernameRecord {
    pub username_hash: BytesN<32>,
    pub owner: Address,
    pub registered_at: u64,
    pub core_contract: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeployConfig {
    pub core_contract_wasm_hash: BytesN<32>,
    pub resolver: Address,
}
