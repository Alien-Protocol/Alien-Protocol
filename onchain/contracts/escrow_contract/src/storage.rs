use crate::errors::EscrowError;
use crate::types::{AutoPay, DataKey, LegacyVault, ScheduledPayment, VaultConfig, VaultState};
use soroban_sdk::{Address, BytesN, Env};

pub(crate) const PERSISTENT_BUMP_AMOUNT: u32 = 518_400;
pub(crate) const PERSISTENT_LIFETIME_THRESHOLD: u32 = 120_960;

pub fn read_vault_config(env: &Env, commitment: &BytesN<32>) -> Option<VaultConfig> {
    let storage = env.storage().persistent();
    if let Some(config) = storage.get(&DataKey::VaultConfig(commitment.clone())) {
        return Some(config);
    }
    let legacy: LegacyVault = storage.get(&DataKey::Vault(commitment.clone()))?;
    Some(VaultConfig {
        owner: legacy.owner,
        token: legacy.token,
        created_at: legacy.created_at,
    })
}

pub fn write_vault_config(env: &Env, commitment: &BytesN<32>, config: &VaultConfig) {
    let key = DataKey::VaultConfig(commitment.clone());
    env.storage().persistent().set(&key, config);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn read_vault_state(env: &Env, commitment: &BytesN<32>) -> Option<VaultState> {
    let storage = env.storage().persistent();
    if let Some(state) = storage.get(&DataKey::VaultState(commitment.clone())) {
        return Some(state);
    }
    let legacy: LegacyVault = storage.get(&DataKey::Vault(commitment.clone()))?;
    Some(VaultState {
        balance: legacy.balance,
        is_active: legacy.is_active,
    })
}

pub fn write_vault_state(env: &Env, commitment: &BytesN<32>, state: &VaultState) {
    let key = DataKey::VaultState(commitment.clone());
    env.storage().persistent().set(&key, state);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn increment_payment_id(env: &Env) -> Result<u32, EscrowError> {
    let id: u32 = env
        .storage()
        .instance()
        .get(&DataKey::PaymentCounter)
        .unwrap_or(0);

    let next = id
        .checked_add(1)
        .ok_or(EscrowError::PaymentCounterOverflow)?;

    env.storage()
        .instance()
        .set(&DataKey::PaymentCounter, &next);

    Ok(id)
}

pub fn read_registration_contract(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::RegistrationContract)
}

pub fn write_registration_contract(env: &Env, address: &Address) {
    env.storage()
        .instance()
        .set(&DataKey::RegistrationContract, address);
}

pub fn write_scheduled_payment(env: &Env, id: u32, payment: &ScheduledPayment) {
    let key = DataKey::ScheduledPayment(id);
    env.storage().persistent().set(&key, payment);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn increment_auto_pay_id(env: &Env) -> Result<u32, EscrowError> {
    let id: u32 = env
        .storage()
        .instance()
        .get(&DataKey::AutoPayCounter)
        .unwrap_or(0);

    let next = id
        .checked_add(1)
        .ok_or(EscrowError::AutoPayCounterOverflow)?;

    env.storage()
        .instance()
        .set(&DataKey::AutoPayCounter, &next);

    Ok(id)
}

pub fn read_auto_pay_count(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::AutoPayCounter)
        .unwrap_or(0)
}

pub fn write_auto_pay(env: &Env, commitment: &BytesN<32>, rule_id: u32, auto_pay: &AutoPay) {
    let key = DataKey::AutoPay(commitment.clone(), rule_id as u64);
    env.storage().persistent().set(&key, auto_pay);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn read_auto_pay(env: &Env, commitment: &BytesN<32>, rule_id: u32) -> Option<AutoPay> {
    env.storage()
        .persistent()
        .get(&DataKey::AutoPay(commitment.clone(), rule_id as u64))
}

pub fn delete_auto_pay(env: &Env, from: &BytesN<32>, rule_id: u32) {
    let key = DataKey::AutoPay(from.clone(), rule_id as u64);
    env.storage().persistent().remove(&key);
}

pub fn read_escrow_admin(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::EscrowAdmin)
}

pub fn write_escrow_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::EscrowAdmin, admin);
}

pub fn read_paused(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&DataKey::Paused)
        .unwrap_or(false)
}

pub fn write_paused(env: &Env, paused: bool) {
    env.storage().instance().set(&DataKey::Paused, &paused);
}
