use shared::storage as shared_storage;
use soroban_sdk::{contracttype, Address, Bytes, BytesN, Env, Vec};

use crate::errors::{ChainAddressError, CoreError};
use crate::events::{
    emit_chain_added, emit_chain_removed, emit_shielded_add, emit_stellar_add, emit_stellar_rem,
};
use crate::registration::{DataKey as CommitmentKey, Registration};
use crate::storage;
use crate::types::{ChainType, Permission};

#[contracttype]
#[derive(Clone)]
pub enum ChainAddrKey {
    ChainAddress(BytesN<32>, ChainType),
}

pub struct AddressManager;

impl AddressManager {
    pub fn add_chain_address(
        e: Env,
        caller: Address,
        username_hash: BytesN<32>,
        chain: ChainType,
        address: Bytes,
    ) -> Result<(), ChainAddressError> {
        caller.require_auth();

        let owner_key = CommitmentKey::Commitment(username_hash.clone());
        let owner: Address = e
            .storage()
            .persistent()
            .get(&owner_key)
            .ok_or(ChainAddressError::NotRegistered)?;

        if owner != caller
            && !storage::has_permission(&e, &username_hash, &caller, Permission::AddChainAddress)
        {
            return Err(ChainAddressError::Unauthorized);
        }

        if !Self::validate_address(&chain, &address) {
            return Err(ChainAddressError::InvalidAddress);
        }

        let key = ChainAddrKey::ChainAddress(username_hash.clone(), chain.clone());
        shared_storage::set_persistent(&e, &key, &address);

        emit_chain_added(&e, &username_hash, &chain, &address);
        Ok(())
    }

    pub fn get_chain_address(
        e: Env,
        username_hash: BytesN<32>,
        chain: ChainType,
    ) -> Option<Bytes> {
        let key = ChainAddrKey::ChainAddress(username_hash, chain);
        shared_storage::get_persistent(&e, &key)
    }

    pub fn remove_chain_address(
        e: Env,
        caller: Address,
        username_hash: BytesN<32>,
        chain: ChainType,
    ) -> Result<(), ChainAddressError> {
        caller.require_auth();

        let owner_key = CommitmentKey::Commitment(username_hash.clone());
        let owner: Address = e
            .storage()
            .persistent()
            .get(&owner_key)
            .ok_or(ChainAddressError::NotRegistered)?;

        if owner != caller
            && !storage::has_permission(
                &e,
                &username_hash,
                &caller,
                Permission::RemoveChainAddress,
            )
        {
            return Err(ChainAddressError::Unauthorized);
        }

        let key = ChainAddrKey::ChainAddress(username_hash.clone(), chain.clone());
        e.storage().persistent().remove(&key);

        emit_chain_removed(&e, &username_hash, &chain);
        Ok(())
    }

    pub fn add_stellar_address(
        e: Env,
        caller: Address,
        username_hash: BytesN<32>,
        stellar_address: Address,
    ) -> Result<(), CoreError> {
        caller.require_auth();

        let owner =
            Registration::get_owner(e.clone(), username_hash.clone()).ok_or(CoreError::NotFound)?;

        if owner != caller
            && !storage::has_permission(
                &e,
                &username_hash,
                &caller,
                Permission::AddStellarAddress,
            )
        {
            return Err(CoreError::Unauthorized);
        }

        let addresses_key = storage::DataKey::StellarAddresses(username_hash.clone());
        let mut linked_addresses: Vec<Address> =
            shared_storage::get_persistent(&e, &addresses_key).unwrap_or_else(|| Vec::new(&e));
        linked_addresses.push_back(stellar_address.clone());
        shared_storage::set_persistent(&e, &addresses_key, &linked_addresses);

        let primary_key = storage::DataKey::StellarAddress(username_hash);
        shared_storage::set_persistent(&e, &primary_key, &stellar_address);

        emit_stellar_add(&e, &stellar_address);
        Ok(())
    }

    pub fn remove_stellar_address(
        e: Env,
        caller: Address,
        username_hash: BytesN<32>,
        stellar_address: Address,
    ) -> Result<(), CoreError> {
        caller.require_auth();

        let owner =
            Registration::get_owner(e.clone(), username_hash.clone()).ok_or(CoreError::NotFound)?;

        if owner != caller
            && !storage::has_permission(
                &e,
                &username_hash,
                &caller,
                Permission::RemoveStellarAddress,
            )
        {
            return Err(CoreError::Unauthorized);
        }

        let addresses_key = storage::DataKey::StellarAddresses(username_hash.clone());
        let existing: Vec<Address> =
            shared_storage::get_persistent(&e, &addresses_key).unwrap_or_else(|| Vec::new(&e));

        let mut updated: Vec<Address> = Vec::new(&e);
        for addr in existing.iter() {
            if addr != stellar_address {
                updated.push_back(addr);
            }
        }
        shared_storage::set_persistent(&e, &addresses_key, &updated);

        let primary_key = storage::DataKey::StellarAddress(username_hash.clone());
        let primary: Option<Address> = shared_storage::get_persistent(&e, &primary_key);

        if let Some(p) = primary {
            if p == stellar_address {
                if updated.is_empty() {
                    e.storage().persistent().remove(&primary_key);
                } else {
                    let last = updated
                        .get(updated.len() - 1)
                        .expect("updated stellar address list should be non-empty");
                    shared_storage::set_persistent(&e, &primary_key, &last);
                }
            }
        }

        emit_stellar_rem(&e, &username_hash, &stellar_address);
        Ok(())
    }

    pub fn get_stellar_addresses(
        e: Env,
        username_hash: BytesN<32>,
    ) -> Result<Vec<Address>, CoreError> {
        if Registration::get_owner(e.clone(), username_hash.clone()).is_none() {
            return Err(CoreError::NotFound);
        }

        Ok(shared_storage::get_persistent::<storage::DataKey, Vec<Address>>(
            &e,
            &storage::DataKey::StellarAddresses(username_hash),
        )
        .unwrap_or_else(|| Vec::new(&e)))
    }

    pub fn resolve_stellar(e: Env, username_hash: BytesN<32>) -> Result<Address, CoreError> {
        if Registration::get_owner(e.clone(), username_hash.clone()).is_none() {
            return Err(CoreError::NotFound);
        }

        shared_storage::get_persistent(&e, &storage::DataKey::StellarAddress(username_hash))
            .ok_or(CoreError::NoAddressLinked)
    }

    pub fn add_shielded_address(
        e: Env,
        caller: Address,
        username_hash: BytesN<32>,
        address_commitment: BytesN<32>,
    ) -> Result<(), CoreError> {
        caller.require_auth();
        let owner =
            Registration::get_owner(e.clone(), username_hash.clone()).ok_or(CoreError::NotFound)?;
        if owner != caller {
            return Err(CoreError::Unauthorized);
        }
        storage::set_shielded_address(&e, &username_hash, &address_commitment);
        emit_shielded_add(&e, &username_hash, &address_commitment);
        Ok(())
    }

    pub fn get_shielded_address(e: Env, username_hash: BytesN<32>) -> Option<BytesN<32>> {
        storage::get_shielded_address(&e, &username_hash)
    }

    pub fn is_shielded(e: Env, username_hash: BytesN<32>) -> bool {
        storage::has_shielded_address(&e, &username_hash)
    }

    fn validate_address(chain: &ChainType, address: &Bytes) -> bool {
        let len = address.len();
        match chain {
            ChainType::Evm => {
                len == 42 && address.get(0) == Some(0x30) && address.get(1) == Some(0x78)
            }
            ChainType::Bitcoin => (25..=62).contains(&len),
            ChainType::Solana => (32..=44).contains(&len),
            ChainType::Cosmos => (39..=45).contains(&len),
        }
    }
}
