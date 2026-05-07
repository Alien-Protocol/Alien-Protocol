use crate::errors::EscrowError;
use crate::types::{AutoPay, DataKey, LegacyVault, ScheduledPayment, VaultConfig, VaultState};
use shared::storage as shared_storage;
use soroban_sdk::{Address, BytesN, Env};

/// The amount of ledger entries to bump persistent storage by.
#[allow(dead_code)]
pub(crate) const PERSISTENT_BUMP_AMOUNT: u32 = 518_400;
/// The threshold for persistent storage TTL to trigger an auto-bump.
#[allow(dead_code)]
pub(crate) const PERSISTENT_LIFETIME_THRESHOLD: u32 = 120_960;

pub fn read_vault_config(env: &Env, commitment: &BytesN<32>) -> Option<VaultConfig> {
    let key = DataKey::VaultConfig(commitment.clone());
    if let Some(config) = shared_storage::get_persistent(env, &key) {
        return Some(config);
    }
    let legacy: LegacyVault =
        shared_storage::get_persistent(env, &DataKey::Vault(commitment.clone()))?;
    Some(VaultConfig {
        owner: legacy.owner,
        token: legacy.token,
        created_at: legacy.created_at,
    })
}

pub fn write_vault_config(env: &Env, commitment: &BytesN<32>, config: &VaultConfig) {
    let key = DataKey::VaultConfig(commitment.clone());
    shared_storage::set_persistent(env, &key, config);
}

pub fn read_vault_state(env: &Env, commitment: &BytesN<32>) -> Option<VaultState> {
    let key = DataKey::VaultState(commitment.clone());
    if let Some(state) = shared_storage::get_persistent(env, &key) {
        return Some(state);
    }
    let legacy: LegacyVault =
        shared_storage::get_persistent(env, &DataKey::Vault(commitment.clone()))?;
    Some(VaultState {
        balance: legacy.balance,
        is_active: legacy.is_active,
    })
}

pub fn write_vault_state(env: &Env, commitment: &BytesN<32>, state: &VaultState) {
    let key = DataKey::VaultState(commitment.clone());
    shared_storage::set_persistent(env, &key, state);
}

pub fn increment_payment_id(env: &Env) -> Result<u32, EscrowError> {
    let id: u32 = shared_storage::get_instance(env, &DataKey::PaymentCounter).unwrap_or(0);

    let next = id
        .checked_add(1)
        .ok_or(EscrowError::PaymentCounterOverflow)?;

    shared_storage::set_instance(env, &DataKey::PaymentCounter, &next);

    Ok(id)
}

pub fn read_registration_contract(env: &Env) -> Option<Address> {
    shared_storage::get_instance(env, &DataKey::RegistrationContract)
}

pub fn write_registration_contract(env: &Env, address: &Address) {
    shared_storage::set_instance(env, &DataKey::RegistrationContract, address);
}

pub fn write_scheduled_payment(env: &Env, id: u32, payment: &ScheduledPayment) {
    let key = DataKey::ScheduledPayment(id);
    shared_storage::set_persistent(env, &key, payment);
}

pub fn increment_auto_pay_id(env: &Env) -> Result<u32, EscrowError> {
    let id: u32 = shared_storage::get_instance(env, &DataKey::AutoPayCounter).unwrap_or(0);

    let next = id
        .checked_add(1)
        .ok_or(EscrowError::AutoPayCounterOverflow)?;

    shared_storage::set_instance(env, &DataKey::AutoPayCounter, &next);

    Ok(id)
}

pub fn read_auto_pay_count(env: &Env) -> u32 {
    shared_storage::get_instance(env, &DataKey::AutoPayCounter).unwrap_or(0)
}

pub fn write_auto_pay(env: &Env, commitment: &BytesN<32>, rule_id: u32, auto_pay: &AutoPay) {
    let key = DataKey::AutoPay(commitment.clone(), rule_id as u64);
    shared_storage::set_persistent(env, &key, auto_pay);
}

pub fn read_auto_pay(env: &Env, commitment: &BytesN<32>, rule_id: u32) -> Option<AutoPay> {
    shared_storage::get_persistent(env, &DataKey::AutoPay(commitment.clone(), rule_id as u64))
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
