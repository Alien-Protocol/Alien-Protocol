#![no_std]

pub mod address_manager;
pub mod admin;
pub mod alien_gateway;
pub mod errors;
pub mod events;
pub mod registration;
pub mod resolver;
pub mod smt_root;
pub mod storage;
pub mod transfer;
pub mod types;
pub mod zk_verifier;

#[cfg(test)]
mod test;

use address_manager::AddressManager;
use admin::Admin;
use registration::Registration;
use resolver::Resolver;
use soroban_sdk::{contract, contractimpl, Address, Bytes, BytesN, Env, Symbol};
use transfer::Transfer;
use types::{ChainType, PermissionSet, PrivacyMode, Proof, PublicSignals};

#[contract]
pub struct Contract;

#[rustfmt::skip]
#[contractimpl]
impl Contract {
    pub fn initialize(e: Env, o: Address) { Admin::initialize(e, o) }

    pub fn get_contract_owner(e: Env) -> Address { Admin::get_contract_owner(e) }

    pub fn get_admin(e: Env) -> Address { Admin::get_admin(e) }

    pub fn get_operator(e: Env) -> Address { Admin::get_operator(e) }

    pub fn set_admin(e: Env, a: Address) { Admin::set_admin(e, a) }

    pub fn set_operator(e: Env, o: Address) { Admin::set_operator(e, o) }

    pub fn get_smt_root(e: Env) -> BytesN<32> { Admin::get_smt_root(e) }

    pub fn update_smt_root(e: Env, r: BytesN<32>) { Admin::update_smt_root(e, r) }

    pub fn submit_proof(e: Env, c: Address, p: Proof, s: PublicSignals) { Registration::submit_proof(e, c, p, s) }

    pub fn register_resolver(e: Env, c: Address, h: BytesN<32>, p: Proof, s: PublicSignals) { Resolver::register_resolver(e, c, h, p, s); }

    pub fn set_memo(e: Env, c: Address, cm: BytesN<32>, m: u64) { Resolver::set_memo(e, c, cm, m) }

    pub fn set_privacy_mode(e: Env, c: Address, h: BytesN<32>, m: PrivacyMode) { Resolver::set_privacy_mode(e, c, h, m); }

    pub fn get_privacy_mode(e: Env, h: BytesN<32>) -> PrivacyMode { Resolver::get_privacy_mode(e, h) }

    pub fn resolve(e: Env, c: BytesN<32>) -> (Address, Option<u64>) { Resolver::resolve(e, c) }

    pub fn register(e: Env, c: Address, h: BytesN<32>) { Registration::register(e, c, h) }

    pub fn get_owner(e: Env, h: BytesN<32>) -> Option<Address> { Registration::get_owner(e, h) }

    pub fn get_username(e: Env) -> Option<Symbol> { e.storage().instance().get(&alien_gateway::storage::username_key(&e)) }

    pub fn get_created_at(e: Env, h: BytesN<32>) -> Option<u64> { Registration::get_created_at(e, h) }

    pub fn add_chain_address(e: Env, c: Address, h: BytesN<32>, t: ChainType, a: Bytes) { AddressManager::add_chain_address(e, c, h, t, a); }

    pub fn get_chain_address(e: Env, h: BytesN<32>, t: ChainType) -> Option<Bytes> { AddressManager::get_chain_address(e, h, t) }

    pub fn remove_chain_address(e: Env, c: Address, h: BytesN<32>, t: ChainType) { AddressManager::remove_chain_address(e, c, h, t); }

    pub fn add_stellar_address(e: Env, c: Address, h: BytesN<32>, a: Address) { AddressManager::add_stellar_address(e, c, h, a); }

    pub fn remove_stellar_address(e: Env, c: Address, h: BytesN<32>, a: Address) { AddressManager::remove_stellar_address(e, c, h, a); }

    pub fn get_stellar_addresses(e: Env, h: BytesN<32>) -> soroban_sdk::Vec<Address> { AddressManager::get_stellar_addresses(e, h) }

    pub fn resolve_stellar(e: Env, h: BytesN<32>) -> Address { AddressManager::resolve_stellar(e, h) }

    pub fn transfer_ownership(e: Env, c: Address, h: BytesN<32>, n: Address) { Transfer::transfer_ownership(e, c, h, n); }

    pub fn transfer(e: Env, c: Address, h: BytesN<32>, n: Address, p: Proof, s: PublicSignals) { Transfer::transfer(e, c, h, n, p, s); }

    pub fn add_shielded_address(e: Env, c: Address, h: BytesN<32>, a: BytesN<32>) { AddressManager::add_shielded_address(e, c, h, a); }

    pub fn get_shielded_address(e: Env, h: BytesN<32>) -> Option<BytesN<32>> { AddressManager::get_shielded_address(e, h) }

    pub fn is_shielded(e: Env, h: BytesN<32>) -> bool { AddressManager::is_shielded(e, h) }

    pub fn grant_delegate(e: Env, o: Address, h: BytesN<32>, d: Address, p: PermissionSet) {
        Registration::grant_delegate(e, o, h, d, p);
    }

    pub fn revoke_delegate(e: Env, o: Address, h: BytesN<32>, d: Address) {
        Registration::revoke_delegate(e, o, h, d);
    }
}
