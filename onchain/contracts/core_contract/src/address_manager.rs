use soroban_sdk::{contracttype, panic_with_error, Address, Bytes, BytesN, Env, Vec};

use crate::errors::{ChainAddressError, CoreError};
use crate::events::{shielded_add_event, stellar_rem_event, ADDR_ADD, CHAIN_ADD, CHAIN_REM};
use crate::registration::{DataKey as CommitmentKey, Registration};
use crate::storage::{self, PERSISTENT_BUMP_AMOUNT, PERSISTENT_LIFETIME_THRESHOLD};
use crate::types::{ChainType, Permission};

#[contracttype]
#[derive(Clone)]
pub enum ChainAddrKey {
    ChainAddress(BytesN<32>, ChainType),
}

pub struct AddressManager;

impl AddressManager {
    pub fn add_chain_address(
        env: Env,
        caller: Address,
        username_hash: BytesN<32>,
        chain: ChainType,
        address: Bytes,
    ) {
        caller.require_auth();

        let owner_key = CommitmentKey::Commitment(username_hash.clone());
        let owner: Address = env
            .storage()
            .persistent()
            .get(&owner_key)
            .unwrap_or_else(|| panic_with_error!(&env, ChainAddressError::NotRegistered));

        if owner != caller
            && !storage::has_permission(&env, &username_hash, &caller, Permission::AddChainAddress)
        {
            panic_with_error!(&env, ChainAddressError::Unauthorized);
        }

        if !Self::validate_address(&chain, &address) {
            panic_with_error!(&env, ChainAddressError::InvalidAddress);
        }

        let key = ChainAddrKey::ChainAddress(username_hash.clone(), chain.clone());
        env.storage().persistent().set(&key, &address);
        env.storage().persistent().extend_ttl(
            &key,
            PERSISTENT_LIFETIME_THRESHOLD,
            PERSISTENT_BUMP_AMOUNT,
        );

        #[allow(deprecated)]
        env.events()
            .publish((CHAIN_ADD,), (username_hash, chain, address));
    }

    pub fn get_chain_address(
        env: Env,
        username_hash: BytesN<32>,
        chain: ChainType,
    ) -> Option<Bytes> {
        let key = ChainAddrKey::ChainAddress(username_hash, chain);
        env.storage().persistent().get(&key)
    }

    pub fn remove_chain_address(
        env: Env,
        caller: Address,
        username_hash: BytesN<32>,
        chain: ChainType,
    ) {
        caller.require_auth();

        let owner_key = CommitmentKey::Commitment(username_hash.clone());
        let owner: Address = env
            .storage()
            .persistent()
            .get(&owner_key)
            .unwrap_or_else(|| panic_with_error!(&env, ChainAddressError::NotRegistered));

        if owner != caller
            && !storage::has_permission(
                &env,
                &username_hash,
                &caller,
                Permission::RemoveChainAddress,
            )
        {
            panic_with_error!(&env, ChainAddressError::Unauthorized);
        }

        let key = ChainAddrKey::ChainAddress(username_hash.clone(), chain.clone());
        env.storage().persistent().remove(&key);

        #[allow(deprecated)]
        env.events().publish((CHAIN_REM,), (username_hash, chain));
    }

    pub fn add_stellar_address(
        env: Env,
        caller: Address,
        username_hash: BytesN<32>,
        stellar_address: Address,
    ) {
        caller.require_auth();

        let owner = Registration::get_owner(env.clone(), username_hash.clone())
            .unwrap_or_else(|| panic_with_error!(&env, CoreError::NotFound));

        if owner != caller
            && !storage::has_permission(
                &env,
                &username_hash,
                &caller,
                Permission::AddStellarAddress,
            )
        {
            panic_with_error!(&env, CoreError::Unauthorized);
        }

        let mut linked_addresses: Vec<Address> = env
            .storage()
            .persistent()
            .get(&storage::DataKey::StellarAddresses(username_hash.clone()))
            .unwrap_or_else(|| Vec::new(&env));
        linked_addresses.push_back(stellar_address.clone());
        env.storage().persistent().set(
            &storage::DataKey::StellarAddresses(username_hash.clone()),
            &linked_addresses,
        );

        env.storage().persistent().set(
            &storage::DataKey::StellarAddress(username_hash),
            &stellar_address,
        );

        #[allow(deprecated)]
        env.events().publish((ADDR_ADD,), stellar_address.clone());
    }

    pub fn remove_stellar_address(
        env: Env,
        caller: Address,
        username_hash: BytesN<32>,
        stellar_address: Address,
    ) {
        caller.require_auth();

        let owner = Registration::get_owner(env.clone(), username_hash.clone())
            .unwrap_or_else(|| panic_with_error!(&env, CoreError::NotFound));

        if owner != caller
            && !storage::has_permission(
                &env,
                &username_hash,
                &caller,
                Permission::RemoveStellarAddress,
            )
        {
            panic_with_error!(&env, CoreError::Unauthorized);
        }

        let existing: Vec<Address> = env
            .storage()
            .persistent()
            .get(&storage::DataKey::StellarAddresses(username_hash.clone()))
            .unwrap_or_else(|| Vec::new(&env));

        let mut updated: Vec<Address> = Vec::new(&env);
        for addr in existing.iter() {
            if addr != stellar_address {
                updated.push_back(addr);
            }
        }
        env.storage().persistent().set(
            &storage::DataKey::StellarAddresses(username_hash.clone()),
            &updated,
        );

        let primary: Option<Address> = env
            .storage()
            .persistent()
            .get(&storage::DataKey::StellarAddress(username_hash.clone()));

        if let Some(p) = primary {
            if p == stellar_address {
                if updated.is_empty() {
                    env.storage()
                        .persistent()
                        .remove(&storage::DataKey::StellarAddress(username_hash.clone()));
                } else {
                    let last = updated
                        .get(updated.len() - 1)
                        .expect("updated stellar address list should be non-empty");
                    env.storage().persistent().set(
                        &storage::DataKey::StellarAddress(username_hash.clone()),
                        &last,
                    );
                }
            }
        }

        #[allow(deprecated)]
        env.events()
            .publish((stellar_rem_event(&env),), (username_hash, stellar_address));
    }

    pub fn get_stellar_addresses(env: Env, username_hash: BytesN<32>) -> Vec<Address> {
        if Registration::get_owner(env.clone(), username_hash.clone()).is_none() {
            panic_with_error!(&env, CoreError::NotFound);
        }

        env.storage()
            .persistent()
            .get::<storage::DataKey, Vec<Address>>(&storage::DataKey::StellarAddresses(
                username_hash,
            ))
            .unwrap_or_else(|| Vec::new(&env))
    }

    pub fn resolve_stellar(env: Env, username_hash: BytesN<32>) -> Address {
        if Registration::get_owner(env.clone(), username_hash.clone()).is_none() {
            panic_with_error!(&env, CoreError::NotFound);
        }

        env.storage()
            .persistent()
            .get::<storage::DataKey, Address>(&storage::DataKey::StellarAddress(username_hash))
            .unwrap_or_else(|| panic_with_error!(&env, CoreError::NoAddressLinked))
    }

    pub fn add_shielded_address(
        env: Env,
        caller: Address,
        username_hash: BytesN<32>,
        address_commitment: BytesN<32>,
    ) {
        caller.require_auth();
        let owner = Registration::get_owner(env.clone(), username_hash.clone())
            .unwrap_or_else(|| panic_with_error!(&env, CoreError::NotFound));
        if owner != caller {
            panic_with_error!(&env, CoreError::Unauthorized);
        }
        storage::set_shielded_address(&env, &username_hash, &address_commitment);
        #[allow(deprecated)]
        env.events().publish(
            (shielded_add_event(&env),),
            (username_hash, address_commitment),
        );
    }

    pub fn get_shielded_address(env: Env, username_hash: BytesN<32>) -> Option<BytesN<32>> {
        storage::get_shielded_address(&env, &username_hash)
    }

    pub fn is_shielded(env: Env, username_hash: BytesN<32>) -> bool {
        storage::has_shielded_address(&env, &username_hash)
    }

    /// Validates the format of an address based on the chain type.
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
