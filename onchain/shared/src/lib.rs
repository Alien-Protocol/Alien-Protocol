#![no_std]

pub mod errors;

use soroban_sdk::{panic_with_error, Env};

// ─── Authorization Helpers ──────────────────────────────────────────────────

/// Unwraps an `Option<T>` stored in contract storage, panicking with a contract
/// error if the value is absent.
///
/// # Arguments
/// * `env`   – The current Soroban environment.
/// * `opt`   – The `Option<T>` to unwrap.
/// * `error` – A `#[contracterror]` value to panic with if `opt` is `None`.
///
/// # Example
/// ```ignore
/// let addr = get_or_panic(&env, get_auction_contract(&env), FactoryError::Unauthorized);
/// ```
#[inline]
pub fn get_or_panic<T, E>(env: &Env, opt: Option<T>, error: E) -> T
where
    E: Into<soroban_sdk::Error> + Copy,
{
    match opt {
        Some(v) => v,
        None => panic_with_error!(env, error),
    }
}

/// Asserts an `Option<T>` is `None`, panicking with a contract error if a value
/// already exists.  Useful for "must not already be registered" guards.
///
/// # Arguments
/// * `env`   – The current Soroban environment.
/// * `opt`   – The `Option<T>` to check.
/// * `error` – A `#[contracterror]` value to panic with if `opt` is `Some`.
#[inline]
pub fn assert_none_or_panic<T, E>(env: &Env, opt: Option<T>, error: E)
where
    E: Into<soroban_sdk::Error> + Copy,
{
    if opt.is_some() {
        panic_with_error!(env, error);
    }
}

// ─── TTL / Timestamp Helpers ─────────────────────────────────────────────────

/// Returns `true` if `release_at` is strictly in the future relative to the
/// current ledger timestamp.
#[inline]
pub fn is_future_timestamp(env: &Env, release_at: u64) -> bool {
    release_at > env.ledger().timestamp()
}

// ─── Counter Helpers ─────────────────────────────────────────────────────────

/// Reads the current `u32` value at `key` from instance storage (defaulting to
/// 0), increments it by 1, stores the new value, and **returns the old value**
/// as the allocated ID.
///
/// Returns `Err(overflow_error)` if the counter would overflow `u32::MAX`.
///
/// Callers supply a concrete `#[contracterror]` variant so the helper stays
/// generic and avoids coupling to any particular contract's error enum.
#[inline]
pub fn increment_instance_counter<K, E>(env: &Env, key: &K, overflow_error: E) -> Result<u32, E>
where
    K: soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>
        + soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>,
    E: Copy,
{
    let id: u32 = env.storage().instance().get(key).unwrap_or(0);
    let next = id.checked_add(1).ok_or(overflow_error)?;
    env.storage().instance().set(key, &next);
    Ok(id)
}
