#![no_std]

pub mod errors;
pub mod events;
pub mod storage;
pub mod types;

#[cfg(test)]
mod test;

use crate::errors::EscrowError;
use crate::events::Events;
use crate::storage::{
    delete_auto_pay, increment_auto_pay_id, increment_payment_id, read_auto_pay,
    read_auto_pay_count, read_escrow_admin, read_paused, read_registration_contract,
    read_vault_config, read_vault_state, write_auto_pay, write_escrow_admin, write_paused,
    write_registration_contract, write_scheduled_payment, write_vault_config, write_vault_state,
};
use crate::types::{AutoPay, DataKey, ScheduledPayment, VaultConfig, VaultState};
use shared::auth as shared_auth;
use soroban_sdk::{contract, contractimpl, token, vec, Address, BytesN, Env, IntoVal, Symbol};

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {
    pub fn initialize(
        env: Env,
        admin: Address,
        registration_contract: Address,
    ) -> Result<(), EscrowError> {
        shared_auth::require_address_auth(&admin);
        if read_registration_contract(&env).is_some() {
            return Err(EscrowError::AlreadyInitialized);
        }
        write_registration_contract(&env, &registration_contract);
        write_escrow_admin(&env, &admin);
        Ok(())
    }

    pub fn rotate_admin(env: Env, new_admin: Address) -> Result<(), EscrowError> {
        let old_admin = read_escrow_admin(&env).ok_or(EscrowError::Unauthorized)?;
        shared_auth::require_address_auth(&old_admin);
        write_escrow_admin(&env, &new_admin);
        Events::admin_rotated(&env, old_admin, new_admin);
        Ok(())
    }

    pub fn set_paused(env: Env, paused: bool) -> Result<(), EscrowError> {
        let admin = read_escrow_admin(&env).ok_or(EscrowError::Unauthorized)?;
        shared_auth::require_address_auth(&admin);
        write_paused(&env, paused);
        Events::pause_toggled(&env, paused);
        Ok(())
    }

    pub fn get_admin(env: Env) -> Option<Address> {
        read_escrow_admin(&env)
    }

    pub fn is_paused(env: Env) -> bool {
        read_paused(&env)
    }

    pub fn create_vault(
        env: Env,
        commitment: BytesN<32>,
        token: Address,
    ) -> Result<(), EscrowError> {
        if read_paused(&env) {
            return Err(EscrowError::ContractPaused);
        }

        let registration =
            read_registration_contract(&env).ok_or(EscrowError::CommitmentNotRegistered)?;
        let owner: Option<Address> = env.invoke_contract(
            &registration,
            &Symbol::new(&env, "get_owner"),
            vec![&env, commitment.into_val(&env)],
        );
        let owner = owner.ok_or(EscrowError::CommitmentNotRegistered)?;
        shared_auth::require_address_auth(&owner);

        if read_vault_config(&env, &commitment).is_some() {
            return Err(EscrowError::VaultAlreadyExists);
        }

        write_vault_config(
            &env,
            &commitment,
            &VaultConfig {
                owner: owner.clone(),
                token: token.clone(),
                created_at: env.ledger().timestamp(),
            },
        );

        write_vault_state(
            &env,
            &commitment,
            &VaultState {
                balance: 0,
                is_active: true,
            },
        );

        Events::vault_crt(&env, commitment, token, owner);
        Ok(())
    }

    pub fn deposit(env: Env, commitment: BytesN<32>, amount: i128) -> Result<(), EscrowError> {
        if amount <= 0 {
            return Err(EscrowError::InvalidAmount);
        }
        if read_paused(&env) {
            return Err(EscrowError::ContractPaused);
        }

        let config = read_vault_config(&env, &commitment).ok_or(EscrowError::VaultNotFound)?;
        let mut state = read_vault_state(&env, &commitment).ok_or(EscrowError::VaultNotFound)?;
        require_vault_owner(&config);

        if !state.is_active {
            return Err(EscrowError::VaultInactive);
        }

        let token_client = token::Client::new(&env, &config.token);
        token_client.transfer(&config.owner, &env.current_contract_address(), &amount);

        state.balance = state
            .balance
            .checked_add(amount)
            .ok_or(EscrowError::ArithmeticError)?;
        write_vault_state(&env, &commitment, &state);

        Events::deposit(&env, commitment, amount, state.balance);
        Ok(())
    }

    pub fn withdraw(env: Env, commitment: BytesN<32>, amount: i128) -> Result<(), EscrowError> {
        if amount <= 0 {
            return Err(EscrowError::InvalidAmount);
        }
        if read_paused(&env) {
            return Err(EscrowError::ContractPaused);
        }

        let config = read_vault_config(&env, &commitment).ok_or(EscrowError::VaultNotFound)?;
        let mut state = read_vault_state(&env, &commitment).ok_or(EscrowError::VaultNotFound)?;
        require_vault_owner(&config);

        if !state.is_active {
            return Err(EscrowError::VaultInactive);
        }
        if state.balance < amount {
            return Err(EscrowError::InsufficientBalance);
        }

        let token_client = token::Client::new(&env, &config.token);
        token_client.transfer(&env.current_contract_address(), &config.owner, &amount);

        state.balance = state
            .balance
            .checked_sub(amount)
            .ok_or(EscrowError::ArithmeticError)?;
        write_vault_state(&env, &commitment, &state);

        Events::withdraw(&env, commitment, amount, state.balance);
        Ok(())
    }

    pub fn schedule_payment(
        env: Env,
        from: BytesN<32>,
        to: BytesN<32>,
        amount: i128,
        release_at: u64,
    ) -> Result<u32, EscrowError> {
        if amount <= 0 {
            return Err(EscrowError::InvalidAmount);
        }
        if release_at <= env.ledger().timestamp() {
            return Err(EscrowError::PastReleaseTime);
        }
        if read_paused(&env) {
            return Err(EscrowError::ContractPaused);
        }

        let config = read_vault_config(&env, &from).ok_or(EscrowError::VaultNotFound)?;
        let mut state = read_vault_state(&env, &from).ok_or(EscrowError::VaultNotFound)?;
        require_vault_owner(&config);

        if !state.is_active {
            return Err(EscrowError::VaultInactive);
        }
        if state.balance < amount {
            return Err(EscrowError::InsufficientBalance);
        }

        state.balance = state
            .balance
            .checked_sub(amount)
            .ok_or(EscrowError::ArithmeticError)?;
        write_vault_state(&env, &from, &state);

        let payment_id = increment_payment_id(&env)?;
        let payment = ScheduledPayment {
            from,
            to,
            token: config.token.clone(),
            amount,
            release_at,
            executed: false,
        };
        write_scheduled_payment(&env, payment_id, &payment);

        Events::schedule_pay(
            &env,
            payment_id,
            payment.from,
            payment.to,
            payment.amount,
            payment.release_at,
        );
        Ok(payment_id)
    }

    pub fn execute_scheduled(env: Env, payment_id: u32) -> Result<(), EscrowError> {
        if read_paused(&env) {
            return Err(EscrowError::ContractPaused);
        }

        let key = DataKey::ScheduledPayment(payment_id);
        let mut payment: ScheduledPayment = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(EscrowError::PaymentNotFound)?;

        if payment.executed {
            return Err(EscrowError::PaymentAlreadyExecuted);
        }
        if env.ledger().timestamp() < payment.release_at {
            return Err(EscrowError::PaymentNotYetDue);
        }

        let state = read_vault_state(&env, &payment.from).ok_or(EscrowError::VaultNotFound)?;
        if !state.is_active {
            return Err(EscrowError::VaultInactive);
        }

        let recipient = resolve(&env, &payment.to)?;
        let token_client = token::Client::new(&env, &payment.token);
        token_client.transfer(&env.current_contract_address(), &recipient, &payment.amount);

        payment.executed = true;
        write_scheduled_payment(&env, payment_id, &payment);

        Events::pay_exec(&env, payment_id, payment.from, payment.to, payment.amount);
        Ok(())
    }

    pub fn cancel_vault(env: Env, commitment: BytesN<32>) -> Result<(), EscrowError> {
        let config = read_vault_config(&env, &commitment).ok_or(EscrowError::VaultNotFound)?;
        require_vault_owner(&config);
        let mut state = read_vault_state(&env, &commitment).ok_or(EscrowError::VaultNotFound)?;

        let refunded_amount = if state.balance > 0 {
            let token_client = token::Client::new(&env, &config.token);
            token_client.transfer(
                &env.current_contract_address(),
                &config.owner,
                &state.balance,
            );
            state.balance
        } else {
            0
        };

        state.is_active = false;
        state.balance = 0;
        write_vault_state(&env, &commitment, &state);

        Events::vault_cancel(&env, commitment, refunded_amount);
        Ok(())
    }

    pub fn setup_auto_pay(
        env: Env,
        from: BytesN<32>,
        to: BytesN<32>,
        amount: i128,
        interval: u64,
    ) -> Result<u32, EscrowError> {
        if amount <= 0 {
            return Err(EscrowError::InvalidAmount);
        }
        if interval == 0 {
            return Err(EscrowError::InvalidInterval);
        }
        if from == to {
            return Err(EscrowError::SelfPaymentNotAllowed);
        }

        let config = read_vault_config(&env, &from).ok_or(EscrowError::VaultNotFound)?;
        require_vault_owner(&config);
        let rule_id = increment_auto_pay_id(&env)?;

        let auto_pay = AutoPay {
            from: from.clone(),
            to: to.clone(),
            token: config.token.clone(),
            amount,
            interval,
            last_paid: 0,
        };
        write_auto_pay(&env, &from, rule_id, &auto_pay);

        Events::auto_set(&env, rule_id, from, to, amount, interval);
        Ok(rule_id)
    }

    pub fn cancel_auto_pay(env: Env, from: BytesN<32>, rule_id: u32) -> Result<(), EscrowError> {
        let config = read_vault_config(&env, &from).ok_or(EscrowError::VaultNotFound)?;
        require_vault_owner(&config);

        if read_auto_pay(&env, &from, rule_id).is_none() {
            return Err(EscrowError::AutoPayNotFound);
        }

        delete_auto_pay(&env, &from, rule_id);
        Events::auto_cancel(&env, from, rule_id);
        Ok(())
    }

    pub fn trigger_auto_pay(env: Env, from: BytesN<32>, rule_id: u32) -> Result<(), EscrowError> {
        if read_paused(&env) {
            return Err(EscrowError::ContractPaused);
        }

        let mut auto_pay =
            read_auto_pay(&env, &from, rule_id).ok_or(EscrowError::AutoPayNotFound)?;
        let current_time = env.ledger().timestamp();
        let next_payment_time = auto_pay
            .last_paid
            .checked_add(auto_pay.interval)
            .ok_or(EscrowError::ArithmeticError)?;

        if current_time < next_payment_time {
            return Err(EscrowError::IntervalNotElapsed);
        }

        let mut state = read_vault_state(&env, &from).ok_or(EscrowError::VaultNotFound)?;
        if !state.is_active {
            return Err(EscrowError::VaultInactive);
        }
        if state.balance < auto_pay.amount {
            return Err(EscrowError::InsufficientBalance);
        }

        let recipient = resolve(&env, &auto_pay.to)?;
        let token_client = token::Client::new(&env, &auto_pay.token);
        token_client.transfer(
            &env.current_contract_address(),
            &recipient,
            &auto_pay.amount,
        );

        state.balance = state
            .balance
            .checked_sub(auto_pay.amount)
            .ok_or(EscrowError::ArithmeticError)?;
        write_vault_state(&env, &from, &state);

        auto_pay.last_paid = current_time;
        write_auto_pay(&env, &from, rule_id, &auto_pay);

        Events::auto_pay(
            &env,
            rule_id,
            auto_pay.from,
            auto_pay.to,
            auto_pay.amount,
            current_time,
        );
        Ok(())
    }

    pub fn get_balance(env: Env, commitment: BytesN<32>) -> Option<i128> {
        read_vault_state(&env, &commitment).map(|state| state.balance)
    }

    pub fn get_auto_pay_count(env: Env) -> u32 {
        read_auto_pay_count(&env)
    }

    pub fn get_auto_pay(env: Env, from: BytesN<32>, rule_id: u32) -> Option<AutoPay> {
        read_auto_pay(&env, &from, rule_id)
    }

    pub fn get_scheduled_payment(env: Env, payment_id: u32) -> Option<ScheduledPayment> {
        let key = DataKey::ScheduledPayment(payment_id);
        env.storage().persistent().get(&key)
    }

    pub fn is_vault_active(env: Env, commitment: BytesN<32>) -> Option<bool> {
        read_vault_state(&env, &commitment).map(|state| state.is_active)
    }
}

fn resolve(env: &Env, commitment: &BytesN<32>) -> Result<Address, EscrowError> {
    let config = read_vault_config(env, commitment).ok_or(EscrowError::VaultNotFound)?;
    Ok(config.owner)
}

fn require_vault_owner(config: &VaultConfig) {
    shared_auth::require_address_auth(&config.owner);
}
