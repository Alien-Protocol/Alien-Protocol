#![no_std]

mod errors;
mod events;
mod storage;
mod types;

#[cfg(test)]
mod test;

use soroban_sdk::{
    contract, contractclient, contractimpl, token::TokenClient, Address, BytesN, Env, String,
    Symbol, Vec,
};

use crate::errors::CoreError;
use crate::events::{
    emit_escrow_created, emit_escrow_refunded, emit_escrow_released, emit_ownership_transferred,
    emit_primary_set, emit_send, emit_wallet_added, emit_wallet_removed,
};
use crate::storage::{
    get_escrow, get_escrow_counter, get_owner, get_primary_address, get_username_hash, get_wallet,
    get_wallet_labels, has_wallet, remove_wallet, set_escrow, set_escrow_counter, set_owner,
    set_primary_address, set_username_hash, set_wallet, set_wallet_labels,
};
use crate::types::{ChainType, EscrowRecord, EscrowStatus, WalletEntry};

const MAX_WALLETS: u32 = 20;

#[contractclient(name = "FactoryClient")]
pub trait FactoryInterface {
    fn core_contract(env: Env, username_hash: BytesN<32>) -> Option<Address>;
}

#[contractclient(name = "PeerCoreClient")]
pub trait PeerCoreInterface {
    fn resolve(env: Env) -> Result<Address, CoreError>;
}

#[contract]
pub struct CoreContract;

#[contractimpl]
impl CoreContract {
    pub fn __constructor(env: Env, owner: Address, username_hash: BytesN<32>) {
        set_owner(&env, &owner);
        set_username_hash(&env, &username_hash);
    }

    pub fn set_primary_address(env: Env, address: Address) -> Result<(), CoreError> {
        let owner = get_owner(&env).ok_or(CoreError::Unauthorized)?;
        owner.require_auth();
        set_primary_address(&env, &address);
        emit_primary_set(&env, &owner, &address);
        Ok(())
    }

    pub fn get_primary_address(env: Env) -> Option<Address> {
        get_primary_address(&env)
    }

    pub fn resolve(env: Env) -> Result<Address, CoreError> {
        get_primary_address(&env).ok_or(CoreError::NoAddressLinked)
    }

    pub fn get_username_hash(env: Env) -> Option<BytesN<32>> {
        get_username_hash(&env)
    }

    pub fn get_owner(env: Env) -> Option<Address> {
        get_owner(&env)
    }
    pub fn transfer_ownership(env: Env, new_owner: Address) -> Result<(), CoreError> {
        let owner = get_owner(&env).ok_or(CoreError::Unauthorized)?;
        owner.require_auth();
        if owner == new_owner {
            return Err(CoreError::SameOwner);
        }
        set_owner(&env, &new_owner);
        emit_ownership_transferred(&env, &owner, &new_owner);
        Ok(())
    }

    pub fn add_wallet(
        env: Env,
        label: Symbol,
        address: String,
        chain: ChainType,
    ) -> Result<(), CoreError> {
        let owner = get_owner(&env).ok_or(CoreError::Unauthorized)?;
        owner.require_auth();

        let is_new = !has_wallet(&env, &label);

        if is_new {
            let labels = get_wallet_labels(&env);
            if labels.len() >= MAX_WALLETS {
                return Err(CoreError::WalletLimitReached);
            }
        }

        let entry = WalletEntry {
            label: label.clone(),
            address,
            chain,
            added_at: env.ledger().timestamp(),
        };
        set_wallet(&env, &label, &entry);

        if is_new {
            let mut labels = get_wallet_labels(&env);
            labels.push_back(label.clone());
            set_wallet_labels(&env, &labels);
        }

        emit_wallet_added(&env, &label);
        Ok(())
    }

    pub fn remove_wallet(env: Env, label: Symbol) -> Result<(), CoreError> {
        let owner = get_owner(&env).ok_or(CoreError::Unauthorized)?;
        owner.require_auth();

        if !has_wallet(&env, &label) {
            return Err(CoreError::WalletNotFound);
        }

        remove_wallet(&env, &label);

        let labels = get_wallet_labels(&env);
        let mut new_labels: Vec<Symbol> = Vec::new(&env);
        for lbl in labels.iter() {
            if lbl != label {
                new_labels.push_back(lbl);
            }
        }
        set_wallet_labels(&env, &new_labels);

        emit_wallet_removed(&env, &label);
        Ok(())
    }

    pub fn get_wallet(env: Env, label: Symbol) -> Option<WalletEntry> {
        get_wallet(&env, &label)
    }

    pub fn get_all_wallets(env: Env) -> Vec<WalletEntry> {
        let labels = get_wallet_labels(&env);
        let mut result: Vec<WalletEntry> = Vec::new(&env);
        for label in labels.iter() {
            if let Some(entry) = get_wallet(&env, &label) {
                result.push_back(entry);
            }
        }
        result
    }

    pub fn get_wallet_labels(env: Env) -> Vec<Symbol> {
        get_wallet_labels(&env)
    }
    pub fn send_to_address(
        env: Env,
        asset: Address,
        amount: i128,
        to: Address,
    ) -> Result<(), CoreError> {
        if amount <= 0 {
            return Err(CoreError::InvalidAmount);
        }
        let owner = get_owner(&env).ok_or(CoreError::Unauthorized)?;
        owner.require_auth();

        let token = TokenClient::new(&env, &asset);
        token.transfer(&owner, &to, &amount);

        emit_send(&env, &asset, amount, &to);
        Ok(())
    }

    pub fn send_to_username(
        env: Env,
        factory: Address,
        username_hash: BytesN<32>,
        asset: Address,
        amount: i128,
    ) -> Result<(), CoreError> {
        if amount <= 0 {
            return Err(CoreError::InvalidAmount);
        }
        let owner = get_owner(&env).ok_or(CoreError::Unauthorized)?;
        owner.require_auth();

        let factory_client = FactoryClient::new(&env, &factory);
        let peer_core_addr = factory_client
            .core_contract(&username_hash)
            .ok_or(CoreError::UsernameNotFound)?;

        let peer_core = PeerCoreClient::new(&env, &peer_core_addr);
        let recipient = peer_core.resolve();

        let token = TokenClient::new(&env, &asset);
        token.transfer(&owner, &recipient, &amount);

        emit_send(&env, &asset, amount, &recipient);
        Ok(())
    }

    pub fn create_escrow(
        env: Env,
        asset: Address,
        amount: i128,
        recipient: Address,
        release_at: u64,
        note: String,
    ) -> Result<u32, CoreError> {
        if amount <= 0 {
            return Err(CoreError::InvalidAmount);
        }

        let owner = get_owner(&env).ok_or(CoreError::Unauthorized)?;
        owner.require_auth();

        let id = get_escrow_counter(&env);
        let next_id = id.checked_add(1).ok_or(CoreError::EscrowCounterOverflow)?;
        set_escrow_counter(&env, next_id);

        let token = TokenClient::new(&env, &asset);
        token.transfer(&owner, env.current_contract_address(), &amount);

        let record = EscrowRecord {
            id,
            asset: asset.clone(),
            amount,
            recipient: recipient.clone(),
            release_at,
            status: EscrowStatus::Active,
            created_at: env.ledger().timestamp(),
            note,
        };
        set_escrow(&env, id, &record);

        emit_escrow_created(&env, id, &asset, amount, &recipient, release_at);
        Ok(id)
    }

    pub fn release_escrow(env: Env, id: u32) -> Result<(), CoreError> {
        let mut record = get_escrow(&env, id).ok_or(CoreError::NotFound)?;

        if record.status != EscrowStatus::Active {
            return Err(CoreError::EscrowAlreadySettled);
        }
        if env.ledger().timestamp() < record.release_at {
            return Err(CoreError::EscrowNotUnlocked);
        }

        record.status = EscrowStatus::Released;
        set_escrow(&env, id, &record);

        let token = TokenClient::new(&env, &record.asset);
        token.transfer(
            &env.current_contract_address(),
            &record.recipient,
            &record.amount,
        );

        emit_escrow_released(&env, id, &record.recipient, record.amount);
        Ok(())
    }

    pub fn refund_escrow(env: Env, id: u32) -> Result<(), CoreError> {
        let owner = get_owner(&env).ok_or(CoreError::Unauthorized)?;
        owner.require_auth();

        let mut record = get_escrow(&env, id).ok_or(CoreError::NotFound)?;

        if record.status != EscrowStatus::Active {
            return Err(CoreError::EscrowAlreadySettled);
        }

        record.status = EscrowStatus::Refunded;
        set_escrow(&env, id, &record);

        let token = TokenClient::new(&env, &record.asset);
        token.transfer(&env.current_contract_address(), &owner, &record.amount);

        emit_escrow_refunded(&env, id, &owner, record.amount);
        Ok(())
    }

    pub fn get_escrow(env: Env, id: u32) -> Option<EscrowRecord> {
        get_escrow(&env, id)
    }

    pub fn escrow_count(env: Env) -> u32 {
        get_escrow_counter(&env)
    }
}
