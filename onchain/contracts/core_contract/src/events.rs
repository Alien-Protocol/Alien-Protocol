#![allow(dead_code)]

use soroban_sdk::{symbol_short, Address, Bytes, BytesN, Env, Symbol};
use crate::types::{ChainType, PrivacyMode};

pub const EVENT_INIT: Symbol = symbol_short!("INIT");
pub const EVENT_TRANSFER: Symbol = symbol_short!("TRANSFER");
pub const EVENT_REGISTER: Symbol = symbol_short!("REGISTER");
pub const EVENT_ROOT_UPD: Symbol = symbol_short!("ROOT_UPD");
pub const EVENT_ADMIN_SET: Symbol = symbol_short!("ADMIN_SET");
pub const EVENT_OPER_SET: Symbol = symbol_short!("OPER_SET");
pub const EVENT_ADDR_ADD: Symbol = symbol_short!("ADDR_ADD");
pub const EVENT_CHAIN_ADD: Symbol = symbol_short!("CHAIN_ADD");
pub const EVENT_CHAIN_REM: Symbol = symbol_short!("CHAIN_REM");
pub const EVENT_ROLE_GNT: Symbol = symbol_short!("ROLE_GNT");
pub const EVENT_ROLE_RVN: Symbol = symbol_short!("ROLE_RVN");
pub const EVENT_PRIVACY: Symbol = symbol_short!("PRIVACY");
pub const EVENT_SHIELD_ADD: Symbol = symbol_short!("SHL_ADD");
pub const EVENT_USERNAME: Symbol = symbol_short!("USERNAME");
pub const EVENT_STELLAR_ADD: Symbol = symbol_short!("STLR_ADD");
pub const EVENT_STELLAR_REM: Symbol = symbol_short!("STLR_REM");
pub const EVENT_DLG_GNT: Symbol = symbol_short!("DLG_GNT");
pub const EVENT_DLG_RVN: Symbol = symbol_short!("DLG_RVN");

pub fn emit_init(env: &Env, owner: &Address) {
    #[allow(deprecated)]
    env.events().publish((EVENT_INIT,), owner.clone());
}

pub fn emit_transfer(env: &Env, commitment: &BytesN<32>, from: &Address, to: &Address) {
    #[allow(deprecated)]
    env.events().publish((EVENT_TRANSFER,), (commitment.clone(), from.clone(), to.clone()));
}

pub fn emit_register(env: &Env, commitment: &BytesN<32>, owner: &Address) {
    #[allow(deprecated)]
    env.events().publish((EVENT_REGISTER,), (commitment.clone(), owner.clone()));
}

pub fn emit_root_updated(env: &Env, old_root: Option<&BytesN<32>>, new_root: &BytesN<32>) {
    #[allow(deprecated)]
    env.events().publish((EVENT_ROOT_UPD,), (old_root.cloned(), new_root.clone()));
}

pub fn emit_admin_set(env: &Env, admin: &Address) {
    #[allow(deprecated)]
    env.events().publish((EVENT_ADMIN_SET,), admin.clone());
}

pub fn emit_operator_set(env: &Env, operator: &Address) {
    #[allow(deprecated)]
    env.events().publish((EVENT_OPER_SET,), operator.clone());
}

pub fn emit_addr_added(env: &Env, address: &Address) {
    #[allow(deprecated)]
    env.events().publish((EVENT_ADDR_ADD,), address.clone());
}

pub fn emit_chain_added(env: &Env, username_hash: &BytesN<32>, chain: &ChainType, address: &Bytes) {
    #[allow(deprecated)]
    env.events().publish((EVENT_CHAIN_ADD,), (username_hash.clone(), chain.clone(), address.clone()));
}

pub fn emit_chain_removed(env: &Env, username_hash: &BytesN<32>, chain: &ChainType) {
    #[allow(deprecated)]
    env.events().publish((EVENT_CHAIN_REM,), (username_hash.clone(), chain.clone()));
}

pub fn emit_role_granted(env: &Env, role: &Symbol, account: &Address) {
    #[allow(deprecated)]
    env.events().publish((EVENT_ROLE_GNT,), (role.clone(), account.clone()));
}

pub fn emit_role_revoked(env: &Env, role: &Symbol, account: &Address) {
    #[allow(deprecated)]
    env.events().publish((EVENT_ROLE_RVN,), (role.clone(), account.clone()));
}

pub fn emit_privacy_set(env: &Env, username_hash: &BytesN<32>, mode: &PrivacyMode) {
    #[allow(deprecated)]
    env.events().publish((EVENT_PRIVACY,), (username_hash.clone(), mode.clone()));
}

pub fn emit_shielded_add(env: &Env, username_hash: &BytesN<32>, address_commitment: &BytesN<32>) {
    #[allow(deprecated)]
    env.events().publish((EVENT_SHIELD_ADD,), (username_hash.clone(), address_commitment.clone()));
}

pub fn emit_username_reg(env: &Env, commitment: &BytesN<32>) {
    #[allow(deprecated)]
    env.events().publish((EVENT_USERNAME,), commitment);
}

pub fn emit_stellar_add(env: &Env, address: &Address) {
    #[allow(deprecated)]
    env.events().publish((EVENT_STELLAR_ADD,), address.clone());
}

pub fn emit_stellar_rem(env: &Env, username_hash: &BytesN<32>, address: &Address) {
    #[allow(deprecated)]
    env.events().publish((EVENT_STELLAR_REM,), (username_hash.clone(), address.clone()));
}

pub fn emit_delegate_granted(
    env: &Env,
    username_hash: &BytesN<32>,
    delegate: &Address,
    permissions: u32,
) {
    #[allow(deprecated)]
    env.events().publish(
        (EVENT_DLG_GNT,),
        (username_hash.clone(), delegate.clone(), permissions),
    );
}

pub fn emit_delegate_revoked(env: &Env, username_hash: &BytesN<32>, delegate: &Address) {
    #[allow(deprecated)]
    env.events().publish((EVENT_DLG_RVN,), (username_hash.clone(), delegate.clone()));
}
