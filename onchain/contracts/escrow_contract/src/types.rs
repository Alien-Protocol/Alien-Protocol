use soroban_sdk::{contracttype, Address, BytesN};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    VaultConfig(BytesN<32>),
    VaultState(BytesN<32>),
    ScheduledPayment(u32),
    PaymentCounter,
    AutoPay(BytesN<32>, u64),
    AutoPayCounter,
    Vault(BytesN<32>),
    RegistrationContract,
    EscrowAdmin,
    Paused,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VaultConfig {
    pub owner: Address,
    pub token: Address,
    pub created_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VaultState {
    pub balance: i128,
    pub is_active: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduledPayment {
    pub from: BytesN<32>,
    pub to: BytesN<32>,
    pub token: Address,
    pub amount: i128,
    pub release_at: u64,
    pub executed: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyVault {
    pub owner: Address,
    pub token: Address,
    pub created_at: u64,
    pub balance: i128,
    pub is_active: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutoPay {
    pub from: BytesN<32>,
    pub to: BytesN<32>,
    pub token: Address,
    pub amount: i128,
    pub interval: u64,
    pub last_paid: u64,
}
